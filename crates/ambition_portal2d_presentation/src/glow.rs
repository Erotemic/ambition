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
//!
//! ## Whose it is
//!
//! A glow is of one gameplay session: it is spawned under that session's
//! scope, and what this system knows of rooms ([`GlowMemory`]) is of that
//! session too. A new session has the same room ordinals, so neither may go
//! from one session to the next: the old glows are taken away, and each room
//! of the new session is new.

use std::collections::HashMap;

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d};

use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, LiveRoomInstance, SessionScopeId, SessionScopedEntity, SessionSpawnScope,
    SpawnSessionScopedExt,
};
use ambition_portal2d::{PlacedPortal, PortalChannel, PORTAL_VISUAL_THICKNESS};

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

/// How the line of a portal is coloured.
///
/// By default a line has one colour, its own. The colour of its partner is
/// seen where it belongs: through the portal, when its window opens and shows
/// the far side with the partner's line in it. A game that has no windows, or
/// that wants each line to say where it leads, turns the partner's colour on:
/// insert this resource with `partner_colour` set.
#[derive(Resource, Reflect, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[reflect(Resource)]
pub struct PortalGlowStyle {
    /// Draw the side of the line that is away from the room in the colour of
    /// the partner portal.
    pub partner_colour: bool,
}

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
    /// The channel that pairs the portal: which aperture this glow is.
    pub channel: PortalChannel,
    /// The channel whose colour it is drawn in: the portal's
    /// [`PortalLook`](ambition_portal2d::PortalLook), or its channel.
    pub look: PortalChannel,
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
    (portal.opening_half_length() * 2.0).max(PORTAL_VISUAL_THICKNESS)
}

/// The quad that draws an opening of `length` at `pos` with `normal`: its long
/// axis along the wall, and its local +y toward the room.
///
/// AMBITION_REVIEW(spatial): the engine is y-down and render space is y-up, so
/// the wall direction's y is negated before the angle is taken.
fn glow_transform(frame: &crate::PortalWorldFrame, pos: Vec2, normal: Vec2, length: f32) -> Transform {
    glow_transform_at(frame, pos, normal, length, crate::PORTAL_FRAME_Z)
}

