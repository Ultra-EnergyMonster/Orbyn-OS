# ADR-0014: Boot Memory Map Handling and the Bootstrap Frame Allocator

- **Status:** accepted
- **Date:** 2026-10-10
- **Owners:** Project owner / contributors
- **Supersedes:** None

## Context

The kernel runs on page tables created by the reused `bootloader` 0.11.17 component and receives a
firmware-derived memory map in `BootInfo`. That map is untrusted input: it comes from firmware via a
third-party component, it is plain data with addresses and lengths, and any mistake in interpreting it
turns into an allocator handing out firmware, MMIO or kernel memory. Milestone 2 needs somewhere to
put early physical memory management, and the project has no allocator, no paging of its own, and no
locking primitives yet.

## Requirements

- Never hand out memory the kernel is running on, firmware structures, MMIO, or the framebuffer.
- Use checked arithmetic on every address/length computation; report defects instead of panicking.
- Keep the untrusted-input logic unit-testable on the host, separate from hardware access.
- Make the limits of what is managed visible on the console and in the evidence record, so no reader
  can mistake "the allocator works" for "all physical memory is managed".
- Do not claim virtual memory, isolation or deallocation that does not exist.
- Stay within the small-change rule: no scaffolding for future subsystems.

## Options considered

1. **Trust `BootInfo.memory_regions` and allocate from `Usable` regions directly.** Simple, but it
   trusts firmware data with no validation, and it would hand out the first MiB (the firmware reports
   most of it as usable) and anything the bootloader forgot to mark.
2. **Parse firmware tables ourselves (E820/UEFI memory map) from the RSDP/smbios data.** More
   self-contained but re-implements what the bootloader already translates, adds a large
   untrusted-parser surface, and cannot be boot-tested meaningfully before storage and ACPI work
   exist.
3. **Validate the bootloader's map, then build a bitmap frame allocator with conservative policies
   and explicit reservations, all in a host-testable no_std crate** (`crates/physmem`). Chosen.
4. **Build our own page tables first and manage frames through them.** Larger step: it needs frame
   allocation first anyway, so it cannot come before (3).

## Decision

1. **Validate before use.** `physmem::MapValidator` checks each region for zero length and for an end
   address that is not representable, using checked arithmetic, and tracks byte totals with an
   explicit saturation flag. Any defect is fatal: the kernel prints which region was bad and halts
   instead of managing memory from a map it cannot trust. Firmware start/end pairs are converted
   through `MemoryRegion::from_bounds`, which refuses an end before the start rather than wrapping.
2. **Classify: usable, or not.** Only `MemoryRegionKind::Usable` regions may be handed out.
   `Bootloader` regions (which include the page tables the kernel is executing on), `UnknownUefi`,
   `UnknownBios` and anything unrecognised are reserved. Raw E820/UEFI type tags are deliberately not
   reinterpreted: the bootloader has already made that decision, and second-guessing it is not
   something this milestone can validate.
3. **Two passes, in this order.** All usable regions are marked first, then every non-usable claim,
   so a non-usable claim wins regardless of the order firmware listed regions in. The allocator
   enforces the ordering (`FrameError::UsableAfterNonUsable`) rather than trusting callers, because
   the failure mode of getting it wrong is handing out firmware-reserved memory.
4. **Conservative frame accounting.** Usable marking frees only whole frames entirely inside a
   region; reservation clears every frame that overlaps a claim, including partially covered frames
   at either end. Both directions withhold memory rather than risk handing out a frame that is partly
   reserved.
5. **Explicit reservations beyond the map.** The kernel reserves, by physical address, the first 1 MiB
   (interrupt vector table, BIOS data area and the movable extended BIOS data area, which firmware
   still reports as usable) and its own image from `BootInfo.kernel_addr`/`kernel_len`. The
   reservation output shows how many frames the policy took back from the map, so the cost is visible.
6. **A 4 GiB bitmap.** The allocator's bitmap is a 128 KiB `.bss` array covering the first 4 GiB in
   4 KiB frames, which covers the current smallest and largest tested guest with room to spare. Whole
   usable frames above the bitmap are counted and printed; they are not silently dropped.
7. **No deallocation, no reclamation, no paging.** One bit per frame cannot distinguish "allocated"
   from "reserved", and an ownership model invented before the page-table work that needs it would be
   guesswork. `Bootloader` memory is not reclaimed for the same reason: the kernel is still running on
   it. The kernel therefore still executes on the bootloader's page tables.

## Consequences and trade-offs

- Physical memory management can be tested on the host (21 unit tests in `crates/physmem`) and
  boot-tested in QEMU without any virtual memory existing yet.
- The 4 GiB limit and the withheld partial frames mean the kernel manages slightly less RAM than
  firmware advertises. The console report states both, so the loss is visible rather than assumed.
- The kernel stack and the framebuffer are **not** reserved by address: `BootInfo` exposes both as
  *virtual* addresses (`kernel_stack_bottom` is documented as virtual, and the framebuffer's mapping
  is a fixed high virtual address), so the kernel cannot compute their physical frames. It prints
  that fact instead of pretending to protect them, and relies on the bootloader having marked their
  frames non-usable. This is recorded as a limitation, not a solved problem.
- Frames are handed out but nothing maps them yet: the bootstrap allocation is visible on the console
  and is held for the page-table work that follows.
- Replacing the bootloader later (see ADR-0003) means implementing everything above ourselves,
  including the firmware-table parsing that was deliberately avoided here.

## Security and privacy impact

- The memory map is now treated as untrusted input: validated with checked arithmetic, rejected
  wholesale when defective, and unit tested against malformed maps.
- Unusable memory is withheld by default: every frame starts unavailable, and only explicit claims
  make it available.
- No protection is claimed. There is no address-space isolation, no privilege separation, no
  guard pages, no W^X and no deallocation; a kernel bug can still corrupt any memory it can address.
  The allocator reduces the chance of the kernel corrupting firmware or its own image, and nothing
  more.
- Privacy: no memory contents are read, logged or uploaded anywhere. The boot report prints region
  geometry only.

## Validation plan and evidence

- Host unit tests for the validator and allocator, including zero-length regions, unrepresentable end
  addresses, saturating totals, unsupported frame sizes, reversed bounds, partial-frame handling,
  bitmap overflow, non-usable claims overriding usable ones, call-order enforcement and rejected
  calls leaving state unchanged: `cargo test -p physmem`.
- QEMU boot test asserting both guest markers in order (`NEW_OS_KERNEL_MEMORY_OK` then
  `NEW_OS_KERNEL_BOOT_OK`) for the BIOS and UEFI images, plus the zero-filled-image negative control.
- Evidence, exact commands and observed serial output: `docs/verification/M2-MEMORY-EVIDENCE.md`.

## References

- `bootloader_api` 0.11.17 `info.rs` — `BootInfo` field meanings (`kernel_addr` is physical,
  `kernel_stack_bottom` is virtual), `MemoryRegion`, `MemoryRegionKind`; read from the pinned crate
  source on 2026-10-10.
- ADR-0003 (boot path) — the reused boot component and the bootloader's live page tables.
- ADR-0005 (first test platform) — QEMU configuration used for the boot evidence.
- `docs/verification/M2-MEMORY-EVIDENCE.md` — the tests and runs behind this decision.
