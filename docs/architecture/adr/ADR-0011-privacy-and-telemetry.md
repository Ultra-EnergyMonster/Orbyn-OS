# ADR-0011: Privacy, Telemetry and Local-First Defaults

- **Status:** accepted as owner-supplied product direction
- **Date:** 2026-10-10

## Decision
No advertising, Recall-like recording or indexing of user activity, persistent screenshots, keylogging, covert analytics, or mandatory cloud account for basic local use. Diagnostic collection must be disabled by default and, if later offered, clearly disclose data types, purposes, recipients, retention and revocation. Core desktop/file functions should work offline when their function does not inherently require a network.

## Consequences
The OS still needs security updates and may require network requests when a user uses network functionality. Document system network endpoints and separate required updates from optional diagnostics. “No telemetry” is not permission to ignore security logging that is necessary locally; local logs must remain minimised and privacy-conscious.

## Validation plan
Threat model and data-flow review; network tests in later milestones; tests for default-off diagnostics and opt-in revocation; configuration audit before releases.
