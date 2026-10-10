# New OS — Project Blueprint

> Working title only. The final OS name and licence are owner decisions.

This repository blueprint describes an ambitious, independent x86-64-first desktop operating system. The OS will have its own kernel and architecture, a distinctive premium desktop with Windows-like usability, privacy-first defaults, no preinstalled bloatware or advertising, and a long-term goal of running a meaningful selection of Windows applications through a real compatibility implementation.

**This repository currently contains planning documents only.** It does not yet contain an implemented kernel, boot image, desktop, driver stack, installer or Windows compatibility runtime. Do not mistake the blueprint for a working OS.

## Start here

1. `MASTER_PROMPT.md` — the full product and engineering charter for DeepSeek/OpenCode.
2. `AGENTS.md` — standing rules for coding agents.
3. `FIRST_TASK.md` — the first, deliberately narrow implementation task.
4. `REPOSITORY_STRUCTURE.md` — proposed code and documentation layout.
5. `docs/roadmap/MILESTONES.md` — milestone sequence and acceptance gates.
6. `docs/verification/VERIFICATION.md` — evidence and test-reporting policy.
7. `docs/architecture/adr/` — architectural decisions and decisions that still need experiments.
8. `docs/security/THREAT_MODEL.md` and `docs/security/DOWNLOADS_AND_EXECUTABLES.md` — initial security principles.

## Product principles

- Own kernel; not the Linux or Windows kernel.
- x86-64 and QEMU first; ARM64 only after the baseline is stable.
- Rust first for systems code, with C and assembly where justified.
- Open-source components may be reused after maintenance, target, licence and security review.
- Privacy, local operation, and user control by default.
- Normal GUI installation/launch workflows; never require end users to type `chmod +x` to launch a Windows `.exe`.
- Honest capability statements: `.exe` recognition, PE loading, Win32 API compatibility, graphics support, game support and Windows-driver support are separate capabilities.
- Work in small, testable feature branches. Never push secrets or bypass review protections.

## Status vocabulary

Use `planned`, `in progress`, `implemented`, `build verified`, `unit tested`, `QEMU boot tested`, `tested on physical hardware`, or `tested with <named application>` as appropriate. Do not collapse these into one vague “done” status.

## Licence status

The project licence is deliberately undecided. See `LICENCE-STATUS.md`. Do not assume code is licensed for redistribution until the owner makes a decision and the appropriate licence file is added.
