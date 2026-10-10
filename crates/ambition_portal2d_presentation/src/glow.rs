//! The portal's line of light: where the opening of an aperture is.
//!
//! A portal is drawn as one thin glowing line along its opening
//! (`shaders/portal_glow.wgsl`): a bright core, a glow that is wider on the
//! room side (the side you go in from), a node at each end of the opening, and
//! faint streaks drawn into it from the room side. The room side has the
//! portal's own colour and the other side has its partner's.
//!
//! A portal that is ADDED to a room opens from its middle, and a portal that
//! is REMOVED breaks up and closes. A portal that is in its room when the room
//! comes (authored level content) does not open: it is there.
//!
//! One [`PortalGlow`] entity for each portal, kept from frame to frame: its
//! state is where its two effects are. It reads the portal simulation and
//! writes nothing to it.
//!
//! ## What "added" means here
//!
//! Presentation decides it, from what it sees, by three rules:
//!
//! 1. A portal first seen while its room is younger than
//!    [`ROOM_SETTLE_S`] was there with the room. It does not open.
//! 2. A portal first seen later was added. It opens.
//! 3. A portal entity that is gone and another that comes on the same channel
//!    at the same place in the same frame are one aperture (a rollback gives a
//!    restored portal a new entity). It does not open again, and the old one
//!    does not dissolve.

use std::collections::HashMap;

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d};

use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
use ambition_portal2d::{find_portal, PlacedPortal, PortalChannel, PORTAL_VISUAL_THICKNESS};

use crate::PortalFrames;

/// How long a portal that is added takes to open, in seconds.
pub const APPEAR_S: f32 = 0.42;
/// How long a portal that is removed takes to dissolve, in seconds.
pub const DISSOLVE_S: f32 = 0.5;
/// A portal first seen while its room is younger than this was there with the
/// room: level content is spawned with its room, or in the frames after.
pub const ROOM_SETTLE_S: f32 = 0.25;
/// How far the quad reaches past each end of the opening, in px: room for the
/// end nodes' glow and for the overshoot of the opening.
const PAD_ALONG: f32 = 30.0;
/// How far the quad reaches to each side of the line, in px: room for the
/// glow, the streaks and the bits that lift off a dissolving portal.
const PAD_ACROSS: f32 = 46.0;
/// Two poses nearer than this, in px, are one place.
const SAME_PLACE_PX: f32 = 1.0;

/// `shape`: `x` the opening's length in px, `y` and `z` the quad's size along
/// and across the opening, `w` a seed. `phase`: `x` how far it has opened (0
/// to 1), `y` how far it has dissolved (0 to 1). `front` and `back`: the
/// colour of the room side and of the other side, linear.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, PartialEq)]
pub struct PortalGlowMaterial {
    #[uniform(0)]
    pub shape: Vec4,
    #[uniform(1)]
    pub phase: Vec4,
    #[uniform(2)]
    pub front: Vec4,
    #[uniform(3)]
    pub back: Vec4,
}

impl Material2d for PortalGlowMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://ambition_portal2d_presentation/shaders/portal_glow.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Register the embedded shader and the material. A host with no asset
/// registry (a headless test) draws no glow.
pub(crate) fn add_portal_glow_material_plugin(app: &mut App) {
    if app
        .world()
        .get_resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
        .is_none()
    {
        return;
    }
    if app.is_plugin_added::<Material2dPlugin<PortalGlowMaterial>>() {
        return;
    }
    embedded_asset!(app, "shaders/portal_glow.wgsl");
    app.add_plugins(Material2dPlugin::<PortalGlowMaterial>::default());
}

/// The line of light of one portal, and where its two effects are.
#[derive(Component, Clone, Debug)]
pub struct PortalGlow {
    /// The portal it draws. `None` when that portal is gone: it dissolves.
    pub portal: Option<Entity>,
    pub channel: PortalChannel,
    pub room: LiveRoomInstance,
    /// The portal's last pose, in engine world coordinates.
    pub pos: Vec2,
    pub normal: Vec2,
    /// The opening's length in px.
    pub length: f32,
    /// How far it has opened, 0 to 1. A portal that was there with its room
    /// starts at 1.
    pub appear: f32,
    /// How far it has dissolved, 0 to 1.
    pub dissolve: f32,
}

