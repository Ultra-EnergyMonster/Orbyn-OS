# ADR-0018 — The kernel builds and installs its own top-level page table

**Status:** accepted (2026-10-11) on boot-experiment evidence: `docs/verification/M2-PAGETABLES-EVIDENCE.md`.
**Supersedes nothing.** Related: ADR-0003 (boot component), ADR-0014 (boot memory handling), ADR-0017
(read-only address translation).

## Context

The `bootloader` 0.11 component creates the page tables the kernel starts on and maps the kernel
image, the kernel stack, the framebuffer and — because `kernel/src/main.rs` requests it — all of
physical memory at a fixed offset. ADR-0017 used that mapping to *read* those tables, which let the
kernel reserve the stack and framebuffer frames by translating their virtual addresses.

Reading the bootloader's tables is not owning memory. Until the kernel has a page table of its own it
cannot create a mapping, cannot decide where anything lives, and cannot later confine a process to its
own address space. This is the M2 "virtual memory/page tables" item, and it must be reached without a
step that can brick the boot.

## Decision

`kernel/src/paging.rs::install_own_page_tables` builds a **new PML4** and installs it with `mov cr3`:

1. Allocate one frame from the kernel's own bitmap frame allocator for the new PML4.
2. **Copy all 512 entries** of the current PML4 (the base in `CR3`) into it. The copied entries still
   point at the bootloader's lower-level tables, so every existing mapping keeps working byte for byte.
3. Find the first free top-level slot from index 256 upward and build a fresh `PDPT -> PD -> PT`
   chain there from newly allocated frames, all explicitly zeroed (fresh frames are not guaranteed to
   hold zeros), ending in one newly allocated data frame.
4. Write a known magic value into that data frame through the bootloader's physical-memory mapping.
5. Load the new PML4 into `CR3`, then verify: `CR3` reads back unchanged, and reading the magic back
   **through the new virtual address** returns the value written.
6. Re-translate the kernel stack to prove the bootloader's mappings survived the switch.

The virtual address chosen for the first kernel mapping is `0xFFFF_8000_0000_0000` — PML4 slot 256 with
the three lower indices at zero. It is canonical (the upper half of the address space), and the
bootloader's mappings all live in the lower half (kernel image at slot 2, stack at slot 48,
framebuffer and the physical-memory offset at slot 0), so slot 256 is free. The free slot is found at
runtime rather than assumed, and a full upper half fails the operation instead of corrupting a table.

## Consequences

- The kernel now owns its top-level page table: after this call the address space is described by a
  frame this kernel allocated, not the bootloader's.
- The kernel can grow its address space without the bootloader, which is the prerequisite for an
  address space per process and for confining userland later.
- **Only the top level is ours.** The lower levels behind the mirrored entries are still the
  bootloader's tables; there is still no `map`/`unmap` of an arbitrary range, no per-region
  protection (everything stays writable and user-accessible as the bootloader left it), no
  deallocation of page-table frames, and no address space per process. Those are not claimed.
- Copying rather than replacing is deliberate: replacing the bootloader's mappings means recreating
  the kernel image, stack, framebuffer and physical-memory mappings first, which is a larger and much
  more dangerous change and is left as its own step.
- A failure is reported, not fatal: the kernel keeps running on the bootloader's table, and because
  the success marker is only emitted on success, the boot smoke test still fails rather than passing
  quietly.

## Alternatives considered

- **Keep running on the bootloader's table.** Simple, and enough for ADR-0017's read-only walk, but it
  blocks every later memory feature. Rejected as a resting place.
- **Build a PML4 from scratch with only the mappings the kernel needs now.** Correct in principle, but
  it requires reproducing the kernel image, stack, framebuffer and physical-memory mappings before the
  switch, and any mistake faults before there is a working handler for it. Rejected for this step as
  too dangerous to combine with the first `CR3` switch; the mirror first, the replacement later.
- **Keep the bootloader's PML4 frame and edit it in place.** Cheaper, but it is not ownership, and a
  table shared with the bootloader's structures cannot be reorganised later. Rejected.