/// [`glow_transform`] at the render z `z`.
fn glow_transform_at(frame: &crate::PortalWorldFrame, pos: Vec2, normal: Vec2, length: f32, z: f32) -> Transform {
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

fn material_of(glow: &PortalGlow, partner_look: PortalChannel) -> PortalGlowMaterial {
    let size = quad_size(glow.length);
    PortalGlowMaterial {
        shape: Vec4::new(glow.length, size.x, size.y, seed_of(glow.pos)),
        phase: Vec4::new(glow.appear, glow.dissolve, 0.0, 0.0),
        front: linear(glow.look.display().1),
        back: linear(partner_look.display().1),
    }
}

/// What [`sync_portal_glows`] knows from frame to frame: how long each live
/// room has been seen, in the session it was seen in. A room ordinal is not
/// an identity over sessions, so the ages are of one session.
#[derive(Default)]
pub struct GlowMemory {
    session: Option<SessionScopeId>,
    room_ages: HashMap<LiveRoomInstance, f32>,
}

/// Keep one [`PortalGlow`] for each portal: open the ones that are added,
/// dissolve the ones that are removed, and follow the ones that move.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync_portal_glows(
    mut commands: Commands,
    time: Res<Time>,
    frames: PortalFrames,
    portals: Query<(Entity, &PlacedPortal)>,
    looks: Query<&ambition_portal2d::PortalLook>,
    active_session: Option<Res<ActiveSessionScope>>,
    style: Option<Res<PortalGlowStyle>>,
    mut glows: Query<(
        Entity,
        &mut PortalGlow,
        &mut Transform,
        &MeshMaterial2d<PortalGlowMaterial>,
        Option<&SessionScopedEntity>,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PortalGlowMaterial>>,
    mut unit_mesh: Local<Option<Handle<Mesh>>>,
    mut memory: Local<GlowMemory>,
) {
    let dt = time.delta_secs().min(0.1);
    // The session this frame draws. With no session lifecycle installed (a
    // legacy app, a test) there is one unscoped world. With the lifecycle and
    // no current session there is no gameplay to draw.
    let Some(scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        for (entity, ..) in &glows {
            commands.entity(entity).despawn();
        }
        *memory = GlowMemory::default();
        return;
    };
    if memory.session != scope.id() {
        // A new session: its rooms are new, whatever their ordinals are.
        *memory = GlowMemory { session: scope.id(), room_ages: HashMap::new() };
    }
    let room_ages = &mut memory.room_ages;
    // A glow of another session is not of this world: it goes, and it is not
    // matched to a portal of this one.
    let of_this_session = |owner: Option<&SessionScopedEntity>| owner.map(|owner| owner.0) == scope.id();
    for (entity, _, _, _, owner) in &glows {
        if !of_this_session(owner) {
            commands.entity(entity).despawn();
        }
    }
    // Each portal that can be drawn: its room is live.
    let live: Vec<(Entity, crate::PortalPlacement, &PlacedPortal)> = portals
        .iter()
        .filter_map(|(entity, portal)| Some((entity, frames.of(entity)?, portal)))
        .collect();

    // The look of each live portal, by its room and channel, so the far side
    // of a line has its partner's look. The first in the crate's stable order
    // answers, as `find_portal` does.
    let look_of = |entity: Entity, portal: &PlacedPortal| looks.get(entity).map_or(portal.channel, |look| look.0);
    let mut by_order: Vec<&(Entity, crate::PortalPlacement, &PlacedPortal)> = live.iter().collect();
    by_order.sort_by(|a, b| ambition_portal2d::stable_portal_order(a.2, b.2));
    let mut looks_by: HashMap<(LiveRoomInstance, PortalChannel), PortalChannel> = HashMap::new();
    for (entity, placement, portal) in by_order {
        looks_by.entry((placement.room, portal.channel)).or_insert_with(|| look_of(*entity, portal));
    }
    let far_side_look = |room: LiveRoomInstance, channel: PortalChannel, own: PortalChannel| {
        if style.as_deref().is_some_and(|style| style.partner_colour) {
            looks_by.get(&(room, channel.partner())).copied().unwrap_or(own)
        } else {
            own
        }
    };

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
    for (entity, mut glow, _, _, owner) in &mut glows {
        if !of_this_session(owner) {
            continue;
        }
        let gone = glow.portal.is_some_and(|portal| !live.iter().any(|(e, ..)| *e == portal));
        if gone {
            glow.portal = None;
            left_this_frame.push(entity);
        }
    }

    // A glow for each portal that has none.
    for (entity, placement, portal) in &live {
        if glows
            .iter()
            .any(|(_, glow, _, _, owner)| of_this_session(owner) && glow.portal == Some(*entity))
        {
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
            look: look_of(*entity, portal),
            room: placement.room,
            pos: portal.pos,
            normal,
            length: opening_length(portal),
            appear: if was_there { 1.0 } else { 0.0 },
            dissolve: 0.0,
        };
        let partner = far_side_look(placement.room, portal.channel, glow.look);
        let mesh = unit_mesh.get_or_insert_with(|| meshes.add(Rectangle::default())).clone();
        // Of this session and of this room: it leaves with either.
        commands.spawn_session_scoped(scope.in_room(Some(placement.room)), (
            Mesh2d(mesh),
            MeshMaterial2d(materials.add(material_of(&glow, partner))),
            glow_transform(&placement.frame, glow.pos, glow.normal, glow.length),
            Name::new(format!("Portal glow ({})", portal.channel.name())),
            glow,
        ));
    }

    // Each glow: follow its portal and open, or dissolve and go.
    for (entity, mut glow, mut transform, material, owner) in &mut glows {
        if !of_this_session(owner) {
            continue;
        }
        let Some(placement) = frames.in_room(Some(glow.room)) else {
            // Its room is not live: nothing of it is drawn.
            commands.entity(entity).despawn();
            continue;
        };
        match glow.portal.and_then(|portal| live.iter().find(|(e, ..)| *e == portal)) {
            Some((portal_entity, _, portal)) => {
                glow.pos = portal.pos;
                glow.normal = portal.normal.normalize_or_zero();
                glow.length = opening_length(portal);
                glow.channel = portal.channel;
                glow.look = look_of(*portal_entity, portal);
                glow.appear = (glow.appear + dt / APPEAR_S).min(1.0);
                glow.dissolve = 0.0;
            }
            None => {
                glow.dissolve += dt / DISSOLVE_S;
                if glow.dissolve >= 1.0 {
                    commands.entity(entity).despawn();
                    continue;
                }
            }
        }
        let placed = glow_transform(&placement.frame, glow.pos, glow.normal, glow.length);
        if *transform != placed {
            *transform = placed;
        }
        // Written only when it changes: a portal at rest costs no upload.
        let wanted = material_of(&glow, far_side_look(glow.room, glow.channel, glow.look));
        if materials.get(&material.0).is_some_and(|current| *current != wanted) {
            if let Some(mut current) = materials.get_mut(&material.0) {
                *current = wanted;
            }
        }
    }
}

/// A line of a portal's light that is NO portal: nothing goes through it, and
/// the simulation does not know it. A game puts one where a portal is a look
/// (an arm that stays in its portal): on an entity of a live room, and moves
/// it by writing `pos`. [`sync_portal_light_lines`] draws it with the material
/// of a portal's line, and opens it as a portal that was added opens.
///
/// A [`PlacedPortal`] with no partner is not this: a free portal that moves
/// pushes each body on its plane to one side of it.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PortalLightLine {
    /// The channel whose colour the line has.
    pub channel: PortalChannel,
    /// Its middle, in engine world coordinates.
    pub pos: Vec2,
    /// The side of the line that the room is on.
    pub normal: Vec2,
    /// Its length in px.
    pub length: f32,
    /// The render z it is drawn at, when it is not a portal's
    /// (`PORTAL_FRAME_Z`): a line on a limb is drawn over the limb
    /// (`limb_seams`).
    pub depth: Option<f32>,
}

