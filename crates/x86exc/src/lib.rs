//! x86-64 exception metadata and descriptor encoding for the New OS kernel.
//!
//! # Scope (milestone 2, exception slice)
//!
//! This crate holds the parts of x86-64 exception handling that are plain data and arithmetic and
//! can therefore be tested on the host, away from hardware:
//!
//! - the architectural vector numbering (0..=31), with each vector's mnemonic, description and
//!   whether the CPU pushes an error code for it ([`Vector`]);
//! - the 16-byte IDT gate descriptor encoding ([`GateDescriptor`]);
//! - the 10-byte `GDTR`/`IDTR` pointer layout ([`TablePointer`]);
//! - decoding of the page-fault error code ([`PageFaultErrorCode`]).
//!
//! # Deliberately absent
//!
//! There is no port or memory-mapped I/O and no assembly here: installing the tables (`lgdt`,
//! `lidt`, `ltr`) and running the handlers is the kernel's job. Interrupt gates only are encoded;
//! trap gates, task gates, `int 3`-style software gates at other privilege levels, and the rest of
//! the descriptor-table machinery are not implemented and are not claimed.
//!
//! The error-code table is the part most worth testing: getting it wrong shifts every subsequent
//! field the handler reads, so a wrong answer here silently corrupts the diagnostic rather than
//! failing loudly.

// The kernel is `no_std`. Host `cargo test` needs `std` for the test harness, so `no_std` applies
// only when the crate is not being compiled for tests (same convention as `uart16550`/`physmem`).
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_op_in_unsafe_fn)]

use core::fmt;

/// Number of architectural exception/interrupt vectors defined by x86-64: vectors `0..=31`.
///
/// Vectors 32 and above are external interrupts (or, with the local APIC, local interrupt sources),
/// which this milestone does not use; the kernel routes any unexpected vector to a generic handler.
pub const EXCEPTION_VECTORS: u8 = 32;

/// `(mnemonic, description)` for each architectural vector, indexed by vector number.
///
/// Names follow Intel SDM Vol. 3A, "Exception and Interrupt Reference". Vectors Intel lists as
/// reserved carry a `(reserved)` mnemonic so that the console report never invents a meaning for
/// them.
const EXCEPTION_NAMES: [(&str, &str); EXCEPTION_VECTORS as usize] = [
    ("#DE", "divide-by-zero error"),
    ("#DB", "debug"),
    ("NMI", "non-maskable interrupt"),
    ("#BP", "breakpoint"),
    ("#OF", "overflow"),
    ("#BR", "bound-range exceeded"),
    ("#UD", "invalid opcode"),
    ("#NM", "device not available"),
    ("#DF", "double fault"),
    ("(reserved)", "coprocessor segment overrun"),
    ("#TS", "invalid task-state segment"),
    ("#NP", "segment not present"),
    ("#SS", "stack-segment fault"),
    ("#GP", "general-protection fault"),
    ("#PF", "page fault"),
    ("(reserved)", "reserved"),
    ("#MF", "x87 floating-point exception"),
    ("#AC", "alignment check"),
    ("#MC", "machine check"),
    ("#XM", "SIMD floating-point exception"),
    ("#VE", "virtualization exception"),
    ("#CP", "control-protection exception"),
    ("(reserved)", "reserved"),
    ("(reserved)", "reserved"),
    ("(reserved)", "reserved"),
    ("(reserved)", "reserved"),
    ("(reserved)", "reserved"),
    ("(reserved)", "reserved"),
    ("#HV", "hypervisor-injection exception"),
    ("#VC", "VMM-communication exception"),
    ("#SX", "security exception"),
    ("(reserved)", "reserved"),
];

/// An architectural exception/interrupt vector as found in the exception frame.
///
/// A raw `u8` is accepted (the kernel routes unexpected vectors to a generic handler that reports
/// them), so a value of 32 or above is representable and reports itself as unknown rather than
/// being rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vector(u8);

