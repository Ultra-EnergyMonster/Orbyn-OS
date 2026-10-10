# Building

This file is a template until the first-task implementation confirms exact commands and tool versions. Do not publish guessed commands as verified.

## Expected initial environment

- Git
- Rust toolchain selected for the kernel target
- Target-specific Rust components and linker tools required by the chosen boot approach
- QEMU and firmware/boot assets for the selected emulation path
- Any image-generation tool selected after dependency/licence review

## Build steps

Once verified, document:

1. Supported host OS and architecture.
2. Exact Rust toolchain and target triple.
3. Required system packages/tools, versions and installation links.
4. A clean checkout setup command.
5. The kernel build command.
6. The image creation command and output location.
7. The QEMU run command with serial output and timeout.
8. Cleanup instructions that only target generated files inside the repository.

Never require a user to disable security controls or write to a physical disk to run the development build. Prefer a VM. Do not claim the instructions were tested on a clean host until they have been.
