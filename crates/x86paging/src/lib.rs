//! x86-64 paging: virtual-address decomposition, page-table entry decoding, and a four-level
//! address-translation walk.
//!
//! # Scope (milestone 2, virtual-memory slice)
//!
//! This crate turns a virtual address into a physical one by walking the page tables that are
//! already installed, given a way to read physical memory. The walk is generic: the caller supplies a
//! closure that returns the 8-byte entry at a physical address, so the whole thing is pure and is
//! unit tested on the host against a synthetic table built in memory.
//!
//! It handles 4 KiB, 2 MiB and 1 GiB mappings (the `PS`/huge-page bit at the PD and PDPT levels) and
//! reports which level failed when an address is not mapped.
//!
//! # Deliberately absent
//!
//! This crate only *reads* paging structures. It does not allocate, create, modify or install page
//! tables, does not handle 5-level paging, does not touch `CR3`, and does not decode the
//! accessed/dirty/NX or protection-key bits beyond what translation needs. Address spaces,
//! mapping/unmapping and switching to tables of our own are not implemented here and are not claimed.

// The kernel is `no_std`. Host `cargo test` needs `std` for the test harness, so `no_std` applies
// only when the crate is not being compiled for tests (same convention as the other crates).
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_op_in_unsafe_fn)]

/// Size of a base page.
pub const PAGE_SIZE: u64 = 4096;

/// Mask for the in-page offset.
pub const PAGE_OFFSET_MASK: u64 = PAGE_SIZE - 1;

/// Entry bit 0: the entry is present.
pub const ENTRY_PRESENT: u64 = 1 << 0;

/// Entry bit 1: the mapping is writable.
pub const ENTRY_WRITABLE: u64 = 1 << 1;

/// Entry bit 2: the mapping is accessible from user mode.
pub const ENTRY_USER: u64 = 1 << 2;

/// Entry bit 7: "page size" — a huge page in a PDPT (1 GiB) or PD (2 MiB) entry.
pub const ENTRY_HUGE: u64 = 1 << 7;

/// Mask of the physical address bits of a paging-structure entry (bits 12..51).
pub const ENTRY_ADDRESS_MASK: u64 = 0x000F_FFFF_FFFF_F000;

/// Mask of the base address bits of a 1 GiB PDPT entry (bits 30..51).
pub const HUGE_1GIB_ADDRESS_MASK: u64 = 0x000F_FFFF_C000_0000;

/// Mask of the base address bits of a 2 MiB PD entry (bits 21..51).
pub const HUGE_2MIB_ADDRESS_MASK: u64 = 0x000F_FFFF_FFE0_0000;

/// Mask of the offset within a 1 GiB page (bits 0..29).
pub const HUGE_1GIB_OFFSET_MASK: u64 = 0x3FFF_FFFF;

/// Mask of the offset within a 2 MiB page (bits 0..20).
pub const HUGE_2MIB_OFFSET_MASK: u64 = 0x1F_FFFF;

/// Highest address in the lower canonical half of the 48-bit (4-level) address space.
pub const LOWER_CANONICAL_MAX: u64 = 0x0000_7FFF_FFFF_FFFF;

/// The four levels of the x86-64 4-level paging hierarchy, top down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Page-map level 4 (the table `CR3` points at).
    Pml4,
    /// Page-directory pointer table.
    Pdpt,
    /// Page directory.
    Pd,
    /// Page table.
    Pt,
}

impl Level {
    /// Short name for diagnostics, e.g. `PML4`.
    pub const fn name(self) -> &'static str {
        match self {
            Level::Pml4 => "PML4",
            Level::Pdpt => "PDPT",
            Level::Pd => "PD",
            Level::Pt => "PT",
        }
    }
}

/// Why a virtual address could not be translated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslateError {
    /// The address is not canonical for 4-level paging.
    NotCanonical,
    /// No present entry was found at this level.
    NotMapped(Level),
}

impl core::fmt::Display for TranslateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TranslateError::NotCanonical => write!(f, "not a canonical 4-level address"),
            TranslateError::NotMapped(level) => {
                write!(f, "no present entry at the {} level", level.name())
            }
        }
    }
}

/// Whether `virt` is canonical under 4-level paging: bits 63..47 must all equal bit 47.
pub const fn is_canonical(virt: u64) -> bool {
    let sign = virt >> 47;
    sign == 0 || sign == 0x1_FFFF
}

