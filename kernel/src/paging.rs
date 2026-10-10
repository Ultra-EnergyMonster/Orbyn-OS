//! Read-only virtual-to-physical address translation over the page tables that are currently
//! installed.
//!
//! # Why this exists
//!
//! `BootInfo` reports the kernel image as a *physical* address but the kernel stack and the
//! framebuffer only as *virtual* addresses, so the allocator could not reserve their frames and had
//! to rely on the bootloader having marked them non-usable. This module implements the missing
//! piece: it walks the page tables `CR3` points at, using the physical-memory mapping the bootloader
//! exposes, so the kernel can translate those virtual addresses itself.
//!
//! [`install_own_page_tables`] then builds the kernel's **own** top-level page table: a private PML4
//! with every one of the bootloader's mappings copied in, plus one new mapping built from frames the
//! kernel allocated, installed by loading `CR3`. After that call the kernel runs on a table it owns.
//!
//! # What this does NOT do
//!
//! The four lower levels behind the mirrored entries are still the bootloader's tables, so there is
//! still no address space of our own, no `map`/`unmap` of arbitrary ranges, and no protection or
//! isolation: every mapped page stays writable and user-accessible exactly as the bootloader left
//! it. There is no frame reclamation either, so frames allocated for page tables are never freed.
//! Replacing the bootloader's mappings outright is later work and is not claimed here.

use bootloader_api::BootInfo;
use core::arch::asm;
use core::fmt;
use physmem::FrameAllocator;
use uart16550::SerialPort;
use x86paging::{ENTRY_ADDRESS_MASK, TranslateError};

use crate::write_line;

/// Why a translation or page-table operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// The bootloader did not expose a physical-memory mapping, so paging-structure entries cannot
    /// be read or written at all.
    NoPhysicalMemoryOffset,
    /// The page-table walk itself failed.
    Walk(TranslateError),
    /// The frame allocator had no free frame for a page table or the mapped frame.
    OutOfFrames,
    /// Every candidate top-level slot was already occupied by the bootloader's tables.
    NoFreeTableSlot,
    /// The new tables were installed but reading back through them did not return what was written.
    VerificationFailed,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::NoPhysicalMemoryOffset => {
                write!(f, "the bootloader exposed no physical-memory offset")
            }
            Failure::Walk(err) => write!(f, "{err}"),
            Failure::OutOfFrames => write!(f, "the frame allocator had no free frame"),
            Failure::NoFreeTableSlot => write!(f, "no free top-level page-table slot"),
            Failure::VerificationFailed => {
                write!(f, "the installed page tables did not read back as written")
            }
        }
    }
}

/// The physical-memory offset, or the failure that explains why translation is impossible.
fn physical_memory_offset(boot_info: &BootInfo) -> Result<u64, Failure> {
    boot_info
        .physical_memory_offset
        .as_ref()
        .copied()
        .ok_or(Failure::NoPhysicalMemoryOffset)
}