impl PortalGlow {
    /// Whether `portal` of `room` is this aperture: its channel, at its place.
    fn is_the_aperture(&self, room: LiveRoomInstance, portal: &PlacedPortal) -> bool {
        self.room == room
            && self.channel == portal.channel
            && self.pos.distance(portal.pos) <= SAME_PLACE_PX
            && self.normal.distance(portal.normal.normalize_or_zero()) <= 0.01
    }
}

/// The length of a portal's opening in px: its extent along its wall.
pub fn opening_length(portal: &PlacedPortal) -> f32 {
    let n = portal.normal.normalize_or_zero();
    let along = Vec2::new(-n.y, n.x);
    let half = along.x.abs() * portal.half_extent.x + along.y.abs() * portal.half_extent.y;
    (half * 2.0).max(PORTAL_VISUAL_THICKNESS)
}

/// The quad that draws an opening of `length` at `pos` with `normal`: its long
/// axis along the wall, and its local +y toward the room.
///
/// AMBITION_REVIEW(spatial): the engine is y-down and render space is y-up, so
/// the wall direction's y is negated before the angle is taken.
fn glow_transform(frame: &crate::PortalWorldFrame, pos: Vec2, normal: Vec2, length: f32, z: f32) -> Transform {
    let along = Vec2::new(-normal.y, normal.x);
    let angle = (-along.y).atan2(along.x);
    Transform {
        translation: frame.to_render(pos, z),
        rotation: Quat::from_rotation_z(angle),
        scale: quad_size(length).extend(1.0),
    }
}

fn quad_size(length: f32) -> Vec2 {
    Vec2::new(length + PAD_ALONG * 2.0, PAD_ACROSS * 2.0)
}

fn linear(color: Color) -> Vec4 {
    let c = color.to_linear();
    Vec4::new(c.red, c.green, c.blue, c.alpha)
}

/// A seed from a place, so two portals do not shimmer in step.
fn seed_of(pos: Vec2) -> f32 {
    (pos.x * 0.0131 + pos.y * 0.0173).fract().abs()
}

fn material_of(glow: &PortalGlow, partner: PortalChannel) -> PortalGlowMaterial {
    let size = quad_size(glow.length);
    PortalGlowMaterial {
        shape: Vec4::new(glow.length, size.x, size.y, seed_of(glow.pos)),
        phase: Vec4::new(glow.appear, glow.dissolve, 0.0, 0.0),
        front: linear(glow.channel.display().1),
        back: linear(partner.display().1),
    }
}

