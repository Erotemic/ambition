//! The pirate's up-special: summon a burning flying shark and ride it.
//!
//! ```text
//! Smash authoring   an EffectRef on the up-special's timeline
//!        ↓
//! THIS MODULE       recognises the key, summons a mount, seats the summoner
//!        ↓
//! ambition_mount    the saddle pin, the lease, and leaving it
//! ```
//!
//! Most of this is existing machinery. ADR 0020 models a mount and rider as
//! two linked actors with the mount owning locomotion;
//! `npc_burning_flying_shark` carries `Mountable { class: "shark" }`; and
//! `steer_mount_from_rider` routes the rider's movement intent to the mount but
//! not its attack intent, so the rider can attack and use specials while
//! riding. This module adds the ruleset's half: which key summons what, how
//! long the ride lasts, what ends it, and where the shark goes afterwards.
//!
//! The genre rules live here, not in the engine: a jump or dodge means "put me
//! down", and a launch takes you off. `ambition_mount` owns the mechanism
//! (`DismountRequested`).

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::entity_catalog::smash_ride::{SummonRideParams, SUMMON_RIDE};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::mount::{
    DismountReason, DismountRequested, RideLease, RiderDismounted, RidingOn,
};

/// A mount on its way out: it holds this heading until the clock runs down and
/// then despawns.
///
/// It does not leave "the screen": the simulation must not read the camera,
/// because peers with different window sizes would desync. The sim reading of
/// "off screen" is the nearest blast direction, which the stage owns.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Departing {
    /// Seconds of sim time before the body is removed.
    pub remaining: f32,
    /// World velocity held for the whole departure.
    pub velocity: ae::Vec2,
}

/// How close the shark must be before the admiral is aboard.
///
/// The summon does not get the position it asks for: `translate_shark_summons`
/// names the admiral's centre, and `actor_spawn_center_for_collision` keeps the
/// authored bottom edge when the collision footprint differs. Measured at 62px
/// for this pair by
/// `an_admiral_picked_off_the_grid_can_ride_the_shark_it_summons`; this
/// radius leaves room.
///
/// A shark that misses the radius is not refused: it waits out its
/// reservation and leaves, and the admiral silently loses the up-B.
///
/// After D246 (the fly-in), this becomes the "you have arrived" radius.
const SUMMON_BOARD_RADIUS: f32 = 200.0;

/// How long a summoned shark waits for its summoner before it gives up.
const SUMMON_BOARD_DEADLINE_S: f32 = 1.0;

/// How much punishment the RECOVERY shark takes before it dies.
///
/// It must exceed the largest single hit in the game. Jon's rule: the shark
/// dies to about three hits (a damage threshold, in effect a health pool).
/// The authored shark has 6, fair in its own game; the summon overrides it.
///
/// The cast's largest hit is George Booul's forward smash: `damage: 21` at
/// `smash_charge_mult = 1.7`, 36 exactly. 40 clears it: about five typical
/// connections, or one charged George smash plus a little more.
/// `a_recovery_mount_cannot_be_deleted_by_one_hit` reads the whole selectable
/// cast, so a bigger smash trips it.
pub const SUMMON_SHARK_HEALTH: u32 = 40;

/// The summoned shark's actor id: one string for every shark this move makes.
///
/// It is also a save key: the death path persists a defeated actor as
/// `enemy_<id>_dead`. Summons decline persisted liveness at construction
/// (`RespawnPolicy::OnRoomReenter`), which keeps a shared id safe.
pub const SUMMON_SHARK_ID: &str = "smash_ride_shark";

/// How fast a dismissed shark leaves.
const DEPART_SPEED: f32 = 1_400.0;

/// How long it flies before it is removed. Generous: it only has to outlast
/// the time the body takes to leave view.
const DEPART_SECONDS: f32 = 2.0;

