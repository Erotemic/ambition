#![cfg(feature = "rl_sim")]
//! **WHAT ACTUALLY SURVIVES A ROLLBACK FOR A PROVOKED BODY** — through the real
//! GGRS machinery, with nothing called by hand.
//!
//! ⛔⛔ **`reconcile_autonomous_actors` HAS NO PRODUCTION CALL SITE** (ledger
//! D104, reported by GPT 5.6 and verified 2026-08-12). Every test of that
//! function calls it directly, which proves the function works and says nothing
//! about whether it runs. The only system in `AmbitionLoadWorldSet::Reconcile`
//! is `reconcile_brain_bindings`, and it filters on `binding.active_preset()?` —
//! `None` for `ProvokedDefault`, `ProvokedProfile` and `CharacterProfile`. So
//! every provoked and every character-first body is SKIPPED by the only
//! reconciler that runs.
//!
//! ⭐ **this file is the question that answers "so is that a bug?"** If the
//! registered codecs already restore the provoked state, the reconciler is
//! redundant — 506 code lines of D73's acceptance list — and wiring it in would
//! make things WORSE, because it rebuilds a full `fresh_health_pool` over a
//! damaged actor's restored HP.
//!
//! ⚠ **it drives a SYNC-TEST session at prediction distance 4**, so `SaveWorld`
//! and `LoadWorld` genuinely run every frame and every frame is resimulated. A
//! test that stepped a plain fixed-tick sim would exercise no rollback at all
//! and pass without touching the thing it names.
//!
//! ⚠ **the provoked state is folded into the BASELINE, not written mid-window.**
//! A direct world write inside a live prediction window is not reproduced during
//! resimulation, so it would be erased by the first rollback and the test would
//! be measuring its own fixture rather than the codecs.

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::characters::actor::character_catalog::{AutonomousSource, BrainBinding};
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::characters::brain::Brain;
use bevy::prelude::{Entity, World};

fn hall_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("hall_of_characters")
            // ⭐ prediction distance 4: the session predicts and then RESIMULATES,
            // so `LoadWorld` runs for real. `0` is what the shipped build uses and
            // saves nothing (`rollback_lifecycle_reset`'s cost probe measured it),
            // which would make this test vacuous.
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the GGRS sync-test harness builds in the Hall")
}

/// The lowest-entity body carrying a `BrainBinding`, so the choice is stable
/// across runs without depending on Bevy's query iteration order.
fn a_bound_body(world: &mut World) -> Entity {
    let mut q = world.query::<(Entity, &BrainBinding)>();
    let mut found: Vec<Entity> = q.iter(world).map(|(entity, _)| entity).collect();
    found.sort();
    *found
        .first()
        .expect("the Hall stages catalog NPCs, which carry brain bindings")
}

/// **A PROVOKED, DAMAGED BODY, in the rollback baseline.**
///
/// This is the state a live provocation leaves behind — the binding says
/// provoked, the mind is the provoked one, and the body has since been hurt —
/// folded in so resimulation reproduces it.
fn stage_provoked_and_wounded(sim: &mut Platformer2dSimHarness) -> (Entity, i32) {
    let world = sim.world_mut();
    let body = a_bound_body(world);

    world
        .get_mut::<BrainBinding>(body)
        .expect("the chosen body has a binding")
        .provoke();
    // A mind that is not the peaceful one, so "the brain survived" cannot be
    // satisfied by nothing having happened.
    *world.get_mut::<Brain>(body).expect("the body has a brain") = Brain::stand_still();

    let wounded = {
        let mut health = world
            .get_mut::<BodyHealth>(body)
            .expect("the body has health");
        let wounded = (health.health.max / 2).max(1);
        health.health.current = wounded;
        wounded
    };

    sim.rebase_rollback_history()
        .expect("the provoked, wounded body becomes the rollback baseline");
    (body, wounded)
}

/// **THE CODECS ALREADY RESTORE A PROVOKED BODY, AND THE MISSING RECONCILER IS
/// NOT A GAP.**
///
/// ⭐ every component the absent `reconcile_autonomous_actors` would rebuild is
/// registered rollback state: `Brain` (cursor), `BrainBinding`, `BodyHealth`,
/// `ActorSurfaceState`, `TemporaryControl` and `CombatCapabilities` (canonical),
/// and `ActorConfig`, `ActionSet`, `Mounted`, `MountSlot`, `RidingOn` (clone).
///
/// ⛔ **the HP assertion is the one that matters, and it is the one that would
/// BREAK if the reconciler were wired in as-is.** Its provoked reconstruction
/// calls `fresh_health_pool(max_health)`, which would replace this body's
/// restored half-HP with a full pool every single load — a damaged actor healing
/// itself on every rollback frame, which is precisely the class of divergence
/// `rollback_lifecycle_reset`'s campaign note recorded as "a mid-brawl enemy
/// full-heal".
#[test]
fn a_provoked_wounded_body_survives_the_real_rollback_window() {
    let mut sim = hall_sim();
    // Let the room finish staging before anything is folded into the baseline.
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let (body, wounded) = stage_provoked_and_wounded(&mut sim);

    // Enough frames that the session has predicted, rolled back and resimulated
    // many times over — every one of them running SaveWorld and LoadWorld.
    for _ in 0..180 {
        sim.step(AgentAction::default());
    }

    let world = sim.world_mut();
    assert_eq!(
        world.get::<BrainBinding>(body).map(|b| b.source.clone()),
        Some(AutonomousSource::ProvokedDefault),
        "the provoked SOURCE did not survive the rollback window — the binding \
         codec is canonical rollback state, so if this fails the codec is the \
         thing to look at, not a missing reconciler"
    );
    assert_eq!(
        world.get::<Brain>(body).map(|brain| brain.label()),
        Some("stand_still"),
        "the provoked MIND did not survive — `Brain` is a rollback cursor, and \
         `reconcile_brain_bindings` deliberately skips a source with no active \
         preset, so nothing else was going to put it back"
    );
    assert_eq!(
        world
            .get::<BodyHealth>(body)
            .map(|health| health.health.current),
        Some(wounded),
        "the body's DAMAGE did not survive. ⛔ if this ever fails because \
         `reconcile_autonomous_actors` was wired into LoadWorld, that is the \
         bug: its provoked reconstruction calls `fresh_health_pool(max_health)` \
         and would heal a damaged actor on every load"
    );
}