/// Keep one [`PortalGlow`] for each portal: open the ones that are added,
/// dissolve the ones that are removed, and follow the ones that move.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync_portal_glows(
    mut commands: Commands,
    time: Res<Time>,
    frames: PortalFrames,
    viewers: Option<Res<crate::PortalViewers>>,
    rigs: Query<&crate::PortalViewRig>,
    portals: Query<(Entity, &PlacedPortal)>,
    mut glows: Query<(Entity, &mut PortalGlow, &mut Transform, &MeshMaterial2d<PortalGlowMaterial>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PortalGlowMaterial>>,
    mut unit_mesh: Local<Option<Handle<Mesh>>>,
    mut room_ages: Local<HashMap<LiveRoomInstance, f32>>,
) {
    let dt = time.delta_secs().min(0.1);
    let by_room = frames.portals_by_room(portals.iter());

    // Each portal that can be drawn: its room is live.
    let live: Vec<(Entity, crate::PortalPlacement, &PlacedPortal)> = portals
        .iter()
        .filter_map(|(entity, portal)| Some((entity, frames.of(entity)?, portal)))
        .collect();

    // How long each live room has been seen, with or without a portal: the
    // first portal of an old room was added to it. A room seen for the first
    // time is new this frame.
    let rooms: Vec<LiveRoomInstance> = frames.live_rooms().collect();
    room_ages.retain(|room, _| rooms.contains(room));
    for room in rooms {
        *room_ages.entry(room).or_insert(-dt) += dt;
    }

    // Glows whose portal is gone. One that went this frame can be the same
    // aperture as a portal that came this frame (rule 3).
    let mut left_this_frame: Vec<Entity> = Vec::new();
    for (entity, mut glow, ..) in &mut glows {
        let gone = glow.portal.is_some_and(|portal| !live.iter().any(|(e, ..)| *e == portal));
        if gone {
            glow.portal = None;
            left_this_frame.push(entity);
        }
    }

    // A glow for each portal that has none.
    for (entity, placement, portal) in &live {
        if glows.iter().any(|(_, glow, ..)| glow.portal == Some(*entity)) {
            continue;
        }
        let same = left_this_frame
            .iter()
            .copied()
            .find(|left| glows.get(*left).is_ok_and(|(_, glow, ..)| glow.is_the_aperture(placement.room, portal)));
        if let Some(same) = same {
            if let Ok((_, mut glow, ..)) = glows.get_mut(same) {
                glow.portal = Some(*entity);
            }
            left_this_frame.retain(|left| *left != same);
            continue;
        }
        let was_there = room_ages.get(&placement.room).is_none_or(|age| *age < ROOM_SETTLE_S);
        let normal = portal.normal.normalize_or_zero();
        let glow = PortalGlow {
            portal: Some(*entity),
            channel: portal.channel,
            room: placement.room,
            pos: portal.pos,
            normal,
            length: opening_length(portal),
            appear: if was_there { 1.0 } else { 0.0 },
            dissolve: 0.0,
        };
        let room_portals = by_room.in_room(Some(placement.room));
        let partner = partner_channel(room_portals, portal.channel);
        let z = crate::visuals::portal_frame_z(*placement, room_portals, portal, viewers.as_deref(), &rigs);
        let mesh = unit_mesh.get_or_insert_with(|| meshes.add(Rectangle::default())).clone();
        commands.spawn((
            Mesh2d(mesh),
            MeshMaterial2d(materials.add(material_of(&glow, partner))),
            glow_transform(&placement.frame, glow.pos, glow.normal, glow.length, z),
            placement.stamp(),
            Name::new(format!("Portal glow ({})", portal.channel.name())),
            glow,
        ));
    }

    // Each glow: follow its portal and open, or dissolve and go.
    for (entity, mut glow, mut transform, material) in &mut glows {
        let Some(placement) = frames.in_room(Some(glow.room)) else {
            // Its room is not live: nothing of it is drawn.
            commands.entity(entity).despawn();
            continue;
        };
        let room_portals = by_room.in_room(Some(glow.room));
        let mut z = transform.translation.z;
        match glow.portal.and_then(|portal| live.iter().find(|(e, ..)| *e == portal)) {
            Some((_, _, portal)) => {
                glow.pos = portal.pos;
                glow.normal = portal.normal.normalize_or_zero();
                glow.length = opening_length(portal);
                glow.channel = portal.channel;
                glow.appear = (glow.appear + dt / APPEAR_S).min(1.0);
                glow.dissolve = 0.0;
                z = crate::visuals::portal_frame_z(placement, room_portals, portal, viewers.as_deref(), &rigs);
            }
            None => {
                glow.dissolve += dt / DISSOLVE_S;
                if glow.dissolve >= 1.0 {
                    commands.entity(entity).despawn();
                    continue;
                }
            }
        }
        let placed = glow_transform(&placement.frame, glow.pos, glow.normal, glow.length, z);
        if *transform != placed {
            *transform = placed;
        }
        // Written only when it changes: a portal at rest costs no upload.
        let wanted = material_of(&glow, partner_channel(room_portals, glow.channel));
        if materials.get(&material.0).is_some_and(|current| *current != wanted) {
            if let Some(mut current) = materials.get_mut(&material.0) {
                *current = wanted;
            }
        }
    }
}

/// The channel drawn on the far side of the line of a portal on `channel`:
/// its partner's when the partner is placed, and its own when it is not.
fn partner_channel(room_portals: &[PlacedPortal], channel: PortalChannel) -> PortalChannel {
    find_portal(room_portals, channel.partner()).map_or(channel, |partner| partner.channel)
}

#[cfg(test)]
mod tests;
