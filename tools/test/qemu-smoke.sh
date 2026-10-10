#!/usr/bin/env bash
#
# Bounded QEMU boot smoke test for the New OS kernel prototype (milestones 1 and 2).
#
# Boots a generated disk image, captures the *guest kernel's* COM1 serial output to a log file, and
# fails unless all four success markers appear, in the order the kernel is supposed to emit them:
#
#   NEW_OS_KERNEL_EXCEPTION_OK  the exception tables were installed and both exception self-tests
#                               (a real breakpoint trap and a real, resumed page fault) passed
#   NEW_OS_KERNEL_TIMER_OK      the PIT timer was started and was observed to interrupt the CPU
#   NEW_OS_KERNEL_MEMORY_OK     the memory map was validated and the frame allocator was built and used
#   NEW_OS_KERNEL_BOOT_OK       the whole initialisation path succeeded
#
# A timeout on its own is never treated as a pass.
#
# Safety (AGENTS.md rule 4, docs/verification/VERIFICATION.md):
#   The only disks QEMU touches are generated image files under build/boot/ plus a read-only copy
#   of the system firmware. No physical disk, host block device, USB device or passthrough storage
#   is ever attached, and no host boot configuration is modified.
#
# Usage: tools/test/qemu-smoke.sh [bios|uefi]
#
# Environment overrides:
#   QEMU_BIN      QEMU binary to use               (default: qemu-system-x86_64)
#   QEMU_ACCEL    QEMU accelerator                 (default: tcg, for reproducible CI results)
#   QEMU_TIMEOUT  seconds to wait for the marker   (default: 30)
#   GUEST_MEMORY  guest RAM size                   (default: 256M)

set -euo pipefail

readonly EXCEPTION_MARKER="NEW_OS_KERNEL_EXCEPTION_OK"
readonly TIMER_MARKER="NEW_OS_KERNEL_TIMER_OK"
readonly MEMORY_MARKER="NEW_OS_KERNEL_MEMORY_OK"
readonly BOOT_MARKER="NEW_OS_KERNEL_BOOT_OK"
readonly MODE="${1:-bios}"
readonly QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
readonly QEMU_ACCEL="${QEMU_ACCEL:-tcg}"
readonly QEMU_TIMEOUT="${QEMU_TIMEOUT:-30}"
readonly GUEST_MEMORY="${GUEST_MEMORY:-256M}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly repo_root
readonly serial_log="$repo_root/build/test/serial-$MODE.log"
readonly qemu_log="$repo_root/build/test/qemu-$MODE.log"

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

case "$MODE" in
    bios | uefi) ;;
    *) fail "unknown mode '$MODE' (usage: $0 [bios|uefi])" ;;
esac

command -v "$QEMU_BIN" >/dev/null 2>&1 ||
    fail "$QEMU_BIN not found in PATH. Install it (Debian/Ubuntu: 'sudo apt install qemu-system-x86 ovmf') and re-run. This boot test is NOT RUN, not passed."

image="$repo_root/build/boot/$MODE.img"
if [ ! -f "$image" ]; then
    fail "missing boot image '$image'. Generate it first: cargo build -p boot-image (from the repository root)."
fi

mkdir -p "$(dirname "$serial_log")"
: >"$serial_log"
: >"$qemu_log"

qemu_args=(-accel "$QEMU_ACCEL" -m "$GUEST_MEMORY" -display none -no-reboot -serial "file:$serial_log")

if [ "$MODE" = "bios" ]; then
    # SeaBIOS-compatible legacy boot from the raw BIOS disk image.
    qemu_args+=(-machine pc -drive "format=raw,file=$image")
