//! Emit movement feedback for every integrated body.

use bevy::prelude::*;
use ambition_platformer2d_actor_monolith::body::{
    BodyFrameOutput, arm_ground_contact_anim_overlay, arm_movement_anim_overlays, emit_movement_fx,
};
use ambition_platformer2d_core::BodyKinematics;
use ambition_characters::actor::{BodyAnimFacts, BodyCombat};
use ambition_platformer2d_shared_tangle::{
    camera_ease::{CameraShakeState, CameraShakeTuning, PlayerBlinkCameraState, hard_fall_shake_amplitude},
    lifecycle::InRoomInstance,
    markers::PrimaryPlayer,
};
use ambition_sfx::{BodyPresentationSource, SfxWriter};
use ambition_vfx::vfx::VfxWriter;

/// Publish each body's effects once after its movement step.
pub(crate) fn emit_body_movement_feedback(
    mut sfx: SfxWriter,
    mut vfx: VfxWriter,
    mut shake: Option<ResMut<CameraShakeState>>,
    shake_tuning: Option<Res<CameraShakeTuning>>,
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
        emit_movement_fx(&mut sfx, &mut vfx.for_room(room.map(|room| room.0)),
            events, kin.pos, kin.facing, kin.size, source.map(BodyPresentationSource::id));
        if let Some(mut anim) = anim {
            arm_ground_contact_anim_overlay(&mut anim, events.ground_contact);
            arm_movement_anim_overlays(&mut anim, events);
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
        if followed {
            if let (Some(shake), Some(tuning)) = (shake.as_deref_mut(), shake_tuning.as_deref()) {
                let amplitude = hard_fall_shake_amplitude(events.ground_contact.landing_impact_speed());
                if amplitude > 0.0 { shake.kick(amplitude, *tuning); }
            }
        }
    }
}
