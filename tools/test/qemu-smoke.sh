#!/usr/bin/env bash
#
# Bounded QEMU boot smoke test for the New OS kernel prototype (milestone 1).
#
# Boots a generated disk image, captures the *guest kernel's* COM1 serial output to a log file,
# and fails unless the success marker appears. A timeout on its own is never treated as a pass.
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

readonly MARKER="NEW_OS_KERNEL_BOOT_OK"
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
    grep -q -F "$MARKER" "$serial_log" 2>/dev/null && break
    # Stop waiting early if QEMU died on its own (for example a firmware or image error).
    kill -0 "$qemu_pid" 2>/dev/null || break
    sleep 0.2
done

cleanup
trap - EXIT

echo "--- guest serial output ---"
cat "$serial_log" 2>/dev/null || true
echo "--- end guest serial output ---"

if grep -q -F "$MARKER" "$serial_log" 2>/dev/null; then
    echo "PASS: guest kernel emitted $MARKER (mode=$MODE, qemu=$("$QEMU_BIN" --version | head -n 1), accel=$QEMU_ACCEL)"
    exit 0
fi

echo "--- qemu output (tail) ---" >&2
tail -n 20 "$qemu_log" >&2 || true
fail "guest kernel did not emit $MARKER within ${QEMU_TIMEOUT}s (mode=$MODE); see $serial_log"