else
    # UEFI boot: unit 0 is the read-only firmware code, unit 1 is a writable copy-on-write vars
    # store (snapshot=on keeps the system OVMF_VARS file untouched), the disk image is the ESP.
    ovmf_code=""
    ovmf_vars=""
    for code in /usr/share/OVMF/OVMF_CODE_4M.fd /usr/share/OVMF/OVMF_CODE.fd /usr/share/OVMF/OVMF_CODE_2M.fd; do
        vars="${code/CODE/VARS}"
        if [ -f "$code" ] && [ -f "$vars" ]; then
            ovmf_code="$code"
            ovmf_vars="$vars"
            break
        fi
    done
    if [ -z "$ovmf_code" ]; then
        fail "UEFI firmware not found (looked for OVMF_CODE*/OVMF_VARS* under /usr/share/OVMF). Install the 'ovmf' package and re-run. This boot test is NOT RUN, not passed."
    fi
    echo "using UEFI firmware: $ovmf_code"
    qemu_args+=(
        -machine q35
        -drive "if=pflash,format=raw,unit=0,readonly=on,file=$ovmf_code"
        -drive "if=pflash,format=raw,unit=1,snapshot=on,file=$ovmf_vars"
        -drive "format=raw,file=$image"
    )
fi

echo "booting $MODE image in $("$QEMU_BIN" --version | head -n 1)"
echo "guest serial log: $serial_log"

"$QEMU_BIN" "${qemu_args[@]}" >"$qemu_log" 2>&1 &
qemu_pid=$!

cleanup() {
    kill "$qemu_pid" 2>/dev/null || true
    wait "$qemu_pid" 2>/dev/null || true
}
trap cleanup EXIT

deadline=$((SECONDS + QEMU_TIMEOUT))
while ((SECONDS < deadline)); do
    grep -q -F "$EXCEPTION_MARKER" "$serial_log" 2>/dev/null &&
        grep -q -F "$TIMER_MARKER" "$serial_log" 2>/dev/null &&
        grep -q -F "$MEMORY_MARKER" "$serial_log" 2>/dev/null &&
        grep -q -F "$BOOT_MARKER" "$serial_log" 2>/dev/null && break
    # Stop waiting early if QEMU died on its own (for example a firmware or image error).
    kill -0 "$qemu_pid" 2>/dev/null || break
    sleep 0.2
done

cleanup
trap - EXIT

echo "--- guest serial output ---"
cat "$serial_log" 2>/dev/null || true
echo "--- end guest serial output ---"

# Every marker is produced by the guest kernel's own serial code, so each one must be present.
missing=""
grep -q -F "$EXCEPTION_MARKER" "$serial_log" 2>/dev/null || missing="$missing $EXCEPTION_MARKER"
grep -q -F "$TIMER_MARKER" "$serial_log" 2>/dev/null || missing="$missing $TIMER_MARKER"
grep -q -F "$MEMORY_MARKER" "$serial_log" 2>/dev/null || missing="$missing $MEMORY_MARKER"
grep -q -F "$BOOT_MARKER" "$serial_log" 2>/dev/null || missing="$missing $BOOT_MARKER"
if [ -n "$missing" ]; then
    echo "--- qemu output (tail) ---" >&2
    tail -n 20 "$qemu_log" >&2 || true
    fail "guest kernel did not emit:$missing within ${QEMU_TIMEOUT}s (mode=$MODE); see $serial_log"
fi

# Order matters: the exception and memory markers are emitted from inside the initialisation path,
# and the boot marker only afterwards. A boot marker printed first would mean the path was reordered
# or shortened in a way the test must not accept.
exception_line="$(grep -n -m 1 -F "$EXCEPTION_MARKER" "$serial_log" | cut -d: -f1)"
timer_line="$(grep -n -m 1 -F "$TIMER_MARKER" "$serial_log" | cut -d: -f1)"
memory_line="$(grep -n -m 1 -F "$MEMORY_MARKER" "$serial_log" | cut -d: -f1)"
boot_line="$(grep -n -m 1 -F "$BOOT_MARKER" "$serial_log" | cut -d: -f1)"
if ((exception_line >= timer_line || timer_line >= memory_line || memory_line >= boot_line)); then
    fail "guest kernel emitted its markers out of order: $EXCEPTION_MARKER (line $exception_line), $TIMER_MARKER (line $timer_line), $MEMORY_MARKER (line $memory_line), $BOOT_MARKER (line $boot_line); the initialisation path did not run in order"
fi

echo "PASS: guest kernel emitted $EXCEPTION_MARKER (line $exception_line), $TIMER_MARKER (line $timer_line), $MEMORY_MARKER (line $memory_line) then $BOOT_MARKER (line $boot_line) (mode=$MODE, qemu=$("$QEMU_BIN" --version | head -n 1), accel=$QEMU_ACCEL)"
exit 0
