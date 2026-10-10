# ADR-0015: Exception Handling, Our Own GDT/TSS and IDT

- **Status:** accepted
- **Date:** 2026-10-10
- **Owners:** Project owner / contributors
- **Supersedes:** None

## Context

Milestone 1 handed control to the kernel with no interrupt-descriptor table of our own. Inspecting the
pinned `bootloader` / `bootloader-x86_64-common` 0.11.17 source showed that the boot component creates
a GDT (kernel code at selector `0x08`, kernel data at `0x10`) but installs **no IDT and no TSS**. The
consequence was that any CPU exception — even an ordinary page fault — had no handler and took the
machine down with a triple fault and no diagnostic. That is exactly the failure mode milestone 2's
"exception reporting and controlled fatal handling" item exists to remove, and it blocks the M2 gate,
which requires faults to produce useful diagnostics in QEMU.

## Requirements

- Install tables so an exception is reported on the serial console before the kernel halts, instead of
  an unexplained triple fault.
- Keep a double fault from triple-faulting silently: give vectors that may run on a failed stack their
  own stack.
- Report useful detail: the exception's identity, the error code where the CPU supplies one, the
  faulting instruction pointer and the register state.
- Make the fatal-by-default policy explicit: only a deliberately resumable case returns, everything
  else halts after reporting.
- Keep the untrusted/encoding logic host-testable, separate from hardware access.
- Do not overclaim: no interrupt *sources* are enabled yet, there is no privilege separation, and the
  page-fault handler cannot fix up a real fault.
- Stay within the small-change rule: no scaffolding for future subsystems.

## Options considered

1. **Do nothing; rely on the boot loader's tables.** Not possible: the boot component installs no IDT
   and no TSS, so there is nothing to rely on.
2. **Add the `x86_64` crate and use its GDT/IDT/TSS abstractions.** Attractive and well-tested, but it
   is a new third-party dependency whose provenance and licence review this milestone does not need,
   and the tables are a small, stable, well-documented piece of the architecture the project intends
   to own. (The crate is already present transitively through the boot component; that does not
   justify depending on it directly.)
3. **Write our own minimal GDT, TSS and IDT in the kernel, with the pure encoding/decoding logic in a
   host-testable `no_std` crate (`crates/x86exc`).** Chosen.

## Decision

1. **Our own GDT, matching the selectors already loaded.** A GDT with a null descriptor, a ring-0
   kernel code descriptor at `0x08`, a ring-0 kernel data descriptor at `0x10` and a 64-bit TSS
   descriptor at `0x18`. Because the code/data values equal the ones the boot component already
   loaded, `CS`/`DS` stay valid and no segment reload is needed; only `lgdt` and `ltr` are issued.
2. **A TSS with a dedicated IST stack for the "cannot trust the stack" vectors.** `IST[1]` points at a
   16 KiB `.bss` stack, used by `#DF` (8), `#NMI` (2) and `#MC` (18). The TSS is a `#[repr(C, packed)]`
   structure, because the hardware expects `rsp0` at offset 4 and would otherwise reject a padded
   layout; its fields are written through raw pointers with `write_unaligned`.
3. **A 256-entry IDT.** Vectors `0..=31` each get their own entry stub; vectors `32..=255` share one
   unexpected-vector stub that reports an unknown vector. Filling all 256 means an unexpected vector is
   reported rather than becoming a triple fault. Gates are 64-bit interrupt gates (`0xE`, `IF` cleared
   on entry), DPL 0.
4. **A uniform frame, built by the stubs.** Because some vectors push an error code and some do not,
   each stub pushes a dummy `0` first for the no-error vectors and then pushes the vector number, so
   every frame the dispatcher sees has the same shape (`error_code`, `vector`, `rip`, `cs`, `rflags`
   over the saved registers). The correctness of the error-code table is the one thing that silently
   shifts the whole frame if it is wrong, so it is unit tested in `x86exc`.
5. **One shared assembly path, written as `#[unsafe(naked)]` functions.** All stubs `jmp` to
   `exception_common`, which saves all general-purpose registers, aligns the stack for the Rust ABI,
   calls `exception_dispatch`, restores the registers and returns with `iretq`. Using naked functions
   instead of a `global_asm!` block plus 32 hand-written `extern` declarations keeps the stubs typed
   and lets the IDT table be an ordinary Rust array.
