# ADR-0010: Windows Application Compatibility Strategy

- **Status:** proposed
- **Date:** 2026-10-10

## Context
The owner wants Windows application UX to be as natural as native apps over the long term. The kernel is original and cannot assume Windows APIs, Windows system services or Windows driver compatibility.

## Options to evaluate
- Build compatible user-mode APIs incrementally for a chosen application subset.
- Port or adapt appropriate existing open-source compatibility components, after detailed upstream, licence, ABI and dependency review.
- Use an app-by-app compatibility approach with a published matrix while growing broader API coverage.

## Decision
Treat compatibility as a separate multi-stage programme, designed for from the beginning but implemented after the process model, user ABI, filesystem, graphics and runtime boundaries exist. Begin with a small, benign test application and an explicitly scoped API surface. Do not claim that Wine/Proton or any Linux-hosted compatibility layer can run unchanged on this kernel; determine actual host syscall, libc, process, graphics and service assumptions before proposing a port.

## Consequences
This may take years and still not cover all Windows apps, especially proprietary kernel drivers, anti-cheat, DRM and apps depending on undocumented Windows behaviour. The UI should be click-first even when support is missing, with clear GUI limitations and no terminal requirement for users.

## Validation plan
Use a compatibility matrix with exact app build, OS commit, runtime build, environment, scenarios and known failures. Never infer support for one app from another.
