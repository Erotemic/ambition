# Closeout review followups — routed residuals

**Status:** closed as an independent planning authority on 2026-08-30.

The July closeout review accumulated several unrelated residuals. Current HEAD
and later architecture work give each surviving item a better owner, so this
file no longer owns executable work. The full investigation remains in git
history and in the existing superseded-planning archive.

## Current routing

- **Portal mapping convention:** still source-backed. The live process-global
  `PORTAL_MAP_ROTATION` policy is queue row `D-PORTAL-POLICY` under
  [`simulation-authority-and-determinism.md`](simulation-authority-and-determinism.md).
- **Shipping/fresh-clone configurations:** owned by
  [`project-build-and-distribution.md`](project-build-and-distribution.md) and the
  build/platform reservoir in [`../tracks.md`](../tracks.md).
- **Rollback schema fingerprint cost:** the registry now memoizes its schema
  fingerprint with `OnceLock`; do not preserve the old hot-path claim as open
  work.
- **Projectile-view cloning / repeated collision composition:** neither has a
  current measured budget failure. They are covered by the standing rule in
  [`performance-and-iteration.md`](performance-and-iteration.md): reopen generic
  CPU work only from representative measurements.
- **Dormant `GravityFlipSwitch`:** retained as a small convergence trigger in
  [`../tracks.md`](../tracks.md); delete the unused path unless a real authored
  overlap-plate customer appears.
- **Provider-owned persistence and item identities:** retained as external-provider
  triggers in [`../tracks.md`](../tracks.md). Do not build generic provider
  abstractions before a second real consumer needs them.

No future work should be added here. Route new findings directly to the current
queue, a focused plan, `tracks.md`, or a maintainer decision.
