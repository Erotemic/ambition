//! The combat rules a match plays under are RESOLVED by the shipped app, not
//! borrowed from it. (AE6)
//!
//! The inline tests beside `track_versus_roster` prove the fold and prove the
//! route declares rather than writes. They cannot prove the thing that actually
//! ships: that the SHIPPED COMPOSITION installs the projection, in a set that
//! runs before the readers.
//!
//! A reader wired correctly against a resource nothing publishes fails silently — the
//! `Option<Res<..>>` every combat reader carries for headless minimalism means "absent"
//! resolves to the engine default rather than panicking. So a match could declare its rules,
//! the declaration could be correct, the fold could be correct, and every unit test could pass,
//! while the live game played under the baseline forever. The question to ask is never "is the
//! reader right" but *"which plugin installs it"*.
//!
//! So this test composes `AmbitionGameSimulationPlugin` — the same plugin the game
//! boots — and asks the world.

use ambition_app::app::StartRoomOverride;
use ambition_platformer2d::combat::rules::{DeclaredCombatRules, ResolvedCombatTuning};
use bevy::app::{PluginGroup, ScheduleRunnerPlugin};
use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::transform::TransformPlugin;

/// A DI budget nobody ships, so "the world's baseline" cannot be confused with
/// "the engine default" in any assertion below.
const AUTHORED_BASELINE_DI: f32 = 0.12;
/// Different from the baseline AND from the default, so a resolved value can
/// only have come from the declaration.
const DECLARED_DI: f32 = 0.31;

fn composed_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(std::time::Duration::ZERO)));
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::sim::GameMode>();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    app.insert_resource(StartRoomOverride("portal_lab".to_string()));
    // K2b edit 2: the shell host, booted to gameplay. This added the
    // simulation plugin alone and inherited the `SessionRoot` it published at
    // plugin-build time; that publisher is gone, so the composition is the one
    // a player runs. `StartRoomOverride` survives it — it is consumed while the
    // prepared content is assembled, before any activation.
    ambition_app::app::shell_host::compose_ambition_gameplay_host(&mut app);
    app.finish();
    // one update is no longer enough: activation is asynchronous, behind a
    // load barrier and eight preparation work items.
    ambition_platformer2d::platformer::lifecycle::settle_until_session_world(
        &mut app,
        ambition_platformer2d::platformer::lifecycle::SESSION_SETTLE_FRAMES,
    )
    .unwrap_or_else(|budget| {
        panic!("the shell-composed fixture produced no session world in {budget} frames")
    });
    app
}

fn resolved(app: &App) -> ResolvedCombatTuning {
    *app.world().resource::<ResolvedCombatTuning>()
}

/// The shipped composition publishes the resolved rules at all.
///
/// Absent, every combat reader silently falls back to the engine default and
/// nothing anywhere reports it — which is what makes this the assertion worth
/// making first.
#[test]
fn the_shipped_composition_installs_the_resolution() {
    let app = composed_app();
    assert!(
        app.world().get_resource::<ResolvedCombatTuning>().is_some(),
        "no plugin in the shipped simulation publishes ResolvedCombatTuning, so \
         every combat reader is falling back to its Option<Res<..>> default and \
         a declared match rule reaches nothing"
    );
}

/// An undeclared world resolves to its OWN authored tuning, not to the
/// engine default. The distinction is the one AE3's restore kept getting wrong.
#[test]
fn an_undeclared_world_resolves_to_the_tuning_it_authored() {
    let mut app = composed_app();
    app.world_mut()
        .resource_mut::<ambition_platformer2d::combat::feel::Platformer2dFeelTuningMonolith>()
        .di_max_angle = AUTHORED_BASELINE_DI;
    app.world_mut()
        .resource_mut::<ambition_platformer2d::combat::targeting::FriendlyFire>()
        .enabled = true;
    app.update();

    assert_eq!(
        resolved(&app).di_max_angle,
        AUTHORED_BASELINE_DI,
        "the fold ignored the world's authored DI and substituted a default"
    );
    assert!(
        resolved(&app).friendly_fire,
        "the fold ignored the world's authored friendly-fire rule"
    );
}

