//! ECS systems for capture acquisition, hold, pummels, throws, and release.
//!
//! Capture acquisition requires the body state needed by the full relationship so
//! a capture cannot be established that later lifecycle stages cannot process.
//! Shields do not block grabs. Decisions are made read-only and applied afterward
//! in deterministic order.

use bevy::math::bounding::IntersectsVolume as _;
use bevy::prelude::*;

use super::{captive_of, CaptureAttemptRequested, CapturedBy};
use crate::hitbox::StrikeVictim;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt as _;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// What a body must BE to take part in a capture, at either end of it.
///
/// acquisition asks for what the whole LIFECYCLE needs, and this is that list. A capture is not
/// one moment. It is a hold, then pummels, then a throw or a release, and each beat reaches for
/// different body state. Stating the requirement once, and asking it at the one moment where
/// refusing is still free, is what makes an established capture mean "every beat of this
/// relationship can run".
///
/// ```text
/// BodyKinematics      the hold anchor; a throw's impact point
/// BodyGroundState     v1 eligibility (standing grabs), suspended by the hold
/// BodyHealth          a pummel's damage; a throw's damage and percent scaling
/// BodyCombat          the interruption rule at BOTH ends; a throw's hitstun
/// BodyFlightState     the throw's launch reaction
/// ActorSurfaceState   gravity suspended at acquisition, restored at release
/// ```
///
///  it is deliberately not `CenteredAabb`. The coarse box is victim-side
/// geometry, already required by [`StrikeVictim`] where the overlap is asked; a
/// captor needs none. This type is the BODY ROLE the relationship operates on,
/// not everything either end happens to carry.
#[derive(bevy::ecs::query::QueryData)]
pub struct CaptureParticipant {
    pub kin: &'static ae::BodyKinematics,
    pub ground: &'static ambition_platformer2d_core::BodyGroundState,
    pub health: &'static ambition_characters::actor::BodyHealth,
    /// The world's hands are off this body. NOT redundant with `health`: a
    /// fighter waiting out its death beat has already had `health.reset()`
    /// called on it, so it reads ALIVE for the whole interlude.
    pub out_of_play: bevy::prelude::Has<crate::death_rules::OutOfPlay>,
    pub combat: &'static ambition_characters::actor::BodyCombat,
    pub flight: &'static ae::BodyFlightState,
    pub surface: &'static ae::ActorSurfaceState,
}

/// Everything a body needs to be ALLOWED to start a capture: the shared
/// [`CaptureParticipant`] role, plus the allegiance facts only the captor's side
/// of the hostility question reads.
#[derive(bevy::ecs::query::QueryData)]
pub struct CaptorView {
    body: CaptureParticipant,
    faction: &'static crate::components::ActorFaction,
    team: Option<&'static crate::targeting::MatchTeam>,
    frame: Option<&'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
}

/// Acquire a captive for each live grab attempt that reaches an eligible body.
///
/// Captor and victim must both be grounded, alive, uncaptured
/// [`CaptureParticipant`] bodies under autonomous control, and the ordinary
/// `damage_lands_between` relation must permit the interaction. Overlapping
/// candidates are ordered by distance to the grab volume, then stable [`SimId`],
/// so arbitration is deterministic rather than query-order dependent.
pub fn acquire_captures(
    mut commands: Commands,
    mut attempts: MessageReader<CaptureAttemptRequested>,
    captors: Query<CaptorView, Without<ambition_characters::control::ScriptedControl>>,
    victims: Query<StrikeVictim, Without<ambition_characters::control::ScriptedControl>>,
    captives: Query<(Entity, &CapturedBy)>,
    //  the ONE eligibility question, asked of the victim too. Not a
    // convenience join: a body the rest of the lifecycle could not operate on is
    // refused here rather than captured and then stranded.
    participants: Query<CaptureParticipant>,
    identities: Query<&SimId>,
    // The captive's in-flight move, so capture can END it. See the note at
    // the insertion below for why this is not optional.
    mut playbacks: Query<&mut crate::moveset::MovePlayback>,
    // ⭐ THE CAPTIVE'S AIR RECOVERY, restored BY THE CATCH.
    //
    // Being grabbed re-seats a body: the hold suspends its gravity and pins it
    // to the captor, and what it is released into is a fresh airtime. The genre
    // gives the double jump and the air dodge back for that, and the CAUSE is
    // the capture — not the throw, and not "a throw is a hit so the hit rule
    // covers it". A captive that escapes a hold instead of being thrown was
    // grabbed just the same and gets the same recovery.
    mut budgets: Query<(
        &ae::BodyAbilities,
        &mut ae::BodyJumpState,
        &mut ae::BodyDodgeState,
        &ae::MotionModel,
    )>,
    tuning: Option<Res<crate::rules::ResolvedCombatTuning>>,
) {
    //  the WHOLE resolved row, not just the friendly-fire flag: a hold's
    // deadline is a declared rule too, and it is decided at acquisition.
    let rules = tuning.map(|t| *t).unwrap_or_default();
    let friendly_fire = rules.friendly_fire();
    // ⛔⛔ RESOLVED FIRST, GRANTED SECOND, because `Commands` are DEFERRED and
    // the `captives` query below cannot see what this pass is about to insert.
    //
    // ⭐ THE INVARIANT THIS FUNCTION CLAIMS — "one captor, one captive" — held
    // for every tick except the one it mattered on. Two grabs that connect on
    // the SAME tick are two messages in one pass: neither body is in `captives`
    // yet when the other's attempt is judged, so BOTH succeeded and each fighter
    // became captor AND captive of the other.
    //
    // ⛔ and it DEADLOCKED, which is why it cost a third of a match rather than
    // looking odd: the brain's capture policy answers `captured` FIRST and
    // returns, so a body that is both can only ever struggle. Neither side could
    // pummel, neither could throw, both were control-held, and the lock ended
    // only when the hold clock ran out. Measured 2026-08-23 in the full app,
    // `npc_pirate_admiral` rung 9 mirror: 40 captures, 2028 capture-ticks (28% of
    // the match), ZERO pummels and ZERO throws.
    //
    // ⚠ a MIRROR match makes it CERTAIN rather than rare: two identical brains
    // reach for a grab on the same tick, every time.
    // PHASE ONE: who would each attempt catch? Resolved against the state as
    // this pass found it, so the answer does not depend on message order.
    let mut resolved: Vec<(CaptureAttemptRequested, Entity)> = Vec::new();
    for attempt in attempts.read() {
        let Ok(captor) = captors.get(attempt.captor) else {
            continue;
        };
        if crate::util::body_is_untouchable(Some(captor.body.health), captor.body.out_of_play) {
            continue;
        }
        if !captor.body.ground.on_ground {
            continue;
        }
        //  a captor already holding somebody is not a captor twice. This is
        // where the "one captor, one captive" half of the invariant is upheld;
        // the other half is upheld by skipping a victim that is already held.
        if captive_of(attempt.captor, &captives).is_some() {
            continue;
        }

        let body_frame = captor
            .frame
            .map(|f| f.basis())
            .unwrap_or(ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR));
        let placed = crate::moveset::place_body_local_volume(
            &ambition_entity_catalog::VolumeShape::Rect {
                offset: (attempt.offset.x, attempt.offset.y),
                half_extents: (attempt.half_extents.x, attempt.half_extents.y),
            },
            captor.body.kin.facing,
            &body_frame,
        );
        let reach_centre = captor.body.kin.pos + placed.world_offset;
        let reach = ae::CenteredAabb::new(reach_centre, placed.half_extent).aabb();

        //  built ONCE, not asked per candidate: `captives` is the authority on
        // who is already held, and the other half of "one captive, one captor".
        let already_held: std::collections::HashSet<Entity> =
            captives.iter().map(|(entity, _)| entity).collect();

        // Gather, then rank.  never "take the first overlap": see the doc.
        let mut candidates: Vec<(f32, &SimId, Entity)> = Vec::new();
        for victim in &victims {
            if victim.entity == attempt.captor {
                continue;
            }
            if victim.is_corpse() || victim.is_intangible() {
                continue;
            }
            //  the same role the captor had to satisfy. A body with no health,
            // no combat state or no surface can be reached by a grab volume and
            // still cannot be pummelled, thrown or released — so it is not a
            // victim, it is a bystander.
            let Ok(victim_body) = participants.get(victim.entity) else {
                continue;
            };
            if !victim_body.ground.on_ground {
                continue;
            }
            if already_held.contains(&victim.entity) {
                continue;
            }
            if !crate::targeting::damage_lands_between(
                *captor.faction,
                victim.effective_faction(),
                captor.team,
                victim.team,
                friendly_fire,
                None,
                victim.entity,
            ) {
                continue;
            }
            // The published silhouette when there is one, the coarse box when
            // there is not — the same fallback every body-overlap road uses.
            let body = victim.aabb.aabb();
            if !reach.intersects(&body) {
                continue;
            }
            //  a body with no `SimId` cannot participate in the deterministic
            // tie-break, so it cannot be captured. That is a fixture shape, not
            // a live one: every constructed body has an identity.
            let Ok(id) = identities.get(victim.entity) else {
                continue;
            };
            candidates.push((
                body.center().distance_squared(reach_centre),
                id,
                victim.entity,
            ));
        }
        candidates.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.cmp(b.1))
        });
        let Some((_, _, victim)) = candidates.first().copied() else {
            continue;
        };
        resolved.push((*attempt, victim));
    }

    // PHASE TWO: ⭐⭐ THE ACCEPTED CAPTURES ARE A MATCHING, and that one word is
    // the whole rule: NO BODY MAY BE BOTH A CAPTOR AND A CAPTIVE on the same
    // tick, whatever shape the attempts make.
    //
    // ⛔⛔ THIS WAS A SPECIAL CASE FOR MUTUAL PAIRS AND THAT WAS NOT ENOUGH.
    // `A↔B` was arbitrated by `SimId`, which fixed the measured mirror deadlock
    // — but `B→C` and `A→B` on one tick left B captive of A AND captor of C,
    // recreating exactly the contradictory state the pair rule existed to
    // prevent. Worse, it depended on MESSAGE ORDER: `A→B` then `B→C` granted one
    // edge, the reverse granted both. Found by a GPT review reading the accept
    // condition, which rejected `taken(victim)` but never `holding(victim)`.
    //
    // ⇒ ONE loop, ONE set, ONE question: accept an edge iff NEITHER endpoint has
    // already participated in an accepted one. `A↔B`, `A→B→C`, `A→C←B` and the
    // three-cycle stop being four cases and become instances of the invariant.
    // The mutual-pair arbitration is DELETED rather than kept beside this: it is
    // the two-body instance of the same greedy pass.
    //
    // ⭐ AND THE ORDER IS THE PRIORITY, NOT THE MAILBOX. `resolved` is sorted by
    // the captor's `SimId` and then the victim's, so the outcome is a property of
    // the world rather than of which message the reader happened to visit first,
    // and it is stable across a rollback. A body with no `SimId` sorts LAST and
    // therefore loses a contest rather than winning one by absence — the same
    // reading the candidate ranking above takes of a missing identity.
    //
    // ⚠ ANY winner is a per-seat asymmetry on a symmetric stage, and there is no
    // symmetric alternative — a mirror is a FIXED POINT, so identical inputs
    // produce identical states however the tie is resolved, and the only
    // resolution that preserves the reflection is the one that never grants a
    // grab. Granting NEITHER was tried and MEASURED: 126 attempts over a minute
    // produced ZERO captures, zero pummels, zero throws. One winner it is, which
    // is also the genre's answer (Ultimate resolves a same-frame grab by port).
    // `two_emmys_hold_a_mirror_far_longer_than_two_ordinary_fighters` measures
    // that reflection and had to learn this rule exists.
    resolved.sort_by(|(a_attempt, a_victim), (b_attempt, b_victim)| {
        let key = |captor: Entity, victim: Entity| {
            (
                identities.get(captor).ok().cloned(),
                identities.get(victim).ok().cloned(),
            )
        };
        let (a_captor_id, a_victim_id) = key(a_attempt.captor, *a_victim);
        let (b_captor_id, b_victim_id) = key(b_attempt.captor, *b_victim);
        // `None` sorts last: no identity, no standing in a contest.
        match (a_captor_id, b_captor_id) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
        .then_with(|| match (a_victim_id, b_victim_id) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        })
    });
    // ONE set. A body that appears in an accepted edge — in EITHER role — is
    // spent for this tick.
    let mut spent: std::collections::HashSet<Entity> = Default::default();
    for (attempt, victim) in &resolved {
        let (attempt, victim) = (*attempt, *victim);
        if spent.contains(&attempt.captor) || spent.contains(&victim) {
            continue;
        }
        spent.insert(victim);
        spent.insert(attempt.captor);

        //  THE CAPTIVE'S MOVE ENDS HERE, AND ITS VOLUMES WITH IT.
        //
        //  through the ONE teardown path (`cancel_move_playback`), which is why
        // that was extracted from its four hand-copies first rather than becoming
        // a fifth here.
        if let Ok(mut playback) = playbacks.get_mut(victim) {
            crate::moveset::cancel_move_playback(&mut commands, victim, &mut playback);
        }
        // The captive's control projection. `CapturedBy` stays the authority;
        // this is only what it means for input, and it is CLAIMED rather than
        // inserted: a KO card or a round break can legitimately hold this same
        // body while the grab lasts, and a release that removed the marker
        // outright would take their hold off with its own.
        //
        //  it is a PROJECTION, and that is the shape escape needs later: a
        // mash-to-escape read samples the captive's raw participant input into a
        // restricted capture channel, and would be impossible if capture meant
        // "this body's input ceases to exist".
        ambition_characters::control::claim_control_hold(
            &mut commands,
            victim,
            ambition_characters::control::ControlHold::Relationship,
        );
        if let Ok((abilities, mut jump, mut dodge, model)) = budgets.get_mut(victim) {
            jump.air_jumps_available = abilities.abilities.air_jump_count(model.air_jumps());
            dodge.air_dodge_spent = false;
        }
        commands.entity(victim).insert(CapturedBy {
            captor: attempt.captor,
            hold_offset_local: attempt.hold_offset,
            //  REMEMBERED, not assumed: a flying body's scale is not 1.0, and a
            // release that wrote a constant would land it on the floor.
            prior_gravity_scale: participants
                .get(victim)
                .map(|body| body.surface.gravity_scale)
                .unwrap_or(1.0),
        });
        //  the RULESET's half of the hold, inserted beside the relation.
        // Pummel count, hold age and escape progress are platform-fighter
        // policy; `CapturedBy` is the relation and answers none of them. A hold
        // without this component is one this ruleset has no opinion about.
        //
        //  and its deadline is decided HERE, once, from the captive's damage
        // at the moment it was caught. That is the genre's rule — a hold does
        // not grow because its captor pummelled — and it is why the seconds are
        // stored rather than asked for again every tick.
        commands.entity(victim).insert(
            ambition_characters::smash_capture::SmashHoldState::lasting(
                rules.grab_hold_seconds(
                    participants
                        .get(victim)
                        .map(|body| body.health.damage_taken())
                        .unwrap_or(0),
                ),
            ),
        );
    }
}

