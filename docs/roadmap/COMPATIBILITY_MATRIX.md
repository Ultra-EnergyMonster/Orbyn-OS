# Compatibility Matrix

No hardware or third-party Windows application compatibility is verified by this blueprint. Add rows
only after a test actually runs. Avoid broad labels such as “NVIDIA support” or “Windows apps
supported.”

Only the boot paths below have been tested. They prove that a generated image boots in QEMU and that
our kernel's own serial code runs; they say nothing about physical hardware, storage, graphics,
networking or any Windows application.

| Category | Device/application + exact version | OS commit/build | Runtime/driver version | Environment | Scenario tested | Result | Limitations/logs/date |
|---|---|---|---|---|---|---|---|
| Boot/VM (legacy BIOS) | New OS kernel prototype (M1), own entry path | branch `feat/kernel-m1-bootable-prototype`; image built by `cargo build -p boot-image` | `bootloader`/`bootloader_api` 0.11.17; in-repo `uart16550` guest driver | QEMU 8.2.2, `-machine pc`, `-accel tcg`, `-m 256M`, SeaBIOS, `-serial file:` | Boot `build/boot/bios.img` and assert the guest kernel's serial marker `NEW_OS_KERNEL_BOOT_OK` | **PASS** — marker emitted by the guest kernel's own COM1 code | Virtual devices only; boot images not byte-for-byte reproducible; `docs/verification/M1-BOOT-EVIDENCE.md`; 2026-10-10 |
| Boot/VM (UEFI) | New OS kernel prototype (M1), own entry path | same as above | `bootloader`/`bootloader_api` 0.11.17; in-repo `uart16550` guest driver | QEMU 8.2.2, `-machine q35`, OVMF 2024.02 (Ubuntu package) with `unit=1,snapshot=on`, `-accel tcg`, `-m 256M` | Boot `build/boot/uefi.img` and assert the same guest marker | **PASS** — marker emitted by the guest kernel's own COM1 code | Virtual devices only; no Secure Boot engagement tested; UEFI image is not byte-for-byte reproducible (FAT/ESP metadata); `docs/verification/M1-BOOT-EVIDENCE.md`; 2026-10-10 |
| Kernel behaviour | Boot marker absent case (negative control) | same as above | zero-filled image, same size | QEMU 8.2.2, `-machine pc`, `QEMU_TIMEOUT=8` | Confirm the smoke test fails when no kernel boots | **PASS (expected failure)** — exit 1, both markers reported missing | Control run only; not a compatibility claim; 2026-10-10 |
| Kernel memory | Boot memory-map validation and bootstrap frame allocation (M2 first slice) | branch `feat/m2-boot-memory-and-frame-allocator` | in-repo `crates/physmem`, no new third-party dependency | QEMU 8.2.2, `-machine pc` (BIOS) and `-machine q35` + OVMF (UEFI), `-accel tcg`, `-m 256M` | Validate `BootInfo` regions, mark usable frames, reserve the first MiB and the kernel image, allocate 8 bootstrap frames, assert `NEW_OS_KERNEL_MEMORY_OK` then `NEW_OS_KERNEL_BOOT_OK` | **PASS** — 254 MiB usable mapped to 64,951 free frames, 159 low-memory frames reclaimed, bootstrap frames above 1 MiB | Physical frames only: no virtual memory, no page tables of our own, no deallocation, no fault handling; kernel stack/framebuffer not reservable by address; `docs/verification/M2-MEMORY-EVIDENCE.md`; 2026-10-10 |
| Physical hardware | none | — | — | — | — | Not tested | No physical-hardware testing has been performed or approved |
| Windows applications | none | — | no runtime exists | — | — | Not tested | No compatibility runtime, PE parser or Win32 API implementation exists |

## Required categories over time

- QEMU machine and firmware versions.
- Virtual block storage and each filesystem operation.
- Virtual networking and protocol scenarios.
- Input/display modes and supported resolutions.
- Native applications, installer/update/uninstall cases.
- Windows executable recognition, loader, named apps and installer scenarios.
- Windows graphics APIs, audio, networking and input where implemented.
- Physical hardware by exact make/model/firmware/driver version.
- Gaming, DRM and anti-cheat by exact game/build and test scenario; never infer support from another title.
