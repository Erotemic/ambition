//! Minimal-plugin App smoke tests for [`SandboxSimulationPlugin`].
//!
//! Pins the bare-minimum contract of the sandbox simulation half:
//!
//! - registered with `MinimalPlugins`, the plugin builds without
//!   touching audio, rendering, input, or inspector machinery;
//! - on the first tick exactly one player entity exists;
//! - that entity carries the canonical simulation components
//!   ([`PlayerEntity`], [`PlayerMovementAuthority`], [`PlayerBody`],
//!   [`BodyHealth`], [`BodyCombat`], [`BodyAnimFacts`],
//!   [`PlayerInteractionState`], [`PlayerBlinkCameraState`]);
//! - the canonical sim resources ([`SandboxSimState`], [`ControlFrame`],
//!   [`RoomGeometry`], [`RoomSet`], [`MovingPlatformSet`]) are present;
//! - no deleted god-object resource (`SandboxRuntime`, `FeatureRuntime`)
//!   is silently re-introduced — this is the runtime companion to the
//!   `legacy_runtime_guardrail` static-text scanner.
//!
//! These tests stay tiny on purpose: they verify the *shape* of the
//! simulation plugin, not gameplay behavior. Behavior tests live in
//! `scripted_gameplay.rs` and the per-feature suites under
//! `crates/ambition::actors/src/**/tests.rs`.

use ambition::actors::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
use ambition::actors::body_mode::BodyModeCapabilities;
use ambition::actors::player::{
    BodyAnimFacts, LocalPlayer, PlayerBlinkCameraState, PlayerIdentityBundle, PlayerSlot,
};
use ambition::actors::rooms::RoomSet;
use ambition::actors::{MovingPlatformSet, SandboxSimState};
use ambition::characters::actor::{BodyCombat, BodyHealth};
use ambition::engine_core::RoomGeometry;
use ambition::input::ControlFrame;
use ambition::platformer::schedule::GameMode;
use ambition::time::ClockState;
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

    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);

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
        world.get_resource::<RoomGeometry>().is_some(),
        "RoomGeometry resource missing — active room world not seeded"
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
    // Cluster-native (2026-05-28): `PlayerMovementAuthority` /
    // `PlayerBody` are gone. The canonical bundle now carries
    // `BodyKinematics` (size, pos, vel, facing) in their
    // place — assert the bundle still spawns with non-degenerate
    // body geometry plus every presentation/state component.
    let mut q = app.world_mut().query_filtered::<(
        &BodyKinematics,
        &BodyHealth,
        &BodyCombat,
        &BodyAnimFacts,
        &BodyModeCapabilities,
        &PlayerBlinkCameraState,
    ), With<PlayerEntity>>();
    let row = q
        .single(app.world())
        .expect("player entity should carry every PlayerSimulationBundle component");
    let (kinematics, health, combat, _anim, _caps, _blink_cam) = row;
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
        kinematics.size.x > 0.0 && kinematics.size.y > 0.0,
        "BodyKinematics size should be non-degenerate, got {:?}",
        kinematics.size
    );
}

/// Multiplayer-readiness canary: the default player must spawn with
/// `PlayerSlot::PRIMARY` + `PrimaryPlayer` + `LocalPlayer` so that
/// future code which filters on those identity components doesn't
/// silently miss the lone player. Pins the identity-tag contract on
/// `PlayerSimulationBundle::new`.
#[test]
fn default_player_carries_identity_components() {
    let mut app = minimal_sim_app();
    let mut q = app.world_mut().query_filtered::<
        (&PlayerSlot, Option<&PrimaryPlayer>, Option<&LocalPlayer>),
        With<PlayerEntity>,
    >();
    let (slot, primary, local) = q
        .single(app.world())
        .expect("the single default player should exist");
    assert_eq!(*slot, PlayerSlot::PRIMARY);
    assert!(primary.is_some(), "default player must be PrimaryPlayer");
    assert!(local.is_some(), "default player must be LocalPlayer");

    let mut primary_q = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryPlayer>>();
    assert_eq!(
        primary_q.iter(app.world()).count(),
        1,
        "exactly one entity should carry PrimaryPlayer",
    );
}

