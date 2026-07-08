//! Gate-portal presentation: sprite visibility / animation row / ring spin
//! driven by the sim's `GatePortalRegistry` phase (E4 slices 10+20 — these
//! systems used to live INSIDE the sim crate and matched render entities by
//! a render-inserted sim `FeatureName`; now they are render systems matching
//! the render-local [`PropVisual::name`]).

use bevy::prelude::*;

use super::primitives::{LoadingZoneVisual, PortalSprite, PropVisual};
use ambition_world::rooms::{GatePortalPhase, GatePortalRegistry};
use ambition_sprite_sheet::character::{CharacterAnim, CharacterAnimator};
use ambition_time::WorldTime;

/// Hide the debug door-zone visual that `spawn_loading_zone`
/// spawns for any LoadingZone that's registered as a portal — the
/// portal's gate sprites ARE the visual, the door box behind them
/// is redundant.
///
/// Runs each frame so re-spawned visuals (after a room reload)
/// also get hidden.
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
    mut sprites: Query<(Entity, &PropVisual, &mut Visibility, Option<&PortalSprite>)>,
) {
    for config in portals.portals.values() {
        let target_visibility = if config.phase.portal_sprite_visible() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        for (entity, prop, mut vis, marker) in &mut sprites {
            if prop.name != config.portal_sprite_name {
                continue;
            }
            if marker.is_none() {
                commands.entity(entity).insert(PortalSprite);
            }
            if *vis != target_visibility {
                *vis = target_visibility;
            }
        }
    }
}

/// Per-frame angular velocity (radians/sec) of the gate ring during the
/// portal's `Opening` phase. 8 rad/s ≈ 1.27 revolutions/s — fast enough to
/// read, slow enough not to disorient. Tuneable if the boot beat lengthens.
const RING_OPENING_SPIN_RAD_PER_SEC: f32 = 8.0;

/// Drive the portal sprite's animation row from its phase. Borrows
/// existing CharacterAnim variants as semantic slots — see
/// `GATE_PORTAL_SHEET` for the mapping:
/// - Phase::Opening → request(Idle)  [row 0 = opening one-shot]
/// - Phase::On      → request(Walk)  [row 1 = stable loop]
/// - Phase::Closing → request(Run)   [row 2 = closing one-shot]
/// - Phase::Off     → sprite is hidden; no work to do
///
/// The portal's `PortalSprite` marker excludes it from
/// `animate_characters`/`animate_props`, so this system is the sole owner
/// of the portal entity's animator state — it also ticks the animator and
/// writes the resulting frame into the sprite atlas.
pub fn sync_portal_sprite_animation(
    world_time: Res<WorldTime>,
    portals: Res<GatePortalRegistry>,
    mut sprites: Query<(&PropVisual, &mut Sprite, &mut CharacterAnimator)>,
) {
    let dt = world_time.scaled_dt;
    for config in portals.portals.values() {
        let target_anim = match config.phase {
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

/// Rotate the gate ring during the portal's `Opening` phase so the boot
/// sequence reads as "the ring spins up to bring the portal online."
/// During `On`, `Off`, and `Closing` the ring sits at rotation 0 and its
/// sprite plays the idle animation.
pub fn sync_portal_ring_rotation_system(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    portals: Res<GatePortalRegistry>,
    mut rings: Query<(
        Entity,
        &PropVisual,
        &mut Transform,
        &mut Sprite,
        &mut CharacterAnimator,
        Option<&PortalSprite>,
    )>,
) {
    // Use scaled dt so the boot-spin slows during bullet time and
    // freezes during pause — same world-clock the phase timer reads.
    let dt = world_time.scaled_dt;
    for config in portals.portals.values() {
        let spinning = matches!(config.phase, GatePortalPhase::Opening { .. });
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
                commands.entity(entity).insert(PortalSprite);
            }
            animator.request(target_anim);
            let index = animator.tick(dt);
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = index;
            }
            if spinning {
                tf.rotate_local_z(RING_OPENING_SPIN_RAD_PER_SEC * dt);
            } else if !matches!(config.phase, GatePortalPhase::Closing { .. }) {
                // Snap back to upright when fully Off or On — the
                // boot beat is the only time the ring should look
                // physically rotated. Closing keeps the last
                // rotation (it'll get reset when phase reaches Off).
                if tf.rotation != bevy::math::Quat::IDENTITY {
                    tf.rotation = bevy::math::Quat::IDENTITY;
                }
            }
        }
    }
}
