//! A hit staged by the OUTGOING population never lands on the player in the NEW
//! one — in the shipped composition, at the moment the rule actually acts.
//!
//! ⛔⛔ THE SYSTEM WAS COVERED, ITS INSTALLER WAS COVERED, AND THE COMPOSITION WAS
//! NOT. `ambition_damage`'s `a_lifecycle_boundary_voids_staged_player_hits` adds
//! `void_pending_player_hits_at_lifecycle_boundaries` to its OWN `App`, and
//! `the_lifecycle_guard_installer_registers_the_guard` calls the installer on its
//! own `App`. MEASURED (`dev/installer_call_coverage.json`): deleting
//! `install_staged_hit_lifecycle_guard(app, sim)` from `combat_schedule.rs` left
//! ALL 583 `app_it` tests green.
//!
//! ⛔⛔ AND MY FIRST FIXTURE FOR IT WAS MEASURING THE REPLAY'S HEALTH REFILL.
//! Pushing a staged hit from OUTSIDE the frame puts it in the FIFO before
//! `apply_player_hit_events`, which runs in `PlayerSimulation` — early. The drain
//! consumed it, the damage landed on the OLD body, and the rebuild then refilled
//! the player, so "no damage across the boundary" was true with the guard
//! UNINSTALLED. The poison passed. ⇒ **The window the rule acts in is between the
//! drain and `ResetProcessing`**, and a fixture that stages outside that window is
//! not testing the rule, whatever it asserts.
//!
//! ⭐ SO THE HIT IS STAGED FROM INSIDE THE FRAME, after
//! `PlayerHitResolutionSet` — the same place the real
//! `stage_player_victim_hit_events` sits. Then the boundary frame's void is what
//! stands between that hit and the rebuilt body, and its absence is a hit landing
//! on a body the player just got back.
//!
//! ⚠ THREE ARMS, because two cannot tell the guard from the refill: a hit with no
//! boundary must HURT (or the fixture proves nothing), and a boundary with no hit
//! gives the health a replay alone leaves behind, which is what the guarded arm
//! has to match.

use crate::common::{base, fixed_60hz_sim};

use ambition_app::Platformer2dSimHarness;
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::combat::events::{
    HitEvent, HitMode, HitSource, HitTarget, PendingPlayerHitEvents, StagedPlayerHit,
};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

/// How much the staged hit takes off. Big enough that a partial refill still shows.
const DAMAGE: i32 = 2;

/// The body to stage a hit onto on the NEXT run of [`stage_a_hit_from_inside_the_frame`],
/// taken once so the fixture stages exactly one.
#[derive(Resource, Default)]
struct StageAHitOn(Option<Entity>);

/// Stage one victim hit at the moment the real stager does.
///
/// ⚠ ORDERED AFTER `PlayerHitResolutionSet` ON PURPOSE. Staged before it, the hit
/// is drained the same frame and never meets a boundary; staged after
/// `ResetProcessing`, it belongs to the incoming population and the rule does not
/// apply to it. Between the two is the only window this guard exists for.
fn stage_a_hit_from_inside_the_frame(
    mut cue: ResMut<StageAHitOn>,
    mut pending: ResMut<PendingPlayerHitEvents>,
) {
    let Some(body) = cue.0.take() else {
        return;
    };
    pending.0.push(StagedPlayerHit {
        event: HitEvent {
            strike_sfx: None,
            volume: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)).into(),
            damage: DAMAGE,
            source: HitSource::Melee,
            attacker: None,
            target: HitTarget::Body(body),
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        },
        attacker_id: None,
        victim_id: None,
    });
}

fn the_player(sim: &mut Platformer2dSimHarness) -> Entity {
    let mut query = sim
        .world_mut()
        .query_filtered::<Entity, With<PrimaryPlayer>>();
    let world = sim.world();
    query
        .iter(world)
        .next()
        .expect("the hosted app has a primary player")
}

fn health(sim: &Platformer2dSimHarness, body: Entity) -> i32 {
    sim.world()
        .get::<BodyHealth>(body)
        .expect("the player carries a health pool")
        .health
        .current
}

/// Run one arm and report the player's health once everything has settled.
fn arm(stage_a_hit: bool, cross_a_boundary: bool) -> i32 {
    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);

    let sim_schedule =
        ambition_platformer2d::platformer::schedule::SimScheduleExt::sim_schedule(sim.app_mut());
    sim.app_mut().init_resource::<StageAHitOn>().add_systems(
        sim_schedule,
        stage_a_hit_from_inside_the_frame
            .after(ambition_platformer2d::damage::PlayerHitResolutionSet),
    );

    let player = the_player(&mut sim);
    if stage_a_hit {
        sim.world_mut().resource_mut::<StageAHitOn>().0 = Some(player);
    }
    if cross_a_boundary {
        sim.world_mut().write_message(
            ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual(),
        );
    }
    // Two frames: the boundary frame stages and (should) void; the next frame is
    // where an unvoided hit would be drained onto the rebuilt body.
    sim.step_n(base(), 2);
    // …then let the rebuild finish, so a refill that was going to happen has.
    sim.step_n(base(), 60);

    let player = the_player(&mut sim);
    health(&sim, player)
}

#[test]
fn a_room_boundary_voids_a_hit_staged_by_the_outgoing_population() {
    let replay_only = arm(false, true);
    let hit_only = arm(true, false);
    let hit_across_the_boundary = arm(true, true);

    // ⚠ NON-VACUITY FIRST. A staged hit that cannot hurt makes "it did not hurt"
    // true for free, and that is exactly the shape my first attempt had.
    assert!(
        hit_only < replay_only,
        "the staged hit did not hurt the player at all (hit-only {hit_only}, \
         replay-only {replay_only}), so this fixture cannot tell a VOIDED hit \
         from one that was never able to land"
    );

    assert_eq!(
        hit_across_the_boundary, replay_only,
        "a hit staged by the OUTGOING population landed on the rebuilt body: \
         health across the boundary is {hit_across_the_boundary}, a replay with \
         no hit leaves {replay_only}, and the same hit with no boundary leaves \
         {hit_only}. `install_staged_hit_lifecycle_guard` is what voids it; \
         deleting its call from `combat_schedule.rs` leaves every other app test \
         green."
    );
}
