# ADR-0002: Kernel Architecture

- **Status:** proposed; decide after a focused architecture review/experiment
- **Date:** 2026-10-10

## Context
The project requires its own kernel and wants a maintainable, secure path from a bootable x86-64 prototype to a desktop OS. No kernel architecture has yet been selected by the owner.

## Options considered
- **Monolithic kernel:** many core services and drivers execute in kernel space; simpler initial performance path but larger privileged attack surface and less fault isolation.
- **Microkernel:** move more services/drivers to user space behind message-passing interfaces; better isolation potential but substantially greater complexity in IPC, performance analysis and early bootstrapping.
- **Hybrid/modular approach:** a small initial kernel with explicit interfaces and a gradual move of selected services out of kernel space; may balance scope and separation but requires careful boundaries.

## Decision
Do not lock the final approach from this template alone. During milestone 1, keep the prototype small and design module boundaries to avoid unnecessary coupling. Before implementing drivers, processes or complex IPC, compare the options against the available developer capacity, threat model, driver needs, debugging complexity and performance needs. Record the result and supporting evidence in a new accepted revision or superseding ADR.

## Security and privacy impact
The decision affects trusted computing base size, fault isolation, attack surface and ability to sandbox services. Document which components run privileged.

## Validation plan
Prototype the simplest path to a reproducible QEMU boot; separately evaluate driver/service boundaries before those components become difficult to move.
