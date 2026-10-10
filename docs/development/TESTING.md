# Testing

Use the verification vocabulary in `docs/verification/VERIFICATION.md`.

## Initial checks

Only use these once the relevant project configuration exists and confirm target support:

```sh
cargo fmt --check
cargo test --workspace
```

Run Clippy only for targets where its lints/tooling are configured and available. Kernel build/test commands and QEMU smoke-test commands must be added by the first implementation task after they are verified in the selected toolchain.

## Boot smoke test requirements

- Build the kernel and generate a virtual boot image in an ignored `build/` or `target/` directory.
- Launch QEMU with an explicit machine/firmware configuration and serial output capture.
- Enforce a timeout; never let CI hang indefinitely.
- Assert the expected marker appears after the intended initialisation succeeded.
- Store concise test logs/artifacts where useful, without credentials or private host data.
- Do not attach real host disks or USB block devices.
- A marker emitted by a host helper script is not proof that the guest kernel booted. The marker must originate from the guest kernel's serial path.

## Unavailable dependencies

If QEMU, firmware or target toolchain components are missing, run other checks that can be verified, record the missing dependency, and mark the boot test `NOT RUN`. Never fake a pass.
