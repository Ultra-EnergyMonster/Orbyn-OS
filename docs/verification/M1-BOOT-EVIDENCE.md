# Milestone 1 (M1) Boot Evidence

> **Scope note (added after milestone 2 started):** every serial value below was produced by the
> *milestone-1* kernel (816,624-byte ELF). The kernel has since grown a boot memory-map validation
> and frame-allocation path, so current boot output differs; the current evidence is
> `docs/verification/M2-MEMORY-EVIDENCE.md`. The milestone-1 image-reproducibility claims were also
> corrected after a counter-observation — see checks 6 and 7 below.

**Date:** 2026-10-10
**Branch:** `feat/kernel-m1-bootable-prototype`
**Claim level:** *QEMU boot tested* for the BIOS and UEFI images (see
`docs/verification/VERIFICATION.md` for the vocabulary). Nothing here claims physical-hardware or
application compatibility.

The commands, exit codes and serial output below are from the final re-run made before this
milestone was committed (see "Independent re-run and build-hygiene checks"); any value that differs
from an earlier draft of this document was replaced with the observed one.

## What was tested

The milestone-1 kernel prototype booting in QEMU from generated disk images, asserting that the
guest kernel prints `NEW_OS_KERNEL_BOOT_OK` on its own COM1 serial path.

## Environment

| Item | Value |
| --- | --- |
| Host | Ubuntu 24.04 (noble), x86-64, KVM device present |
| Toolchain | rustup `nightly-2026-10-08` (rustc 1.101.0-nightly 1d81eb4ad 2026-10-07), components `rustfmt`, `clippy`, `llvm-tools`, `rust-src`; targets `x86_64-unknown-none`, `x86_64-unknown-uefi` |
| Target triple | `x86_64-unknown-none` for the kernel; host triple for the image builder |
| QEMU | 8.2.2 (Debian 1:8.2.2+ds-0ubuntu1.18) |
| BIOS firmware | SeaBIOS as shipped with QEMU 8.2.2 |
| UEFI firmware | OVMF from Ubuntu `ovmf` (2024.02), `/usr/share/OVMF/OVMF_CODE_4M.fd` + `OVMF_VARS_4M.fd` (`snapshot=on`) |
| Machine/accelerator | BIOS: `-machine pc`; UEFI: `-machine q35`; both `-accel tcg`, `-m 256M`, `-display none`, `-no-reboot` |
| Boot component | `bootloader` / `bootloader_api` 0.11.17 (MIT OR Apache-2.0) |

## Commands and results

| # | Command | Result |
| --- | --- | --- |
| 1 | `cargo fmt --all --check` | exit 0, no diff |
| 2 | `cargo clippy --target x86_64-unknown-none -p kernel -p uart16550 -- -D warnings` | exit 0, no warnings |
| 3 | `cargo clippy -p uart16550 -p boot-image --all-targets -- -D warnings` | exit 0, no warnings |
| 4 | `cargo test -p uart16550` | exit 0, `test result: ok. 5 passed; 0 failed` |
| 5 | `cargo build -p boot-image` | exit 0; produced `build/boot/bios.img` (3,638,272 B) and `build/boot/uefi.img` (3,211,264 B) |
| 6 | `./tools/test/qemu-smoke.sh bios` | exit 0, `PASS: guest kernel emitted NEW_OS_KERNEL_BOOT_OK` |
| 7 | `./tools/test/qemu-smoke.sh uefi` | exit 0, `PASS: guest kernel emitted NEW_OS_KERNEL_BOOT_OK` |
| 8 | Negative control: zero-filled image of the same size, `QEMU_TIMEOUT=8 ./tools/test/qemu-smoke.sh bios` | exit 1, `FAIL: guest kernel did not emit NEW_OS_KERNEL_BOOT_OK within 8s`, empty serial log; original image restored afterwards and sha256 verified identical |

## Independent re-run and build-hygiene checks (2026-10-10)

The whole sequence above was re-run from scratch on the same host and toolchain before the milestone
was committed, and how the images behave when a developer cleans up was checked at the same time.
Every result below is observed output.

| # | Check | Result |
| --- | --- | --- |
| 1 | `rm -rf build/boot && cargo build -p boot-image` | exit 0; `bios.img` (3,638,272 B) and `uefi.img` (3,211,264 B) recreated |
| 2 | `cargo build -p boot-image -v` with nothing changed | `Fresh boot-image`; the images were not rewritten |
| 3 | `head -c 3638272 /dev/zero > build/boot/bios.img && cargo build -p boot-image` | exit 0; `bios.img` regenerated to the correct sha256 |
| 4 | kernel source changed, then `cargo build -p boot-image` | both images regenerated for the new kernel (image mtime and BIOS image sha256 both changed) |
| 5 | kernel restored, then `cargo build -p boot-image` | BIOS image regenerated back to the original sha256 |
| 6 | BIOS image across independent regenerations | sha256 `27fd70d9ded3bcb502578add7f6a3a3e90ae26c851d0880470a30026b95106f4` in seven regenerations of this kernel |
| 7 | UEFI image across independent regenerations | differed between builds of the same kernel (`203489ee…`, `92565095…`, `9c32c866…`) |
| 8 | CI action pin re-checked against `https://api.github.com/repos/actions/checkout/git/ref/tags/v5` | matches the SHA used in `.github/workflows/ci.yml`: `fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09` |

