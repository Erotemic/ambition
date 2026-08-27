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
//! ⭐⭐ ALMOST NONE OF THIS IS NEW MACHINERY, and that is the point. ADR 0020
//! already models a mount and its rider as two linked actors with the mount
//! owning locomotion, `npc_burning_flying_shark` is already authored content
//! carrying `Mountable { class: "shark" }`, and `steer_mount_from_rider` already
//! routes the rider's movement intent onto the mount while deliberately NOT
//! routing its attack intent — which is exactly *"they are allowed to use
//! attacks and specials while they are riding the shark"*. What this module adds
//! is the RULESET's half: which key summons what, how long the ride lasts, what
//! ends it, and where the shark goes afterwards.
//!
//! ⛔ THE GENRE STATEMENTS LIVE HERE, NOT IN THE ENGINE. That a jump or a dodge
//! means *put me down*, and that a launch takes you off, are things a platform
//! fighter believes; a game where you ride a horse through a field believes
//! neither. `ambition_mount` owns the mechanism (`DismountRequested`) and this
//! owns the opinions.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ActionRequest, SpecialActionSpec};
use ambition_platformer2d::characters::brain::ActorActionMessage;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::characters::smash_ride::{SummonRideParams, SUMMON_RIDE};
use ambition_platformer2d::mount::{
    DismountReason, DismountRequested, RideLease, RiderDismounted, RidingOn,
};

/// A mount on its way out: it holds this heading until the clock runs down and
/// then despawns.
///
/// ⛔⛔ IT DOES NOT LEAVE "THE SCREEN". Jon asked for the shark to ride away to
/// the nearest off-screen position, and the simulation may not read the camera —
/// a sim that framed its behaviour on what is currently visible would do
/// different things on two peers with different window sizes, which is a desync
/// with a very innocent-looking cause. The sim-side reading of "off screen" is
/// the nearest BLAST direction, which the stage owns and which is the same
/// answer in every case that matters.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Departing {
    /// Seconds of sim time before the body is removed.
    pub remaining: f32,
    /// World velocity held for the whole departure.
    pub velocity: ae::Vec2,
}

/// How close the shark must be before the admiral is aboard.
///
/// ⛔⛔ THE SUMMON DOES NOT GET THE POSITION IT ASKS FOR, and this number has to
/// cover the difference. `translate_shark_summons` names the admiral's own
/// centre, and construction then places the body through
/// `actor_spawn_center_for_collision`, which preserves the authored BOTTOM EDGE
/// when the sprite's collision footprint differs from the requested box. ⭐
/// MEASURED at 62px for this pair, through the real composition, by
/// `an_admiral_picked_off_the_grid_can_ride_the_shark_it_summons`.
///
/// ⚠ IT WAS 96, WHICH LEFT 34px OF MARGIN NOBODY HAD MEASURED. A shark that
/// misses this radius is not refused — it waits out its reservation and leaves,
/// so the admiral spends the up-B and gets nothing. That failure is silent by
/// construction, which is exactly the shape this repo keeps paying for, so the
/// allowance is set from the measurement with room rather than from taste.
///
/// ⭐ D246 REPLACES THE REASON FOR THIS NUMBER. Once the shark flies in, this
/// stops being "cover construction's placement" and becomes "you have arrived".
const SUMMON_BOARD_RADIUS: f32 = 200.0;

/// How long a summoned shark waits for its summoner before it gives up.
const SUMMON_BOARD_DEADLINE_S: f32 = 1.0;

/// How much punishment the RECOVERY shark takes before it dies.
///
/// ⛔⛔ THE AUTHORED SHARK HAS 6, AND THAT IS ONE CONNECTION HERE. Six is a fair
/// pool in the game the burning flying shark was written for; the admiral's own
/// move table runs 2–17, so nearly every clean hit deleted it — and the summon
/// places it exactly where its rider is, which mid-fight is exactly where the
/// hits are. Jon's log showed `boarded` followed by a death about twenty
/// milliseconds later on EVERY press, which is what a 6 HP body does when it
/// materialises inside a fight.
///
/// ⭐ JON'S NUMBER IS A COUNT, NOT A POOL: *"the rule for the shark is hitting it
/// 'enough', so some threshold on damage — which effectively is a healthpool"*,
/// and roughly three hits. Against a 2–17 table with a middle around 8, three
/// hits is ~24. ⚠ A BALANCE FIGURE, so it is one constant with its derivation
/// written down rather than a number defended on feel.
const SUMMON_SHARK_HEALTH: u32 = 24;

