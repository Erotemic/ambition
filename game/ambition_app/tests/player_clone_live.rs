// Live, in-app proof that a brain-driven player CLONE moves through the same
// player movement systems as the human — the payoff of the universal-brain
// refactor. Only built with the RL stepping API.
#![cfg(feature = "rl_sim")]
//! Spawns a `PlayerClone` (a non-player body carrying the full player movement
//! clusters + a `PlayerDemo` brain) into the LIVE sandbox app via the
//! `SpawnPlayerCloneRequest` resource, steps the real schedule, and asserts the
//! clone runs, leaves the ground, and rises — driven entirely by its brain, with
//! no human input. Complements the engine-level proof in
//! `ambition_platformer2d::actors::avatar::clone_probe_tests`.

use crate::common::base;

use ambition_app::app::{PlayerClone, SpawnPlayerCloneRequest};
use ambition_platformer2d::engine_core::BodyGroundState;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::construction::SpawnOrigin;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::{Entity, Name, With, World};

/// (x, y, on_ground) of the single player clone, if it exists.
fn clone_state(world: &mut World) -> Option<(f32, f32, bool)> {
    let mut q = world.query_filtered::<(&BodyKinematics, &BodyGroundState), With<PlayerClone>>();
    q.iter(world)
        .next()
        .map(|(kin, ground)| (kin.pos.x, kin.pos.y, ground.on_ground))
}

/// The single player clone's entity and the `Name` it was built with.
///
/// ⚠ THE NAME IS CARRIED OUT, not just the entity, because the diagnostics below
/// have to say WHICH body is unnameable. An entity id alone sends the reader back
/// to re-derive the population by hand.
fn clone_entity(world: &mut World) -> Option<(Entity, String)> {
    let mut q = world.query_filtered::<(Entity, Option<&Name>), With<PlayerClone>>();
    q.iter(world).next().map(|(entity, name)| {
        (
            entity,
            name.map_or_else(|| "no Name either".to_string(), |name| name.to_string()),
        )
    })
}

