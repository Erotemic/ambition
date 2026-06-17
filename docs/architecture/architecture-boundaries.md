# Architecture boundary guardrails

Source-scanning tests keep the crate boundaries honest. The live boundaries are real crates — foundations ← machinery lib (`ambition_gameplay_core`)
← content (`ambition_content`) ← app (`ambition_app`) — and the guards (~22) live
in `crates/ambition_app/tests/architecture_boundaries.rs`. Not a substitute for
rustc, but fast directional feedback.

## Current guardrails (representative)

- **Machinery imports no content**: every `ambition_gameplay_core` dir is scanned for
`crate::content::` / `ambition_content::` — none may appear. `crate::features`
(the named actor/boss ECS world still in the lib) is the one tracked exception.
- Foundation crates (`ambition_platformer_primitives`, `ambition_portal`,
`ambition_time`, `ambition_input`, `ambition_menu`, `ambition_audio`) must not
depend on `ambition_gameplay_core`/content/app or name game content.
- The combat kit (`combat`) must name no archetype/boss content.
- The enemy roster is content-owned DATA: the lib's persisted `EnemyConfig` +
per-frame `EnemyMut` stay archetype-free (project `EnemyTuning` /
`EnemyBrainSpec` / `CombatCapabilities` at spawn), and there is no
`EnemyArchetype` enum — enemies resolve by spawn brain key against the
content-installed `EnemyRoster`. Guard:
`architecture_boundaries_enemy_config_is_archetype_free`.
- Room-authored spawn modules under `features/ecs/spawn*.rs` should not add raw
`commands.spawn(...)`; use `SpawnScopedExt::spawn_room_scoped`.
- Lib `menu`/`dev` keep only the persistence/sim-coupled pieces; the menu host
stack + dev overlays/inspectors live in `ambition_app`.
- Non-portal mechanics call `platformer_runtime::collision::raycast_solids`, not
the portal mechanic; cross-subsystem ordering prefers public `SystemSet` labels
(e.g. `ItemPickupSet`) over concrete function references.
- New gameplay subsystems are self-owning `Plugin`s, not app-assembly hand-wiring.

## Updating the allowlist

The allowlist lives in
`docs/architecture/architecture-boundary-allowlist.txt`. It records legacy raw
spawn counts by source-relative path. Prefer reducing counts by migrating call
sites to lifecycle helpers. Increase a count only when the raw spawn is
intentional, non-room-authored, and documented in the review/commit.

Run the guardrails with:

```bash
cargo test -p ambition_app --test architecture_boundaries
```

When a boundary intentionally changes, update this document, the allowlist, and
`tests/architecture_boundaries.rs` in the same patch so the new rule is visible.
