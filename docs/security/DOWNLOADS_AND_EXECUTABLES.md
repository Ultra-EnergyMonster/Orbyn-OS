# Downloads and Windows Executable UX

## Product requirement

A user should be able to download a Windows `.exe` in a browser, open Downloads, double-click it, and run a supported app or installer through a graphical workflow. They must not need a terminal command or manually set an execute bit. This is a long-term requirement for a complete compatibility subsystem, not a promise of the current prototype.

## Core design

1. Browser downloads save to a predictable user Downloads directory by default.
2. The filesystem and shell preserve file content and relevant origin/provenance metadata where possible; metadata is not treated as cryptographic proof.
3. The file manager identifies PE/COFF executables from validated content and extension rather than trusting the extension alone.
4. The shell looks up the file association. A Windows `.exe` is routed to the OS's compatibility service only when a valid runtime is installed. It must never be sent to a host shell or executed as a native binary by accident.
5. New Internet downloads are never executed automatically when download completes.
6. On first launch, present concise and proportionate security information, including source/origin if known, signature publisher/status where available, requested access, and known compatibility limitations. Do not state that signed means safe.
7. Unsupported apps receive a clear graphical message. Do not force the user into terminal troubleshooting.
8. An installer initially runs within a constrained compatibility prefix/app environment rather than gaining unrestricted OS-wide authority. Show changes, permissions and uninstall/recovery options.
9. Users can select “Open with…”, change defaults, inspect properties, cancel, and remove the download.

## Security and privacy

- No upload of executable contents, hashes, paths or browsing history to an online reputation service without explicit informed opt-in.
- Do not build a persistent activity timeline, screenshot index, keylogger, or covert application-use database.
- Parse untrusted PE metadata with size limits, bounds checks, checked arithmetic, malformed-input tests and fuzzing when available.
- File type recognition does not equal executable support; PE parsing does not equal execution; successful startup does not equal full functionality.
- Do not silently weaken security settings for compatibility.

## Compatibility layers required

A complete solution may need a PE loader, memory/process/thread semantics, loader relocations/import resolution, API and service implementation, filesystem/registry-like behaviour, graphics/input/audio/network integration, installer handling, dependency management, and application-specific compatibility work. Windows kernel drivers and some anti-cheat/DRM systems may remain unsupported. Consider third-party compatibility projects only after reviewing actual porting requirements and licences.

## Acceptance states

- **Recognised:** UI can identify a PE file and show its properties.
- **Associated:** double-click opens a handler or informative runtime-unavailable dialogue.
- **Loaded:** a compatible PE has been loaded into an isolated process. This alone is not application success.
- **Launched:** entry point started successfully.
- **Application tested:** named application/version passed explicit scenarios with known limitations recorded.

The UI and documentation must use accurate language for each state.
