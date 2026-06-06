# 0002 — architecture boundary guardrails

Added the first lightweight tests and docs for same-crate architectural seams.

Why this matters:

- Keeps `platformer_runtime/**` free of Ambition content, app assembly,
  presentation, dev tools, and portal references.
- Starts policing raw room-authored spawn growth.
- Prevents `app/plugins.rs` from re-owning portal and held-item registrations
  once those systems move into module-owned plugins.

Main files:

- `crates/ambition_sandbox/tests/architecture_boundaries.rs`
- `docs/adr/0019-pluginized-platformer-runtime.md`
- `docs/architecture/architecture-boundaries.md`
- `docs/architecture/architecture-boundary-allowlist.txt`
