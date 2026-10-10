# ADR-0013: Project Licence and Third-Party Provenance

- **Status:** proposed; owner decision required before selecting a project-wide licence
- **Date:** 2026-10-10

## Context
The GitHub repository is intended to be public, but the owner has not selected a project licence. Third-party boot, graphics, compatibility or systems components may impose different licence obligations.

## Decision
Do not silently select or add a project-wide licence. Maintain `LICENCE-STATUS.md` and a dependency/licence inventory. Preserve notices for each reused component and assess source, binary, asset, firmware and patent terms before redistributing. Present the owner with viable options and consequences, then record the decision in a superseding ADR.

## Consequences
Public visibility does not itself grant broad reuse permissions. An unlicensed project can discourage contributions and create uncertainty. The licence decision should be made before significant public implementation and before distributing a build.

## Validation plan
Audit the dependency graph and assets before public releases; attach an SBOM/licence inventory once release engineering exists.
