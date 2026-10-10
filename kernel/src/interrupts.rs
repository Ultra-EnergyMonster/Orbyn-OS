//! Exception handling for the New OS kernel: our own GDT, a TSS with a double-fault IST stack, and
//! our own IDT.
//!
//! # Why this exists
//!
//! Milestone 1 handed control to the kernel with no interrupt-descriptor table of our own: the
//! reused `bootloader` component installs neither an IDT nor a TSS (verified against the pinned
//! 0.11.17 source), so before this module *any* CPU exception — even a plain page fault — had no
//! handler and took the machine down with a triple fault and no diagnostic. This module installs
//! the tables so that a fault is reported on the serial console before the kernel takes its
//! controlled halt path, and so that a deliberately resumable fault can return.
//!
//! # What is installed
//!
//! - A GDT with a null descriptor, a ring-0 kernel code descriptor at selector `0x08`, a ring-0
//!   kernel data descriptor at `0x10` and a 64-bit TSS descriptor at `0x18`. The code/data values
//!   match the selectors the bootloader already loaded, so `CS`/`DS` stay valid without a reload.
//! - A TSS whose `IST[1]` entry points at a dedicated 16 KiB `.bss` stack, so the double fault
//!   handler runs on a known-good stack even if the current one is corrupt.
//! - An IDT with all 256 entries: vectors `0..=31` use one stub each (with the dummy error code the
//!   CPU-supplied frame needs), vectors `32..=255` use a shared unexpected-vector stub. Vectors 2
//!   (`#NMI`), 8 (`#DF`) and 18 (`#MC`) run on the IST stack.
//! - A hardware timer: the 8259 PICs are remapped so IRQ0 becomes vector 32, the PIT is programmed
//!   for [`TIMER_HZ`], and [`enable_timer`] unmasks IRQ0 and enables interrupts. The timer is the
//!   only interrupt source; its handler counts and acknowledges and never touches the console.
//! - One shared assembly path ([`exception_common`]) that saves every general-purpose register,
//!   calls the Rust dispatcher ([`exception_dispatch`]), restores the registers and returns with
//!   `iretq`.
//!
//! # What this does NOT do
//!
//! Only the legacy 8259 PIC and PIT are used: no local APIC, no I/O APIC, no MSI, no per-CPU
//! interrupt routing, and every PIC line except the timer is left masked. There is no scheduler:
//! the timer only counts ticks and nothing consumes them yet. Vectors 32..=255 exist so that an
//! unexpected interrupt is reported instead of triple-faulting. There is no privilege separation
//! yet, so the handlers run at ring 0 on the kernel stack and the ring-3 paths (a TSS `rsp0` that
//! matters, `SS`/`RSP` in the frame) are unused. The page-fault handler cannot fix up a real fault;
//! only the kernel's own armed self-test probe is resumed, and everything else halts.

use core::arch::{asm, naked_asm};
use core::cell::UnsafeCell;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst};
use uart16550::SerialPort;
use x86exc::{EXCEPTION_VECTORS, GateDescriptor, PageFaultErrorCode, TablePointer, Vector};

use crate::{halt_forever, write_line};

/// I/O port base of COM1, the console the dispatcher reports on.
///
/// The dispatcher runs with no arguments and possibly before the kernel's console was opened, so
/// it opens its own handle the same way the panic handler does; see [`SerialPort::new`].
const COM1_BASE: u16 = 0x03F8;

/// Selector of the ring-0 kernel code descriptor in our GDT (also the value the bootloader loaded).
const KERNEL_CODE_SELECTOR: u16 = 0x08;

/// Selector of the TSS descriptor in our GDT.
const TSS_SELECTOR: u16 = 0x18;

/// Entries in the GDT: null, kernel code, kernel data, and the two halves of the TSS descriptor.
const GDT_ENTRIES: usize = 5;

