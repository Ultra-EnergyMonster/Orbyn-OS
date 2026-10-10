# Architecture Overview

**Status:** initial design outline; architecture decisions remain proposed until the first experiments and ADR reviews are complete.

## Goal

Build an original x86-64-first operating system with its own kernel, a distinct premium desktop, privacy-first defaults, and click-first application management. It should not use the Linux kernel or Windows kernel. The eventual user experience should feel familiar to Windows users, but the implementation, product identity, service model and security defaults should be independent.

## Layers (conceptual, not all implemented)

1. **Firmware and boot path:** UEFI firmware, then a reviewed boot component/loader, which transfers control and a documented boot-information structure to our kernel.
2. **Kernel:** CPU setup, exceptions/interrupts, physical and virtual memory, allocation, scheduling, synchronisation, syscall boundary, process model, timers and core resource management.
3. **Drivers and hardware abstraction:** bus/device discovery, block and input drivers, display output, networking and other peripherals. Start with virtual hardware and only claim the devices actually tested.
4. **Core userland services:** process startup, device management, storage services if appropriate, logging, package management, permissions, updates and recovery.
5. **Native application environment:** runtime/ABI, standard libraries and app packaging.
6. **Graphics and desktop:** display/input stack, compositor/window manager, shell/launcher/taskbar, file manager, settings, notifications and GUI installation.
7. **Windows compatibility subsystem:** PE/COFF parser and loader, process/ABI support, Windows API implementation, graphics/audio services and compatibility tooling. This is a major independent programme, not a file association.
8. **Developer/release infrastructure:** reproducible builds, QEMU tests, CI, SBOM/licence inventory, signed updates/releases and verified recovery.

## Foundational strategy

- x86-64 and QEMU first.
- Prefer a documented UEFI-first boot path; evaluate reusable bootloader components in ADR-0003.
- Use Rust `no_std` in kernel contexts, with C/assembly only where justified. Pin toolchains after testing.
- Use serial diagnostics and a bounded QEMU boot smoke test before building a GUI.
- Maintain a strict separation between host tools and code that runs inside the OS.
- Use the public GitHub repository as the audit trail; keep secrets out of it and work through feature branches.

## Compatibility claims

The desktop shell's ability to associate `.exe` files is not Windows compatibility. Meaningful compatibility requires a loader and a large set of Windows user-mode APIs/services. Some programs depend on proprietary DRM/anti-cheat, kernel drivers, graphics features or OS behaviours that may not be supportable. Each claim must be tied to exact versions and test evidence in `docs/roadmap/COMPATIBILITY_MATRIX.md`.

## Current implementation state

This document is a plan only. No kernel, loader, desktop, driver model or runtime should be listed as implemented until it exists and has corresponding verification evidence.
