# New OS — Project Blueprint

> Working title only. The final OS name and licence are owner decisions.

This repository blueprint describes an ambitious, independent x86-64-first desktop operating system. The OS will have its own kernel and architecture, a distinctive premium desktop with Windows-like usability, privacy-first defaults, no preinstalled bloatware or advertising, and a long-term goal of running a meaningful selection of Windows applications through a real compatibility implementation.

**Repository contents today:** planning documents plus a milestone-1 bootable x86-64 kernel prototype that has its own kernel entry path and boots in QEMU. There is no desktop, window manager, driver stack, filesystem, userspace, installer, package manager or Windows compatibility runtime yet. Do not mistake the blueprint for a working OS.

## Current status

Status vocabulary is defined in `docs/verification/VERIFICATION.md`. Nothing below claims more than
what has been observed.

| Area | Status |
| --- | --- |
| M1 bootable kernel skeleton | Implemented; build verified; unit tested; **QEMU boot tested (BIOS and UEFI)** |
| Kernel memory/paging/interrupts/scheduling | Not started (M2/M3) |
| Drivers beyond the debug UART | Not started (M4) |
| Userspace, filesystem, packages | Not started (M5/M6) |
| Desktop, file manager, settings | Not started (M7) |
| Windows `.exe` recognition, PE loading, Win32 APIs | Not started (M8–M11) |

The milestone-1 prototype is deliberately tiny: firmware hands control to the reused `bootloader`
0.11 component, which loads our kernel ELF and jumps to our `_start`/`kernel_main`. The kernel then
initialises COM1, prints a summary of the boot handoff, prints `NEW_OS_KERNEL_BOOT_OK` on the serial
console, and halts. It does not manage memory, enable interrupts, schedule tasks or run userspace.

### Building and boot-testing the prototype

Requires a Linux host with `rustup`, `git` and QEMU. No `sudo` is needed to build; installing QEMU
does need administrator rights on the host.

```sh
# 1. Toolchain: rustup installs the pinned nightly from rust-toolchain.toml automatically.
# 2. Build the kernel and generate bootable images under build/boot/:
cargo build -p boot-image

# 3. Boot-test the generated image in QEMU and assert the guest kernel's serial marker:
./tools/test/qemu-smoke.sh bios    # legacy BIOS + SeaBIOS
./tools/test/qemu-smoke.sh uefi    # UEFI + OVMF  (needs the 'ovmf' package)
```

Debian/Ubuntu package needed for the boot test: `qemu-system-x86` (and `ovmf` for the UEFI mode).

Exact tested versions and commands are recorded in `docs/development/BUILDING.md`,
`docs/development/TESTING.md` and `docs/verification/M1-BOOT-EVIDENCE.md`. Where a tool was missing
or a check could not run, that is recorded as `NOT RUN` rather than passed.

## Start here

1. `MASTER_PROMPT.md` — the full product and engineering charter for DeepSeek/OpenCode.
2. `AGENTS.md` — standing rules for coding agents.
3. `FIRST_TASK.md` — the first, deliberately narrow implementation task.
4. `REPOSITORY_STRUCTURE.md` — proposed code and documentation layout.
5. `docs/roadmap/MILESTONES.md` — milestone sequence and acceptance gates.
6. `docs/verification/VERIFICATION.md` — evidence and test-reporting policy.
7. `docs/verification/M1-BOOT-EVIDENCE.md` — what the milestone-1 boot test actually proved.
8. `docs/architecture/adr/` — architectural decisions and decisions that still need experiments.
9. `docs/security/THREAT_MODEL.md` and `docs/security/DOWNLOADS_AND_EXECUTABLES.md` — initial security principles.

## Where the code lives

```text
kernel/                 our kernel: entry point, serial diagnostics, panic path, halt
crates/uart16550/       polled 16550 UART driver used by the kernel for diagnostics
boot/image/             builds BIOS and UEFI disk images from the kernel ELF
tools/test/qemu-smoke.sh bounded QEMU boot smoke test against a generated image
build/                  generated images and test logs (gitignored, never committed)
```

## Product principles

- Own kernel; not the Linux or Windows kernel.
- x86-64 and QEMU first; ARM64 only after the baseline is stable.
- Rust first for systems code, with C and assembly where justified.
- Open-source components may be reused after maintenance, target, licence and security review.
- Privacy, local operation, and user control by default.
- Normal GUI installation/launch workflows; never require end users to type `chmod +x` to launch a Windows `.exe`.
- Honest capability statements: `.exe` recognition, PE loading, Win32 API compatibility, graphics support, game support and Windows-driver support are separate capabilities.
- Work in small, testable feature branches. Never push secrets or bypass review protections.

## Licence status

The project licence is deliberately undecided. See `LICENCE-STATUS.md`. Do not assume code is licensed for redistribution until the owner makes a decision and the appropriate licence file is added.

> **Inconsistency to resolve (owner decision, flagged during milestone 1):** the repository currently
> contains a root `LICENSE` file granting the MIT licence, while `LICENCE-STATUS.md` and ADR-0013
> state that no licence has been chosen. Only the owner can decide; this milestone neither added nor
> changed licence terms, and no `license` field was added to any `Cargo.toml`.