/// What a brain is told about this body's place in a capture.
///
///  a struct rather than the `(bool, bool, u8)` it replaces, and the tuple's
/// own comment is the argument: *"inserting it mid-list silently shifted two
/// positional arguments into the wrong slots and the compiler reported it as a
/// type error three parameters away"*. Giving a brain the hold's AGE would have
/// been that edit a second time.
///
///  it also deletes the two byte-identical inline resolutions that stood at
/// the snapshot's two call sites.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CaptureFacts {
    /// This body is held by somebody.
    pub captured: bool,
    /// How long it has been held, in scaled seconds. `0.0` when free.
    pub captured_for: f32,
    /// This body is holding somebody.
    pub holding_captive: bool,
    /// Pummels landed on the hold this body OWNS; `0` unless `holding_captive`.
    pub pummels_landed: u8,
}

impl CaptureFacts {
    /// Read both ends of the relationship out of the one table that records it.
    ///
    ///  the RELATION answers who, and the RULESET's state answers how long
    /// and how many. `SmashHoldState` is `Option` because a hold this ruleset
    /// has no opinion about is a real thing — a game that constrains bodies
    /// without pummelling them has the relation and not the policy — and
    /// `unwrap_or_default` reads that as "no pummels, no age", which is true.
    pub fn resolve(
        body: Entity,
        captives: &Query<(
            Entity,
            &CapturedBy,
            Option<&ambition_characters::smash_capture::SmashHoldState>,
        )>,
    ) -> Self {
        let held = captives.iter().find(|(entity, _, _)| *entity == body);
        let holding = captives.iter().find(|(_, held, _)| held.captor == body);
        Self {
            captured: held.is_some(),
            captured_for: held
                .and_then(|(_, _, state)| state)
                .map(|state| state.held_for)
                .unwrap_or(0.0),
            holding_captive: holding.is_some(),
            pummels_landed: holding
                .and_then(|(_, _, state)| state)
                .map(|state| state.pummels_landed)
                .unwrap_or(0),
        }
    }
}

/// The captive's restricted channel: a mash reaches the hold, and nothing
/// else does.
///
///  placed where the frame is still LIVE, and that placement IS the
/// design. A captive carries `ScriptedControl`, and blanking is what makes
/// that marker mean something — so a reader placed after the blanking samples
/// zeros and would conclude that captives never struggle. This runs immediately
/// before each blanking, which is why it is scheduled TWICE: human input is
/// blanked in `PlayerInputSet::ControlGate`, and an actor brain writes its frame
/// a whole phase later in `WorldPrep`, so there is no single position where both
/// are observable. The blanking pair has exactly this shape for exactly this
/// reason, and neither placement double-counts the other: whichever writer is
/// live for a body, the other position sees the zeros the first blanking left.
///
///  it reads the frame and writes nothing back to it. The captive still
/// cannot walk, jump or attack — the press is credited to an escape and then
/// blanked exactly as before. That is what a restricted context means here: the
/// input exists, and there is precisely one thing it can do.
///
///  one credit per tick, not one per button. Otherwise a chord of six
/// buttons would be six presses, and escape would reward a control-scheme trick
/// rather than a mash.
pub fn sample_capture_escape(
    mut captives: Query<
        (
            &mut ambition_characters::smash_capture::SmashHoldState,
            &ambition_characters::control::ActorControl,
        ),
        //  without this filter every body that has ever been captured keeps accumulating
        // escape progress from ordinary play, forever — a write to ROLLBACK STATE every tick,
        // churning the checksum for a hold that ended.
        bevy::prelude::With<CapturedBy>,
    >,
    tuning: Option<Res<crate::rules::ResolvedCombatTuning>>,
) {
    let rules = tuning.map(|t| *t).unwrap_or_default();
    for (mut held, control) in &mut captives {
        let frame = &control.0;
        // Any action press. Asking for one specific button would be a
        // control-scheme decision this has no reason to make, and a captive
        // mashing the "wrong" one would look like a broken mechanic.
        let pressed = frame.melee_pressed
            || frame.jump_pressed
            || frame.burst_pressed
            || frame.special_pressed
            || frame.grab_pressed
            || frame.projectile_pressed;
        if pressed {
            held.mash_credit += rules.grab_mash_seconds;
        }
    }
}

/// The captor centred its stick, so a DIRECTION now means "throw".
///
/// ⛔⛔ THIS IS RULESET STATE AND IT LIVES IN A RULESET SYSTEM. It was first
/// written inside `restrict_captor_control` — the generic restricted-control
/// projection — mutating the generic `CapturedBy` relation to implement a
/// platform-fighter input gesture. A GPT review caught it: "centre the stick
/// before a direction-alone throw" is not a fact about who holds whom or what
/// release restores, and a game that constrains bodies without a throw
/// vocabulary should not pay to rewind it.
///
/// ⭐ THE QUERY IS THE BOUNDARY. Asking for `SmashHoldState` is what scopes this
/// to holds this ruleset has an opinion about; a capture without one is simply
/// not seen here, exactly as `tick_capture_holds` reads it.
pub fn arm_smash_throw_edge(
    captors: Query<(Entity, &ambition_characters::control::ActorControl)>,
    mut holds: Query<(
        &CapturedBy,
        &mut ambition_characters::smash_capture::SmashHoldState,
    )>,
) {
    if holds.is_empty() {
        return;
    }
    let centred: std::collections::HashSet<Entity> = captors
        .iter()
        .filter(|(_, control)| control.0.attack_axis == ae::LocalAxes::ZERO)
        .map(|(entity, _)| entity)
        .collect();
    for (held, mut state) in &mut holds {
        // ⛔ LATCHED, never cleared here. The edge is spent by the throw that
        // consumes it and by the hold ending; un-arming on a re-press would
        // make a captor who flicks the stick twice unable to throw at all.
        if !state.throw_armed && centred.contains(&held.captor) {
            state.throw_armed = true;
        }
    }
}

/// A hold ages, and ends when its clock runs out or its captive gets out.
///
/// The third and fourth ways a capture can end, beside the throw and the
/// interruption — and like both of those, through the ONE
/// [`release_capture`], so gravity, the relationship and the control claim come
/// back together however the hold ended.
///
///  scaled seconds, so a hold does not age during hitstop. A pummel's own
/// freeze frames would otherwise buy the captor time it did not earn.
pub fn tick_capture_holds(
    mut commands: Commands,
    time: Res<ambition_time::WorldTime>,
    mut captives: Query<(
        Entity,
        &CapturedBy,
        //  the clock and the escape are the RULESET's, so this system asks
        // for them explicitly. A hold with no `SmashHoldState` is one this
        // ruleset does not time out or let anybody mash out of — which is the
        // correct behaviour for a game that constrains bodies under different
        // rules, and is why the query REQUIRES it rather than treating absence
        // as zero and releasing everybody on the first tick.
        &mut ambition_characters::smash_capture::SmashHoldState,
        Option<&mut ae::ActorSurfaceState>,
        Option<&mut ambition_platformer2d_core::BodyGroundState>,
        Option<&mut ambition_characters::control::ControlHolds>,
    )>,
) {
    let dt = time.scaled_dt;
    for (victim, held, mut state, surface, ground, holds) in &mut captives {
        state.held_for += dt;
        if !state.escaped() {
            continue;
        }
        let ended = *held;
        release_capture(
            &mut commands,
            victim,
            &ended,
            surface.map(|surface| surface.into_inner()),
            ground.map(|ground| ground.into_inner()),
            holds.map(|holds| holds.into_inner()),
        );
    }
}