/// IDT entries. All 256 are filled so an unexpected vector is reported, not a triple fault.
const IDT_ENTRIES: usize = 256;

/// IST index used for `#DF` (and the other "cannot trust the stack" vectors). `IST[1]` in the TSS
/// is entry 0 of `interrupt_stack_table`, i.e. IST index 1.
const DOUBLE_FAULT_IST: u8 = 1;

/// Size of the dedicated double-fault stack. One page would do; 16 KiB leaves room to report.
const DOUBLE_FAULT_STACK_SIZE: usize = 16 * 1024;

/// Size of the 64-bit TSS structure.
const TSS_SIZE: u16 = 104;

/// Long-mode ring-0 code descriptor: access `0x9A`, flags `0xA` (`G = 1`, `L = 1`, `D = 0`).
const KERNEL_CODE_DESCRIPTOR: u64 = 0x00AF_9A00_0000_FFFF;

/// Long-mode ring-0 data descriptor: access `0x92`, flags `0xC` (`G = 1`, `D = 1`).
const KERNEL_DATA_DESCRIPTOR: u64 = 0x00CF_9200_0000_FFFF;

/// TSS descriptor access byte: present, ring 0, type 9 (available 64-bit TSS).
const TSS_ACCESS_BYTE: u64 = 0x89;

/// A canonical but deliberately unmapped address the page-fault self-test probes.
///
/// The very last page of the lower canonical half. The bootloader maps the kernel and framebuffer
/// far below this, so the read must fault; the self-test treats "no fault" as a failure rather than
/// a silent pass.
const PAGE_FAULT_PROBE_ADDRESS: u64 = 0x0000_7FFF_FFFF_F000;

/// I/O ports of the legacy 8259 PICs (master and slave).
const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

/// End-of-interrupt command for the master PIC.
const PIC_EOI: u8 = 0x20;

/// Vector offsets the PICs are remapped to, so IRQ0..15 land on vectors 32..47 and no longer
/// collide with the CPU exceptions at 0..31.
const PIC_MASTER_OFFSET: u8 = 0x20;
const PIC_SLAVE_OFFSET: u8 = 0x28;

/// I/O ports and input clock of the 8253/8254 PIT.
const PIT_CHANNEL0: u16 = 0x40;
const PIT_COMMAND: u16 = 0x43;
const PIT_INPUT_HZ: u32 = 1_193_182;

/// Frequency the PIT is programmed for, in Hz.
pub const TIMER_HZ: u16 = 100;

/// Vector IRQ0 is delivered on once the PICs are remapped.
const TIMER_VECTOR: u8 = 32;

/// Set by the dispatcher when the `int3` self-test reaches it.
static BREAKPOINT_OBSERVED: AtomicBool = AtomicBool::new(false);

/// Armed only while the page-fault self-test is running, so a real page fault is never resumed.
static PAGE_FAULT_ARMED: AtomicBool = AtomicBool::new(false);

/// Set by the dispatcher when it resumed the armed page-fault probe.
static PAGE_FAULT_OBSERVED: AtomicBool = AtomicBool::new(false);

/// Timer interrupts observed since boot. Bumped from the interrupt handler.
static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);

/// The 64-bit task-state segment. Only `IST[1]` is used.
///
/// `#[repr(C, packed)]` is required: the hardware reads `rsp0` at offset 4, so the `u64` fields
/// must not be padded to 8-byte alignment. Fields are written and read through raw pointers with
/// `write_unaligned`/`read_unaligned` for that reason.
#[repr(C, packed)]
struct TaskStateSegment {
    reserved_1: u32,
    privilege_stack_table: [u64; 3],
    reserved_2: u64,
    interrupt_stack_table: [u64; 7],
    reserved_3: u64,
    reserved_4: u16,
    iomap_base: u16,
}

/// The one TSS, `.bss`.
struct TssStorage(UnsafeCell<TaskStateSegment>);