impl Vector {
    /// Wraps a raw vector number without validating it.
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }

    /// The raw vector number.
    pub const fn raw(self) -> u8 {
        self.0
    }

    /// Whether this is one of the 32 architectural vectors (`0..=31`).
    pub const fn is_architectural(self) -> bool {
        self.0 < EXCEPTION_VECTORS
    }

    /// Short mnemonic such as `#PF`, or `?` for a vector with no architectural meaning.
    pub fn mnemonic(self) -> &'static str {
        EXCEPTION_NAMES
            .get(self.0 as usize)
            .map_or("?", |entry| entry.0)
    }

    /// Human-readable description such as `page fault`.
    pub fn description(self) -> &'static str {
        EXCEPTION_NAMES
            .get(self.0 as usize)
            .map_or("unknown vector", |entry| entry.1)
    }

    /// Whether the CPU pushes an error code on the stack for this vector.
    ///
    /// On x86-64 these are vectors 8, 10, 11, 12, 13, 14, 17, 21, 29 and 30. Every other vector
    /// has no error code, so the entry stub must push a dummy value to keep the frame layout
    /// uniform, and a wrong answer here would shift `rip`/`cs`/`rflags` in the frame.
    pub const fn pushes_error_code(self) -> bool {
        matches!(self.0, 8 | 10 | 11 | 12 | 13 | 14 | 17 | 21 | 29 | 30)
    }

    /// Whether returning from the handler resumes the interrupted code instead of being fatal.
    ///
    /// These are the trap-class exceptions whose saved `rip` already points at the next
    /// instruction: `#DB` (1), `#BP` (3) and `#OF` (4). `#DB` is only resumable in the narrow
    /// sense that the CPU pushes a next-instruction `rip`; a debug handler must interpret the
    /// debug status register, which this kernel does not do, so the kernel only resumes `#BP`.
    pub const fn is_resumable(self) -> bool {
        matches!(self.0, 1 | 3 | 4)
    }
}

/// A 16-byte x86-64 interrupt-gate descriptor (one IDT entry), split into its two 64-bit halves.
///
/// The encoding follows Intel SDM Vol. 3A, 6.14.1 ("64-Bit Mode IDT"):
///
/// | Bits | Meaning |
/// | --- | --- |
/// | 0..15 | handler address bits 0..15 |
/// | 16..31 | code segment selector |
/// | 32..34 | IST index (0 = do not switch stacks) |
/// | 40..43 | gate type (0xE = 64-bit interrupt gate) |
/// | 45..46 | descriptor privilege level |
/// | 47 | present |
/// | 48..63 | handler address bits 16..31 |
/// | 64..95 | handler address bits 32..63 |
/// | 96..127 | reserved, zero |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateDescriptor {
    low: u64,
    high: u64,
}

impl GateDescriptor {
    /// Gate type field for a 64-bit interrupt gate (`0xE`), which clears `IF` on entry.
    const INTERRUPT_GATE_TYPE: u64 = 0x0E;

    /// Builds an interrupt gate for `handler` using `code_selector`, `dpl` and an IST index.
    ///
    /// `ist` is masked to its three hardware bits; `dpl` is masked to two. Values `ist = 0` and
    /// `dpl = 0` are the ordinary kernel case. `handler` is an absolute address.
    pub const fn interrupt_gate(handler: u64, code_selector: u16, dpl: u8, ist: u8) -> Self {
        let type_attr = Self::INTERRUPT_GATE_TYPE | ((dpl as u64 & 0x3) << 5) | 0x80; // present bit
        let low = (handler & 0xFFFF)
            | ((code_selector as u64) << 16)
            | ((ist as u64 & 0x7) << 32)
            | (type_attr << 40)
            | (((handler >> 16) & 0xFFFF) << 48);
        let high = handler >> 32;
        Self { low, high }
    }

    /// The descriptor as the two little-endian 64-bit words an IDT stores it as.
    pub const fn into_u64_pair(self) -> (u64, u64) {
        (self.low, self.high)
    }
}

