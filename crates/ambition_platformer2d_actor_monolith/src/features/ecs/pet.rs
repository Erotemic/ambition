//! Petting: Interact next to a pettable character pets it.
//!
//! The gesture is two timers on `BodyAnimFacts`: the petter's
//! (`pet_anim_timer`) and the petted body's (`petted_anim_timer`). Everything
//! else is derived from them. [`project_gesture_holds`] holds both bodies still
//! while they run, and `body_state_clip` draws the two rows. A pet that runs
//! out lets go by itself, and a rollback restores it with the timers.
//!
//! Whether a character can be petted is authored on its catalog row
//! (`petting`), so Interact still talks to everyone who has no such row.

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::BodyAnimFacts;
use ambition_characters::control::{
    claim_control_hold, release_control_hold, ControlHold, ControlHolds,
};
use ambition_combat::components::{ActorDisposition, ActorInteraction, CenteredAabb};
use ambition_platformer2d_core::{AabbExt, BodyKinematics};
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_sfx::{SfxId, SfxMessage, SfxWriter};
use ambition_vfx::vfx::VfxMessage;
use bevy::prelude::*;

/// How long a pet lasts. The petter's `pet` row is authored to this length.
pub const PET_SECONDS: f32 = 2.0;

/// The gap between the petter and the petted body's side, in world units.
/// The petter stands just off the petted body's front, where its bowed head is.
const PET_STANDOFF: f32 = 6.0;

/// Start a pet when a driven body's Interact reaches a pettable character.
///
/// Runs before doors and dialogue read the press, and consumes it, so a
/// pettable character is petted instead of talked to. A door the petter also
/// overlaps keeps the press when it is the nearer of the two: the companion
/// dog roams a basement with a door every few steps, and a robot standing on a
/// door means the door. The petter turns to face the petted body, which turns
/// to face it back, and steps to stand at its front.
#[allow(clippy::too_many_arguments)]
pub fn pet_pettable_characters(
    driven: ambition_held_items::DrivenBodies,
    mut acting: crate::control::ActingParticipant,
    primary: Query<
        Entity,
        (
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
    >,
    catalog: Res<CharacterCatalog>,
    mut bodies: Query<(&mut BodyKinematics, &mut BodyAnimFacts)>,
    pettable: Query<
        (
            Entity,
            &CenteredAabb,
            &ActorDisposition,
            &ActorInteraction,
            Option<&ambition_characters::actor::BodyHealth>,
            (
                Has<ambition_combat::death_rules::OutOfPlay>,
                Option<&ambition_platformer2d_core::DepthPlane>,
            ),
        ),
        With<FeatureSimEntity>,
    >,
    // `Option`: a fixture with no session world has no doors either.
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
    >,
    mut sfx: SfxWriter,
    mut vfx: MessageWriter<VfxMessage>,
) {
    let mut subjects = driven.entities();
    if subjects.is_empty() {
        subjects.extend(primary.iter().next());
    }
    for subject in subjects {
        if !acting.buffered_interact(subject) {
            continue;
        }
        let Ok((kin, anim)) = bodies.get(subject) else {
            continue;
        };
        if anim.in_shared_gesture() {
            continue;
        }
        let reach = kin.aabb();
        let found = pettable.iter().find_map(
            |(entity, aabb, disposition, interaction, health, (out_of_play, plane))| {
                if entity == subject
                    || disposition.is_hostile()
                    || ambition_combat::util::body_is_untouchable(health, out_of_play, plane)
                    || !aabb.aabb().strict_intersects(reach)
                {
                    return None;
                }
                let character = ambition_conversation::character_id_of(&interaction.interactable)?;
                let petting = catalog.get(character)?.petting.as_ref()?;
                Some((entity, *aabb, petting.sound.clone()))
            },
        );
        let Some((petted, aabb, sound)) = found else {
            continue;
        };
        let center = |b: &ambition_platformer2d_core::Aabb| (b.min + b.max) * 0.5;
        let from_petter = |at: ambition_platformer2d_core::Vec2| at.distance(kin.pos);
        let nearer_door = room_set.as_deref().is_some_and(|rooms| {
            rooms.active_loading_zones().iter().any(|zone| {
                matches!(
                    zone.activation,
                    ambition_platformer2d_world::rooms::LoadingZoneActivation::Door
                ) && zone.aabb.strict_intersects(reach)
                    && from_petter(center(&zone.aabb)) < from_petter(aabb.center)
            })
        });
        if nearer_door {
            continue;
        }
        let Ok([(mut petter_kin, mut petter_anim), (mut petted_kin, mut petted_anim)]) =
            bodies.get_many_mut([subject, petted])
        else {
            continue;
        };
        if petted_anim.in_shared_gesture() {
            continue;
        }
        acting.consume_interact(subject);
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
        if let Some(sound) = sound {
            sfx.write(SfxMessage::Play {
                id: SfxId::new(&sound),
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