/// A declaration reaches the resolution through the real schedule, and the
/// baseline it plays over is never written.
///
/// The second half is the property that replaces AE3's save/restore: there is
/// no borrow, so there is no restore that can be skipped by a crash, no window
/// in which another writer wins, and no way for "restore" to quietly become
/// "reset to the engine default" — which is what it had become.
#[test]
fn a_declaration_wins_and_the_world_it_plays_over_is_untouched() {
    let mut app = composed_app();
    app.world_mut()
        .resource_mut::<ambition_platformer2d::combat::feel::Platformer2dFeelTuningMonolith>()
        .di_max_angle = AUTHORED_BASELINE_DI;
    app.world_mut()
        .resource_mut::<ambition_platformer2d::combat::targeting::FriendlyFire>()
        .enabled = true;

    app.world_mut().insert_resource(DeclaredCombatRules {
        bark_chance: None,
        // The versus route drops a trumped body where it hung.
        ledge_trump_pop: None,
        ledge_occupancy: None,
        double_jump_cancel: None,
        edge_cancel_recovery: None,
        special_turn: None,
        special_turn_reverses_drift: None,
        // A declaration names its declarer, so a stage's giveback can ask
        // whether the live rules are its own before removing them.
        declared_by: "a_declaring_stage".to_string(),
        di_max_angle: DECLARED_DI,
        knockback_growth: 0.0,
        friendly_fire: false,
        clank_damage_window: 0.0,
        clank_rebound_speed: 0.0,
        sudden_death_damage: None,
        downward_hit: Default::default(),
        // ...nor the meteor window: this fixture is about DI and knockback
        // growth, and a spike it never throws needs no sentence.
        meteor_lock_time: 0.0,
        rage_per_damage: 0.0,
        rage_max_scale: 1.0,
        stale_step: 0.0,
        stale_floor: 1.0,
        // ...nor crouch cancel: this fixture is about DI and knockback growth.
        crouch_cancel_scale: 1.0,
        hit_repeat_window_scale: 1.0,
        // ...nor the grab clock: this fixture throws no grab, so it takes the
        // undeclared world's flat hold rather than inventing a rule.
        grab_hold_base_seconds: ambition_platformer2d::combat::rules::FLAT_GRAB_HOLD_SECONDS,
        grab_hold_per_damage: 0.0,
        grab_hold_max_seconds: ambition_platformer2d::combat::rules::FLAT_GRAB_HOLD_SECONDS,
        grab_mash_seconds: ambition_platformer2d::combat::rules::FLAT_GRAB_MASH_SECONDS,
        // this fixture is about DI and knockback growth, not the floor
    });
    app.update();

    assert_eq!(
        resolved(&app).di_max_angle,
        DECLARED_DI,
        "the declaration did not reach the resolution: either the projection is \
         not in the schedule, or it runs somewhere the declaration is not \
         visible yet"
    );
    assert!(!resolved(&app).friendly_fire);
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::combat::feel::Platformer2dFeelTuningMonolith>()
            .di_max_angle,
        AUTHORED_BASELINE_DI,
        "a declared match rule was written into the world's tuning — the borrow \
         AE6 removed, reintroduced"
    );
    assert!(
        app.world()
            .resource::<ambition_platformer2d::combat::targeting::FriendlyFire>()
            .enabled,
        "same for friendly fire: the baseline is not a match's to write"
    );

    // Dropping the declaration IS the exit. No restore step runs, and the world
    // is already what it always was.
    app.world_mut().remove_resource::<DeclaredCombatRules>();
    app.update();
    assert_eq!(
        resolved(&app).di_max_angle,
        AUTHORED_BASELINE_DI,
        "the match's DI outlived the declaration that asked for it"
    );
    assert!(resolved(&app).friendly_fire);
}
