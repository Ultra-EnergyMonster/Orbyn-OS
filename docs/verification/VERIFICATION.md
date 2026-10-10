# Verification and Evidence Policy

## Why this exists

Kernel and OS development creates a high risk of convincing-looking but incorrect output: an image file that was never booted, a build script that was never run, a mocked UI, or an agent claiming success based on its own generated code. Every capability claim must be backed by evidence proportionate to the claim.

## Evidence levels

Use these labels precisely:

1. **Specified** — requirements/design exist; no implementation claim.
2. **Implemented** — code exists; may not yet compile.
3. **Build verified** — exact build command completed successfully in a stated environment.
4. **Unit tested** — named tests were executed and passed; list suite/command.
5. **Integration tested** — components were exercised together in a reproducible environment.
6. **QEMU boot tested** — generated artifact actually booted in a specified QEMU version/configuration and emitted an expected marker or passed a defined test.
7. **Physical-hardware tested** — exact PC/device/firmware configuration was used; list what was and was not tested.
8. **Application compatibility tested** — exact application name/version, runtime build, test steps, observed result and known limitations are recorded.
9. **Security reviewed** — scope, reviewer, date, findings and remediations are recorded. Automated linting alone is not a security review.

Never substitute a higher status for a lower one. `cargo build` does not prove a kernel boots. A QEMU boot does not prove physical Wi-Fi or GPU support. Recognising `.exe` does not prove that it can run.

## Required test report fields

For each task/milestone record:

- Commit SHA and branch/PR.
- Host OS, CPU architecture and relevant tool versions.
- Toolchain and target triple.
- Exact commands executed.
- Exit codes and concise relevant output.
- QEMU version, machine, firmware/boot configuration, virtual devices and timeout, if used.
- What the test actually proves.
- What it does **not** prove.
- Not-run/blocked checks and exact reason.
- Known defects and follow-up issue/step.

## Suggested checks as the project grows

Only include commands for components that exist. Confirm command names and flags against installed tools/current docs.

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings` where all targets support it and lints are suitably configured.
- `cargo test --workspace` for host-testable crates; target-specific crates may require separate commands.
- The target-specific kernel build documented in `docs/development/BUILDING.md`.
- A bounded QEMU boot script that captures serial output and searches for an explicit success marker after the intended initialisation path.
- Parser unit tests, property tests and fuzz tests for file formats and other untrusted input.
- `git diff --check`, full diff review, staged-file listing, and secret/large-binary scan before commit.

## QEMU safety

- Keep generated images under an ignored build directory.
- Do not attach real host block devices to the guest.
- Do not use a physical disk path, host raw device, host boot partition or passthrough storage device in smoke tests.
- Always use a timeout and capture serial output. A timeout by itself is not a pass.
- Prefer disposable images/snapshots. Delete generated files only within the project's known build directory and only when their provenance is clear.

## Host safety

Owner approval is mandatory before writing physical disks/USB media, changing host boot settings, flashing firmware, running privileged installation commands, or doing destructive cleanup. A routine build should not require those actions.

## Compatibility evidence

The compatibility matrix needs these columns: application/device, exact version/model, OS commit/build, compatibility runtime, environment (QEMU or physical), test scenario, outcome, severity of limitations, logs/artifact link, date, and tester. Distinguish “opens”, “basic function works”, “installer succeeds”, “updates”, “uninstall”, “graphics acceleration”, “audio”, “network”, and “anti-cheat/DRM” instead of one green compatibility indicator.