/// Splits `virt` into its `(PML4, PDPT, PD, PT)` table indices.
pub const fn indices(virt: u64) -> [u16; 4] {
    [
        ((virt >> 39) & 0x1FF) as u16,
        ((virt >> 30) & 0x1FF) as u16,
        ((virt >> 21) & 0x1FF) as u16,
        ((virt >> 12) & 0x1FF) as u16,
    ]
}

/// Translates `virt` by walking four levels, starting at the table physically at `pml4_phys`.
///
/// `read` must return the 8-byte entry at a given physical address. Only present entries are
/// followed, so a walk over a well-formed table never dereferences an address the caller has not
/// already been given as a table base.
pub fn translate<F: Fn(u64) -> u64>(
    pml4_phys: u64,
    virt: u64,
    read: F,
) -> Result<u64, TranslateError> {
    if !is_canonical(virt) {
        return Err(TranslateError::NotCanonical);
    }
    let [pml4_index, pdpt_index, pd_index, pt_index] = indices(virt);

    let pml4_entry = read(pml4_phys + u64::from(pml4_index) * 8);
    if pml4_entry & ENTRY_PRESENT == 0 {
        return Err(TranslateError::NotMapped(Level::Pml4));
    }

    let pdpt_phys = pml4_entry & ENTRY_ADDRESS_MASK;
    let pdpt_entry = read(pdpt_phys + u64::from(pdpt_index) * 8);
    if pdpt_entry & ENTRY_PRESENT == 0 {
        return Err(TranslateError::NotMapped(Level::Pdpt));
    }
    if pdpt_entry & ENTRY_HUGE != 0 {
        // A 1 GiB page: bits 30..51 are the base, bits 0..29 the offset.
        return Ok((pdpt_entry & HUGE_1GIB_ADDRESS_MASK) + (virt & HUGE_1GIB_OFFSET_MASK));
    }

    let pd_phys = pdpt_entry & ENTRY_ADDRESS_MASK;
    let pd_entry = read(pd_phys + u64::from(pd_index) * 8);
    if pd_entry & ENTRY_PRESENT == 0 {
        return Err(TranslateError::NotMapped(Level::Pd));
    }
    if pd_entry & ENTRY_HUGE != 0 {
        // A 2 MiB page: bits 21..51 are the base, bits 0..20 the offset.
        return Ok((pd_entry & HUGE_2MIB_ADDRESS_MASK) + (virt & HUGE_2MIB_OFFSET_MASK));
    }

    let pt_phys = pd_entry & ENTRY_ADDRESS_MASK;
    let pt_entry = read(pt_phys + u64::from(pt_index) * 8);
    if pt_entry & ENTRY_PRESENT == 0 {
        return Err(TranslateError::NotMapped(Level::Pt));
    }
    Ok((pt_entry & ENTRY_ADDRESS_MASK) + (virt & PAGE_OFFSET_MASK))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A synthetic physical address space: entries present in the map, zero (not present) otherwise.
    struct FakeMemory {
        entries: HashMap<u64, u64>,
    }

    impl FakeMemory {
        fn new() -> Self {
            Self {
                entries: HashMap::new(),
            }
        }

        fn set_entry(&mut self, table_phys: u64, index: u16, value: u64) {
            self.entries
                .insert(table_phys + u64::from(index) * 8, value);
        }

        fn read(&self, phys: u64) -> u64 {
            *self.entries.get(&phys).unwrap_or(&0)
        }
    }

    const PML4: u64 = 0x1000;
    const PDPT: u64 = 0x2000;
    const PD: u64 = 0x3000;
    const PT: u64 = 0x4000;

    fn four_level_tables() -> FakeMemory {
        let mut memory = FakeMemory::new();
        memory.set_entry(PML4, 0, PDPT | ENTRY_PRESENT | ENTRY_WRITABLE);
        memory.set_entry(PDPT, 0, PD | ENTRY_PRESENT | ENTRY_WRITABLE);
        memory.set_entry(PD, 0, PT | ENTRY_PRESENT | ENTRY_WRITABLE);
        memory.set_entry(PT, 0, 0x5000 | ENTRY_PRESENT | ENTRY_WRITABLE);
        memory
    }

    #[test]
    fn splits_a_virtual_address_into_four_indices() {
        // Bit patterns chosen so each index is distinct and visible.
        let virt = (0x123_u64 << 39) | (0x45_u64 << 30) | (0x67_u64 << 21) | (0x89_u64 << 12);
        assert_eq!(indices(virt), [0x123, 0x45, 0x67, 0x89]);
        assert_eq!(indices(0), [0, 0, 0, 0]);
    }

    #[test]
    fn recognises_canonical_and_non_canonical_addresses() {
        assert!(is_canonical(0));
        assert!(is_canonical(LOWER_CANONICAL_MAX));
        assert!(is_canonical(0xFFFF_8000_0000_0000));
        assert!(is_canonical(u64::MAX));
        assert!(!is_canonical(0x0000_8000_0000_0000));
        assert!(!is_canonical(0x0000_FFFF_FFFF_FFFF));
    }

    #[test]
    fn translates_a_4kib_mapping() {
        let memory = four_level_tables();
        // 0x0234 has PT index 0 (which the fixture maps) and in-page offset 0x234.
        let translated = translate(PML4, 0x0234, |p| memory.read(p)).unwrap();
        assert_eq!(translated, 0x5234);
    }

    #[test]
    fn translates_a_2mib_huge_page() {
        let mut memory = four_level_tables();
        // PD entry 1 is a 2 MiB page at 0x20_0000.
        memory.set_entry(
            PD,
            1,
            0x20_0000 | ENTRY_PRESENT | ENTRY_WRITABLE | ENTRY_HUGE,
        );
        let virt = 1_u64 << 21; // PD index 1, everything else zero
        let translated = translate(PML4, virt + 0x1234, |p| memory.read(p)).unwrap();
        assert_eq!(translated, 0x20_0000 + 0x1234);
    }

    #[test]
    fn translates_a_1gib_huge_page() {
        let mut memory = four_level_tables();
        // PDPT entry 1 is a 1 GiB page at 0x4000_0000.
        memory.set_entry(
            PDPT,
            1,
            0x4000_0000 | ENTRY_PRESENT | ENTRY_WRITABLE | ENTRY_HUGE,
        );
        let virt = 1_u64 << 30; // PDPT index 1
        let translated = translate(PML4, virt + 0x5678, |p| memory.read(p)).unwrap();
        assert_eq!(translated, 0x4000_0000 + 0x5678);
    }

    #[test]
    fn reports_the_level_that_is_not_present() {
        let memory = four_level_tables();
        // Nothing at PML4 index 1.
        assert_eq!(
            translate(PML4, 1_u64 << 39, |p| memory.read(p)),
            Err(TranslateError::NotMapped(Level::Pml4))
        );
        // PML4 entry present, PDPT entry absent.
        assert_eq!(
            translate(PML4, 1_u64 << 30, |p| memory.read(p)),
            Err(TranslateError::NotMapped(Level::Pdpt))
        );
        // Down to the PD, then a missing PT entry.
        assert_eq!(
            translate(PML4, 1_u64 << 12, |p| memory.read(p)),
            Err(TranslateError::NotMapped(Level::Pt))
        );
    }

    #[test]
    fn rejects_non_canonical_addresses_before_walking() {
        let mut memory = FakeMemory::new();
        // A table entry that would translate if it were ever read; it must not be.
        memory.set_entry(PML4, 0, PDPT | ENTRY_PRESENT);
        // Non-canonical: PML4 index would be 0 but the address is invalid.
        let non_canonical = (0x1000_u64 << 48) | (1_u64 << 12);
        assert_eq!(
            translate(PML4, non_canonical, |p| memory.read(p)),
            Err(TranslateError::NotCanonical)
        );
    }

    #[test]
    fn ignores_non_address_flag_bits_and_uses_the_high_half() {
        let mut memory = FakeMemory::new();
        let high_half = 0xFFFF_8000_0000_0000_u64;
        let [pml4_index, pdpt_index, pd_index, pt_index] = indices(high_half + 0x2000);
        let table = 0x9000;
        memory.set_entry(
            PML4,
            pml4_index,
            table | ENTRY_PRESENT | ENTRY_WRITABLE | ENTRY_USER,
        );
        // A 1 GiB page at 0x4000_0000 (bit 30 is the lowest address bit such a page can use).
        memory.set_entry(table, pdpt_index, 0x4000_0000 | ENTRY_PRESENT | ENTRY_HUGE);
        assert_eq!(pd_index, 0);
        assert_eq!(pt_index, 2);
        let translated = translate(PML4, high_half + 0x2000, |p| memory.read(p)).unwrap();
        assert_eq!(translated, 0x4000_0000 + 0x2000);
    }
}