/// How fast a dismissed shark leaves.
const DEPART_SPEED: f32 = 1_400.0;

/// How long it flies before it is removed. Generous: the despawn is bookkeeping
/// and the flight is the read, so this only has to outlast the time the body
/// takes to clear anything a viewer can see.
const DEPART_SECONDS: f32 = 2.0;

/// Recognise the authored summon-and-ride and ask for the mount.
///
/// ⛔⛔ IT NO LONGER REFUSES A MOUNTED CASTER, BECAUSE IT IS TOO LATE HERE. It
/// used to: `if riders.get(actor).is_ok() { continue }`, which read as the rule
/// *"no recast from the saddle"* and was not one. By the time this runs the move
/// has been accepted, the recovery charge spent and the startup played, so all
/// the check achieved was that no shark appeared — an accept-then-veto, and a
/// mounted pirate who got flinched (which refunds the recovery) could press up-B
/// and lose the charge to nothing.
///
/// ⭐ THE RULE MOVED TO WHERE A MOVE IS ALLOWED TO BEGIN:
/// `MoveGates::forbidden_while_held`, asked beside `afford_recovery`. Once
/// `call_the_shark` starts, its authored summon is owed.
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
        // ⭐ THE SHARK APPEARS WHERE THE PIRATE IS, so the saddle weld is a
        // no-op rather than a teleport. Jon considered flying it in from
        // offscreen and deferred it as a balance risk, not as a rejected idea:
        // *"that might be too hard to balance… have it spawn in directly where
        // the pirate is."* A fly-in is a different mount ARRIVAL, not a
        // different mount.
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
                    // The identity suffix comes from the summoner's own
                    // sequence counter, so two sharks from one pirate are two
                    // bodies and not one id claimed twice.
                    id: "smash_ride_shark".to_string(),
                    name: "Burning Flying Shark".to_string(),
                    pos: kin.pos,
                    half_size: ae::Vec2::new(params.half_extents.0, params.half_extents.1),
                    character_id: params.character_id.clone(),
                    encounter_id: "smash".to_string(),
                    // Nobody's enemy: this shark deals no damage and takes no
                    // side in the match. Jon: *"No, the shark doesn't have
                    // contact damage in smash."*
                    faction: ambition_platformer2d::vfx::HitSide::Neutral,
                    // ⛔ NEUTRAL DOES NOT MEAN UNHITTABLE — `damage_lands` is
                    // true for `Foe | Neutral`, which is right: an opponent must
                    // be able to gimp the recovery. What it must not do is die
                    // to the first stray hit. See `SUMMON_SHARK_HEALTH`.
                    health: Some(SUMMON_SHARK_HEALTH),
                    // ⭐ THE RIDE'S LENGTH TRAVELS WITH THE SUMMON, so the
                    // spawn, the board and the lease are one transaction inside
                    // the executor's exclusive command. Installing the lease
                    // here would leave an orphan behind a refused board.
                    ridden_by_summoner: Some(ambition_platformer2d::vfx::SummonedRide {
                        seconds: params.seconds,
                        // ⭐ GENEROUS TODAY BECAUSE THE SHARK APPEARS UNDERFOOT,
                        // and it is the number D246 will shrink: once the shark
                        // is called from off-screen, this is the radius at which
                        // it has ARRIVED and the admiral either gets on or is
                        // gimped. Wide enough here that a body which drifted a
                        // few pixels in the tick between the summon and the
                        // board still boards.
                        board_within: SUMMON_BOARD_RADIUS,
                        // ⭐ SHORT, BECAUSE THE SHARK APPEARS UNDERFOOT. If the
                        // admiral is not aboard within this, something is wrong
                        // rather than slow, and the ruleset would rather hear
                        // about it than watch an unclaimed shark hover forever.
                        // D246's fly-in will want a longer one — it is the
                        // approach budget then, not an error budget.
                        board_deadline_s: SUMMON_BOARD_DEADLINE_S,
                    }),
                },
            ),
        });
    }
}

