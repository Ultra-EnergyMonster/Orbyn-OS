# Boot integration

This directory holds the bootloader integration. The kernel itself lives in `kernel/`; the boot
component is a reused, reviewed dependency rather than our own loader (ADR-0003).

## `image/` — boot image builder

`boot/image` is a normal host-side Cargo crate. Its build script depends on the `kernel` crate as a
cargo *artifact dependency*, so building it:

1. compiles the kernel for `x86_64-unknown-none`,
2. loads that ELF into the `bootloader` 0.11 crate's `BiosBoot`/`UefiBoot` image creation, and
3. writes two bootable disk images:
   - `build/boot/bios.img` — legacy BIOS (SeaBIOS) image,
   - `build/boot/uefi.img` — UEFI image with a FAT EFI system partition.

```sh
cargo build -p boot-image   # builds the kernel and (re)generates the images
cargo run -p boot-image     # builds the same way and prints the image paths
```

Images are written only when their contents would change: the build script hashes the kernel ELF and
both images and skips the work when all three are unchanged, so an up-to-date build is a no-op while
a deleted or stale image is rebuilt. See `docs/development/BUILDING.md`.

Generated images live under `/build/`, which is gitignored. They are never written to a physical
disk by any script in this repository; `tools/test/qemu-smoke.sh` only attaches them as QEMU
`-drive` files.

## Dependency

| Component | Version | Licence | Notes |
| --- | --- | --- | --- |
| `bootloader` | 0.11.17 | MIT OR Apache-2.0 | Reused boot component. Experimental upstream; pinned exactly in `Cargo.toml`. Requires nightly Rust plus `llvm-tools`. |
| `bootloader_api` | 0.11.17 | MIT OR Apache-2.0 | Kernel side of the same project: `entry_point!` macro and `BootInfo`. |

The `bootloader` crate owns firmware interaction, page-table setup and the kernel handoff. It is not
a kernel: all code that runs after the handoff is ours (`kernel/`). Replacing it later means writing
our own UEFI loader, which is tracked as future work, not a claimed feature.
