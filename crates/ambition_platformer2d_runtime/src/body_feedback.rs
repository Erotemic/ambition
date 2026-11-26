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
    mut bodies: Query<(
        &BodyKinematics,
        &BodyFrameOutput,
        Option<&mut BodyAnimFacts>,
        Option<&mut BodyCombat>,
        Option<&mut PlayerBlinkCameraState>,
        Option<&BodyPresentationSource>,
        Option<&InRoomInstance>,
        Has<PrimaryPlayer>,
    ), Changed<BodyFrameOutput>>,
) {
    for (kin, output, anim, combat, blink_camera, source, room, followed) in &mut bodies {
        if output.reset.is_some() { continue; }
        let events = &output.events;
        let transition = events.ground_contact;
        if !events.operations.is_empty() || !events.blinks.is_empty()
            || transition.landing_impact_speed().is_some()
            || events.contacts.iter().any(|contact| contact.kind == ambition_platformer2d_core::collision_semantics::ContactKind::Side)
        {
            emit_movement_fx(&mut sfx, &mut vfx.for_room(room.map(|room| room.0)),
                events, kin.pos, kin.facing, kin.size(), source.map(BodyPresentationSource::id));
        }
        if let Some(mut anim) = anim {
            if transition != ambition_platformer2d_core::GroundContactTransition::Unchanged {
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
            if events.reset.is_some_and(ambition_platformer2d_core::ResetCause::is_hazardous) {
                combat.hit_flash = 0.12;
            }
        }
        if followed && transition.landing_impact_speed().is_some() {
            let amplitude_px = hard_fall_shake_amplitude(transition.landing_impact_speed());
            if amplitude_px > 0.0 { shake.write(CameraShakeRequest { amplitude_px, room: room.map(|room| room.0) }); }
        }
    }
}
