//! The shadow a body throws on the ground under it.
//!
//! A character with no shadow is a picture on top of a room. A soft dark
//! ellipse under its feet puts it in the room, and says how high a body in
//! the air is: the shadow is smaller and fainter the higher the body is, and
//! it is gone at [`REACH`].
//!
//! The shadow is a published picture of the room's theme
//! (`RoomDressingPart::Shadow`) drawn as one sprite for each body: no shader.
//! It is presentation only. It reads where each body is from the read models
//! the renderer has (`BodyPoseView` for the player, `FeatureViewIndex` for
//! each other actor) and the live geometry of the body's room.
//!
//! Limits:
//!
//! - The ground is the top of the nearest solid block or one-way platform
//!   under the middle of the body. A body that stands on a moving platform or
//!   another body has its shadow on the ground under that.
//! - Down is down the screen: a room with another gravity has shadows that
//!   are not under the feet.
//! - A room that names no theme, or a theme with no shadow picture, has none.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_BLOCK};
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomInstance, LiveRoomOf, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use ambition_sprite_sheet::game_assets::{GameAssets, ParallaxTheme, RoomDressingPart};

use super::primitives::{FeatureVisual, RoomVisual};

/// How far over the ground a body still has a shadow, in world units.
pub const REACH: f32 = 150.0;
/// The alpha of the shadow of a body that stands on the ground.
const FULL_ALPHA: f32 = 0.78;
/// The height of the shadow of a body that stands, in world units.
const THICKNESS: f32 = 12.0;
/// The least width of the shadow of a body that stands, in world units.
const LEAST_WIDTH: f32 = 30.0;
/// How far under the top of the ground the middle of the shadow is, in world
/// units: the larger part of it is on the ground, not on the air over it.
const SINK: f32 = 1.5;
/// In front of the terrain and its trims and of a door, behind each actor.
const SHADOW_Z: f32 = WORLD_Z_BLOCK + 7.0;

/// The shadow sprite of one body. `key` says whose: the id of a feature, or
/// [`PLAYER_KEY`].
#[derive(Component, Clone, Debug)]
pub struct GroundShadow {
    pub key: String,
}

const PLAYER_KEY: &str = "\u{0}player";

/// The top of the ground under a body whose feet are at `feet`: the nearest
/// top of a solid block, a blink wall or a one-way platform that is at the
/// feet or below them and under the middle of the body. `half_width` is half
/// the width of the body. `None` when there is none within [`REACH`].
pub fn ground_below(world: &ae::World, feet: ae::Vec2, half_width: f32) -> Option<f32> {
    // The feet of a body that stands are on the top, to a part of a unit.
    const STANDS: f32 = 3.0;
    let reach = half_width * 0.5;
    world
        .blocks
        .iter()
        .filter(|block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
            )
        })
        .filter_map(|block| {
            let (centre, half) = (block.aabb.center(), block.aabb.half_size());
            let top = centre.y - half.y;
            let over = centre.x - half.x < feet.x + reach && centre.x + half.x > feet.x - reach;
            (over && top >= feet.y - STANDS && top - feet.y <= REACH).then_some(top)
        })
        .min_by(|a, b| a.total_cmp(b))
}

/// The size and the alpha of the shadow of a body `width` wide that is
/// `height` over the ground: full when it stands, nothing at [`REACH`].
pub fn shadow_shape(width: f32, height: f32) -> (Vec2, f32) {
    let t = (height / REACH).clamp(0.0, 1.0);
    let scale = 1.0 - 0.5 * t;
    // The box of a body is narrower than its picture: the shadow is wider
    // than the box, and not less than [`LEAST_WIDTH`].
    let full = (width * 1.5).max(LEAST_WIDTH);
    (Vec2::new(full * scale, THICKNESS * scale), FULL_ALPHA * (1.0 - t).powf(1.3))
}

