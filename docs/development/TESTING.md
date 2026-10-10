# Testing

Use the verification vocabulary in `docs/verification/VERIFICATION.md`. A check that did not run is
reported as `NOT RUN`, never as passed.

## Verified checks (milestones 1–2)

Commands were run from the repository root on 2026-10-10 in the environment recorded in
`docs/development/BUILDING.md`. Results for each run are recorded in
`docs/verification/M1-BOOT-EVIDENCE.md` and `docs/verification/M2-MEMORY-EVIDENCE.md`.

```sh
cargo fmt --all --check                                                    # formatting
cargo clippy --target x86_64-unknown-none -p kernel -p uart16550 -- -D warnings   # bare-metal target
cargo clippy -p uart16550 -p boot-image --all-targets -- -D warnings       # host-side crates
cargo test -p uart16550                                                    # host unit tests
cargo test -p physmem                                                      # memory-map and allocator tests
cargo build -p boot-image                                                  # kernel + boot images
./tools/test/qemu-smoke.sh bios                                            # boot smoke test (BIOS)
./tools/test/qemu-smoke.sh uefi                                            # boot smoke test (UEFI)
```

`cargo test --workspace` is deliberately not used: the `kernel` crate is a bare-metal binary that
cannot be built for the host, so a workspace-wide test run would fail for reasons unrelated to
correctness. Tests are run per package.

## What each check proves, and what it does not

| Check | Proves | Does not prove |
| --- | --- | --- |
| `cargo fmt --all --check` | Formatting matches rustfmt | Anything about behaviour |
| `cargo clippy --target x86_64-unknown-none -p kernel -p uart16550 -- -D warnings` | Kernel and UART driver compile warning-free for `x86_64-unknown-none`, including the `clippy::undocumented_unsafe_blocks` lint that enforces a safety comment per unsafe block | That the kernel boots, or that the unsafe code is correct |
| `cargo clippy -p uart16550 -p boot-image --all-targets -- -D warnings` | Host-side crates (including test code) are warning-free | Runtime behaviour |
| `cargo test -p uart16550` | The pure baud-divisor arithmetic and the port-handle accessor behave as asserted, on the host, without touching hardware | That real UART I/O works; that a boot happens |
| `cargo test -p physmem` | Memory-map validation and the bitmap frame allocator behave as asserted for well-formed and malformed input, on the host, without touching hardware | That the firmware map on a real machine looks like the fixtures; that the kernel uses the allocator correctly |
| `cargo build -p boot-image` | The kernel compiles for its target and the boot images can be generated | That the images boot |
| `tools/test/qemu-smoke.sh bios\|uefi` | The generated image boots and the **guest kernel** prints `NEW_OS_KERNEL_MEMORY_OK` and then `NEW_OS_KERNEL_BOOT_OK` on its own COM1 serial path, in that order — so the memory path cannot stop running without a failure | Physical hardware, other devices, virtual memory, interrupts, userspace, or any desktop/Windows-compatibility feature |

## Boot smoke test requirements

`tools/test/qemu-smoke.sh` implements the requirements from ADR-0005 and
`docs/verification/VERIFICATION.md`:

- the assertions are markers written by the guest kernel's own serial code, not by any host helper
  (`NEW_OS_KERNEL_MEMORY_OK`, then `NEW_OS_KERNEL_BOOT_OK` after it, so the initialisation path
  cannot be shortened while the test still passes);
- a timeout alone is never a pass, and a failure names exactly which marker(s) were missing;
- the only disks QEMU receives are generated files under `build/boot/` and a read-only
  copy-on-write OVMF vars store — no physical disk, host block device, USB device or passthrough;
- serial output is captured to `build/test/serial-<mode>.log` for inspection;
- QEMU runs with an explicit accelerator (`QEMU_ACCEL`, default `tcg`) so local and CI runs are
  comparable and KVM is not required.

Environment overrides: `QEMU_BIN`, `QEMU_ACCEL`, `QEMU_TIMEOUT` (seconds, default 30),
`GUEST_MEMORY` (default `256M`).

### Negative control

A boot test that can only ever pass is worthless, so the outcome was falsified once: replacing
`build/boot/bios.img` with a zero-filled file of the same size made the smoke test exit `1` with
`FAIL: guest kernel did not emit NEW_OS_KERNEL_BOOT_OK within 8s` and an empty serial log. The
original image was then restored and its sha256 verified identical to the pre-test image. The test
itself never rewrites an image, so a replaced or truncated image also stays replaced until you
rebuild; `cargo build -p boot-image` then detects the wrong contents and regenerates it (verified
2026-10-10).

## Unit test coverage in this milestone

Only logic that is genuinely pure is unit tested. `crates/uart16550` (5 tests) covers baud divisor
computation including zero baud, out-of-range rates and the maximum representable divisor, plus the
port-handle accessor. `crates/physmem` (21 tests) covers boot memory-map validation and the bitmap
frame allocator: zero-length regions, unrepresentable end addresses, saturating byte totals, reversed
start/end bounds, unsupported frame sizes, whole-frame-only usable marking, partial-frame
reservation, bitmap-overflow reporting, non-usable claims overriding usable ones, allocation order
and exhaustion, call-order enforcement, and rejected calls leaving state unchanged.

Register-level I/O, the kernel entry path, the print/parsing wiring, the panic path and the halt
loop are **not** unit tested; they are exercised by the QEMU boot test. No test was added purely to
raise a count.

## Not run / not applicable

| Check | Status | Reason |
| --- | --- | --- |
| Kernel unit tests | NOT APPLICABLE | The kernel is a `no_main` bare-metal binary; it cannot run the host test harness. Covered by the QEMU boot test instead. |
| Integration/fault-injection tests | NOT RUN | No filesystem or syscall boundary exists yet; the memory paths added in M2 are covered by host unit tests plus the QEMU boot test. |
| CPU fault diagnostics (page fault, double fault, GP fault) | NOT RUN | No IDT or fault handler exists yet; this is the remaining milestone-2 item. |
| Virtual memory and page tables of our own | NOT RUN | Not implemented; the kernel runs on the bootloader's page tables. |
| Fuzzing | NOT RUN | No untrusted-input parser exists yet. PE parsing and fuzzing are milestone M9. |
| Physical hardware boot | NOT RUN | Out of scope for milestone 1; requires owner approval and a reviewed procedure. |
| Memory-map validation tests | DONE (2026-10-10) | `cargo test -p physmem` plus the QEMU memory marker; see `docs/verification/M2-MEMORY-EVIDENCE.md`. |
| Windows/Microsoft application tests | NOT RUN | No compatibility runtime exists. |
