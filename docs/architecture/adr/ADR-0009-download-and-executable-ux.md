# ADR-0009: Browser Downloads and Click-to-Launch Executable UX

- **Status:** accepted as owner-supplied product requirement; implementation architecture remains proposed
- **Date:** 2026-10-10

## Decision
The target user journey is browser download → Downloads folder → double-click → GUI launch or installer flow for supported Windows `.exe` applications, without command-line use or manual execute-permission changes. The shell must route PE/COFF files to an explicit compatibility handler and must not execute files automatically when a download completes.

## Important distinction
Recognising a file and associating it with a handler is not enough to run Windows code. A real compatibility subsystem needs a PE loader, process/thread and memory semantics, Windows API/service implementations, relevant graphics/input/audio/network behaviour, dependencies and installer support. Support varies by application. Some anti-cheat, DRM, proprietary services and Windows kernel drivers may remain unsupported.

## Security requirements
Preserve provenance where available; show transparent first-launch warnings; never imply a signature means malware-free; do not upload files/hashes without informed opt-in; run installers with least privilege and isolated app data/prefixes when feasible; present useful GUI errors; add malformed-PE parser tests and fuzzing.

## Validation plan
Maintain distinct statuses for recognised, associated, loaded, launched and application-tested. Publish exact app/version/runtime/test scenario for each compatibility claim.