/// Recognise the authored summon-and-ride and ask for the mount.
///
/// It does not refuse a mounted caster: by now the move was accepted, the
/// recovery charge spent and the startup played. The "no recast from the
/// saddle" rule is `MoveGates::forbidden_while_held`, checked beside
/// `afford_recovery`. Once `call_the_shark` starts, its summon is owed.
pub fn translate_shark_summons(
    mut actions: MessageReader<ActorActionMessage>,
    mut effects: MessageWriter<ambition_platformer2d::vfx::EffectRequest>,
    bodies: Query<&ae::BodyKinematics>,
) {
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != SUMMON_RIDE {
            continue;
        }
        let params = match params.hydrate::<SummonRideParams>() {
            Ok(params) => params,
            Err(err) => {
                warn!("summon-ride params did not hydrate: {err}");
                continue;
            }
        };
        let Ok(kin) = bodies.get(message.actor) else {
            continue;
        };
        // The shark appears where the pirate is, so the saddle weld is a
        // no-op, not a teleport. A fly-in (a different arrival) was deferred
        // as a balance risk.
        bevy::log::info!(
            target: "ambition::mount",
            "summon requested: summoner={:?} character=`{}` at {:?}",
            message.actor,
            params.character_id,
            kin.pos,
        );
        effects.write(ambition_platformer2d::vfx::EffectRequest {
            owner: message.actor,
            effect: ambition_platformer2d::vfx::Effect::Summon(
                ambition_platformer2d::vfx::SummonSpec {
                    // The id suffix comes from the summoner's sequence counter,
                    // so two sharks from one pirate are two bodies.
                    id: SUMMON_SHARK_ID.to_string(),
                    name: "Burning Flying Shark".to_string(),
                    pos: kin.pos,
                    half_size: ae::Vec2::new(params.half_extents.0, params.half_extents.1),
                    character_id: params.character_id.clone(),
                    encounter_id: "smash".to_string(),
                    // Nobody's enemy: it takes no side. This alone does not make
                    // it harmless; see `keeps_contact_damage`.
                    faction: ambition_platformer2d::vfx::HitSide::Neutral,
                    // Neutral is hittable (`damage_lands` is true for
                    // `Foe | Neutral`), so an opponent can gimp the recovery,
                    // but it must survive a stray hit. See `SUMMON_SHARK_HEALTH`.
                    health: Some(SUMMON_SHARK_HEALTH),
                    // No contact damage in Smash. The authored shark carries
                    // `ContactDamage`, and `Neutral` does not suppress it; it
                    // was quiet only because a neutral body acquires no target.
                    keeps_contact_damage: false,
                    // The ride's length travels with the summon, so spawn, board
                    // and lease are one transaction in the executor's exclusive
                    // command. A lease installed here would orphan on a refused
                    // board.
                    ridden_by_summoner: Some(ambition_platformer2d::vfx::SummonedRide {
                        seconds: params.seconds,
                        // Generous because the shark appears underfoot: a body
                        // that drifted between summon and board still boards.
                        // D246's fly-in will shrink it to an arrival radius.
                        board_within: SUMMON_BOARD_RADIUS,
                        // Short, because the shark appears underfoot: a late
                        // board means something is wrong. D246's fly-in will
                        // need a longer approach budget.
                        board_deadline_s: SUMMON_BOARD_DEADLINE_S,
                    }),
                },
            ),
        });
    }
}

/// A rider that jumps or dodges gets off.
///
/// `steer_mount_from_rider` does not copy the rider's jump to the mount ("the
/// jump edge is the mount's own to decide"), so the press is free for this.
pub fn bail_out_of_the_saddle(
    riders: Query<
        (
            Entity,
            &ambition_platformer2d::characters::control::ActorControl,
        ),
        With<RidingOn>,
    >,
    mut dismounts: MessageWriter<DismountRequested>,
) {
    // Sorted: `Query` order is not stable, and riders bailing on one tick
    // must ask in a stable order.
    let mut bailing: Vec<Entity> = riders
        .iter()
        .filter(|(_, control)| control.0.jump_pressed || control.0.burst_pressed)
        .map(|(rider, _)| rider)
        .collect();
    bailing.sort();
    for rider in bailing {
        dismounts.write(DismountRequested {
            rider,
            reason: DismountReason::RiderBailed,
        });
    }
}