/// **A brain-driven clone enters the simulation already identified.**
///
/// ⚠ A DIFFERENT SUBJECT FROM THE ARM ABOVE, WHICH IS WHY IT IS ITS OWN TEST.
/// That one asks whether the clone MOVES under its brain. This asks whether the
/// body the same road built can be NAMED — and the two fail independently.
/// Assertion order decides what a regression can reach, so an arm placed behind
/// `left_ground` would never be reached by a spawn-site defect.
///
/// ⛔⛔ THE CLONE IS A MECHANICAL BODY THE SWEEPER CANNOT NAME, AND NOTHING SAID
/// SO. `ensure_sim_id` mints from an authored `FeatureId` or from `PrimaryPlayer`;
/// `spawn_requested_player_clone` inserts NEITHER, and deliberately not
/// `PrimaryPlayer` (*"so `is_primary` gates the world-globals off for it"*). So
/// the clone lands on the `(None, None)` arm and is passed over in silence, on
/// every tick, forever.
///
/// ⚠ AND IT IS INVISIBLE TO BOTH SHIPPED CENSUSES, which is why this arm had to
/// be written rather than inherited. The sweeper's own `debug_assert` fires only
/// when the body carries `CenteredAabb` AND `ActorFaction` — `StrikeVictim`'s
/// pair — and `observe_damageable_body_identity` queries exactly that pair too.
/// The clone carries the `CenteredAabb` and **no `ActorFaction`**, so it slips
/// both. `UnmintedBodyCensus` is the one instrument whose population is
/// `BodyKinematics`, which is the population this body is actually in.
///
/// ⚠ THE EDIT THAT MAKES THIS FALSE: drop the identity mint from
/// `spawn_requested_player_clone`. The clone then reaches the sim unnameable and
/// this test fails naming the body and the road that built it.
#[test]
fn the_player_clone_road_builds_an_identified_body() {
    let mut sim = crate::common::fixed_60hz_sim();
    // Let the player settle, so the PRIMARY is identified before the clone is
    // built from it. `ensure_sim_id` runs `.before(CoreSimulation)` and the clone
    // spawns inside it (`WorldPrep`), so this is ordering the schedule already
    // guarantees — stepped here so the assertion is about identity rather than
    // about a half-booted room.
    sim.step_n(base(), 30);
    sim.world_mut().resource_mut::<SpawnPlayerCloneRequest>().0 = true;
    sim.step_n(base(), 50);

    // ⛔ ANTI-VACUITY FIRST. A run in which no clone was built reports no
    // unidentified bodies and reads exactly like a healthy one.
    let (clone, name) = clone_entity(sim.world_mut())
        .expect("the clone spawned; without one every assertion below is vacuous");

    let sim_id = sim
        .world()
        .get::<SimId>(clone)
        .map(|id| id.as_str().to_string());
    let origin = sim.world().get::<SpawnOrigin>(clone).cloned();
    let census = sim
        .world()
        .get_resource::<ambition_platformer2d::runtime::sim_identity::UnmintedBodyCensus>()
        .cloned()
        .expect("the runtime plugin installs `UnmintedBodyCensus`");
    println!("[clone] {clone} ({name}) sim_id={sim_id:?} origin={origin:?}");
    println!(
        "[clone] census: {} body-observations judged, {} skipped over {} distinct \
         bod(ies){}: {:?}",
        census.observed,
        census.skipped,
        census.skipped_bodies.len(),
        if census.capped { " (CAPPED — a floor)" } else { "" },
        census.skipped_bodies
    );

    // ⛔ THE INSTRUMENT'S OWN FLOOR, before any number it reports is read.
    assert!(
        census.observed > 0,
        "the sweeper census judged NO body, so its `skipped: {}` is a reading \
         about the observer rather than about the tree — check that \
         `observe_unminted_bodies` is still installed after the TAIL \
         `ensure_sim_id` pass",
        census.skipped
    );

    // ⭐ THE SPECIFIC CLAIM, BEFORE THE POPULATION ONE. A defect at this road's
    // spawn site must redden the arm that names this road, not a wider arm that
    // merely counts.
    assert!(
        sim_id.is_some(),
        "the player clone `{clone}` ({name}) reached the simulation with no \
         `SimId`. It carries `BodyKinematics`, is a `PlayerEntity`, and is \
         integrated by the same `player_simulation_system` the human player \
         runs — but `ensure_sim_id` mints only from an authored `FeatureId` or \
         from `PrimaryPlayer`, and `spawn_requested_player_clone` inserts \
         neither, so nothing can ever name it. ⚠ The repair is at the SPAWN \
         SITE, which already holds everything a deterministic identity needs: \
         it queries the PRIMARY body under `PrimaryPlayerOnly`, and the primary \
         carries both a `SimId` and the `SimIdCounter` that `SimId` requires."
    );

    // ⭐ AND ITS PROVENANCE, which is the half a `SimId` alone does not supply.
    // ADR 0030: a dynamic entity states the spawner it descends from or it is
    // unreconstructable — *"dynamic, parent unknown" is not a state worth being
    // able to spell*.
    assert!(
        matches!(origin, Some(SpawnOrigin::Dynamic { .. })),
        "the player clone `{clone}` ({name}) carries no `SpawnOrigin::Dynamic`, \
         so nothing records which body it descends from: origin={origin:?}. A \
         blob-rebuilt clone has no way back to its parent, and ADR 0030 makes \
         provenance a component precisely for that case."
    );

    // ⛔ THE POPULATION CLAIM LAST. It is the WIDER statement — every body this
    // fixture's roads built — and a foreign road regressing should not be able to
    // absorb the clone-specific failure above.
    // ⚠ THE COUNTER IS CUMULATIVE OBSERVATIONS, NOT A HEADCOUNT, and reading it
    // as a population sends the next reader hunting bodies that do not exist.
    // `observe_unminted_bodies` judges every body on every tick and adds to both
    // fields, so ONE unnameable body standing for fifty ticks reports ~50.
    // Measured before the repair: `209 judged, 49 skipped` for a single clone.
    assert_eq!(
        census.skipped, 0,
        "{} body-OBSERVATION(s) — not that many bodies — found a body carrying \
         `BodyKinematics` with no `SimId`, no `FeatureId` and no `PrimaryPlayer` \
         a tick after it became a body, so BOTH `ensure_sim_id` passes and every \
         in-tick spawner declined to name it. {} distinct bod(ies){}, each with \
         the CONSTRUCTION ROAD that let it through: {:?}. ⚠ This fixture travels \
         the player-clone road, which the sandbox census does not — if the origin \
         above is the clone's, the repair is at its spawn site.",
        census.skipped,
        census.skipped_bodies.len(),
        if census.capped { " (CAPPED — a floor)" } else { "" },
        census.skipped_bodies
    );
}

#[test]
fn brain_driven_player_clone_runs_and_leaves_the_ground_in_the_live_app() {
    let mut sim = crate::common::fixed_60hz_sim();
    // Let the player settle into the room.
    sim.step_n(base(), 30);

    // Ask the app to spawn a brain-driven clone next frame (the K-hotkey path,
    // poked directly so the test needs no synthetic key event).
    sim.world_mut().resource_mut::<SpawnPlayerCloneRequest>().0 = true;
    // Spawn, then let it fall + settle onto the floor before timing the demo.
    sim.step_n(base(), 50);

    let (start_x, start_y, _) = clone_state(sim.world_mut()).expect("the clone spawned");

    let mut min_y = start_y; // engine +y is DOWN, so smaller y == higher
    let mut max_x = start_x;
    let mut left_ground = false;
    // ~6s — several full Run -> Jump -> Dash -> Fly cycles.
    for _ in 0..(60 * 6) {
        sim.step_n(base(), 1);
        if let Some((x, y, on_ground)) = clone_state(sim.world_mut()) {
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            if !on_ground {
                left_ground = true;
            }
        }
    }

    assert!(
        left_ground,
        "the clone's brain (jump/fly) took it off the ground in-app",
    );
    assert!(
        min_y < start_y - 24.0,
        "the clone rose off the floor under brain control (min_y={min_y}, start_y={start_y})",
    );
    assert!(
        max_x > start_x + 60.0,
        "the clone ran horizontally under brain control (dx={})",
        max_x - start_x,
    );
}