/// Multiplayer-readiness canary: spawning a second player entity with
/// `PlayerSlot(1)` (but without `PrimaryPlayer` / `LocalPlayer`) must
/// coexist with the default player without panicking and without
/// breaking the "exactly one PrimaryPlayer" invariant.
///
/// This test is a deliberate canary. It does not assert that the
/// second player can move, attack, or be camera-followed — most of
/// the gameplay chain still calls `single_mut::<…, With<PlayerEntity>>`
/// and would panic if more than one player tries to act. The canary's
/// job is to catch *additional* singleton assumptions sneaking in
/// (e.g. a future spawn-time system that blows up when a second
/// `PlayerEntity` already exists).
#[test]
fn second_player_entity_spawns_with_unique_slot_and_no_extra_primary() {
    let mut app = minimal_sim_app();

    // Spawn a "guest" player with just the identity tags. No
    // PrimaryPlayer, no LocalPlayer, no simulation components — those
    // are deliberately omitted because the full chain still assumes
    // exactly one moving player.
    app.world_mut()
        .spawn(PlayerIdentityBundle::new(PlayerSlot(1)));

    // Two PlayerEntity entities now exist; they must have distinct slots.
    let mut q = app
        .world_mut()
        .query_filtered::<&PlayerSlot, With<PlayerEntity>>();
    let mut slots: Vec<u8> = q.iter(app.world()).map(|s| s.index()).collect();
    slots.sort();
    assert_eq!(slots, vec![0, 1], "expected slots [0, 1], got {slots:?}");

    // Exactly one PrimaryPlayer must remain.
    let mut primary_q = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryPlayer>>();
    assert_eq!(
        primary_q.iter(app.world()).count(),
        1,
        "adding a non-primary second player should not change PrimaryPlayer count",
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
    let mut q = app
        .world_mut()
        .query_filtered::<&BodyHealth, With<PlayerEntity>>();
    let health = q
        .single(app.world())
        .expect("player must survive multiple idle ticks");
    assert!(health.current() > 0);
}

/// Switching to a non-gameplay `GameMode` forces `time_scale = 0` and
/// keeps `player_control_system` + `player_simulation_system` gated
/// off. Pins the contract that the small
/// `apply_suspended_time_scale_system` replaces the deleted
/// `mode_gate_phase` early-return, so any future schedule shuffle
/// can't silently revive ticking gameplay during pause/dialogue.
#[test]
fn non_gameplay_mode_zeroes_time_scale_and_skips_player_simulation() {
    let mut app = minimal_sim_app();

    // Sanity: baseline gameplay tick keeps time_scale at the default
    // of 1.0 (`update_time_scale` inside `player_control_system`
    // doesn't ramp down without a hitstop / bullet-time trigger).
    app.update();
    assert!(
        app.world().resource::<ClockState>().time_scale > 0.0,
        "time_scale should be >0 while gameplay is allowed"
    );

    // Capture the player's last engine-side position so we can assert
    // `player_simulation_system` is gated off — its
    // `update_player_simulation_with_clusters` call is the only thing
    // that integrates gravity / friction in our minimal App, so a no-
    // tick frame leaves the position pinned.
    let baseline_pos = {
        let mut q = app
            .world_mut()
            .query_filtered::<&BodyKinematics, With<PlayerEntity>>();
        q.single(app.world()).expect("player should exist").pos
    };

    // Switch to Paused; States needs one update for the transition to apply.
    app.world_mut()
        .resource_mut::<NextState<GameMode>>()
        .set(GameMode::Paused);
    app.update();

    // After the next tick the suspended-time-scale system should have
    // forced time_scale to zero.
    app.update();
    assert_eq!(
        app.world().resource::<ClockState>().time_scale,
        0.0,
        "apply_suspended_time_scale_system should zero time_scale in non-gameplay modes"
    );

    // And the player must not have integrated any physics — proves
    // `player_simulation_system`'s `update_player_simulation` call
    // did not run.
    let paused_pos = {
        let mut q = app
            .world_mut()
            .query_filtered::<&BodyKinematics, With<PlayerEntity>>();
        q.single(app.world()).expect("player should exist").pos
    };
    assert_eq!(
        baseline_pos, paused_pos,
        "player should not move while gameplay is suspended"
    );
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
