//! The recall-mark beacon visual (was in abilities/traversal/mark_recall) — a
//! glowing beacon sprite at the player's dropped mark. Render-only; reads the
//! sim-side mark read-model.

use ambition_engine_core as ae;
use ambition_platformer_primitives::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_sim_view::MarkBeaconsView;
use bevy::prelude::*;

/// Marks the persistent beacon sprite shown at a player's dropped recall mark.
#[derive(Component)]
pub struct MarkBeaconVisual;

/// How far above the mark (player center) the beacon's center sits, so it reads
/// as a marker standing UP from the spot rather than buried in the floor.
const BEACON_RISE: f32 = 18.0;
/// In-world display size of the beacon sprite (3:7, matching the rendered prop).
const BEACON_SIZE: ae::Vec2 = ae::Vec2::new(30.0, 70.0);

/// Draw a persistent glowing beacon at each player's dropped recall mark so they
/// can see where `Blink` will recall them to (the mark used to be VFX-only).
/// Clear-and-rebuild each frame — one mark per player, despawns when the mark is
/// cleared. Visible build only.
pub fn sync_mark_beacon_visual(
    mut commands: Commands,
    world: ambition_platformer_primitives::lifecycle::SessionWorldRef<ambition_engine_core::RoomGeometry>,
    asset_server: Res<AssetServer>,
    active_session: Option<Res<ActiveSessionScope>>,
    visuals: Query<Entity, With<MarkBeaconVisual>>,
    marks: Res<MarkBeaconsView>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    for &pos in &marks.0 {
        // +Y is down in world space, so "up" (toward the ceiling) is -Y.
        let translation = ambition_engine_core::config::world_to_bevy(
            &world.0,
            pos - ae::Vec2::new(0.0, BEACON_RISE),
            7.0,
        );
        let mut sprite = Sprite::from_image(asset_server.load("sprites/props/mark_beacon.png"));
        sprite.custom_size = Some(BEACON_SIZE);
        commands.spawn_session_scoped(
            session_scope,
            (
                MarkBeaconVisual,
                sprite,
                Transform::from_translation(translation),
                Name::new("Mark beacon visual"),
            ),
        );
    }
}
