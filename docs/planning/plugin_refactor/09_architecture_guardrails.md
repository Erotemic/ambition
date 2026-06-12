# Architecture Guardrails

> Historical execution note: this file records the completed plugin-refactor run. It is not current planning guidance; use `docs/planning/plugin_refactor/README.md`, `22_monolith_breaker_survey.md`, and `runtime_extraction_backlog.md` for active follow-up.


This refactor needs tests that enforce architectural rules. These tests can be simple grep-style checks at first. The point is to make bad dependency direction visible to humans and agents.

## Forbidden import tests

Add a test such as `tests/architecture_boundaries.rs` or a small tool under `tools/validation/`.

### Runtime must not import game/content modules

Forbidden from `src/platformer_runtime/**` or future `ambition_platformer_*` crates:

```text
crate::content
crate::ambition_content
crate::intro
crate::boss_encounter
crate::quest
crate::assets::sandbox_assets
crate::music
crate::portal
crate::items
crate::app
crate::dev
crate::presentation
```

### Mechanics must not import Ambition content or apps

Forbidden from `src/mechanics/**` or future mechanics crates:

```text
crate::ambition_content
crate::app
crate::assets::sandbox_assets
crate::intro
crate::quest
crate::boss_encounter
```

Portal-specific forbidden imports:

```text
crate::items::Item
crate::inventory
crate::oot_menu
crate::ControlFrame
crate::world::ldtk_world
crate::presentation
crate::dev::debug_overlay
```

These should live in `AmbitionPortalIntegrationPlugin`, `PortalLdtkPlugin`, or `PortalRenderPlugin`.

## Spawn-site lifecycle tests

The room-scope leak showed that raw spawn calls can create persistent bugs.

Guardrails:

```text
- crates/ambition_sandbox/src/features/ecs/spawn*.rs should not call commands.spawn directly except through approved helpers.
- room-authored entities must have RoomScopedEntity or equivalent lifetime marker.
- dynamic room-local entities such as portal shots, dropped portal gun pickups, thrown items, and projectiles should declare lifetime explicitly.
```

Possible test approach:

```rust
#[test]
fn room_feature_spawns_use_lifecycle_helpers() {
    // scan spawn modules for raw "commands.spawn(" and allowlist known exceptions
}
```

## Plugin registration tests

Guardrail:

```text
New subsystem systems should be registered inside their subsystem plugin, not directly in app/plugins.rs.
```

A crude test can scan `app/plugins.rs` for known subsystem paths and require an allowlist entry.

## Schedule ordering guardrails

Guardrail:

```text
Cross-plugin ordering uses stable SystemSet types or messages.
Concrete `.after(module::system)` across plugin boundaries should be rare and justified.
```

## Feature-gate guardrails

Add checks that supported build personas compile.

```bash
cargo check -p ambition_sandbox --no-default-features --features headless
cargo check -p ambition_sandbox --features desktop_dev
```

After portal feature gate:

```bash
cargo check -p ambition_sandbox --no-default-features --features "headless portal"
cargo check -p ambition_sandbox --no-default-features --features "ldtk portal portal_ldtk"
```

## Generated inventory guardrails

Use `scripts/ecs_inventory.py` snapshots to detect broad architectural drift:

```text
registered systems in app/plugins.rs should trend down
raw spawn sites should trend down or become lifecycle-explicit
resource counts in app-level modules should trend down
portal-owned resources/types should move into portal plugin modules
Ambition content types should move into ambition_content
```

These numbers are signals, not hard pass/fail rules.

## Agent patch guardrails

For hotspot files such as `portal.rs`, `app/plugins.rs`, `world_flow.rs`, and large content modules:

```text
- Avoid stale full-file replacement.
- Prefer move-only patches when splitting files.
- Prefer one architectural seam per patch.
- Run targeted tests after each seam.
- Do not preserve old and new registration paths simultaneously unless explicitly planned.
```
