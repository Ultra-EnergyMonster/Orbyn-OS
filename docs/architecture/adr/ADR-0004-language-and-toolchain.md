# ADR-0004: Systems Language and Toolchain Policy

- **Status:** accepted as owner preference; exact versions remain to be verified
- **Date:** 2026-10-10

## Decision
Prefer Rust for kernel and systems code, use C when justified for interoperability or proven low-level components, and use assembly only for CPU entry/transition/context-switching tasks or similarly necessary operations. Kernel code uses `no_std` where required and defines its own entry, panic, memory allocation and runtime assumptions as the architecture develops. Pin or document an exact compiler/toolchain after proving the boot path.

## Consequences
Rust can reduce some memory-safety risks but does not eliminate logic errors, unsafe-code bugs, hardware/ABI mistakes or the need for review. A `no_std` environment does not automatically provide allocation, process runtime, or OS services. Dependencies must support the actual target.

## Security impact
Require adjacent safety comments for `unsafe`, checked arithmetic in untrusted input handling, and tests for pure helper logic. Avoid broad lint suppression.

## Validation plan
Use a target-specific build and boot experiment. Document installed components, compiler version, target triple, linker/LLVM requirements and CI equivalent.

## References
- https://doc.rust-lang.org/ — official Rust documentation; verify target support/toolchain instructions at implementation time.
