# Milestone 2 Exception Evidence (exception reporting and controlled fatal handling)

> **Scope note (added after the timer and address-translation slices):** the kernel now also starts a
> PIT timer and translates its stack/framebuffer addresses after the exception self-tests, and
> `tools/test/qemu-smoke.sh` asserts two further markers (`NEW_OS_KERNEL_TIMER_OK`, then
> `NEW_OS_KERNEL_PAGING_OK`) between the exception marker and the memory marker described here. The
> exception evidence below is unchanged; the current boot evidence is
> `docs/verification/M2-PAGING-EVIDENCE.md`.

**Date:** 2026-10-10
**Branch:** `feat/m2-exception-handling` (stacked on the milestone-2 memory branch, still unpushed)
**Claim level:** *QEMU boot tested* for the exception tables and the two fault self-tests (see
`docs/verification/VERIFICATION.md` for the vocabulary). Nothing here claims virtual memory,
privilege separation, interrupt sources, timers or scheduling.

## What was tested

The kernel's exception handling, installed by `kernel/src/interrupts.rs`:

1. our own GDT (null, ring-0 code `0x08`, ring-0 data `0x10`, 64-bit TSS descriptor `0x18`), loaded
   with `lgdt`, with a TSS loaded via `ltr` whose `IST[1]` points at a dedicated 16 KiB stack;
2. our own 256-entry IDT, loaded with `lidt`: vectors `0..=31` use per-vector entry stubs, vectors
   `32..=255` a shared unexpected-vector stub, and `#NMI`/`#DF`/`#MC` run on the IST stack;
3. a shared assembly path that saves every general-purpose register, calls the Rust dispatcher,
   restores the registers and returns with `iretq`;
4. two self-tests that run *through* those tables in the boot path, and the resulting serial marker
   asserted, in order, by `tools/test/qemu-smoke.sh`:
   - a real `int3` breakpoint (`#BP`, vector 3), a resumable trap, reported and resumed;
   - a real page fault (`#PF`, vector 14) on a deliberately unmapped canonical address, reported with
     its error code and `cr2`, then resumed by pointing `rip` at the address the probe prepared.

## Environment

| Item | Value |
| --- | --- |
| Host | Ubuntu 24.04 (noble), x86-64, KVM device present |
| Toolchain | rustup `nightly-2026-10-08` (rustc 1.101.0-nightly 1d81eb4ad 2026-10-07), components `rustfmt`, `clippy`, `llvm-tools`, `rust-src`; targets `x86_64-unknown-none`, `x86_64-unknown-uefi` |
| QEMU | 8.2.2 (Debian 1:8.2.2+ds-0ubuntu1.18), accelerator `tcg`, `-m 256M`, `-display none`, `-no-reboot` |
| BIOS mode | `-machine pc` + SeaBIOS from QEMU 8.2.2 |
| UEFI mode | `-machine q35` + OVMF 2024.02-2ubuntu0.10 (`/usr/share/OVMF/OVMF_CODE_4M.fd`, vars store with `snapshot=on`) |
| Boot component | `bootloader` / `bootloader_api` 0.11.17 (creates only a GDT; no IDT, no TSS) |
| New crate | `crates/x86exc` (this repository), no new third-party dependency |

## Commands and results