/// A captive is DRAWN as held.
///
///  a mirror, not a latch, and that is the whole implementation note. It
/// writes `held` for EVERY body with animation facts, from that body's own
/// `CapturedBy` — so a released body is drawn free on the next frame without
/// anybody remembering to clear it. Writing `true` on captives alone would leave
/// the last-held body stuck in the pose forever, which is the shape
/// `release_interrupted_captures` would then have had to know about.
///
///  and it is why the anim layer does not query `CapturedBy` itself. The
/// relation belongs to combat and the pose to presentation; a render system
/// reaching across for a gameplay component is how the two stop agreeing about
/// when a hold ended. The sim publishes the fact here, and every picker — the
/// controlled road and the actor road alike — reads the same one.
pub fn mirror_capture_into_anim_facts(
    //  a SECOND read of the same relation, because the captor is not reachable
    // from its own row: `CapturedBy` lives on the CAPTIVE and names the captor,
    // so the only way to ask "is anybody holding me... or am I holding anybody"
    // is to read every hold once.
    holds: Query<&CapturedBy>,
    mut bodies: Query<(
        Entity,
        &mut ambition_characters::actor::BodyAnimFacts,
        Option<&CapturedBy>,
    )>,
) {
    let captors: Vec<Entity> = holds.iter().map(|hold| hold.captor).collect();
    for (entity, mut anim, held) in &mut bodies {
        let now = held.is_some();
        //  `is_changed()` writes must be idempotent under rollback, so only
        // touch the component when the answer actually moved.
        if anim.held != now {
            anim.held = now;
        }
        let holding = captors.contains(&entity);
        if anim.holding != holding {
            anim.holding = holding;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_core::BodyShieldState;

    /// A body standing on the floor that is a complete [`CaptureParticipant`].
    ///
    ///  complete on purpose. Acquisition requires the body role the whole
    /// lifecycle needs, so a fixture that spawned half a body would be refused —
    /// and a fixture that was refused for a reason the test did not name is how
    /// the end-to-end acceptance came to be misdiagnosed. A test that wants an
    /// INCOMPLETE body removes what it means to remove, by name.
    fn grounded_body(app: &mut App, id: &str, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                ae::BodyKinematics {
                    pos,
                    facing: 1.0,
                    size: ae::Vec2::new(16.0, 24.0),
                    ..Default::default()
                },
                crate::components::CenteredAabb::new(pos, ae::Vec2::new(8.0, 12.0)),
                crate::components::ActorFaction::Enemy,
                ambition_platformer2d_core::BodyGroundState {
                    head_contact: false,
                    on_ground: true,
                    contact_initialized: true,
                },
                ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health {
                    current: 100,
                    max: 100,
                    invulnerable: Default::default(),
                }),
                ambition_characters::actor::BodyCombat::default(),
                ae::BodyFlightState::default(),
                surface_state(1.0),
                // The movement columns every integrated body carries from
                // spawn. `MotionModel`'s own doc says absence is not a policy,
                // and the throw reads the air budget off it — a fixture without
                // them is not a body the throw system can see.
                ae::BodyAbilities::default(),
                ae::BodyDashState::default(),
                ae::BodyJumpState::default(),
                ae::BodyDodgeState::default(),
                ae::MotionModel::axis_swept(ae::AxisSweptParams::default()),
                SimId::placement(id),
            ))
            .id()
    }

    /// A floor-clinging surface state at `gravity_scale`. There is no `Default`
    /// on purpose — a surface normal has no sensible neutral, so a fixture states
    /// the floor explicitly.
    fn surface_state(gravity_scale: f32) -> ae::ActorSurfaceState {
        ae::ActorSurfaceState {
            surface_normal: ae::Vec2::new(0.0, -1.0),
            gravity_scale,
        }
    }

    /// A hold built the way `acquire_captures` builds one, for a fixture that
    /// hands a body its relation directly.  NOT `SmashHoldState::default()`:
    /// that row's `escape_seconds` is `0.0`, which is a hold already over.
    fn fresh_hold() -> ambition_characters::smash_capture::SmashHoldState {
        ambition_characters::smash_capture::SmashHoldState::lasting(
            crate::rules::ResolvedCombatTuning::default().grab_hold_seconds(0),
        )
    }

    fn capture_app() -> App {
        let mut app = App::new();
        app.add_message::<CaptureAttemptRequested>();
        app.add_systems(Update, acquire_captures);
        app
    }

    fn attempt(captor: Entity) -> CaptureAttemptRequested {
        CaptureAttemptRequested {
            captor,
            offset: ae::Vec2::new(16.0, 0.0),
            half_extents: ae::Vec2::new(12.0, 14.0),
            hold_offset: ae::Vec2::new(18.0, 0.0),
        }
    }

    /// ⭐ THE CATCH GIVES THE CAPTIVE ITS AIR RECOVERY BACK, AND THE CATCH IS
    /// WHERE THE RULE LIVES.
    ///
    /// Being grabbed re-seats a body: the hold suspends its gravity, pins it to
    /// the captor, and what it is released into is a fresh airtime. The genre
    /// hands back the double jump and the air dodge for that.
    ///
    /// ⛔⛔ IT USED TO ARRIVE VIA THE THROW, as a side effect of the throw
    /// calling the shared hit reaction with a whole-air-budget refresh. Two
    /// things were wrong with that. A captive that MASHES OUT instead of being
    /// thrown was grabbed just the same and got nothing; and it made "a hit
    /// refreshes the air budget" look like a rule, which is how an ordinary
    /// edge-guard hit came to hand back a double jump it has no business
    /// touching.
    #[test]
    fn being_caught_restores_the_captives_double_jump_and_air_dodge() {
        let mut app = capture_app();
        let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        // Opposed sides, or friendly fire refuses the grab.
        app.world_mut()
            .entity_mut(victim)
            .insert(crate::components::ActorFaction::Player);
        {
            let mut victim_mut = app.world_mut().entity_mut(victim);
            // ⭐ GRANTED, not assumed. `BodyAbilities::default()` authors no
            // double jump at all, so every assertion below would hold for a body
            // that never had one — the shape of a test that cannot fail.
            victim_mut
                .get_mut::<ae::BodyAbilities>()
                .unwrap()
                .abilities
                .double_jump = true;
            victim_mut
                .get_mut::<ae::BodyJumpState>()
                .unwrap()
                .air_jumps_available = 0;
            victim_mut
                .get_mut::<ae::BodyDodgeState>()
                .unwrap()
                .air_dodge_spent = true;
        }
        let authored = {
            let world = app.world();
            world
                .get::<ae::BodyAbilities>(victim)
                .unwrap()
                .abilities
                .air_jump_count(world.get::<ae::MotionModel>(victim).unwrap().air_jumps())
        };
        assert!(
            authored > 0,
            "the fixture grants no air jump, so the restore restores nothing and \
             this test cannot fail"
        );

        app.world_mut().write_message(attempt(captor));
        app.update();

        assert!(
            app.world().get::<CapturedBy>(victim).is_some(),
            "the fixture never caught anybody, so nothing was measured"
        );
        assert_eq!(
            app.world()
                .get::<ae::BodyJumpState>(victim)
                .unwrap()
                .air_jumps_available,
            authored,
            "the catch did not give the captive its double jump back"
        );
        assert!(
            !app.world()
                .get::<ae::BodyDodgeState>(victim)
                .unwrap()
                .air_dodge_spent,
            "the catch did not give the captive its air dodge back"
        );
    }

    /// BOTH ENDS OF A HOLD ARE DRAWN, AND BOTH ARE RELEASED.
    ///
    ///  the mirror is what lets the anim layer stay out of `CapturedBy`, so the
    /// captor half has to come from the same pass — the relation lives on the
    /// CAPTIVE and names its captor, so nothing on the captor's own row says it
    /// is holding anybody.  and the release half is the assertion that matters:
    /// a `holding` flag set on acquisition and never cleared would leave the last
    /// captor stuck in the hold pose for the rest of the match, which is exactly
    /// the latch shape this system was written as a mirror to avoid.
    #[test]
    fn a_hold_draws_the_captor_as_well_as_the_captive() {
        let mut app = App::new();
        app.add_systems(Update, mirror_capture_into_anim_facts);
        let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        for body in [captor, victim] {
            app.world_mut()
                .entity_mut(body)
                .insert(ambition_characters::actor::BodyAnimFacts::default());
        }
        app.world_mut().entity_mut(victim).insert(CapturedBy {
            captor,
            hold_offset_local: ae::Vec2::new(18.0, 0.0),
            prior_gravity_scale: 1.0,
        });

        app.update();

        let facts = |app: &App, body: Entity| {
            let f = app
                .world()
                .get::<ambition_characters::actor::BodyAnimFacts>(body)
                .expect("the body kept its anim facts");
            (f.held, f.holding)
        };
        assert_eq!(facts(&app, victim), (true, false), "the captive");
        assert_eq!(facts(&app, captor), (false, true), "the captor");

        app.world_mut().entity_mut(victim).remove::<CapturedBy>();
        app.update();

        assert_eq!(facts(&app, victim), (false, false), "the freed body");
        assert_eq!(
            facts(&app, captor),
            (false, false),
            "the captor kept the hold pose after letting go"
        );
    }

    /// A HURT CAPTIVE IS HELD LONGER, AND THE HOLD DOES NOT GROW UNDER IT.
    #[test]
    fn a_hold_is_measured_from_the_damage_the_captive_had_when_it_was_caught() {
        let rules = crate::rules::ResolvedCombatTuning {
            grab_hold_base_seconds: 1.0,
            grab_hold_per_damage: 0.02,
            grab_hold_max_seconds: 10.0,
            ..Default::default()
        };
        let deadline = |damage: i32| {
            let mut app = capture_app();
            app.insert_resource(rules);
            let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
            let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
            //  a DIFFERENT faction: `grounded_body` makes everybody an enemy,
            // and a captor may not grab its own side with friendly fire off.
            app.world_mut()
                .entity_mut(victim)
                .insert(crate::components::ActorFaction::Player);
            app.world_mut().entity_mut(victim).insert(
                ambition_characters::actor::BodyHealth::restored(
                    ambition_characters::actor::Health {
                        current: 100,
                        max: 100,
                        invulnerable: Default::default(),
                    },
                    damage,
                    Default::default(),
                ),
            );
            app.world_mut().write_message(attempt(captor));
            app.update();
            assert!(
                app.world().get::<CapturedBy>(victim).is_some(),
                "no capture was established, so this measured nothing"
            );
            let held = *app
                .world()
                .get::<ambition_characters::smash_capture::SmashHoldState>(victim)
                .expect("a held body carries the ruleset's hold state");
            (app, victim, held.escape_seconds)
        };

        let (_, _, fresh) = deadline(0);
        let (mut app, victim, hurt) = deadline(50);
        assert_eq!(fresh, 1.0);
        assert_eq!(hurt, 2.0, "the captive's damage did not lengthen the hold");

        //  and now hurt it FURTHER, mid-hold. The deadline must not move.
        app.world_mut()
            .get_mut::<ambition_characters::actor::BodyHealth>(victim)
            .expect("the captive kept its health")
            .damage(40);
        app.update();
        assert_eq!(
            app.world()
                .get::<ambition_characters::smash_capture::SmashHoldState>(victim)
                .expect("still held")
                .escape_seconds,
            hurt,
            "damage dealt DURING the hold extended it — a pummel is supposed to \
             cost the captor its throw window, not buy more of one"
        );
    }

    ///  A SHIELD DOES NOT STOP A GRAB — the third leg of the triangle.
    ///
    /// attack beats grab, grab beats shield, shield beats attack. If a capture
    /// ever consults `BodyShieldState`, that cycle collapses into "shield beats
    /// everything" and the whole neutral game goes with it. This is the test
    /// that would go red the day somebody adds the defensive check that looks
    /// obviously correct from inside the hit path.
    #[test]
    fn a_shielding_body_is_captured_anyway() {
        let mut app = capture_app();
        let captor = grounded_body(&mut app, "captor", ae::Vec2::new(0.0, 0.0));
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        app.world_mut()
            .entity_mut(victim)
            .insert(BodyShieldState::default());
        app.world_mut()
            .entity_mut(victim)
            .insert(crate::components::ActorFaction::Player);
        app.world_mut().write_message(attempt(captor));
        app.update();
        assert!(
            app.world().get::<CapturedBy>(victim).is_some(),
            "a raised shield stopped a grab — the rock-paper-scissors triangle \
             has no third leg if a guard beats a capture too"
        );
    }

    ///  A FREED BODY STOPS ACCUMULATING ESCAPE PROGRESS.
    ///
    /// `release_capture` removes the RELATION and leaves `SmashHoldState` on the
    /// body — which is fine, because a fresh capture overwrites it. What is not
    /// fine is a sampler that reads the ruleset's half without asking whether
    /// the body is actually held: every body that has ever been captured would
    /// then keep crediting escape presses from ordinary play, forever, writing
    /// ROLLBACK STATE every tick for a hold that ended.
    #[test]
    fn a_freed_body_stops_crediting_escape_presses() {
        let mut app = capture_app();
        app.add_systems(bevy::prelude::Update, sample_capture_escape);
        let captor = grounded_body(&mut app, "captor", ae::Vec2::new(0.0, 0.0));
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        app.world_mut()
            .entity_mut(victim)
            .insert(crate::components::ActorFaction::Player);
        app.world_mut()
            .entity_mut(victim)
            .insert(ambition_characters::control::ActorControl(
                ambition_characters::actor::control::ActorControlFrame::neutral(),
            ));
        app.world_mut().write_message(attempt(captor));
        app.update();
        assert!(
            app.world().get::<CapturedBy>(victim).is_some(),
            "no capture was established, so this measured nothing"
        );

        // Mash while HELD: progress accrues.
        app.world_mut()
            .get_mut::<ambition_characters::control::ActorControl>(victim)
            .expect("the captive carries a control frame")
            .0
            .melee_pressed = true;
        app.update();
        let while_held = app
            .world()
            .get::<ambition_characters::smash_capture::SmashHoldState>(victim)
            .expect("a held body carries the ruleset's hold state")
            .mash_credit;
        assert!(
            while_held > 0.0,
            "mashing while held credited nothing, so the sampler is not running"
        );

        // Free the body, keep mashing: progress must not move.
        app.world_mut().entity_mut(victim).remove::<CapturedBy>();
        for _ in 0..10 {
            app.update();
        }
        let after_release = app
            .world()
            .get::<ambition_characters::smash_capture::SmashHoldState>(victim)
            .expect("the state stays on the body, and that is fine")
            .mash_credit;
        assert_eq!(
            after_release, while_held,
            "a body nobody is holding kept accumulating escape progress — the \
             sampler is reading the ruleset's half without asking whether there \
             is a hold, and writing rollback state every tick for it"
        );
    }

    ///  A HOLD IS TWO COMPONENTS, AND ACQUIRING ONE INSERTS BOTH.
    ///
    /// A body carrying only the first is held by somebody with no clock and nothing to mash out
    /// of: `tick_capture_holds` requires the state, so such a hold would never time out, and
    /// `sample_capture_escape` would never credit a press.
    ///
    ///  pinned because the pairing is a convention, not a type. Nothing
    /// stops a future site inserting the relation alone, and there is exactly
    /// ONE production site that establishes a hold — precisely the situation
    /// where a second one lands later and nobody notices.
    #[test]
    fn acquiring_a_capture_inserts_both_halves_of_the_hold() {
        let mut app = capture_app();
        let captor = grounded_body(&mut app, "captor", ae::Vec2::new(0.0, 0.0));
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        app.world_mut()
            .entity_mut(victim)
            .insert(crate::components::ActorFaction::Player);
        app.world_mut().write_message(attempt(captor));
        app.update();

        //  the zero floor: no capture at all would satisfy "no unpaired hold"
        // by having no hold, which is the vacuous pass this must not take.
        assert!(
            app.world().get::<CapturedBy>(victim).is_some(),
            "no capture was established, so this measured nothing"
        );
        assert!(
            app.world()
                .get::<ambition_characters::smash_capture::SmashHoldState>(victim)
                .is_some(),
            "the relation was inserted without this ruleset's half — the captive \
             has no hold clock and no escape accumulator, so the grab would last \
             forever and no mash could end it"
        );
    }

    /// ⭐⭐ TWO BODIES THAT GRAB EACH OTHER ON ONE TICK MAKE **ONE** HOLD.
    ///
    /// ⛔⛔ THE INVARIANT THIS FILE ALREADY CLAIMED, and it held on every tick
    /// except the one it mattered on. `acquire_captures` asks the `captives`
    /// QUERY whether a body is already held — and its own inserts go through
    /// `Commands`, which are deferred, so an attempt granted earlier in the SAME
    /// pass is invisible to the next one. Two bodies reaching for each other on
    /// one tick therefore both succeeded, and each became captor AND captive of
    /// the other.
    ///
    /// ⛔ IT DEADLOCKS, which is why it cost a third of a match rather than
    /// looking odd. The brain's capture policy answers "am I captured" first and
    /// returns, so a body that is both can only ever struggle: neither side
    /// pummels, neither throws, both are control-held, and the lock ends only
    /// when the hold clock runs out. Measured in the full app before this fix,
    /// `npc_pirate_admiral` rung 9 mirror, 3600 ticks: 40 captures, 2028
    /// capture-ticks (28% of the match), and ZERO pummels and ZERO throws.
    ///
    /// ⚠ ONE, not zero. A CLASH — granting neither — was the first fix and was
    /// MEASURED before it was believed: it deletes the mechanic, because in a
    /// mirror match the two brains are identical, so every grab is simultaneous.
    /// 126 attempts over a minute produced ZERO captures, zero pummels and zero
    /// throws. A mirror is a fixed point: identical inputs produce identical
    /// states however a tie is resolved, so the only symmetric resolution is the
    /// one that never grants a grab at all.
    #[test]
    fn two_bodies_grabbing_each_other_on_one_tick_make_one_hold() {
        let mut app = capture_app();
        let east = grounded_body(&mut app, "east", ae::Vec2::new(0.0, 0.0));
        let west = grounded_body(&mut app, "west", ae::Vec2::new(16.0, 0.0));
        // Hostile to each other, so neither grab is refused by friendly fire.
        app.world_mut()
            .entity_mut(west)
            .insert(crate::components::ActorFaction::Player);
        // Facing each other, so each one's grab volume reaches the other.
        app.world_mut()
            .entity_mut(west)
            .get_mut::<ae::BodyKinematics>()
            .expect("the fixture body has kinematics")
            .facing = -1.0;

        // ONE pass, both attempts.
        app.world_mut().write_message(attempt(east));
        app.world_mut().write_message(attempt(west));
        app.update();

        let held: Vec<Entity> = [east, west]
            .into_iter()
            .filter(|body| app.world().get::<CapturedBy>(*body).is_some())
            .collect();
        assert_eq!(
            held.len(),
            1,
            "two bodies grabbed each other on one tick and {} hold(s) came out of \
             it. TWO is the deadlock — each is captor and captive of the other, \
             and no fighter can act out of that. ZERO means the tie clashed, which \
             measures as a capture kit that can never be reached in a mirror.",
            held.len()
        );
        // The loser is held by the winner — not by itself, and not dangling.
        let captive = held[0];
        let captor = app
            .world()
            .get::<CapturedBy>(captive)
            .expect("the hold that exists")
            .captor;
        assert_ne!(captor, captive, "a body captured itself");
        assert!(
            app.world().get::<CapturedBy>(captor).is_none(),
            "the winner of the tie is ALSO held, which is the deadlock this guards"
        );
        // ⛔ AND THE WINNER IS THE `SimId` TIE-BREAK'S, not whichever message
        // happened to be written first. `east` sorts before `west`.
        assert_eq!(captive, west, "the tie went the other way");
    }

    /// ⛔⛔ NO BODY IS EVER BOTH A CAPTOR AND A CAPTIVE, and the accepted
    /// captures do not depend on which message the reader visited first.
    ///
    /// This replaces a test that wrote ONE message order and asserted only two
    /// of the three bodies. It passed while `B→C` then `A→B` left B captive of A
    /// AND captor of C — the exact contradictory state the mutual-pair rule
    /// existed to prevent, reached by a shape the pair rule never looked at.
    /// Found by a GPT review reading the accept condition: it rejected a victim
    /// already TAKEN this pass, but never one already HOLDING.
    ///
    /// ⭐ BOTH ORDERS, AND THE WHOLE RELATIONSHIP SET. An arbitration that
    /// depends on the mailbox produces different sets from the same world, so
    /// the two orders are compared against each OTHER as well as against the
    /// invariant.
    #[test]
    fn a_chain_of_grabs_on_one_tick_never_makes_a_body_both_roles() {
        // A -> B -> C, each within its neighbour's reach and facing it.
        let build = || {
            let mut app = capture_app();
            let a = grounded_body(&mut app, "a", ae::Vec2::new(0.0, 0.0));
            let b = grounded_body(&mut app, "b", ae::Vec2::new(16.0, 0.0));
            let c = grounded_body(&mut app, "c", ae::Vec2::new(32.0, 0.0));
            // ⛔⛔ ALTERNATING FACTIONS, because friendly fire is OFF and the
            // grab asks `damage_lands_between`. The version this replaces made
            // B and C allies, so B→C was refused at resolution and the "chain"
            // it claimed to test never existed — which is also why it could
            // assert nothing about C.
            app.world_mut()
                .entity_mut(a)
                .insert(crate::components::ActorFaction::Enemy);
            app.world_mut()
                .entity_mut(b)
                .insert(crate::components::ActorFaction::Player);
            app.world_mut()
                .entity_mut(c)
                .insert(crate::components::ActorFaction::Enemy);
            (app, a, b, c)
        };

        let run = |forward: bool| -> Vec<(usize, Option<usize>)> {
            let (mut app, a, b, c) = build();
            if forward {
                app.world_mut().write_message(attempt(a));
                app.world_mut().write_message(attempt(b));
            } else {
                app.world_mut().write_message(attempt(b));
                app.world_mut().write_message(attempt(a));
            }
            app.update();
            let index = |e: Entity| [a, b, c].iter().position(|x| *x == e).unwrap();
            [a, b, c]
                .iter()
                .enumerate()
                .map(|(i, body)| {
                    (
                        i,
                        app.world()
                            .get::<CapturedBy>(*body)
                            .map(|h| index(h.captor)),
                    )
                })
                .collect()
        };

        let forward = run(true);
        let reversed = run(false);
        assert_eq!(
            forward, reversed,
            "the accepted captures changed with the MESSAGE ORDER, so the \
             arbitration is a property of the mailbox rather than of the world"
        );

        for set in [&forward, &reversed] {
            let captives: Vec<usize> = set.iter().filter_map(|(i, by)| by.map(|_| *i)).collect();
            let captors: Vec<usize> = set.iter().filter_map(|(_, by)| *by).collect();
            for body in &captives {
                assert!(
                    !captors.contains(body),
                    "body {body} is both a captive and a captor in {set:?} — the \
                     contradictory state the whole arbitration exists to refuse"
                );
            }
            assert!(
                !captives.is_empty(),
                "no grab landed at all, so this measured nothing: {set:?}"
            );
        }

        // ⭐ AND THE SPECIFIC OUTCOME, so a rule that refuses EVERYTHING also
        // fails: A holds B, and C — B's target — walks away free.
        assert_eq!(
            forward,
            vec![(0, None), (1, Some(0)), (2, None)],
            "expected A holding B with C free"
        );
    }

    /// ⛔⛔ TWO CAPTORS REACHING ONE VICTIM: EXACTLY ONE GETS THE HOLD.
    ///
    /// ⭐ THIS IS THE TOPOLOGY THAT EXERCISES THE VICTIM CHECK, and the chain
    /// test above does NOT — I poisoned it by deleting `spent.contains(&victim)`
    /// and it stayed green, because a chain's second edge is refused by the
    /// CAPTOR check instead. A guard that is green for the wrong reason is worth
    /// less than no guard, so the topology that isolates the clause lives here.
    ///
    /// Both message orders, because "whichever attempt the reader visited first"
    /// is not a rule.
    #[test]
    fn two_captors_reaching_one_victim_yield_exactly_one_hold() {
        let build = || {
            let mut app = capture_app();
            let a = grounded_body(&mut app, "a", ae::Vec2::new(0.0, 0.0));
            let c = grounded_body(&mut app, "c", ae::Vec2::new(16.0, 0.0));
            let b = grounded_body(&mut app, "b", ae::Vec2::new(32.0, 0.0));
            // B reaches LEFT, so both A and B name C and nobody names each other.
            app.world_mut()
                .entity_mut(b)
                .get_mut::<ae::BodyKinematics>()
                .expect("the fixture body carries kinematics")
                .facing = -1.0;
            app.world_mut()
                .entity_mut(c)
                .insert(crate::components::ActorFaction::Player);
            (app, a, b, c)
        };

        let run = |forward: bool| -> (usize, Option<usize>, Option<usize>) {
            let (mut app, a, b, c) = build();
            if forward {
                app.world_mut().write_message(attempt(a));
                app.world_mut().write_message(attempt(b));
            } else {
                app.world_mut().write_message(attempt(b));
                app.world_mut().write_message(attempt(a));
            }
            app.update();
            let index = |e: Entity| [a, b, c].iter().position(|x| *x == e).unwrap();
            let held: usize = [a, b, c]
                .iter()
                .filter(|body| app.world().get::<CapturedBy>(**body).is_some())
                .count();
            (
                held,
                app.world().get::<CapturedBy>(c).map(|h| index(h.captor)),
                app.world().get::<CapturedBy>(a).map(|h| index(h.captor)),
            )
        };

        let forward = run(true);
        let reversed = run(false);
        assert_eq!(
            forward, reversed,
            "which captor won changed with the MESSAGE ORDER"
        );
        assert_eq!(
            forward.0, 1,
            "expected exactly one body held; got {} — two captors both took the \
             same victim, or the contest refused both",
            forward.0
        );
        assert_eq!(forward.1, Some(0), "C is held, and by A (the lower SimId)");
        assert_eq!(forward.2, None, "a captor became a captive");
    }

    /// AN AIRBORNE VICTIM IS NOT CAPTURED BY A STANDING GRAB (v1).
    ///
    /// Aerial grabs are a named future technique. A standing grab that happened
    /// to catch a jumping body would be a bad aerial grab nobody designed.
    #[test]
    fn an_airborne_victim_is_out_of_reach_of_a_standing_grab() {
        let mut app = capture_app();
        let captor = grounded_body(&mut app, "captor", ae::Vec2::new(0.0, 0.0));
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        app.world_mut()
            .entity_mut(victim)
            .insert(crate::components::ActorFaction::Player);
        app.world_mut()
            .entity_mut(victim)
            .insert(ambition_platformer2d_core::BodyGroundState {
                head_contact: false,
                on_ground: false,
                contact_initialized: true,
            });
        app.world_mut().write_message(attempt(captor));
        app.update();
        assert!(app.world().get::<CapturedBy>(victim).is_none());
    }

    /// THE CAPTIVE IS HELD AT THE CAPTOR'S ANCHOR, MIRRORED BY ITS FACING.
    ///
    /// The anchor is captor-body-local, so a captor that turns around swings its
    /// captive across rather than leaving it behind — which is what makes a
    /// forward throw point where the player is looking.
    #[test]
    fn a_captive_is_posed_at_its_captors_hold_anchor() {
        let mut app = App::new();
        app.add_systems(Update, maintain_existing_capture_pose);
        let captor = grounded_body(&mut app, "captor", ae::Vec2::new(100.0, 50.0));
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(400.0, 400.0));
        app.world_mut()
            .entity_mut(victim)
            .insert(surface_state(1.0));
        //  a hold is TWO components: the relation, and this ruleset's half.
        app.world_mut().entity_mut(victim).insert((
            CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(20.0, -4.0),
                prior_gravity_scale: 1.0,
            },
            fresh_hold(),
        ));
        app.update();
        let kin = app.world().get::<ae::BodyKinematics>(victim).unwrap();
        assert_eq!(
            kin.pos,
            ae::Vec2::new(120.0, 46.0),
            "facing +1: anchor adds"
        );
        let aabb = app
            .world()
            .get::<crate::components::CenteredAabb>(victim)
            .unwrap();
        assert_eq!(
            aabb.center, kin.pos,
            "the coarse-box mirror was left where the body was grabbed, so a \
             strike aimed at the floor could still reach a captive held overhead"
        );

        // Turn the captor around: the captive swings to the other side.
        app.world_mut()
            .get_mut::<ae::BodyKinematics>(captor)
            .unwrap()
            .facing = -1.0;
        app.update();
        assert_eq!(
            app.world().get::<ae::BodyKinematics>(victim).unwrap().pos,
            ae::Vec2::new(80.0, 46.0),
            "the hold anchor did not mirror with the captor's facing"
        );
    }

    /// ⛔⛔ **A HELD BODY'S CONTACT BASELINE IS INVALIDATED, NOT JUST ITS FLAG.**
    ///
    /// Jon, 2026-08-22: *"Grabbing a character still pushes them into the ground
    /// causing sfx to play repeatedly."* This system used to write
    /// `on_ground = false` beside the pose, which does not say *"carried"* — it
    /// says **"airborne last tick"**. The kernel then samples support at the
    /// forced pose, reads `false -> true`, and reports a LANDING: an
    /// `SfxMessage::Land` and a dust puff at the feet, EVERY TICK.
    ///
    ///  this asserts `contact_initialized`, which is the half that was
    /// missing. `on_ground` was already false and stayed false, so a test that
    /// only read the flag agreed with the bug. The behavioural half — 120
    /// landings in 120 ticks — is
    /// `movement::tests::contacts::a_carrier_that_clears_the_ground_flag_does_not_re_land_the_body_each_tick`
    /// in the core; this one pins the WIRING that test cannot see.
    #[test]
    fn a_held_bodys_contact_baseline_is_invalidated_every_tick() {
        let mut app = App::new();
        app.add_systems(Update, maintain_existing_capture_pose);
        let captor = grounded_body(&mut app, "captor", ae::Vec2::new(100.0, 50.0));
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(400.0, 400.0));
        app.world_mut().entity_mut(victim).insert((
            CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(20.0, -4.0),
                prior_gravity_scale: 1.0,
            },
            fresh_hold(),
        ));

        // The fixture starts with a settled, believed baseline — exactly what a
        // body that was standing when it got grabbed carries in.
        assert!(
            app.world()
                .get::<ambition_platformer2d_core::BodyGroundState>(victim)
                .unwrap()
                .contact_initialized,
            "the fixture must start with a real baseline or this measures nothing"
        );

        for tick in 0..3 {
            app.update();
            let ground = app
                .world()
                .get::<ambition_platformer2d_core::BodyGroundState>(victim)
                .unwrap();
            assert!(
                !ground.contact_initialized,
                "tick {tick}: the hold left the captive's contact BASELINE \
                 standing, so the next kernel step reads airborne -> grounded \
                 and voices a landing",
            );
            assert!(
                !ground.on_ground,
                "tick {tick}: a held body is not standing"
            );
            // Put the baseline back, as a kernel step would.
            app.world_mut()
                .get_mut::<ambition_platformer2d_core::BodyGroundState>(victim)
                .unwrap()
                .contact_initialized = true;
        }
    }

    /// ⭐⭐ **A RELEASED BODY RE-ENTERS PLAY AIRBORNE, WITH A BELIEVED BASELINE.**
    ///
    /// `apply_capture_throws` says so in writing — *"a thrown body is AIRBORNE by
    /// construction"* — and until 2026-08-22 that held by ACCIDENT: the hold
    /// wrote `on_ground = false` and left the baseline standing, so the fact was
    /// a residue of how the constraint spelled itself.
    ///
    /// ⛔⛔ the hold now `invalidate()`s, which it must — otherwise the kernel
    /// manufactures a landing every tick — and an invalidated baseline is
    /// RE-SAMPLED at the hold pose, which sits at the captor's own height and is
    /// therefore usually on the floor. A throw off a grounded captor came out
    /// GROUNDED, and a ground launch is not a launch:
    /// `the_stage_kills::a_team_victory_names_the_team_and_not_its_last_survivor`
    /// went red because two fighters thrown at 4800px/s never left the stage.
    #[test]
    fn a_released_body_re_enters_play_airborne() {
        let mut app = App::new();
        app.add_systems(
            Update,
            (maintain_existing_capture_pose, release_interrupted_captures).chain(),
        );
        let captor = grounded_body(&mut app, "captor", ae::Vec2::new(100.0, 50.0));
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(400.0, 400.0));
        app.world_mut().entity_mut(victim).insert((
            CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(20.0, -4.0),
                prior_gravity_scale: 1.0,
            },
            fresh_hold(),
        ));
        app.update();

        // Held: the baseline is invalidated, which is the landing-loop fix.
        let ground = app
            .world()
            .get::<ambition_platformer2d_core::BodyGroundState>(victim)
            .unwrap();
        assert!(
            !ground.contact_initialized,
            "a held body's contact is unknown"
        );

        // Now end the hold the way a real interruption does — despawn the
        // captor, which `release_interrupted_captures` treats as captor-gone.
        app.world_mut().entity_mut(captor).despawn();
        app.update();

        assert!(
            app.world().get::<CapturedBy>(victim).is_none(),
            "the fixture did not actually release the body, so this measured nothing"
        );
        let ground = app
            .world()
            .get::<ambition_platformer2d_core::BodyGroundState>(victim)
            .unwrap();
        assert!(
            !ground.on_ground,
            "a released body must re-enter play AIRBORNE — a throw off a grounded \
             captor is a launch, not a step"
        );
        assert!(
            ground.contact_initialized,
            "and it must BELIEVE that, or the next kernel step re-samples the \
             hold pose it was let go from and reads the floor it was held over"
        );
    }

    /// A CAPTOR KEEPS ITS ATTACK PRESS AND LOSES EVERYTHING ELSE.
    ///
    /// Both halves matter and they fail in opposite directions: strip the attack
    /// and a captor can hold a body and do nothing to it forever; keep the rest
    /// and a captor walks off with somebody in its hands.
    #[test]
    fn a_captor_is_restricted_but_can_still_press_attack() {
        let mut app = App::new();
        app.add_systems(Update, restrict_captor_control);
        let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        frame.locomotion = ae::LocalAxes::X;
        frame.jump_pressed = true;
        frame.special_pressed = true;
        frame.shield_held = true;
        frame.melee_pressed = true;
        frame.attack_axis = ae::LocalAxes::X;
        app.world_mut()
            .entity_mut(captor)
            .insert(ambition_characters::control::ActorControl(frame));
        //  a hold is TWO components: the relation, and this ruleset's half.
        app.world_mut().entity_mut(victim).insert((
            CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(16.0, 0.0),
                prior_gravity_scale: 1.0,
            },
            fresh_hold(),
        ));
        app.update();
        let held = &app
            .world()
            .get::<ambition_characters::control::ActorControl>(captor)
            .unwrap()
            .0;
        assert_eq!(held.locomotion, ae::LocalAxes::ZERO, "a captor walked away");
        assert!(!held.jump_pressed && !held.special_pressed && !held.shield_held);
        assert!(
            held.melee_pressed && held.attack_axis == ae::LocalAxes::X,
            "the attack press was stripped, so this captor can hold a body and \
             never pummel or throw it"
        );
    }

    /// A HIT ON EITHER BODY ENDS THE HOLD, AND GIVES BACK WHAT IT SUSPENDED.
    ///
    ///  the restored gravity is the body's OWN prior value, not `1.0`. This
    /// captive was floating at `0.25` before it was grabbed, which is the case a
    /// constant would silently break.
    #[test]
    fn a_hit_captor_drops_its_captive_and_gravity_comes_back() {
        let mut app = App::new();
        app.add_systems(Update, release_interrupted_captures);
        let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        for body in [captor, victim] {
            app.world_mut()
                .entity_mut(body)
                .insert(ambition_characters::actor::BodyCombat::default());
        }
        app.world_mut()
            .entity_mut(victim)
            .insert(surface_state(0.0));
        app.world_mut().entity_mut(victim).insert((
            CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(16.0, 0.0),
                prior_gravity_scale: 0.25,
            },
            ambition_characters::control::ScriptedControl,
            // The hold as a CLAIM: a bare marker is nobody's, and the release
            // deliberately leaves what it did not claim.
            ambition_characters::control::ControlHolds::only(
                ambition_characters::control::ControlHold::Relationship,
            ),
        ));

        app.update();
        assert!(
            app.world().get::<CapturedBy>(victim).is_some(),
            "the fixture is wrong: nobody has been hit, so the hold must survive"
        );

        app.world_mut()
            .get_mut::<ambition_characters::actor::BodyCombat>(captor)
            .unwrap()
            .hitstun_timer = 0.2;
        app.update();

        assert!(
            app.world().get::<CapturedBy>(victim).is_none(),
            "still held"
        );
        assert!(
            app.world()
                .get::<ambition_characters::control::ScriptedControl>(victim)
                .is_none(),
            "the captive is free and still cannot move — the control projection \
             outlived the relationship it projects"
        );
        assert_eq!(
            app.world()
                .get::<ae::ActorSurfaceState>(victim)
                .unwrap()
                .gravity_scale,
            0.25,
            "gravity came back as a CONSTANT rather than as what this body had"
        );
    }

    ///  AN EXISTING CAPTOR MISSING COMBAT STATE IS NOT A DESPAWNED ONE.
    ///
    /// The invariant, not the fix. The interruption rule asked `combat.get(captor).is_err()`
    /// and read the answer as *"the captor is gone"*. That misreading is what kept the
    /// end-to-end acceptance red, and what got it diagnosed wrong.
    ///
    ///  built by hand ON PURPOSE. Acquisition now refuses such a captor
    /// outright, so a grab press can no longer reach this state — which is
    /// exactly why the guard must construct it. The rule is *existence is asked
    /// of the world, never inferred from a component*, and it has to hold for
    /// any hold however established: an escape, a scripted capture, a future
    /// carry relationship.
    #[test]
    fn an_existing_captor_without_combat_state_is_not_a_despawned_one() {
        let mut app = App::new();
        app.add_systems(Update, release_interrupted_captures);
        let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        // The one difference from a healthy pair, stated rather than implied.
        app.world_mut()
            .entity_mut(captor)
            .remove::<ambition_characters::actor::BodyCombat>();
        //  a hold is TWO components: the relation, and this ruleset's half.
        app.world_mut().entity_mut(victim).insert((
            CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(16.0, 0.0),
                prior_gravity_scale: 1.0,
            },
            fresh_hold(),
        ));

        app.update();

        assert!(
            app.world().get::<CapturedBy>(victim).is_some(),
            "a captor that is alive but carries no combat state was read as \
             DESPAWNED, and its hold was released underneath it"
        );
    }

    /// A captor that really is gone frees its captive.
    ///
    /// The other direction of the same predicate, and the reason the guard above
    /// is not vacuous: after teaching the rule to stop treating a missing
    /// component as a missing entity, a rule that had stopped noticing genuinely
    /// despawned captors would pass that guard just as happily — and leave every
    /// captive of a KO'd fighter frozen in mid-air with gravity suspended.
    #[test]
    fn a_despawned_captor_frees_its_captive() {
        let mut app = App::new();
        app.add_systems(Update, release_interrupted_captures);
        let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        app.world_mut().entity_mut(victim).insert((
            surface_state(0.0),
            CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(16.0, 0.0),
                prior_gravity_scale: 0.75,
            },
            //  the ruleset's half of the hold: without it there is no
            // clock and nothing to mash out of.
            fresh_hold(),
        ));
        app.world_mut().entity_mut(captor).despawn();

        app.update();

        assert!(
            app.world().get::<CapturedBy>(victim).is_none(),
            "the captor is gone and the hold outlived it"
        );
        assert_eq!(
            app.world()
                .get::<ae::ActorSurfaceState>(victim)
                .unwrap()
                .gravity_scale,
            0.75,
            "freed by a despawn and still weightless — the release path was not \
             the one that ran"
        );
    }

    ///  NOBODY IS HELD FOREVER, AND STRUGGLING BEATS NOT STRUGGLING.
    ///
    /// An unbounded relationship is a gameplay bug rather than a missing feature.
    ///
    ///  the two halves are one test on purpose. A hold that ended on its
    /// clock but ignored the captive would pass a mash test that only asserted
    /// "it ends"; a hold that freed anybody who touched a button would pass a
    /// timeout test that only asserted "it ends". Measuring BOTH escapes against
    /// each other is what pins that the captive's input did the work.
    #[test]
    fn a_hold_ends_on_its_own_clock_and_sooner_when_the_captive_struggles() {
        let mut ticks_to_free = Vec::new();
        for struggling in [false, true] {
            let mut app = App::new();
            let mut time = ambition_time::WorldTime::default();
            time.scaled_dt = 1.0 / 60.0;
            time.raw_dt = 1.0 / 60.0;
            app.insert_resource(time);
            app.add_systems(Update, (sample_capture_escape, tick_capture_holds).chain());
            let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
            let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
            app.world_mut().entity_mut(victim).insert((
                ambition_characters::control::ScriptedControl,
                ambition_characters::control::ControlHolds::only(
                    ambition_characters::control::ControlHold::Relationship,
                ),
                ambition_characters::control::ActorControl(
                    ambition_characters::actor::control::ActorControlFrame::neutral(),
                ),
                CapturedBy {
                    captor,
                    hold_offset_local: ae::Vec2::new(16.0, 0.0),
                    prior_gravity_scale: 1.0,
                },
                //  the ruleset's half of the hold: without it there is no
                // clock and nothing to mash out of.  built the way acquisition
                // builds it — this app installs no `ResolvedCombatTuning`, so
                // the deadline is the undeclared world's flat hold.
                ambition_characters::smash_capture::SmashHoldState::lasting(
                    crate::rules::ResolvedCombatTuning::default().grab_hold_seconds(0),
                ),
            ));

            let mut ticks = 0;
            while app.world().get::<CapturedBy>(victim).is_some() {
                if struggling {
                    // A press this tick. In production the pipeline consumes an
                    // edge once; here the sampler is the only reader, so writing
                    // it fresh each tick IS the mash.
                    app.world_mut()
                        .get_mut::<ambition_characters::control::ActorControl>(victim)
                        .unwrap()
                        .0
                        .melee_pressed = true;
                }
                app.update();
                ticks += 1;
                assert!(
                    ticks < 60 * 30,
                    "thirty seconds in and the hold has not ended — a capture \
                     with no clock is a body taken out of the match"
                );
            }
            assert!(
                app.world()
                    .get::<ambition_characters::control::ScriptedControl>(victim)
                    .is_none(),
                "freed and still unable to move: the release did not go through \
                 the one that gives the control claim back"
            );
            ticks_to_free.push(ticks);
        }
        let [waited, struggled] = ticks_to_free[..] else {
            unreachable!()
        };
        assert!(
            struggled < waited,
            "mashing did not shorten the hold at all ({struggled} ticks vs \
             {waited}) — the captive's input never reached the relationship, \
             which is the whole claim the restricted channel makes"
        );
        //  this app installs no `ResolvedCombatTuning`, so the ceiling is the
        // undeclared world's — asked of the same value the system asks, rather
        // than of a constant a reader would have to trust still matches it.
        let ceiling = crate::rules::ResolvedCombatTuning::default().grab_hold_max_seconds;
        assert!(
            waited <= (ceiling * 60.0).ceil() as i32 + 1,
            "the hold outlived its own stated ceiling"
        );
    }

    /// Releasing a capture removes only the relationship hold. Other concurrent
    /// control holds (round break, countdown, KO presentation, etc.) must remain.
    /// The test covers both with and without an additional hold.
    #[test]
    fn a_release_ends_this_holds_claim_and_leaves_the_others() {
        use ambition_characters::control::{ControlHold, ControlHolds, ScriptedControl};

        for also_held_by_the_stage in [true, false] {
            let mut app = App::new();
            app.add_systems(Update, release_interrupted_captures);
            let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
            let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
            let mut holds = ControlHolds::only(ControlHold::Relationship);
            if also_held_by_the_stage {
                holds.claim(ControlHold::Interlude);
            }
            app.world_mut().entity_mut(victim).insert((
                ScriptedControl,
                holds,
                surface_state(0.0),
                CapturedBy {
                    captor,
                    hold_offset_local: ae::Vec2::new(16.0, 0.0),
                    prior_gravity_scale: 1.0,
                },
                //  the ruleset's half of the hold: without it there is no
                // clock and nothing to mash out of.
                fresh_hold(),
            ));
            // The interruption: the captor is hit.
            app.world_mut()
                .get_mut::<ambition_characters::actor::BodyCombat>(captor)
                .unwrap()
                .hitstun_timer = 0.2;

            app.update();

            assert!(
                app.world().get::<CapturedBy>(victim).is_none(),
                "the interrupted capture did not end"
            );
            let still_held = app.world().get::<ScriptedControl>(victim).is_some();
            assert_eq!(
                still_held, also_held_by_the_stage,
                "released while the stage was holding it: {also_held_by_the_stage}. \
                 A capture ending must free the body when it was the last \
                 authority, and must NOT free it when another one is still \
                 holding — those are the two halves of releasing only your own \
                 claim."
            );
            assert_eq!(
                app.world().get::<ControlHolds>(victim).copied(),
                also_held_by_the_stage.then(|| ControlHolds::only(ControlHold::Interlude)),
                "the surviving claim set is not what the other authority left"
            );
        }
    }

    ///  A BODY THE REST OF THE LIFECYCLE COULD NOT OPERATE ON IS NOT
    /// CAPTURED.
    #[test]
    fn a_body_the_lifecycle_could_not_operate_on_is_never_captured() {
        for (label, strip_captor) in [("captor", true), ("victim", false)] {
            let mut app = capture_app();
            let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
            let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
            app.world_mut()
                .entity_mut(victim)
                .insert(crate::components::ActorFaction::Player);
            // Health, because a pummel and a throw both spend it, and a body
            // without it is one no beat after the first could touch.
            let stripped = if strip_captor { captor } else { victim };
            app.world_mut()
                .entity_mut(stripped)
                .remove::<ambition_characters::actor::BodyHealth>();

            app.world_mut().write_message(attempt(captor));
            app.update();

            assert!(
                app.world().get::<CapturedBy>(victim).is_none(),
                "a capture was established with a {label} the lifecycle cannot \
                 operate on — it would be stranded on its first pummel"
            );
        }
    }

    ///  A PUMMEL DAMAGES ITS CAPTIVE AND DOES NOT BREAK THE HOLD.
    ///
    /// The one that would catch the whole category error. If a pummel were
    /// routed through the ordinary hit path it would arm `hitstun_timer`,
    /// `release_interrupted_captures` would fire on the next tick, and the
    /// pummel would release the grab it belongs to — a mechanic that destroys
    /// itself on its own first beat. So this runs BOTH systems and asserts the
    /// hold survives, which is the property the shortcut breaks.
    #[test]
    fn a_pummel_damages_the_captive_and_leaves_the_hold_standing() {
        let mut app = App::new();
        app.add_message::<crate::capture::CapturePummelRequested>();
        app.add_systems(
            Update,
            (apply_capture_pummels, release_interrupted_captures).chain(),
        );
        let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        for body in [captor, victim] {
            app.world_mut()
                .entity_mut(body)
                .insert(ambition_characters::actor::BodyCombat::default());
        }
        app.world_mut().entity_mut(victim).insert((
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health {
                current: 100,
                max: 100,
                invulnerable: Default::default(),
            }),
            surface_state(0.0),
            CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(16.0, 0.0),
                prior_gravity_scale: 1.0,
            },
            //  the ruleset's half of the hold: without it there is no
            // clock and nothing to mash out of.
            fresh_hold(),
        ));

        for _ in 0..2 {
            app.world_mut()
                .write_message(crate::capture::CapturePummelRequested { captor, damage: 3 });
            app.update();
        }

        // the RELATION survives the pummel, and the COUNT is the ruleset's.
        app.world()
            .get::<CapturedBy>(victim)
            .expect("the pummel released the grab it belongs to");
        let state = app
            .world()
            .get::<ambition_characters::smash_capture::SmashHoldState>(victim)
            .expect("a held body carries this ruleset's hold state");
        assert_eq!(
            state.pummels_landed, 2,
            "the hold did not count its pummels"
        );
        assert_eq!(
            app.world()
                .get::<ambition_characters::actor::BodyHealth>(victim)
                .unwrap()
                .damage_taken(),
            6,
            "the percent meter did not advance, so a pummel is free"
        );
        let combat = app
            .world()
            .get::<ambition_characters::actor::BodyCombat>(victim)
            .unwrap();
        assert_eq!(
            (combat.hitstun_timer, combat.recoil_lock_timer),
            (0.0, 0.0),
            "a pummel armed a hit reaction — that is the ordinary hit path, and \
             it is what makes the grab release itself one tick later"
        );
    }

    fn throw_app() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_message::<crate::capture::CaptureThrowRequested>();
        app.add_systems(Update, apply_capture_throws);
        let captor = grounded_body(&mut app, "captor", ae::Vec2::ZERO);
        let victim = grounded_body(&mut app, "victim", ae::Vec2::new(16.0, 0.0));
        app.world_mut().entity_mut(victim).insert((
            ae::BodyFlightState::default(),
            ambition_characters::actor::BodyCombat::default(),
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health {
                current: 100,
                max: 100,
                invulnerable: Default::default(),
            }),
            surface_state(0.0),
            ambition_characters::control::ScriptedControl,
            // The hold as a CLAIM: a bare marker is nobody's, and the release
            // deliberately leaves what it did not claim.
            ambition_characters::control::ControlHolds::only(
                ambition_characters::control::ControlHold::Relationship,
            ),
            CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(16.0, 0.0),
                prior_gravity_scale: 1.0,
            },
            //  the ruleset's half of the hold: without it there is no
            // clock and nothing to mash out of.
            fresh_hold(),
        ));
        (app, captor, victim)
    }

    fn throw(captor: Entity, growth: f32) -> crate::capture::CaptureThrowRequested {
        crate::capture::CaptureThrowRequested {
            captor,
            damage: 9,
            knockback: 100.0,
            knockback_growth: growth,
            launch_dir: ae::Vec2::new(1.0, -1.0),
        }
    }

    /// A THROW ENDS THE HOLD, HURTS, AND LAUNCHES — in that order.
    ///
    /// All three at once because the order is the mechanic: the damage has to
    /// land before the launch reads the meter, and the release has to land
    /// before the launch arms hitstun (or the interruption rule would do the
    /// releasing a tick later, by accident, through a different path).
    #[test]
    fn a_throw_releases_damages_and_launches_its_captive() {
        let (mut app, captor, victim) = throw_app();
        app.world_mut().write_message(throw(captor, 0.0));
        app.update();

        assert!(
            app.world().get::<CapturedBy>(victim).is_none(),
            "the captive is still held after its own throw"
        );
        assert!(
            app.world()
                .get::<ambition_characters::control::ScriptedControl>(victim)
                .is_none(),
            "thrown and still unable to move"
        );
        assert_eq!(
            app.world()
                .get::<ae::ActorSurfaceState>(victim)
                .unwrap()
                .gravity_scale,
            1.0,
            "a thrown body kept the suspended gravity of its hold"
        );
        assert_eq!(
            app.world()
                .get::<ambition_characters::actor::BodyHealth>(victim)
                .unwrap()
                .damage_taken(),
            9
        );
        let vel = app.world().get::<ae::BodyKinematics>(victim).unwrap().vel;
        assert!(
            vel.length() > 1.0,
            "the throw did not launch anybody: {vel:?}"
        );
        assert!(
            vel.x > 0.0,
            "the throw sent its victim BEHIND the captor, which is a forward \
             throw pointing the wrong way: {vel:?}"
        );
    }

    /// A THROW GETS THE PERCENT SCALING EVERY OTHER LAUNCHER GETS.
    ///
    /// The proof that this rides the shared knockback law rather than a second
    /// launch engine: the same throw on a hurt body sends it farther. If throws
    /// ever grow their own velocity path, the two numbers become equal and this
    /// goes red.
    #[test]
    fn a_throw_launches_a_damaged_body_farther() {
        let launch_at = |accumulated: i32| {
            let (mut app, captor, victim) = throw_app();
            app.world_mut()
                .get_mut::<ambition_characters::actor::BodyHealth>(victim)
                .unwrap()
                .damage(accumulated);
            app.world_mut().write_message(throw(captor, 4.0));
            app.update();
            app.world()
                .get::<ae::BodyKinematics>(victim)
                .unwrap()
                .vel
                .length()
        };
        let fresh = launch_at(0);
        let hurt = launch_at(80);
        assert!(
            hurt > fresh * 1.2,
            "percent scaling did not reach the throw: fresh {fresh}, hurt {hurt}"
        );
    }

    /// A CAPTOR ALREADY HOLDING SOMEBODY TAKES NOBODY ELSE.
    ///
    /// Half of the "one captor, one captive" invariant. Without it a grab whose
    /// window is still live would keep acquiring, and `captive_of` — which
    /// assumes at most one — would start returning whichever the scan reached
    /// first.
    #[test]
    fn a_captor_that_already_holds_somebody_acquires_nobody() {
        let mut app = capture_app();
        let captor = grounded_body(&mut app, "captor", ae::Vec2::new(0.0, 0.0));
        let first = grounded_body(&mut app, "first", ae::Vec2::new(16.0, 0.0));
        let second = grounded_body(&mut app, "second", ae::Vec2::new(17.0, 0.0));
        for body in [first, second] {
            app.world_mut()
                .entity_mut(body)
                .insert(crate::components::ActorFaction::Player);
        }
        //  a hold is TWO components: the relation, and this ruleset's half.
        app.world_mut().entity_mut(first).insert((
            CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(18.0, 0.0),
                prior_gravity_scale: 1.0,
            },
            fresh_hold(),
        ));
        app.world_mut().write_message(attempt(captor));
        app.update();
        assert!(
            app.world().get::<CapturedBy>(second).is_none(),
            "a captor holding one body grabbed a second one"
        );
    }
}

