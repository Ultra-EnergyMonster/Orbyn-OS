# Architecture Overview

**Status:** initial design outline. Layers 1–2 are partially implemented and boot-tested at their smallest size (milestones 1–3); every other layer is planned only. ADR-0003, ADR-0004, ADR-0005, ADR-0014, ADR-0015, ADR-0016 and ADR-0017 are now accepted on boot-experiment evidence; ADR-0002, ADR-0007 and ADR-0013 remain proposed.

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

**Booting, serial diagnostics, the first memory work and exception handling (milestones 1–2,
2026-10-10).** What exists and is verified:

- Our own kernel crate (`kernel/`) with an x86-64 `no_std` entry path reached from the reused
  `bootloader` 0.11.17 component, a controlled panic path, a bounded halt loop, and a polled 16550
  serial console (`crates/uart16550/`).
- Exception handling (`kernel/src/interrupts.rs` + `crates/x86exc/`): our own GDT, a TSS whose `IST[1]`
  points at a dedicated double-fault stack, and a 256-entry IDT. One shared assembly path saves the
  registers and calls a Rust dispatcher that reports the exception, its decoded error code, `cr2` for
  page faults, the faulting `rip`/`cs`/`rflags` and the register file before the controlled halt; a
  real `int3` trap and a real, resumed page fault are run in the boot path and asserted. A real page
  fault is still fatal.
- A hardware timer (`kernel/src/interrupts.rs`): the 8259 PICs are re-initialised so IRQ0 is vector
  32, the PIT runs at 100 Hz, every other PIC line is masked, and `sti` is enabled only after the IDT
  and PIC are ready. The handler acknowledges the PIC and counts ticks; the boot waits for real
  interrupts and asserts `NEW_OS_KERNEL_TIMER_OK`. **No scheduler, no timekeeping and no APIC** —
  nothing consumes the ticks yet.
- Read-only address translation (`crates/x86paging` + `kernel/src/paging.rs`): the kernel walks the
  page tables `CR3` points at, through the physical-memory mapping it asks the bootloader to create,
  and reserves the kernel stack and framebuffer physical frames by translating their virtual
  addresses instead of trusting the bootloader's classification. **It reads the bootloader's tables;
  it does not own them** — no `CR3` switch, no `map`/`unmap`, no isolation.
- Boot-time memory handling (`crates/physmem/`): the firmware memory map from `BootInfo` is validated
  with checked arithmetic and a defective map is fatal; usable regions feed a bitmap **physical frame**
  allocator (first 4 GiB, 4 KiB frames) that starts with every frame unavailable, applies every
  non-usable claim afterwards, reserves the first MiB and the kernel image explicitly, and hands out a
  small bootstrap allocation. There is **no deallocation** and no reclamation of bootloader memory.
- Bootable BIOS and UEFI disk images generated from the kernel ELF by `boot/image`, with a bounded
  QEMU smoke test (`tools/test/qemu-smoke.sh`) that asserts three guest-originated serial markers
  (`NEW_OS_KERNEL_EXCEPTION_OK`, `NEW_OS_KERNEL_MEMORY_OK`, `NEW_OS_KERNEL_BOOT_OK`), in the order the
  kernel is supposed to emit them.
- A pinned nightly toolchain, least-privilege CI definition, and the evidence records in
  `docs/verification/M1-BOOT-EVIDENCE.md`, `docs/verification/M2-MEMORY-EVIDENCE.md` and
  `docs/verification/M2-EXCEPTION-EVIDENCE.md`.

**Nothing else is implemented.** Specifically absent: page tables and virtual memory of our own (the
kernel translates through, but still runs on, the bootloader's tables), the APIC/I/O APIC and every
interrupt source other than the PIT timer, scheduling, threads/processes, syscalls, userspace, privilege separation, any
driver model, any storage/filesystem/network/audio/input driver, graphics or compositor, the desktop
shell, the file manager, the package manager, the installer, and the entire Windows compatibility
programme (PE parsing, loader, Win32 APIs). The kernel reports the framebuffer details the bootloader
hands over; it still draws nothing.

Do not describe any planned item above as existing until it has verification evidence.
