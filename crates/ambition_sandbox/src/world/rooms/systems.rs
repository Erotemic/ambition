use bevy::prelude::{
    Commands, Component, Entity, Query, Res, ResMut, Sprite, Transform, Visibility,
};

use super::{
    tick_gate_portal_phase, ActiveRoomMetadata, GatePortalPhase, GatePortalRegistry,
    RoomMusicRequest, RoomSet,
};
use crate::presentation::character_sprites::{CharacterAnim, CharacterAnimator};
use crate::WorldTime;

/// Tag on the portal + gate-ring visual entities so the generic
/// `animate_characters` system skips them. Without this filter the
/// generic NPC animator re-pins them to `Idle` every frame and
/// clobbers the row that the portal-presentation systems request
/// based on `GatePortalPhase`. The portal/ring systems own these
/// entities' animator request, frame tick, and atlas index.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalSprite;

/// Mirror `RoomSet::active_metadata()` into the `ActiveRoomMetadata`
/// resource, but only when the metadata actually changes. The
/// PartialEq guard means change-detection consumers (e.g. a future
/// room-music selector) only fire when the active room's biome /
/// music_track / ambient / theme really differ — not on every frame.
pub fn sync_active_room_metadata(room_set: Res<RoomSet>, mut active: ResMut<ActiveRoomMetadata>) {
    let current = room_set.active_metadata().clone();
    if current != active.0 {
        active.0 = current;
    }
}

/// Push the active room's `music_track` into `RoomMusicRequest` so the
/// audio system knows the room-default track when no encounter
/// override is active. Empty values clear the request, falling back to
/// `sandbox_data.audio.default_music_track`.
pub fn sync_room_music_request(
    active: Res<ActiveRoomMetadata>,
    mut request: ResMut<RoomMusicRequest>,
) {
    let next = active.0.music_track.clone();
    if next != request.desired_track {
        request.desired_track = next;
    }
}

/// Advance every registered portal's phase based on its controlling
/// switch's state + the per-phase timer. Pure state update — sprite
/// visibility + ring rotation are downstream presentation systems.
///
/// The switch's "true / false" state is what tells the portal what
/// it *should* be doing (boot or shutdown); the portal still runs
/// its own one-shot Opening / Closing animations between Off and
/// On, so the traversal check (only `On` allows it) remains stable
/// even when the switch flickers.
pub fn tick_portal_phases_system(
    world_time: Res<WorldTime>,
    save: Res<crate::persistence::save::SandboxSave>,
    mut portals: ResMut<GatePortalRegistry>,
) {
    // Scaled dt — pause / hitstop / bullet-time naturally freezes
    // or slows the portal boot/shutdown sequence so the ring spin
    // and one-shot anims stay in sync with everything else.
    let dt = world_time.scaled_dt;
    if dt <= 0.0 {
        return;
    }
    for config in portals.portals.values_mut() {
        let switch_on = save.data().switch(&config.switch_id);
        tick_gate_portal_phase(&mut config.phase, switch_on, dt);
    }
}

/// Hide the debug door-zone visual that `spawn_loading_zone`
/// spawns for any LoadingZone that's registered as a portal — the
/// portal's gate sprites ARE the visual, the door box behind them
/// is redundant.
///
/// Runs each frame so re-spawned visuals (after a room reload)
/// also get hidden.
pub fn hide_portal_loading_zone_visuals(
    portals: Res<GatePortalRegistry>,
    mut visuals: Query<(
        &crate::presentation::rendering::LoadingZoneVisual,
        &mut Visibility,
    )>,
) {
    for (visual, mut vis) in &mut visuals {
        if portals.is_portal(&visual.id) && *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}

/// Hide the portal sprite while its phase is `Off`; show it
/// otherwise. Matches entity by `FeatureName` against
/// `GatePortalConfig::portal_sprite_name`.
///
/// Presentation-only — gated by `cfg(feature = "ui")` callers via
/// the Bevy plugin registration in `app/plugins.rs`. Headless skips
/// the registration entirely.
pub fn sync_portal_sprite_visibility(
    mut commands: Commands,
    portals: Res<GatePortalRegistry>,
    mut sprites: Query<(
        Entity,
        &crate::features::FeatureName,
        &mut Visibility,
        Option<&PortalSprite>,
    )>,
) {
    for config in portals.portals.values() {
        let target_visibility = if config.phase.portal_sprite_visible() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        for (entity, name, mut vis, marker) in &mut sprites {
            if name.0 != config.portal_sprite_name {
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

/// Rotate the gate ring during the portal's `Opening` phase so the
/// boot sequence reads as "the ring spins up to bring the portal
/// online." During `On`, `Off`, and `Closing` the ring sits at
/// rotation 0 and its sprite plays the idle animation.
///
/// Per-frame angular velocity (radians/sec). 8 rad/s ≈ 1.27
/// revolutions/s — fast enough to read, slow enough not to
/// disorient. Tuneable if the boot beat lengthens.
const RING_OPENING_SPIN_RAD_PER_SEC: f32 = 8.0;

/// Drive the portal sprite's animation row from its phase. Borrows
/// existing CharacterAnim variants as semantic slots — see
/// `GATE_PORTAL_SHEET` for the mapping:
/// - Phase::Opening → request(Idle)  [row 0 = opening one-shot]
/// - Phase::On      → request(Walk)  [row 1 = stable loop]
/// - Phase::Closing → request(Run)   [row 2 = closing one-shot]
/// - Phase::Off     → sprite is hidden; no work to do
///
/// Matches the portal sprite entity by `FeatureName`. The portal's
/// `PortalSprite` marker excludes it from `animate_characters`, so
/// this system is the sole owner of the portal entity's animator
/// state — it also ticks the animator and writes the resulting
/// frame into the sprite atlas, matching what `animate_characters`
/// does for non-portal sprites.
pub fn sync_portal_sprite_animation(
    world_time: Res<WorldTime>,
    portals: Res<GatePortalRegistry>,
    mut sprites: Query<(
        &crate::features::FeatureName,
        &mut Sprite,
        &mut CharacterAnimator,
    )>,
) {
    let dt = world_time.scaled_dt;
    for config in portals.portals.values() {
        let target_anim = match config.phase {
            GatePortalPhase::Off => continue,
            GatePortalPhase::Opening { .. } => CharacterAnim::Idle,
            GatePortalPhase::On => CharacterAnim::Walk,
            GatePortalPhase::Closing { .. } => CharacterAnim::Run,
        };
        for (name, mut sprite, mut animator) in &mut sprites {
            if name.0 != config.portal_sprite_name {
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

pub fn sync_portal_ring_rotation_system(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    portals: Res<GatePortalRegistry>,
    mut rings: Query<(
        Entity,
        &crate::features::FeatureName,
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
        for (entity, name, mut tf, mut sprite, mut animator, marker) in &mut rings {
            if name.0 != config.ring_sprite_name {
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
