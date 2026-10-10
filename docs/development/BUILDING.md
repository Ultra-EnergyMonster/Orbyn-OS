# Building

**Status:** the commands below were run and verified on the environment recorded here (2026-10-10,
milestone 1). Anything not marked "verified" is untested.

## Verified build environment

| Item | Value |
| --- | --- |
| Host OS | Ubuntu 24.04 (noble), x86-64 |
| Rust toolchain | `nightly-2026-10-08` → rustc 1.101.0-nightly (1d81eb4ad 2026-10-07), cargo 1.101.0-nightly (f3865b2a4 2026-09-29) |
| Toolchain source | `rust-toolchain.toml` (pinned channel, components `rustfmt`, `clippy`, `llvm-tools`, `rust-src`; targets `x86_64-unknown-none`, `x86_64-unknown-uefi`) |
| QEMU | 8.2.2 (Debian 1:8.2.2+ds-0ubuntu1.18) |
| UEFI firmware | OVMF from Ubuntu's `ovmf` package (`/usr/share/OVMF/OVMF_CODE_4M.fd`) |
| Boot component | `bootloader` / `bootloader_api` 0.11.17 (MIT OR Apache-2.0) |

Why nightly: `bootloader` 0.11.17 requires a nightly compiler, `llvm-tools` for linking, and
`rust-src` because it builds its BIOS/UEFI bootloader stages with `-Zbuild-std`. See ADR-0003 and
ADR-0004.

## Prerequisites

```sh
# Rust toolchain manager (the pinned toolchain, components and targets are declared in
# rust-toolchain.toml and are installed automatically on the first cargo command in this repository).
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# QEMU and OVMF are only needed for the boot smoke test, not for building.
sudo apt-get install -y qemu-system-x86 ovmf
```

Building requires no administrator rights.

## Build the kernel and boot images

```sh
cargo build -p boot-image
```

This single command:

1. compiles the kernel crate for `x86_64-unknown-none`,
2. runs `boot/image/build.rs`, which loads the kernel ELF into the `bootloader` crate's image
   creation, and
3. writes two bootable images:

| Output | Size (dev profile, milestone 1) | Format |
| --- | --- | --- |
| `build/boot/bios.img` | 3,638,272 bytes | MBR partitioned, legacy BIOS/SeaBIOS bootable |
| `build/boot/uefi.img` | 3,211,264 bytes | GPT partitioned, EFI system partition |

`cargo run -p boot-image` builds the same images and prints their paths. (It does not regenerate
them by itself — image creation happens in the build script, so `cargo build -p boot-image` is the
command that rebuilds a missing or stale image.)

`build/` is gitignored. Generated images are commit-excluded on purpose (AGENTS.md: no generated
binaries in the repository).

### Regenerating the images

`boot/image/build.rs` is the only thing that writes the images. Cargo runs it when the kernel or
the script changes, so to also cover a removed or overwritten image the script records the content
hashes of the kernel ELF and of both images in cargo's `OUT_DIR` and regenerates when any of the
three differs from what it last wrote. Measured behaviour (2026-10-10, evidence in
`docs/verification/M1-BOOT-EVIDENCE.md`):

| Situation | Result |
| --- | --- |
| `cargo build -p boot-image`, nothing changed | no-op; the images are left untouched |
| `rm -rf build/` (or `rm -rf build/boot`) then `cargo build -p boot-image` | both images are recreated |
| kernel source changed and rebuilt | both images are regenerated for the new kernel |
| an image was overwritten or truncated | regenerated back to the correct contents |

One expected detail: the build after a regeneration reports `Compiling boot-image` once more before
it settles back to a no-op, because the script re-runs once to confirm the images it just wrote are
still current.

## Boot-test the images

```sh
./tools/test/qemu-smoke.sh bios   # SeaBIOS / legacy BIOS
./tools/test/qemu-smoke.sh uefi   # OVMF / UEFI
```

See `docs/development/TESTING.md` for what the smoke test asserts and the env overrides
(`QEMU_BIN`, `QEMU_ACCEL`, `QEMU_TIMEOUT`, `GUEST_MEMORY`).

## Cleaning up

```sh
rm -rf build/   # generated images and test logs only; safe and inside the repository
cargo clean     # removes target/
```

`rm -rf build/` is safe: the next `cargo build -p boot-image` recreates the images (verified). Never
delete anything outside `build/` or `target/` to clean up this project.

## Not yet verified

- **Release profile** (`cargo build -p boot-image --release`): not run. The dev profile is what the
  smoke test used.
- **Cross-host builds**: only the Ubuntu 24.04 x86-64 host above was tested. Windows or macOS hosts
  are untested.
- **Reproducibility of the images byte-for-byte**: measured (2026-10-10) with a mixed result.
  `build/boot/bios.img` was byte-identical across independent regenerations (sha256
  `27fd70d9ded3bcb502578add7f6a3a3e90ae26c851d0880470a30026b95106f4` every time), but
  `build/boot/uefi.img` was not: its bytes differed between builds of the same kernel, almost
  certainly because the EFI system partition's FAT metadata carries host timestamps. Only the BIOS
  image may be treated as reproducible; the UEFI image is reproducible in behaviour (it boots and
  emits the same marker) but not in bytes. See `docs/verification/M1-BOOT-EVIDENCE.md`.
- **`cargo build --workspace`**: expected to fail for the `kernel` crate, which is a bare-metal
  binary that cannot link against the host standard library. Use the per-package commands above.
