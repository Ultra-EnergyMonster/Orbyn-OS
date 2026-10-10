# M2 evidence — the kernel builds and installs its own page table (2026-10-11)

This record covers the M2 slice that makes the kernel own its top-level page table: it allocates a
private PML4, mirrors the bootloader's entries into it, adds one mapping built from freshly allocated
frames, installs it by loading `CR3`, and proves the new mapping by reading a magic value back through
the new virtual address. Decisions are in
`docs/architecture/adr/ADR-0018-kernel-page-tables.md`.

Vocabulary is that of `docs/verification/VERIFICATION.md`. Everything below was actually run unless
explicitly marked `NOT RUN`.

## Environment

Same host as `docs/development/BUILDING.md`: Ubuntu 24.04 x86-64, pinned nightly
`nightly-2026-10-08` (rustc 1.101.0-nightly), QEMU 8.2.2 (`-accel tcg`), OVMF from the Ubuntu
`ovmf` package for the UEFI image. No physical hardware, no KVM.

## What changed

| File | Change |
| --- | --- |
| `kernel/src/paging.rs` | New `install_own_page_tables`: allocate + mirror the PML4, find a free top-level slot, build and zero `PDPT`/`PD`/`PT` and a data frame from the kernel's own allocator, map the frame at `0xFFFF_8000_0000_0000`, write a magic value, load `CR3`, then verify `CR3`, the magic read through the new mapping, and that the bootloader's stack mapping survived. New `write_physical`/`write_cr3` helpers; `Failure` gained `OutOfFrames`, `NoFreeTableSlot` and `VerificationFailed`. |
| `kernel/src/main.rs` | Calls `install_own_page_tables` after the bootstrap allocation and emits `NEW_OS_KERNEL_PAGETABLES_OK` only on success. |
| `tools/test/qemu-smoke.sh` | Asserts the new marker, and requires it to appear between the paging marker and the memory marker. |
| `crates/x86paging` | No change (the walker is reused); `ENTRY_PRESENT`/`ENTRY_WRITABLE` already existed. |

No new third-party dependency and no new crate.

## Checks run

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --target x86_64-unknown-none -p kernel -p uart16550 -- -D warnings` | exit 0 |
| `cargo clippy -p physmem -p x86paging -p x86exc -p uart16550 -p boot-image --all-targets -- -D warnings` | exit 0 |
| `cargo test -p physmem -p uart16550 -p x86exc -p x86paging` | 44 passed, 0 failed (21 + 5 + 10 + 8) |
| `cargo build -p boot-image` | exit 0; kernel + both images regenerated |
| `./tools/test/qemu-smoke.sh bios` | **PASS** |
| `./tools/test/qemu-smoke.sh uefi` | **PASS** |

No host unit test was added for this slice: the new logic is entirely register-level (control-register
access, the frame allocator, the installed tables), which the host cannot exercise. It is covered by
the boot test below.

## Guest evidence (BIOS, `-machine pc` + SeaBIOS)

```
  ACPI RSDP address: 0x00000000000f52b0
  physical memory offset: 0x0000028000000000
  reserved kernel stack: virtual 0x0000018000001000 -> physical 0x0000000001125000..0x0000000001139000 (81920 bytes, 0 previously usable frame(s) reclaimed)
  reserved framebuffer: virtual 0x0000020000000000 -> physical 0x00000000fd000000..0x00000000fd2a3000 (2764800 bytes, 0 previously usable frame(s) reclaimed)
NEW_OS_KERNEL_PAGING_OK
bootstrap allocation: 8 frames of 4096 bytes for the page-table work that follows
  physical range 0x0000000000100000..0x0000000000108000; 8 frame(s) now allocated, 63882 still free
  page tables: own PML4 at physical 0x0000000000108000; slot 256 maps 0xffff800000000000 -> physical 0x000000000010c000 (read back 0xc0ffee0012345678)
NEW_OS_KERNEL_PAGETABLES_OK
NEW_OS_KERNEL_MEMORY_OK
NEW_OS_KERNEL_BOOT_OK
initialisation path complete; halting (no scheduler in milestone 2)
```

Marker order in that run: `NEW_OS_KERNEL_EXCEPTION_OK` (line 42), `NEW_OS_KERNEL_TIMER_OK` (43),
`NEW_OS_KERNEL_PAGING_OK` (59), `NEW_OS_KERNEL_PAGETABLES_OK` (66), `NEW_OS_KERNEL_MEMORY_OK` (67),
`NEW_OS_KERNEL_BOOT_OK` (68). The smoke test asserts that ascending order, so the page-table step
cannot be skipped or moved while the test still passes.

The kernel moved its own top-level table to physical `0x108000` — the first frame the earlier
bootstrap allocation below 1 MiB handed out — and read back `0xc0ffee0012345678` through
`0xffff800000000000`, which is only reachable if (a) all 512 copied entries carried the bootloader's
mappings over, and (b) the freshly built `PML4 -> PDPT -> PD -> PT -> frame` chain is correct. The
kernel then kept running on its own table for the rest of the boot: the memory marker, the boot
marker and the halt all come after the switch.

## Guest evidence (UEFI, `-machine q35` + OVMF)

`PASS` with the same six markers in order: `EXCEPTION_OK` (line 49), `TIMER_OK` (50), `PAGING_OK` (66),
`PAGETABLES_OK` (73), `MEMORY_OK` (74), `BOOT_OK` (75). The UEFI boot therefore performs the same
`CR3` switch on a different firmware path.

## Negative control

The new marker was checked on its own, as `docs/development/TESTING.md` requires. The page-table
installation was left running but its marker emission removed, the image rebuilt, and the smoke test
re-run:

```
FAIL: guest kernel did not emit: NEW_OS_KERNEL_PAGETABLES_OK within 30s (mode=bios); see build/test/serial-bios.log
exit code: 1
```

Only that one marker was reported missing, so the assertion is genuinely about the page-table step and
is not covered by the other markers. `kernel/src/main.rs` was then restored, the image rebuilt, and
`./tools/test/qemu-smoke.sh bios` passed again.

## What this proves, and what it does not

**Proves:** the kernel allocates its own top-level page table, mirrors the bootloader's mappings into
it, grows its address space with a mapping built from its own frames, switches to it with `CR3`, and
reads that mapping back correctly — on both the BIOS and UEFI paths, with a negative control showing
the check is real.

**Does not prove:** that the lower-level tables are ours (they are still the bootloader's), any
`map`/`unmap` of arbitrary ranges, per-region protection or user/supervisor separation (every mapping
stays as the bootloader left it: writable and user-accessible), frame or page-table reclamation, a
per-process address space, or any behaviour on physical hardware, under KVM, or on a second host.
`NOT RUN`: hardware, KVM, other hosts.

## Follow-up work (not done here)

- Replace the mirrored lower levels with tables the kernel owns end to end, starting with an explicit
  set of kernel mappings reproduced before the switch.
- A real `map`/`unmap`/`protect` API over our own tables, including allocating lower-level tables on
  demand.
- Page-table frame reclamation once deallocation exists (the allocator is take-only today).
- Per-page protection flags and a user/supervisor split, which require the tables to be ours first.
