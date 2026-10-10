# ADR-0012: Public GitHub Development and CI Security

- **Status:** accepted as owner-supplied workflow direction
- **Date:** 2026-10-10

## Decision
The repository is public from the beginning. Use feature branches with logical commits, push branches when credentials and remotes are configured, open a reviewable PR, and merge only after required checks pass and repository policy allows. Do not bypass branch protections or owner-approval requirements.

## CI security
Default `GITHUB_TOKEN` permissions to read-only. Grant individual write permissions only to the exact job that needs them and document why. Never expose secrets to fork/untrusted PR code. Avoid third-party Actions unless justified; pin them to immutable SHAs where practical. No production signing secrets in ordinary CI. Do not print tokens or all environment variables.

## Owner boundaries
Autonomous agent may edit, build, test, commit, push branches and open PRs. Owner approval is required for physical disk writes, host boot changes, firmware flashes, destructive cleanup and other privileged/irreversible actions. Agent must not change public repository visibility, choose a project licence or publish a release unless explicitly authorised.

## Validation plan
Review workflow permissions, test that the default PR path has no secret access, and document actual CI checks. Do not claim a push/PR/merge occurred unless verified.

## References
- https://docs.github.com/en/actions/reference/security/secure-use — verify current GitHub security guidance when implementing workflows.