// SAFETY: a single CPU with interrupts disabled and no scheduler reaches this storage; every access
// is on the one boot path from `install`. Introducing interrupt sources or a second context requires
// proper synchronisation first.
unsafe impl Sync for TssStorage {}

/// The one GDT, `.bss`.
struct GdtStorage(UnsafeCell<[u64; GDT_ENTRIES]>);

// SAFETY: same single-context argument as `TssStorage`; written once during `install`.
unsafe impl Sync for GdtStorage {}

/// The one IDT, `.bss`.
struct IdtStorage(UnsafeCell<[u64; IDT_ENTRIES * 2]>);

// SAFETY: same single-context argument as `TssStorage`; written once during `install`.
unsafe impl Sync for IdtStorage {}

/// The dedicated double-fault stack, `.bss`.
struct StackStorage(UnsafeCell<[u8; DOUBLE_FAULT_STACK_SIZE]>);

// SAFETY: the CPU switches to this stack only for vectors 2/8/18, which do not nest, and it is
// written only by the CPU pushing an exception frame; no Rust reference into it is ever taken.
unsafe impl Sync for StackStorage {}

/// The all-zero TSS the kernel starts from: no privilege stacks, no I/O bitmap.
const fn empty_tss() -> TaskStateSegment {
    TaskStateSegment {
        reserved_1: 0,
        privilege_stack_table: [0; 3],
        reserved_2: 0,
        interrupt_stack_table: [0; 7],
        reserved_3: 0,
        reserved_4: 0,
        // An I/O-map base at the TSS limit, i.e. past the end of the segment: no bitmap, so all
        // I/O is privileged. Unused today because there is no ring 3.
        iomap_base: TSS_SIZE,
    }
}

static TSS: TssStorage = TssStorage(UnsafeCell::new(empty_tss()));
static GDT: GdtStorage = GdtStorage(UnsafeCell::new([0; GDT_ENTRIES]));
static IDT: IdtStorage = IdtStorage(UnsafeCell::new([0; IDT_ENTRIES * 2]));
static DOUBLE_FAULT_STACK: StackStorage =
    StackStorage(UnsafeCell::new([0; DOUBLE_FAULT_STACK_SIZE]));

/// The register/stack state the shared stub builds for the dispatcher.
///
/// The field order mirrors the push order in [`exception_common`] read from the lowest address:
/// the last value pushed (`r15`) is the first field. `vector`/`error_code` are pushed by the entry
/// stub, `rip`/`cs`/`rflags` by the CPU. When an IST is used the CPU also pushes `rsp`/`ss`, which
/// sit *after* `rflags` and are never read, so the layout below is identical in both cases.
#[repr(C)]
struct ExceptionFrame {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rbp: u64,
    rsi: u64,
    rdi: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,
    /// Vector number, pushed by the entry stub.
    vector: u64,
    /// Error code: the CPU's, or a dummy `0` the stub pushed so the layout is uniform.
    error_code: u64,
    /// Instruction pointer at the fault/trap.
    rip: u64,
    /// Code selector at the fault/trap.
    cs: u64,
    /// Flags at the fault/trap.
    rflags: u64,
}

impl ExceptionFrame {
    /// The vector number, masked to 8 bits so a frame the stub built cannot report a bogus value.
    fn vector(&self) -> Vector {
        Vector::new((self.vector & 0xFF) as u8)
    }
}

/// Defines one entry stub.
///
/// The stack layout the CPU leaves for an exception is `[error_code?] rip cs rflags`. The stub
/// pushes the vector number so the dispatcher can tell vectors apart, and for vectors with no
/// error code it first pushes a dummy `0`, keeping every frame identical.
macro_rules! define_exception_stub {
    ($name:ident, $vector:literal, with_error) => {
        #[unsafe(naked)]
        extern "C" fn $name() {
            naked_asm!(
                "push {vector}",
                "jmp {common}",
                vector = const $vector,
                common = sym exception_common,
            );
        }
    };
    ($name:ident, $vector:literal, without_error) => {
        #[unsafe(naked)]
        extern "C" fn $name() {
            naked_asm!(
                "push 0",
                "push {vector}",
                "jmp {common}",
                vector = const $vector,
                common = sym exception_common,
            );
        }
    };
}