/// A CAPTIVE IS POSED AT TWO POINTS IN A TICK, and they are two different
/// operations: [`maintain_existing_capture_pose`] puts a held body back after
/// integration moved it, and [`finalize_new_capture_pose`] gives a body caught
/// THIS TICK its pose before the tick ends. This is the one rule both apply.
///
/// The same physical mechanism the saddle pin uses —
/// [`constrain_body_pose`](ae::movement::constrain_body_pose) — and deliberately
/// NOT the mount relationship. A rider steers its mount and a captive does not
/// steer its captor; unifying them would make one of those two false.
///
///  velocity follows the CAPTOR's, not `ZERO`. A body released or
/// interrupted mid-carry then inherits the motion of the thing that was carrying
/// it, which is what a person expects to see. Zeroing it would make every
/// release look like the captive hit an invisible wall.
fn pose_captives(
    //  `Without<CapturedBy>` is a SEMANTIC claim, not a borrow trick — though
    // Bevy asking for it is what made the claim explicit. A captive can never be
    // a captor: acquisition refuses a captor already under `ScriptedControl`, and
    // every captive carries it, so a chain A-holds-B-holds-C cannot form. The two
    // queries are disjoint because the relationship says they are, and if that
    // ever stops being true this line is where it should be re-argued rather than
    // relaxed into a `ParamSet`.
    captors: Query<
        (
            &ae::BodyKinematics,
            Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        ),
        Without<CapturedBy>,
    >,
    mut captives: Query<(
        &CapturedBy,
        &mut ae::BodyKinematics,
        &mut ae::ActorSurfaceState,
        &mut ambition_platformer2d_core::BodyGroundState,
        Option<&mut crate::components::CenteredAabb>,
    )>,
) {
    for (held, mut kin, mut surface, mut ground, aabb) in &mut captives {
        let Ok((captor_kin, captor_frame)) = captors.get(held.captor) else {
            // The captor is gone. Releasing is the RELEASE path's job, not this
            // one's — a constraint system that also dissolved relationships
            // would be a second authority on when a capture ends.
            continue;
        };
        let frame = captor_frame
            .map(|f| f.basis())
            .unwrap_or(ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR));
        // The hold anchor is captor-body-local: mirror by the captor's live
        // facing, then rotate into its frame. So a captor that turns around
        // swings its captive across, and a captor under flipped gravity holds it
        // overhead — both for free, because the anchor was never a world offset.
        let local = ae::Vec2::new(
            held.hold_offset_local.x * captor_kin.facing,
            held.hold_offset_local.y,
        );
        let pos = captor_kin.pos + frame.to_world(local);
        ae::movement::constrain_body_pose(&mut kin, pos, captor_kin.vel);
        // Gravity is SUSPENDED, not deleted — `CapturedBy::prior_gravity_scale`
        // holds what to give back.
        surface.gravity_scale = 0.0;
        // ⛔⛔ **`invalidate()`, NOT `on_ground = false`.** A hold writes the
        // captive's pose discretely every tick, which is precisely the "discrete
        // pose change" this call exists for: it clears the contact BASELINE as
        // well as the flag. Setting only the flag tells the kernel this body was
        // AIRBORNE last tick, so the moment the hold puts it against the floor
        // the kernel reads `false -> true` and reports a LANDING — every tick,
        // for as long as the grab lasts. That is one `SfxMessage::Land` and one
        // dust puff per tick, which is what a grab sounded and looked like.
        ground.invalidate();
        //  the coarse-box mirror moves in the SAME tick, exactly as the mount's
        // does: combat and presentation both read it, and a captive whose damage
        // box stayed where it was grabbed could be hit in mid-air by a strike
        // aimed at the floor.
        if let Some(mut aabb) = aabb {
            aabb.center = pos;
        }
    }
}

