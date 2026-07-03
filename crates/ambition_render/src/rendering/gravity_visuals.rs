//! Gravity-zone / gravity-switch visuals (visible build only — registered by
//! the presentation rendering plugin). Extracted from `ambition_gameplay_core::portal::presentation`
//! (Stage 6 follow-up): these visualize a *gravity mechanic*, not a portal, and
//! must not depend on `ambition_gameplay_core::portal`.

use bevy::prelude::*;

use ambition_engine_core::{self as ae};
use ambition_gameplay_core::physics::{GravityField, GravityZone};
use ambition_engine_core::RoomGeometry;

use ambition_gameplay_core::gravity::GravityFlipSwitch;

/// Marks the visual for a [`GravityZone`].
#[derive(Component)]
pub struct GravityZoneVisual;

/// Draw each gravity zone as a translucent tinted region so the player can see
/// where gravity changes (violet = up, teal = down/other).
pub fn sync_gravity_zone_visual(
    mut commands: Commands,
    world: Res<RoomGeometry>,
    visuals: Query<Entity, With<GravityZoneVisual>>,
    zones: Query<&GravityZone>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    for zone in &zones {
        let color = if zone.dir.y < 0.0 {
            Color::srgba(0.62, 0.40, 0.95, 0.16) // up = violet
        } else {
            Color::srgba(0.30, 0.80, 0.80, 0.16) // else teal
        };
        let center = (zone.aabb.min + zone.aabb.max) * 0.5;
        let size = zone.aabb.max - zone.aabb.min;
        let translation = ambition_engine_core::config::world_to_bevy(&world.0, center, 7.5);
        commands.spawn((
            GravityZoneVisual,
            Sprite::from_color(color, size),
            Transform::from_translation(translation),
            Name::new("Gravity zone visual"),
        ));
        // A brighter band on the edge gravity pulls TOWARD (the "down" edge under
        // this zone's gravity), so the zone reads as a DIRECTION — you can see
        // which way you'll fall before stepping in, not just that something
        // changes here.
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
            ambition_engine_core::config::world_to_bevy(&world.0, band_center, 7.6);
        commands.spawn((
            GravityZoneVisual,
            Sprite::from_color(band_color, band_size),
            Transform::from_translation(band_translation),
            Name::new("Gravity zone direction band"),
        ));
    }
}

/// Marks the visual for a [`GravityFlipSwitch`].
#[derive(Component)]
pub struct GravitySwitchVisual;

/// Draw the gravity-flip switch column, tinted green when gravity is normal and
/// orange when it's flipped, so the player can see the current gravity state.
pub fn sync_gravity_switch_visual(
    mut commands: Commands,
    world: Res<RoomGeometry>,
    gravity: Option<Res<GravityField>>,
    visuals: Query<Entity, With<GravitySwitchVisual>>,
    switches: Query<&GravityFlipSwitch>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let flipped = gravity.as_deref().is_some_and(|g| g.dir.y < 0.0);
    let color = if flipped {
        Color::srgba(0.95, 0.55, 0.20, 0.65)
    } else {
        Color::srgba(0.40, 0.90, 0.60, 0.65)
    };
    for sw in &switches {
        let translation = ambition_engine_core::config::world_to_bevy(&world.0, sw.pos, 8.5);
        commands.spawn((
            GravitySwitchVisual,
            Sprite::from_color(color, sw.half_extent * 2.0),
            Transform::from_translation(translation),
            Name::new("Gravity switch visual"),
        ));
    }
}