/// Generates all 32 exception stubs and the table the IDT is built from.
macro_rules! exception_stub_table {
    ($(($name:ident, $vector:literal, $has_error:ident)),* $(,)?) => {
        $( define_exception_stub!($name, $vector, $has_error); )*

        /// Entry-stub addresses for vectors `0..=31`, indexed by vector number.
        static EXCEPTION_STUBS: [extern "C" fn(); EXCEPTION_VECTORS as usize] =
            [ $( $name ),* ];
    };
}

exception_stub_table!(
    (stub_0, 0, without_error),
    (stub_1, 1, without_error),
    (stub_2, 2, without_error),
    (stub_3, 3, without_error),
    (stub_4, 4, without_error),
    (stub_5, 5, without_error),
    (stub_6, 6, without_error),
    (stub_7, 7, without_error),
    (stub_8, 8, with_error),
    (stub_9, 9, without_error),
    (stub_10, 10, with_error),
    (stub_11, 11, with_error),
    (stub_12, 12, with_error),
    (stub_13, 13, with_error),
    (stub_14, 14, with_error),
    (stub_15, 15, without_error),
    (stub_16, 16, without_error),
    (stub_17, 17, with_error),
    (stub_18, 18, without_error),
    (stub_19, 19, without_error),
    (stub_20, 20, without_error),
    (stub_21, 21, with_error),
    (stub_22, 22, without_error),
    (stub_23, 23, without_error),
    (stub_24, 24, without_error),
    (stub_25, 25, without_error),
    (stub_26, 26, without_error),
    (stub_27, 27, without_error),
    (stub_28, 28, without_error),
    (stub_29, 29, with_error),
    (stub_30, 30, with_error),
    (stub_31, 31, without_error),
);

// Entry stub for the hardware timer, which IRQ0 is remapped to (vector 32). Like the exception
// stubs it pushes a dummy error code, because an interrupt delivers no error code; that keeps the
// frame layout identical for the shared path.
define_exception_stub!(timer_stub, 32, without_error);

/// Entry stub for every vector the kernel does not expect (32..=255).
///
/// The vector number is a sentinel so the dispatcher can say the vector is unknown rather than
/// misattributing it to vector 0.
#[unsafe(naked)]
extern "C" fn unexpected_vector_stub() {
    naked_asm!(
        "push 0",
        "push {vector}",
        "jmp {common}",
        vector = const 0xFFu64,
        common = sym exception_common,
    );
}

/// The shared entry path every stub jumps to: save registers, dispatch, restore, `iretq`.
///
/// `r15` is used to remember the frame address: it is callee-saved, so the Rust dispatcher
/// preserves it across the call, and it is itself part of the frame, so restoring it later needs
/// no extra storage.
#[unsafe(naked)]
extern "C" fn exception_common() {
    naked_asm!(
        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rdi",
        "push rsi",
        "push rbp",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov r15, rsp",
        // An exception can be delivered with the stack at either alignment. Align it for the Rust
        // ABI; the aligned stack is below the frame, so the call cannot overwrite the frame.
        "and rsp, -16",
        "mov rdi, r15",
        "call {dispatch}",
        "mov rsp, r15",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rbp",
        "pop rsi",
        "pop rdi",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",
        // Discard the vector number and error code, then return to the interrupted code.
        "add rsp, 16",
        "iretq",
        dispatch = sym exception_dispatch,
    );
}

