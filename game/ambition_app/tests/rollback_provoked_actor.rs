#![cfg(feature = "rl_sim")]
//! WHAT ACTUALLY SURVIVES A ROLLBACK FOR A PROVOKED BODY — through the real
//! GGRS machinery, with nothing called by hand.
//!
//! Every test of that function calls it directly, which proves the function works and says
//! nothing about whether it runs. The only system in `AmbitionLoadWorldSet:Reconcile` is
//! `reconcile_brain_bindings`, and it filters on `binding.active_preset?` — `None` for
//! `ProvokedDefault`, `ProvokedProfile` and `CharacterProfile`. So every provoked and every
//! character-first body is SKIPPED by the only reconciler that runs.
//!
//! (The helper it used, `fresh_health_pool`, is itself gone as of: the LIVE provoke flip was
//! its last caller, and provocation no longer writes health either.)
//!
//! it drives a SYNC-TEST session at prediction distance 4, so `SaveWorld` and `LoadWorld`
//! genuinely run every frame and every frame is resimulated.
//!
//! the provoked state is folded into the BASELINE, not written mid-window.
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
            .with_required_start_room("hall_of_characters")
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the GGRS sync-test harness builds in the Hall")
}

/// DID A ROLLBACK ACTUALLY HAPPEN? — asked of the runtime, not assumed.
///
/// If the sync-test session had silently stopped rolling back — a changed harness default, a
/// feature flag, a prediction distance that quietly became 0 — every assertion would still have
/// passed, because state that is never disturbed always survives. A test whose subject can
/// vanish while it stays green is not evidence.
///
/// `RollbackExecutionStats` is a permanent, always-on counter incremented by
/// `count_load_run` inside `AmbitionLoadWorldSet::Reconcile` — the very set the
/// absent reconciler would have been installed in. There is no need to infer
/// rollback from timing or from a cost probe; the runtime counts it.
///
/// `lifetime_load_runs`, NOT `load_runs`, and the type's own doc is why: a
/// rebase installs a NEW session and zeroes the per-session counters. These
/// tests rebase, so the unprefixed field would report a number reset underneath
/// them — which is the exact misreading that made the exit oracle look like a
/// session that had stopped being driven at frame 12 when it had executed 2915
/// advances.
fn load_runs(sim: &mut Platformer2dSimHarness) -> u64 {
    sim.world_mut()
        .get_resource::<ambition_platformer2d::rollback::RollbackExecutionStats>()
        .map(|stats| stats.lifetime_load_runs)
        .expect("the GGRS session publishes execution stats")
}

/// Assert that the window just stepped genuinely restored state, and say how
/// many times. The NUMBER is the point: `> before` proves motion, and a healthy
/// sync-test window at distance 4 produces many loads, not one.
fn assert_rolled_back(sim: &mut Platformer2dSimHarness, before: u64, what: &str) {
    let after = load_runs(sim);
    assert!(
        after > before,
        "{what}: LoadWorld never ran ({before} → {after}). The session is not \
         rolling back, so nothing below this line is evidence about rollback — \
         check the harness's prediction distance, which is 0 in the shipped \
         build and saves nothing"
    );
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

/// A PROVOKED, DAMAGED BODY, in the rollback baseline.
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

/// THE CODECS ALREADY RESTORE A PROVOKED BODY, AND THE MISSING RECONCILER IS
/// NOT A GAP.
///
/// every component the absent `reconcile_autonomous_actors` would rebuild is
/// registered rollback state: `Brain` (cursor), `BrainBinding`, `BodyHealth`,
/// `ActorSurfaceState`, `TemporaryControl` and `CombatCapabilities` (canonical),
/// and `ActorConfig`, `ActionSet`, `Mounted`, `MountSlot`, `RidingOn` (clone).
///
/// the HP assertion is the one that matters, and it is the one that would
/// Provoked reconstruction must preserve restored current health rather than
/// rebuilding a fresh pool from `max_health` on rollback loads.
#[test]
fn a_provoked_wounded_body_survives_the_real_rollback_window() {
    let mut sim = hall_sim();
    // Let the room finish staging before anything is folded into the baseline.
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let (body, wounded) = stage_provoked_and_wounded(&mut sim);
    // read AFTER the rebase: the rebase installs a new session, and the
    // lifetime counter is the one that carries across it.
    let loads_before = load_runs(&mut sim);

    // Enough frames that the session has predicted, rolled back and resimulated
    // many times over — every one of them running SaveWorld and LoadWorld.
    for _ in 0..180 {
        sim.step(AgentAction::default());
    }
    assert_rolled_back(&mut sim, loads_before, "the provoked-body window");

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
         bug: its provoked reconstruction rebuilt a fresh pool from \
         `max_health` and would heal a damaged actor on every load"
    );
}

