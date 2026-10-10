# ADR-0005: QEMU as the First Integration Environment

- **Status:** accepted for initial development approach; configuration below validated by experiment
- **Date:** 2026-10-10

## Decision
Use QEMU as the first OS integration environment, with a documented machine/firmware configuration, serial capture, generated virtual storage only, and a timeout on all smoke tests. Physical PC testing comes later and requires a reviewed procedure and owner approval for disk/boot changes.

## Validated configuration (milestone 1)

| Mode | Command flags |
| --- | --- |
| BIOS | `-machine pc -accel tcg -m 256M -display none -no-reboot -serial file:<log> -drive format=raw,file=build/boot/bios.img` |
| UEFI | `-machine q35 -accel tcg -m 256M -display none -no-reboot -serial file:<log> -drive if=pflash,format=raw,unit=0,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd -drive if=pflash,format=raw,unit=1,snapshot=on,file=/usr/share/OVMF/OVMF_VARS_4M.fd -drive format=raw,file=build/boot/uefi.img` |

Implemented as `tools/test/qemu-smoke.sh <bios|uefi>` with `QEMU_TIMEOUT` (default 30 s) and
`QEMU_ACCEL` (default `tcg`, so KVM is never required and local and CI behaviour is comparable).
QEMU 8.2.2 was used; the marker assertion and the image-generation steps are in
`docs/verification/M1-BOOT-EVIDENCE.md`.

## Consequences
QEMU makes builds repeatable and failures easier to debug, but success in a VM does not prove physical GPU, Wi-Fi, power management, USB, or vendor-driver support. Only the virtual devices actually configured and exercised may be claimed supported.

## Security impact
Never attach physical host storage or use raw device passthrough in routine tests. Use disposable images and avoid privileged host actions. The smoke script only attaches generated files under
`build/boot/` and the OVMF vars store with `snapshot=on`, so the system firmware file is never
modified.

## Validation plan
The first task must boot the guest and detect a kernel-originated serial marker. A timeout alone is failure, not success. **Done for both BIOS and UEFI**, with a negative control (zeroed image) that
failed as required.

## Not yet covered
KVM-accelerated runs (only `tcg` was used), other QEMU versions, snapshots of a writable guest disk
(no guest storage driver exists yet), and any physical hardware.