/// Installs the GDT, TSS and IDT. Must run once, early, with interrupts disabled.
pub fn init() {
    // SAFETY: this is the earliest kernel code to run after the bootloader handoff. There is a
    // single CPU, interrupts are disabled (`IF = 0`), there is no scheduler and no other agent that
    // could read the tables while they are being written, so this is the only execution context.
    unsafe { install() }
}

/// Builds and loads the GDT/TSS and then the IDT.
///
/// # Safety
///
/// Must be called exactly once, on a single CPU, with interrupts disabled and before any other code
/// depends on the tables, so that no reader observes a half-written descriptor.
unsafe fn install() {
    let tss_base = TSS.0.get() as u64;
    let double_fault_stack_top = DOUBLE_FAULT_STACK.0.get() as u64 + DOUBLE_FAULT_STACK_SIZE as u64;

    // SAFETY: no other context can reach these tables (see `init`), and the field writes go
    // through raw pointers with `write_unaligned` because the TSS is a packed structure whose
    // fields are not naturally aligned.
    unsafe {
        let tss = ptr::addr_of_mut!(*TSS.0.get());
        ptr::addr_of_mut!((*tss).interrupt_stack_table)
            .cast::<u64>()
            .write_unaligned(double_fault_stack_top);

        let gdt = &mut *GDT.0.get();
        gdt[0] = 0;
        gdt[1] = KERNEL_CODE_DESCRIPTOR;
        gdt[2] = KERNEL_DATA_DESCRIPTOR;
        let (tss_low, tss_high) = tss_descriptor(tss_base, TSS_SIZE);
        gdt[3] = tss_low;
        gdt[4] = tss_high;

        let gdt_pointer = TablePointer::new(gdt.as_ptr() as u64, (size_of_val(gdt) - 1) as u16);
        asm!(
            "lgdt [{pointer}]",
            pointer = in(reg) &gdt_pointer,
            options(nostack, preserves_flags, readonly),
        );
        asm!(
            "ltr {selector:x}",
            selector = in(reg) TSS_SELECTOR,
            options(nostack, nomem, preserves_flags),
        );

        let idt = &mut *IDT.0.get();
        for vector in 0..IDT_ENTRIES {
            let (handler, ist) = if (vector as u8) < EXCEPTION_VECTORS {
                // #NMI, #DF and #MC must not run on a stack that may be the thing that failed.
                let ist = match vector as u8 {
                    2 | 8 | 18 => DOUBLE_FAULT_IST,
                    _ => 0,
                };
                (EXCEPTION_STUBS[vector] as *const () as u64, ist)
            } else if (vector as u8) == TIMER_VECTOR {
                (timer_stub as *const () as u64, 0)
            } else {
                (unexpected_vector_stub as *const () as u64, 0)
            };
            let (low, high) = GateDescriptor::interrupt_gate(handler, KERNEL_CODE_SELECTOR, 0, ist)
                .into_u64_pair();
            idt[vector * 2] = low;
            idt[vector * 2 + 1] = high;
        }

        let idt_pointer = TablePointer::new(idt.as_ptr() as u64, (size_of_val(idt) - 1) as u16);
        asm!(
            "lidt [{pointer}]",
            pointer = in(reg) &idt_pointer,
            options(nostack, preserves_flags, readonly),
        );
    }
}

/// Encodes the two GDT slots a 64-bit TSS descriptor occupies.
///
/// Layout (Intel SDM Vol. 3A, 3.5.2): limit/base split across the first eight bytes, then the high
/// 32 bits of the base in the ninth through twelfth bytes.
const fn tss_descriptor(base: u64, size: u16) -> (u64, u64) {
    let limit = (size as u64).saturating_sub(1);
    let low = (limit & 0xFFFF)
        | ((base & 0x00FF_FFFF) << 16)
        | (TSS_ACCESS_BYTE << 40)
        | (((limit >> 16) & 0xF) << 48)
        | (((base >> 24) & 0xFF) << 56);
    let high = base >> 32;
    (low, high)
}