6. **Fatal by default.** The dispatcher reports the exception and then, unless it is one of the two
   deliberate self-test cases, takes the controlled halt path. `#BP` (a trap the CPU has already
   resumed) returns normally, and `#PF` returns *only* when the kernel's own self-test probe is armed
   and the fault is on the exact address the probe passed, so a real page fault is never swallowed.
7. **Prove it with real exceptions in the boot path.** After the tables are installed the kernel runs
   two self-tests through them: `int3` (a resumable trap that exercises the IDT, the stub, the shared
   path and the `iretq` resume) and an armed load from a deliberately unmapped canonical address (a real
   page fault that the handler diagnoses and resumes). The smoke test asserts the resulting
   `NEW_OS_KERNEL_EXCEPTION_OK` marker, so the exception path cannot silently stop working.
8. **No interrupt sources are enabled.** `IF` stays clear: there is no PIC/APIC programming, no timer,
   no `sti`. Vectors 32+ exist only so an unexpected one is reported. `#DB` is classified as resumable
   in principle (its saved `rip` is a next-instruction address) but the kernel does not interpret `DR6`
   and does not resume it.

## Consequences and trade-offs

- A fault now produces a readable report (identity, error code, `rip`, `cr2` for page faults, and the
  register file) before the controlled halt, instead of a silent triple fault.
- A double fault has a known-good stack, so a corrupt kernel stack is reported rather than causing a
  triple fault.
- The encoding/decoding logic (vectors, error-code table, gate layout, table pointer layout,
  page-fault error code) is host-testable without hardware, in `crates/x86exc`.
- The kernel now owns a small amount of architecture the boot component previously left to it. When
  the boot component is replaced (ADR-0003) this code is expected to move into a later loader/kernel
  boundary rather than be rewritten.
- The handler alignment dance (`and rsp, -16` with the frame address held in `r15`) is a small piece of
  assembly with a real invariant; it is exercised by the boot test in both boot modes.
- **Not solved:** the page-fault handler cannot repair a real fault, there is no guard page, no
  privilege separation, no W^X, and the ring-3 paths (a meaningful TSS `rsp0`, `SS`/`RSP` in the
  frame) are unused because there is no ring 3 yet.

## Security and privacy impact

- The failure mode is now observable and diagnosable rather than silent; this is a prerequisite for
  any later security work, not a security feature by itself.
- **No protection is claimed.** The kernel still runs at ring 0, on the boot component's page tables,
  with a shared address space. The IDT adds no isolation.
- The dispatcher prints register values and fault addresses. That is kernel state on a local serial
  console only; nothing is captured, stored or transmitted, consistent with the privacy defaults in
  ADR-0011.

## Validation plan and evidence

- Host unit tests in `crates/x86exc` for the vector metadata, the error-code table, the IDT gate
  bit layout, the table-pointer layout and page-fault error-code decoding and display:
  `cargo test -p x86exc`.
- QEMU boot test asserting `NEW_OS_KERNEL_EXCEPTION_OK`, then `NEW_OS_KERNEL_MEMORY_OK`, then
  `NEW_OS_KERNEL_BOOT_OK`, in that order, for the BIOS and UEFI images, plus negative controls:
  `tools/test/qemu-smoke.sh bios|uefi`.
- Evidence, exact commands and observed serial output: `docs/verification/M2-EXCEPTION-EVIDENCE.md`.

## References

- Intel SDM Vol. 3A: "Exception and Interrupt Reference" (vector table and error codes), 6.14.1
  (64-bit IDT/gate format), 8.8.1 (IST and TSS), 3.5.2 (descriptor format), 4.7 (page-fault error
  code).
- `bootloader-x86_64-common` 0.11.17 `src/gdt.rs` — shows the boot component creates only a GDT
  (code/data selectors) and installs no IDT or TSS; read from the pinned crate source on 2026-10-10.
- ADR-0003 (boot path) — the reused boot component and its page tables.
- ADR-0005 (first test platform) — QEMU configuration used for the boot evidence.
- ADR-0014 (boot memory handling) — the memory-map validation the exception handler now protects.
- `docs/verification/M2-EXCEPTION-EVIDENCE.md` — the tests and runs behind this decision.