/// On a [`PortalLightLine`] that [`dress_held_light_lines`] keeps: the body
/// whose view has the line, and the line's entry and slot.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct HeldLightLineOf {
    pub owner: Entity,
    pub entry: std::sync::Arc<str>,
    pub slot: u32,
}

/// Keep a [`PortalLightLine`] for each line that a module asked for this
/// tick (each body's `HeldLightLines` view, `ambition.portal.light_line`),
/// in the body's live room. A line that is in no view is taken away, so a
/// line goes when its module stops asking or its body goes.
pub fn dress_held_light_lines(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    holders: Query<(Entity, &ambition_portal2d::extension::HeldLightLines)>,
    mut lines: Query<(
        Entity,
        &HeldLightLineOf,
        &mut PortalLightLine,
        Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        Option<&mut crate::limb_seams::LightLineOnLimb>,
    )>,
) {
    let light_of = |line: &ambition_portal2d::extension::HeldLightLine| PortalLightLine {
        channel: PortalChannel::Authored(ambition_portal2d::PortalChannelColor::Indexed(line.look)),
        pos: line.at,
        normal: line.normal,
        length: line.length,
        depth: None,
    };
    // A line on a limb: where the module put it is kept on its
    // `LightLineOnLimb`, and `lay_light_lines_on_limb_seams` places it.
    let on_limb_of = |line: &ambition_portal2d::extension::HeldLightLine| {
        line.on_limb.as_ref().map(|on| {
            crate::limb_seams::LightLineOnLimb::new(on.limb.clone(), on.on_its_body, (line.at, line.normal, line.length))
        })
    };
    let mut wanted: Vec<(Entity, &ambition_portal2d::extension::HeldLightLine)> =
        holders.iter().flat_map(|(owner, held)| held.0.iter().map(move |line| (owner, line))).collect();
    for (entity, of, mut light, room, on_limb) in &mut lines {
        let Some(index) =
            wanted.iter().position(|(owner, line)| *owner == of.owner && line.entry == of.entry && line.slot == of.slot)
        else {
            commands.entity(entity).try_despawn();
            continue;
        };
        let (owner, line) = wanted.swap_remove(index);
        let want = light_of(line);
        match (on_limb_of(line), on_limb) {
            // On a limb: the module's place is kept, and the line is placed
            // from it and the seam (its colour is the module's).
            (Some(wanted_on), Some(mut on)) => {
                if on.limb != wanted_on.limb || on.on_its_body != wanted_on.on_its_body {
                    *on = wanted_on;
                } else if on.nominal != wanted_on.nominal {
                    on.nominal = wanted_on.nominal;
                }
                if light.channel != want.channel {
                    light.channel = want.channel;
                }
            }
            (wanted_on, had) => {
                match (wanted_on, had) {
                    (Some(on), None) => {
                        commands.entity(entity).insert(on);
                    }
                    (None, Some(_)) => {
                        commands.entity(entity).remove::<crate::limb_seams::LightLineOnLimb>();
                    }
                    _ => {}
                }
                if *light != want {
                    *light = want;
                }
            }
        }
        // The line is drawn in its body's room: a body that crossed takes it.
        if let Some(owner_room) = live.of(owner) {
            if room.map(|room| room.0) != Some(owner_room) {
                commands
                    .entity(entity)
                    .insert(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(owner_room));
            }
        }
    }
    let Some(scope) = SessionSpawnScope::for_optional_active_session(active_session.as_deref()) else {
        return;
    };
    for (owner, line) in wanted {
        let Some(room) = live.of(owner) else {
            continue;
        };
        let spawned = commands
            .spawn_session_scoped(
                scope.in_room(Some(room)),
                (
                    light_of(line),
                    HeldLightLineOf { owner, entry: line.entry.clone(), slot: line.slot },
                    Name::new(format!("light line {} of {}", line.slot, line.entry)),
                ),
            )
            .id();
        if let Some(on) = on_limb_of(line) {
            commands.entity(spawned).insert(on);
        }
    }
}