/// The Rust exception dispatcher, called by [`exception_common`] with a pointer to the frame.
///
/// It reports the exception on COM1 and then decides: the `int3` self-test and the armed page-fault
/// probe are resumed by returning, and every other exception is fatal and halts after reporting.
extern "C" fn exception_dispatch(frame: *mut ExceptionFrame) {
    let frame_address = frame as u64;
    // SAFETY: `exception_common` passes the address of a frame it built on the current stack for
    // the duration of this call, and it is the only reference to it.
    let frame = unsafe { &mut *frame };

    let vector = frame.vector();

    // Hardware interrupts (vectors 32+) arrive here only through their IDT entry. The timer (IRQ0
    // remapped to vector 32) is the only source enabled; it is counted and acknowledged without
    // touching the console, so a 100 Hz interrupt cannot flood it. Any other interrupt is
    // unexpected and fatal.
    if vector.raw() >= 32 {
        if vector.raw() == TIMER_VECTOR {
            TIMER_TICKS.fetch_add(1, SeqCst);
            // SAFETY: the master PIC is the source of IRQ0 and the only PIC with an unmasked line;
            // writing its command port acknowledges the interrupt.
            unsafe {
                outb(PIC1_COMMAND, PIC_EOI);
            }
            return;
        }
        // SAFETY: the kernel is the sole owner of the COM1 UART (single CPU); writing it touches
        // only I/O ports.
        let mut console = unsafe { SerialPort::new(COM1_BASE) };
        write_line(
            &mut console,
            format_args!("[FATAL] unexpected interrupt on vector {}", vector.raw()),
        );
        halt_forever();
    }

    // SAFETY: the kernel is the sole owner of the COM1 UART (single CPU; the only interrupt source
    // is the timer, whose handler does not write here), and writing it touches only I/O ports. The
    // dispatcher only needs to transmit, so it uses a handle on the already-initialised port,
    // exactly as the panic handler does.
    let mut console = unsafe { SerialPort::new(COM1_BASE) };

    let page_fault_address = if vector.raw() == 14 {
        Some(read_cr2())
    } else {
        None
    };

    write_line(
        &mut console,
        format_args!(
            "[EXCEPTION] {} {} (vector {})",
            vector.mnemonic(),
            vector.description(),
            vector.raw()
        ),
    );
    if vector.pushes_error_code() {
        let code = PageFaultErrorCode::new(frame.error_code);
        if vector.raw() == 14 {
            write_line(
                &mut console,
                format_args!(
                    "  error code {:#x}: {code}; cr2={:#018x}",
                    frame.error_code,
                    page_fault_address.unwrap_or(0)
                ),
            );
        } else {
            write_line(
                &mut console,
                format_args!("  error code {:#x}", frame.error_code),
            );
        }
    }
    write_line(
        &mut console,
        format_args!(
            "  rip={:#018x} cs={:#06x} rflags={:#018x}; stack frame at {:#018x}",
            frame.rip, frame.cs, frame.rflags, frame_address
        ),
    );
    write_line(
        &mut console,
        format_args!(
            "  rax={:016x} rbx={:016x} rcx={:016x} rdx={:016x}",
            frame.rax, frame.rbx, frame.rcx, frame.rdx
        ),
    );
    write_line(
        &mut console,
        format_args!(
            "  rsi={:016x} rdi={:016x} rbp={:016x} r8 ={:016x}",
            frame.rsi, frame.rdi, frame.rbp, frame.r8
        ),
    );
    write_line(
        &mut console,
        format_args!(
            "  r9 ={:016x} r10={:016x} r11={:016x} r12={:016x}",
            frame.r9, frame.r10, frame.r11, frame.r12
        ),
    );
    write_line(
        &mut console,
        format_args!(
            "  r13={:016x} r14={:016x} r15={:016x}",
            frame.r13, frame.r14, frame.r15
        ),
    );

    // #BP from the self-test is a trap the CPU already resumed: return normally and the shared
    // stub's `iretq` continues at the next instruction.
    if vector.raw() == 3 {
        BREAKPOINT_OBSERVED.store(true, SeqCst);
        write_line(
            &mut console,
            format_args!("  resuming: breakpoint is a resumable trap"),
        );
        return;
    }

    // #PF: resume only the kernel's own armed probe, and only when the fault is on the address the
    // probe actually asked for. Any other page fault stays fatal, so a real bug is not swallowed.
    if vector.raw() == 14 {
        let cr2 = page_fault_address.unwrap_or(0);
        if PAGE_FAULT_ARMED.load(SeqCst) && cr2 == frame.rdi {
            frame.rip = frame.rax;
            PAGE_FAULT_OBSERVED.store(true, SeqCst);
            write_line(
                &mut console,
                format_args!(
                    "  resuming: armed page-fault self-test probe, continuing at rip={:#018x}",
                    frame.rip
                ),
            );
            return;
        }
    }

    write_line(
        &mut console,
        format_args!("[FATAL] unhandled {}", vector.mnemonic()),
    );
    halt_forever()
}

