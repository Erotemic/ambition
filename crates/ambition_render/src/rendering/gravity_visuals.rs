//! Gravity-zone visuals (visible build only, registered by the presentation
//! rendering plugin). These show a gravity mechanic, not a portal, so they
//! must not depend on portal mechanics.

use bevy::prelude::*;

use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d_core::{self as ae};
use ambition_platformer2d_shared_tangle::gravity::GravityZone;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomInstance, SessionScopeId, SessionSpawnScope, SpawnSessionScopedExt,
};

use super::retained_entities::RetainedEntities;

/// Marks the visual for a [`GravityZone`].
#[derive(Component)]
#[require(crate::rendering::WorldSprite)]
pub struct GravityZoneVisual;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum ZoneSprite {
    Fill,
    DirectionBand,
}

type ZoneDrawKey = (Entity, LiveRoomInstance, Option<SessionScopeId>, ZoneSprite);

/// The hue of a zone that pulls along `dir`, by the axis it pulls along most:
/// up violet, down teal, left amber, right green. Each way its own, so two
/// side fields that pull against each other are told apart at a glance (Jon,
/// 2026-10-30: "left and right gravity don't have a different color, and
/// that would help"). (pure)
pub fn zone_hue(dir: ae::Vec2) -> Color {
    if dir.x.abs() > dir.y.abs() {
        if dir.x < 0.0 {
            Color::srgb(0.98, 0.66, 0.24) // left = amber
        } else {
            Color::srgb(0.46, 0.88, 0.36) // right = green
        }
    } else if dir.y < 0.0 {
        Color::srgb(0.62, 0.40, 0.95) // up = violet
    } else {
        Color::srgb(0.30, 0.80, 0.80) // down = teal
    }
}

/// Run condition of [`sync_gravity_zone_visual`]: a gravity zone stands, or
/// a visual of one remains to retire.
pub fn a_zone_or_its_visual_stands(
    zones: Query<(), With<GravityZone>>,
    visuals: Query<(), With<GravityZoneVisual>>,
) -> bool {
    !zones.is_empty() || !visuals.is_empty()
}

/// Draw each gravity zone as a translucent tinted region so the player can see
/// where gravity changes, in the hue of the way it pulls ([`zone_hue`]).
///
/// Keep the same drawables when a zone moves or changes size.
pub(super) fn sync_gravity_zone_visual(
    mut commands: Commands,
    // Each zone is drawn in its own live room, by that room's geometry.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<RoomGeometry>,
    active_session: Option<Res<ActiveSessionScope>>,
    mut visuals: Query<(&mut Transform, &mut Sprite), With<GravityZoneVisual>>,
    zones: Query<(Entity, &GravityZone)>,
    mut retained: Local<RetainedEntities<ZoneDrawKey>>,
) {
    let scope = SessionSpawnScope::for_optional_active_session(active_session.as_deref());
    retained.reconcile(&mut commands, |(zone, room, session, _)| {
        scope.is_some_and(|scope| scope.id() == *session)
            && zones.contains(*zone)
            && rooms.room_of(*zone) == Some(*room)
            && rooms.in_room(*room).is_some()
    });
    let Some(session_scope) = scope else {
        return;
    };
    for (entity, zone) in &zones {
        // A zone whose live room cannot be told is not drawn.
        let Some((room, world)) = rooms.room_of(entity).and_then(|room| Some((room, rooms.in_room(room)?))) else {
            continue;
        };
        let hue = zone_hue(zone.dir);
        let color = hue.with_alpha(0.16);
        let center = (zone.aabb.min + zone.aabb.max) * 0.5;
        let size = zone.aabb.max - zone.aabb.min;
        let translation = ambition_platformer2d_core::config::world_to_bevy(&world.0, center, 7.5);
        // A brighter band on the edge gravity pulls toward, so the zone shows a
        // direction: you can see which way you will fall before stepping in.
        let band_color = hue.with_alpha(0.55);
        let half_along = (size.x * zone.dir.x.abs() + size.y * zone.dir.y.abs()) * 0.5;
        let thickness = 10.0_f32.min(half_along * 0.8);
        let band_center = center + zone.dir * (half_along - thickness * 0.5);
        let band_size = ae::Vec2::new(
            if zone.dir.x != 0.0 { thickness } else { size.x },
            if zone.dir.y != 0.0 { thickness } else { size.y },
        );
        let band_translation =
            ambition_platformer2d_core::config::world_to_bevy(&world.0, band_center, 7.6);
        for (piece, color, size, at, name) in [
            (ZoneSprite::Fill, color, size, translation, "Gravity zone visual"),
            (ZoneSprite::DirectionBand, band_color, band_size, band_translation, "Gravity zone direction band"),
        ] {
            let key = (entity, room, session_scope.id(), piece);
            if let Some((mut transform, mut sprite)) =
                retained.get(&key).and_then(|id| visuals.get_mut(*id).ok())
            {
                transform.set_if_neq(Transform::from_translation(at));
                if sprite.color != color || sprite.custom_size != Some(size) {
                    sprite.color = color;
                    sprite.custom_size = Some(size);
                }
            } else {
                let drawable = commands.spawn_session_scoped(
                    session_scope.in_room(Some(room)),
                    (
                        GravityZoneVisual,
                        Sprite::from_color(color, size),
                        Transform::from_translation(at),
                        Name::new(name),
                    ),
                ).id();
                retained.insert(key, drawable);
            }
        }
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

    /// Each of the four ways a zone pulls has its own hue.
    #[test]
    fn each_way_a_zone_pulls_has_its_own_hue() {
        let hues: Vec<[f32; 4]> = [(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)]
            .into_iter()
            .map(|(x, y)| zone_hue(ae::Vec2::new(x, y)).to_srgba().to_f32_array())
            .collect();
        for (i, a) in hues.iter().enumerate() {
            for b in &hues[i + 1..] {
                assert_ne!(a, b, "two ways a zone pulls share a hue");
            }
        }
    }

    /// Move a zone while its drawables remain in use.
    #[test]
    fn a_moving_zone_keeps_its_visuals() {
        let world = ae::World::new("gravity room", ae::Vec2::new(800.0, 600.0), ae::Vec2::new(40.0, 40.0), Vec::new());
        let mut app = App::new();
        insert_live_room_component(app.world_mut(), RoomGeometry(world));
        let zone = app.world_mut().spawn((
            GravityZone {
                aabb: ambition_platformer2d_core::Aabb::new(ae::Vec2::new(100.0, 200.0), ae::Vec2::new(20.0, 20.0)),
                dir: ae::Vec2::new(0.0, -1.0),
            },
            InRoomInstance(LiveRoomInstance::ACTIVATION),
        )).id();
        app.add_systems(Update, sync_gravity_zone_visual);
        let visuals = |app: &mut App| {
            let world = app.world_mut();
            let mut q = world.query_filtered::<Entity, With<GravityZoneVisual>>();
            let mut found: Vec<Entity> = q.iter(world).collect();
            found.sort();
            found
        };
        app.update();
        let first = visuals(&mut app);
        assert_eq!(first.len(), 2, "a zone draws a fill and a band");
        let old = app.world().get::<Transform>(first[0]).unwrap().translation;
        {
            let mut zone = app.world_mut().get_mut::<GravityZone>(zone).unwrap();
            zone.aabb.min.x += 16.0;
            zone.aabb.max.x += 16.0;
        }
        app.update();
        assert_eq!(visuals(&mut app), first);
        assert_eq!(app.world().get::<Transform>(first[0]).unwrap().translation, old + Vec3::X * 16.0);
    }
}
