//! Dormancy policy for Ambition-authored actors.
//!
//! Scripted/externally driven actors use [`DormancyPolicy::Never`]. Roaming AI may use
//! [`DormancyPolicy::AwakeNearObservers`] with [`AMBITION_WAKE_RADIUS`], chosen to give
//! sufficient lead time for the protagonist's fastest traversal.

use bevy::prelude::*;

use ambition_characters::actor::limb::Limb;
use ambition_platformer2d_actor_monolith::features::ecs::dormancy::DormancyPolicy;
use ambition_platformer2d_actor_monolith::features::{
    ActorFaction, BodyKinematics, EncounterMob, FeatureId, Mountable,
};

/// How near an observer has to be for one of Ambition's roaming hostiles to
/// keep thinking.
///
/// NOT Mary-O's 720 and NOT Sanic's 4800, and not this game's run speed
/// either. A wake radius is a lead time wearing distance's clothes: it must be
/// long enough that an actor is already moving by the time an observer can see
/// it, so it is `observer top speed × lead time`, and the mistake to avoid is
/// taking "top speed" to mean the number in the run tuning.
///
/// Ambition's protagonist BLINKS. `MAX_RUN_SPEED` is 270 px/s, `DASH_SPEED` is
/// 760 — but `BLINK_DISTANCE` is 190 px on a `BLINK_COOLDOWN` of 0.180 s, so a
/// chaining player closes ≈1056 px/s, nearly four times the run and well
/// past the dash. Deriving from 270 would have produced a 648 px radius and
/// enemies snapping into motion in full view, which is worse than one that was
/// walking all along.
///
/// So: the same 2.4 s of lead the other two games use, against 1056 px/s  2534,
/// rounded to 2560.
///
/// and it mostly does not fire today, which is stated rather than tuned
/// away. Ambition is authored as discrete rooms reached through loading
/// zones, and only the active room's contents are staged; the largest
/// enemy-bearing room (`scroll_lab`, 3200×900) has a 3324 px diagonal and most
/// are under 1500. So an enemy sleeps only in the far corner of `scroll_lab`,
/// `vertical_shaft` or `square_arena`. Shrinking the number until it fired more
/// often would break the property it exists to hold — the radius is a fact about
/// the OBSERVER, not about how big a room happens to be — and it becomes live
/// the day content authors a long level.
pub const AMBITION_WAKE_RADIUS: f32 = 2560.0;

/// Whether this body's `ActorControl` may be written by something other than
/// its own brain: a LIMB, which the host's rig fans intents onto every tick, or
/// a MOUNT, which a rider — up to and including the PLAYER (ADR 0020,
/// `ControlGrant::Total`) — may board and steer.
///
/// Neither may sleep. Going dormant RETRACTS `ActorControl` on the sleep
/// transition — the engine seam clears the frame because a body goes on
/// integrating the brain's last word — while the driver writes that same
/// component every tick, so a dozing hand or a dozing ridden mount is two
/// authorities over one frame. GNU-ton's giant and its two hands are the live
/// always-driven case.
///
/// a mount declares `Never` whether or not it is carrying a rider today.
/// Boarding is a runtime event and a stance is declared once at spawn, so
/// "rideable" is the only form of the question a declaration can answer — and
/// getting it wrong in the other direction means a body the player is riding
/// having its control frame retracted underneath them. That covers the cove's
/// four flying sharks, which roam unridden and can also be boarded.
fn is_driven_by_another_body(is_mount: bool, is_limb: bool) -> bool {
    is_mount || is_limb
}

