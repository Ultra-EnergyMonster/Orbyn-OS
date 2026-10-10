# First Task Prompt: Establish a Reproducible Bootable Kernel Prototype

Paste this prompt into DeepSeek/OpenCode as the first implementation task, with the repository root as the workspace. Follow `AGENTS.md` and `MASTER_PROMPT.md`.

## Objective

Implement the smallest real, reproducible x86-64 kernel prototype that can be built and booted in QEMU (when tools are available). The prototype must use our own kernel code, not the Linux or Windows kernel. It should initialise through a documented boot path, print an unmistakable success marker to a serial console, and stop safely or idle after success. This is a foundation test—not a desktop, daily-driver OS, or Windows compatibility layer.

## First inspect, then decide

1. Inspect the repository root, `git status --short --branch`, existing files, commit history, remotes, `AGENTS.md`, relevant ADRs, installed `rustc`, `cargo`, target components, linker/toolchain, QEMU, and UEFI/boot image tools. Do not assume this repo is empty; preserve existing work.
2. Record the exact host/VM build environment and existing limitations. Do not print secrets or dump all environment variables.
3. Read current official documentation for the selected Rust target, bootloader/UEFI approach, QEMU invocation and dependencies. Evaluate the currently maintained options and their licences. A Rust OS-development bootloader crate may be appropriate, but verify its current compatibility, version, toolchain requirements and experimental status. Do not silently pick a stale recipe.
4. If a material architectural decision is needed, add or update a **proposed** ADR with options, source links and a minimal test/experiment. Do not spend the task designing future subsystems.

## Implementation scope

- Establish a small Cargo workspace only if useful at this stage; avoid unnecessary crates.
- Implement the kernel entry point and the minimum boot handoff needed by the selected approach.
- Use `no_std` / `no_main` as required. Include a deliberate panic path; do not rely on a host OS standard library inside the kernel.
- Make a serial console log possible in QEMU (or use the most straightforward documented debug output supported by the chosen boot path). Output a unique marker such as `NEW_OS_KERNEL_BOOT_OK` only after the relevant initialisation path succeeds.
- Fail clearly rather than silently hanging if critical initialisation fails; where a panic handler must halt, provide a serial diagnostic first when possible.
- Add a documented build/run command and a bounded QEMU smoke-test script if the environment supports it. Capture serial output, enforce a timeout, and assert the success marker. QEMU must use only generated virtual disk/image files in a repository build directory; no physical disk access.
- Add a minimal CI workflow for checks that are actually available and useful. Use least-privilege workflow permissions. Do not add secrets. If QEMU is unavailable on a runner or is too costly, make the build/format job useful and clearly separate the unavailable boot test.
- Write or update README quickstart, `docs/development/BUILDING.md`, `docs/development/TESTING.md`, the relevant ADR, and milestone tracking. State exactly which commands were tested in this environment.
- Add tests for pure/helper logic where useful. Do not invent meaningless tests merely to increase counts.

## Out of scope

Do not implement or pretend to implement the desktop, compositor, Start menu, file manager, browser, native package manager, full driver model, physical PC installer, Windows `.exe` execution, Win32 APIs, DirectX, networking, Wi-Fi, audio, GPU acceleration, or ARM support in this task. They remain in the roadmap. Do not write to a physical disk, change host boot settings, flash firmware, disable host security, or run downloaded binaries on the host.

## Acceptance criteria

- [ ] A real x86-64 kernel entry path is present; no Linux or Windows kernel is used.
- [ ] Exact dependency versions/toolchain requirements and licences are documented or pinned as appropriate.
- [ ] A reproducible build command is documented and actually run if tools are available.
- [ ] When QEMU and boot tooling are available, a bounded boot smoke test runs against a generated artifact and detects `NEW_OS_KERNEL_BOOT_OK` in serial output.
- [ ] Format/lint/unit-test checks that are applicable actually run, and their results are reported accurately.
- [ ] A test not run due to unavailable tools is explicitly marked `NOT RUN` with the missing dependency and next step.
- [ ] No generated boot images, caches, secrets or unrelated changes are accidentally committed.
- [ ] `README.md`, building/testing docs, architecture ADR and milestone status match the actual implementation.
- [ ] Changes are reviewed with `git diff --check` and staged-file inspection.
- [ ] Changes are committed to a descriptive feature branch and pushed if a valid remote/authentication exists. Open a PR if possible. Do not bypass protections or merge if approval/policy prevents it.

## Required completion report

Report:
1. Summary of the exact boot flow and kernel entry point.
2. Files added/changed.
3. Exact environment/toolchain and commands executed.
4. Actual build/test/QEMU results, including relevant serial lines.
5. Any checks not run and the reason.
6. Architectural trade-offs and dependencies/versions/licences selected.
7. Known limitations and the next smallest milestone.
8. Branch name, commit SHA, push status and PR URL if available.

Never state that a build or boot passed without real output. If the environment blocks a boot test, report partial completion truthfully rather than fabricating a result or expanding the task into unrelated code.