/// A rider that jumps or dodges gets off.
///
/// ⭐ THE PRESS IS FREE TO MEAN THIS. `steer_mount_from_rider` copies the
/// rider's locomotion, velocity target and facing onto the mount and
/// deliberately not its jump — *"the jump edge is the mount's own to decide"* —
/// so a bail-out reads a press nothing else is consuming.
pub fn bail_out_of_the_saddle(
    riders: Query<
        (Entity, &ambition_platformer2d::characters::control::ActorControl),
        With<RidingOn>,
    >,
    mut dismounts: MessageWriter<DismountRequested>,
) {
    // Collected and sorted: `Query` order is not guaranteed and two riders
    // bailing on one tick must ask in a stable order.
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
/// ⛔⛔ TUMBLE, NOT "WAS HIT", AND THE DIFFERENCE IS THE WHOLE RULE. Jon named
/// two thresholds and they are not the same one: a hit that FLINCHES refreshes
/// the up-B, and a hit that LAUNCHES takes you off the shark. A jab that
/// flinches a rider leaves it aboard. `BodyMotionFacts::tumbling` is the
/// engine's own word for *"launched with no control"*, so this is a named
/// condition rather than a knockback magnitude somebody would have to defend.
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
/// One arm for all four reasons on purpose: Jon asked for the shark to leave
/// *"when the rider gets off"*, and a shark that departed on a lease expiry but
/// loitered after a bail-out would be a live NPC shark swimming around a
/// platform-fighter stage — which is what ADR 0020 says an unridden mount does,
/// and is right for a mount somebody left in a field.
pub fn depart_when_riderless(
    mut commands: Commands,
    mut left: MessageReader<RiderDismounted>,
    departs: Query<&ae::BodyKinematics, With<ambition_platformer2d::mount::MountSlot>>,
) {
    for event in left.read() {
        let Ok(kin) = departs.get(event.mount) else {
            continue;
        };
        commands.entity(event.mount).insert(Departing {
            remaining: DEPART_SECONDS,
            velocity: departure_heading(kin.pos) * DEPART_SPEED,
        });
    }
}

/// A RIDER THAT LEAVES PLAY LEAVES THE SADDLE.
///
/// ⛔⛔ WITHOUT THIS, A STOCK LOST IN THE SADDLE OUTLIVES THE STOCK. Jon asked
/// for riding out of bounds to be possible — *"Yes you can ride out of bounds
/// and kill yourself"* — and that is the exact path nothing else covers:
/// `dismount_launched_riders` fires on `BodyMotionFacts::tumbling`, and a pirate
/// who STEERS into the blast zone is not tumbling, they are flying. The stock is
/// spent, the body goes `OutOfPlay` and waits out its `DeathInterlude`, and
/// `RidingOn` is still on it the whole time — so the corpse rides along, and
/// when the respawn places it, `sync_riders_to_mounts` snaps it straight back to
/// the shark. Found by a GPT review of this branch, and it is the more serious
/// of the two lifecycle holes because it costs a stock and then hands the ride
/// back.
///
/// ⭐ LEASED RIDES ONLY, AND THAT IS THE SAME LINE `dissolve_the_ride_when_the_shark_dies`
/// DRAWS. ADR 0020 keeps the link across a death ON PURPOSE — *"keeping the link
/// record lets the same-room reset path re-mount the rider once the mount is
/// alive again"* — which is right for an authored pair whose shark is still
/// standing there when you respawn beside it. A summoned, leased ride is
/// disposable, so for it the same event means the opposite thing. The rule is
/// not "a dead rider dismounts"; it is "a TRANSIENT ride does not survive its
/// rider leaving play".
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
    // Self-limiting: the dismount takes `RidingOn` and `RideLease` off the body,
    // so this cannot re-ask on the next tick of the same interlude.
    let mut gone: Vec<Entity> = riders.iter().collect();
    gone.sort();
    for rider in gone {
        dismounts.write(DismountRequested {
            rider,
            reason: DismountReason::RiderLeftPlay,
        });
    }
}

