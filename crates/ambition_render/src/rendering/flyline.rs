//! The wire: a rope from a point in the flies down to a body on it.
//!
//! Same shape as `submerged.rs` and `morph_ball.rs`: a procedural per-body
//! visual, spawned while a state holds and retired when it ends. Kept in that
//! shape so the three can later be generalized together (see the note at the
//! end of `morph_ball.rs`).
//!
//! Procedural, like the trapdoor: the rope must exist for the whole lift and
//! change length every frame as the winch shortens it. An FX-atlas row plays
//! once at a fixed size.
//!
//! Both body roads are required. `PlayerVisual` is only on the session's
//! single exploration player, so a visual gated on it never appears in a
//! versus match. Every match fighter is a `FeatureVisual` that reads
//! `FeatureViewIndex`.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, PlayerVisual, SessionSpawnScope, SpawnSessionScopedExt,
};

/// The wire a body is currently hanging from.
///
/// One per body on a wire. A versus match has four fighters and any of them
/// may be the Performer; a singleton would draw one rope and move it between
/// them.
#[derive(Component)]
pub struct FlylineVisual {
    /// The body this wire is holding up.
    pub body: Entity,
}

/// The rope's texture handle, built once.
#[derive(Resource, Clone, Default)]
pub struct FlylineSprite {
    pub handle: Handle<Image>,
}

const WIRE_TEXTURE_W: u32 = 8;
const WIRE_TEXTURE_H: u32 = 32;

/// How wide the wire is drawn, in world px. Thin, so it reads as a flying
/// wire, not a pillar.
const WIRE_WIDTH: f32 = 3.0;

/// A length of bright steel cable, tiled along its own axis.
///
/// Two strands and a highlight, so it reads as twisted wire. The sprite is
/// stretched along the rope, so only detail across its width survives; almost
/// every feature varies with `u`, not `v`.
pub fn build_flyline_image() -> Image {
    let (w, h) = (WIRE_TEXTURE_W, WIRE_TEXTURE_H);
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / (w - 1) as f32;
            let v = y as f32 / (h - 1) as f32;
            // The round section: bright on one side, dark on the other, so it
            // catches light from one direction.
            let across = (u - 0.5) * 2.0;
            let shade = (1.0 - across.abs()).max(0.0).powf(0.6);
            let mut lit = 0.26 + shade * 0.42;
            // The twist: a slow braid along the length. The only thing that varies
            // with `v`, so a fast rope does not read as a static bar.
            let braid = ((v * std::f32::consts::TAU * 3.0) + across * 1.4).sin();
            lit += braid * 0.06;
            // The specular strand, off-centre so the cable has a near side.
            if (u - 0.34).abs() < 0.10 {
                lit += 0.30;
            }
            let mut a = 1.0_f32;
            // Soft edges, so a 3px rope does not alias into a dotted line.
            if !(0.08..=0.92).contains(&u) {
                a = 0.0;
            } else if !(0.16..=0.84).contains(&u) {
                a = 0.55;
            }
            let rgb = [lit * 0.86, lit * 0.90, lit * 1.00];
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

/// Startup: build the wire texture once.
pub fn build_flyline_sprite(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(build_flyline_image());
    commands.insert_resource(FlylineSprite { handle });
}

/// Give every body on a wire a wire, remove it from every body that was let
/// go, and keep the rest stretched between the anchor and the body.
///
/// The rope ends at the body centre, not the feet: `WireState` measures its
/// length to the centre. A rope to the ankles would swing out of step.
pub fn sync_flyline_visuals(
    mut commands: Commands,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    sprite: Option<Res<FlylineSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    bodies: Query<
        (
            Entity,
            &ambition_sim_view::BodyPoseView,
            Option<&ambition_sim_view::PresentedPose>,
        ),
        With<PlayerVisual>,
    >,
    // The other road, used by match fighters. See the module doc.
    actors: Query<(Entity, &crate::rendering::FeatureVisual), Without<PlayerVisual>>,
    // `Option`: a plain `Res` stops any composition that does not build the
    // index with "Resource does not exist". No index means nothing to draw.
    feature_views: Option<Res<ambition_sim_view::FeatureViewIndex>>,
    mut wires: Query<(Entity, &FlylineVisual, &mut Transform, &mut Sprite)>,
) {
    // Both roads reduced to two facts: where the rope hangs from and where the
    // body is. The code below reads only this.
    let mut hanging: Vec<(Entity, bevy::math::Vec2, bevy::math::Vec2)> = Vec::new();
    for (body, pose, presented) in &bodies {
        if let Some(anchor) = pose.wire_anchor {
            hanging.push((
                body,
                bevy::math::Vec2::new(anchor.x, anchor.y),
                ambition_sim_view::presented_pose::draw_pos(pose, presented),
            ));
        }
    }
    for (body, visual) in &actors {
        let Some(view) = feature_views.as_ref().and_then(|i| i.get(&visual.id)) else {
            continue;
        };
        if let Some(anchor) = view.wire_anchor {
            hanging.push((
                body,
                bevy::math::Vec2::new(anchor.x, anchor.y),
                bevy::math::Vec2::new(view.pos.x, view.pos.y),
            ));
        }
    }
    // Retire the wires whose body was let go or went away, and move the rest.
    let mut standing = bevy::platform::collections::HashSet::new();
    for (wire, owner, mut transform, mut art) in &mut wires {
        let Some((_, anchor, at)) = hanging.iter().copied().find(|(b, _, _)| *b == owner.body)
        else {
            commands.entity(wire).despawn();
            continue;
        };
        standing.insert(owner.body);
        place_wire(&world.0, &mut transform, &mut art, anchor, at);
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
    for (body, anchor, at) in hanging {
        if standing.contains(&body) {
            continue;
        }
        let mut transform = Transform::default();
        let mut art = Sprite {
            image: sprite.handle.clone(),
            ..default()
        };
        place_wire(&world.0, &mut transform, &mut art, anchor, at);
        commands.spawn_session_scoped(
            session_scope,
            (
                art,
                transform,
                Visibility::Visible,
                FlylineVisual { body },
                // Which body this drawable draws, in the shared spelling that any
                // consumer can query. `FlylineVisual` above keeps it for placement.
                ambition_platformer2d_shared_tangle::lifecycle::PresentationOf(body),
                Name::new("Flyline Visual"),
            ),
        );
    }
}

/// Stretch and rotate the rope so it runs from `anchor` to `at`.
///
/// One sprite at the midpoint, rotated, not a chain of segments like
/// `grapple.rs`. A swinging rope changes angle every frame, and a chain
/// would have to respawn its whole length.
pub(crate) fn place_wire(
    room: &ambition_platformer2d_core::World,
    transform: &mut Transform,
    art: &mut Sprite,
    anchor: bevy::math::Vec2,
    at: bevy::math::Vec2,
) {
    let span = at - anchor;
    let length = span.length().max(1.0);
    let middle = anchor + span * 0.5;
    transform.translation = ambition_platformer2d_core::config::world_to_bevy(
        room,
        ambition_platformer2d_core::Vec2::new(middle.x, middle.y),
        // Behind the body, so the rope does not cut across it.
        ambition_platformer2d_core::config::WORLD_Z_PLAYER - 0.05,
    );
    // The texture axis is its height. World +y is down and Bevy +y is up, so
    // the angle is measured from straight down in world terms and the sign
    // flips.
    transform.rotation = Quat::from_rotation_z(f32::atan2(-span.x, -span.y));
    art.custom_size = Some(bevy::math::Vec2::new(WIRE_WIDTH, length));
}

#[cfg(test)]
mod tests;