/// A rider hit hard enough to be launched comes off.
///
/// Tumble, not "was hit": a hit that flinches refreshes the up-B and leaves the
/// rider aboard; a hit that launches takes the rider off.
/// `BodyMotionFacts::tumbling` is the engine's word for "launched with no
/// control".
pub fn dismount_launched_riders(
    riders: Query<(Entity, &ae::BodyMotionFacts), With<RidingOn>>,
    mut dismounts: MessageWriter<DismountRequested>,
) {
    let mut launched: Vec<Entity> = riders
        .iter()
        .filter(|(_, facts)| facts.tumbling)
        .map(|(rider, _)| rider)
        .collect();
    launched.sort();
    for rider in launched {
        dismounts.write(DismountRequested {
            rider,
            reason: DismountReason::RiderLaunched,
        });
    }
}

/// A shark whose saddle emptied rides away, whatever emptied it.
///
/// One arm for all four reasons. An unridden mount otherwise stays as a live
/// NPC (ADR 0020), which is right for a field but not a fighter stage.
pub fn depart_when_riderless(
    mut commands: Commands,
    mut left: MessageReader<RiderDismounted>,
    departs: Query<&ae::BodyKinematics, With<ambition_platformer2d::mount::MountSlot>>,
) {
    for event in left.read() {
        let Ok(kin) = departs.get(event.mount) else {
            continue;
        };
        // Log the departure here, not in the refused-board arm.
        bevy::log::info!(
            target: "ambition::mount",
            "shark departing (rider left): mount={:?} reason={:?}",
            event.mount,
            event.reason,
        );
        commands.entity(event.mount).insert(Departing {
            remaining: DEPART_SECONDS,
            velocity: departure_heading(kin.pos) * DEPART_SPEED,
        });
    }
}

/// A rider that leaves play leaves the saddle.
///
/// Riding into the blast zone is allowed. The rider is not tumbling, so
/// `dismount_launched_riders` does not fire. Without this, the body goes
/// `OutOfPlay`, keeps `RidingOn` through its `DeathInterlude`, and
/// `sync_riders_to_mounts` snaps the respawned body back onto the shark.
///
/// Leased rides only, as in `dissolve_the_ride_when_the_shark_dies`. ADR 0020
/// keeps the link across a death on purpose, for authored pairs; a summoned
/// ride is disposable.
pub fn dismount_riders_who_left_play(
    riders: Query<
        Entity,
        (
            With<RidingOn>,
            With<RideLease>,
            With<ambition_platformer2d::combat::death_rules::OutOfPlay>,
        ),
    >,
    mut dismounts: MessageWriter<DismountRequested>,
) {
    // Self-limiting: the dismount removes `RidingOn` and `RideLease`, so this
    // does not re-ask on the next tick of the same interlude.
    let mut gone: Vec<Entity> = riders.iter().collect();
    gone.sort();
    for rider in gone {
        dismounts.write(DismountRequested {
            rider,
            reason: DismountReason::RiderLeftPlay,
        });
    }
}

/// A shark nobody got on leaves too.
///
/// Other departures hang off [`RiderDismounted`], which needs a ride that
/// started. When `mount::board` refuses the pair, the shark has no
/// `MountSlot`, so `depart_when_riderless` cannot see it.
///
/// D246's fly-in relies on this: a rider not in a mountable state on arrival
/// is refused by design, the shark flies off, and the player is gimped.
pub fn send_away_a_shark_nobody_boarded(
    mut commands: Commands,
    mut refused: MessageReader<ambition_platformer2d::mount::RideRefused>,
    // Not `With<MountSlot>`: a refused mount never got one.
    bodies: Query<&ae::BodyKinematics>,
) {
    for event in refused.read() {
        let Ok(kin) = bodies.get(event.mount) else {
            bevy::log::warn!(
                target: "ambition::mount",
                "refused shark has no body to send away: mount={:?}",
                event.mount,
            );
            continue;
        };
        bevy::log::info!(
            target: "ambition::mount",
            "shark departing (board refused): mount={:?}",
            event.mount,
        );
        commands.entity(event.mount).insert(Departing {
            remaining: DEPART_SECONDS,
            velocity: departure_heading(kin.pos) * DEPART_SPEED,
        });
    }
}

