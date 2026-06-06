# Disk Layout

## Final workspace layout

The final disk layout should make dependency direction obvious.

```text
ambition/
  Cargo.toml
  Cargo.lock

  docs/
    adr/
    systems/
    planning/
    generated/

  tools/
    ecs_inventory.py
    ambition_ldtk_tools/
    refactor/
    validation/

  crates/
    ambition_math/
    ambition_data/
    ambition_schedule/
    ambition_messages/

    ambition_platformer_core/
    ambition_platformer_ecs/
    ambition_platformer_physics/
    ambition_platformer_rooms/
    ambition_platformer_input/
    ambition_platformer_projectile/

    ambition_mechanics_portal/
    ambition_mechanics_gravity/
    ambition_mechanics_held_items/
    ambition_mechanics_combat/
    ambition_mechanics_interaction/
    ambition_mechanics_encounter/
    ambition_mechanics_boss_runtime/
    ambition_mechanics_inventory/

    ambition_platformer_render/
    ambition_platformer_ldtk/
    ambition_portal_render/
    ambition_portal_ldtk/
    ambition_audio/
    ambition_dialogue/
    ambition_ui_core/
    ambition_devtools/

    ambition_content/
    ambition_sandbox/
    ambition_headless/
    ambition_tools/
```

## Proto-crate layout before real crate extraction

Before moving real crates, mirror the final topology inside `ambition_sandbox/src/`:

```text
crates/ambition_sandbox/src/
  platformer_runtime/
    mod.rs
    prelude.rs
    lifecycle/
    world_query/
    body/
    rooms/
    movement/
    projectiles/
    schedule/

  mechanics/
    portal/
      mod.rs
      plugin.rs
      schedule.rs
      config.rs
      gun/
      placement/
      transit/
      gate/
      lifecycle.rs

    gravity/
    held_items/
    combat/
    encounter/
    boss_runtime/

  ambition_content/
    plugin.rs
    worlds/
    items/
    quests/
    bosses/
    enemies/
    music/
    dialogue/
    portal/

  app/
  host/
  dev/
  presentation/
```

This proto-crate phase should enforce:

```text
platformer_runtime/ must not import mechanics/, ambition_content/, app/, dev/, or presentation/.
mechanics/ may import platformer_runtime/.
ambition_content/ may import platformer_runtime/ and mechanics/.
app/ may compose everything.
```

## Portal final disk layout

```text
crates/ambition_mechanics_portal/
  Cargo.toml
  src/
    lib.rs
    prelude.rs
    plugin.rs
    schedule.rs
    config.rs
    color.rs
    source.rs
    transform.rs

    gun/
      mod.rs
      components.rs
      messages.rs
      systems.rs
      pickup.rs

    placement/
      mod.rs
      surface.rs
      fit.rs
      projectile.rs
      failure.rs

    transit/
      mod.rs
      components.rs
      systems.rs
      math.rs
      messages.rs

    gate/
      mod.rs
      components.rs
      registry.rs
      systems.rs
      messages.rs

    lifecycle.rs

    tests/
      mod.rs
      placement.rs
      transit.rs
      gate.rs

crates/ambition_portal_render/
  Cargo.toml
  src/
    lib.rs
    plugin.rs
    sprites.rs
    body_pieces.rs
    shot_visuals.rs
    disorientation.rs
    debug_gizmos.rs

crates/ambition_portal_ldtk/
  Cargo.toml
  src/
    lib.rs
    plugin.rs
    entities.rs
    conversion.rs
    validation.rs
```

Ambition-specific portal glue:

```text
crates/ambition_content/src/portal/
  mod.rs
  plugin.rs
  input_adapter.rs
  inventory_adapter.rs
  ldtk_schema.rs
  save_adapter.rs
  debug_adapter.rs
```

## What remains in `ambition_sandbox`

Long term, `ambition_sandbox/src` should mostly be app composition and host shell:

```text
crates/ambition_sandbox/src/
  main.rs
  lib.rs

  app/
    plugin.rs
    cli.rs
    personas.rs

  host/
    desktop.rs
    web.rs
    android.rs

  dev/
    overlay.rs
    trace.rs
    profiling.rs

  save/
    settings.rs
    persistence.rs
```

It should not permanently own large reusable/gameplay systems like:

```text
portal.rs
item_pickup.rs
combat.rs
physics.rs
quest.rs
boss_encounter/
content/features/ecs/
engine_core/
world/ldtk_world/
presentation/rendering/
```

Those should move to runtime, mechanics, adapters, rendering, or content crates.
