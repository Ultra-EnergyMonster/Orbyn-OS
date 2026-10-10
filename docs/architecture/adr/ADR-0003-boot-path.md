# ADR-0003: UEFI-First Boot Path and Kernel Handoff

- **Status:** accepted (2026-10-10); validated by the milestone-1 boot experiment
- **Date:** 2026-10-10

## Context
The initial target is x86-64 PCs and QEMU. A reusable boot component is permitted, provided the OS retains its own kernel and the dependency is acceptable.

## Options considered
- Use a maintained bootloader project with a documented kernel handoff and image tooling.
- Write a minimal UEFI loader independently.
- Begin with another well-specified QEMU boot path, provided it does not undermine the target platform goal.

## Decision
Use the `rust-osdev/bootloader` project as the boot component for the first milestones, pinned at
**0.11.17** (`bootloader` and `bootloader_api`; licence MIT OR Apache-2.0). It was re-verified as
current at implementation time (crates.io 0.11.17, February 2026; upstream README states it is an
experimental x86_64 bootloader for BIOS and UEFI, requiring a nightly compiler plus
`llvm-tools`, and `rust-src` for its `-Zbuild-std` bootloader-stage build).

Details of the integration:

- The kernel is an `x86_64-unknown-none` binary crate using `bootloader_api::entry_point!`, which
  generates the `_start` entry point and a serialized boot configuration in a dedicated ELF section.
- `boot/image` builds both a BIOS image (covered by our own boot path on `-machine pc`) and a UEFI
  image (`-machine q35` with OVMF).
- The kernel receives `&'static mut BootInfo` and, in milestone 1, only reports its contents.

Writing our own UEFI loader remains a legitimate future option but is **not** justified yet: it would
add firmware, executable-format and recovery work before the kernel itself has memory management.

## Consequences
An established loader accelerates initial kernel development but adds a dependency and constrains
handoff design. The `bootloader` crate is explicitly experimental upstream, so the version is pinned
exactly and the handoff surface (`BootInfo`) is treated as an external ABI that this project does not
control. Upgrading it is a reviewed change, not an automatic one.

## Security impact
Boot information (memory map, framebuffer, RSDP) is untrusted input from firmware and/or the
bootloader; milestone 2 must validate it before use. Milestone 1 only prints it. Test artifacts stay
virtual; physical installation and secure-boot/release signing remain out of scope.

## Validation plan and evidence
Build, boot under QEMU, and assert a serial marker emitted by the guest kernel. **Done:** both BIOS
and UEFI images booted in QEMU 8.2.2 (accelerator `tcg`) and the guest kernel printed
`NEW_OS_KERNEL_BOOT_OK`; see `docs/verification/M1-BOOT-EVIDENCE.md`. A zeroed-image control run
failed as expected.

## References
- https://github.com/rust-osdev/bootloader — checked 2026-10-10 (0.11.17, MIT OR Apache-2.0).
- https://github.com/rust-osdev/bootloader/blob/main/docs/create-disk-image.md — disk image creation, checked 2026-10-10.
- https://uefi.org/specifications — UEFI specifications; not yet consulted in detail (no own loader).