/// The stance for one staged actor — the call site where each choice is
/// justified. `None` means "not a candidate", never "undecided".
pub fn stance_for(
    faction: ActorFaction,
    feature_id: Option<&str>,
    is_encounter_mob: bool,
    is_mount: bool,
    is_limb: bool,
) -> Option<DormancyPolicy> {
    // The observer itself, and the prop-like bodies that act like actors only
    // for hit detection. Neither has a brain to sleep; the wake test is defined
    // relative to the first of them.
    if matches!(faction, ActorFaction::Player | ActorFaction::Neutral) {
        return None;
    }

    // The duel is the one content pair that genuinely fights under its own
    // decisions and still must never sleep. The exhibition IS the simulation:
    // `register_duel_content_staging` stages the fight as part of room
    // construction so the pair is already battling the instant the player walks
    // in, and `<<duel>>` stages it beside the player anywhere. A fight that only
    // starts once you are close enough is not the exhibit.
    if matches!(
        feature_id,
        Some(crate::duel_arena::DUEL_PCA_ID) | Some(crate::duel_arena::DUEL_ROBOT_ID)
    ) {
        return Some(DormancyPolicy::Never);
    }

    // The wave timeline is the fight. An encounter mob exists only while its
    // encounter is running, in the arena the player is standing in, and the
    // reducer — not a distance — decides when it appears and when it is done.
    if is_encounter_mob {
        return Some(DormancyPolicy::Never);
    }

    // A mount or a limb: driven, so it has no decision of its own to suspend.
    if is_driven_by_another_body(is_mount, is_limb) {
        return Some(DormancyPolicy::Never);
    }

    match faction {
        // The boss's phase machine is the fight — the exact case the engine's
        // doc names for `Never`. Two further facts make silence here especially
        // misleading: a boss already HAS a wake concept of its own
        // (`BossEncounterPhase::Dormant` → Intro, driven by the encounter, not by
        // a distance), and the brain tick that honours `Dormant` explicitly
        // excludes bosses (`Without<BossConfig>`) — so a radius on a boss would
        // be a rule that reads as enforced and is not.
        ActorFaction::Boss => Some(DormancyPolicy::Never),

        // The placed peaceful cast.
        //
        // and 144 of them are the Hall, which is a stress test as much as
        // an exhibition: *"Eventually we are going to give all those characters
        // normal brains"*. `Never` is the declaration that keeps that true —
        // handing the Hall a wake radius would quietly delete the load it exists
        // to apply, which is the "cap it, it's only a debug room" shortcut
        // `docs/concepts/hall-of-characters-is-not-special.md` rejects. When the
        // Hall is slow, the answer is an engine fix, not fewer thinking
        // characters.
        ActorFaction::Npc => Some(DormancyPolicy::Never),

        ActorFaction::Enemy => Some(DormancyPolicy::AwakeNearObservers {
            radius: AMBITION_WAKE_RADIUS,
        }),

        ActorFaction::Player | ActorFaction::Neutral => None,
    }
}

/// Declare a stance on every actor this crate stages that does not carry one.
///
/// A tagging pass rather than a field on the spawn request, for the reason
/// Mary-O gives: `SpawnActorRequest` is the ENGINE's vocabulary for what an
/// actor IS, and dormancy is a decision the CONTENT makes about it. It also
/// means an actor staged by any path — LDtk placement, registered room staging,
/// an encounter wave, a mount's limb rig — is covered by the same statement.
///
/// `With<BodyKinematics>` because that is what the engine's `assess_dormancy`
/// reads: a policy on a body it cannot locate would be a declaration nothing
/// consults.
pub fn declare_ambition_dormancy(
    mut commands: Commands,
    fresh: Query<
        (
            Entity,
            &ActorFaction,
            Option<&FeatureId>,
            Has<EncounterMob>,
            Has<Mountable>,
            Has<Limb>,
        ),
        (With<BodyKinematics>, Without<DormancyPolicy>),
    >,
) {
    for (entity, faction, feature_id, is_encounter_mob, is_mount, is_limb) in &fresh {
        if let Some(policy) = stance_for(
            *faction,
            feature_id.map(|id| id.as_str()),
            is_encounter_mob,
            is_mount,
            is_limb,
        ) {
            commands.entity(entity).try_insert(policy);
        }
    }
}

/// Register the declaration pass: in `WorldPrep`, before the engine decides who
/// is awake, so an actor's stance is in place on the first tick it exists.
pub fn register(app: &mut App) {
    use ambition_platformer2d_shared_tangle::schedule::{
        Platformer2dSimulationPhaseMonolith, SimScheduleExt,
    };
    let sim = app.sim_schedule();
    app.add_systems(
        sim,
        declare_ambition_dormancy
            .in_set(Platformer2dSimulationPhaseMonolith::WorldPrep)
            .before(ambition_platformer2d_actor_monolith::features::ecs::dormancy::assess_dormancy),
    );
}
