//! THE WIRE ITSELF — a rope from a point in the flies down to a body on it.
//!
//! ⭐⭐ THE THIRD CUSTOMER OF THE SHAPE `submerged.rs` AND `morph_ball.rs` SHARE:
//! a procedural, per-body visual with a lifecycle, spawned while a state holds
//! and retired when it ends. Kept deliberately in the same shape, because
//! `morph_ball.rs` ends with *"generalize modal body morphs — that is what this
//! means, and it deletes this whole file"* and a generalization is better with
//! three examples than with one.
//!
//! ⛔⛔ AND IT IS PROCEDURAL FOR THE REASON THE TRAPDOOR IS. A rope has to be
//! there for the whole lift and has to be a DIFFERENT LENGTH every frame — the
//! winch is shortening it. An FX-atlas row plays once at a fixed size and ends;
//! borrowing one to hold a wire open would be a second consumer of a system that
//! exists to finish.
//!
//! ⛔⛔ BOTH ROADS, AND THAT IS NOT DEFENSIVE. `PlayerVisual` is inserted in
//! exactly ONE place in the engine — the session's single exploration player —
//! so a visual gated on it alone appears in an Ambition room and never once in a
//! versus match. That is precisely what happened to the trapdoor, and every test
//! it had spawned a `PlayerVisual`, so none of them could fail. Every match
//! fighter is a `FeatureVisual` reading `FeatureViewIndex`.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, PlayerVisual, SessionSpawnScope, SpawnSessionScopedExt,
};

/// The wire a body is currently hanging from.
///
/// ⛔ ONE PER BODY ON A WIRE, not one. A versus match has four fighters and any
/// of them may be the Performer; a singleton would draw one rope and move it
/// between them. The same correction `submerged.rs` records having made.
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

/// How wide the wire is drawn, in world px. Thin: it is a stagehand's flying
/// wire, not a ship's hawser, and a fat rope would read as a pillar.
const WIRE_WIDTH: f32 = 3.0;

/// A length of bright steel cable, tiled along its own axis.
///
/// ⭐ TWO STRANDS AND A HIGHLIGHT, so it reads as twisted wire rather than a
/// drawn line. The sprite is stretched along the rope's length, and the detail
/// that survives that stretch is the one across its WIDTH — which is why every
/// feature here varies with `u` and almost nothing with `v`.
pub fn build_flyline_image() -> Image {
    let (w, h) = (WIRE_TEXTURE_W, WIRE_TEXTURE_H);
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / (w - 1) as f32;
            let v = y as f32 / (h - 1) as f32;
            // The cable's round section: bright down one side, dark down the
            // other, so it catches the stage light from a consistent direction.
            let across = (u - 0.5) * 2.0;
            let shade = (1.0 - across.abs()).max(0.0).powf(0.6);
            let mut lit = 0.26 + shade * 0.42;
            // The twist — a slow braid along its length. It is the only thing
            // here that varies with `v`, and it is what stops a fast-moving rope
            // reading as a static bar.
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

/// Give every body on a wire a wire, take it away from every body that has been
/// let go, and keep the ones that remain stretched between the anchor and her.
///
/// ⛔ THE ROPE IS DRAWN TO HER CENTRE, not to her feet, because that is where
/// the kernel hangs her from — `WireState`'s length is measured to the body
/// centre. A wire ending at the ankles would swing visibly out of step with the
/// body it is supposed to be carrying.
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
    // ⛔⛔ THE OTHER ROAD, AND IT IS THE ONE A MATCH FIGHTER TAKES. See the
    // module doc: this is the whole reason the trapdoor drew in an Ambition room
    // and never in a versus match.
    actors: Query<(Entity, &crate::rendering::FeatureVisual), Without<PlayerVisual>>,
    // ⛔⛔ `Option`, AND IT IS NOT DEFENSIVE. A plain `Res` here is a HARD STOP
    // for any composition that does not build the index, with the undebuggable
    // *"Parameter ... failed validation: Resource does not exist"*. A projection
    // nobody has published yet has nothing to say.
    feature_views: Option<Res<ambition_sim_view::FeatureViewIndex>>,
    mut wires: Query<(Entity, &FlylineVisual, &mut Transform, &mut Sprite)>,
) {
    // Both roads reduced to the only two facts a rope needs: where it hangs
    // from, and where the body is. Retirement and spawning below read this and
    // nothing else, so neither has to learn there are two kinds of body visual.
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
                Name::new("Flyline Visual"),
            ),
        );
    }
}

/// Stretch and rotate the rope so it runs from `anchor` to `at`.
///
/// ⛔ THE SPRITE IS PLACED AT THE MIDPOINT AND ROTATED, rather than drawn as a
/// chain of segments the way `grapple.rs` draws its line out of VFX bursts. A
/// swinging rope changes angle every frame, and a segment chain would have to
/// respawn its whole length each time.
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
        // BEHIND her. She is hanging on the end of it, and a rope drawn over the
        // body would cut her in half down the middle.
        ambition_platformer2d_core::config::WORLD_Z_PLAYER - 0.05,
    );
    // The texture's own axis is its HEIGHT, and world +y is DOWN while Bevy's is
    // up — so the angle is measured from straight down in world terms, which is
    // straight up in Bevy's, and the sign flips with it.
    transform.rotation = Quat::from_rotation_z(f32::atan2(-span.x, -span.y));
    art.custom_size = Some(bevy::math::Vec2::new(WIRE_WIDTH, length));
}

#[cfg(test)]
mod tests;
