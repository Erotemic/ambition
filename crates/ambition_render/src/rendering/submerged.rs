//! A body under the stage is not drawn; a trapdoor is drawn instead.
//!
//! Same shape as `morph_ball.rs` and `flyline.rs`: a modal body visual, kept
//! in that shape so the three can later be generalized together.
//!
//! It runs after the morph-ball sync and has the last word. Both systems
//! restore a hidden body to `Inherited`, and morph-ball restores whenever the
//! body is not morphed. If morph-ball ran last, a submerged body would be
//! made visible under the stage. Ordering fixes this; morph-ball does not need
//! to know about submersion.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, PlayerVisual, SessionSpawnScope, SpawnSessionScopedExt,
};

/// Hide every submerged body, and hand every other one back.
///
/// Restore to `Inherited`, never `Visible`. A death overlay or a
/// room-transition fade hides bodies through the parent; `Visible` would
/// leave a surfacing fighter as the only thing on screen. Morph-ball uses
/// the same rule.
pub fn sync_submerged_visibility(
    mut bodies: Query<(&ambition_sim_view::BodyPoseView, &mut Visibility), With<PlayerVisual>>,
) {
    for (pose, mut visibility) in &mut bodies {
        if pose.submerged {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        } else if matches!(*visibility, Visibility::Hidden) {
            *visibility = Visibility::Inherited;
        }
    }
}

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// The door itself
// ---------------------------------------------------------------------------

/// The trapdoor she is replaced with while she is under the stage.
///
/// The body is replaced with a trapdoor on the ground, so an opponent can
/// see where she is. Without it the move would cost nothing in readability.
///
/// One door per submerged body: a versus match has four fighters, and any of
/// them may use this move. The door names its body and is removed when that
/// body surfaces.
///
/// Procedural, like the morph ball: the shipped `trapdoor_boards` art is an
/// effect (eight frames that play once), and this needs a persistent object.
#[derive(Component)]
pub struct TrapdoorVisual {
    /// The submerged body this door belongs to.
    pub body: Entity,
}

/// The door's texture handle, built once.
#[derive(Resource, Clone, Default)]
pub struct TrapdoorSprite {
    pub handle: Handle<Image>,
}

const DOOR_TEXTURE_W: u32 = 64;
const DOOR_TEXTURE_H: u32 = 16;

/// The door, closed: boards flush in the floor, a seam down the middle, a
/// ring pull, and a frame around them.
///
/// Closed on purpose. An open hatch would show where she is and give the
/// move away; a closed door must be watched. Opening is the next stage's own
/// effect.
pub fn build_trapdoor_image() -> Image {
    let (w, h) = (DOOR_TEXTURE_W, DOOR_TEXTURE_H);
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / (w - 1) as f32;
            let v = y as f32 / (h - 1) as f32;
            // The boards, with grain along their length and more light near the top
            // edge, where the floor catches it.
            let grain = ((x as f32 * 0.9).sin() * 0.5 + 0.5) * 0.08;
            let lift = (1.0 - v).powf(1.6) * 0.22;
            let base = 0.30 + grain + lift;
            let mut rgb = [base * 1.00, base * 0.70, base * 0.42];
            let mut a = 1.0_f32;
            // The seam down the middle: two boards, not one plank.
            if (u - 0.5).abs() < 0.014 {
                rgb = [0.10, 0.07, 0.05];
            }
            // The frame: a dark line all round, so it reads as a door in the floor
            // and not a rug on it.
            if v < 0.10 || v > 0.90 || u < 0.03 || u > 0.97 {
                rgb = [0.13, 0.09, 0.06];
            }
            // The ring pull, off to one side of the seam.
            let ring = ((u - 0.31).powi(2) * 9.0 + (v - 0.5).powi(2)).sqrt();
            if (ring - 0.20).abs() < 0.075 {
                rgb = [0.62, 0.52, 0.24];
            }
            // Rounded ends, so a door on a narrow platform does not read as a
            // full-width plank.
            if u < 0.015 || u > 0.985 {
                a = 0.0;
            }
            let i = ((y * w + x) * 4) as usize;
            data[i] = (rgb[0].clamp(0.0, 1.0) * 255.0) as u8;
            data[i + 1] = (rgb[1].clamp(0.0, 1.0) * 255.0) as u8;
            data[i + 2] = (rgb[2].clamp(0.0, 1.0) * 255.0) as u8;
            data[i + 3] = (a * 255.0) as u8;
        }
    }
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Startup: build the door texture once.
pub fn build_trapdoor_sprite(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(build_trapdoor_image());
    commands.insert_resource(TrapdoorSprite { handle });
}

