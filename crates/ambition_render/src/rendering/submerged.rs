//! A body under the stage is not drawn.
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

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, PlayerVisual, SessionSpawnScope, SpawnSessionScopedExt,
};
use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// The closed hatch that stands in for a body under the stage.
#[derive(Component)]
pub struct SubmergedMarkerVisual;

/// The procedural hatch texture, built once.
#[derive(Resource)]
pub struct SubmergedMarkerSprite {
    pub handle: Handle<Image>,
}

/// Texture resolution of the hatch. Small on purpose: it is a flat shape with
/// three bands and nothing that rewards more texels.
const MARKER_TEXTURE: u32 = 64;

/// BEHIND the bodies still on the stage. She is under the floor; a marker that
/// drew over the fighter standing on top of her would read as an object in the
/// air rather than a hatch in the ground.
const MARKER_Z: f32 = ambition_platformer2d_core::config::WORLD_Z_PLAYER - 0.05;

/// A closed trapdoor, seen from the side: two dark boards with a lit upper lip
/// and an iron hinge band at one end.
///
/// ⛔ CLOSED, WHICH IS THE WHOLE INSTRUCTION. Jon, 2026-08-27: *"you should just
/// see an unopened trap door move around to indicate where she is, and then it
/// opens when she emerges."* The OPENING is the `trapdoor_boards` effect the
/// move already plays at both beats; this is the shut hatch in between, and if
/// it ever animates, the two will be saying different things about the same
/// door.
pub fn build_submerged_marker_image() -> Image {
    let size = MARKER_TEXTURE;
    let mut data = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        // The hatch occupies a shallow band across the middle: it is a lid lying
        // in a floor, not a box standing in one.
        let v = y as f32 / (size as f32 - 1.0);
        let (r, g, b, a) = if !(0.42..=0.62).contains(&v) {
            (0u8, 0u8, 0u8, 0u8)
        } else if v < 0.46 {
            (186, 133, 88, 255) // the lit lip
        } else if v > 0.585 {
            (86, 56, 37, 255) // the shadow under the boards
        } else {
            (139, 94, 60, 255) // timber
        };
        for x in 0..size {
            let u = x as f32 / (size as f32 - 1.0);
            let i = ((y * size + x) * 4) as usize;
            // The hinge: an iron band at the near end, so the closed hatch reads
            // as the same object that swings open.
            let iron = (0.06..=0.16).contains(&u) && a > 0;
            let (r, g, b) = if iron { (78, 76, 84) } else { (r, g, b) };
            // A plank seam, so it is boards rather than a slab.
            let seam = a > 0 && ((u - 0.5).abs() < 0.012);
            let (r, g, b) = if seam { (86, 56, 37) } else { (r, g, b) };
            data[i] = r;
            data[i + 1] = g;
            data[i + 2] = b;
            data[i + 3] = a;
        }
    }
    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

pub fn build_submerged_marker_sprite(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(build_submerged_marker_image());
    commands.insert_resource(SubmergedMarkerSprite { handle });
}

fn new_marker_sprite(handle: Handle<Image>) -> impl Bundle {
    (
        Sprite {
            image: handle,
            custom_size: Some(bevy::math::Vec2::new(56.0, 56.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, MARKER_Z),
        Visibility::Hidden,
        SubmergedMarkerVisual,
        Name::new("Submerged Marker Visual"),
    )
}

/// Seed the pool with one hatch; `sync` grows it if two fighters are under at
/// once, which a mirror match makes ordinary.
pub fn spawn_submerged_marker_visual(
    mut commands: Commands,
    sprite: Option<Res<SubmergedMarkerSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    existing: Query<(), With<SubmergedMarkerVisual>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(sprite) = sprite else { return };
    if sprite.handle == Handle::default() {
        return;
    }
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        return;
    };
    commands.spawn_session_scoped(session_scope, new_marker_sprite(sprite.handle.clone()));
}

/// Put a closed hatch wherever a body is travelling under the stage.
pub fn sync_submerged_markers(
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    markers: Res<ambition_sim_view::SubmergedMarkersView>,
    mut commands: Commands,
    sprite: Option<Res<SubmergedMarkerSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    mut visuals: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<SubmergedMarkerVisual>>,
) {
    let facts = &markers.0;
    let pool = visuals.iter().count();
    let mut assigned = 0usize;
    for (mut transform, mut sprite_mut, mut vis) in &mut visuals {
        if let Some(fact) = facts.get(assigned).copied() {
            transform.translation = ambition_platformer2d_core::config::world_to_bevy(
                &world.0,
                fact.pos,
                MARKER_Z,
            );
            // The hatch lies in the surface this body went through, so it turns
            // with that body's own gravity rather than with the screen.
            transform.rotation = Quat::from_rotation_z(
                ambition_platformer2d_shared_tangle::gravity::gravity_upright_angle(
                    fact.gravity_dir,
                ),
            );
            sprite_mut.custom_size =
                Some(bevy::math::Vec2::new(fact.size.x * 1.9, fact.size.x * 1.9));
            *vis = Visibility::Visible;
            assigned += 1;
        } else if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
    if facts.len() > pool {
        let (Some(sprite), Some(session_scope)) = (
            sprite,
            SessionSpawnScope::for_optional_active_session(active_session.as_deref()),
        ) else {
            return;
        };
        commands.spawn_session_scoped(session_scope, new_marker_sprite(sprite.handle.clone()));
    }
}

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
