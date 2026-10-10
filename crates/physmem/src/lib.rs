//! Physical memory handling for the New OS kernel: validation of the firmware-provided boot memory
//! map and a bootstrap physical frame allocator.
//!
//! # Scope (milestone 2, first slice)
//!
//! - Validate the memory map the bootloader hands over, using checked arithmetic, *before* anything
//!   in the kernel trusts it.
//! - Track physical frames as free or not-free in a caller-provided bitmap and hand out single
//!   frames.
//! - Report defects as values instead of panicking, so the kernel can print a diagnostic and take
//!   its controlled fatal path.
//!
//! # Deliberately absent
//!
//! These are not implemented and must not be claimed:
//!
//! - **No virtual memory**: no page tables, no address spaces, no mapping API. The kernel still runs
//!   on the page tables the `bootloader` component created.
//! - **No deallocation**: an allocated frame cannot be returned yet. One bit per frame cannot
//!   distinguish "allocated" from "reserved", and inventing an ownership model before the
//!   page-table work that needs it would be guesswork. This is a bootstrap allocator.
//! - **No reclamation** of `Reclaimable` regions: bootloader memory is treated as non-usable until
//!   the kernel can prove the bootloader no longer needs it.
//! - **No NUMA, huge-page splitting, memory hotplug or swap.**
//!
//! All of the logic here is safe Rust and host-testable: it performs no port or memory-mapped I/O
//! and touches no hardware, so the arithmetic and the bitmap policies are unit tested on the host
//! and only the wiring is exercised in QEMU.

// The kernel is `no_std`. Host `cargo test` needs `std` for the test harness, so `no_std` applies
// only when the crate is not being compiled for tests (same convention as `uart16550`).
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_op_in_unsafe_fn)]

use core::fmt;

/// Physical memory region kinds the kernel distinguishes.
///
/// Firmware reports many kinds; the kernel only needs to know whether a region may be handed out,
/// may become free later, or may not be used at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// Free RAM that the allocator may hand out.
    Usable,
    /// Memory the firmware/bootloader used that *may* become usable later, once the kernel can
    /// prove it is no longer needed. Treated as non-usable for now.
    Reclaimable,
    /// Everything else: kernel image and stack, framebuffer, ACPI and firmware structures, MMIO
    /// holes, and any kind the kernel does not recognise. Never handed out.
    Reserved,
}

/// A physical memory region as reported by firmware via the bootloader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    /// First physical byte address of the region.
    pub start: u64,
    /// Length of the region in bytes. A region may start or end mid-frame; the allocator handles
    /// that conservatively.
    pub len: u64,
    /// What the region may be used for.
    pub kind: RegionKind,
}

impl MemoryRegion {
    /// Builds a region from the start/end pair firmware reports, where `end` is exclusive.
    ///
    /// # Errors
    ///
    /// Returns [`BoundsError`] if `end < start`: such a pair cannot describe a region, and
    /// subtracting the addresses directly would wrap and describe a huge one.
    pub const fn from_bounds(start: u64, end: u64, kind: RegionKind) -> Result<Self, BoundsError> {
        match end.checked_sub(start) {
            Some(len) => Ok(Self { start, len, kind }),
            None => Err(BoundsError { start, end }),
        }
    }
}

/// A start/end pair that cannot describe a memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundsError {
    /// The start address that was given.
    pub start: u64,
    /// The end address that was given, which precedes `start`.
    pub end: u64,
}

impl fmt::Display for BoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "memory region {:#018x}..{:#018x} ends before it starts",
            self.start, self.end
        )
    }
}

/// A defect that makes the reported memory map unsafe to use.
///
/// Defects are fatal by design: a map containing them cannot be trusted to describe real memory,
/// and guessing the intended values could hand out firmware or MMIO memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapDefect {
    /// A region was reported with a length of zero bytes.
    ZeroLength {
        /// Position of the region in the order it was reported: zero-based, counting every region
        /// added, including previously rejected ones.
        index: usize,
    },
    /// `start + len` does not fit in 64 bits, so the region's end address is not representable.
    AddressOverflow {
        /// Position of the region in the order it was reported.
        index: usize,
    },
}