/// Draw each [`PortalLightLine`]: give it its quad, open it, and follow it.
pub fn sync_portal_light_lines(
    mut commands: Commands,
    time: Res<Time>,
    frames: PortalFrames,
    mut lines: Query<(
        Entity,
        &PortalLightLine,
        Option<&MeshMaterial2d<PortalGlowMaterial>>,
        Option<&mut Transform>,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PortalGlowMaterial>>,
    mut unit_mesh: Local<Option<Handle<Mesh>>>,
) {
    let dt = time.delta_secs().min(0.1);
    for (entity, line, material, transform) in &mut lines {
        let Some(placement) = frames.of(entity) else {
            continue;
        };
        let normal = line.normal.normalize_or_zero();
        let length = line.length.max(PORTAL_VISUAL_THICKNESS);
        let placed =
            glow_transform_at(&placement.frame, line.pos, normal, length, line.depth.unwrap_or(crate::PORTAL_FRAME_Z));
        let size = quad_size(length);
        let colour = linear(line.channel.display().1);
        let (Some(material), Some(mut transform)) = (material, transform) else {
            let mesh = unit_mesh.get_or_insert_with(|| meshes.add(Rectangle::default())).clone();
            commands.entity(entity).insert((
                Mesh2d(mesh),
                MeshMaterial2d(materials.add(PortalGlowMaterial {
                    shape: Vec4::new(length, size.x, size.y, seed_of(line.pos)),
                    phase: Vec4::ZERO,
                    front: colour,
                    back: colour,
                })),
                placed,
                Visibility::default(),
            ));
            continue;
        };
        if *transform != placed {
            *transform = placed;
        }
        let Some(current) = materials.get(&material.0) else {
            continue;
        };
        // The seed stays: a line that moves does not shimmer by its place.
        let wanted = PortalGlowMaterial {
            shape: Vec4::new(length, size.x, size.y, current.shape.w),
            phase: Vec4::new((current.phase.x + dt / APPEAR_S).min(1.0), 0.0, 0.0, 0.0),
            front: colour,
            back: colour,
        };
        if *current != wanted {
            if let Some(mut current) = materials.get_mut(&material.0) {
                *current = wanted;
            }
        }
    }
}

#[cfg(test)]
mod tests;
