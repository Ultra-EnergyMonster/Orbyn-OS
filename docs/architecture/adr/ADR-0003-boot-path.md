# ADR-0003: UEFI-First Boot Path and Kernel Handoff

- **Status:** proposed; validate current crate/tool support before acceptance
- **Date:** 2026-10-10

## Context
The initial target is x86-64 PCs and QEMU. A reusable boot component is permitted, provided the OS retains its own kernel and the dependency is acceptable.

## Options considered
- Use a maintained bootloader project with a documented kernel handoff and image tooling.
- Write a minimal UEFI loader independently.
- Begin with another well-specified QEMU boot path, provided it does not undermine the target platform goal.

## Decision
Prefer investigating a UEFI-first flow and a maintained Rust-compatible bootloader before writing a custom loader. The `rust-osdev/bootloader` project is one candidate, but must be checked for current API, toolchain/nightly needs, maintenance, supported architecture, licence, and image-building behaviour at implementation time. Do not treat any candidate as guaranteed suitable merely because it appears in this ADR. Pin the chosen version and record exact commands once a build/boot experiment works.

## Consequences
An established loader can accelerate initial kernel development but adds a dependency and constrains handoff design. A custom loader provides control but creates additional firmware, executable format and recovery work.

## Security impact
Validate boot information and memory-map data. Keep test artifacts virtual. Physical installation and secure-boot/release signing are out of scope for the first task.

## Validation plan
Build, boot under QEMU, and assert a serial marker emitted by the guest kernel. Record exact QEMU version/configuration and toolchain.

## References
- https://github.com/rust-osdev/bootloader — candidate project; verify current state at implementation time.
- https://uefi.org/specifications — UEFI specifications; verify applicable version at implementation time.