/// A body ALREADY in somebody's hands, put back after it moved.
///
/// Runs after the bodies integrate, for the reason the mount's own note gives:
/// integration has just moved the captive under its own velocity, and this is
/// the external constraint that gets the last word.
///
/// ⛔ THIS IS NOT THE SAME OPERATION AS [`finalize_new_capture_pose`], and for
/// a long time it was the same *system* registered twice — once here and once
/// after acquisition — with the scheduler asked to hold two indistinguishable
/// instances of one function and idempotence asked to make that mean something.
/// It also made the name unusable as an ordering subject: Bevy refuses to order
/// against a `SystemTypeSet` carrying more than one instance, so every host
/// that installed both panicked on schedule build.
pub fn maintain_existing_capture_pose(
    captors: Query<
        (
            &ae::BodyKinematics,
            Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        ),
        Without<CapturedBy>,
    >,
    captives: Query<(
        &CapturedBy,
        &mut ae::BodyKinematics,
        &mut ae::ActorSurfaceState,
        &mut ambition_platformer2d_core::BodyGroundState,
        Option<&mut crate::components::CenteredAabb>,
    )>,
) {
    pose_captives(captors, captives);
}

/// A body caught THIS TICK, given its pose on the tick it was caught.
///
/// Runs at the end of the capture loop — after acquisition creates the
/// relationship, after the pummel that lands with it, and after the throw, so a
/// body that has just LEFT a hold is not snapped back into it.
///
/// ⭐ Without this the maintenance pass above is the first thing to pose a new
/// captive, and it already ran earlier in this tick: a body grabbed now would
/// hang where it stood until the next frame, one visible frame of a captive
/// standing free inside somebody's grab animation.
pub fn finalize_new_capture_pose(
    captors: Query<
        (
            &ae::BodyKinematics,
            Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        ),
        Without<CapturedBy>,
    >,
    captives: Query<(
        &CapturedBy,
        &mut ae::BodyKinematics,
        &mut ae::ActorSurfaceState,
        &mut ambition_platformer2d_core::BodyGroundState,
        Option<&mut crate::components::CenteredAabb>,
    )>,
) {
    pose_captives(captors, captives);
}

