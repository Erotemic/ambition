//! A wire from an anchor to the body it holds.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope;

/// The wire a body is currently hanging from.
///
/// One per body on a wire. A versus match has four fighters and any of them
/// may be the Performer; a singleton would draw one rope and move it between
/// them.
pub enum FlylineEffect {}

pub type FlylineVisual = super::attached_visuals::BodyAttachedVisual<FlylineEffect>;

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
    bodies: super::presented_bodies::PresentedBodies,
    sprite: Option<Res<FlylineSprite>>,
    active_session: Option<Res<ActiveSessionScope>>,
    mut wires: Query<(Entity, &mut FlylineVisual, &mut Transform, &mut Sprite)>,
    roots: Query<&Transform, Without<FlylineVisual>>,
) {
    super::attached_visuals::reconcile_presented_visuals(
        &mut commands,
        &bodies,
        active_session.as_deref(),
        sprite
            .as_ref()
            .map(|s| &s.handle)
            .filter(|handle| **handle != Handle::default()),
        &mut wires,
        "Flyline Visual",
        |body| body.wire_anchor.map(|anchor| (anchor, body.pos, wire_z(&roots, body.visual))),
        |room, transform, art, (anchor, at, z)| place_wire(room, transform, art, *anchor, *at, *z),
    );
}

/// How far under its body a rope is drawn, so it does not cut across it.
pub(crate) const WIRE_UNDER_BODY: f32 = 0.05;

/// The depth a body's rope is drawn at: just under the body's drawn root,
/// whatever layer or seat the body is drawn in. A player is at the player
/// plane and an actor at the actor layer, and a rope at the player plane cut
/// across every actor of its layer, its own body included. A body whose root
/// is not drawn keeps the player plane.
pub(crate) fn wire_z<F: bevy::ecs::query::QueryFilter>(roots: &Query<&Transform, F>, visual: Entity) -> f32 {
    roots.get(visual).map_or(ambition_platformer2d_core::config::WORLD_Z_PLAYER, |root| root.translation.z)
        - WIRE_UNDER_BODY
}

/// Stretch and rotate the rope so it runs from `anchor` to `at`, at depth `z`
/// ([`wire_z`]).
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
    z: f32,
) {
    let span = at - anchor;
    let length = span.length().max(1.0);
    let middle = anchor + span * 0.5;
    transform.translation = ambition_platformer2d_core::config::world_to_bevy(
        room,
        ambition_platformer2d_core::Vec2::new(middle.x, middle.y),
        z,
    );
    // The texture axis is its height. World +y is down and Bevy +y is up, so
    // the angle is measured from straight down in world terms and the sign
    // flips.
    transform.rotation = Quat::from_rotation_z(f32::atan2(-span.x, -span.y));
    art.custom_size = Some(bevy::math::Vec2::new(WIRE_WIDTH, length));
}

#[cfg(test)]
mod tests;
