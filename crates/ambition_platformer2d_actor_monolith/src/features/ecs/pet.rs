//! Petting: a conversation's `<<pet>>` pets the character being talked to.
//!
//! The gesture is two timers on `BodyAnimFacts`: the petter's
//! (`pet_anim_timer`) and the petted body's (`petted_anim_timer`). Everything
//! else is derived from them. [`project_gesture_holds`] holds both bodies still
//! while they run, and `body_state_clip` draws the two rows. A pet that runs
//! out lets go by itself, and a rollback restores it with the timers.
//!
//! Whether a character can be petted is authored on its catalog row
//! (`petting`). The dialogue offers the pet as a choice, so Interact always
//! talks and the pet does not take the conversation's place.

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::BodyAnimFacts;
use ambition_characters::control::{
    claim_control_hold, release_control_hold, ControlHold, ControlHolds,
};
use ambition_combat::components::{ActorInteraction, CenteredAabb};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use ambition_sfx::{SfxId, SfxMessage, SfxWriter};
use ambition_vfx::vfx::VfxMessage;
use bevy::prelude::*;

/// How long a pet lasts. The petter's `pet` row is authored to this length.
pub const PET_SECONDS: f32 = 2.0;

/// The gap between the petter and the petted body's side, in world units.
/// The petter stands just off the petted body's front, where its bowed head is.
const PET_STANDOFF: f32 = 6.0;

/// A conversation asked one body to pet another, by stable identity.
///
/// Routed by `SimId` for the reason [`crate::features::ChallengeRequested`] is:
/// the narrative ledger releases it again on every replay of its tick, and an
/// `Entity` is not stable across that.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct PetRequested {
    /// The body that pets: the one who started the conversation.
    pub petter: SimId,
    /// The body that is petted: the one being talked to.
    pub petted: SimId,
}

/// Start the pet a conversation asked for. (sim)
///
/// The petted body's catalog row must author `petting`, because only such a
/// character has a petted animation row and a sound for it. The petter turns
/// to face the petted body, which turns to face it back, and the petter steps
/// to stand at its front.
pub fn apply_pet_requests(
    mut requests: MessageReader<PetRequested>,
    ids: Query<(Entity, &SimId)>,
    catalog: Res<CharacterCatalog>,
    pettable: Query<(&CenteredAabb, &ActorInteraction)>,
    mut bodies: Query<(&mut BodyKinematics, &mut BodyAnimFacts)>,
    mut sfx: SfxWriter,
    mut vfx: MessageWriter<VfxMessage>,
) {
    for request in requests.read() {
        let entity_of = |wanted: &SimId| {
            ids.iter()
                .find(|(_, id)| *id == wanted)
                .map(|(entity, _)| entity)
        };
        let (Some(petter), Some(petted)) = (entity_of(&request.petter), entity_of(&request.petted))
        else {
            warn!(
                target: "crate::features::pet",
                "a pet of {} by {} names no live body; ignoring",
                request.petted,
                request.petter,
            );
            continue;
        };
        let Ok((aabb, interaction)) = pettable.get(petted) else {
            warn!(
                target: "crate::features::pet",
                "{} is not a character that can be talked to; ignoring the pet",
                request.petted,
            );
            continue;
        };
        let Some(petting) = ambition_conversation::character_id_of(&interaction.interactable)
            .and_then(|character| catalog.get(character))
            .and_then(|row| row.petting.as_ref())
        else {
            warn!(
                target: "crate::features::pet",
                "{} has no `petting` on its catalog row; ignoring the pet",
                request.petted,
            );
            continue;
        };
        let Ok([(mut petter_kin, mut petter_anim), (mut petted_kin, mut petted_anim)]) =
            bodies.get_many_mut([petter, petted])
        else {
            continue;
        };
        if petter_anim.in_shared_gesture() || petted_anim.in_shared_gesture() {
            continue;
        }
        // The side of the petted body the petter is on; it faces that way, and
        // the petter faces back across it.
        let side = if petter_kin.pos.x >= aabb.center.x {
            1.0
        } else {
            -1.0
        };
        petted_kin.facing = side;
        petter_kin.facing = -side;
        petter_kin.pos.x =
            aabb.center.x + side * (aabb.half_size.x + petter_kin.size.x * 0.5 + PET_STANDOFF);
        // Both stop where they stand: the hold blanks their control from the
        // next tick, and a walking dog would otherwise slide out from under
        // the hand on its momentum.
        petter_kin.vel.x = 0.0;
        petted_kin.vel.x = 0.0;
        petter_anim.pet_anim_timer = PET_SECONDS;
        petted_anim.petted_anim_timer = PET_SECONDS;
        if let Some(sound) = &petting.sound {
            sfx.write(SfxMessage::Play {
                id: SfxId::new(sound),
                pos: aabb.center,
            });
        }
        vfx.write(VfxMessage::Hearts {
            pos: aabb.center
                + ambition_platformer2d_core::Vec2::new(side * aabb.half_size.x, -aabb.half_size.y),
            count: 5,
        });
    }
}

/// Hold every body still while a shared gesture plays on it, and let go when
/// it ends. Derived from the gesture timers each tick, so it is idempotent and
/// a rollback that restores the timers restores the hold with them.
pub fn project_gesture_holds(
    mut commands: Commands,
    mut bodies: Query<(Entity, &BodyAnimFacts, Option<&mut ControlHolds>)>,
) {
    for (entity, anim, holds) in &mut bodies {
        let held = holds
            .as_ref()
            .is_some_and(|holds| holds.holds(ControlHold::Gesture));
        match (anim.in_shared_gesture(), held) {
            (true, false) => claim_control_hold(&mut commands, entity, ControlHold::Gesture),
            (false, true) => release_control_hold(
                &mut commands,
                entity,
                holds.map(|holds| holds.into_inner()),
                ControlHold::Gesture,
            ),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
