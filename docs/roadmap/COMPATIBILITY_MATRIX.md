# Compatibility Matrix

No hardware or third-party Windows application compatibility is verified by this blueprint. Add rows only after a test actually runs. Avoid broad labels such as “NVIDIA support” or “Windows apps supported.”

| Category | Device/application + exact version | OS commit/build | Runtime/driver version | Environment | Scenario tested | Result | Limitations/logs/date |
|---|---|---|---|---|---|---|---|
| Boot/VM | TBD | TBD | QEMU/firmware TBD | Virtual | TBD | Not tested | No implementation evidence yet |

## Required categories over time

- QEMU machine and firmware versions.
- Virtual block storage and each filesystem operation.
- Virtual networking and protocol scenarios.
- Input/display modes and supported resolutions.
- Native applications, installer/update/uninstall cases.
- Windows executable recognition, loader, named apps and installer scenarios.
- Windows graphics APIs, audio, networking and input where implemented.
- Physical hardware by exact make/model/firmware/driver version.
- Gaming, DRM and anti-cheat by exact game/build and test scenario; never infer support from another title.