/// A shark NOBODY EVER GOT ON leaves too.
///
/// ⛔⛔ THIS IS THE ARM THAT WAS MISSING, AND JON FOUND IT BY PLAYING. Every
/// other departure hangs off [`RiderDismounted`], which presupposes a ride that
/// STARTED. When `mount::board` refuses the pair, the shark spawns and then
/// exists forever: it never received a `MountSlot`, so `depart_when_riderless`
/// — which filters on exactly that — cannot see it, and no other system in the
/// world has a reason to look at a summoned body with an empty saddle.
///
/// ⭐ IT IS ALSO THE MECHANISM THE FLY-IN NEEDS (D246). Once the shark is called
/// from off-screen and boards ON ARRIVAL, a rider who is not in a mountable
/// state at that moment is refused BY DESIGN — Jon: *"the special ends, the
/// shark flys off, and the player is gimped."* That is this system, reached
/// deliberately instead of by accident.
pub fn send_away_a_shark_nobody_boarded(
    mut commands: Commands,
    mut refused: MessageReader<ambition_platformer2d::mount::RideRefused>,
    // ⛔ NOT `With<MountSlot>`. A refused mount never got one — that is the
    // whole reason this system exists rather than another arm of the one below.
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
        bevy::log::info!(
            target: "ambition::mount",
            "shark departing (rider left): mount={:?}",
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
/// ⛔⛔ WITHOUT THIS, KILLING A SHARK PERMANENTLY DISABLES THE UP-B. ADR 0020's
/// `enforce_mount_rider_link` deliberately KEEPS `RidingOn` attached when a
/// mount dies — *"keeping the link record lets the same-room reset path re-mount
/// the rider once the mount is alive again"* — which is exactly right for an
/// AUTHORED pair whose shark respawns underneath its pirate. A summoned shark
/// never comes back, so the admiral would be left logically riding a corpse
/// forever, and `translate_shark_summons` refuses anybody already carrying
/// `RidingOn`. One dead shark, no more sharks, for the rest of the match.
///
/// ⭐ A SMASH BRIDGE RATHER THAN A CHANGE TO ADR 0020. The persistent-pair
/// behaviour is correct for its own customer; what differs is that THIS ride is
/// disposable, and the ruleset that made it disposable is the one that knows.
/// `DismountReason::MountLost` existed with no producer; this is it.
pub fn dissolve_the_ride_when_the_shark_dies(
    mut died: MessageReader<ambition_platformer2d::platformer::body::MountDied>,
    leased: Query<Entity, With<RideLease>>,
    mut dismounts: MessageWriter<DismountRequested>,
) {
    // Only a LEASED ride dissolves. An authored pair carries no lease, so this
    // cannot reach into the metroidvania's shark riders and take their link.
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

/// Which way is out. Horizontal, toward whichever side of the stage this body is
/// already nearer — the sim-side reading of *"the nearest off-screen position"*.
fn departure_heading(pos: ae::Vec2) -> ae::Vec2 {
    let centre_x = crate::stage_centre().x;
    // A shark exactly on the centre line leaves to the right rather than
    // standing still: `signum(0.0)` is zero, and a departure with no direction
    // is a shark that hovers where it was until its clock runs out.
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
        // ⛔⛔ IT WRITES AN INTENT, NOT A POSITION. The first version set
        // `kin.vel` AND advanced `kin.pos` itself, in `Settle` — after the
        // ordinary movement pass had already integrated this body. That is two
        // integrators owning one position: on every tick the shark's own brain
        // moved it, and then this moved it again.
        //
        // ⭐ `velocity_target` IS THE SEAM AND IT IS WORLD-SPACE. The shark is an
        // aerial body, and an aerial body steers by exactly this — the same
        // field `steer_mount_from_rider` uses to hand a rider's intent to its
        // mount, chosen there for the same frame-safety reason.
        //
        // ⚠ ONE TICK OF LATENCY, deliberately taken. This runs in `Settle`, so
        // the intent written now is integrated by the next tick's movement pass.
        // A departing shark holds still for one frame, which nobody can see, and
        // the alternative is a second position authority, which everybody
        // eventually can.
        control.0.velocity_target =
            ambition_platformer2d::engine_core::WorldVec2(departure.velocity);
        control.0.locomotion = Default::default();
        if departure.remaining <= 0.0 {
            gone.push(entity);
        }
    }
    // Sorted: `Query` order is not guaranteed and a despawn is a world edit.
    gone.sort();
    for entity in gone {
        commands.entity(entity).despawn();
    }
}

/// The projection a rollback localizer probes `Departing` through.
///
/// BOTH FIELDS, because both are positions: `remaining` decides which tick the
/// shark stops existing on and `velocity` decides where it is when that
/// happens. A presence-only probe would satisfy the coverage oracle while
/// seeing nothing of the value.
pub fn departing_probe(departing: &Departing) -> u64 {
    let mut hash = departing.remaining.to_bits() as u64;
    hash = hash.rotate_left(17) ^ departing.velocity.x.to_bits() as u64;
    hash.rotate_left(17) ^ departing.velocity.y.to_bits() as u64
}