Checks 1–5 came out of a defect found while re-verifying this milestone: `boot/image/build.rs`
generated the images only when cargo happened to run the script, so deleting `build/` (documented as
safe in `docs/development/BUILDING.md`) and rebuilding reported success while leaving the images
missing — and the smoke test then told the reader to run the very command that did nothing. The
script now hashes the kernel ELF and both images into cargo's `OUT_DIR` and regenerates when any of
the three differs from what it last wrote, so deletion recovery and "images follow the kernel" work
without turning every build into a rewrite. That change is build-script logic; the kernel itself was
not modified for it.

**Checks 6 and 7 do not support a reproducibility claim, and the original draft of this document drew
one too strongly.** The BIOS image was identical across the regenerations measured here, but a later
regeneration in the same environment produced a *different* `bios.img` for the same source
(sha256 `cd96c2e3…` once, then `94153b13…` seven times) with the cause unidentified. Byte-for-byte
reproducibility is therefore **not established for either image**; see limitation 2 of
`docs/verification/M2-MEMORY-EVIDENCE.md`. Both images are reproducible in *behaviour*: they boot and
the kernel emits the same markers.

## Clean-checkout verification (2026-10-10)

The documented workflow was verified from a fresh clone of the committed branch rather than only in
the working copy that produced it:

```sh
git clone --branch feat/kernel-m1-bootable-prototype <repository> /tmp/orbyn-clean-check
```

In that clone (commit `865b626`) every documented command was run and passed: `cargo fmt --all
--check`, `cargo clippy --target x86_64-unknown-none -p kernel -p uart16550 -- -D warnings`,
`cargo clippy -p uart16550 -p boot-image --all-targets -- -D warnings`, `cargo test -p uart16550`
(5 passed), `cargo build -p boot-image` (kernel compiled and both images generated from nothing),
and `./tools/test/qemu-smoke.sh bios` / `uefi` (both `PASS`). This is the same single Ubuntu 24.04
x86-64 host as above; no second machine or other operating system was tested.

## Guest serial output (BIOS run, kernel lines only)

The `INFO :`/`TRACE:` lines before this block come from the reused `bootloader` component; the
lines below are emitted by our kernel's own COM1 code.

```text
new-os kernel: milestone 1 prototype (x86-64, no_std)
boot path: firmware -> bootloader 0.11 -> _start -> kernel_main
boot handoff:
  kernel ELF: physical 0x0000000001000000, 816624 bytes, mapped at virtual offset 0x10000000000
  kernel stack: 81920 bytes at 0x0000018000001000
  physical memory regions reported: 10
  framebuffer: 1280x720, stride 1280, 3 bytes/pixel, Bgr
  ACPI RSDP address: 0x00000000000f52b0
NEW_OS_KERNEL_BOOT_OK
initialisation path complete; halting (no scheduler in milestone 1)
```

The UEFI run produced the same marker with different values (`kernel ELF: physical
0x000000000e077000`, `physical memory regions reported: 130`, `framebuffer: 1280x800, 4 bytes/pixel`,
`ACPI RSDP address: 0x000000000f77e014`), which is consistent with OVMF reporting a different memory
map and display mode. The absolute addresses and region count vary between firmware runs; the marker
and the handoff fields are what the test asserts on.

## What this proves

- The kernel is our own code and has a real x86-64 entry path: the `bootloader` component jumps to a
  `_start`/`kernel_main` defined in `kernel/src/main.rs`, and only code in this repository runs after
  that handoff.
- A reproducible build produces bootable images, and both the legacy BIOS and the UEFI boot paths
  reach the kernel in QEMU.
- The kernel's own COM1 driver works: the marker and handoff report were transmitted by
  `crates/uart16550`, and the kernel verifies that none of the marker's bytes were dropped before
  halting.
- The failure path is observable: the zeroed-image control run failed loudly instead of passing on a
  timeout.

## What this does NOT prove

- **No memory management, paging of our own, allocator, interrupts, timers, scheduling or
  userspace.** The kernel only *reports* the memory map; it does not validate or use it. That is M2/M3.
- **No device support beyond the debug UART.** The framebuffer is reported from `BootInfo` but the
  kernel does not draw to it; there is no driver model, storage, input, networking or audio (M4).
- **No Windows compatibility of any kind**, no PE parsing, no installer, no desktop, no file manager.
- **Neither boot image is byte-for-byte reproducible.** The BIOS image was identical across the
  regenerations measured here, but a counter-example was observed later for the same source; see
  checks 6 and 7 above and the correction they carry. Behaviour is identical in both cases.
- The kernel runs with whatever page tables the `bootloader` component created. Their correctness has
  not been independently reviewed by this project.

## Not run

| Check | Status | Blocker / next step |
| --- | --- | --- |
| Physical hardware boot | NOT RUN | Out of scope for M1; requires owner approval and a reviewed procedure. |
| Kernel unit tests | NOT APPLICABLE | Bare-metal binary cannot run the host test harness. |
| Fuzzing / untrusted input parsers | NOT RUN | No parser exists yet; starts at M9. |
| CI execution of this workflow | NOT RUN | The workflow was authored and reviewed locally but no push/CI run has happened yet. Treating it as passing would be false. |
| Reproducible-image comparison | ATTEMPTED, NOT ESTABLISHED | Repeated regenerations did not consistently produce identical bytes; see checks 6 and 7 and limitation 2 of the milestone-2 evidence. |