| # | Command | Result |
| --- | --- | --- |
| 1 | `cargo fmt --all --check` | exit 0, no diff |
| 2 | `cargo clippy --target x86_64-unknown-none -p kernel -p uart16550 -- -D warnings` | exit 0, no warnings |
| 3 | `cargo clippy -p physmem -p uart16550 -p x86exc -p boot-image --all-targets -- -D warnings` | exit 0, no warnings |
| 4 | `cargo test -p physmem -p uart16550 -p x86exc` | exit 0: physmem `21 passed`, uart16550 `5 passed`, x86exc `10 passed` |
| 5 | `cargo build -p boot-image` | exit 0; `build/boot/bios.img` (3,638,272 B) and `build/boot/uefi.img` (3,211,264 B) |
| 6 | `./tools/test/qemu-smoke.sh bios` | exit 0; `PASS: guest kernel emitted NEW_OS_KERNEL_EXCEPTION_OK (line 41), NEW_OS_KERNEL_MEMORY_OK (line 59) then NEW_OS_KERNEL_BOOT_OK (line 60)` |
| 7 | `./tools/test/qemu-smoke.sh uefi` | exit 0; `PASS: ... NEW_OS_KERNEL_EXCEPTION_OK (line 48), NEW_OS_KERNEL_MEMORY_OK (line 66) then NEW_OS_KERNEL_BOOT_OK (line 67)` |
| 8 | Negative control: zero-filled `build/boot/bios.img`, `QEMU_TIMEOUT=8 ./tools/test/qemu-smoke.sh bios` | exit 1; `FAIL: guest kernel did not emit: NEW_OS_KERNEL_EXCEPTION_OK NEW_OS_KERNEL_MEMORY_OK NEW_OS_KERNEL_BOOT_OK within 8s`; original image restored and sha256 verified unchanged (`3e52ca2d…`) |
| 9 | Targeted negative control: the exception marker's emission removed from `kernel/src/main.rs`, rebuilt, `QEMU_TIMEOUT=15 ./tools/test/qemu-smoke.sh bios` | exit 1; `FAIL: guest kernel did not emit: NEW_OS_KERNEL_EXCEPTION_OK within 15s` — i.e. the new assertion is genuinely required, and the other two markers do not substitute for it; source restored and the images rebuilt afterwards |

## Guest serial output (BIOS run, kernel lines only)

```text
new-os kernel: milestone 2 prototype (x86-64, no_std)
boot path: firmware -> bootloader 0.11 -> _start -> kernel_main
exceptions: GDT, TSS (IST for #DF) and 256-entry IDT installed
[EXCEPTION] #BP breakpoint (vector 3)
  rip=0x0000010000005be8 cs=0x0008 rflags=0x0000000000000002; stack frame at 0x0000018000014dc0
  rax=0000010000007700 rbx=0000000000004f98 rcx=00000100000171a9 rdx=0000000000000004
  rsi=0000000000000000 rdi=00000100000171a9 rbp=0000000000000000 r8 =0000010000011230
  r9 =000000000000000b r10=0000000000149f00 r11=0000000000000018 r12=0000000000006290
  r13=0000000000007728 r14=0000028000000000 r15=00000000000077a0
  resuming: breakpoint is a resumable trap
[EXCEPTION] #PF page fault (vector 14)
  error code 0x0: not-present page, read, supervisor; cr2=0x00007ffffffff000
  rip=0x0000010000005a3f cs=0x0008 rflags=0x0000000000000002; stack frame at 0x0000018000014db0
  rax=0000010000005a42 rbx=0000000000004f98 rcx=00000100000171aa rdx=0000000000000004
  rsi=0000000000000000 rdi=00007ffffffff000 rbp=0000000000000000 r8 =0000010000011230
  r9 =000000000000000b r10=0000000000149f00 r11=0000000000000018 r12=0000000000006290
  r13=0000000000007728 r14=0000028000000000 r15=00000000000077a0
  resuming: armed page-fault self-test probe, continuing at rip=0x0000010000005a42
NEW_OS_KERNEL_EXCEPTION_OK
boot handoff:
  kernel ELF: physical 0x0000000001000000, 946120 bytes, mapped at virtual offset 0x10000000000
  ...
NEW_OS_KERNEL_MEMORY_OK
NEW_OS_KERNEL_BOOT_OK
initialisation path complete; halting (no scheduler in milestone 2)
```

The `#PF` report is internally consistent: the faulting `rip` (`0x…5a3f`) is the probe's faulting
load, `cr2` (`0x00007ffffffff000`) is exactly the address the probe passed in `rdi`, the error code
`0x0` decodes as *not-present page, read, supervisor* (matching a ring-0 load from an unmapped page),
and the handler resumed at `0x…5a42`, the address the probe had left in `rax`. The UEFI run produced
the same two exception reports with different addresses and the same marker order (line numbers in
the table above).

## What this proves

- **A CPU exception now produces useful diagnostics instead of a triple fault.** A page fault is
  reported with its vector, decoded error code and `cr2`, the faulting `rip`/`cs`/`rflags`, and the
  full general-purpose register file, before the kernel takes its controlled halt path.
