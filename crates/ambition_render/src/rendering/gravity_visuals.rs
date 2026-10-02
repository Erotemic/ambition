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
    // Each zone is drawn in its own live room, by that room's geometry.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<RoomGeometry>,
    active_session: Option<Res<ActiveSessionScope>>,
    visuals: Query<Entity, With<GravityZoneVisual>>,
    zones: Query<(Entity, &GravityZone)>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    for (entity, zone) in &zones {
        // A zone whose live room cannot be told is not drawn.
        let Some((room, world)) = rooms.room_of(entity).and_then(|room| Some((room, rooms.in_room(room)?))) else {
            continue;
        };
        let session_scope = session_scope.in_room(Some(room));
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

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_live_room_component, spawn_live_room, InRoomInstance, LiveRoomInstance,
    };

    /// Each gravity zone is drawn in its own live room (view half, cut V2h).
    /// Two live rooms of different sizes, a zone in each at one simulation
    /// position: each zone's visuals carry its room's stamp and that room's
    /// position. A zone whose room cannot be told is not drawn.
    #[test]
    fn each_gravity_zone_is_drawn_in_its_own_live_room() {
        let world_of = |size: ae::Vec2| ae::World::new("gravity room", size, ae::Vec2::new(40.0, 40.0), Vec::new());
        let (big, small) = (ae::Vec2::new(800.0, 600.0), ae::Vec2::new(400.0, 300.0));
        let mut app = App::new();
        insert_live_room_component(app.world_mut(), RoomGeometry(world_of(big)));
        let second = LiveRoomInstance::ACTIVATION.next();
        spawn_live_room(app.world_mut(), second, RoomGeometry(world_of(small)));
        let zone = || GravityZone {
            aabb: ambition_platformer2d_core::Aabb::new(ae::Vec2::new(100.0, 200.0), ae::Vec2::new(20.0, 20.0)),
            dir: ae::Vec2::new(0.0, 1.0),
        };
        app.world_mut().spawn((zone(), InRoomInstance(LiveRoomInstance::ACTIVATION)));
        app.world_mut().spawn((zone(), InRoomInstance(second)));
        app.add_systems(Update, sync_gravity_zone_visual);
        app.update();

        let world = app.world_mut();
        let mut q = world.query_filtered::<(&Transform, Option<&InRoomInstance>, &Name), With<GravityZoneVisual>>();
        let mut drawn: Vec<(Option<u32>, (i32, i32))> = q
            .iter(world)
            .filter(|(_, _, name)| name.as_str() == "Gravity zone visual")
            .map(|(transform, stamp, _)| {
                (
                    stamp.map(|stamp| stamp.0.ordinal()),
                    (transform.translation.x as i32, transform.translation.y as i32),
                )
            })
            .collect();
        drawn.sort();
        let flipped = |size: ae::Vec2| ((100.0 - size.x * 0.5) as i32, (size.y * 0.5 - 200.0) as i32);
        assert_eq!(
            drawn,
            vec![(Some(LiveRoomInstance::ACTIVATION.ordinal()), flipped(big)), (Some(second.ordinal()), flipped(small))],
            "(room, position) of each zone visual: each must be placed by its zone's live room and stamped with it"
        );
    }
}
