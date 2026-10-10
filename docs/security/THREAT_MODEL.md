# Initial Threat Model

**Status:** initial outline; update before introducing user processes, downloads, packages, networking, updates, or privileged services.

## Assets

- Kernel integrity, boot configuration, page tables and privileged services.
- User files, passwords/credentials, application data and local account information.
- Package/update signing keys, build credentials and GitHub credentials.
- System binaries, application binaries, package metadata and compatibility-runtime prefixes.
- Privacy: browsing/download metadata, app usage, logs, crash dumps, microphone/camera access and screen content.
- Integrity of build artifacts and provenance of third-party dependencies.

## Adversaries and failure sources

- Malicious or malformed downloads, archives, `.exe` files, fonts, images, media and package metadata.
- A malicious native or compatibility app attempting privilege escalation or cross-app data access.
- Compromised dependency, build action, package source, update mirror, GitHub account or CI workflow.
- Accidental data loss caused by installers, partitioning, boot code, updates or agent commands.
- Hostile or buggy drivers and firmware.
- A compromised user session.
- An OS vendor/service accidentally collecting more user data than intended.

## Security principles

- Least privilege, explicit boundaries, defence in depth and auditable permission changes.
- No automatic execution of new downloads.
- No system-wide install authority for an app merely because its installer is an `.exe`.
- Separate native applications and Windows compatibility programs from privileged kernel services as architecture permits.
- Validate all untrusted input with checked arithmetic and bounds checks; parser fuzzing is a planned requirement.
- Signed updates with rollback/recovery and carefully protected signing keys before production releases.
- No secrets in public repos or untrusted CI contexts; least-privilege GitHub Actions.
- No covert telemetry or screen/activity capture. Diagnostics remain off unless the user explicitly opts in.
- Logging should be useful but must not capture credentials, private file contents, or broad user-activity histories by default.

## Threats specific to Windows `.exe` handling

- Fake extensions or malformed PE headers exploiting parser bugs.
- Social engineering and phishing through installers.
- Signed but malicious programs; signature validity is not a safety verdict.
- DLL search/path confusion, unsafe shared libraries, installer persistence and cross-prefix data access.
- Programs that assume Windows kernel behaviour or load vulnerable drivers.
- Compatibility layers exposing undocumented services or unsafe handles.

## Staged mitigations

Early stages: no execution support; parser tests must use benign test fixtures and never launch untrusted files. Later stages: safe parser, least-privilege loader, runtime isolation, download-origin metadata, transparent warning UI, app-specific data directories/prefixes, controlled IPC, reproducible compatibility tests, uninstall/rollback, and vulnerability reporting process.

## Open questions

- Kernel architecture and whether selected complex drivers/services can be isolated.
- Capability/permission model and user/kernel separation.
- Verified boot and update signing/key recovery.
- Sandboxing mechanism and host/runtime ABI boundaries.
- How to retain download-origin metadata across browser, filesystem, archive extraction and file-manager actions.
- Crash log retention and redaction defaults.

Do not mark these mitigations implemented until code and tests exist. “No telemetry” does not itself demonstrate system security.
