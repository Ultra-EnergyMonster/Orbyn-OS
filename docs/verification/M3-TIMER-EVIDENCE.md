# Milestone 3 Timer Evidence (8259 PIC + PIT timer interrupts)

**Date:** 2026-10-10
**Branch:** `feat/m3-timer-interrupts`
**Claim level:** *QEMU boot tested* for the timer interrupt path (see `docs/verification/VERIFICATION.md`
for the vocabulary). Nothing here claims a scheduler, preemption, wall-clock timekeeping, the APIC or
any interrupt source other than the PIT timer.

## What was tested

The kernel's first interrupt source (`kernel/src/interrupts.rs`):

1. both 8259 PICs are re-initialised (ICW1–ICW4) so hardware IRQ0..15 land on vectors `0x20..0x2F`,
   clear of the CPU exceptions;
2. every PIC line is masked except IRQ0;
3. PIT channel 0 is programmed in mode 3 for 100 Hz;
4. `sti` is executed only after the IDT and PIC are ready, so vector 32 (IRQ0) is delivered through
   the IDT gate we installed in milestone 2;
5. the interrupt handler acknowledges the PIC (`EOI`) and increments a counter, and does not print;
6. the boot self-test waits (`hlt`) for a minimum number of real timer interrupts and only then emits
   `NEW_OS_KERNEL_TIMER_OK`, which `tools/test/qemu-smoke.sh` asserts — in order, ahead of the memory
   and boot markers.

## Environment

| Item | Value |
| --- | --- |
| Host | Ubuntu 24.04 (noble), x86-64, KVM device present |
| Toolchain | rustup `nightly-2026-10-08` (rustc 1.101.0-nightly 1d81eb4ad 2026-10-07), components `rustfmt`, `clippy`, `llvm-tools`, `rust-src`; targets `x86_64-unknown-none`, `x86_64-unknown-uefi` |
| QEMU | 8.2.2 (Debian 1:8.2.2+ds-0ubuntu1.18), accelerator `tcg`, `-m 256M`, `-display none`, `-no-reboot` |
| BIOS mode | `-machine pc` + SeaBIOS from QEMU 8.2.2 |
| UEFI mode | `-machine q35` + OVMF 2024.02-2ubuntu0.10 (`/usr/share/OVMF/OVMF_CODE_4M.fd`, vars store with `snapshot=on`) |
| Timer hardware | Legacy 8259 PIC + 8254 PIT as emulated by QEMU; not the local APIC |
| Boot component | `bootloader` / `bootloader_api` 0.11.17 |

## Commands and results

| # | Command | Result |
| --- | --- | --- |
| 1 | `cargo fmt --all --check` | exit 0, no diff |
| 2 | `cargo clippy --target x86_64-unknown-none -p kernel -p uart16550 -- -D warnings` | exit 0, no warnings |
| 3 | `cargo clippy -p physmem -p uart16550 -p x86exc -p boot-image --all-targets -- -D warnings` | exit 0, no warnings |
| 4 | `cargo test -p physmem -p uart16550 -p x86exc` | exit 0: `21 passed`, `5 passed`, `10 passed` |
| 5 | `cargo build -p boot-image` | exit 0; `build/boot/bios.img` and `build/boot/uefi.img` regenerated |
| 6 | `./tools/test/qemu-smoke.sh bios` | exit 0; `PASS: guest kernel emitted NEW_OS_KERNEL_EXCEPTION_OK (line 41), NEW_OS_KERNEL_TIMER_OK (line 42), NEW_OS_KERNEL_MEMORY_OK (line 61) then NEW_OS_KERNEL_BOOT_OK (line 62)` |
| 7 | `./tools/test/qemu-smoke.sh uefi` | exit 0; `PASS: ... NEW_OS_KERNEL_EXCEPTION_OK (line 48), NEW_OS_KERNEL_TIMER_OK (line 49), NEW_OS_KERNEL_MEMORY_OK (line 68) then NEW_OS_KERNEL_BOOT_OK (line 69)` |
| 8 | Negative control: zero-filled `build/boot/bios.img`, `QEMU_TIMEOUT=8 ./tools/test/qemu-smoke.sh bios` | exit 1; `FAIL: guest kernel did not emit: NEW_OS_KERNEL_EXCEPTION_OK NEW_OS_KERNEL_TIMER_OK NEW_OS_KERNEL_MEMORY_OK NEW_OS_KERNEL_BOOT_OK within 8s`; original image restored and sha256 verified unchanged |
| 9 | Targeted negative control: the timer marker's emission removed from `kernel/src/main.rs`, rebuilt, `QEMU_TIMEOUT=15 ./tools/test/qemu-smoke.sh bios` | exit 1; `FAIL: guest kernel did not emit: NEW_OS_KERNEL_TIMER_OK within 15s` — the timer assertion is genuinely required and the other three markers do not substitute for it; source restored and images rebuilt afterwards |

