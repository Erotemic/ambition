# Architecture boundary guardrails

Source-scanning tests keep the crate boundaries honest. The live boundaries are
real crates — foundations ← machinery lib (`ambition_actors`) ←
content (`ambition_content`) ← app (`ambition_app`). They are enforced by
declarative policies + custom scanners, not by rustc; fast directional feedback.

## Where the guards live

The authoritative guards live in the sequestered workspace-policy package
`tests/ambition_workspace_policy` (see
`docs/planning/engine/test-organization-migration.md` for the migration matrix).
That package inspects the workspace as DATA (parsed manifests + source walking)
and links no production crate, so running the policy suite never compiles
`ambition_app`.

The legacy `game/ambition_app/tests/architecture_boundaries.rs` binary is **gone**
(deleted 2026-07-10). All 67 of its tests were migrated: 65 to declarative
`policies/*.toml` rules and 2 to custom scanners (raw-spawn allowlist,
archetype-free enemy config). Each ported rule keeps a stable policy ID derived
from its old test name, so `git log -S <old-name>` still finds the history.

## Ownership model

Policies are grouped by scope, and each scope is an independently filterable test:

- **repository** — workspace membership, top-level layout, umbrella/demo homes.
- **engine** — `crates/*` layering, foundation purity, determinism, control-frame
  ownership, module-size.
- **game** — `game/*` content ownership, app composition, named-content
  registration.

Every policy carries `id`, `scope`, `owners`, `watch_paths`, `kind`, `rationale`,
`source_doc`, and `severity`. Declarative rule kinds (required/forbidden path,
workspace-member, dependency allow/deny-list, forbidden-source-reference,
module-size) are DATA in `policies/*.toml`. Unusual semantic scanners (the
determinism lints, the ControlFrame allowlist, umbrella/plugin composition
shapes) stay as readable custom Rust modules under `src/custom/`, configured by
their own data files. A failure names the policy ID, the owners, the offending
file/dependency, and the relevant policy/waiver file.

## Current guardrails (representative)

- **Machinery imports no content**: every `ambition_actors` dir is scanned for
`crate::content::` / `ambition_content::` — none may appear.
- Foundation crates (`ambition_platformer_primitives`, `ambition_portal`,
`ambition_time`, `ambition_input`, `ambition_menu`, `ambition_audio`) must not
depend on `ambition_actors`/content/app or name game content.
- The combat kit (`combat`) must name no archetype/boss content.
- The enemy roster is content-owned DATA: the persisted `EnemyConfig` +
per-frame `EnemyMut` stay archetype-free, and there is no `EnemyArchetype` enum.
- Room-authored spawn modules under `features/ecs/spawn*.rs` should not add raw
`commands.spawn(...)`; use `SpawnScopedExt::spawn_room_scoped`.
- Presentation (`ambition_render`) reads the `ambition_sim_view` read-model, never
live sim STATE; the windowed host names no content.
- New gameplay subsystems are self-owning `Plugin`s, not app-assembly hand-wiring.

## Updating the raw-spawn allowlist

The allowlist lives in
`docs/architecture/architecture-boundary-allowlist.txt`. It records legacy raw
spawn counts by source-relative path. Prefer reducing counts by migrating call
sites to lifecycle helpers. Increase a count only when the raw spawn is
intentional, non-room-authored, and documented in the review/commit.

## Running the guards

```bash
cargo test -p ambition_workspace_policy repository_policies
cargo test -p ambition_workspace_policy engine_policies
cargo test -p ambition_workspace_policy game_policies
cargo test -p ambition_workspace_policy            # all scopes + self-tests
```

When a boundary intentionally changes, update this document and the relevant
policy file (or custom module) in the same patch, so the new rule is visible.
