# Milestone 2 Memory Evidence (first slice)

> **Scope note (added after the exception slice):** every serial value below was produced by the
> memory-slice kernel, before exception handling existed. The kernel now installs its own GDT/TSS/IDT
> and runs two fault self-tests before the memory work, and `tools/test/qemu-smoke.sh` asserts a third
> marker (`NEW_OS_KERNEL_EXCEPTION_OK`) *before* the two markers described here. The current boot
> evidence is `docs/verification/M2-EXCEPTION-EVIDENCE.md`.
>
> **Scope note 2 (added after the address-translation slice):** limitation 1 below — the kernel stack
> and framebuffer not being reservable by address — is now **addressed**: the kernel translates their
> virtual addresses by walking the installed page tables and reserves the resulting physical frames.
> See `docs/verification/M2-PAGING-EVIDENCE.md`. The serial values here are still the memory-slice
> ones and are unchanged.

**Date:** 2026-10-10
**Branch:** `feat/m2-boot-memory-and-frame-allocator` (stacked on the milestone-1 branch, still
unpushed)
**Claim level:** *QEMU boot tested* for the boot memory-map validation and the bootstrap frame
allocator (see `docs/verification/VERIFICATION.md` for the vocabulary). Nothing here claims virtual
memory, address-space isolation, physical-hardware support or application compatibility.

## What was tested

The kernel's first memory work:

1. the memory map from `BootInfo` is validated with checked arithmetic before anything uses it;
2. a bitmap physical frame allocator is built from the validated map, with explicit reservations and
   conservative whole-frame policies (`crates/physmem`);
3. the allocator hands out a small bootstrap allocation, and the guest reports all of it on COM1;
4. the guest then emits `NEW_OS_KERNEL_MEMORY_OK` and, after the rest of the initialisation path,
   `NEW_OS_KERNEL_BOOT_OK`, which `tools/test/qemu-smoke.sh` asserts **in that order**.

## Environment

| Item | Value |
| --- | --- |
| Host | Ubuntu 24.04 (noble), x86-64, KVM device present |
| Toolchain | rustup `nightly-2026-10-08` (rustc 1.101.0-nightly 1d81eb4ad 2026-10-07), components `rustfmt`, `clippy`, `llvm-tools`, `rust-src`; targets `x86_64-unknown-none`, `x86_64-unknown-uefi` |
| QEMU | 8.2.2 (Debian 1:8.2.2+ds-0ubuntu1.18), accelerator `tcg`, `-m 256M`, `-display none`, `-no-reboot` |
| BIOS mode | `-machine pc` + SeaBIOS from QEMU 8.2.2 |
| UEFI mode | `-machine q35` + OVMF 2024.02-2ubuntu0.10 (`/usr/share/OVMF/OVMF_CODE_4M.fd`, vars store with `snapshot=on`) |
| Boot component | `bootloader` / `bootloader_api` 0.11.17 |
| New crate | `crates/physmem` (this repository), no new third-party dependency |

## Commands and results

| # | Command | Result |
| --- | --- | --- |
| 1 | `cargo fmt --all --check` | exit 0, no diff |
| 2 | `cargo clippy --target x86_64-unknown-none -p kernel -p uart16550 -- -D warnings` | exit 0, no warnings |
| 3 | `cargo clippy -p physmem -p uart16550 -p boot-image --all-targets -- -D warnings` | exit 0, no warnings |
| 4 | `cargo test -p physmem -p uart16550` | exit 0: physmem `21 passed; 0 failed`, uart16550 `5 passed; 0 failed` |
| 5 | `cargo build -p boot-image` | exit 0; `build/boot/bios.img` (3,638,272 B) and `build/boot/uefi.img` (3,211,264 B) |
| 6 | `./tools/test/qemu-smoke.sh bios` | exit 0; `PASS: guest kernel emitted NEW_OS_KERNEL_MEMORY_OK (line 42) then NEW_OS_KERNEL_BOOT_OK (line 43)` |
| 7 | `./tools/test/qemu-smoke.sh uefi` | exit 0; `PASS: ... NEW_OS_KERNEL_MEMORY_OK (line 49) then NEW_OS_KERNEL_BOOT_OK (line 50)` |
| 8 | Negative control: zero-filled `build/boot/bios.img`, `QEMU_TIMEOUT=8 ./tools/test/qemu-smoke.sh bios` | exit 1, empty serial log, `FAIL: guest kernel did not emit: NEW_OS_KERNEL_MEMORY_OK NEW_OS_KERNEL_BOOT_OK within 8s`; original image restored and sha256 verified unchanged |

## Guest serial output (BIOS run, kernel lines only)

```text
new-os kernel: milestone 2 prototype (x86-64, no_std)
boot path: firmware -> bootloader 0.11 -> _start -> kernel_main
boot handoff:
  kernel ELF: physical 0x0000000001000000, 898744 bytes, mapped at virtual offset 0x10000000000
  kernel stack: 81920 bytes at virtual 0x0000018000001000
  physical memory regions reported: 10
  framebuffer: 1280x720, stride 1280, 3 bytes/pixel, Bgr
  ACPI RSDP address: 0x00000000000f52b0
memory map: 10 regions validated
  usable: 266693632 bytes (254 MiB), reserved: 12886578176 bytes, reclaimable: 0 bytes
  highest reported address: 0x0000010000000000
  reserved first 1 MiB (real-mode firmware structures): 0x0000000000000000..0x0000000000100000 (1048576 bytes, 159 previously usable frame(s) reclaimed)
  reserved kernel image: 0x0000000001000000..0x00000000010db6b8 (898744 bytes, 0 previously usable frame(s) reclaimed)
  not reservable by address (BootInfo exposes virtual addresses only): kernel stack at virtual 0x0000018000001000 + 81920 bytes, framebuffer at virtual 0x0000020000000000
frame allocator: 4096 byte frames; 64951 free of 1048576 representable (254 MiB of RAM made available)
  marking: 65110 frames made usable, 0 whole frame(s) above the 4096 MiB bitmap (not managed), 0 reclaimed by non-usable regions
  firmware reported 266693632 usable bytes (254 MiB); frames only partly inside a region are withheld
bootstrap allocation: 8 frames of 4096 bytes for the page-table work that follows
  physical range 0x0000000000100000..0x0000000000108000; 8 frame(s) now allocated, 64943 still free
NEW_OS_KERNEL_MEMORY_OK
NEW_OS_KERNEL_BOOT_OK
initialisation path complete; halting (no scheduler in milestone 2)
```

