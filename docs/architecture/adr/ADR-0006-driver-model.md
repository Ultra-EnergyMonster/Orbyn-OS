# ADR-0006: Driver Model and Initial Hardware Support

- **Status:** proposed
- **Date:** 2026-10-10

## Context
The long-term ambition is broad hardware support, but a new kernel will not automatically run Windows drivers. Driver isolation and ABI choices affect security, compatibility and architectural complexity.

## Options considered
- Kernel-mode drivers with a narrow, documented interface.
- User-mode drivers/services for suitable devices and protocols.
- A hybrid model with only highly time-sensitive/core functions privileged.

## Decision
Defer the final model until process/isolation requirements are clearer. Start with QEMU devices and documented interfaces. Evaluate ACPI and PCI enumeration, virtio or another justified virtual block/network baseline, then storage/input/network/display incrementally. Do not claim vendor GPU or Wi-Fi support based on virtual devices.

## Security impact
Treat drivers as high-risk code. Prefer isolation for complex or untrusted components where technically feasible. Do not load arbitrary Windows kernel drivers.

## Validation plan
Each device gets a named test configuration and test matrix row. The driver model ADR must be accepted before a broad physical-driver programme begins.
