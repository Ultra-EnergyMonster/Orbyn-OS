# Releasing

**Status:** template only. No release process is verified or implemented yet.

Do not publish a release until the owner has explicitly authorised a release process, the project licence is decided, build inputs are documented, and security/compatibility claims match evidence.

## Future release checklist

- [ ] Release is built from an identified reviewed commit/tag.
- [ ] Build toolchain and inputs are recorded; build is reproducible to the documented standard.
- [ ] Applicable unit/integration tests pass.
- [ ] QEMU boot tests pass for the claimed boot path, with logs retained.
- [ ] The image is verified to avoid writing to or modifying host disks when run in its documented default mode.
- [ ] SBOM/dependency licence inventory and third-party notices are prepared.
- [ ] Update/package signing keys are not stored in source control or ordinary CI secrets available to untrusted code.
- [ ] Checksums and build provenance are generated.
- [ ] Security and privacy status, known issues, hardware matrix and Windows-app compatibility matrix are published.
- [ ] Install, update, rollback and recovery procedures have actually been tested, where implemented.
- [ ] Project licence and redistribution rights are clear.
- [ ] Human owner approves the release.

Do not issue production-grade security or broad-compatibility claims based only on automated CI.