impl fmt::Display for MapDefect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapDefect::ZeroLength { index } => {
                write!(f, "memory region #{index} has zero length")
            }
            MapDefect::AddressOverflow { index } => write!(
                f,
                "memory region #{index} ends beyond the 64-bit physical address space"
            ),
        }
    }
}

/// Totals over the accepted regions of a memory map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MapSummary {
    /// Number of regions accepted (every added region that was not rejected as defective).
    pub regions: usize,
    /// Total bytes reported as [`RegionKind::Usable`].
    pub usable_bytes: u64,
    /// Total bytes reported as [`RegionKind::Reclaimable`].
    pub reclaimable_bytes: u64,
    /// Total bytes reported as [`RegionKind::Reserved`].
    pub reserved_bytes: u64,
    /// Highest end address of any accepted region, or `0` if there were none.
    pub highest_end: u64,
    /// `true` if a byte total saturated at [`u64::MAX`] instead of overflowing. A map that sets
    /// this is not necessarily invalid (each region passed its own checks), but the totals are then
    /// lower bounds and must not be reported as exact figures.
    pub totals_saturated: bool,
}

/// Validates a memory map one region at a time, without allocating.
///
/// The validator sees each region once, so it checks per-region properties (non-zero length,
/// representable end address) and accumulates totals. Conflicting claims *between* regions are not
/// detected here: they are resolved and reported by [`FrameAllocator`], where the effect on actual
/// frames is visible.
#[derive(Debug, Default)]
pub struct MapValidator {
    seen: usize,
    summary: MapSummary,
}

impl MapValidator {
    /// Creates a validator that has seen no regions.
    pub const fn new() -> Self {
        Self {
            seen: 0,
            summary: MapSummary {
                regions: 0,
                usable_bytes: 0,
                reclaimable_bytes: 0,
                reserved_bytes: 0,
                highest_end: 0,
                totals_saturated: false,
            },
        }
    }

    /// Validates and counts one region.
    ///
    /// # Errors
    ///
    /// Returns the [`MapDefect`] describing the first problem with `region`. A rejected region is
    /// not counted in the summary. The validator stays usable so a caller can inspect later regions
    /// too (reporting every defect in one boot is more useful than stopping at the first), but a
    /// caller that intends to use the map must treat any error as fatal.
    pub fn add(&mut self, region: MemoryRegion) -> Result<(), MapDefect> {
        let index = self.seen;
        self.seen += 1;

        if region.len == 0 {
            return Err(MapDefect::ZeroLength { index });
        }
        let Some(end) = region.start.checked_add(region.len) else {
            return Err(MapDefect::AddressOverflow { index });
        };

        let current = match region.kind {
            RegionKind::Usable => self.summary.usable_bytes,
            RegionKind::Reclaimable => self.summary.reclaimable_bytes,
            RegionKind::Reserved => self.summary.reserved_bytes,
        };
        let (total, saturated) = match current.checked_add(region.len) {
            Some(total) => (total, false),
            None => (u64::MAX, true),
        };
        match region.kind {
            RegionKind::Usable => self.summary.usable_bytes = total,
            RegionKind::Reclaimable => self.summary.reclaimable_bytes = total,
            RegionKind::Reserved => self.summary.reserved_bytes = total,
        }
        self.summary.totals_saturated |= saturated;

        self.summary.regions += 1;
        self.summary.highest_end = self.summary.highest_end.max(end);
        Ok(())
    }

    /// The totals over the regions accepted so far.
    pub const fn summary(&self) -> MapSummary {
        self.summary
    }

    /// Number of regions added, including those rejected as defective.
    pub const fn regions_seen(&self) -> usize {
        self.seen
    }
}