## Guest serial output (BIOS run, kernel lines around the timer)

```text
exceptions: GDT, TSS (IST for #DF) and 256-entry IDT installed
[EXCEPTION] #BP breakpoint (vector 3)
  ...
  resuming: breakpoint is a resumable trap
[EXCEPTION] #PF page fault (vector 14)
  error code 0x0: not-present page, read, supervisor; cr2=0x00007ffffffff000
  ...
  resuming: armed page-fault self-test probe, continuing at rip=0x0000010000005a42
NEW_OS_KERNEL_EXCEPTION_OK
NEW_OS_KERNEL_TIMER_OK
timer: PIT at 100 Hz; 3 interrupt(s) observed (IRQ0 -> vector 32)
boot handoff:
  ...
NEW_OS_KERNEL_MEMORY_OK
NEW_OS_KERNEL_BOOT_OK
initialisation path complete; halting (no scheduler in milestone 2)
```

The UEFI run reported the same timer line (`3 interrupt(s) observed`) with the same marker order.

## What this proves

- **Interrupts are actually delivered.** The marker is emitted only after at least three real IRQ0
  interrupts have reached the dispatcher, so a kernel that programs the PIT but never unmasks IRQ0 or
  never executes `sti` cannot print it. The targeted control confirms the assertion is not padded.
- **The IDT and shared stub path work for hardware interrupts, not just exceptions.** The timer is
  delivered through the vector-32 gate we installed and returns via the same `iretq` path; boot then
  continues normally, which proves the interrupt returned cleanly rather than faulting or losing state.
- **The PIC remap works.** Software interrupts still reach vectors `0..31` (the exception self-tests
  run first) while hardware IRQ0 arrives on vector 32, so there is no collision.
- **Only the timer can fire.** Every other line is masked, so the boot stays deterministic.
- **The `EOI` path works.** Without acknowledging the PIC, no further timer interrupts would arrive
  and the self-test would hang; three observed ticks prove the acknowledge works.

## What this does NOT prove

- **No scheduler or preemption.** Nothing consumes the ticks; there is no task, no context switch and
  no timers-with-callbacks.
- **No timekeeping.** No wall clock, no monotonic time base exposed to anything, and no measurement of
  the timer's actual frequency (100 Hz is the programmed value).
- **No APIC/I/O APIC/MSI**, no SMP, no per-CPU interrupt routing.
- **Only IRQ0 is tested.** No other device interrupt is enabled, and a non-timer interrupt is treated
  as fatal by design.
- **No interrupt-safety of shared state beyond the tick counter**, which is a single atomic. There is
  no lock, no critical section and no demonstration that any other data structure is interrupt-safe.
- **No physical-hardware, KVM or multi-host test**, as in the previous evidence records.

## Known limitations and open questions

1. **The PIC settle delay (`io_wait`) is minimal.** QEMU tolerates it; real hardware sometimes needs
   longer or more careful sequencing. This cannot be validated until physical-hardware testing is in
   scope.
2. **Interrupts remain enabled for the rest of the boot.** The only shared state the handler touches is
   an atomic counter, so this is safe today; that will have to be revisited as soon as interrupt-time
   code touches more than one location.
3. **The `hlt`-based wait assumes the timer is the only wake source**, which is true while every other
   line is masked. If another source is enabled later, the wait logic must change.
4. **Ticks are counted from the moment `sti` executes**, not aligned to any period boundary, so the
   count is not a time base.

## Not run

| Check | Status | Reason / next step |
| --- | --- | --- |
| Scheduler / preemption / context switch | NOT RUN | Milestone 3's later items; nothing consumes ticks yet |
| Timer frequency measurement | NOT RUN | No time base to measure against; needs a reference clock or calibrated TSC |
| APIC / I/O APIC / MSI | NOT RUN | Milestone 4; legacy PIC/PIT only for now |
| Other device interrupts (keyboard, disk, network) | NOT RUN | No drivers exist |
| Deliberate double fault on the IST stack | NOT RUN | Still outstanding from ADR-0015 |
| Physical hardware boot | NOT RUN | Out of scope; requires owner approval and a reviewed procedure |
| CI execution for this branch | NOT RUN | No pull request has been opened for this branch, so the workflow has not run on it. It has run and passed on `main` and on the exception branch's PR (2026-10-10). |