/// Reads CR2, the linear address of the most recent page fault.
fn read_cr2() -> u64 {
    let address: u64;
    // SAFETY: reading CR2 is a read-only privileged control-register access with no memory operand;
    // the kernel runs at ring 0.
    unsafe {
        asm!("mov {address}, cr2", address = out(reg) address, options(nomem, nostack, preserves_flags));
    }
    address
}

/// A deliberate 8-byte load (`rdf::probe`) that the page-fault self-test expects to fault.
///
/// The address is passed in `rdi`. Before the faulting load, `rax` is set to the address of the
/// instruction after it, so the handler can set `rip` there and resume cleanly; on resume the
/// `ret` returns to the Rust caller as if nothing happened.
#[unsafe(naked)]
extern "C" fn page_fault_probe(_address: u64) {
    naked_asm!("lea rax, [rip + 2f]", "mov rcx, [rdi]", "2:", "ret",);
}

/// Writes one byte to an I/O port.
///
/// # Safety
///
/// `port` must belong to a device the kernel owns; writing it is otherwise unreviewed.
unsafe fn outb(port: u16, value: u8) {
    // SAFETY: `out` writes one byte to an I/O port and touches no memory; the caller guarantees the
    // port is one the kernel owns. The kernel runs at ring 0.
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}

/// A short settle delay for the legacy PIC: a write to the unused POST/diagnostic port `0x80`.
///
/// # Safety
///
/// Writing the POST port is harmless on the test platform.
unsafe fn io_wait() {
    // SAFETY: port 0x80 is the POST/diagnostic port; writing zero has no effect this kernel relies
    // on and is the conventional PIC settle delay.
    unsafe {
        outb(0x80, 0);
    }
}