/// An error from a [`FrameAllocator`] operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// The requested frame size shift is outside `MIN_FRAME_SHIFT..=MAX_FRAME_SHIFT`.
    UnsupportedFrameSize,
    /// The region's end address (`start + len`) is not representable in 64 bits.
    AddressOverflow,
    /// A usable region was announced after a non-usable one.
    ///
    /// The allocator requires all usable claims first and all non-usable claims afterwards, so that
    /// a non-usable claim wins no matter what order the firmware listed regions in. Allowing a
    /// usable claim afterwards could silently re-enable firmware-reserved memory.
    UsableAfterNonUsable,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::UnsupportedFrameSize => write!(f, "unsupported frame size"),
            FrameError::AddressOverflow => {
                write!(f, "region end address is not representable in 64 bits")
            }
            FrameError::UsableAfterNonUsable => write!(
                f,
                "usable region announced after a non-usable one (all usable regions must come first)"
            ),
        }
    }
}

/// What a [`FrameAllocator::mark_usable`] call changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[must_use]
pub struct UsableReport {
    /// Frames newly marked usable (free).
    pub frames_marked: u64,
    /// Whole frames inside the region that lie beyond the allocator's bitmap and are therefore not
    /// managed. Non-zero means real memory is being ignored, which the caller should report.
    pub out_of_range_frames: u64,
}

/// What a [`FrameAllocator::mark_reserved`] call changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[must_use]
pub struct NonUsableReport {
    /// Frames that a previous usable claim had marked and that this non-usable claim has now
    /// cleared. Non-zero means the map contained overlapping usable and non-usable regions, and the
    /// non-usable claim won.
    pub frames_cleared: u64,
    /// Whole frames inside the region beyond the allocator's bitmap. See
    /// [`UsableReport::out_of_range_frames`].
    pub out_of_range_frames: u64,
}

/// Smallest frame size the allocator accepts: 2^12 bytes, the x86-64 base page size.
pub const MIN_FRAME_SHIFT: u32 = 12;

/// Largest frame size the allocator accepts: 2^21 bytes, the x86-64 large-page size.
pub const MAX_FRAME_SHIFT: u32 = 21;

/// A bitmap physical frame allocator, usable before any virtual memory exists.
///
/// One bit per frame: `1` means the frame is free (may be handed out), `0` means it may not. Every
/// frame starts as `0`, so the caller must mark usable regions before allocating; frames that are
/// never mentioned are never handed out.
///
/// # Call order
///
/// All usable regions must be marked before the first non-usable region (see
/// [`FrameError::UsableAfterNonUsable`]). The kernel therefore walks the map twice: once to mark
/// usable regions, once to reserve everything else.
///
/// # Conservative frame accounting
///
/// [`mark_usable`](Self::mark_usable) frees only whole frames lying *entirely* inside the region, so
/// a partially usable frame at a region boundary is never handed out.
/// [`mark_reserved`](Self::mark_reserved) clears every frame that *overlaps* the region, so a
/// partially reserved frame is never handed out either. Both directions err towards withholding
/// memory, which is the safe direction.
pub struct FrameAllocator<'a> {
    bitmap: &'a mut [u64],
    frame_shift: u32,
    /// Index the next allocation search starts from, so sequential allocations stay cheap.
    next_frame: u64,
    allocated: u64,
    saw_non_usable: bool,
}

impl<'a> FrameAllocator<'a> {
    /// Creates an allocator over `bitmap`, whose bits map one-to-one onto frames of
    /// `2^frame_shift` bytes starting at physical address 0.
    ///
    /// The bitmap is not zeroed here: the caller owns the storage and must pass a zeroed bitmap
    /// (every frame starts out not free). A kernel bitmap lives in a `static` that is zero-filled
    /// by the loader's `.bss` handling, which is why this type does not clear it.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::UnsupportedFrameSize`] unless `frame_shift` is within
    /// `MIN_FRAME_SHIFT..=MAX_FRAME_SHIFT`.
    pub fn new(bitmap: &'a mut [u64], frame_shift: u32) -> Result<Self, FrameError> {
        if !(MIN_FRAME_SHIFT..=MAX_FRAME_SHIFT).contains(&frame_shift) {
            return Err(FrameError::UnsupportedFrameSize);
        }
        Ok(Self {
            bitmap,
            frame_shift,
            next_frame: 0,
            allocated: 0,
            saw_non_usable: false,
        })
    }

    /// Size of one frame in bytes.
    pub const fn frame_size(&self) -> u64 {
        1 << self.frame_shift
    }

