# Architecture boundary guardrails

The plugin refactor uses a few intentionally simple tests to keep the new
same-crate boundaries honest while the code is still in `ambition_sandbox`.
These tests are not a substitute for rustc, but they give implementation agents
fast feedback when a patch moves in the wrong architectural direction.

## Current guardrails

- `platformer_runtime/**` must not import Ambition content, app assembly,
presentation, dev tools, portal, music, quest, or sandbox asset modules.
- Room-authored spawn modules under `content/features/ecs/spawn*.rs` should not
add new raw `commands.spawn(...)` calls. New room-authored spawn sites should use
`SpawnScopedExt::spawn_room_scoped` or another explicit lifecycle helper.
- `app/plugins.rs` should not regain portal or held-item subsystem registration
helpers after those registrations moved into module-owned plugins.
- Non-portal mechanics should call `platformer_runtime::collision::raycast_solids`
  for plain solid raycasts instead of importing the portal mechanic.
- Cross-subsystem ordering should prefer public `SystemSet` labels, such as
  `ItemPickupSet`, rather than concrete function references.
- The portal mechanic should remain a facade module at `src/portal/` with
  plugin registration in `portal/plugin.rs`, schedule labels in
  `portal/schedule.rs`, and implementation details behind `portal/implementation.rs`.
  Remove the legacy `src/portal.rs` file when applying overlays that introduce
  this split.

## Updating the allowlist

The allowlist lives in
`docs/architecture/architecture-boundary-allowlist.txt`. It records legacy raw
spawn counts by source-relative path. Prefer reducing counts by migrating call
sites to lifecycle helpers. Increase a count only when the raw spawn is
intentional, non-room-authored, and documented in the review/commit.

Run the guardrails with:

```bash
cargo test -p ambition_sandbox architecture_boundaries
```

When a boundary intentionally changes, update this document, the allowlist, and
`tests/architecture_boundaries.rs` in the same patch so the new rule is visible.