/// The 10-byte `GDTR`/`IDTR` pseudo-descriptor that `lgdt`/`lidt` read from memory.
///
/// `#[repr(C, packed)]` matters: the hardware expects a 16-bit limit at offset 0 followed by a
/// 64-bit base at offset 2, with no padding between them.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TablePointer {
    /// Table limit in bytes (size minus one).
    pub limit: u16,
    /// Linear address of the table.
    pub base: u64,
}

impl TablePointer {
    /// Builds a pointer to a table of `size` bytes at `base`.
    pub const fn new(base: u64, limit: u16) -> Self {
        Self { limit, base }
    }

    /// Size of the on-memory encoding, used to guard the `lidt`/`lgdt` operand.
    pub const SIZE: usize = 10;
}

/// The error code the CPU pushes for a page fault (vector 14), decoded.
///
/// Intel SDM Vol. 3A, 4.7 describes the bits. Bit 0 distinguishes a not-present page from a
/// protection violation; the remaining bits describe the access that caused the fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFaultErrorCode(u64);

impl PageFaultErrorCode {
    /// Wraps the raw error code.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw error code, for the diagnostic line.
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Bit 0: false means the page was not present, true means a protection violation.
    pub const fn present(self) -> bool {
        self.0 & 0b1 != 0
    }

    /// True when the faulting access was a write.
    pub const fn write(self) -> bool {
        self.0 & 0b10 != 0
    }

    /// True when the access happened at user (ring 3) privilege.
    pub const fn user(self) -> bool {
        self.0 & 0b100 != 0
    }

    /// True when the fault was caused by a set reserved bit in a paging-structure entry.
    pub const fn reserved_bit(self) -> bool {
        self.0 & 0b1000 != 0
    }

    /// True when the fault was caused by an instruction fetch.
    pub const fn instruction_fetch(self) -> bool {
        self.0 & 0b1_0000 != 0
    }

    /// True when the fault was caused by a protection-key violation.
    pub const fn protection_key(self) -> bool {
        self.0 & 0b10_0000 != 0
    }

    /// True when the fault was a shadow-stack access (`SS` bit 7, CET).
    pub const fn shadow_stack(self) -> bool {
        self.0 & 0b1000_0000 != 0
    }

    /// True when the fault was an SGX access (bit 15).
    pub const fn sgx(self) -> bool {
        self.0 & (1 << 15) != 0
    }
}