    /// Number of frames this allocator can represent at all, free or not.
    pub fn capacity_frames(&self) -> u64 {
        self.bitmap.len() as u64 * 64
    }

    /// Number of frames currently free.
    pub fn free_frames(&self) -> u64 {
        self.bitmap
            .iter()
            .map(|word| u64::from(word.count_ones()))
            .sum()
    }

    /// Number of frames handed out by [`allocate`](Self::allocate) so far.
    pub const fn allocated_frames(&self) -> u64 {
        self.allocated
    }

    /// Whether the frame containing `address` is free.
    ///
    /// An address outside the allocator's range is not free.
    pub fn is_free(&self, address: u64) -> bool {
        let index = address / self.frame_size();
        index < self.capacity_frames() && self.bit_is_set(index)
    }

    /// Marks every whole frame fully inside `[start, start + len)` as usable (free).
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::AddressOverflow`] if `start + len` is not representable, or
    /// [`FrameError::UsableAfterNonUsable`] if a non-usable region was already announced. In both
    /// cases the bitmap and the allocator's state are left unchanged.
    pub fn mark_usable(&mut self, start: u64, len: u64) -> Result<UsableReport, FrameError> {
        if self.saw_non_usable {
            return Err(FrameError::UsableAfterNonUsable);
        }
        let (first, last) = self.whole_frame_span(start, len)?;

        let capacity = self.capacity_frames();
        let mut report = UsableReport {
            frames_marked: 0,
            out_of_range_frames: last.saturating_sub(first.max(capacity)),
        };

        let mut index = first;
        while index < last && index < capacity {
            if !self.bit_is_set(index) {
                self.set_bit(index);
                report.frames_marked += 1;
            }
            index += 1;
        }
        Ok(report)
    }

    /// Clears every frame that overlaps `[start, start + len)`, so it cannot be handed out.
    ///
    /// Used for reserved, reclaimable and unrecognised regions. Call it after all usable regions
    /// have been marked. A zero-length range clears nothing.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::AddressOverflow`] if `start + len` is not representable, in which case
    /// the bitmap and the allocator's state are left unchanged.
    pub fn mark_reserved(&mut self, start: u64, len: u64) -> Result<NonUsableReport, FrameError> {
        let (first, last) = self.overlapping_frame_span(start, len)?;
        self.saw_non_usable = true;

        let capacity = self.capacity_frames();
        let mut report = NonUsableReport {
            frames_cleared: 0,
            out_of_range_frames: last.saturating_sub(first.max(capacity)),
        };

        let mut index = first;
        while index < last && index < capacity {
            if self.bit_is_set(index) {
                self.clear_bit(index);
                report.frames_cleared += 1;
            }
            index += 1;
        }
        Ok(report)
    }

    /// Hands out the first free frame at or above the allocation cursor and returns its physical
    /// address, or `None` if no free frame remains.
    ///
    /// The frame is no longer free afterwards. There is no deallocation in this milestone; see the
    /// crate documentation.
    pub fn allocate(&mut self) -> Option<u64> {
        let capacity = self.capacity_frames();
        let mut index = self.next_frame;
        while index < capacity {
            if self.bit_is_set(index) {
                self.clear_bit(index);
                self.allocated += 1;
                self.next_frame = index + 1;
                return index.checked_mul(self.frame_size());
            }
            index += 1;
        }
        self.next_frame = capacity;
        None
    }

    /// Frame indexes for the whole frames entirely inside `[start, start + len)`.
    ///
    /// The first index is rounded up and the last rounded down, so partially covered frames at
    /// either end are excluded.
    fn whole_frame_span(&self, start: u64, len: u64) -> Result<(u64, u64), FrameError> {
        let end = start.checked_add(len).ok_or(FrameError::AddressOverflow)?;
        let size = self.frame_size();
        let first = start.div_ceil(size);
        let last = (end / size).max(first);
        Ok((first, last))
    }

    /// Frame indexes for every frame overlapping `[start, start + len)`.
    ///
    /// The first index is rounded down and the last rounded up, so partially covered frames at
    /// either end are included. A zero-length range covers nothing.
    fn overlapping_frame_span(&self, start: u64, len: u64) -> Result<(u64, u64), FrameError> {
        let end = start.checked_add(len).ok_or(FrameError::AddressOverflow)?;
        let size = self.frame_size();
        let first = start / size;
        let last = if end == start {
            first
        } else {
            end.div_ceil(size)
        };
        Ok((first, last))
    }

    /// Whether the bit for frame `index` is set. `index` must be below the capacity.
    fn bit_is_set(&self, index: u64) -> bool {
        let word = (index / 64) as usize;
        let bit = (index % 64) as u32;
        match self.bitmap.get(word) {
            Some(value) => value & (1_u64 << bit) != 0,
            None => false,
        }
    }

    /// Sets the bit for frame `index`. `index` must be below the capacity.
    fn set_bit(&mut self, index: u64) {
        let word = (index / 64) as usize;
        let bit = (index % 64) as u32;
        if let Some(value) = self.bitmap.get_mut(word) {
            *value |= 1_u64 << bit;
        }
    }

    /// Clears the bit for frame `index`. `index` must be below the capacity.
    fn clear_bit(&mut self, index: u64) {
        let word = (index / 64) as usize;
        let bit = (index % 64) as u32;
        if let Some(value) = self.bitmap.get_mut(word) {
            *value &= !(1_u64 << bit);
        }
    }
}

