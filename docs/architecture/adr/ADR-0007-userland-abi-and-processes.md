# ADR-0007: Userland ABI, Process Model and Executable Formats

- **Status:** proposed; defer commitment until memory/process design has evidence
- **Date:** 2026-10-10

## Context
Native user applications and future Windows compatibility require a stable boundary between user space and the kernel. Process, thread, handle, memory mapping, IPC and exception semantics are foundational.

## Decision
Define a minimal, documented native ABI and syscall boundary only after the kernel has a clear memory and task model. Keep the native ABI distinct from Windows API emulation. Treat PE/COFF parsing/validation as a separate library and stage from executable loading and API compatibility. Do not make `.exe` support a kernel feature by assumption.

## Consequences
An early ABI mistake creates expensive compatibility debt. Avoid promising ABI stability before it is intentional. Compatibility processes may require a substantial userspace subsystem and service emulation.

## Security impact
Process boundaries and handles are security boundaries. Validate user pointers, sizes and permissions at all kernel entry points; never trust userspace-provided lengths or object IDs.

## Validation plan
Write ABI invariants, syscall conformance tests, parser tests and compatibility-test targets before claiming support.
