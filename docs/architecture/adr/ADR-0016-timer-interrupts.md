# ADR-0016: Timer Interrupts via the Legacy 8259 PIC and PIT

- **Status:** accepted
- **Date:** 2026-10-10
- **Owners:** Project owner / contributors
- **Supersedes:** None

## Context

Milestone 2 installed our own GDT/TSS/IDT and proved exception handling, but left `IF` clear: no
interrupt source was enabled, so the kernel had no clock. Milestone 3's first item is "Establish
IDT/exception/interrupt handling and timer source." The IDT/exception half is done; the timer source
is the next step, and a periodic interrupt is the foundation for later scheduling, timekeeping and
preemption.

QEMU's `-machine pc`/`q35` provide the legacy 8259 PICs and the 8254 PIT, so a timer can be brought
up with no new dependency and no ACPI/APIC parsing.

## Requirements

- Deliver periodic interrupts through the IDT we already own, using our own handler.
- Avoid colliding with the CPU exception vectors at `0..31`.
- Keep every other interrupt source masked, so an unexpected interrupt cannot fire and the boot stays
  deterministic.
- Do not pretend to have a scheduler, preemption or timekeeping; nothing consumes the ticks yet.
- Keep it verifiable: the boot must prove a *real* interrupt was delivered, not just that a device was
  programmed.
- Stay within the small-change rule.

## Options considered

1. **Local APIC timer (with ACPI/APIC discovery).** The modern path and the eventual one, but it needs
   ACPI table parsing, APIC MMIO mapping and interrupt-routing decisions that are milestone-4 work.
   Too large for this slice, and it would add an untested parser.
2. **Legacy 8259 PIC + PIT.** Small, self-contained, no ACPI, well specified, and exactly what a
   virtual QEMU PC offers. Chosen.
3. **No timer yet; poll the PIT or the TSC for time.** Leaves `IF` clear and gives no interrupt, which
   is the thing milestone 3 actually needs.

## Decision

1. **Remap the PICs.** Initialise both 8259s with the standard ICW1–ICW4 sequence, moving IRQ0..7 to
   vectors `0x20..0x27` and IRQ8..15 to `0x28..0x2F`, so hardware interrupts no longer overlap the
   exception vectors. A write to the unused POST port (`0x80`) provides the conventional settle delay
   between PIC commands.
2. **Unmask only IRQ0.** The master's mask is set to `0xFE` and the slave's to `0xFF`, so the timer is
   the only source that can deliver. An unexpected interrupt therefore means a real bug, and the
   dispatcher treats it as fatal rather than guessing.
3. **Program PIT channel 0** in mode 3 (square wave) at 100 Hz (`divisor = 1193182 / 100`). 100 Hz is
   fast enough to observe quickly and slow enough not to dominate the emulated CPU.
4. **Reuse the existing exception machinery.** Vector 32 gets its own entry stub, exactly like the
   exception stubs, pushing a dummy error code so the shared frame layout is unchanged. The timer is
   acknowledged (`EOI` to the master PIC) and counted in the dispatcher; it does **not** print, so a
   100 Hz interrupt cannot flood the serial console.
5. **Enable interrupts only after the tables and the PIC are ready.** `enable_timer` programs the PIC
   with `IF = 0` and only then executes `sti`, so no interrupt can arrive mid-reprogramming.
6. **Prove it with the boot self-test.** The kernel waits (`hlt`) for a minimum number of timer ticks
   and only then prints `NEW_OS_KERNEL_TIMER_OK`, which the smoke test asserts. A kernel that programs
   the PIT but never actually receives an interrupt cannot print the marker.

## Consequences and trade-offs

- The kernel now has a clock, which later milestones (scheduling, timeouts, preemption) build on.
- Interrupts stay enabled for the rest of the boot; the counter is the only shared state they touch,
  and it is an atomic, so no locking is needed for one CPU.
- The legacy PIC/PIT caps the design at 16 IRQ lines and an unmultiplexed timer, which milestone 4
  replaces with the APIC/IO-APIC. That work is deliberately deferred.
- Firmware may leave the PIC in an undefined state; the full re-initialisation makes the kernel
  independent of it.
- **Not solved:** there is no scheduler, no preemption, no wall-clock timekeeping, no interrupt-safe
  synchronisation in the usual sense (only one atomic counter is shared), and no other device
  interrupt is enabled or tested.

## Security and privacy impact

- Fewer enabled interrupt lines is strictly less exposure: only IRQ0 can fire.
- An unexpected interrupt is reported and halts, rather than being ignored and leaving a stuck device
  line or a silent flood.
- No new data is captured, stored or transmitted; the timer only increments a counter in memory.

## Validation plan and evidence

- QEMU boot test asserting `NEW_OS_KERNEL_EXCEPTION_OK`, `NEW_OS_KERNEL_TIMER_OK`,
  `NEW_OS_KERNEL_MEMORY_OK`, then `NEW_OS_KERNEL_BOOT_OK`, in that order, for the BIOS and UEFI
  images, plus the zeroed-image negative control and a targeted control that removes only the timer
  marker: `tools/test/qemu-smoke.sh bios|uefi`.
- Evidence, exact commands and observed serial output: `docs/verification/M3-TIMER-EVIDENCE.md`.

## References

- Intel SDM Vol. 3A, 10.5 (8259A-compatible interrupt controllers) and the 8254 PIT programming
  interface (mode 3, lobyte/hibyte, channel 0).
- ADR-0015 (exception handling) — the GDT/TSS/IDT and shared stub this builds on.
- ADR-0005 (first test platform) — QEMU configuration used for the boot evidence.
- `docs/verification/M3-TIMER-EVIDENCE.md` — the tests and runs behind this decision.
