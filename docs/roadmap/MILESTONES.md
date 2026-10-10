# Milestones and Acceptance Checklist

**Status at blueprint creation:** all implementation milestones are planned; no OS implementation is yet verified by this blueprint. Update this file only after real repository evidence exists.

Use a status of `PLANNED`, `IN PROGRESS`, `BLOCKED`, or `VERIFIED`. For a VERIFIED milestone, link to the commit/PR and record the exact tests and environments. A code merge alone does not prove that a feature works.

## M0 — Repository and reproducible developer workflow

- [ ] Confirm project licence decision or mark it explicitly undecided.
- [ ] Establish build environment documentation and pinned toolchain as needed.
- [ ] Add `.gitignore`, contribution guidance, ADR template and basic CI with least privilege.
- [ ] Record dependency provenance and supported development host(s).
- [ ] Verify clean checkout instructions on at least one development environment.
- **Gate:** clean checkout can run documented checks; no secrets or accidental binaries committed.

## M1 — Bootable x86-64 kernel skeleton

- [ ] Document UEFI/bootloader/kernel handoff.
- [ ] Build a real kernel with an explicit entry point and controlled panic path.
- [ ] Print serial diagnostics and `NEW_OS_KERNEL_BOOT_OK` only after boot path succeeds.
- [ ] Provide bounded QEMU smoke-test script when tooling permits.
- [ ] Keep output images in ignored build directories; no physical disk writes.
- **Gate:** reproducible build; QEMU boot marker observed, or the exact boot blocker is documented as NOT RUN.

## M2 — Boot info, memory and fault diagnostics

- [ ] Validate boot information and memory map safely.
- [ ] Establish early physical page management and virtual memory/page tables.
- [ ] Add exception reporting and controlled fatal handling.
- [ ] Add tests for pure address/map logic and malformed inputs.
- **Gate:** tests pass; faults produce useful diagnostics in QEMU; no unsupported memory-management claims.

## M3 — Interrupts, timers, basic task/process model

- [ ] Establish IDT/exception/interrupt handling and timer source.
- [ ] Implement safe interrupt-safe synchronisation primitives as required.
- [ ] Document scheduler/task invariants and CPU state handling.
- [ ] Add deterministic tests where possible.
- **Gate:** documented interrupt/timer/task behaviours are observed in QEMU; no unsupported SMP claim.

## M4 — Device discovery and first virtual devices

- [ ] Add minimum ACPI/PCI support as justified by architecture decisions.
- [ ] Select and implement a virtual block device path.
- [ ] Enumerate a known QEMU device configuration and report absent/unsupported devices safely.
- [ ] Begin driver model ADR and record isolation/security choices.
- **Gate:** tests run against documented QEMU configuration; no blanket claim of physical PC driver support.

## M5 — Storage, filesystem and native process ABI

- [ ] Define file and block IO boundaries.
- [ ] Implement or integrate a suitable filesystem with licence/provenance review.
- [ ] Define a minimal executable/process ABI and system call boundary.
- [ ] Add robust tests for malformed filesystem metadata and error cases.
- **Gate:** test files can be written/read in disposable virtual storage and a minimal user-mode test process runs.

## M6 — Userland, services and native application installation

- [ ] Establish init/service startup and an error/recovery model.
- [ ] Define native package metadata, verification/signing approach and removable install model.
- [ ] Add a GUI-independent package management API and tests.
- [ ] Ensure offline/local operation is possible for core tasks.
- **Gate:** a test native application can be installed, launched, updated/reverted or removed through the documented native path.

## M7 — Display, input and first functional desktop

- [ ] Create a real display/input abstraction and document supported modes.
- [ ] Build a functional windowing/compositing strategy.
- [ ] Implement a small but working shell/launcher and settings foundation.
- [ ] Implement a file manager with Downloads, properties, file associations and keyboard accessibility.
- [ ] Create a design system and UX review checklist; no copied proprietary Windows assets.
- **Gate:** interactive GUI is booted and tested in a documented environment; no static mockup presented as the real desktop.

## M8 — Safe browser downloads and executable association UX

- [ ] Establish browser/download integration or documented download directory conventions.
- [ ] Recognise PE/COFF `.exe` as an application file type without requiring manual permission changes.
- [ ] Double-click routes a file only to a registered compatibility handler, not to a host shell.
- [ ] Add provenance/signature/status display, first-run warning and clear unsupported-runtime dialog.
- [ ] Do not auto-execute on download completion; avoid unconsented remote scanning/uploads.
- [ ] Add UI, association and malicious/malformed file tests.
- **Gate:** the GUI flow works end-to-end. At this stage it may truthfully report “Windows support not implemented”; it must not claim it can run the program until the runtime does.

## M9 — PE parser and loader research/prototype

- [ ] Implement defensive PE parsing in a testable library separated from execution.
- [ ] Test malformed offsets, integer overflow, invalid sections, truncated files and unsupported machine types.
- [ ] Define loader, relocation, import, exception and ABI limitations through ADRs.
- [ ] Add fuzzing when practical.
- **Gate:** parser tests/fuzz corpus pass; parsing a file is not represented as running it.

## M10 — Minimal Windows user-mode compatibility

- [ ] Define realistic app-class target(s), test fixtures and API scope.
- [ ] Establish process/thread, memory, file/handle, time, environment, exception and library requirements for the selected test apps.
- [ ] Implement a tiny licensed/provenance-safe test app or known redistributable test target first.
- [ ] Add GUI launch and useful per-app failure reporting.
- **Gate:** named app(s) run with documented versions and tests. Do not claim broad Windows compatibility.

## M11 — Broader application and graphics compatibility

- [ ] Prioritise user-demanded application categories using a published matrix.
- [ ] Evaluate the feasibility and licensing of integrating/porting existing compatibility components; don't assume Wine/Proton runs unchanged on a new kernel.
- [ ] Develop graphics/audio/network API layers according to documented targets.
- [ ] Evaluate installers inside isolated prefixes/containers, with safe uninstall/rollback.
- **Gate:** repeatable app tests and published limitations. Gaming and anti-cheat compatibility remain per-title, not universal.

## M12 — Security hardening, updates and recovery

- [ ] Threat model, permission model and sandbox boundaries reviewed.
- [ ] Verified update metadata and signatures, rollback/recovery, and key management are designed and tested.
- [ ] Add parser fuzzing and security regression tests.
- [ ] Document telemetry/network endpoints and verify default-off diagnostic behaviour.
- [ ] Independent security review before production claims.
- **Gate:** documented test evidence and resolved/triaged security findings; no “secure by default” claim without a defined baseline.

## M13 — Physical hardware programme and beta readiness

- [ ] Publish exact tested hardware/device matrix.
- [ ] Run install, update, rollback, suspend/resume, storage recovery and peripheral tests on selected physical hardware with approval.
- [ ] Establish accessibility, crash reporting opt-in, support and security response processes.
- [ ] Create reproducible images/releases, checksums, provenance/SBOM and release signing process.
- [ ] Document known issues and recovery path.
- **Gate:** beta criteria reviewed by humans; do not market as a general daily-driver replacement until user-critical functions and compatibility are proven.

## Cross-cutting acceptance rules

- Every feature has a measurable definition of done and appropriate automated tests.
- Every milestone has a known environment and limitations.
- Every major architectural change is documented in an ADR.
- No stage is completed by writing only UI, a stub, or docs that overstate capability.
- No release contains unreviewed secrets, uncertain-licence material or destructive installer behaviour.