- **The tables we install are the ones that run.** Both self-tests reach the dispatcher only through
  the IDT entry stubs and the shared assembly path; the marker is emitted only after both have, so a
  kernel that merely loaded an IDT cannot print it.
- **The resume path works.** `int3` is resumed at the next instruction, and the page-fault probe is
  resumed at the address the probe prepared, so the boot continues normally rather than ending in a
  fault. That exercises the full `iretq` path, not just entry.
- **The double fault has a dedicated stack.** `#DF` (with `#NMI` and `#MC`) is routed to an IST stack,
  so a corrupt kernel stack is reported rather than triple-faulting. (The instruction sequence itself
  is not separately exercised; see limitations.)
- **Unexpected vectors are contained.** Every vector `32..=255` has a handler, so an unexpected one is
  reported as unknown instead of becoming a triple fault.
- **The encoding is tested where it can be.** The error-code table, the IDT gate bit layout, the
  table-pointer layout and the page-fault error-code decoding are unit tested on the host; a wrong
  error-code table would silently shift the whole frame, so that specific table is asserted exactly.

## What this does NOT prove

- **No interrupt sources.** `IF` stays clear: no PIC/APIC programming, no timer, no `sti`, no
  scheduling. Vectors 32+ are only a safety net.
- **No protection or isolation.** Still ring 0, still the boot component's page tables, still one
  address space. The IDT adds no isolation, no guard pages, no W^X.
- **A real page fault is still fatal.** Only the kernel's own armed probe is resumed. A fault from a
  kernel bug is reported and then halts.
- **No privilege separation.** The TSS `rsp0` and the `SS`/`RSP` frame fields are unused because there
  is no ring 3. The ring-3 privilege-transition path is untested and untestable today.
- **No `#DB` handling.** `#DB` saves a next-instruction `rip`, so it is *classified* as resumable, but
  the kernel does not read `DR6` and never resumes it; a `#DB` is reported and then halts.
- **Physical hardware, KVM and other hosts are not tested**, as in the memory evidence.
- **The dispatcher itself is not fault-tolerant.** If the frame were so corrupt that writing to COM1
  faulted, the report would be lost; the dispatcher makes no attempt to survive that.

## Known limitations and open questions

1. **The self-test probe address is a fixed constant** (`0x00007FFF_FFFF_F000`, the last page of the
   lower canonical half). If a future mapping ever occupied it, the probe would not fault and the
   self-test would fail loudly (it checks that a fault *was* observed) rather than pass silently.
2. **The `#DF` IST path is installed but not separately faulted.** Producing a genuine double fault
   safely (stack overflow plus a fault in the overflow handler) is not done here; what is proven is
   that the TSS/IST descriptor loads and that `#DF` is routed to IST index 1, not that a double fault
   was delivered on that stack.
3. **Alignment handling in the shared stub is a small invariant.** An exception can be delivered with
   the stack at either alignment; the stub aligns it for the Rust ABI and restores it from `r15`. This
   is exercised by both boot modes but by only the two self-tests.
4. **The page-fault handler resumes by trusting `rax` from the frame.** This is safe only because the
   probe is the sole caller and `rax` is set immediately before the faulting load; it is a test hook,
   not a general fault-recovery mechanism.

## Not run

| Check | Status | Reason / next step |
| --- | --- | --- |
| Deliberate double fault on the IST stack | NOT RUN | Producing one safely needs a reviewed method; the IST descriptor is installed and asserted only structurally |
| Interrupts/timers (PIC/APIC, `sti`) | NOT RUN | Milestone 3; `IF` is deliberately left clear |
| Virtual memory and page tables of our own | NOT RUN | Follows the bootstrap allocation; today the handler runs on the boot component's tables |
| Physical hardware boot | NOT RUN | Out of scope; requires owner approval and a reviewed procedure |
| KVM-accelerated run | NOT RUN | Only `tcg` was used, for comparability with CI |
| Fuzzing of the descriptor/vector encoding | NOT RUN | The inputs are kernel constants, not untrusted data; PE parsing and fuzzing are milestone M9 |
| CI execution | DONE (2026-10-10) | The workflow ran and passed on this branch's pull request (#1) and on the resulting `main` push; both runs completed with conclusion `success` |
