# Agent Instructions

These rules apply to every AI coding agent and contributor working in this repository. Read `README.md`, `docs/architecture/OVERVIEW.md` when present, relevant ADRs, and `docs/roadmap/MILESTONES.md` before substantial changes. The full product charter is in `MASTER_PROMPT.md`.

## Mission

Build a new x86-64-first desktop operating system with its own kernel and independently implemented OS architecture—not the Linux kernel and not the Windows kernel. Use Rust primarily, with C and assembly where justified. The long-term user experience should be familiar and click-first like Windows, but with an original premium design, privacy by default, no bloatware, no advertising, no Recall-style activity capture, local-first operation, and user control.

A core product goal is normal GUI handling of Windows `.exe` downloads: download in a browser, find in Downloads, double-click, and launch or install through a real compatibility runtime without terminal commands or manual `chmod`. This requires actual PE/COFF, process, Windows API, security and installer compatibility work over time. Never imply that file association alone means Windows compatibility exists.

## Non-negotiable engineering rules

1. **Make small, verifiable changes.** Work on one milestone or task at a time. Don't scaffold the entire future OS with hundreds of empty modules.
2. **Inspect before editing.** Confirm repo root, `git status --short --branch`, current branch, recent commits, remotes, existing user changes, toolchain and test availability.
3. **Protect existing work.** Never discard uncommitted changes. Do not run `git reset --hard`, `git clean -fdx`, force push, destructive rebase, or overwrite a file without understanding its current contents and need.
4. **Never perform host-destructive actions autonomously.** Owner approval is required for privileged host changes, physical disk writes, bootloader/boot configuration changes, flashing firmware, writing USB boot media, irreversible operations, or operations that could lose data. Use QEMU images created in the repository's build/output directory for routine testing.
5. **Do not leak secrets.** Never print or commit API tokens, GitHub PATs, SSH private keys, signing keys, passwords, or all environment variables. Never put secrets in logs, prompts, issues, test fixtures or model context.
6. **Use least privilege.** GitHub workflows default to read-only permissions. Do not expose secrets to untrusted pull-request code. Pin third-party Actions to immutable SHAs where practical. Do not bypass branch protection.
7. **Use feature branches.** Use a descriptive branch, coherent commits, checks, reviewable PR and push if authentication is configured. Merge only after required checks pass and repository policy permits. If owner approval is required, leave a ready PR.
8. **Verify sources.** For version-sensitive APIs, dependencies, boot formats and licences, consult current official/upstream documentation. Do not trust model-generated symbols or stale tutorials without checking them.
9. **Minimise `unsafe`.** Each Rust `unsafe` block needs a nearby safety comment describing the invariant and why the operation is sound. Keep unsafe code small and centralised. Avoid `unwrap()`/`expect()` on recoverable/untrusted-input paths.
10. **Handle untrusted inputs defensively.** Use checked arithmetic and explicit bounds checks for files, images, PE headers, package metadata and hardware tables. Fuzz parsers once they exist.
11. **Don't fake features.** No UI or stub may be described as a working OS feature unless it functions. Label stubs and unimplemented code explicitly.
12. **Privacy by default.** No advertising, covert telemetry, Recall-like capture, screenshots, keylogging, behavioural profiling or silent uploads. Optional diagnostics must be opt-in and transparent.
13. **No universal compatibility claims.** Record exact app/device/version/test environment. Windows kernel drivers, DirectX, anti-cheat, DRM and proprietary dependencies may remain unsupported.
14. **Never test unknown executables on the host.** Use safe test fixtures and disposable VMs. Don't globally weaken host security to make a test pass.
15. **Report truthfully.** Distinguish written, compiled, unit-tested, boot-tested in QEMU, hardware-tested, and tested with a named application. A test that was not run is `NOT RUN`, not passed.

## Per-task workflow

1. Read the relevant docs and ADRs; locate the task's definition of done.
2. Check working tree and tools. Do not overwrite existing work.
3. Make the smallest coherent change on a feature branch.
4. Add/update tests and docs with code changes.
5. Run applicable checks, record exact commands and results, and investigate failures rather than hiding them.
6. Review `git diff --check`, the full diff, staged files and secret-sensitive content.
7. Update milestone/ADR status only with evidence.
8. Commit and push the feature branch if configured. Open/update PR if supported; never bypass protection.
9. Report changes, files, test evidence, tests not run, limitations, commit SHA and push/PR status.

## First task and milestones

Follow `FIRST_TASK.md`. Do not begin desktop UI, a Windows compatibility runtime, broad physical drivers, or a production installer until the first bootable kernel task has a truthful result. If required tools are unavailable, do as much as can be verified, document the blocker, and do not invent output.

## Do not

- Use Linux or Windows kernels under a new name.
- Copy proprietary Windows code/assets or imply Microsoft affiliation.
- Add telemetry or network services without a documented, explicit product need and privacy review.
- Commit generated binaries, downloaded firmware, secrets, or unclear-licence material without justification.
- write to host physical disks or modify the host's boot setup as part of routine development.
- suppress warnings, disable safety checks, or weaken security just to obtain a green build.
- claim CI, push, PR, merge, test, boot, benchmark or compatibility success unless it actually occurred.