/// How much wider than the body the door is drawn. The shipped
/// `trapdoor_boards` effect is 52-64 px across against a ~30 px fighter; a
/// door exactly as wide as the body reads as a hatch.
const DOOR_WIDTH_FACTOR: f32 = 1.8;
/// The door's drawn height, in world px. It lies on the floor.
const DOOR_HEIGHT: f32 = 12.0;

/// Give every submerged body a door, remove it from every body that
/// surfaced, and keep the rest on the floor the body went through.
///
/// The door is at the feet, not the centre. A submerged body never moves
/// along gravity (`integrate_submerged_clusters` pins that axis), so the feet
/// line is the surface; the centre would float the door half a body up.
pub fn sync_trapdoor_visuals(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    sprite: Option<Res<TrapdoorSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    bodies: Query<
        (
            Entity,
            &ambition_sim_view::BodyPoseView,
            Option<&ambition_sim_view::PresentedPose>,
        ),
        With<PlayerVisual>,
    >,
    // The other road, used by match fighters. `PlayerVisual` is only on the
    // session's single exploration player; every actor is a `FeatureVisual`
    // reading `FeatureViewIndex`, which carries `submerged`.
    actors: Query<
        (Entity, &crate::rendering::FeatureVisual),
        Without<PlayerVisual>,
    >,
    // `Option`: a plain `Res` fails any composition that does not build the
    // index ("Resource does not exist"). No index means nothing to draw.
    feature_views: Option<Res<ambition_sim_view::FeatureViewIndex>>,
    mut doors: Query<(Entity, &TrapdoorVisual, &mut Transform, &mut Sprite)>,
) {
    // Both roads reduced to two facts: where the body is and how big it is.
    // The code below reads only this.
    let mut under: Vec<(Entity, bevy::math::Vec2, bevy::math::Vec2)> = Vec::new();
    for (body, pose, presented) in &bodies {
        if pose.submerged {
            under.push((
                body,
                ambition_sim_view::presented_pose::draw_pos(pose, presented),
                bevy::math::Vec2::new(pose.size.x, pose.size.y),
            ));
        }
    }
    for (body, visual) in &actors {
        let Some(view) = feature_views.as_ref().and_then(|i| i.get(&visual.id)) else {
            continue;
        };
        if view.submerged {
            under.push((
                body,
                bevy::math::Vec2::new(view.pos.x, view.pos.y),
                bevy::math::Vec2::new(view.size.x, view.size.y),
            ));
        }
    }
    // Retire the doors whose body surfaced or went away, and move the rest.
    let mut standing = bevy::platform::collections::HashSet::new();
    for (door, owner, mut transform, mut art) in &mut doors {
        let Some((_, at, size)) = under.iter().copied().find(|(b, _, _)| *b == owner.body) else {
            commands.entity(door).despawn();
            continue;
        };
        standing.insert(owner.body);
        place_door(&world.0, &mut transform, &mut art, at, size);
    }
    let Some(sprite) = sprite else {
        return;
    };
    if sprite.handle == Handle::default() {
        return;
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    for (body, at, size) in under {
        if standing.contains(&body) {
            continue;
        }
        let mut transform = Transform::default();
        let mut art = Sprite {
            image: sprite.handle.clone(),
            ..default()
        };
        place_door(&world.0, &mut transform, &mut art, at, size);
        commands.spawn_session_scoped(
            session_scope,
            (
                art,
                transform,
                Visibility::Visible,
                TrapdoorVisual { body },
                // Which body this drawable draws, in the shared spelling that any
                // consumer can query. `TrapdoorVisual` above keeps it for placement.
                ambition_platformer2d_shared_tangle::lifecycle::PresentationOf(body),
                Name::new("Trapdoor Visual"),
            ),
        );
    }
}

fn place_door(
    room: &ambition_platformer2d_core::World,
    transform: &mut Transform,
    art: &mut Sprite,
    at: bevy::math::Vec2,
    size: bevy::math::Vec2,
) {
    let feet = at + bevy::math::Vec2::new(0.0, size.y * 0.5);
    transform.translation = ambition_platformer2d_core::config::world_to_bevy(
        room,
        feet,
        ambition_platformer2d_core::config::WORLD_Z_PLAYER + 0.05,
    );
    art.custom_size = Some(bevy::math::Vec2::new(size.x * DOOR_WIDTH_FACTOR, DOOR_HEIGHT));
}
