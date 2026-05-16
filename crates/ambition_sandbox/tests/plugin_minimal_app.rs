//! Minimal-plugin App smoke tests for [`SandboxSimulationPlugin`].
//!
//! Pins the bare-minimum contract of the sandbox simulation half:
//!
//! - registered with `MinimalPlugins`, the plugin builds without
//!   touching audio, rendering, input, or inspector machinery;
//! - on the first tick exactly one player entity exists;
//! - that entity carries the canonical simulation components
//!   ([`PlayerEntity`], [`PlayerMovementAuthority`], [`PlayerBody`],
//!   [`PlayerHealth`], [`PlayerCombatState`], [`PlayerAnimState`],
//!   [`PlayerInteractionState`], [`PlayerBlinkCameraState`]);
//! - the canonical sim resources ([`SandboxSimState`], [`ControlFrame`],
//!   [`GameWorld`], [`RoomSet`], [`MovingPlatformSet`]) are present;
//! - no deleted god-object resource (`SandboxRuntime`, `FeatureRuntime`)
//!   is silently re-introduced — this is the runtime companion to the
//!   `legacy_runtime_guardrail` static-text scanner.
//!
//! These tests stay tiny on purpose: they verify the *shape* of the
//! simulation plugin, not gameplay behavior. Behavior tests live in
//! `scripted_gameplay.rs` and the per-feature suites under
//! `crates/ambition_sandbox/src/**/tests.rs`.

use ambition_sandbox::input::ControlFrame;
use ambition_sandbox::player::{
    PlayerAnimState, PlayerBlinkCameraState, PlayerBody, PlayerCombatState, PlayerEntity,
    PlayerHealth, PlayerInteractionState, PlayerMovementAuthority,
};
use ambition_sandbox::rooms::RoomSet;
use ambition_sandbox::{GameMode, GameWorld, MovingPlatformSet, SandboxSimState};
use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;
use bevy::MinimalPlugins;

/// Build the minimal-plugin sandbox simulation App: MinimalPlugins + the
/// transitive sim dependencies (assets, images, transforms, state) +
/// the `SandboxSimulationPlugin`.
fn minimal_sim_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<GameMode>();

    ambition_sandbox::app::init_sandbox_resources(&mut app);
    ambition_sandbox::app::add_simulation_plugins(&mut app);

    // First Update runs Startup (player spawn, plugin resources, etc.).
    app.update();
    app
}

#[test]
fn sandbox_simulation_plugin_inserts_core_resources() {
    let app = minimal_sim_app();
    let world = app.world();

    assert!(
        world.get_resource::<SandboxSimState>().is_some(),
        "SandboxSimState resource missing after add_simulation_plugins"
    );
    assert!(
        world.get_resource::<ControlFrame>().is_some(),
        "ControlFrame resource missing — sim/presentation input seam broken"
    );
    assert!(
        world.get_resource::<GameWorld>().is_some(),
        "GameWorld resource missing — active room world not seeded"
    );
    assert!(
        world.get_resource::<RoomSet>().is_some(),
        "RoomSet resource missing — LDtk world not parsed into rooms"
    );
    assert!(
        world.get_resource::<MovingPlatformSet>().is_some(),
        "MovingPlatformSet resource missing — moving-platform sim disabled"
    );
}

#[test]
fn sandbox_simulation_plugin_spawns_exactly_one_player() {
    let mut app = minimal_sim_app();
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<PlayerEntity>>();
    let count = q.iter(app.world()).count();
    assert_eq!(
        count, 1,
        "expected exactly one PlayerEntity after startup, found {count}"
    );
}

#[test]
fn player_entity_carries_canonical_sim_components() {
    let mut app = minimal_sim_app();
    let mut q = app.world_mut().query_filtered::<
        (
            &PlayerMovementAuthority,
            &PlayerBody,
            &PlayerHealth,
            &PlayerCombatState,
            &PlayerAnimState,
            &PlayerInteractionState,
            &PlayerBlinkCameraState,
        ),
        With<PlayerEntity>,
    >();
    let row = q
        .single(app.world())
        .expect("player entity should carry every PlayerSimulationBundle component");
    let (_authority, body, health, combat, _anim, _interaction, _blink_cam) = row;
    assert!(
        health.current() > 0,
        "player should start at >0 HP, got {}",
        health.current()
    );
    assert!(
        !combat.attacking,
        "player should not be mid-attack on the first tick"
    );
    assert!(
        body.size.x > 0.0 && body.size.y > 0.0,
        "PlayerBody size should be non-degenerate, got {:?}",
        body.size
    );
}

#[test]
fn sandbox_simulation_plugin_advances_ticks_without_presentation() {
    let mut app = minimal_sim_app();
    // The first tick already ran inside minimal_sim_app; drive a few
    // more to catch panics that show up after the first frame (e.g.
    // a system that assumes a resource inserted by a presentation
    // plugin we didn't load).
    for _ in 0..5 {
        app.update();
    }
    // Player still exists and is alive.
    let mut q = app.world_mut().query_filtered::<&PlayerHealth, With<PlayerEntity>>();
    let health = q
        .single(app.world())
        .expect("player must survive multiple idle ticks");
    assert!(health.current() > 0);
}

/// Runtime companion to `legacy_runtime_guardrail`: assert the deleted
/// god-object resources are not re-inserted at startup. The static
/// scanner catches identifiers in code; this catches a resource that
/// might be inserted from a less-obvious code path (a macro, a feature
/// gate, etc.).
#[test]
fn legacy_god_object_resources_are_absent() {
    let app = minimal_sim_app();
    let mut violations: Vec<String> = Vec::new();
    for (info, _ptr) in app.world().iter_resources() {
        let name = format!("{}", info.name());
        let short = name.rsplit("::").next().unwrap_or(name.as_str());
        for forbidden in ["SandboxRuntime", "FeatureRuntime"] {
            if short == forbidden {
                violations.push(name.clone());
            }
        }
    }
    assert!(
        violations.is_empty(),
        "legacy god-object resource(s) re-introduced as Bevy resources: {violations:?}",
    );
}
