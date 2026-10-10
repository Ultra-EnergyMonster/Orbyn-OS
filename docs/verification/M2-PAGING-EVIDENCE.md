# Milestone 2 Paging Evidence (address translation, stack/framebuffer reservation)

> **Scope note (added after the page-table slice):** the kernel now goes further than the read-only
> walk described here — it allocates a private PML4, mirrors the bootloader's entries, adds one new
> mapping from its own frames and installs it with `mov cr3`, and `tools/test/qemu-smoke.sh` asserts
> one further marker (`NEW_OS_KERNEL_PAGETABLES_OK`) between `NEW_OS_KERNEL_PAGING_OK` and
> `NEW_OS_KERNEL_MEMORY_OK`. The evidence below is unchanged; the current boot evidence is
> `docs/verification/M2-PAGETABLES-EVIDENCE.md`.

**Date:** 2026-10-10
**Branch:** `feat/m2-address-translation`
**Claim level:** *QEMU boot tested* for read-only address translation over the installed page tables
(see `docs/verification/VERIFICATION.md`). Nothing here claims our own page tables, an address space,
`map`/`unmap`, protection or isolation.

## What was tested

The kernel's first virtual-memory capability (`crates/x86paging` + `kernel/src/paging.rs`):

1. the kernel asks the bootloader to map all physical memory (`BootloaderConfig.mappings.
   physical_memory = Some(Mapping::Dynamic)`), without which paging-structure entries cannot be read;
2. a pure, host-testable four-level walk decomposes a virtual address, checks canonicity, follows
   present entries, and handles 4 KiB, 2 MiB and 1 GiB mappings;
3. the kernel reads `CR3`, feeds the walk a closure that reads through the bootloader's
   physical-memory offset, and translates the address of its **kernel stack** and the **framebuffer**;
4. those translated physical ranges are **reserved** in the frame allocator, replacing the previous
   behaviour of relying on the bootloader to have marked them non-usable;
5. the boot emits `NEW_OS_KERNEL_PAGING_OK` only after both translations and reservations succeed, and
   `tools/test/qemu-smoke.sh` asserts it in order.

## Environment

| Item | Value |
| --- | --- |
| Host | Ubuntu 24.04 (noble), x86-64, KVM device present |
| Toolchain | rustup `nightly-2026-10-08` (rustc 1.101.0-nightly 1d81eb4ad 2026-10-07), components `rustfmt`, `clippy`, `llvm-tools`, `rust-src`; targets `x86_64-unknown-none`, `x86_64-unknown-uefi` |
| QEMU | 8.2.2 (Debian 1:8.2.2+ds-0ubuntu1.18), accelerator `tcg`, `-m 256M`, `-display none`, `-no-reboot` |
| BIOS mode | `-machine pc` + SeaBIOS from QEMU 8.2.2 |
| UEFI mode | `-machine q35` + OVMF 2024.02-2ubuntu0.10 (`/usr/share/OVMF/OVMF_CODE_4M.fd`, vars store with `snapshot=on`) |
| Boot component | `bootloader` / `bootloader_api` 0.11.17, now configured to map physical memory |
| New crate | `crates/x86paging` (this repository), no new third-party dependency |

## Commands and results

| # | Command | Result |
| --- | --- | --- |
| 1 | `cargo fmt --all --check` | exit 0, no diff |
| 2 | `cargo clippy --target x86_64-unknown-none -p kernel -p uart16550 -- -D warnings` | exit 0, no warnings |
| 3 | `cargo clippy -p physmem -p x86paging -p x86exc -p uart16550 -p boot-image --all-targets -- -D warnings` | exit 0, no warnings |
| 4 | `cargo test -p physmem -p uart16550 -p x86exc -p x86paging` | exit 0: `21 passed`, `5 passed`, `10 passed`, `8 passed` |
| 5 | `cargo build -p boot-image` | exit 0 |
| 6 | `./tools/test/qemu-smoke.sh bios` | exit 0; `PASS: guest kernel emitted NEW_OS_KERNEL_EXCEPTION_OK (line 42), NEW_OS_KERNEL_TIMER_OK (line 43), NEW_OS_KERNEL_PAGING_OK (line 59), NEW_OS_KERNEL_MEMORY_OK (line 65) then NEW_OS_KERNEL_BOOT_OK (line 66)` |
| 7 | `./tools/test/qemu-smoke.sh uefi` | exit 0; `PASS: ... NEW_OS_KERNEL_EXCEPTION_OK (line 49), NEW_OS_KERNEL_TIMER_OK (line 50), NEW_OS_KERNEL_PAGING_OK (line 66), NEW_OS_KERNEL_MEMORY_OK (line 72) then NEW_OS_KERNEL_BOOT_OK (line 73)` |
| 8 | Negative control: zero-filled `build/boot/bios.img`, `QEMU_TIMEOUT=8 ./tools/test/qemu-smoke.sh bios` | exit 1; `FAIL: guest kernel did not emit: NEW_OS_KERNEL_EXCEPTION_OK NEW_OS_KERNEL_TIMER_OK NEW_OS_KERNEL_PAGING_OK NEW_OS_KERNEL_MEMORY_OK NEW_OS_KERNEL_BOOT_OK within 8s`; image restored and sha256 verified unchanged |
| 9 | Targeted negative control: the paging marker's emission removed from `kernel/src/main.rs`, rebuilt, `QEMU_TIMEOUT=15 ./tools/test/qemu-smoke.sh bios` | exit 1; `FAIL: guest kernel did not emit: NEW_OS_KERNEL_PAGING_OK within 15s` — the assertion is genuinely required; source restored and images rebuilt |