/// Remaps the 8259 PICs, unmasks only IRQ0, and programs the PIT to [`TIMER_HZ`].
///
/// # Safety
///
/// The kernel must own the legacy PIC/PIT ports and interrupts must be disabled, so that no
/// interrupt is delivered while the PIC is being reprogrammed.
unsafe fn program_pic_and_timer() {
    // SAFETY: the kernel is the only owner of these ports, and `enable_timer` calls this before
    // `sti`, so `IF = 0` and no interrupt can arrive mid-reprogramming.
    unsafe {
        // ICW1: begin initialisation, expect ICW4, cascade mode.
        outb(PIC1_COMMAND, 0x11);
        io_wait();
        outb(PIC2_COMMAND, 0x11);
        io_wait();
        // ICW2: interrupt vector offsets (master 0x20, slave 0x28).
        outb(PIC1_DATA, PIC_MASTER_OFFSET);
        io_wait();
        outb(PIC2_DATA, PIC_SLAVE_OFFSET);
        io_wait();
        // ICW3: the master's slave is on IRQ2, and the slave's cascade identity is 2.
        outb(PIC1_DATA, 0x04);
        io_wait();
        outb(PIC2_DATA, 0x02);
        io_wait();
        // ICW4: 8086/88 mode.
        outb(PIC1_DATA, 0x01);
        io_wait();
        outb(PIC2_DATA, 0x01);
        io_wait();
        // Mask every line, then unmask IRQ0 only. The slave stays fully masked, so its cascade line
        // can never deliver anything either.
        outb(PIC1_DATA, 0xFE);
        outb(PIC2_DATA, 0xFF);

        // PIT channel 0, lo/hi byte, mode 3 (square wave), binary counting.
        outb(PIT_COMMAND, 0x36);
        let divisor = PIT_INPUT_HZ / u32::from(TIMER_HZ);
        outb(PIT_CHANNEL0, (divisor & 0xFF) as u8);
        outb(PIT_CHANNEL0, ((divisor >> 8) & 0xFF) as u8);
    }
}

/// Programs the PIC and PIT and enables interrupts, starting the timer.
///
/// The only unmasked interrupt source is the timer, and its IDT gate already exists (installed by
/// [`init`]), so after this returns the kernel receives IRQ0 as vector [`TIMER_VECTOR`].
pub fn enable_timer() {
    // SAFETY: `init` installed the IDT before this is called and `IF = 0` until the explicit `sti`
    // below, so the PIC is fully reprogrammed before any timer interrupt can be delivered. Enabling
    // interrupts is then safe because the timer is the only unmasked source and its handler exists.
    unsafe {
        program_pic_and_timer();
        // `sti` changes `IF`, so `preserves_flags` is deliberately not asserted.
        asm!("sti", options(nomem, nostack));
    }
}

/// Number of timer interrupts observed since boot.
pub fn timer_ticks() -> u64 {
    TIMER_TICKS.load(SeqCst)
}

/// Parks the CPU with `hlt` until the timer has fired at least `count` times.
///
/// `hlt` sleeps until the next interrupt; with the timer enabled this returns promptly. Used by the
/// boot self-test, before any scheduler exists.
pub fn wait_for_ticks(count: u64) {
    while timer_ticks() < count {
        // SAFETY: `hlt` is a privileged instruction that touches no memory and only suspends the
        // core; the timer's handler resumes execution.
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

/// Runs both exception self-tests through the installed tables.
///
/// Returns a description of the first step that did not behave as expected. A failure leaves the
/// caller free to take its own fatal path; it does not throw away the console.
pub fn run_self_tests() -> Result<(), &'static str> {
    // 1. A resumable trap. `int3` raises vector 3 through the IDT, the stub and the shared path, and
    //    the handler returns, so this also proves the `iretq` resume path works.
    BREAKPOINT_OBSERVED.store(false, SeqCst);
    // SAFETY: the `int3` instruction raises vector 3, for which a handler is installed. It has no
    // memory operands and does not touch the stack; the handler resumes at the next instruction.
    unsafe {
        asm!("int3", options(nomem, nostack, preserves_flags));
    }
    if !BREAKPOINT_OBSERVED.load(SeqCst) {
        return Err("the breakpoint exception (int3) did not reach the handler");
    }

    // 2. A page fault. The probe loads from an unmapped canonical address; the handler reports it
    //    and resumes, so boot continues instead of ending in a fault.
    PAGE_FAULT_ARMED.store(true, SeqCst);
    PAGE_FAULT_OBSERVED.store(false, SeqCst);
    page_fault_probe(PAGE_FAULT_PROBE_ADDRESS);
    PAGE_FAULT_ARMED.store(false, SeqCst);
    if !PAGE_FAULT_OBSERVED.load(SeqCst) {
        return Err("no page fault was reported for the probe address (is it mapped?)");
    }
    Ok(())
}
