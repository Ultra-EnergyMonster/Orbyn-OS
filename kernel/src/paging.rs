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
//! # What this does NOT do
//!
//! It only reads. The kernel still runs on the page tables the bootloader created: this module does
//! **not** allocate, create, modify or install page tables, does not switch `CR3`, and does not
//! provide an address space, `map`/`unmap`, or protection. Owning our own page tables is the next
//! step in this milestone and is not claimed here.

use bootloader_api::BootInfo;
use core::arch::asm;
use core::fmt;
use physmem::FrameAllocator;
use uart16550::SerialPort;
use x86paging::{ENTRY_ADDRESS_MASK, TranslateError};

use crate::write_line;

/// Why a translation failed, including the case the `x86paging` walk cannot know about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// The bootloader did not expose a physical-memory mapping, so paging-structure entries cannot
    /// be read at all.
    NoPhysicalMemoryOffset,
    /// The page-table walk itself failed.
    Walk(TranslateError),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::NoPhysicalMemoryOffset => {
                write!(f, "the bootloader exposed no physical-memory offset")
            }
            Failure::Walk(err) => write!(f, "{err}"),
        }
    }
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
    let offset = match boot_info.physical_memory_offset.as_ref().copied() {
        Some(offset) => offset,
        None => return Err(Failure::NoPhysicalMemoryOffset),
    };
    // CR3's low bits are flags, and bits 52..63 are reserved in 4-level paging, so masking with the
    // entry address mask leaves the PML4 base.
    let pml4 = read_cr3() & ENTRY_ADDRESS_MASK;
    x86paging::translate(pml4, virt, |physical| read_physical(offset, physical))
        .map_err(Failure::Walk)
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