A first attempt at this slice is part of the evidence: with the bootloader's default configuration,
`BootInfo::physical_memory_offset` is `None`, and the run printed

```text
  could not reserve kernel stack: 0x0000018000001000 is not translatable (the bootloader exposed no physical-memory offset)
  could not reserve framebuffer: 0x0000020000000000 is not translatable (the bootloader exposed no physical-memory offset)
```

with no paging marker — i.e. the kernel correctly reported the blocker instead of pretending to
translate. Enabling the physical-memory mapping fixed it.

## Guest serial output (kernel lines, both boot modes)

BIOS:

```text
  physical memory offset: 0x0000028000000000
  reserved first 1 MiB (real-mode firmware structures): 0x0000000000000000..0x0000000000100000 (1048576 bytes, 159 previously usable frame(s) reclaimed)
  reserved kernel image: 0x0000000001000000..0x00000000010f0788 (984968 bytes, 0 previously usable frame(s) reclaimed)
  reserved kernel stack: virtual 0x0000018000001000 -> physical 0x000000000111d000..0x0000000001131000 (81920 bytes, 0 previously usable frame(s) reclaimed)
  reserved framebuffer: virtual 0x0000020000000000 -> physical 0x00000000fd000000..0x00000000fd2a3000 (2764800 bytes, 0 previously usable frame(s) reclaimed)
NEW_OS_KERNEL_PAGING_OK
```

UEFI (same offset and stack layout, different physical placement):

```text
  physical memory offset: 0x0000028000000000
  reserved kernel stack: virtual 0x0000018000001000 -> physical 0x000000000012d000..0x0000000000141000 (81920 bytes, 0 previously usable frame(s) reclaimed)
  reserved framebuffer: virtual 0x0000020000000000 -> physical 0x0000000080000000..0x00000000803e8000 (4096000 bytes, 0 previously usable frame(s) reclaimed)
NEW_OS_KERNEL_PAGING_OK
```

The numbers are internally consistent: the stack is the 80 KiB (`81920` bytes) region the bootloader
reported, and the framebuffer size matches the reported mode (`1280×720×3 = 2,764,800` bytes under
SeaBIOS; `1280×800×4 = 4,096,000` bytes under OVMF), so the translation is following real mappings and
not returning a plausible-looking constant.

## What this proves

- **The kernel can translate its own virtual addresses.** It reads `CR3`, walks the installed page
  tables and obtains the physical address behind a virtual one, which it previously could not do.
- **The kernel stack and framebuffer frames are now reserved by computation.** Their physical ranges
  come from the page tables rather than from trusting the bootloader's memory-map classification, so
  the allocator cannot hand them out.
- **The walk is exercised where it can be.** Mapping sizes, each missing level, canonicity and index
  decomposition are unit tested on the host; the QEMU run proves the same code works against real
  tables created by the bootloader.
- **The boot environment change is observable and honest.** Enabling the physical-memory mapping is
  visible in the console (`physical memory offset`) and in the slightly smaller usable-RAM figure.

## What this does NOT prove

- **No page tables of our own.** The kernel still runs on the tables the bootloader created; nothing
  here allocates, creates, modifies or installs one, and `CR3` is never written.
- **No address spaces, `map`/`unmap`, guard pages, `W^X` or isolation.** No process model exists.
- **The broad physical-memory mapping is not a protection claim.** It is a large writable view of RAM
  in the kernel address space; it exists to make paging structures readable.
- **No physical-hardware, KVM or multi-host test**, as in the previous evidence records.

## Known limitations and open questions

1. **A translation failure is not fatal** (except for the stack/paging marker); the framebuffer could
   fail to translate and the boot would continue with a warning. On the tested configurations both
   succeed.
2. **The walk assumes the bootloader's offset mapping covers the paging structures it follows.** It
   does under `Mapping::Dynamic`; a future loader that does not map RAM would need a different source
   of entry addresses.
3. **The reserved ranges are page-aligned conservatively** by the allocator's existing policy; the
   reserved stack starts at the translated page base, which is what the 80 KiB region occupies.
4. **Usable RAM reported drops a little** (BIOS run: 254 MiB → 250 MiB) because the bootloader creates
   more page tables for the physical-memory mapping. It is printed, not hidden.

## Not run

| Check | Status | Reason / next step |
| --- | --- | --- |
| Our own page tables / `CR3` switch | NOT RUN | The next step in this milestone |
| `map`/`unmap`, guard pages, `W^X`, NX | NOT RUN | No page-table ownership yet |
| 5-level paging | NOT RUN | 4-level only, matching the boot component's configuration |
| Physical hardware boot | NOT RUN | Out of scope; requires owner approval and a reviewed procedure |
| CI execution for this branch | NOT RUN | No pull request has been opened for this branch, so the workflow has not run on it. It has run and passed on `main` and on the exception branch's PR (2026-10-10). |