/// AND POSSESSION SURVIVES ONE TOO — the other half of the absent
/// reconciler, and the half that could have been a real production bug.
///
/// `reconcile_temporary_control` is the part of `reconcile_autonomous_actors`
/// that could NOT obviously be redundant: it rebuilds mount links from stable
/// ids and INSERTS a `MountSlot` that was absent at save, and a codec cannot
/// insert what the snapshot never held (`construction/mod.rs` cites it for
/// exactly that). If that mattered, possession and mounting across a rewind
/// would be broken in production today — the function does not run.
///
/// this possesses a body for real, through the same Down+Interact hold the possession
/// end-to-end suite uses, and then keeps simulating inside a live prediction window.
#[test]
fn possession_survives_the_real_rollback_window() {
    use ambition_platformer2d::actors::abilities::traversal::possession::PossessionState;
    use ambition_platformer2d::actors::features::FeatureId;
    use ambition_platformer2d::entity_catalog::placements::CharacterBrain;

    // copied from `possession_end_to_end`'s helper rather than reinvented:
    // `interact` is the EDGE and `interact_held` is the hold, and a version that
    // set only one of them would never commit a possession.
    fn down_interact(edge: bool) -> AgentAction {
        AgentAction {
            move_y: 1.0,
            interact: edge,
            interact_held: true,
            ..AgentAction::default()
        }
    }

    let mut sim = hall_sim();
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }

    let player = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d::engine_core::BodyKinematics,
            ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
        >();
        q.single(world).expect("primary player").pos
    };
    // This said only

    // `Custom("cellular_automaton_fighter")`, and that archetype row was

    // DELETED when the automaton became a character — so this fixture had

    // been quietly spawning a generic `combatant` and asserting on it.

    sim.spawn_enemy_character_at(
        "possess_target_rollback",
        "Perfect Cellular Automaton",
        (player.x + 60.0, player.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
        "perfect_cellular_automaton",
    );
    let target = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &FeatureId)>();
        q.iter(world)
            .find(|(_, id)| id.as_str() == "possess_target_rollback")
            .map(|(entity, _)| entity)
            .expect("the spawned body is present")
    };

    // Hold until the possession commits. The mechanic commits at the end of each
    // hold window and the target weaves around the radius, so several windows may
    // pass — the sim is deterministic, so a bounded hold is not a race.
    let mut possessed = None;
    for i in 0..900 {
        sim.step(down_interact(i == 0));
        possessed = sim.world_mut().resource::<PossessionState>().possessed;
        if possessed.is_some() {
            break;
        }
    }
    assert_eq!(
        possessed,
        Some(target),
        "setup: the body was never possessed, so this test would prove nothing \
         about possession surviving anything"
    );

    // Keep simulating INSIDE the prediction window: every one of these frames is
    // saved, predicted and resimulated.
    let loads_before = load_runs(&mut sim);
    for _ in 0..120 {
        sim.step(AgentAction::default());
    }
    assert_rolled_back(&mut sim, loads_before, "the possession window");

    assert_eq!(
        sim.world_mut().resource::<PossessionState>().possessed,
        Some(target),
        "possession did not survive the rollback window. ⛔ if this fails, the \
         absent `reconcile_temporary_control` was load-bearing after all and \
         possession across a rewind has been broken in production — which is a \
         bug to fix in the CODECS, not by installing a reconciler that also \
         heals damaged actors"
    );
    assert!(
        sim.world_mut()
            .get::<ambition_platformer2d::characters::control::DrivingParticipant>(target)
            .is_some(),
        "the possessed body stopped holding the primary seat across the window"
    );
}