impl fmt::Display for PageFaultErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}, {}, {}",
            if self.present() {
                "protection violation"
            } else {
                "not-present page"
            },
            if self.write() { "write" } else { "read" },
            if self.user() { "user" } else { "supervisor" },
        )?;
        if self.instruction_fetch() {
            write!(f, ", instruction fetch")?;
        }
        if self.reserved_bit() {
            write!(f, ", reserved bit set")?;
        }
        if self.protection_key() {
            write!(f, ", protection key")?;
        }
        if self.shadow_stack() {
            write!(f, ", shadow stack")?;
        }
        if self.sgx() {
            write!(f, ", SGX")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn architectural_vectors_have_names() {
        assert_eq!(Vector::new(14).mnemonic(), "#PF");
        assert_eq!(Vector::new(14).description(), "page fault");
        assert_eq!(Vector::new(8).mnemonic(), "#DF");
        assert_eq!(Vector::new(13).mnemonic(), "#GP");
        assert!(Vector::new(14).is_architectural());
    }

    #[test]
    fn reserved_vectors_are_labelled_as_reserved() {
        assert_eq!(Vector::new(15).mnemonic(), "(reserved)");
        assert_eq!(Vector::new(31).description(), "reserved");
    }

    #[test]
    fn unknown_vectors_do_not_invent_a_meaning() {
        let unknown = Vector::new(0xFF);
        assert!(!unknown.is_architectural());
        assert_eq!(unknown.mnemonic(), "?");
        assert_eq!(unknown.description(), "unknown vector");
    }

    /// The error-code table is the field that silently shifts the whole frame if it is wrong.
    #[test]
    fn error_code_vectors_match_the_architecture() {
        // Exactly these vectors push an error code on x86-64, and no others.
        let expected = [8, 10, 11, 12, 13, 14, 17, 21, 29, 30];
        let actual: Vec<u8> = (0..EXCEPTION_VECTORS)
            .filter(|vector| Vector::new(*vector).pushes_error_code())
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn only_trap_class_exceptions_are_resumable() {
        assert!(Vector::new(1).is_resumable());
        assert!(Vector::new(3).is_resumable());
        assert!(Vector::new(4).is_resumable());
        assert!(!Vector::new(14).is_resumable());
        assert!(!Vector::new(13).is_resumable());
        assert!(!Vector::new(8).is_resumable());
    }

    #[test]
    fn interrupt_gate_encodes_every_field_at_the_right_bits() {
        let handler = 0x0000_FFFF_1234_5678_u64;
        let gate = GateDescriptor::interrupt_gate(handler, 0x08, 0, 0);
        let (low, high) = gate.into_u64_pair();

        // Offset 0..15 and 16..31, selector, type/attributes and offset 32..63.
        assert_eq!(low & 0xFFFF, 0x5678, "offset bits 0..15");
        assert_eq!((low >> 16) & 0xFFFF, 0x08, "selector");
        assert_eq!((low >> 32) & 0x7, 0, "IST index");
        assert_eq!((low >> 40) & 0xFF, 0x8E, "present + DPL 0 + interrupt gate");
        assert_eq!((low >> 48) & 0xFFFF, 0x1234, "offset bits 16..31");
        assert_eq!(high, 0x0000_FFFF, "offset bits 32..63");
    }

    #[test]
    fn interrupt_gate_encodes_ist_and_dpl() {
        let gate = GateDescriptor::interrupt_gate(0x1000, 0x10, 3, 2);
        let (low, _) = gate.into_u64_pair();
        assert_eq!((low >> 32) & 0x7, 2, "IST index 2");
        assert_eq!((low >> 40) & 0xFF, 0xEE, "present + DPL 3 + interrupt gate");
        assert_eq!((low >> 16) & 0xFFFF, 0x10, "selector");
    }

    #[test]
    fn table_pointer_is_ten_bytes_with_no_padding() {
        assert_eq!(core::mem::size_of::<TablePointer>(), TablePointer::SIZE);
        let pointer = TablePointer::new(0x0000_1234_5678_9ABC, 0x27FF);
        // The fields are packed, so assert_eq! cannot take references to them directly.
        let limit = pointer.limit;
        let base = pointer.base;
        assert_eq!(limit, 0x27FF);
        assert_eq!(base, 0x0000_1234_5678_9ABC);
    }

    #[test]
    fn page_fault_error_code_decodes_cause_and_access() {
        let not_present_read_supervisor = PageFaultErrorCode::new(0);
        assert!(!not_present_read_supervisor.present());
        assert!(!not_present_read_supervisor.write());
        assert!(!not_present_read_supervisor.user());
        assert_eq!(
            not_present_read_supervisor.to_string(),
            "not-present page, read, supervisor"
        );

        // 0b111 = present, write, user.
        let protection_write_user = PageFaultErrorCode::new(0b111);
        assert!(protection_write_user.present());
        assert!(protection_write_user.write());
        assert!(protection_write_user.user());
        assert_eq!(
            protection_write_user.to_string(),
            "protection violation, write, user"
        );
    }

    #[test]
    fn page_fault_error_code_reports_optional_flags() {
        let code = PageFaultErrorCode::new(0b1_0000 | 0b1000 | 0b10_0000 | 0b1000_0000 | (1 << 15));
        assert!(code.instruction_fetch());
        assert!(code.reserved_bit());
        assert!(code.protection_key());
        assert!(code.shadow_stack());
        assert!(code.sgx());
        let text = code.to_string();
        for expected in [
            "instruction fetch",
            "reserved bit set",
            "protection key",
            "shadow stack",
            "SGX",
        ] {
            assert!(text.contains(expected), "missing '{expected}' in '{text}'");
        }
        assert_eq!(code.raw(), 16 | 8 | 32 | 128 | (1 << 15));
    }
}
