//! GNU-ton's third phase goes to space: as his giant lifts off, the dark of
//! space and its stars come over his hall, the Earth falls away under it,
//! and the Moon is up in the corner. Space is authored art in three
//! pictures (the renderer's `gnu_ton_space` target), posed here; no shader
//! draws it.
//!
//! It reads the conductor's record (`Conductor::ascent`) and decides
//! nothing: how far the sky is in space is the simulation's, so a rewind
//! takes the look back with the record. The Moon drifts on the presentation
//! clock, which is a look only.

use bevy::{
    prelude::*,
    sprite::{Anchor, SpriteImageMode},
};

use ambition_boss_encounter::BossConfig;
use ambition_extension_host::BodyRecords;
use ambition_platformer2d_core::config::world_to_bevy;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, InRoomInstance, SessionScopeSet, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use ambition_render::rendering::RoomVisual;

use crate::bosses::gnu_ton::{ascent_of, GNU_TON_ID};

/// The three pictures (`tools/ambition_sprite2d_renderer`, target
/// `gnu_ton_space`).
const STARS_ART: &str = "sprites/gnu_ton_space/gnu_ton_space_stars.png";
const EARTH_ART: &str = "sprites/gnu_ton_space/gnu_ton_space_earth.png";
const MOON_ART: &str = "sprites/gnu_ton_space/gnu_ton_space_moon.png";
/// Over the parallax panels (`-18.0..=-15.0`) and the room look's backdrop
/// (`-14.5`), under the blocks: the parts are between this and 0.2 over it.
const SPACE_Z: f32 = -14.4;
/// The Earth's picture is four times as wide as it is high, and the top of
/// its limb is this part of the way down it.
const EARTH_ASPECT: f32 = 4.0;
const EARTH_LIMB: f32 = 0.58;

/// The root of a room's space: at the room's corner, so a part's place is in
/// room coordinates with y turned up.
#[derive(Component, Debug, Clone, Copy)]
pub struct GnuTonSky;

/// One picture of the space.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GnuTonSkyPart {
    Stars,
    Earth,
    Moon,
}

impl GnuTonSkyPart {
    const ALL: [GnuTonSkyPart; 3] = [GnuTonSkyPart::Stars, GnuTonSkyPart::Earth, GnuTonSkyPart::Moon];

    fn art(self) -> &'static str {
        match self {
            Self::Stars => STARS_ART,
            Self::Earth => EARTH_ART,
            Self::Moon => MOON_ART,
        }
    }

    fn depth(self) -> f32 {
        match self {
            Self::Stars => 0.0,
            Self::Earth => 0.1,
            Self::Moon => 0.05,
        }
    }
}

/// Where a part is (its middle, in room coordinates, +y down), its size and
/// its alpha, at `ascent` (0 on the ground, 1 in space) in a room of `room`
/// size, `time_s` seconds into the presentation clock. `None`: not shown.
pub fn pose_of(part: GnuTonSkyPart, ascent: f32, room: Vec2, time_s: f32) -> Option<(Vec2, Vec2, f32)> {
    if ascent <= 0.0 {
        return None;
    }
    let a = ascent.clamp(0.0, 1.0);
    match part {
        // The dark comes over all the room as the hall climbs.
        GnuTonSkyPart::Stars => Some((room * 0.5, room, a)),
        // The Earth is big and near at the liftoff, and falls away under
        // the hall: smaller, and lower.
        GnuTonSkyPart::Earth => {
            let width = room.x * (1.8 - 0.6 * a);
            let height = width / EARTH_ASPECT;
            // The top of its limb (`EARTH_LIMB` of the picture down) goes
            // from the middle of the room to near its floor.
            let limb = room.y * (0.5 + 0.38 * a);
            let top = limb - height * EARTH_LIMB;
            Some((Vec2::new(room.x * 0.5, top + height * 0.5), Vec2::new(width, height), (a * 3.0).min(1.0)))
        }
        // The Moon, high in a corner, drifts a little.
        GnuTonSkyPart::Moon => {
            let size = room.y * 0.16;
            let drift = (time_s * 0.05).sin() * room.x * 0.02;
            Some((Vec2::new(room.x * 0.82 + drift, room.y * 0.18), Vec2::splat(size), a))
        }
    }
}

pub fn install(app: &mut App) {
    app.add_systems(
        Update,
        present_gnu_ton_space
            .in_set(SessionScopeSet::Presentation)
            .run_if(resource_exists::<AssetServer>)
            .run_if(any_with_component::<BossConfig>.or_else(any_with_component::<GnuTonSky>)),
    );
}