/// A summoned shark that DIES dissolves its rider's transient ride.
///
/// Otherwise killing a shark disables the up-B for the match. ADR 0020's
/// `enforce_mount_rider_link` keeps `RidingOn` when a mount dies, so an
/// authored pair can re-mount on respawn. A summoned shark never comes back,
/// and `MoveGates::forbidden_while_held` refuses up-B while riding.
///
/// A Smash bridge, not a change to ADR 0020: this ride is disposable. It is
/// the producer for `DismountReason::MountLost`.
pub fn dissolve_the_ride_when_the_shark_dies(
    mut died: MessageReader<ambition_platformer2d::platformer::body::MountDied>,
    leased: Query<Entity, With<RideLease>>,
    mut dismounts: MessageWriter<DismountRequested>,
) {
    // Only a leased ride dissolves; authored pairs carry no lease.
    let mut lost: Vec<Entity> = died
        .read()
        .filter_map(|event| leased.get(event.rider).ok())
        .collect();
    lost.sort();
    lost.dedup();
    for rider in lost {
        dismounts.write(DismountRequested {
            rider,
            reason: DismountReason::MountLost,
        });
    }
}

/// Which way is out: horizontal, toward the nearer side of the stage (the sim
/// reading of "the nearest off-screen position").
fn departure_heading(pos: ae::Vec2) -> ae::Vec2 {
    let centre_x = crate::stage_centre().x;
    // On the centre line, leave right: `signum(0.0)` is zero, and a shark
    // with no direction hovers until its clock runs out.
    let side = if pos.x < centre_x { -1.0 } else { 1.0 };
    ae::Vec2::new(side, 0.0)
}

/// Fly the departing sharks out and remove them.
pub fn tick_departures(
    mut commands: Commands,
    time: Res<ambition_platformer2d::time::WorldTime>,
    mut departing: Query<(
        Entity,
        &mut Departing,
        &mut ambition_platformer2d::characters::control::ActorControl,
    )>,
) {
    let dt = time.sim_dt();
    let mut gone: Vec<Entity> = Vec::new();
    for (entity, mut departure, mut control) in &mut departing {
        departure.remaining -= dt;
        // Write an intent, not a position: one integrator owns the position.
        // `velocity_target` is the world-space seam an aerial body steers by
        // (as `steer_mount_from_rider` uses). This runs in
        // `WorldPrepSet::BeforeIntegrate`, so this tick's pass reads it.
        control.0.velocity_target =
            ambition_platformer2d::engine_core::WorldVec2(departure.velocity);
        control.0.locomotion = Default::default();
        if departure.remaining <= 0.0 {
            gone.push(entity);
        }
    }
    // Sorted: `Query` order is not stable, and a despawn is a world edit.
    gone.sort();
    for entity in gone {
        commands.entity(entity).despawn();
    }
}

/// The projection a rollback localizer probes `Departing` through.
///
/// Both fields: `remaining` decides the tick the shark disappears and
/// `velocity` where it is then. A presence-only probe would see neither.
pub fn departing_probe(departing: &Departing) -> u64 {
    let mut hash = departing.remaining.to_bits() as u64;
    hash = hash.rotate_left(17) ^ departing.velocity.x.to_bits() as u64;
    hash.rotate_left(17) ^ departing.velocity.y.to_bits() as u64
}
