//! A body under the stage is not drawn, and a TRAPDOOR is drawn instead.
//!
//! ⭐⭐ THE SECOND MODAL BODY MORPH, and the file next door already predicted
//! it: `morph_ball.rs` ends with *"generalize modal body morphs — that is what
//! this means, and it deletes this whole file."* This is not that
//! generalization; it is the second customer, kept deliberately in the same
//! shape so the eventual generalization has two examples to be right about
//! rather than one.
//!
//! ⛔⛔ IT RUNS AFTER THE MORPH-BALL SYNC AND IS THE LAST WORD. Both systems
//! restore a hidden body to `Inherited`, and morph-ball's restore is
//! unconditional on "not morphed" — so a body hidden HERE and read THERE on the
//! same frame would be handed back to the renderer visible, standing under the
//! stage in full view. Ordering is the fix rather than teaching morph-ball about
//! submersion, because the thing morph-ball is wrong about is that it believes
//! it is the only mode that hides a body.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, PlayerVisual, SessionSpawnScope, SpawnSessionScopedExt,
};

/// Hide every submerged body, and hand every other one back.
///
/// ⛔ `Inherited` ON THE WAY OUT, NEVER A HARD `Visible`. A death overlay or a
/// room-transition fade hides bodies through the parent; overriding to `Visible`
/// would make a fighter who happened to surface mid-fade the one thing still on
/// screen. Morph-ball states the same rule for the same reason.
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
/// ⭐⭐ JON, 2026-08-28: *"There should be a trapdoor sprite she is replaced
/// with on the ground."* Hiding the body answered half of that and left the
/// other half as nothing at all on stage — an opponent had no idea where she
/// was, which makes a move whose whole cost is being readable free.
///
/// ⛔ ONE DOOR PER SUBMERGED BODY, not one door. `morph_ball.rs` next door is a
/// singleton and its own comments record what that cost: a versus match has
/// four fighters and any of them may hold this move. The door names the body it
/// belongs to and dies with that body's submersion.
///
/// ⛔ AND IT IS PROCEDURAL, for the reason the morph ball is: the shipped
/// `trapdoor_boards` art is an EFFECT — eight frames that play once and end —
/// and the thing wanted here is a persistent object. Borrowing a row out of the
/// FX atlas to hold it open would be a second consumer of a system that exists
/// to finish.
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

/// The door: two boards, a dark seam between them, and a frame around the hole.
///
/// Drawn as a texture rather than as a coloured rect so the seam reads at the
/// sizes a fighter is drawn at — a flat rectangle on the floor looks like a
/// missing tile.
pub fn build_trapdoor_image() -> Image {
    let (w, h) = (DOOR_TEXTURE_W, DOOR_TEXTURE_H);
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / (w - 1) as f32;
            let v = y as f32 / (h - 1) as f32;
            // The hole: dark, and it is what the boards are set into.
            let mut rgb = [0.06_f32, 0.05, 0.05];
            let mut a = 1.0_f32;
            // Two boards, hinged at the outer edges and open toward the middle.
            let seam = (u - 0.5).abs();
            if seam > 0.06 {
                // Wood, with a grain that runs along the board.
                let grain = ((x as f32 * 0.9).sin() * 0.5 + 0.5) * 0.10;
                // Lit at the top edge, shadowed toward the hole.
                let lift = (1.0 - v).powf(1.4) * 0.35;
                let base = 0.34 + grain + lift;
                rgb = [base * 1.00, base * 0.72, base * 0.42];
            }
            // The frame: a bright lip along the top of the opening, which is
            // what makes the door read as set INTO the floor.
            if v < 0.12 {
                rgb = [0.55, 0.42, 0.26];
            }
            // Rounded ends, so a door on a narrow platform does not read as a
            // full-width plank.
            if u < 0.02 || u > 0.98 {
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
/// `trapdoor_boards` effect is 52-64 px across against a ~30 px fighter, and a
/// door exactly the width of the person who went through it reads as a hatch.
const DOOR_WIDTH_FACTOR: f32 = 1.8;
/// The door's drawn height, in world px. It lies on the floor.
const DOOR_HEIGHT: f32 = 12.0;

/// Give every submerged body a door, take it away from every body that has
/// surfaced, and keep the ones that remain sitting on the floor she went
/// through.
///
/// ⛔ THE DOOR IS AT HER FEET, NOT AT HER CENTRE. A submerged body never moves
/// along gravity — `integrate_submerged_clusters` pins that axis — so the feet
/// line IS the surface she is travelling under, and drawing at the centre would
/// float the door half a body above the boards.
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
    mut doors: Query<(Entity, &TrapdoorVisual, &mut Transform, &mut Sprite)>,
) {
    // Retire the doors whose body surfaced or went away, and move the rest.
    let mut standing = bevy::platform::collections::HashSet::new();
    for (door, owner, mut transform, mut art) in &mut doors {
        let Ok((_, pose, presented)) = bodies.get(owner.body) else {
            commands.entity(door).despawn();
            continue;
        };
        if !pose.submerged {
            commands.entity(door).despawn();
            continue;
        }
        standing.insert(owner.body);
        place_door(&world.0, &mut transform, &mut art, pose, presented);
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
    for (body, pose, presented) in &bodies {
        if !pose.submerged || standing.contains(&body) {
            continue;
        }
        let mut transform = Transform::default();
        let mut art = Sprite {
            image: sprite.handle.clone(),
            ..default()
        };
        place_door(&world.0, &mut transform, &mut art, pose, presented);
        commands.spawn_session_scoped(
            session_scope,
            (
                art,
                transform,
                Visibility::Visible,
                TrapdoorVisual { body },
                Name::new("Trapdoor Visual"),
            ),
        );
    }
}

fn place_door(
    room: &ambition_platformer2d_core::World,
    transform: &mut Transform,
    art: &mut Sprite,
    pose: &ambition_sim_view::BodyPoseView,
    presented: Option<&ambition_sim_view::PresentedPose>,
) {
    let at = ambition_sim_view::presented_pose::draw_pos(pose, presented);
    let feet = at + bevy::math::Vec2::new(0.0, pose.size.y * 0.5);
    transform.translation = ambition_platformer2d_core::config::world_to_bevy(
        room,
        feet,
        ambition_platformer2d_core::config::WORLD_Z_PLAYER + 0.05,
    );
    art.custom_size = Some(bevy::math::Vec2::new(
        pose.size.x * DOOR_WIDTH_FACTOR,
        DOOR_HEIGHT,
    ));
}
