//! Gravity-zone visuals (visible build only, registered by the presentation
//! rendering plugin). These show a gravity mechanic, not a portal, so they
//! must not depend on portal mechanics.

use bevy::prelude::*;

use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d_core::{self as ae};
use ambition_platformer2d_shared_tangle::gravity::GravityZone;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};

/// Marks the visual for a [`GravityZone`].
#[derive(Component)]
pub struct GravityZoneVisual;

/// Draw each gravity zone as a translucent tinted region so the player can see
/// where gravity changes (violet = up, teal = down/other).
pub fn sync_gravity_zone_visual(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomGeometry>,
    active_session: Option<Res<ActiveSessionScope>>,
    visuals: Query<Entity, With<GravityZoneVisual>>,
    zones: Query<&GravityZone>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    for zone in &zones {
        let color = if zone.dir.y < 0.0 {
            Color::srgba(0.62, 0.40, 0.95, 0.16) // up = violet
        } else {
            Color::srgba(0.30, 0.80, 0.80, 0.16) // else teal
        };
        let center = (zone.aabb.min + zone.aabb.max) * 0.5;
        let size = zone.aabb.max - zone.aabb.min;
        let translation = ambition_platformer2d_core::config::world_to_bevy(&world.0, center, 7.5);
        commands.spawn_session_scoped(
            session_scope,
            (
                GravityZoneVisual,
                Sprite::from_color(color, size),
                Transform::from_translation(translation),
                Name::new("Gravity zone visual"),
            ),
        );
        // A brighter band on the edge gravity pulls toward, so the zone shows a
        // direction: you can see which way you will fall before stepping in.
        let band_color = if zone.dir.y < 0.0 {
            Color::srgba(0.62, 0.40, 0.95, 0.55) // up = violet
        } else {
            Color::srgba(0.30, 0.80, 0.80, 0.55) // else teal
        };
        let half_along = (size.x * zone.dir.x.abs() + size.y * zone.dir.y.abs()) * 0.5;
        let thickness = 10.0_f32.min(half_along * 0.8);
        let band_center = center + zone.dir * (half_along - thickness * 0.5);
        let band_size = ae::Vec2::new(
            if zone.dir.x != 0.0 { thickness } else { size.x },
            if zone.dir.y != 0.0 { thickness } else { size.y },
        );
        let band_translation =
            ambition_platformer2d_core::config::world_to_bevy(&world.0, band_center, 7.6);
        commands.spawn_session_scoped(
            session_scope,
            (
                GravityZoneVisual,
                Sprite::from_color(band_color, band_size),
                Transform::from_translation(band_translation),
                Name::new("Gravity zone direction band"),
            ),
        );
    }
}