/// Reads `CR3`, the physical address of the active PML4 (page-map level 4) table.
fn read_cr3() -> u64 {
    let value: u64;
    // SAFETY: reading `CR3` is a read-only privileged control-register access with no memory
    // operand; the kernel runs at ring 0.
    unsafe {
        asm!("mov {value}, cr3", value = out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Translates `virt` to a physical address using the currently installed page tables.
pub fn translate(boot_info: &BootInfo, virt: u64) -> Result<u64, Failure> {
    let offset = physical_memory_offset(boot_info)?;
    // CR3's low bits are flags, and bits 52..63 are reserved in 4-level paging, so masking with the
    // entry address mask leaves the PML4 base.
    let pml4 = read_cr3() & ENTRY_ADDRESS_MASK;
    x86paging::translate(pml4, virt, |physical| read_physical(offset, physical))
        .map_err(Failure::Walk)
}

/// Writes the 8-byte value at physical address `physical` through the bootloader's mapping.
fn write_physical(offset: u64, physical: u64, value: u64) {
    // SAFETY: as for `read_physical`, `physical` is a real paging-structure or frame address inside
    // RAM, which the bootloader maps at `offset`; the write is an unaligned-safe 8-byte store.
    unsafe { core::ptr::write_unaligned((offset + physical) as *mut u64, value) }
}

/// Loads `pml4` into `CR3`, which also flushes the non-global TLB entries.
fn write_cr3(pml4: u64) {
    // SAFETY: loading `CR3` is a privileged control-register write with no memory operand. `pml4` is
    // a 4 KiB-aligned frame holding a complete copy of the bootloader's top-level table, so the
    // address space is unchanged by the switch.
    unsafe {
        asm!("mov cr3, {value}", value = in(reg) pml4, options(nostack, preserves_flags));
    }
}

/// Virtual address of the kernel's first own mapping: PML4 slot 256 with the other indices zero.
const KERNEL_MAPPING_VIRTUAL: u64 = 0xFFFF_8000_0000_0000;

/// A value written to a frame and read back through [`KERNEL_MAPPING_VIRTUAL`] to prove the mapping.
const KERNEL_MAPPING_MAGIC: u64 = 0xC0FF_EE00_1234_5678;

/// The first free top-level slot to grow the address space into.
///
/// The bootloader places its mappings in the lower half (kernel image, stack, framebuffer, physical
/// memory), so the kernel builds its own mappings from slot 256 upward.
const FIRST_KERNEL_TABLE_SLOT: u64 = 256;

/// Builds the kernel's own page tables: a private PML4 mirroring the bootloader's entries, plus one
/// newly mapped frame, then installs them by loading `CR3`.
///
/// This is the point at which the kernel owns its top-level page table instead of running on the
/// one the bootloader created. The lower-level tables behind the mirrored entries are still the
/// bootloader's, so the address space is preserved; the new mapping is built entirely from frames
/// this kernel allocated.
///
/// Returns `Err` with the reason rather than faulting or pretending: the caller decides what to do.
pub fn install_own_page_tables(
    console: &mut SerialPort,
    allocator: &mut FrameAllocator<'_>,
    boot_info: &BootInfo,
) -> Result<(), Failure> {
    let offset = physical_memory_offset(boot_info)?;
    let current_pml4 = read_cr3() & ENTRY_ADDRESS_MASK;

    // A private top-level table with every existing mapping copied in. Mirrored entries keep
    // pointing at the bootloader's lower-level tables, so nothing that already worked stops working.
    let new_pml4 = allocator.allocate().ok_or(Failure::OutOfFrames)?;
    for index in 0..512_u64 {
        let entry = read_physical(offset, current_pml4 + index * 8);
        write_physical(offset, new_pml4 + index * 8, entry);
    }

    // Find a free top-level slot for the kernel's own mappings.
    let slot = (FIRST_KERNEL_TABLE_SLOT..512)
        .find(|index| read_physical(offset, new_pml4 + index * 8) == 0)
        .ok_or(Failure::NoFreeTableSlot)?;

    // Allocate and zero the three new levels and the frame they will map. Fresh frames are not
    // guaranteed zeroed (they hold whatever firmware left behind), so they must be cleared.
    let pdpt = allocator.allocate().ok_or(Failure::OutOfFrames)?;
    let pd = allocator.allocate().ok_or(Failure::OutOfFrames)?;
    let pt = allocator.allocate().ok_or(Failure::OutOfFrames)?;
    let mapped = allocator.allocate().ok_or(Failure::OutOfFrames)?;
    for table in [pdpt, pd, pt] {
        for index in 0..512_u64 {
            write_physical(offset, table + index * 8, 0);
        }
    }

    let flags = x86paging::ENTRY_PRESENT | x86paging::ENTRY_WRITABLE;
    write_physical(offset, new_pml4 + slot * 8, pdpt | flags);
    write_physical(offset, pdpt, pd | flags);
    write_physical(offset, pd, pt | flags);
    write_physical(offset, pt, mapped | flags);

    // Put a known value in the frame we are about to map.
    write_physical(offset, mapped, KERNEL_MAPPING_MAGIC);

    // Install the new top-level table and confirm the hardware accepted it.
    write_cr3(new_pml4);
    if read_cr3() & ENTRY_ADDRESS_MASK != new_pml4 {
        return Err(Failure::VerificationFailed);
    }

    // SAFETY: the address is canonical and is mapped by the table just installed (top slot `slot` ->
    // `pdpt` -> `pd` -> `pt` -> the frame holding the magic), so the read touches RAM this kernel
    // allocated and owns.
    let observed = unsafe { core::ptr::read_unaligned(KERNEL_MAPPING_VIRTUAL as *const u64) };
    if observed != KERNEL_MAPPING_MAGIC {
        return Err(Failure::VerificationFailed);
    }

    // The bootloader's mappings must have survived the switch, so re-translate the kernel stack.
    translate(boot_info, boot_info.kernel_stack_bottom)?;

    write_line(
        console,
        format_args!(
            "  page tables: own PML4 at physical {new_pml4:#018x}; slot {slot} maps {KERNEL_MAPPING_VIRTUAL:#018x} -> physical {mapped:#018x} (read back {observed:#018x})"
        ),
    );
    Ok(())
}

/// Reads the 8-byte paging-structure entry at physical address `physical`.
///
/// # Safety contract (upheld by [`translate`])
///
/// `physical` is always either the PML4 base taken from `CR3` or an address read out of a *present*
/// paging-structure entry, so it lies in physical RAM, which the bootloader maps at `offset`.
fn read_physical(offset: u64, physical: u64) -> u64 {
    // SAFETY: the physical-memory mapping the bootloader created covers all of RAM, and `physical`
    // is a real paging-structure address as described above; the read is an unaligned-safe 8-byte
    // load from that mapping.
    unsafe { core::ptr::read_unaligned((offset + physical) as *const u64) }
}

/// Translates one virtual region and reserves its physical frames, reporting the result.
///
/// Returns whether the region was translated and its frames reserved. A translation failure is not
/// fatal: the kernel still runs, so the caller reports it as a limitation rather than halting.
pub fn reserve_translated(
    console: &mut SerialPort,
    allocator: &mut FrameAllocator<'_>,
    boot_info: &BootInfo,
    what: &str,
    virt: u64,
    len: u64,
) -> bool {
    let physical = match translate(boot_info, virt) {
        Ok(physical) => physical,
        Err(err) => {
            write_line(
                console,
                format_args!(
                    "  could not reserve {what}: {virt:#018x} is not translatable ({err})"
                ),
            );
            return false;
        }
    };

    match allocator.mark_reserved(physical, len) {
        Ok(report) => {
            write_line(
                console,
                format_args!(
                    "  reserved {what}: virtual {virt:#018x} -> physical {physical:#018x}..{end:#018x} ({len} bytes, {cleared} previously usable frame(s) reclaimed)",
                    end = physical.saturating_add(len),
                    cleared = report.frames_cleared
                ),
            );
            true
        }
        Err(err) => {
            write_line(
                console,
                format_args!(
                    "  could not reserve {what}: physical {physical:#018x} was rejected by the allocator ({err})"
                ),
            );
            false
        }
    }
}
