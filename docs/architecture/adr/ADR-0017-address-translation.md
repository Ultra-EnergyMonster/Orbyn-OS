# ADR-0017: Reading the Bootloader's Page Tables to Translate Kernel Addresses

- **Status:** accepted
- **Date:** 2026-10-10
- **Owners:** Project owner / contributors
- **Supersedes:** None

## Context

`BootInfo` reports the kernel image as a *physical* address (`kernel_addr`) but the kernel stack and the
framebuffer only as *virtual* addresses. The bootstrap frame allocator therefore could not reserve the
frames those regions actually occupy and relied on the bootloader having marked them non-usable — a
gap recorded as limitation 1 in `docs/verification/M2-MEMORY-EVIDENCE.md`. Milestone 2's remaining item
is virtual memory and page tables; the first thing it needs is the ability to translate an address.

The first attempt used `BootInfo::physical_memory_offset`, which was `None`: `bootloader` 0.11 maps
physical memory only when the kernel asks it to (`Mappings::physical_memory` defaults to `None`), and
without that mapping the kernel has no way to read paging-structure entries, which live at physical
addresses.

## Requirements

- Translate kernel virtual addresses to physical ones using the page tables that are actually
  installed (the bootloader's), not a reimplementation of the firmware memory map.
- Keep the walk pure and host-testable, separate from the hardware access.
- Handle 4 KiB, 2 MiB and 1 GiB mappings, and report which level was not present.
- Use the result to reserve the kernel stack and framebuffer frames explicitly.
- Do **not** claim to own page tables: no mapping/unmapping, no address spaces, no `CR3` switch.
- Keep it verifiable in QEMU with an explicit marker.

## Options considered

1. **Keep relying on the bootloader to mark stack/framebuffer frames non-usable.** No progress; leaves
   a documented gap where a wrong bootloader assumption is undetectable.
2. **Parse the firmware memory map / RSDP ourselves and reason about the stack region.** Does not
   actually resolve a virtual address, and re-implements what the page tables already encode.
3. **Enable the bootloader's physical-memory mapping and walk the installed page tables in our own
   host-testable code.** Chosen.
4. **Take over paging entirely (new PML4, `CR3` switch) in this slice.** Larger and riskier, and it
   needs the translation ability that (3) provides anyway.

## Decision

1. **Ask the bootloader to map physical memory.** The kernel supplies a `BootloaderConfig` with
   `mappings.physical_memory = Some(Mapping::Dynamic)`, so `BootInfo::physical_memory_offset` is
   populated and every paging-structure entry is reachable. Without this, translation is impossible.
2. **Own the walk, not the tables.** `crates/x86paging` decomposes a virtual address into its four
   table indices, checks canonicity, decodes paging-structure entries, and walks four levels given a
   closure that reads physical memory. It is generic over that closure, so it is fully unit tested on
   the host against a synthetic table (4 KiB, 2 MiB and 1 GiB mappings, each "not present" level, and
   non-canonical input).
3. **Use the installed tables.** `kernel/src/paging.rs` reads `CR3`, masks the PML4 base, and feeds the
   walk a closure that reads through the bootloader's physical-memory offset. It translates
   `kernel_stack_bottom` and the framebuffer's virtual address, then reserves their physical frames in
   the allocator.
4. **Own nothing yet, and say so.** This slice does not allocate, create, modify or install page
   tables, does not switch `CR3`, and provides no `map`/`unmap` or protection. The kernel still runs on
   the bootloader's tables. Owning our own tables is the next step.
5. **Assert it in the boot path.** The boot emits `NEW_OS_KERNEL_PAGING_OK` only after the stack and
   framebuffer have been translated and their frames reserved, and the smoke test asserts it in order.

## Consequences and trade-offs

- The allocator now protects the kernel stack and framebuffer frames by **computing** their physical
  ranges, rather than trusting that the bootloader marked them non-usable.
- Enabling the physical-memory mapping makes the boot environment larger and slightly different: the
  bootloader creates more page tables, so usable RAM reported to the allocator drops a little (in the
  BIOS run, 254 MiB → 250 MiB). That is visible in the console report.
- The kernel now depends on `bootloader_api`'s config surface (`Mappings`, `Mapping`), which is part of
  the same pinned 0.11.17 component it already relies on.
- **Not solved:** the kernel still does not own its page tables, has no address spaces, no per-process
  isolation, no `map`/`unmap`, no guard pages and no `W^X`.

## Security and privacy impact

- **The physical-memory mapping is a broad, writable view of RAM** mapped into the kernel's address
  space. That is normal for a kernel that will manage memory itself, but it is an honest increase in
  what a kernel bug could reach, and it is a reason to get our own page tables and protection soon.
  Nothing is claimed about isolation.
- The walk only *reads* paging structures, and only follows entries with the present bit set, so it
  does not guess at unmapped memory.
- A translation failure is reported as a limitation and the boot continues (except that the stack must
  translate for the paging marker); no memory is silently mis-reserved.
- No data is captured, stored or transmitted.

## Validation plan and evidence

- Host unit tests for the walker (mapping sizes, missing levels, canonicity, index decomposition):
  `cargo test -p x86paging`.
- QEMU boot test asserting `NEW_OS_KERNEL_EXCEPTION_OK`, `NEW_OS_KERNEL_TIMER_OK`,
  `NEW_OS_KERNEL_PAGING_OK`, `NEW_OS_KERNEL_MEMORY_OK`, `NEW_OS_KERNEL_BOOT_OK`, in that order, for the
  BIOS and UEFI images, plus the zeroed-image and targeted-marker negative controls.
- Evidence, exact commands and observed serial output: `docs/verification/M2-PAGING-EVIDENCE.md`.

## References

- Intel SDM Vol. 3A, 4.5 ("4-Level Paging") and 4.5.4 (page-table entry formats).
- `bootloader_api` 0.11.17 `src/config.rs` — `BootloaderConfig`, `Mappings::physical_memory` default
  `None`, `Mapping::Dynamic`; read from the pinned crate source on 2026-10-10.
- ADR-0014 (boot memory handling) — the allocator this protects and the limitation this closes.
- ADR-0015 (exception handling) — the page-fault handler that reports a bad walk.
- `docs/verification/M2-PAGING-EVIDENCE.md` — the tests and runs behind this decision.
