# ADR-0004: Systems Language and Toolchain Policy

- **Status:** accepted as owner preference; milestone-1 toolchain now verified and pinned
- **Date:** 2026-10-10

## Decision
Prefer Rust for kernel and systems code, use C when justified for interoperability or proven low-level components, and use assembly only for CPU entry/transition/context-switching tasks or similarly necessary operations. Kernel code uses `no_std` where required and defines its own entry, panic, memory allocation and runtime assumptions as the architecture develops.

## Toolchain record (milestone 1, verified)

| Item | Value |
| --- | --- |
| Channel | `nightly-2026-10-08`, pinned in `rust-toolchain.toml` |
| Compiler | rustc 1.101.0-nightly (1d81eb4ad 2026-10-07); cargo 1.101.0-nightly (f3865b2a4 2026-09-29) |
| Components | `rustfmt`, `clippy`, `llvm-tools`, `rust-src` |
| Targets | `x86_64-unknown-none` (kernel), `x86_64-unknown-uefi` (declared for the bootloader stages) |
| Kernel dependencies | `bootloader_api` 0.11.17; `uart16550` (this repository) |

Nightly is required by the chosen boot component, not by preference: `bootloader` 0.11.17 needs a
nightly compiler, `llvm-tools` for linking and `rust-src` for its `-Zbuild-std` stages. See ADR-0003.

Language split in milestone 1: Rust for all kernel and driver code; **no C**; assembly only as inline
`asm!` for the minimal privileged operations (`out`/`in` for UART port I/O and `hlt` for the halt
loop), each with an adjacent safety comment.

## Consequences
Rust can reduce some memory-safety risks but does not eliminate logic errors, unsafe-code bugs, hardware/ABI mistakes or the need for review. A `no_std` environment does not automatically provide allocation, process runtime, or OS services. Because the toolchain is nightly, dependency updates must be re-validated rather than assumed.

## Security impact
Require adjacent safety comments for `unsafe`, checked arithmetic in untrusted input handling, and tests for pure helper logic. Avoid broad lint suppression. Milestone 1 enforces this mechanically:
`unsafe_op_in_unsafe_fn` is denied workspace-wide and `clippy::undocumented_unsafe_blocks` is enabled,
with CI running clippy under `-D warnings`.

## Validation plan
Use a target-specific build and boot experiment. **Done:** the milestone-1 build and QEMU boot test
were performed with exactly the versions above; see `docs/verification/M1-BOOT-EVIDENCE.md`. The
toolchain channel is date-pinned so local builds and CI resolve the same compiler.

## References
- https://doc.rust-lang.org/rustc/platform-support.html — target support; checked 2026-10-10.
- https://doc.rust-lang.org/cargo/reference/unstable.html#artifact-dependencies — artifact dependencies, required by the boot image build; checked 2026-10-10.