/// Draw the shadow of each body that has ground under it.
#[allow(clippy::too_many_arguments)]
pub fn sync_ground_shadows(
    mut commands: Commands,
    rooms: LiveRoomOf<ae::RoomGeometry>,
    specs: Option<LiveRoomSpecs>,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    active_session: Option<Res<ActiveSessionScope>>,
    feature_views: Option<Res<ambition_sim_view::FeatureViewIndex>>,
    presented_features: Option<Res<ambition_sim_view::PresentedFeaturePoses>>,
    visuals: Query<(Entity, &FeatureVisual)>,
    player: Query<
        (Entity, &ambition_sim_view::BodyPoseView, Option<&ambition_sim_view::PresentedPose>),
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly,
    >,
    mut shadows: Query<(Entity, &GroundShadow, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(specs), Some(assets), Some(session_scope)) =
        (specs, assets, SessionSpawnScope::for_optional_active_session(active_session.as_deref()))
    else {
        return;
    };
    // The shadow picture of each live room whose theme has one.
    let pictures: Vec<(LiveRoomInstance, Handle<Image>)> = specs
        .live_rooms()
        .filter_map(|(room, definition)| {
            let theme = ParallaxTheme::named_by_room_metadata(&specs.rooms().spec(definition).metadata)?;
            Some((room, assets.room_dressing.get(theme, RoomDressingPart::Shadow)?.clone()))
        })
        .collect();
    // A tier that draws no parallax draws no shadows: they are scenery.
    let drawn = quality.is_none_or(|quality| quality.budget.parallax.enabled);

    // Each body that has a shadow now: whose, its room, the box of the body.
    let mut bodies: Vec<(String, LiveRoomInstance, ae::Vec2, ae::Vec2)> = Vec::new();
    if drawn && !pictures.is_empty() {
        if let Ok((body, pose, presented)) = player.single() {
            if let Some(room) = rooms.room_of(body) {
                bodies.push((
                    PLAYER_KEY.to_string(),
                    room,
                    ambition_sim_view::presented_pose::draw_pos(pose, presented),
                    pose.size,
                ));
            }
        }
        if let Some(feature_views) = feature_views.as_deref() {
            for (entity, visual) in &visuals {
                let Some(view) = feature_views.get(&visual.id) else {
                    continue;
                };
                if view.kind != FeatureVisualKind::Actor || !view.visible || view.submerged {
                    continue;
                }
                if let Some(room) = rooms.room_of(entity) {
                    // Where the sprite of the actor is drawn this frame, so
                    // the shadow does not lag it or lead it.
                    let pos = presented_features
                        .as_deref()
                        .map_or(view.pos, |poses| poses.presented(&visual.id, view.pos));
                    bodies.push((visual.id.clone(), room, pos, view.size));
                }
            }
        }
    }

    // Where each shadow is: the key, the picture, the place and the shape.
    let mut wanted: std::collections::HashMap<String, (LiveRoomInstance, Handle<Image>, Vec3, Vec2, f32)> =
        std::collections::HashMap::new();
    for (key, room, pos, size) in bodies {
        let (Some((_, picture)), Some(geometry)) =
            (pictures.iter().find(|(stamp, _)| *stamp == room), rooms.in_room(room))
        else {
            continue;
        };
        let world = &geometry.0;
        let feet = ae::Vec2::new(pos.x, pos.y + size.y * 0.5);
        let Some(top) = ground_below(world, feet, size.x * 0.5) else {
            continue;
        };
        let (shape, alpha) = shadow_shape(size.x, (top - feet.y).max(0.0));
        if alpha <= 0.01 {
            continue;
        }
        let at = world_to_bevy(world, ae::Vec2::new(pos.x, top + SINK), SHADOW_Z);
        wanted.insert(key, (room, picture.clone(), at, shape, alpha));
    }

    for (entity, shadow, mut transform, mut sprite, mut visibility) in &mut shadows {
        match wanted.remove(&shadow.key) {
            Some((_, picture, at, shape, alpha)) => {
                transform.translation = at;
                if sprite.image != picture {
                    sprite.image = picture;
                }
                sprite.custom_size = Some(shape);
                sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
                if *visibility != Visibility::Inherited {
                    *visibility = Visibility::Inherited;
                }
            }
            // The body is gone, or has no ground under it. A shadow is one
            // sprite: take it, and make it again when it is wanted.
            None => commands.entity(entity).try_despawn(),
        }
    }
    for (key, (room, picture, at, shape, alpha)) in wanted {
        commands.spawn_session_scoped(
            session_scope.in_room(Some(room)),
            (
                Sprite {
                    image: picture,
                    custom_size: Some(shape),
                    color: Color::srgba(1.0, 1.0, 1.0, alpha),
                    ..Default::default()
                },
                Transform::from_translation(at),
                Visibility::Inherited,
                GroundShadow { key },
                RoomVisual,
                Name::new("Ground shadow"),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room() -> ae::World {
        let (at, size) = (ae::Vec2::new, ae::Vec2::new);
        ae::World::new(
            "room",
            ae::Vec2::new(800.0, 600.0),
            ae::Vec2::ZERO,
            vec![
                ae::Block::solid("floor", at(0.0, 500.0), size(800.0, 100.0)),
                ae::Block::one_way("platform", at(300.0, 400.0), size(100.0, 16.0)),
                ae::Block::hazard("spikes", at(600.0, 480.0), size(100.0, 20.0)),
                ae::Block::solid("roof", at(0.0, 0.0), size(800.0, 40.0)),
            ],
        )
    }

    /// The ground under a body is the nearest top at its feet or below them.
    /// A platform under the body is nearer than the floor. A hazard is not
    /// ground, the roof over the body is not under it, and a body too high
    /// has no ground.
    #[test]
    fn the_ground_is_the_nearest_top_under_the_feet() {
        let world = room();
        assert_eq!(ground_below(&world, ae::Vec2::new(100.0, 500.0), 12.0), Some(500.0), "it stands on the floor");
        assert_eq!(ground_below(&world, ae::Vec2::new(100.0, 440.0), 12.0), Some(500.0), "60 over the floor");
        assert_eq!(ground_below(&world, ae::Vec2::new(350.0, 380.0), 12.0), Some(400.0), "over the platform");
        assert_eq!(ground_below(&world, ae::Vec2::new(350.0, 430.0), 12.0), Some(500.0), "under the platform");
        assert_eq!(ground_below(&world, ae::Vec2::new(650.0, 460.0), 12.0), Some(500.0), "a hazard is not ground");
        assert_eq!(ground_below(&world, ae::Vec2::new(100.0, 500.0 - REACH - 10.0), 12.0), None, "too high");
    }

    /// A body that stands has the full shadow, and a body in the air a
    /// smaller and fainter one, down to nothing at the reach.
    #[test]
    fn a_shadow_is_smaller_and_fainter_as_the_body_goes_up() {
        let (stand, stand_alpha) = shadow_shape(30.0, 0.0);
        let (mid, mid_alpha) = shadow_shape(30.0, REACH * 0.5);
        let (_, gone) = shadow_shape(30.0, REACH);
        assert_eq!(stand, Vec2::new(45.0, THICKNESS));
        assert_eq!(shadow_shape(10.0, 0.0).0.x, LEAST_WIDTH, "a thin body has a shadow that shows");
        assert_eq!(stand_alpha, FULL_ALPHA);
        assert!(mid.x < stand.x && mid.y < stand.y && mid_alpha < stand_alpha && mid_alpha > 0.0);
        assert_eq!(gone, 0.0);
    }
}
