# Benchmark candidate: split oversized Bevy chained system tuples without losing ordering invariants

## Context

A Bevy game plugin registers most per-frame sandbox systems in one chained tuple so the frame pipeline is deterministic:

```rust
.add_systems(
    Update,
    (
        dialog::dialog_input,
        handle_ldtk_hot_reload,
        handle_debug_hotkeys,
        dev_tools::sync_developer_body_profile,
        crate::trace::handle_trace_hotkey,
        crate::map_menu::handle_map_menu_hotkeys,
        crate::rendering::spawn_dynamic_feature_visuals,
        sync_visuals,
        upgrade_enemy_sprites,
        upgrade_boss_sprites,
        animate_player,
        animate_characters,
        animate_bosses,
        camera_follow,
        debug_overlay::draw_debug_overlay,
        fx::update_particles,
        fx::update_impacts,
        fx::update_slash_previews,
        windowing::window_mode_hotkeys,
        update_hud,
        dialog::sync_dialog_ui,
    )
        .chain()
        .after(sandbox_update),
)
```

A refactor inserted one additional developer-sync system into the tuple. The next handoff failed to compile with `E0599`: the tuple still appeared to have a `chain` method, but the trait bounds were not satisfied for the very large tuple.

## Benchmark question

You are reviewing this Bevy scheduling change before handoff. What failure mode should you anticipate from a large chained tuple of systems, and how should you rewrite the schedule so it compiles while preserving the intended order?

Preserve these ordering invariants:

- input / hot-reload / developer hotkeys run after `sandbox_update`;
- dynamically spawned feature visuals are created before `sync_visuals` reads them;
- sprite upgrades and animation updates happen after visual spawning / sync;
- `camera_follow` runs after animation/sync work and before parallax systems that are already scheduled `.after(camera_follow)`;
- `sync_dialog_ui` remains late enough that dialog redirection scheduled `.before(dialog::sync_dialog_ui)` can still affect what is drawn.

## Expected answer

Do not keep adding systems to one giant tuple. Bevy implements system-config traits for tuples only up to a finite arity; once the tuple grows past that limit, `.chain()` fails with a long trait-bound error that looks unrelated to the new system.

Split the pipeline into multiple smaller chained groups and connect the groups with explicit `.after(...)` edges. For example:

```rust
.add_systems(
    Update,
    (
        dialog::dialog_input,
        handle_ldtk_hot_reload,
        handle_debug_hotkeys,
        dev_tools::sync_developer_body_profile,
        crate::trace::handle_trace_hotkey,
        crate::map_menu::handle_map_menu_hotkeys,
    )
        .chain()
        .after(sandbox_update),
)
.add_systems(
    Update,
    (
        crate::rendering::spawn_dynamic_feature_visuals,
        sync_visuals,
        upgrade_enemy_sprites,
        upgrade_boss_sprites,
        animate_player,
        animate_characters,
        animate_bosses,
    )
        .chain()
        .after(crate::map_menu::handle_map_menu_hotkeys),
)
.add_systems(
    Update,
    (
        camera_follow,
        debug_overlay::draw_debug_overlay,
        fx::update_particles,
        fx::update_impacts,
        fx::update_slash_previews,
        windowing::window_mode_hotkeys,
        update_hud,
        dialog::sync_dialog_ui,
    )
        .chain()
        .after(animate_bosses),
)
```

The exact grouping can vary, but each group should stay well below Bevy's tuple arity limit and the cross-group edges should encode the project-specific frame pipeline dependencies. Avoid replacing the chain with unsequenced tuples; that can compile while silently losing determinism.

## Validation

```bash
cargo fmt --all
cargo check -p ambition_sandbox --lib
```

## What this tests

- Recognizing Bevy's finite tuple implementation limit from an otherwise noisy Rust trait error.
- Preserving schedule semantics while changing code shape.
- Avoiding the tempting but wrong fix of dropping `.chain()` and thereby losing deterministic frame ordering.

## Tags

`bevy-schedule`, `rust-trait-bounds`, `tuple-arity`, `system-ordering`, `overlay-repair`
