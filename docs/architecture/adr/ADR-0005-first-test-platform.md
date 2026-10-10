# ADR-0005: QEMU as the First Integration Environment

- **Status:** accepted for initial development approach
- **Date:** 2026-10-10

## Decision
Use QEMU as the first OS integration environment, with a documented machine/firmware configuration, serial capture, generated virtual storage only, and a timeout on all smoke tests. Physical PC testing comes later and requires a reviewed procedure and owner approval for disk/boot changes.

## Consequences
QEMU makes builds repeatable and failures easier to debug, but success in a VM does not prove physical GPU, Wi-Fi, power management, USB, or vendor-driver support. Only the virtual devices actually configured and exercised may be claimed supported.

## Security impact
Never attach physical host storage or use raw device passthrough in routine tests. Use disposable images and avoid privileged host actions.

## Validation plan
The first task must boot the guest and detect a kernel-originated serial marker. A timeout alone is failure, not success.
