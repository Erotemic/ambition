//! Emit movement feedback for every integrated body.

use bevy::prelude::*;
use ambition_platformer2d_actor::body::{
    BodyFrameOutput, arm_ground_contact_anim_overlay, arm_movement_anim_overlays, emit_movement_fx,
};
use ambition_platformer2d_core::BodyKinematics;
use ambition_characters::actor::{BodyAnimFacts, BodyCombat};
use ambition_platformer2d_shared_tangle::{
    camera_ease::{CameraShakeRequest, PlayerBlinkCameraState, hard_fall_shake_amplitude},
    lifecycle::InRoomInstance,
    markers::PrimaryPlayer,
};
use ambition_sfx::{BodyPresentationSource, SfxWriter};
use ambition_vfx::vfx::VfxWriter;

/// Publish each body's effects once after its movement step.
pub(crate) fn emit_body_movement_feedback(
    mut sfx: SfxWriter,
    mut vfx: VfxWriter,
    // A request, not the live shake: this runs in the simulation, and a landing
    // a correction erases must not have moved the camera.
    mut shake: MessageWriter<CameraShakeRequest>,
    bodies: Query<(Entity, &BodyKinematics, &BodyFrameOutput), Changed<BodyFrameOutput>>,
    mut recipients: Query<(
        Option<&mut BodyAnimFacts>,
        Option<&mut BodyCombat>,
        Option<&mut PlayerBlinkCameraState>,
        Option<&BodyPresentationSource>,
        Option<&InRoomInstance>,
        Has<PrimaryPlayer>,
    )>,
) {
    for (entity, kin, output) in &bodies {
        if output.reset.is_some() { continue; }
        let events = &output.events;
        let transition = events.ground_contact;
        let landing_speed = transition.landing_impact_speed();
        let changed_ground = transition != ambition_platformer2d_core::GroundContactTransition::Unchanged;
        let movement_fx = !events.operations.is_empty() || !events.blinks.is_empty()
            || landing_speed.is_some()
            || events.contacts.iter().any(|contact| contact.kind == ambition_platformer2d_core::collision_semantics::ContactKind::Side);
        let hazard = events.reset.is_some_and(ambition_platformer2d_core::ResetCause::is_hazardous);
        if !(movement_fx || changed_ground || hazard) {
            continue;
        }
        let Ok((anim, combat, blink_camera, source, room, followed)) = recipients.get_mut(entity) else {
            continue;
        };
        if movement_fx {
            emit_movement_fx(&mut sfx, &mut vfx.for_room(room.map(|room| room.0)),
                events, kin.pos, kin.facing, kin.size(), source.map(BodyPresentationSource::id));
        }
        if let Some(mut anim) = anim {
            if changed_ground {
                arm_ground_contact_anim_overlay(&mut anim, transition);
            }
            if !events.operations.is_empty() {
                arm_movement_anim_overlays(&mut anim, events);
            }
        }
        if let Some(mut blink_camera) = blink_camera {
            for blink in &events.blinks {
                blink_camera.blink_in_duration = 0.34;
                blink_camera.blink_in_timer = blink_camera.blink_in_duration;
                blink_camera.blink_camera_from = blink.from;
            }
        }
        if let Some(mut combat) = combat {
            if hazard {
                combat.hit_flash = 0.12;
            }
        }
        if followed && landing_speed.is_some() {
            let amplitude_px = hard_fall_shake_amplitude(landing_speed);
            if amplitude_px > 0.0 { shake.write(CameraShakeRequest { amplitude_px, room: room.map(|room| room.0) }); }
        }
    }
}
