//! Dormancy policy for Ambition-authored actors.
//!
//! Scripted/externally driven actors use [`DormancyPolicy::Never`]. Roaming AI may use
//! [`DormancyPolicy::AwakeNearObservers`] with [`AMBITION_WAKE_RADIUS`], chosen to give
//! sufficient lead time for the protagonist's fastest traversal.

use bevy::prelude::*;

use ambition_characters::actor::limb::Limb;
use ambition_mount::Mountable;
use ambition_platformer2d_actor_monolith::features::ecs::dormancy::DormancyPolicy;
use ambition_platformer2d_actor_monolith::features::{
    ActorFaction, BodyKinematics, EncounterMob, FeatureId,
};

/// Wake radius for roaming Ambition hostiles.
///
/// This is derived from the controlled body's fastest repeated traversal
/// (blink), not ordinary run speed, with roughly 2.4 seconds of lead time. The
/// radius is observer-motion policy and should not be tuned down merely to make
/// dormancy trigger inside today's room sizes.
pub const AMBITION_WAKE_RADIUS: f32 = 2560.0;

/// True when another body may author this body's `ActorControl`.
///
/// Limbs and mounts never sleep because dormancy retracts `ActorControl`, which
/// would conflict with the external driver. Mounts use this stance even while
/// currently unridden because boarding is dynamic.
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

    // The duel is an always-running exhibition and must not pause with observer distance.
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