The UEFI run reported `132` regions, `248 MiB` usable, `160` frames reclaimed from the first MiB and a
bootstrap range starting at `0x000000000014e000`; the same two markers were emitted in the same order.

## What this proves

- The map the bootloader passes down is now checked, not assumed: each region's length and end
  address are validated with checked arithmetic, and a defective region makes the kernel print the
  offending index and halt instead of managing memory.
- Physical frames are only handed out after an explicit claim: the allocator starts with every frame
  unavailable, usable regions are marked first, and every non-usable claim (including the first MiB
  and the kernel image) is applied afterwards.
- The conservative policies do real work and are visible: 159 / 160 frames of firmware-reported
  "usable" low memory were taken back, and whole frames only partly inside a region are withheld.
- The bootstrap allocation came from above 1 MiB (0x100000 BIOS, 0x14e000 UEFI), i.e. not from the
  firmware structures the policy protects.
- The milestone-2 path cannot silently stop running: the smoke test requires the memory marker *and*
  the boot marker, in that order, in both boot modes.
- Unit tests cover the untrusted-input logic on the host, including zero-length regions,
  unrepresentable end addresses, saturating byte totals, reversed start/end bounds, unsupported frame
  sizes, partial-frame handling, bitmap overflow reporting, non-usable claims overriding usable ones,
  call-order enforcement and rejected calls leaving allocator state unchanged.

## What this does NOT prove

- **No virtual memory.** The kernel still runs on the page tables the `bootloader` component created.
  Nothing here maps, unmaps, protects or isolates memory.
- **No deallocation and no reclamation.** One bit per frame cannot distinguish "allocated" from
  "reserved"; there is no `free`, and bootloader memory is not reclaimed.
- **No interrupt, exception, timer or scheduler support.** Page faults and general-protection faults
  are not handled by us yet; the milestone-2 fault-diagnostics work has not started.
- **No protection claim.** A kernel bug can still corrupt anything it can address.
- **Not all physical memory is managed.** The bitmap covers the first 4 GiB at 4 KiB frames; whole
  usable frames above that are counted and printed but not managed.
- **The kernel stack and framebuffer are not reserved by address** (see the limitation below).
- **No physical-hardware validation**, no KVM-accelerated runs, no other host.
- **Byte-for-byte image reproducibility is not established** (see below).

## Known limitations and open questions

1. **Kernel stack and framebuffer reservations.** `BootInfo` gives `kernel_addr` as a physical address
   but `kernel_stack_bottom` as a *virtual* one, and the framebuffer is mapped at a fixed high virtual
   address, so the kernel cannot compute their physical frames and does not pretend to reserve them. It
   relies on the bootloader having marked their frames non-usable in the map, prints that fact on the
   console, and this remains a gap until the kernel owns its page tables and can translate addresses
   itself. A wrong assumption here would not be caught by the current tests.
2. **Image reproducibility is not established, and an earlier claim was too strong.** Multiple
   regenerations of the same kernel produced identical `bios.img` bytes in one session (sha256
   `27fd70d9…` seven times for the milestone-1 kernel), which the milestone-1 evidence recorded as a
   BIOS-image reproducibility property. A later regeneration in the same environment produced a
   different `bios.img` for the same source (sha256 `cd96c2e3…` once, then `94153b13…` seven times),
   and the cause was not identified. Treat **neither** image as byte-for-byte reproducible; the
   milestone-1 document has been corrected. Behaviour (booting and emitting the same markers) is
   reproducible even when the bytes are not.
3. **The `Bootloader` kind is treated as reserved forever.** Reclaiming it is possible in principle
   once the kernel has its own page tables, but that is a later milestone and needs the address
   translation that this slice deliberately avoids.
4. **The 4 GiB bitmap is a fixed kernel constant.** Guests with more usable memory above 4 GiB would
   have that memory reported as unmanaged, not used.

## Not run

| Check | Status | Reason / next step |
| --- | --- | --- |
| Exception and fault diagnostics | NOT RUN | Milestone 2's remaining item; no IDT or fault handler exists yet |
| Page tables of our own | NOT RUN | Follows the bootstrap allocation recorded here |
| Physical hardware boot | NOT RUN | Out of scope; requires owner approval and a reviewed procedure |
| KVM-accelerated run | NOT RUN | Only `tcg` was used, for comparability with CI |
| Reproducible-image comparison | ATTEMPTED, NOT ESTABLISHED | See limitation 2 |
| Fuzzing of the memory-map validation | NOT RUN | The validator is unit tested against malformed maps; fuzzing is scheduled with the parser work in milestone 9 |
| CI execution | NOT RUN | Nothing has been pushed; the workflow has still never executed |