/// A captor is restricted, not blanked.
///
/// While holding somebody a body may not walk, jump, dodge, shield, blink,
/// special, or shoot — but it MUST still be able to feed an attack press and its
/// direction, because that is how a pummel and a throw are chosen.
///
///  deliberately not `ScriptedControl`. That marker means *"normal input
/// does not drive this body"*, and it is what the CAPTIVE gets. A captor still
/// has meaningful agency; it is in a restricted action context, which is a
/// different thing and stays a different thing.
///
///  a future carry relationship may permit locomotion — cargo throws exist in
/// the genre. That is a property of the RELATIONSHIP, so it belongs on whatever
/// component expresses it, not baked in here.
pub fn restrict_captor_control(
    captives: Query<&CapturedBy>,
    mut captors: Query<(Entity, &mut ambition_characters::control::ActorControl)>,
) {
    let holding: std::collections::HashSet<Entity> =
        captives.iter().map(|held| held.captor).collect();
    if holding.is_empty() {
        return;
    }
    for (entity, mut control) in &mut captors {
        if !holding.contains(&entity) {
            continue;
        }
        let frame = &mut control.0;
        frame.locomotion = ae::LocalAxes::ZERO;
        frame.velocity_target = ae::WorldVec2(ae::Vec2::ZERO);
        frame.jump_pressed = false;
        frame.jump_held = false;
        frame.jump_released = false;
        frame.burst_pressed = false;
        frame.blink_pressed = false;
        frame.blink_held = false;
        frame.blink_released = false;
        frame.special_pressed = false;
        frame.shield_held = false;
        frame.fire = None;
        frame.projectile_pressed = false;
        frame.projectile_held = false;
        frame.projectile_released = false;
        frame.fly_toggle_pressed = false;
        frame.fast_fall_pressed = false;
        frame.interact_pressed = false;
        //  a second grab while already holding somebody is not a thing.
        frame.grab_pressed = false;
        //  PRESERVED: `melee_pressed`, `melee_held`, `melee_released` and
        // `attack_axis`. Those four ARE the capture-context vocabulary — strip
        // them and a captor can hold a body and do nothing to it forever.
    }
}