impl fmt::Debug for FrameAllocator<'_> {
    /// Deliberately does not print the bitmap, which can be hundreds of kilobytes; use
    /// [`FrameAllocator::capacity_frames`] and [`FrameAllocator::free_frames`] for state.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameAllocator")
            .field("frame_size", &self.frame_size())
            .field("capacity_frames", &self.capacity_frames())
            .field("free_frames", &self.free_frames())
            .field("allocated_frames", &self.allocated)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usable(start: u64, len: u64) -> MemoryRegion {
        MemoryRegion {
            start,
            len,
            kind: RegionKind::Usable,
        }
    }

    fn reserved(start: u64, len: u64) -> MemoryRegion {
        MemoryRegion {
            start,
            len,
            kind: RegionKind::Reserved,
        }
    }

    /// A one-word bitmap: 64 frames of 4 KiB, covering the first 256 KiB of physical memory.
    const ONE_WORD_FRAMES: u64 = 64;

    #[test]
    fn regions_can_be_built_from_firmware_bounds() {
        assert_eq!(
            MemoryRegion::from_bounds(0x1000, 0x2000, RegionKind::Usable),
            Ok(usable(0x1000, 0x1000))
        );
        // An empty pair is legal here and is rejected later by the validator, which keeps
        // "is this a valid range" and "is this a usable region" separate decisions.
        assert_eq!(
            MemoryRegion::from_bounds(0x1000, 0x1000, RegionKind::Reserved),
            Ok(reserved(0x1000, 0))
        );
        // An end before the start must not wrap into a huge region.
        let error = MemoryRegion::from_bounds(0x2000, 0x1000, RegionKind::Usable).unwrap_err();
        assert_eq!(
            error,
            BoundsError {
                start: 0x2000,
                end: 0x1000
            }
        );
        assert_eq!(
            error.to_string(),
            "memory region 0x0000000000002000..0x0000000000001000 ends before it starts"
        );
    }

    #[test]
    fn validator_accepts_a_typical_map_and_sums_each_kind() {
        let mut validator = MapValidator::new();
        assert_eq!(validator.add(usable(0x1000, 0x9_F000)), Ok(()));
        assert_eq!(validator.add(reserved(0x0, 0x1000)), Ok(()));
        assert_eq!(validator.add(usable(0x10_0000, 0x1000)), Ok(()));

        let summary = validator.summary();
        assert_eq!(summary.regions, 3);
        assert_eq!(summary.usable_bytes, 0x9_F000 + 0x1000);
        assert_eq!(summary.reserved_bytes, 0x1000);
        assert_eq!(summary.reclaimable_bytes, 0);
        assert_eq!(summary.highest_end, 0x10_1000);
        assert!(!summary.totals_saturated);
        assert_eq!(validator.regions_seen(), 3);
    }

    #[test]
    fn validator_counts_reclaimable_regions_separately() {
        let mut validator = MapValidator::new();
        assert_eq!(
            validator.add(MemoryRegion {
                start: 0x1000,
                len: 0x2000,
                kind: RegionKind::Reclaimable,
            }),
            Ok(())
        );
        assert_eq!(validator.summary().reclaimable_bytes, 0x2000);
        assert_eq!(validator.summary().usable_bytes, 0);
    }

    #[test]
    fn validator_rejects_zero_length_regions_without_counting_them() {
        let mut validator = MapValidator::new();
        assert_eq!(
            validator.add(usable(0x1000, 0)),
            Err(MapDefect::ZeroLength { index: 0 })
        );
        assert_eq!(validator.add(usable(0x1000, 1)), Ok(()));

        assert_eq!(validator.summary().regions, 1);
        assert_eq!(validator.summary().usable_bytes, 1);
        assert_eq!(
            validator.regions_seen(),
            2,
            "rejected regions are still seen, so indexes stay meaningful"
        );
    }

    #[test]
    fn validator_rejects_regions_whose_end_address_overflows() {
        let mut validator = MapValidator::new();
        assert_eq!(
            validator.add(usable(u64::MAX - 1, 2)),
            Err(MapDefect::AddressOverflow { index: 0 })
        );
        // A region ending exactly at the top of the address space is representable.
        assert_eq!(validator.add(usable(u64::MAX - 1, 1)), Ok(()));
        assert_eq!(validator.summary().highest_end, u64::MAX);
        assert_eq!(validator.summary().regions, 1);
    }

    #[test]
    fn validator_saturates_byte_totals_and_says_so() {
        // A well-formed, non-overlapping map of a 64-bit address space can never total more than
        // u64::MAX, so saturation is a defensive guard against a malformed map. Here the same huge
        // region is reported twice; the totals become lower bounds and the caller is told.
        let mut validator = MapValidator::new();
        let half = u64::MAX / 2 + 1;
        assert_eq!(validator.add(usable(0, half)), Ok(()));
        assert_eq!(validator.add(usable(0, half)), Ok(()));

        let summary = validator.summary();
        assert_eq!(summary.regions, 2);
        assert_eq!(summary.usable_bytes, u64::MAX, "the total is a lower bound");
        assert!(summary.totals_saturated, "saturation must be reported");
        assert_eq!(summary.highest_end, half);
    }

    #[test]
    fn validator_reports_defects_with_a_readable_message() {
        assert_eq!(
            MapDefect::ZeroLength { index: 3 }.to_string(),
            "memory region #3 has zero length"
        );
        assert_eq!(
            MapDefect::AddressOverflow { index: 7 }.to_string(),
            "memory region #7 ends beyond the 64-bit physical address space"
        );
    }

    #[test]
    fn allocator_rejects_unsupported_frame_sizes() {
        let mut bitmap = [0_u64; 4];
        for rejected in [0, MIN_FRAME_SHIFT - 1, MAX_FRAME_SHIFT + 1] {
            assert_eq!(
                FrameAllocator::new(&mut bitmap, rejected).unwrap_err(),
                FrameError::UnsupportedFrameSize,
                "frame shift {rejected} must be rejected"
            );
        }
        assert!(FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).is_ok());
        assert!(FrameAllocator::new(&mut bitmap, MAX_FRAME_SHIFT).is_ok());
    }

    #[test]
    fn allocator_does_not_clear_the_bitmap_it_is_given() {
        // The kernel keeps its bitmap in `.bss`, so `new` must not assume it can spend time
        // clearing it, and must not treat existing bits as anything but the caller's decision.
        let mut bitmap = [u64::MAX; 1];
        let allocator = FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).unwrap();
        assert_eq!(allocator.capacity_frames(), ONE_WORD_FRAMES);
        assert_eq!(allocator.free_frames(), ONE_WORD_FRAMES);
        assert_eq!(allocator.allocated_frames(), 0);
    }

    #[test]
    fn allocator_marks_only_whole_frames_usable() {
        let mut bitmap = [0_u64; 1];
        let mut allocator = FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).unwrap();

        // Exactly frame 1.
        assert_eq!(
            allocator.mark_usable(0x1000, 0x1000).unwrap(),
            UsableReport {
                frames_marked: 1,
                out_of_range_frames: 0
            }
        );

        // 0x2F00..0x5000 starts inside frame 2 and ends inside frame 5, so frames 3 and 4 are
        // fully covered and only those two are freed.
        assert_eq!(
            allocator.mark_usable(0x2F00, 0x2100).unwrap(),
            UsableReport {
                frames_marked: 2,
                out_of_range_frames: 0
            }
        );
        assert_eq!(allocator.free_frames(), 3);

        // A range smaller than one frame that lies inside frame 7 frees nothing.
        assert_eq!(
            allocator.mark_usable(0x7F00, 0x10).unwrap().frames_marked,
            0
        );
        assert_eq!(allocator.free_frames(), 3);
    }

    #[test]
    fn allocator_marks_a_whole_frame_span_only_once() {
        let mut bitmap = [0_u64; 1];
        let mut allocator = FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).unwrap();
        assert_eq!(allocator.mark_usable(0x0, 0x3000).unwrap().frames_marked, 3);
        // Overlapping the same region again must not double count.
        assert_eq!(
            allocator.mark_usable(0x1000, 0x2000).unwrap().frames_marked,
            0
        );
        assert_eq!(allocator.free_frames(), 3);
    }

    #[test]
    fn allocator_reports_frames_beyond_the_bitmap() {
        let mut bitmap = [0_u64; 1];
        let mut allocator = FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).unwrap();

        let report = allocator.mark_usable(0, 0x1000 * 100).unwrap();
        assert_eq!(report.frames_marked, ONE_WORD_FRAMES);
        assert_eq!(
            report.out_of_range_frames,
            100 - ONE_WORD_FRAMES,
            "memory above the bitmap must be reported, not silently dropped"
        );

        // A region entirely beyond the bitmap marks nothing and is entirely out of range.
        let report = allocator.mark_usable(0x1000 * 100, 0x1000 * 4).unwrap();
        assert_eq!(report.frames_marked, 0);
        assert_eq!(report.out_of_range_frames, 4);
    }

    #[test]
    fn allocator_hands_out_frames_in_order_and_then_reports_exhaustion() {
        let mut bitmap = [0_u64; 1];
        let mut allocator = FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).unwrap();
        assert_eq!(
            allocator
                .mark_usable(0x1000, 0x1000 * 2)
                .unwrap()
                .frames_marked,
            2
        );

        assert_eq!(allocator.allocate(), Some(0x1000));
        assert_eq!(allocator.allocate(), Some(0x2000));
        assert_eq!(allocator.allocate(), None, "only two frames were marked");
        assert_eq!(allocator.allocated_frames(), 2);
        assert_eq!(allocator.free_frames(), 0);
        assert!(!allocator.is_free(0x1000));
    }

    #[test]
    fn non_usable_regions_override_usable_ones_they_overlap() {
        let mut bitmap = [0_u64; 1];
        let mut allocator = FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).unwrap();
        assert_eq!(
            allocator
                .mark_usable(0x0, 0x1000 * 4)
                .unwrap()
                .frames_marked,
            4
        );
        assert_eq!(allocator.free_frames(), 4);

        // A reserved region straddling frames 1 and 2 clears both, including the partial overlap.
        let report = allocator.mark_reserved(0x1800, 0x1000).unwrap();
        assert_eq!(report.frames_cleared, 2);
        assert_eq!(allocator.free_frames(), 2);

        assert_eq!(allocator.allocate(), Some(0x0));
        assert_eq!(allocator.allocate(), Some(0x3000));
        assert_eq!(allocator.allocate(), None);
    }

    #[test]
    fn a_non_usable_region_outside_the_usable_set_clears_nothing() {
        let mut bitmap = [0_u64; 1];
        let mut allocator = FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).unwrap();
        assert_eq!(
            allocator.mark_usable(0x1000, 0x1000).unwrap().frames_marked,
            1
        );
        assert_eq!(
            allocator
                .mark_reserved(0x9000, 0x1000)
                .unwrap()
                .frames_cleared,
            0
        );
        assert_eq!(allocator.free_frames(), 1);
    }

    #[test]
    fn allocator_refuses_a_usable_region_after_a_non_usable_one() {
        let mut bitmap = [0_u64; 1];
        let mut allocator = FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).unwrap();
        assert_eq!(
            allocator.mark_reserved(0x0, 0x1000).unwrap().frames_cleared,
            0
        );

        assert_eq!(
            allocator.mark_usable(0x1000, 0x1000).unwrap_err(),
            FrameError::UsableAfterNonUsable
        );
        assert_eq!(
            allocator.free_frames(),
            0,
            "the rejected call must not have changed the bitmap"
        );
    }

    #[test]
    fn allocator_rejects_overflowing_regions_without_changing_state() {
        let mut bitmap = [0_u64; 1];
        let mut allocator = FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).unwrap();
        assert_eq!(
            allocator.mark_usable(u64::MAX, 1).unwrap_err(),
            FrameError::AddressOverflow
        );
        assert_eq!(allocator.free_frames(), 0);

        // The failed call must not have flipped the allocator into its non-usable phase.
        assert_eq!(
            allocator.mark_usable(0x1000, 0x1000).unwrap().frames_marked,
            1
        );
        assert_eq!(allocator.free_frames(), 1);

        assert_eq!(
            allocator.mark_reserved(u64::MAX, 2).unwrap_err(),
            FrameError::AddressOverflow
        );
        assert_eq!(
            allocator.free_frames(),
            1,
            "a rejected reserve call must not clear anything"
        );
    }

    #[test]
    fn zero_length_regions_are_accepted_by_the_allocator_as_no_ops() {
        let mut bitmap = [0_u64; 1];
        let mut allocator = FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).unwrap();
        // The validator rejects zero-length regions, but the allocator is usable on its own and
        // must not turn an empty range into a claim on memory.
        assert_eq!(allocator.mark_usable(0x2000, 0).unwrap().frames_marked, 0);
        assert_eq!(
            allocator.mark_reserved(0x2000, 0).unwrap().frames_cleared,
            0
        );
        assert_eq!(allocator.free_frames(), 0);
    }

    #[test]
    fn is_free_rejects_addresses_outside_the_bitmap() {
        let mut bitmap = [0_u64; 1];
        let mut allocator = FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).unwrap();
        assert_eq!(
            allocator.mark_usable(0x1000, 0x1000).unwrap().frames_marked,
            1
        );
        assert!(allocator.is_free(0x1000));
        assert!(!allocator.is_free(0x2000));
        assert!(
            !allocator.is_free(u64::MAX),
            "no overflow in the index math"
        );
    }

    #[test]
    fn large_frames_use_the_requested_size() {
        let mut bitmap = [0_u64; 1];
        let mut allocator = FrameAllocator::new(&mut bitmap, MAX_FRAME_SHIFT).unwrap();
        assert_eq!(allocator.frame_size(), 2 * 1024 * 1024);

        assert_eq!(
            allocator
                .mark_usable(0x1000, 2 * 1024 * 1024)
                .unwrap()
                .frames_marked,
            0,
            "a 2 MiB frame is not fully covered by a region starting at 0x1000"
        );
        assert_eq!(allocator.free_frames(), 0);

        let mut bitmap = [0_u64; 1];
        let mut allocator = FrameAllocator::new(&mut bitmap, MAX_FRAME_SHIFT).unwrap();
        assert_eq!(
            allocator
                .mark_usable(0, 2 * 1024 * 1024)
                .unwrap()
                .frames_marked,
            1
        );
        assert_eq!(allocator.free_frames(), 1);
        assert_eq!(allocator.allocate(), Some(0));
    }

    #[test]
    fn debug_output_reports_state_without_dumping_the_bitmap() {
        let mut bitmap = [u64::MAX; 64];
        let allocator = FrameAllocator::new(&mut bitmap, MIN_FRAME_SHIFT).unwrap();
        let text = format!("{allocator:?}");
        assert!(text.contains("FrameAllocator"), "{text}");
        assert!(text.contains("free_frames: 4096"), "{text}");
        assert!(!text.contains("bitmap"), "{text}");
    }
}
