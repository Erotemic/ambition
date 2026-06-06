# Goals and Constraints

## Primary goals

### 1. Clean architecture

The code should communicate architectural boundaries through module layout, crate dependencies, plugin APIs, and tests. Important rules should not live only in comments or tribal knowledge.

Examples of rules that should become code/API-level structure:

- Room-local entities are spawned through lifecycle-aware helpers.
- Cross-plugin ordering goes through schedule sets or messages.
- Optional mechanics do not own generic runtime utilities.
- Game content does not leak into engine/runtime crates.
- Presentation and authoring adapters are optional layers.

### 2. Reusable Bevy 2D platformer runtime

Ambition should eventually produce a reusable platformer runtime that can support other 2D Bevy platformer games.

Reusable runtime code should include things like:

- Body and velocity abstractions.
- Movement/collision primitives.
- Room lifecycle and loading zones.
- Generic world queries and raycasts.
- Generic projectiles/hitboxes where appropriate.
- Generic scheduling conventions.
- Generic interaction and lifecycle messages.

It should not include Ambition-specific nouns such as named bosses, named quests, specific worlds, intro routes, cut-the-rope content, PortalGun item roster entries, or music/dialogue IDs.

### 3. Optional mechanics plugins

Mechanics such as portals, gravity zones, held items, combat, encounters, boss runtime, and falling sand should become optional plugins where possible.

This does not mean every ability becomes a crate immediately. Portal is the best pilot because it is large, cross-cutting, optional, and potentially reusable.

### 4. Faster builds by persona

The final topology should allow supported build personas such as:

- `headless_runtime`
- `gameplay_dev`
- `desktop_dev`
- `web`
- `android`
- `content_validation`

Headless checks should avoid rendering, audio, UI, LDtk runtime, inspector/devtools, and optional mechanics unless explicitly requested.

### 5. Easier onboarding and agent navigation

The disk layout should answer common questions quickly:

```text
Where is reusable movement?          platformer_runtime or platformer_core
Where is portal gameplay?            mechanics_portal
Where are portal visuals?            portal_render
Where is LDtk portal conversion?      portal_ldtk
Where is Ambition's PortalGun item?   ambition_content/portal
Where is app composition?             ambition_sandbox/app
```

## Non-goals

### Do not rewrite the game

This refactor should proceed by creating boundaries, moving ownership, and fixing compile errors. Avoid large rewrites of movement, portal transit, inventory, or world loading unless a specific boundary requires it.

### Do not data-drive everything

The goal is mostly data-driven content, not an interpreter for all mechanics. Keep complex behavior in Rust plugins. Move names, rosters, tuning, map bindings, quest definitions, and content manifests into data/content layers.

Good data-driven candidates:

- Item roster and tuning.
- Quest specs and flags.
- Boss numeric tuning and phase tables.
- Enemy archetype stats.
- Encounter wave composition.
- Reward tables.
- Music cue mapping.
- World manifests.

Good Rust plugin candidates:

- Portal transit algorithm.
- Ledge grab solver.
- Collision repair.
- Projectile collision.
- Gravity field resolution.
- Boss attack execution kernels.

### Do not support arbitrary feature combinations immediately

Use named build personas and validate those. Do not try to make every possible Cargo feature combination work from day one.

### Do not add long-lived compatibility bridges

Prefer breaking imports and fixing forward over building broad bridge layers. Temporary re-exports are acceptable when they are either the final public API path or removed quickly.

## Design constraint: staged shotgun refactor

The preferred style is intentionally breaking the system along useful seams, then fixing forward. Breakage is useful when it reveals the real dependency graph and points toward the final architecture.

Good breakage:

- `crate::portal::raycast_solids` users break after raycast moves to runtime world query.
- `app/plugins.rs` users break when subsystem registration moves into plugins.
- Portal color callers break when gun colors and authored gate colors are split.
- LDtk conversion breaks when portal entities become feature-gated.

Bad breakage:

- Broad stale full-file replacement of a hotspot file.
- Temporary duplicate systems that keep old and new architecture alive indefinitely.
- Compatibility wrappers that hide the dependency direction problem.