/// THE one release. Every path out of a capture goes through it.
///
/// Making that impossible is the entire reason this is a function rather than three lines
/// repeated at each exit.
///
///  this is also what makes escape cheap. Mash-to-escape, when it lands,
/// is another caller: it decides WHEN, and this decides what release means.
pub fn release_capture(
    commands: &mut Commands,
    victim: Entity,
    held: &CapturedBy,
    surface: Option<&mut ae::ActorSurfaceState>,
    ground: Option<&mut ambition_platformer2d_core::BodyGroundState>,
    holds: Option<&mut ambition_characters::control::ControlHolds>,
) {
    commands.entity(victim).try_remove::<CapturedBy>();
    //  ONLY the hold this relationship claimed. The body may be held by a
    // ruleset's KO freeze at the same time — the throw that ends the grab does
    // not end the fight's hold on the fighter it just threw.
    ambition_characters::control::release_control_hold(
        commands,
        victim,
        holds,
        ambition_characters::control::ControlHold::Relationship,
    );
    if let Some(surface) = surface {
        //  what it WAS, not `1.0`. A flying body's scale is not the reference
        // one, and a release that wrote a constant would land it on the floor.
        surface.gravity_scale = held.prior_gravity_scale;
    }
    // ⭐⭐ **A RELEASED BODY RE-ENTERS PLAY AIRBORNE**, and `apply_capture_throws`
    // depends on it in writing: *"a thrown body is AIRBORNE by construction — the
    // hold suspended its gravity and the release hands it back"*.
    //
    // ⛔⛔ that used to hold BY ACCIDENT. The hold wrote `on_ground = false` and
    // left the contact baseline standing, so a released body inherited "airborne"
    // as a believed fact. The hold now `invalidate()`s instead — it must, or the
    // kernel manufactures a landing every tick — and an invalidated baseline is
    // re-sampled at the hold POSE, which is at the captor's own height and
    // therefore usually ON THE FLOOR. A throw off a grounded captor came out
    // GROUNDED, and a ground launch is not a launch.
    //
    // So the fact the throw needs is stated where the body re-enters play, rather
    // than left as a residue of how the hold spelled its constraint. Its landing
    // is then a REAL one — a single transition when it comes down, which is the
    // whole difference from the per-tick loop.
    if let Some(ground) = ground {
        ground.on_ground = false;
        ground.contact_initialized = true;
    }
}

