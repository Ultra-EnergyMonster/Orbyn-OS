# ADR-0001: Project Goals and Scope

- **Status:** accepted as owner-supplied product direction
- **Date:** 2026-10-10

## Context
The owner wants an independent operating system with the usability associated with Windows while addressing bloatware, intrusive telemetry, unwanted AI activity capture, and poor user control. The OS should use its own kernel and not be a Linux distribution or a Windows-kernel derivative.

## Decision
Target x86-64 PCs first and consider ARM64 later. Use Rust with C and assembly where justified. Reuse appropriate open-source libraries, standards and tools after licence, maturity, target and security review. Build an original premium desktop design with familiar workflows. Prioritise privacy by default, local-first operation, no advertising/bloatware/Recall-style capture, graphical app installation and a long-term Windows application compatibility goal.

## Consequences and trade-offs
This is a long-term systems project. Hardware, graphics and Windows user-mode compatibility are large independent workstreams. Driver support and application support must be published as exact, tested matrices. The owner has not selected a final product name or licence.

## Security and privacy impact
No covert telemetry, screen capture, keylogging, or silent user-content uploads. Downloaded executables remain untrusted and must not auto-run.

## Validation plan
Each requirement becomes a milestone acceptance criterion. Claims must use the evidence levels defined in `docs/verification/VERIFICATION.md`.
