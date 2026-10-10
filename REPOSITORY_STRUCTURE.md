# Proposed Repository Structure

This is the target direction, not an instruction to create all directories immediately. Create a directory or crate when there is an actual component to put in it. Keep the first kernel prototype deliberately small.

```text
new-os/
├── README.md
├── MASTER_PROMPT.md
├── AGENTS.md
├── FIRST_TASK.md
├── CONTRIBUTING.md
├── LICENCE-STATUS.md
├── Cargo.toml                   # workspace root when useful
├── Cargo.lock                   # when a Cargo workspace exists
├── rust-toolchain.toml          # pin only after toolchain choice is verified
├── .cargo/config.toml           # target/linker configuration when needed
├── .gitignore
├── Makefile or justfile         # optional thin wrapper for reproducible tasks
├── boot/                        # bootloader integration / image builder
│   ├── README.md
│   └── image/                   # scripts/config; generated images ignored
├── kernel/                      # own kernel code
│   ├── src/
│   │   ├── main.rs              # only if a single crate is appropriate initially
│   │   ├── arch/x86_64/          # entry, CPU, interrupts, paging, context switch
│   │   ├── boot/                 # boot info/handoff parsing
│   │   ├── memory/               # physical/virtual memory and allocation
│   │   ├── sched/                # scheduler and task model
│   │   ├── sync/                 # locks/atomics and IRQ-aware primitives
│   │   ├── interrupt/            # exception and interrupt dispatch
│   │   ├── syscall/               # syscall ABI and dispatch
│   │   ├── panic.rs
│   │   └── log.rs
│   └── tests/                    # target-feasible kernel tests
├── abi/                         # shared, versioned ABI/interface definitions
├── crates/                      # shared code only where ownership/boundaries justify it
│   ├── boot-protocol/
│   ├── pe-format/                # later: parser only, no execution
│   └── common-types/
├── drivers/                     # own driver interfaces and initial drivers
│   ├── bus/                      # PCI, ACPI abstractions as implemented
│   ├── storage/
│   ├── input/
│   ├── net/
│   ├── display/
│   └── audio/
├── services/                    # user-mode system services, once userland exists
│   ├── init/
│   ├── device-manager/
│   ├── network-manager/
│   ├── package-manager/
│   ├── update-manager/
│   └── compatibility-manager/
├── userland/                    # native runtime, libc-like API, utilities
├── desktop/                     # shell, window manager/compositor, settings, file manager
│   ├── shell/
│   ├── file-manager/
│   ├── settings/
│   └── design-system/
├── compat/
│   └── windows/                 # PE loader, API subsystems, compatibility runtime (later)
│       ├── pe-loader/
│       ├── win32/                # only when APIs are implemented
│       ├── graphics/
│       └── docs/
├── apps/                        # small native test apps and eventual bundled essentials
├── tools/
│   ├── build/
│   ├── image/                   # image creation; no physical-device writers by default
│   └── test/
├── tests/
│   ├── unit/
│   ├── integration/
│   ├── boot/
│   ├── compatibility/
│   └── fixtures/                 # only redistributable, provenance-documented fixtures
├── docs/
│   ├── architecture/
│   │   ├── OVERVIEW.md
│   │   └── adr/
│   ├── development/
│   │   ├── BUILDING.md
│   │   ├── TESTING.md
│   │   └── RELEASING.md
│   ├── roadmap/
│   │   ├── MILESTONES.md
│   │   └── COMPATIBILITY_MATRIX.md
│   ├── security/
│   │   ├── THREAT_MODEL.md
│   │   └── DOWNLOADS_AND_EXECUTABLES.md
│   ├── UX/
│   │   └── DESKTOP_PRINCIPLES.md
│   └── verification/
│       └── VERIFICATION.md
└── .github/
    ├── workflows/               # least-privilege CI; grow with real code/tests
    ├── ISSUE_TEMPLATE/
    └── PULL_REQUEST_TEMPLATE.md
```

## Initial repository minimalism

For milestone 1, the implemented structure may only need:

```text
new-os/
├── README.md
├── AGENTS.md
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml            # only if a verified toolchain pin is required
├── .cargo/config.toml             # only if verified as necessary
├── kernel/
├── boot/                          # only if needed by selected boot approach
├── tools/test/qemu-smoke.sh
├── docs/architecture/OVERVIEW.md
├── docs/architecture/adr/
├── docs/development/BUILDING.md
├── docs/development/TESTING.md
├── docs/roadmap/MILESTONES.md
└── .github/workflows/ci.yml
```

Avoid a workspace split until it adds a clear boundary or test/build benefit. Build artifacts, disk images, downloaded toolchains, generated UEFI files and emulator output should be in ignored directories (such as `target/` or `build/`) unless a generated artifact is intentionally attached to a release through a documented process. Never commit a large binary by accident.