/// A capture ends when either body takes a real hit, or the captor stops
/// existing.
///
/// The interruption is the hit reaction the engine already agreed happened.
///
///  hitstop alone does NOT break it. A pummel may deliberately produce a
/// little hitstop while preserving the hold; breaking on that would make the
/// mechanic destroy itself on its own second beat.
///
///  EXISTENCE IS ASKED OF THE WORLD, NEVER INFERRED FROM A COMPONENT. This read
/// `combat.get(captor).is_err()` and called that "the captor is gone". It is not: it is "the
/// captor has no combat state", and the two answers differ for every body that is alive without
/// one. `bodies` answers only the existence question, and answers nothing else.
pub fn release_interrupted_captures(
    mut commands: Commands,
    captives: Query<(Entity, &CapturedBy)>,
    // Existence, and nothing but existence: a despawned entity matches no
    // query, which is precisely the fact wanted and the only one taken.
    bodies: Query<Entity>,
    combat: Query<&ambition_characters::actor::BodyCombat>,
    mut surfaces: Query<&mut ae::ActorSurfaceState>,
    mut grounds: Query<&mut ambition_platformer2d_core::BodyGroundState>,
    mut holds: Query<&mut ambition_characters::control::ControlHolds>,
) {
    let reacted = |body: Entity| {
        combat
            .get(body)
            .map(|c| c.hitstun_timer > 0.0 || c.recoil_lock_timer > 0.0)
            .unwrap_or(false)
    };
    for (victim, held) in &captives {
        // A captor that no longer exists cannot hold anything.
        let captor_gone = bodies.get(held.captor).is_err();
        if !(captor_gone || reacted(victim) || reacted(held.captor)) {
            continue;
        }
        let mut surface = surfaces.get_mut(victim).ok();
        let mut ground = grounds.get_mut(victim).ok();
        let mut held_by = holds.get_mut(victim).ok();
        release_capture(
            &mut commands,
            victim,
            held,
            surface.as_deref_mut(),
            ground.as_deref_mut(),
            held_by.as_deref_mut(),
        );
    }
}

/// Apply pummel damage directly to the body already held by the captor.
///
/// Capture already resolved targeting and defense, so pummels do not reacquire a
/// target, consult shields/i-frames, or arm knockback, hitstun, or recoil. Health
/// damage still uses the canonical `BodyHealth::damage` path.
pub fn apply_capture_pummels(
    mut requests: MessageReader<crate::capture::CapturePummelRequested>,
    mut captives: Query<(
        &CapturedBy,
        &mut ambition_characters::smash_capture::SmashHoldState,
        &mut ambition_characters::actor::BodyHealth,
    )>,
) {
    for request in requests.read() {
        // The captor names itself; the victim comes from the relationship. A
        // pummel cannot miss, because acquisition already decided who it hits.
        let Some((held, mut state, mut health)) = captives
            .iter_mut()
            .find(|(held, _, _)| held.captor == request.captor)
        else {
            continue;
        };
        let _ = held;
        health.damage(request.damage);
        // Saturating: a hold long enough to overflow a u8 has other problems,
        // and wrapping to zero would tell a CPU policy the hold just started.
        //
        //  the count is the RULESET's, not the relation's: "how many pummels"
        // is a question only a game with pummels can ask.
        state.pummels_landed = state.pummels_landed.saturating_add(1);
    }
}

/// Apply an authored throw in dependency order: damage, release, then launch.
/// Damage contributes to the launch's percent scaling; release must happen before
/// hitstun is armed. The launch uses the ordinary `HitKnockback` reaction path so
/// throws inherit weight, DI, gravity, carried momentum, hitstun, and hitstop.
pub fn apply_capture_throws(
    mut commands: Commands,
    mut requests: MessageReader<crate::capture::CaptureThrowRequested>,
    captors: Query<&ae::BodyKinematics, Without<CapturedBy>>,
    mut captives: Query<(
        Entity,
        &CapturedBy,
        &mut ae::BodyKinematics,
        &mut ae::BodyFlightState,
        &mut ambition_characters::actor::BodyCombat,
        &mut ambition_characters::actor::BodyHealth,
        &mut ae::ActorSurfaceState,
        &mut ambition_platformer2d_core::BodyGroundState,
        Option<&crate::components::CombatTuning>,
        Option<&mut ambition_characters::control::ControlHolds>,
        // ⭐ THE AIR BUDGET A THROW GIVES BACK. A thrown fighter is airborne by
        // construction and has to recover from where the captor put it, so it
        // owes the same refresh any other hit does — see `AirBudget` and D203.
        &mut ae::BodyDodgeState,
    )>,
    gravity: Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    feel: Option<Res<crate::feel::Platformer2dFeelTuningMonolith>>,
) {
    let feel = feel.map(|f| *f).unwrap_or_default();
    for request in requests.read() {
        let Some((
            victim,
            held,
            mut kin,
            mut flight,
            mut combat,
            mut health,
            mut surface,
            mut ground,
            tuning,
            mut holds,
            mut dodge,
        )) = captives
            .iter_mut()
            .find(|(_, held, ..)| held.captor == request.captor)
        else {
            continue;
        };
        let Ok(captor_kin) = captors.get(request.captor) else {
            continue;
        };

        // 1. Damage first: the meter this throw adds counts toward its own
        //    launch, which is what makes a throw at high percent a kill move.
        health.damage(request.damage);

        // 2. The hold ends. Through the ONE release, so gravity and the control
        //    projection come back with it.
        let held = *held;
        release_capture(
            &mut commands,
            victim,
            &held,
            Some(&mut surface),
            Some(&mut ground),
            holds.as_deref_mut(),
        );

        // 3. An ordinary launch on a body that is now ordinary.
        let weight = tuning.map(|t| t.weight).unwrap_or(1.0);
        let magnitude = crate::util::scaled_knockback(
            request.knockback,
            request.knockback_growth,
            health.damage_taken(),
            weight,
        );
        let knockback = ae::hit_response::HitKnockback {
            // A throw is a hit: it stuns.
            reaction: ae::hit_response::HitReaction::Strike,
            // The captor's facing decides which way "forward" points, exactly as
            // it does for the hold anchor — a throw follows the hands.
            dir: captor_kin.facing,
            magnitude: ae::hit_response::HitKnockbackMagnitude::LaunchSpeed(magnitude),
            source_pos: captor_kin.pos,
            impact_pos: kin.pos,
            launch_dir: Some(request.launch_dir),
            follow: None,
        };
        let gravity_dir = gravity
            .get(victim)
            .map(|frame| frame.down())
            .unwrap_or(ae::DEFAULT_GRAVITY_DIR);
        let pos = kin.pos;
        let facing = kin.facing;
        crate::hit_reaction::apply_body_hit_reaction(
            &mut kin.vel,
            &mut flight,
            &mut combat,
            pos,
            facing,
            gravity_dir,
            false,
            Some(&knockback),
            // The throw's own damage, which is the term the freeze is computed
            // from — the same value applied to the meter above.
            request.damage,
            //  no DI on a throw's release frame: the captive had no control to
            // hold. Smash DI on throws is a real mechanic and it belongs with
            // the escape work, where a captive's restricted input channel exists.
            ae::Vec2::ZERO,
            //  a thrown body is AIRBORNE by construction — the hold suspended
            // its gravity and the release hands it back — so a downward throw is
            // eligible for the meteor lock exactly like a spike is.  and it is
            // not crouching: a captive has no stance of its own, and letting a
            // thrown body crouch-cancel its own throw would refund the captor
            // the only beat a grab is paid for.
            Default::default(),
            // ⛔ THE JUMP IS NOT THE THROW'S TO GIVE — the CATCH already gave
            // it, at `acquire_captures`. A throw hands over what an ordinary hit
            // hands over, and nothing more.
            Some(&mut dodge),
            // ⛔ NO JUMP STATE THREADED HERE, and the reason is the same as the
            // comment above: the CATCH already re-seated this body. A captive
            // cannot be mid-recovery, so there is no helpless episode for the
            // throw to end.
            None,
            // A captive is not holding an edge: the hold suspended its gravity
            // and pinned it to the captor.
            None,
            feel,
        );
    }
}