/// Show and pose the space of each live room whose GNU-ton is in it.
#[allow(clippy::type_complexity)]
pub fn present_gnu_ton_space(
    mut commands: Commands,
    time: ambition_time::PresentationTime,
    mut elapsed: Local<f32>,
    rooms: LiveRoomSpecs,
    scholars: Query<(&BossConfig, &BodyRecords, Option<&InRoomInstance>), Without<GnuTonSky>>,
    mut skies: Query<(Entity, &InRoomInstance, &mut Visibility), With<GnuTonSky>>,
    mut parts: Query<(&GnuTonSkyPart, &ChildOf, &mut Transform, &mut Sprite, &mut Visibility), Without<GnuTonSky>>,
    asset_server: Res<AssetServer>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    *elapsed = (*elapsed + time.scaled_dt()).rem_euclid(3600.0);
    let Some(session_scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        return;
    };
    let mut posed: Vec<(Entity, f32, Vec2)> = Vec::new();
    for (room, definition) in rooms.live_rooms() {
        let ascent = scholars
            .iter()
            .filter(|(config, _, stamp)| config.behavior.id == GNU_TON_ID && stamp.is_none_or(|stamp| stamp.0 == room))
            .find_map(|(_, records, _)| ascent_of(records))
            .filter(|ascent| *ascent > 0.0);
        let sky = skies.iter_mut().find(|(_, stamp, _)| stamp.0 == room);
        let Some(ascent) = ascent else {
            if let Some((_, _, mut visibility)) = sky {
                visibility.set_if_neq(Visibility::Hidden);
            }
            continue;
        };
        let world = &rooms.rooms().spec(definition).world;
        let size = Vec2::new(world.size.x, world.size.y);
        match sky {
            Some((root, _, mut visibility)) => {
                visibility.set_if_neq(Visibility::Visible);
                posed.push((root, ascent, size));
            }
            None => {
                let root = commands
                    .spawn_session_scoped(
                        session_scope.in_room(Some(room)),
                        (
                            Transform::from_translation(world_to_bevy(world, world.size * 0.0, SPACE_Z)),
                            Visibility::Visible,
                            Name::new("GNU-ton: space"),
                            RoomVisual,
                            GnuTonSky,
                        ),
                    )
                    .id();
                for part in GnuTonSkyPart::ALL {
                    let image_mode = match part {
                        GnuTonSkyPart::Stars => SpriteImageMode::Tiled { tile_x: true, tile_y: true, stretch_value: 1.0 },
                        _ => SpriteImageMode::Auto,
                    };
                    commands.spawn((
                        Sprite { image: asset_server.load(part.art()), image_mode, ..default() },
                        Anchor::CENTER,
                        Transform::from_xyz(0.0, 0.0, part.depth()),
                        Visibility::Hidden,
                        part,
                        ChildOf(root),
                    ));
                }
            }
        }
    }
    for (part, parent, mut transform, mut sprite, mut visibility) in &mut parts {
        let Some(&(_, ascent, size)) = posed.iter().find(|(root, ..)| *root == parent.parent()) else {
            continue;
        };
        match pose_of(*part, ascent, size, *elapsed) {
            None => {
                visibility.set_if_neq(Visibility::Hidden);
            }
            Some((centre, extent, alpha)) => {
                transform.set_if_neq(Transform::from_xyz(centre.x, -centre.y, part.depth()));
                // A tiled sprite cuts its tiles again each time it changes:
                // it is written only when its size or its alpha does.
                let (extent, colour) = (Some(extent), Color::srgba(1.0, 1.0, 1.0, alpha));
                if sprite.custom_size != extent || sprite.color != colour {
                    sprite.custom_size = extent;
                    sprite.color = colour;
                }
                visibility.set_if_neq(Visibility::Inherited);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOM: Vec2 = Vec2::new(1920.0, 1280.0);

    /// On the ground there is no space; in space the dark covers the room,
    /// and the Earth is under the hall, smaller than at the liftoff.
    #[test]
    fn the_sky_goes_to_space_as_the_hall_climbs() {
        for part in GnuTonSkyPart::ALL {
            assert_eq!(pose_of(part, 0.0, ROOM, 1.0), None);
        }
        let (centre, size, alpha) = pose_of(GnuTonSkyPart::Stars, 1.0, ROOM, 1.0).unwrap();
        assert_eq!((centre, size, alpha), (ROOM * 0.5, ROOM, 1.0));
        let (low, near, _) = pose_of(GnuTonSkyPart::Earth, 0.1, ROOM, 1.0).unwrap();
        let (high, far, _) = pose_of(GnuTonSkyPart::Earth, 1.0, ROOM, 1.0).unwrap();
        assert!(far.x < near.x && high.y > low.y, "the Earth falls away: {near:?} {far:?}");
        let limb = high.y - far.y * 0.5 + far.y * EARTH_LIMB;
        assert!(limb > ROOM.y * 0.8 && limb < ROOM.y, "in space the Earth's limb is near the floor: {limb}");
    }
}
