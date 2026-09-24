//! Gate-portal presentation: sprite visibility, animation row, and ring spin,
//! driven by the sim's `GatePortalPhases` and keyed by the zone ids in the
//! authored `GatePortalRegistry`. These render systems match props by the
//! render-local [`PropVisual::name`].

use bevy::prelude::*;

use super::primitives::{LoadingZoneVisual, PortalSprite, PropVisual};
use ambition_sprite_sheet::character::{CharacterAnim, CharacterAnimator};
use ambition_time::PresentationTime;
use ambition_platformer2d_world::rooms::{GatePortalPhase, GatePortalPhases, GatePortalRegistry};

/// Hide the debug door-zone visual that `spawn_loading_zone` spawns for a
/// LoadingZone registered as a portal. The gate sprites are the visual.
///
/// Runs each frame, so visuals re-spawned after a room reload are hidden
/// too.
pub fn hide_portal_loading_zone_visuals(
    portals: Res<GatePortalRegistry>,
    mut visuals: Query<(&LoadingZoneVisual, &mut Visibility)>,
) {
    for (visual, mut vis) in &mut visuals {
        if portals.is_portal(&visual.id) && *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}

/// Hide the portal sprite while its phase is `Off`; show it
/// otherwise. Matches the prop entity by [`PropVisual::name`] against
/// `GatePortalConfig::portal_sprite_name`.
pub fn sync_portal_sprite_visibility(
    mut commands: Commands,
    portals: Res<GatePortalRegistry>,
    phases: Res<GatePortalPhases>,
    mut sprites: Query<(Entity, &PropVisual, &mut Visibility, Option<&PortalSprite>)>,
) {
    for (zone_id, config) in portals.iter() {
        let target_visibility = if phases.phase(zone_id).portal_sprite_visible() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        for (entity, prop, mut vis, marker) in &mut sprites {
            if prop.name != config.portal_sprite_name {
                continue;
            }
            if marker.is_none() {
                // `try_insert`: these are room-scoped prop visuals, and room
                // teardown can despawn them before this deferred write lands.
                // Covered by `deferred_write_safety::production_passes`.
                commands.entity(entity).try_insert(PortalSprite);
            }
            if *vis != target_visibility {
                *vis = target_visibility;
            }
        }
    }
}

/// Angular velocity (rad/s) of the gate ring during the portal's `Opening`
/// phase. 8 rad/s is about 1.27 revolutions/s: readable, not disorienting.
const RING_OPENING_SPIN_RAD_PER_SEC: f32 = 8.0;

/// Drive gate-portal animation from its phase using the row mapping in
/// `GATE_PORTAL_SHEET`. `PortalSprite` excludes these entities from generic
/// character/prop animation, so this system exclusively owns their animator and
/// atlas frame.
pub fn sync_portal_sprite_animation(
    presentation_time: PresentationTime,
    portals: Res<GatePortalRegistry>,
    phases: Res<GatePortalPhases>,
    mut sprites: Query<(&PropVisual, &mut Sprite, &mut CharacterAnimator)>,
) {
    let dt = presentation_time.scaled_dt();
    for (zone_id, config) in portals.iter() {
        let target_anim = match phases.phase(zone_id) {
            GatePortalPhase::Off => continue,
            GatePortalPhase::Opening { .. } => CharacterAnim::Idle,
            GatePortalPhase::On => CharacterAnim::Walk,
            GatePortalPhase::Closing { .. } => CharacterAnim::Run,
        };
        for (prop, mut sprite, mut animator) in &mut sprites {
            if prop.name != config.portal_sprite_name {
                continue;
            }
            animator.request(target_anim);
            let index = animator.tick(dt);
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = index;
            }
        }
    }
}

/// Rotate the gate ring during the portal's `Opening` phase, so the boot
/// sequence reads as the ring spinning up. During `On`, `Off`, and `Closing`
/// the ring is at rotation 0 and plays the idle animation.
pub fn sync_portal_ring_rotation_system(
    mut commands: Commands,
    presentation_time: PresentationTime,
    portals: Res<GatePortalRegistry>,
    phases: Res<GatePortalPhases>,
    mut rings: Query<(
        Entity,
        &PropVisual,
        &mut Transform,
        &mut Sprite,
        &mut CharacterAnimator,
        Option<&PortalSprite>,
    )>,
) {
    // Scaled dt, so the spin slows in bullet time and stops on pause, like
    // the phase timer.
    let dt = presentation_time.scaled_dt();
    for (zone_id, config) in portals.iter() {
        let phase = phases.phase(zone_id);
        let spinning = matches!(phase, GatePortalPhase::Opening { .. });
        // Sheet mapping (see GATE_RING_SHEET):
        // - Idle = the slow always-on row (8f × 140ms)
        // - Walk = the fast `spin` row used during Opening (12f × 85ms)
        let target_anim = if spinning {
            CharacterAnim::Walk
        } else {
            CharacterAnim::Idle
        };
        for (entity, prop, mut tf, mut sprite, mut animator, marker) in &mut rings {
            if prop.name != config.ring_sprite_name {
                continue;
            }
            if marker.is_none() {
                // `try_insert`: these are room-scoped prop visuals, and room
                // teardown can despawn them before this deferred write lands.
                // Covered by `deferred_write_safety::production_passes`.
                commands.entity(entity).try_insert(PortalSprite);
            }
            animator.request(target_anim);
            let index = animator.tick(dt);
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = index;
            }
            if spinning {
                tf.rotate_local_z(RING_OPENING_SPIN_RAD_PER_SEC * dt);
            } else if !matches!(phase, GatePortalPhase::Closing { .. }) {
                // Snap upright when Off or On: only the boot beat shows the ring
                // rotated. Closing keeps the last rotation until the phase
                // reaches Off.
                if tf.rotation != bevy::math::Quat::IDENTITY {
                    tf.rotation = bevy::math::Quat::IDENTITY;
                }
            }
        }
    }
}
