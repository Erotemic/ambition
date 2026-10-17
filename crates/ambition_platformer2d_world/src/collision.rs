//! The composited collision world: the authored room folded together with the
//! per-frame dynamic contributions a running sim adds to it.
//!
//! [`CollisionWorld`] is the single collision read-API every actor sweep/raycast
//! should reach for instead of `ambition_platformer2d_shared_tangle::lifecycle::SoleLiveRoom<RoomGeometry>`: it composites the authored
//! room with moving platforms and the ECS overlay so player, NPC, enemy, and
//! projectile all collide against one truth (the relativity principle as a
//! correctness property), never the bare geometry.
//!
//! Lives in the space IR (refactor-chain R3) because every input is now plain:
//! the authored room, a `Vec<MovingPlatformState>` this crate already owns, and
//! `FeatureEcsWorldOverlay` — a content-free struct of `Block`s and `Aabb`s. The
//! rebuild side that PRODUCES the overlay (querying breakables, pogo volumes,
//! and gates) stays actor-side; only the CONSUMPTION side is here.
//!
//! `world_with_sandbox_solids` adds moving-platform + ECS-overlay solids;
//! `world_with_gate_solids_and_carves` is the projectile view. A portal's hole
//! is open only for the bodies that opened it, so it is cut only in the walls
//! a body meets ([`ComposedRooms::solids_for`], [`RoomCollision::solids_for`]).

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::geometry::subtract_aabb;
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::feature_overlay::{FeatureEcsWorldOverlay, PortalGlue};
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::change_detection::Tick;
use bevy_ecs::prelude::{Component, Query, Res, ResMut, Resource, With};
use bevy_ecs::system::SystemParam;
use std::borrow::Cow;
use std::sync::OnceLock;

use crate::platforms::{world_with_moving_platforms, MovingPlatformState};

/// A live room's moving platforms, on that room's own root
/// (`RoomInstanceRoot`), beside its `RoomGeometry` (OW1 cut 3b).
///
/// The scheduled simulation phase advances every live room's platforms once
/// per frame before body integration, and bodies consume the resulting delta.
/// The session's activation writes the first room's set, and a room
/// publication writes the set of the room it publishes, onto the root of the
/// live room it replaces. A second live room has a second set.
///
/// Lives beside [`MovingPlatformState`] rather than a tier up: it is a newtype
/// over this crate's own vocabulary, and [`CollisionWorld`] reads it.
#[derive(Component, Clone, Debug, Default)]
pub struct MovingPlatformSet(pub Vec<MovingPlatformState>);

impl MovingPlatformSet {
    /// Serialize the live moving-platform state to a deterministic byte string.
    ///
    /// Platform kinematics (`pos`, sweep/path cursor, `last_delta`) are mutable
    /// session state that lives only in this resource; visual entities carry
    /// an index. A within-room rollback must restore it, so it is registered
    /// snapshot state. RON round-trips `MovingPlatformState` exactly for the
    /// same-build determinism contract and keeps the private `motion` cursor
    /// inside this crate.
    pub fn to_snapshot_ron(&self) -> String {
        ron::to_string(&self.0).expect("moving-platform state is always serializable")
    }

    /// Rebuild from [`Self::to_snapshot_ron`] bytes. `None` on malformed input so
    /// snapshot restore reports a decode failure rather than silently dropping
    /// platform state.
    pub fn from_snapshot_ron(s: &str) -> Option<Self> {
        ron::from_str::<Vec<MovingPlatformState>>(s).ok().map(Self)
    }
}

/// The single collision read-API. Composites the authored
/// [`ambition_platformer2d_core::RoomGeometry`] with the per-frame dynamic overlay —
/// moving platforms, ECS-owned solids, and portal carves — into the collision
/// world a sweep or raycast should see.
///
/// It reads each live room off that room's own root (OW1 cut 3d), so a body
/// in one live room never collides with another live room's walls.
/// [`Self::room`] takes the room the reader stands in ([`InRoomInstance`]).
/// The shorthand readers ([`Self::solids`] and the others) take no room and
/// answer only while the session has one live room: that is the named debt of
/// every reader that does not yet give its subject's room.
///
/// The platforms and the overlay are optional, so minimal test apps still
/// satisfy the parameter. With no dynamics the result is the bare authored
/// geometry.
#[derive(SystemParam)]
pub struct CollisionWorld<'w, 's> {
    /// Every live room's identity, geometry, moving platforms and collision
    /// overlay, each tuple off ONE root, so the parts cannot come from
    /// different rooms. A hidden candidate's root is not in it.
    rooms: Query<
        'w,
        's,
        (
            bevy_ecs::entity::Entity,
            Option<&'static LiveRoomInstance>,
            Ref<'static, ae::RoomGeometry>,
            Option<Ref<'static, MovingPlatformSet>>,
            Option<Ref<'static, FeatureEcsWorldOverlay>>,
        ),
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
    /// The walls composed so far this tick, which every reader shares. Absent
    /// in an App that does not install it: each reader then composes its own.
    composed: Option<Res<'w, ComposedSolids>>,
}

/// What a room's composed walls are made from: the change tick of its
/// geometry and of its moving platforms. A write to either gives another key.
/// The overlay is compared by its contents
/// ([`FeatureEcsWorldOverlay::composes_like`]), because it is filled again
/// each tick. So walls kept under a key and an overlay are the walls those
/// parts compose to now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolidsKey {
    root: bevy_ecs::entity::Entity,
    geometry: Tick,
    platforms: Option<Tick>,
}

impl SolidsKey {
    fn of(root: bevy_ecs::entity::Entity, geometry: &Ref<ae::RoomGeometry>, platforms: Option<&Ref<MovingPlatformSet>>) -> Self {
        Self { root, geometry: geometry.last_changed(), platforms: platforms.map(DetectChanges::last_changed) }
    }
}

/// One kept composition: its key, the overlay it was composed with, and the
/// walls.
type KeptSolids = (SolidsKey, FeatureEcsWorldOverlay, ae::World);

fn kept_for<'k>(kept: &'k KeptSolids, key: SolidsKey, overlay: &FeatureEcsWorldOverlay) -> Option<&'k ae::World> {
    (kept.0 == key && kept.1.composes_like(overlay)).then_some(&kept.2)
}

/// How many composed worlds are kept at once: a few rooms, each in a few
/// states in one tick. A reader that finds no free place composes its own, as
/// a reader with no cache does.
const COMPOSED_PLACES: usize = 8;

/// The composed walls of each live room ([`RoomCollision::solids`]), composed
/// once for each state of the room's parts and borrowed by every reader. About
/// twenty systems read the walls each tick; each composed a copy of the whole
/// room before (`performance-and-composition-findings.md`, F24, P14).
///
/// A reader fills a free place; [`retire_stale_composed_solids`] frees the
/// places whose parts were written since. A place is used only under its own
/// key, so where the retire runs changes only how many readers share a world,
/// never what a reader collides with.
#[derive(Resource, Default)]
pub struct ComposedSolids {
    places: [OnceLock<KeptSolids>; COMPOSED_PLACES],
}

impl ComposedSolids {
    /// The walls kept under `key` and `overlay`, composed by `compose` into
    /// a free place when none are. `None` when every place holds others.
    fn get_or_compose(&self, key: SolidsKey, overlay: &FeatureEcsWorldOverlay, compose: impl Fn() -> ae::World) -> Option<&ae::World> {
        if let Some(world) = self.places.iter().find_map(|place| kept_for(place.get()?, key, overlay)) {
            return Some(world);
        }
        for place in &self.places {
            if place.get().is_none() {
                // Two readers can race for one place; the loser goes on.
                let _ = place.set((key, overlay.clone(), compose()));
            }
            if let Some(world) = place.get().and_then(|kept| kept_for(kept, key, overlay)) {
                return Some(world);
            }
        }
        None
    }

    /// How many places hold composed walls.
    pub fn kept(&self) -> usize {
        self.places.iter().filter(|place| place.get().is_some()).count()
    }
}

/// Free each place whose room's parts were written since its walls were
/// composed, and keep the others.
#[allow(clippy::type_complexity)]
pub fn retire_stale_composed_solids(
    composed: Option<ResMut<ComposedSolids>>,
    rooms: Query<
        (
            bevy_ecs::entity::Entity,
            Ref<ae::RoomGeometry>,
            Option<Ref<MovingPlatformSet>>,
            Option<Ref<FeatureEcsWorldOverlay>>,
        ),
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
) {
    let Some(mut composed) = composed else {
        return;
    };
    let empty = FeatureEcsWorldOverlay::default();
    let current: Vec<(SolidsKey, &FeatureEcsWorldOverlay)> = rooms
        .iter()
        .map(|(root, geometry, platforms, overlay)| {
            (SolidsKey::of(root, &geometry, platforms.as_ref()), overlay.map_or(&empty, Ref::into_inner))
        })
        .collect();
    let live = |kept: &KeptSolids| current.iter().any(|(key, overlay)| kept_for(kept, *key, overlay).is_some());
    // Read first: in a tick with nothing stale the resource is not written.
    if composed.places.iter().all(|place| place.get().is_none_or(live)) {
        return;
    }
    for place in &mut composed.places {
        if place.get().is_some_and(|kept| !live(kept)) {
            place.take();
        }
    }
}

impl CollisionWorld<'_, '_> {
    /// The collision inputs of the live room `room` names.
    ///
    /// A reader with no room (`None`) gets the sole live room. A reader with a
    /// room gets that room and no other; if no live room, or more than one,
    /// has that identity, it gets `None`. It does not fall back to "the" room,
    /// because that would collide a body with a room it is not in. Two
    /// sessions can each have a room with one ordinal until the identity is
    /// per session (OW1 cut 5); that is ambiguous, and so is `None`.
    pub fn room(&self, room: Option<&InRoomInstance>) -> Option<RoomCollision<'_>> {
        room_of(self.rooms.iter(), room, self.composed.as_deref())
    }

    /// [`RoomCollision::solids`] of the sole live room.
    pub fn solids(&self) -> Option<Cow<'_, ae::World>> {
        self.room(None)?.solids()
    }

    /// [`RoomCollision::hostable_surfaces`] of the sole live room.
    pub fn hostable_surfaces(&self) -> Option<Cow<'_, ae::World>> {
        self.room(None)?.hostable_surfaces()
    }

    /// [`RoomCollision::base`] of the sole live room.
    pub fn base(&self) -> Option<&ae::World> {
        self.room(None).map(RoomCollision::base)
    }
}

/// [`CollisionWorld::room`] for a reader that has the world and not a system
/// parameter (an extension observation reads `&World`): the same rule picks
/// the room. `None` when no room matches, as there.
pub fn with_room_in_world<R>(
    world: &bevy_ecs::world::World,
    room: Option<&InRoomInstance>,
    read: impl FnOnce(RoomCollision<'_>) -> R,
) -> Option<R> {
    // Only what every room root has is queried: a query that names a
    // component type the world never registered (an `Option` part, too) is
    // refused whole. The optional parts are read per root.
    let state = world.try_query_filtered::<
        (bevy_ecs::entity::Entity, &ae::RoomGeometry),
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >()?;
    let rooms = state.query_manual(world);
    let parts = rooms.iter().filter_map(|(root, _)| {
        let entity = world.entity(root);
        Some((
            root,
            world.get::<LiveRoomInstance>(root),
            entity.get_ref::<ae::RoomGeometry>()?,
            entity.get_ref::<MovingPlatformSet>(),
            entity.get_ref::<FeatureEcsWorldOverlay>(),
        ))
    });
    room_of(parts, room, world.get_resource::<ComposedSolids>()).map(read)
}

fn room_of<'a>(
    rooms: impl Iterator<
        Item = (
            bevy_ecs::entity::Entity,
            Option<&'a LiveRoomInstance>,
            Ref<'a, ae::RoomGeometry>,
            Option<Ref<'a, MovingPlatformSet>>,
            Option<Ref<'a, FeatureEcsWorldOverlay>>,
        ),
    >,
    room: Option<&InRoomInstance>,
    composed: Option<&'a ComposedSolids>,
) -> Option<RoomCollision<'a>> {
    let mut matching = rooms.filter(|(_, live, ..)| match room {
        Some(room) => live.copied() == Some(room.0),
        None => true,
    });
    let (root, _, geometry, platforms, overlay) = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let key = SolidsKey::of(root, &geometry, platforms.as_ref());
    Some(RoomCollision {
        geometry: geometry.into_inner(),
        platforms: platforms.map_or(&[][..], |platforms| &platforms.into_inner().0),
        overlay: overlay.map(Ref::into_inner),
        composed: composed.map(|composed| (composed, key)),
    })
}

/// The composed walls ([`RoomCollision::solids`]) of each live room that the
/// subjects of one system run stand in.
///
/// A room is composed on the first ask and then reused, so a run that steps
/// many bodies in one room composes it once. Two bodies in two live rooms get
/// two worlds, each one the walls of its body's own room. Hold one per system
/// run: the overlays and platforms change between ticks.
#[derive(Default)]
pub struct ComposedRooms<'a> {
    rooms: Vec<(Option<InRoomInstance>, Option<Cow<'a, ae::World>>)>,
}

impl<'a> ComposedRooms<'a> {
    /// [`Self::solids`] as `subject` meets them: without the gate solids that
    /// are open for it ([`GatePass`], Q54), and with the portal holes open for
    /// it cut ([`PortalHolePass`]). With neither, which is every tick a body is
    /// not at a gate or a portal, these are the same composed walls, borrowed.
    ///
    /// [`GatePass`]: ambition_platformer2d_shared_tangle::feature_overlay::GatePass
    /// [`PortalHolePass`]: ambition_platformer2d_shared_tangle::feature_overlay::PortalHolePass
    pub fn solids_for(
        &mut self,
        collision: &'a CollisionWorld<'_, '_>,
        room: Option<&InRoomInstance>,
        subject: bevy_ecs::entity::Entity,
    ) -> Option<Cow<'_, ae::World>> {
        let (open, holes, glues) = collision
            .room(room)
            .map(|room| (room.gates_open_for(subject), room.holes_open_for(subject), room.glues_open_for(subject)))
            .unwrap_or_default();
        let walls = self.solids(collision, room)?;
        Some(as_met_by(walls, &open, &holes, &glues))
    }

    /// The walls of the live room `room` names, as [`CollisionWorld::room`]
    /// resolves it. `None` if that room is not live.
    pub fn solids(
        &mut self,
        collision: &'a CollisionWorld<'_, '_>,
        room: Option<&InRoomInstance>,
    ) -> Option<&ae::World> {
        let room = room.copied();
        let index = match self.rooms.iter().position(|(seen, _)| *seen == room) {
            Some(index) => index,
            None => {
                let solids = collision.room(room.as_ref()).and_then(RoomCollision::solids);
                self.rooms.push((room, solids));
                self.rooms.len() - 1
            }
        };
        self.rooms[index].1.as_deref()
    }
}

/// One live room's collision inputs, from [`CollisionWorld::room`].
#[derive(Clone, Copy)]
pub struct RoomCollision<'a> {
    geometry: &'a ae::RoomGeometry,
    platforms: &'a [MovingPlatformState],
    overlay: Option<&'a FeatureEcsWorldOverlay>,
    /// The shared composed walls and this room's key in them.
    composed: Option<(&'a ComposedSolids, SolidsKey)>,
}

/// `walls` as a body meets them: without the gate solids named in `open`
/// ([`RoomCollision::gates_open_for`]) and with `holes` cut
/// ([`RoomCollision::holes_open_for`]). With neither, which is the common case,
/// these are `walls`, borrowed.
pub fn as_met_by<'w>(
    walls: &'w ae::World,
    open: &[&str],
    holes: &[&ae::frame::FaceRect],
    glues: &[&PortalGlue],
) -> Cow<'w, ae::World> {
    let walls = without_gates(walls, open);
    if holes.is_empty() && glues.is_empty() {
        return walls;
    }
    let mut walls = walls.into_owned();
    cut_portal_holes(&mut walls, holes);
    for glue in glues {
        glue_portal_pair(&mut walls, glue);
    }
    Cow::Owned(walls)
}

/// Glue a portal pair into `world` as a body in it meets it
/// (`docs/planning/engine/portals-a-body-in-two-places.md`): behind the
/// opening it is in, out to the glue's depth, the entry's own geometry goes,
/// and what is in front of the exit there is put, mapped back through the
/// pair. The body meets the far side with the part of it that is past the
/// plane.
///
/// Each block is cut by the exit's region exactly (the region is turned with
/// a portal on a slope) and every corner of the cut is mapped. A cut whose
/// image is a box (a floor, wall or ceiling pair) is that box. The world's
/// solids are boxes, so an image a pair turns has no exact solid: it is put
/// as the box that bounds it, which is solid wherever the true image is and
/// a little more (conservative, never less). A one-way block that the pair
/// turns on its side has no kind that says it, and is left out; a line with
/// lanes or junctions, or a closed one, is left out as the hole leaves it
/// whole. The entry's side is cut by the hole's box, as the hole is.
pub fn glue_portal_pair(world: &mut ae::World, glue: &PortalGlue) {
    use ae::frame::{map_point, map_vec, FaceRect};
    let (enter, exit) = (&glue.enter, &glue.exit);
    let half_out = glue.depth * 0.5;
    let behind = FaceRect {
        center: enter.frame.origin - enter.frame.normal * half_out,
        normal: enter.frame.normal,
        half_along: enter.half_length,
        half_out,
    };
    let before = FaceRect {
        center: exit.frame.origin + exit.frame.normal * half_out,
        normal: exit.frame.normal,
        half_along: exit.half_length,
        half_out,
    };
    let back = |p: ae::Vec2| map_point(&exit.frame, &enter.frame, glue.convention, p);
    let back_dir = |v: ae::Vec2| map_vec(&exit.frame, &enter.frame, glue.convention, v);
    // What is in front of the exit, read before the entry's side is cut: in a
    // thin wall the two regions share space.
    let region = ae::frame::thread::face_rect_planes(&before);
    let mut blocks: Vec<ae::Block> = Vec::new();
    for block in &world.blocks {
        let Some(kind) = carried_kind(block.kind, back_dir) else {
            continue;
        };
        let Some(aabb) = carried_box(block.aabb, &region, back) else {
            continue;
        };
        blocks.push(ae::Block { aabb, kind, velocity: back_dir(block.velocity), ..block.clone() });
    }
    let mut chains: Vec<ae::SurfaceChain> = Vec::new();
    for chain in world.chains.iter().filter(|c| !c.closed && c.junctions.is_empty() && c.depth_lanes.is_empty()) {
        for pair in chain.points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if let Some((t0, t1)) = before.clip_segment(a, b) {
                chains.push(ae::SurfaceChain {
                    points: vec![back(a + (b - a) * t0), back(a + (b - a) * t1)],
                    velocity: back_dir(chain.velocity),
                    earth: Vec::new(),
                    filled: false,
                    ..chain.clone()
                });
            }
        }
    }
    cut_portal_holes(world, &[&behind]);
    world.blocks.extend(blocks);
    world.blocks.sort_by(ae::Block::canonical_cmp);
    world.chains.extend(chains);
}

/// The part of `aabb` inside `region` (exactly, a convex cut), mapped by
/// `back`: the box it is when the map keeps boxes boxes, else the box that
/// bounds it. `None` when no area of it is inside.
fn carried_box(
    aabb: ae::Aabb,
    region: &[ae::frame::thread::HalfPlane],
    back: impl Fn(ae::Vec2) -> ae::Vec2,
) -> Option<ae::Aabb> {
    let outline = vec![
        aabb.min,
        ae::Vec2::new(aabb.max.x, aabb.min.y),
        aabb.max,
        ae::Vec2::new(aabb.min.x, aabb.max.y),
    ];
    let cut = ae::frame::thread::clip_convex(outline, region)?;
    let mapped: Vec<ae::Vec2> = cut.into_iter().map(back).collect();
    let (lo, hi) = mapped
        .iter()
        .fold((ae::Vec2::splat(f32::MAX), ae::Vec2::splat(f32::MIN)), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    ae::geometry::aabb_from_min_max(lo.x, lo.y, hi.x, hi.y)
}

/// A block's kind carried through a pair whose map of directions is `dir`:
/// one that is solid from one side keeps the side it is solid from, and has
/// no kind when the pair turns that side to a wall.
fn carried_kind(kind: ae::BlockKind, dir: impl Fn(ae::Vec2) -> ae::Vec2) -> Option<ae::BlockKind> {
    use ae::BlockKind;
    let down = dir(ae::Vec2::Y);
    let upright = down.distance(ae::Vec2::Y) < 1e-4;
    let upside_down = down.distance(-ae::Vec2::Y) < 1e-4;
    match kind {
        BlockKind::OneWay if upright => Some(BlockKind::OneWay),
        BlockKind::OneWay if upside_down => Some(BlockKind::BonkOnly),
        BlockKind::BonkOnly if upright => Some(BlockKind::BonkOnly),
        BlockKind::BonkOnly if upside_down => Some(BlockKind::OneWay),
        BlockKind::OneWay | BlockKind::BonkOnly => None,
        BlockKind::Rebound { impulse } => Some(BlockKind::Rebound { impulse: dir(impulse) }),
        other => Some(other),
    }
}

/// Cut portal holes out of `world`: each hole's box out of the boxes, and its
/// face out of the lines (a portal on a slope), so a body can sink into a
/// portal through the surface it stood on.
pub fn cut_portal_holes(world: &mut ae::World, holes: &[&ae::frame::FaceRect]) {
    let boxes: Vec<ae::Aabb> = holes.iter().map(|face| face.bounds()).collect();
    carve_portal_apertures(&mut world.blocks, &boxes);
    for hole in holes {
        world.chains = world.chains.iter().flat_map(|chain| chain.without(hole)).collect();
    }
}

/// `walls` without the gate solids named in `open`: the walls a body meets when
/// those gates are open for it ([`RoomCollision::gates_open_for`]). With no open
/// gate, which is the common case, these are `walls`, borrowed.
pub fn without_gates<'w>(walls: &'w ae::World, open: &[&str]) -> Cow<'w, ae::World> {
    if open.is_empty() {
        return Cow::Borrowed(walls);
    }
    let mut walls = walls.clone();
    walls.blocks.retain(|block| !open.contains(&block.name.as_str()));
    Cow::Owned(walls)
}

impl<'a> RoomCollision<'a> {
    /// The names of this room's gate solids that are open for `subject`: the
    /// gates whose [`GatePass`] lists it (Q54). This is the one rule for which
    /// gates a body passes. The integrator ([`ComposedRooms::solids_for`]) and
    /// the brain's movement queries both read it.
    ///
    /// [`GatePass`]: ambition_platformer2d_shared_tangle::feature_overlay::GatePass
    pub fn gates_open_for(&self, subject: bevy_ecs::entity::Entity) -> Vec<&'a str> {
        self.overlay
            .map(|overlay| {
                overlay
                    .gate_passes
                    .iter()
                    .filter(|pass| pass.bodies.contains(&subject))
                    .map(|pass| pass.block.as_str())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The full collision world: authored room + moving platforms + ECS solids,
    /// with portal apertures carved. This is what actor sweeps and traversal
    /// raycasts (grapple / blink / dive / body-mode clearance / dropped items)
    /// read so they collide with everything solid this frame.
    ///
    /// Borrows the base geometry on the no-dynamics fast path so the common
    /// case never clones.
    pub fn solids(self) -> Option<Cow<'a, ae::World>> {
        let overlay_empty = self.overlay.map_or(true, |o| {
            o.blocks.is_empty()
                && o.chains.is_empty()
                && o.gate_solids.is_empty()
                && o.removed_block_names.is_empty()
                && o.climbable_carves.is_empty()
                && o.water_regions.is_empty()
        });
        if self.platforms.is_empty() && overlay_empty {
            return Some(Cow::Borrowed(&self.geometry.0));
        }
        let compose = || {
            let default_overlay = FeatureEcsWorldOverlay::default();
            world_with_sandbox_solids(&self.geometry.0, self.platforms, self.overlay.unwrap_or(&default_overlay))
        };
        let empty = FeatureEcsWorldOverlay::default();
        let overlay = self.overlay.unwrap_or(&empty);
        if let Some(shared) = self.composed.and_then(|(composed, key)| composed.get_or_compose(key, overlay, compose)) {
            return Some(Cow::Borrowed(shared));
        }
        Some(Cow::Owned(compose()))
    }

    /// [`Self::solids`] as `subject` meets them ([`as_met_by`]): the gates
    /// open for it passed, the portal holes open for it cut.
    pub fn solids_for(self, subject: bevy_ecs::entity::Entity) -> Option<Cow<'a, ae::World>> {
        let walls = self.solids()?;
        let (open, holes, glues) = (self.gates_open_for(subject), self.holes_open_for(subject), self.glues_open_for(subject));
        if open.is_empty() && holes.is_empty() && glues.is_empty() {
            return Some(walls);
        }
        Some(Cow::Owned(as_met_by(&walls, &open, &holes, &glues).into_owned()))
    }

    /// The portal pairs `subject` is in this tick ([`PortalGluePass`]): it
    /// meets the far side of each with its part past the plane.
    ///
    /// [`PortalGluePass`]: ambition_platformer2d_shared_tangle::feature_overlay::PortalGluePass
    pub fn glues_open_for(&self, subject: bevy_ecs::entity::Entity) -> Vec<&'a PortalGlue> {
        self.overlay
            .map(|overlay| {
                overlay
                    .portal_glues
                    .iter()
                    .filter(|pass| pass.bodies.contains(&subject))
                    .map(|pass| &pass.glue)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The portal holes open for `subject` this tick: the ones it opened
    /// ([`PortalHolePass`]). For every other body, and every reader that names
    /// no body, the host is whole.
    ///
    /// [`PortalHolePass`]: ambition_platformer2d_shared_tangle::feature_overlay::PortalHolePass
    pub fn holes_open_for(&self, subject: bevy_ecs::entity::Entity) -> Vec<&'a ae::frame::FaceRect> {
        self.overlay
            .map(|overlay| {
                overlay
                    .portal_holes
                    .iter()
                    .filter(|hole| hole.bodies.contains(&subject))
                    .map(|hole| &hole.face)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The surfaces a portal may ANCHOR to: authored geometry plus moving
    /// platforms, UNCARVED.
    ///
    /// It differs from [`Self::solids`] in two ways, both required by portals:
    ///
    /// * Uncarved: a portal is placed on a surface and its aperture is
    ///   subtracted afterwards. A carved view would let a portal be placed in
    ///   another portal's hole.
    /// * No ECS overlay: a gate's lock wall is a transient content solid, not
    ///   a surface an aperture should outlive.
    ///
    /// No consumer outside this module builds a collision world itself.
    /// The surfaces a portal rides, to look up the one face or line a
    /// hosted portal is on each tick: the same surfaces as
    /// [`Self::hostable_surfaces`], read in place instead of copied together.
    pub fn host_surfaces(self) -> HostSurfaces<'a> {
        HostSurfaces { authored: &self.geometry.0, platforms: self.platforms }
    }

    pub fn hostable_surfaces(self) -> Option<Cow<'a, ae::World>> {
        if self.platforms.is_empty() {
            return Some(Cow::Borrowed(&self.geometry.0));
        }
        Some(Cow::Owned(world_with_moving_platforms(
            &self.geometry.0,
            self.platforms,
        )))
    }

    /// The bare authored geometry, no overlay. For metadata / bounds / layout
    /// reads only — never for collision. Prefer `solids()` / `solids_for()`.
    /// The room's collision overlay (contributed solids, gate solids,
    /// carves), when it has one.
    pub fn overlay(self) -> Option<&'a FeatureEcsWorldOverlay> {
        self.overlay
    }

    pub fn base(self) -> &'a ae::World {
        &self.geometry.0
    }
}

/// [`RoomCollision::host_surfaces`]: the authored world and the moving
/// platforms, uncarved, read where they are.
#[derive(Clone, Copy)]
pub struct HostSurfaces<'a> {
    authored: &'a ae::World,
    platforms: &'a [MovingPlatformState],
}

impl<'a> HostSurfaces<'a> {
    /// Where the face `face_ref` names is now: on an authored block or on a
    /// moving platform.
    pub fn resolve_face(&self, face_ref: &ae::GeoFaceRef) -> Option<ae::world::FaceAnchor> {
        if let Some(anchor) = self.authored.resolve_face(face_ref) {
            return Some(anchor);
        }
        self.platforms
            .iter()
            .filter(|platform| platform.plank.is_none())
            .map(MovingPlatformState::as_collision_block)
            .find(|block| block.id == face_ref.geo)
            .and_then(|block| block.face_anchor(face_ref))
    }

    /// The line named `name`: an authored one, or a plank platform's.
    pub fn line(&self, name: &str) -> Option<Cow<'a, ae::SurfaceChain>> {
        if let Some(chain) = self.authored.chains.iter().find(|chain| chain.name == name) {
            return Some(Cow::Borrowed(chain));
        }
        self.platforms
            .iter()
            .find(|platform| platform.id == name)
            .and_then(MovingPlatformState::as_collision_line)
            .map(Cow::Owned)
    }
}

pub fn world_with_sandbox_solids(
    world: &ae::World,
    platforms: &[MovingPlatformState],
    ecs_overlay: &FeatureEcsWorldOverlay,
) -> ae::World {
    let mut collision_world = world_with_moving_platforms(world, platforms);
    // A gate contributes these instead of mutating the authored base mid-room.
    apply_overlay_subtractions(&mut collision_world, ecs_overlay);
    extend_in_canonical_order(&mut collision_world.blocks, &ecs_overlay.blocks);
    // Contributed lines after the room's own, by name, so the composed world
    // does not depend on the order the contributors ran in.
    let start = collision_world.chains.len();
    collision_world.chains.extend(ecs_overlay.chains.iter().cloned());
    collision_world.chains[start..].sort_by(|a, b| a.name.cmp(&b.name));
    // Gate solids (lock walls) are authored-equivalent statics: added alongside
    // the base/platform solids, so a portal's hole (`as_met_by`) splits them
    // exactly as it would a base wall.
    extend_in_canonical_order(&mut collision_world.blocks, &ecs_overlay.gate_solids);
    // Additive liquid folds in alongside the base water regions. This keeps the
    // authored base immutable while a contributor's liquid is a per-frame overlay
    // contribution like the solids above.
    collision_world
        .water_regions
        .extend(ecs_overlay.water_regions.iter().cloned());
    // Portal holes are not cut here: a hole is open only for the bodies that
    // opened it (`as_met_by`).
    collision_world
}

/// Append contributed blocks in [`ae::Block::canonical_cmp`] order, so the
/// composed world does not depend on the order the contributors ran in.
fn extend_in_canonical_order(blocks: &mut Vec<ae::Block>, contributed: &[ae::Block]) {
    let start = blocks.len();
    blocks.extend(contributed.iter().cloned());
    blocks[start..].sort_by(ae::Block::canonical_cmp);
}

/// The room world with only portal apertures carved out. Projectiles do not use
/// moving-platform or ECS-overlay solids, but must be able to enter portal
/// openings.
///
/// Returns `Cow::Borrowed(world)` when there are no active carves — the common
/// case (no body in a portal opening, or no portals at all) — so the per-frame
/// projectile steps don't clone the whole block list every frame for nothing.
pub fn world_with_portal_carves<'w>(
    world: &'w ae::World,
    portal_carves: &[ae::Aabb],
) -> Cow<'w, ae::World> {
    if portal_carves.is_empty() {
        return Cow::Borrowed(world);
    }
    let mut carved = world.clone();
    carve_portal_apertures(&mut carved.blocks, portal_carves);
    Cow::Owned(carved)
}

/// The room world with gate solids (lock walls) added and ONLY the portal
/// apertures carved — moving-platform and ECS-breakable solids omitted. This is
/// the projectile collision world: projectiles pass through moving platforms but
/// must collide with gate solids exactly as they did when lock walls lived in
/// the authored base. Borrows (no clone) when there are neither gate solids nor
/// active carves — the common case.
pub fn world_with_gate_solids_and_carves<'w>(
    world: &'w ae::World,
    gate_solids: &[ae::Block],
    portal_carves: &[ae::Aabb],
    removed_block_names: &[String],
) -> Cow<'w, ae::World> {
    world_with_contributed_solids_and_carves(
        world,
        gate_solids,
        &[],
        portal_carves,
        removed_block_names,
    )
}

/// The same composition, plus the surfaces CONTRIBUTED BY LIVE OBJECTS
/// (`FeatureEcsWorldOverlay::blocks` — a solid crate, a one-way ledge an object
/// publishes).
///
/// This is a separate slice from `gate_solids` because the facts differ. A
/// gate solid is geometry a gate opens and closes. A contributed surface
/// belongs to a thing that can be damaged and can stop existing, so only it
/// needs the rule "the wall that stopped this shot is the target".
///
/// Each contributed block carries its owning occurrence in `GeoId`, which
/// makes that rule possible. See the projectile contact protocol.
pub fn world_with_contributed_solids_and_carves<'w>(
    world: &'w ae::World,
    gate_solids: &[ae::Block],
    contributed_solids: &[ae::Block],
    portal_carves: &[ae::Aabb],
    removed_block_names: &[String],
) -> Cow<'w, ae::World> {
    if gate_solids.is_empty()
        && contributed_solids.is_empty()
        && portal_carves.is_empty()
        && removed_block_names.is_empty()
    {
        return Cow::Borrowed(world);
    }
    let mut composed = world.clone();
    // Subtract authored blocks a gate has opened (the gnu_ton floor-gate) so a
    // shot passes through it exactly as the player does, then add gate solids +
    // carve.
    remove_named_blocks(&mut composed.blocks, removed_block_names);
    extend_in_canonical_order(&mut composed.blocks, gate_solids);
    extend_in_canonical_order(&mut composed.blocks, contributed_solids);
    if !portal_carves.is_empty() {
        carve_portal_apertures(&mut composed.blocks, portal_carves);
    }
    Cow::Owned(composed)
}

/// Apply a content gate's authored-geometry SUBTRACTIONS to a composited world:
/// drop blocks whose name is in `removed_block_names`, and drop climbable regions
/// intersecting any `climbable_carves` AABB. The inverse of adding `gate_solids` —
/// it lets a gate open an authored solid / hide an authored ladder without
/// touching the immutable base. No-op (and no allocation scan) when both lists are
/// empty.
fn apply_overlay_subtractions(world: &mut ae::World, overlay: &FeatureEcsWorldOverlay) {
    if !overlay.removed_block_names.is_empty() {
        world
            .blocks
            .retain(|b| !overlay.removed_block_names.iter().any(|n| n == &b.name));
    }
    if !overlay.climbable_carves.is_empty() {
        world.climbable_regions.retain(|r| {
            !overlay
                .climbable_carves
                .iter()
                .any(|c| r.aabb.strict_intersects(*c))
        });
    }
}

/// Drop authored blocks named in `removed_block_names` from a block list (the
/// projectile-view half of [`apply_overlay_subtractions`]; projectiles don't read
/// climbable regions). No-op when the list is empty.
fn remove_named_blocks(blocks: &mut Vec<ae::Block>, removed_block_names: &[String]) {
    if !removed_block_names.is_empty() {
        blocks.retain(|b| !removed_block_names.iter().any(|n| n == &b.name));
    }
}

/// Split every solid host block by the portal aperture holes, leaving a doorway
/// in the surface (and a solid frame around it). Non-host kinds (hazard, pogo,
/// rebound) pass through untouched.
///
/// The set-difference itself is `ae::geometry::subtract_aabb` — plain rectangle
/// algebra in the foundation. This crate never names the portal MECHANIC; a
/// carve arrives as a `Vec<Aabb>` on the overlay.
fn carve_portal_apertures(blocks: &mut Vec<ae::Block>, holes: &[ae::Aabb]) {
    let original = std::mem::take(blocks);
    for block in original {
        let carvable = ae::collision_semantics::is_support_surface(block.kind);
        if !carvable {
            blocks.push(block);
            continue;
        }
        // Subtract each hole in turn; a block can be split by more than one
        // portal (rare, but cheap to handle).
        let mut pieces = vec![block.aabb];
        for hole in holes {
            let mut next = Vec::with_capacity(pieces.len());
            for piece in pieces.drain(..) {
                subtract_aabb(piece, *hole, &mut next);
            }
            pieces = next;
        }
        for aabb in pieces {
            blocks.push(ae::Block {
                id: ae::GeoId::anon(),
                name: block.name.clone(),
                aabb,
                kind: block.kind,
                // A carved piece of a moving host keeps its motion.
                velocity: block.velocity,
                art_color: None,
            });
        }
    }
}

#[cfg(test)]
mod moving_platform_snapshot_tests {
    use super::*;

    fn advanced_set() -> MovingPlatformSet {
        let mut sweep = MovingPlatformState::from_sweep(
            "lift_a",
            "Lift A",
            ae::Vec2::new(10.0, 20.0),
            ae::Vec2::new(32.0, 8.0),
            48.0,
            30.0,
        );
        // Advance the platform so `pos`, the sweep cursor, and `last_delta` all
        // diverge from the authored start — the exact mutable state a rollback
        // must reconstruct.
        for _ in 0..7 {
            sweep.update(1.0 / 60.0);
        }
        let path = MovingPlatformState::from_path(
            "patrol_b",
            "Patrol B",
            ae::Vec2::new(16.0, 16.0),
            ae::KinematicPath::line(ae::Vec2::new(0.0, 0.0), ae::Vec2::new(100.0, 0.0), 40.0),
        );
        MovingPlatformSet(vec![sweep, path])
    }

    #[test]
    fn snapshot_ron_round_trips_advanced_platform_state() {
        let set = advanced_set();
        let bytes = set.to_snapshot_ron();
        let restored =
            MovingPlatformSet::from_snapshot_ron(&bytes).expect("advanced state decodes");
        assert_eq!(
            set.0, restored.0,
            "moving-platform snapshot lost mutable kinematic state on round-trip"
        );
    }

    #[test]
    fn snapshot_ron_round_trips_the_empty_set() {
        let set = MovingPlatformSet::default();
        let bytes = set.to_snapshot_ron();
        let restored = MovingPlatformSet::from_snapshot_ron(&bytes).expect("empty set decodes");
        assert!(restored.0.is_empty());
    }

    #[test]
    fn malformed_snapshot_bytes_decode_to_none() {
        assert!(MovingPlatformSet::from_snapshot_ron("not ron at all {{{").is_none());
    }
}

#[cfg(test)]
mod collision_world_tests {
    use super::*;
    use bevy_app::{App, Update};
    use bevy_ecs::prelude::{ResMut, Resource};

    /// Captured `(was_owned, block_count)` from a `CollisionWorld::solids()` read,
    /// so a system can report the borrow/own decision out of the App.
    #[derive(Resource, Default, Debug, PartialEq)]
    struct SolidsProbe(Option<(bool, usize)>);

    fn room_one_block() -> ae::RoomGeometry {
        ae::RoomGeometry(ae::World::new(
            "test",
            ae::Vec2::new(400.0, 400.0),
            ae::Vec2::new(50.0, 50.0),
            vec![ae::Block {
                id: ae::GeoId::anon(),
                name: "floor".into(),
                aabb: ae::Aabb::new(ae::Vec2::new(200.0, 380.0), ae::Vec2::new(200.0, 20.0)),
                kind: ae::BlockKind::Solid,
                velocity: ae::Vec2::ZERO,
                art_color: None,
            }],
        ))
    }

    fn probe_solids(world: CollisionWorld, mut out: ResMut<SolidsProbe>) {
        out.0 = world
            .solids()
            .map(|w| (matches!(w, Cow::Owned(_)), w.blocks.len()));
    }

    fn run(app: &mut App) -> Option<(bool, usize)> {
        app.add_systems(Update, probe_solids);
        app.update();
        app.world().resource::<SolidsProbe>().0
    }

    #[test]
    fn no_room_yields_none() {
        let mut app = App::new();
        app.init_resource::<SolidsProbe>();
        assert_eq!(run(&mut app), None);
    }

    #[test]
    fn no_dynamics_borrows_base() {
        let mut app = App::new();
        app.init_resource::<SolidsProbe>();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            room_one_block(),
        );
        // No platforms, no overlay → borrow the base, identical block count.
        assert_eq!(run(&mut app), Some((false, 1)));
    }

    #[test]
    fn empty_overlay_still_borrows() {
        let mut app = App::new();
        app.init_resource::<SolidsProbe>();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            room_one_block(),
        );
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay::default());
        // An empty overlay is still the no-dynamics fast path.
        assert_eq!(run(&mut app), Some((false, 1)));
    }

    #[test]
    fn overlay_solids_compose_owned() {
        let mut app = App::new();
        app.init_resource::<SolidsProbe>();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            room_one_block(),
        );
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay {
            blocks: vec![ae::Block {
                id: ae::GeoId::anon(),
                name: "ecs-solid".into(),
                aabb: ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(10.0, 10.0)),
                kind: ae::BlockKind::Solid,
                velocity: ae::Vec2::ZERO,
                art_color: None,
            }],
            ..Default::default()
        });
        // A non-empty overlay forces an owned composite: base + the ECS solid.
        assert_eq!(run(&mut app), Some((true, 2)));
    }

    /// Where each reader's composed walls are kept (`None`: composed for that
    /// reader alone), and how many blocks they hold.
    #[derive(Resource, Default, Debug)]
    struct SharedProbe(Vec<(Option<usize>, usize)>);

    fn probe_shared(world: CollisionWorld, mut out: ResMut<SharedProbe>) {
        if let Some(walls) = world.solids() {
            let kept = match &walls {
                Cow::Borrowed(kept) => Some(*kept as *const ae::World as usize),
                Cow::Owned(_) => None,
            };
            out.0.push((kept, walls.blocks.len()));
        }
    }

    fn ecs_solid(name: &str) -> ae::Block {
        ae::Block::solid(name, ae::Vec2::new(100.0, 100.0), ae::Vec2::new(10.0, 10.0))
    }

    /// The readers of one tick share one composed world (F24, P14); a write to
    /// the room's overlay gives the next reader walls composed from it, and
    /// the retire frees the walls of the old overlay.
    #[test]
    fn the_readers_of_a_tick_share_one_composed_world_and_a_write_composes_again() {
        let mut app = App::new();
        app.init_resource::<SharedProbe>();
        app.init_resource::<ComposedSolids>();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), room_one_block());
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            FeatureEcsWorldOverlay { blocks: vec![ecs_solid("crate")], ..Default::default() },
        );
        use bevy_ecs::schedule::IntoScheduleConfigs;
        app.add_systems(Update, (retire_stale_composed_solids, probe_shared, probe_shared).chain());
        app.update();
        let first = std::mem::take(&mut app.world_mut().resource_mut::<SharedProbe>().0);
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].1, 2, "the premise: base + the ECS solid");
        assert!(first[0].0.is_some() && first[0].0 == first[1].0, "two readers of one tick composed two worlds: {first:?}");

        app.update();
        let still = std::mem::take(&mut app.world_mut().resource_mut::<SharedProbe>().0);
        assert_eq!(still[0].0, first[0].0, "walls of unchanged parts were composed again the next tick");

        let mut overlay = app.world_mut().query::<&mut FeatureEcsWorldOverlay>();
        // The overlay is cleared and filled again each tick: the same contents
        // are the same walls.
        {
            let mut refilled = overlay.single_mut(app.world_mut()).unwrap();
            refilled.clear_engine_contributions();
            refilled.blocks.push(ecs_solid("crate"));
        }
        app.update();
        let refilled = std::mem::take(&mut app.world_mut().resource_mut::<SharedProbe>().0);
        assert_eq!(refilled[0].0, first[0].0, "an overlay filled again with the same contents was composed again");
        overlay.single_mut(app.world_mut()).unwrap().blocks.push(ecs_solid("barrel"));
        app.update();
        let after = std::mem::take(&mut app.world_mut().resource_mut::<SharedProbe>().0);
        assert_eq!((after[0].1, after[1].1), (3, 3), "a reader after the write collides with the old overlay");
        assert!(after[0].0.is_some() && after[0].0 == after[1].0, "the readers after the write do not share: {after:?}");
        assert_eq!(app.world().resource::<ComposedSolids>().kept(), 1, "the walls of the old overlay are kept");
    }

    fn gate_wall() -> ae::Block {
        ae::Block::solid(
            "lockwall:test_encounter",
            ae::Vec2::new(300.0, 300.0),
            ae::Vec2::new(16.0, 100.0),
        )
    }

    #[test]
    fn gate_solids_compose_into_the_player_collision_view() {
        let mut app = App::new();
        app.init_resource::<SolidsProbe>();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            room_one_block(),
        );
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay {
            gate_solids: vec![gate_wall()],
            ..Default::default()
        });
        // A gate solid is a dynamic contribution → owned composite of base + wall.
        assert_eq!(run(&mut app), Some((true, 2)));
    }

    #[test]
    fn gate_solids_compose_into_the_projectile_collision_view() {
        // Projectiles read base + gate solids + carves (NOT moving platforms /
        // breakables). A lock wall must stop a shot exactly as it did when it
        // lived in the authored base.
        let room = room_one_block();
        let gates = vec![gate_wall()];
        let view = world_with_gate_solids_and_carves(&room.0, &gates, &[], &[]);
        assert!(
            matches!(view, Cow::Owned(_)),
            "gate solids force an owned projectile view"
        );
        assert_eq!(view.blocks.len(), 2, "base floor + the gate wall");
        assert!(view
            .blocks
            .iter()
            .any(|b| b.name == "lockwall:test_encounter"));

        // No gate solids and no carves borrows the base (no per-frame clone).
        let none: Vec<ae::Block> = Vec::new();
        let borrowed = world_with_gate_solids_and_carves(&room.0, &none, &[], &[]);
        assert!(matches!(borrowed, Cow::Borrowed(_)));
    }

    /// The block names each reader composes: the sole room, #0 and #1.
    #[derive(Resource, Default, Debug)]
    struct KeyedProbe(Vec<Option<Vec<String>>>);

    fn probe_keyed(world: CollisionWorld, mut out: ResMut<KeyedProbe>) {
        let names = |room: Option<InRoomInstance>| {
            world.room(room.as_ref()).and_then(RoomCollision::solids).map(|solids| {
                solids.blocks.iter().map(|block| block.name.clone()).collect::<Vec<_>>()
            })
        };
        let first = InRoomInstance(LiveRoomInstance::ACTIVATION);
        let second = InRoomInstance(LiveRoomInstance::ACTIVATION.next());
        out.0 = vec![names(None), names(Some(first)), names(Some(second))];
    }

    fn a_live_room(app: &mut App, instance: LiveRoomInstance, blocks: Vec<ae::Block>) {
        let geometry = ae::World::new("test", ae::Vec2::new(400.0, 400.0), ae::Vec2::new(50.0, 50.0), blocks);
        app.world_mut().spawn((
            ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot,
            instance,
            ae::RoomGeometry(geometry),
            FeatureEcsWorldOverlay::default(),
        ));
    }

    /// GATE-PER-ACTOR (Q54): a gate solid open for one body is absent from the
    /// walls that body meets, and present in the walls every other body and
    /// every reader with no body meets. The control: with no pass, every body
    /// meets the composed walls.
    #[test]
    fn a_gate_open_for_one_body_is_missing_only_from_that_body_s_walls() {
        #[derive(Resource, Default, Debug)]
        struct Probe(Vec<Vec<String>>);
        #[derive(Resource)]
        struct Bodies([bevy_ecs::entity::Entity; 2]);
        fn probe(world: CollisionWorld, bodies: bevy_ecs::prelude::Res<Bodies>, mut out: ResMut<Probe>) {
            let mut composed = ComposedRooms::default();
            let names = |walls: &ae::World| walls.blocks.iter().map(|block| block.name.clone()).collect::<Vec<_>>();
            let mut seen: Vec<Vec<String>> = bodies
                .0
                .iter()
                .map(|body| names(&composed.solids_for(&world, None, *body).expect("one live room")))
                .collect();
            seen.push(names(composed.solids(&world, None).expect("one live room")));
            out.0 = seen;
        }
        let mut app = App::new();
        app.init_resource::<Probe>();
        let floor = ae::Block::solid("floor", ae::Vec2::new(0.0, 380.0), ae::Vec2::new(400.0, 20.0));
        a_live_room(&mut app, LiveRoomInstance::ACTIVATION, vec![floor]);
        let bodies = [app.world_mut().spawn_empty().id(), app.world_mut().spawn_empty().id()];
        app.insert_resource(Bodies(bodies));
        app.add_systems(Update, probe);
        let mut overlay = app.world_mut().query::<&mut FeatureEcsWorldOverlay>();
        overlay.single_mut(app.world_mut()).expect("one overlay").gate_solids.push(gate_wall());
        app.update();
        let both = vec!["floor".to_string(), "lockwall:test_encounter".to_string()];
        assert_eq!(app.world().resource::<Probe>().0, vec![both.clone(); 3], "no pass: every body meets the gate");

        overlay
            .single_mut(app.world_mut())
            .expect("one overlay")
            .gate_passes
            .push(ambition_platformer2d_shared_tangle::feature_overlay::GatePass {
                block: "lockwall:test_encounter".to_string(),
                bodies: vec![bodies[0]],
            });
        app.update();
        assert_eq!(
            app.world().resource::<Probe>().0,
            vec![vec!["floor".to_string()], both.clone(), both],
            "the gate is open for the first body only"
        );
    }

    /// A portal's hole is open only for the bodies that opened it (Jon
    /// 2026-10-16): the floor under a hole is cut from the walls the opener
    /// meets, and whole in the walls of every other body and of a reader that
    /// names no body. A body standing beside the opening keeps its floor.
    #[test]
    fn a_portal_hole_is_open_only_for_the_bodies_that_opened_it() {
        #[derive(Resource, Default, Debug)]
        struct Probe(Vec<bool>);
        #[derive(Resource)]
        struct Bodies([bevy_ecs::entity::Entity; 2]);
        // Is the floor solid under the hole, for each body and then for no body?
        fn probe(world: CollisionWorld, bodies: bevy_ecs::prelude::Res<Bodies>, mut out: ResMut<Probe>) {
            let mut composed = ComposedRooms::default();
            let under = ae::Aabb::new(ae::Vec2::new(200.0, 385.0), ae::Vec2::new(4.0, 4.0));
            let solid = |walls: &ae::World| walls.blocks.iter().any(|block| block.aabb.strict_intersects(under));
            let mut seen: Vec<bool> =
                bodies.0.iter().map(|body| solid(&composed.solids_for(&world, None, *body).expect("one live room"))).collect();
            seen.push(solid(composed.solids(&world, None).expect("one live room")));
            let room = world.room(None).expect("one live room");
            seen.push(solid(&room.solids_for(bodies.0[0]).expect("one live room")));
            out.0 = seen;
        }
        let mut app = App::new();
        app.init_resource::<Probe>();
        let floor = ae::Block::solid("floor", ae::Vec2::new(0.0, 380.0), ae::Vec2::new(400.0, 20.0));
        a_live_room(&mut app, LiveRoomInstance::ACTIVATION, vec![floor]);
        let bodies = [app.world_mut().spawn_empty().id(), app.world_mut().spawn_empty().id()];
        app.insert_resource(Bodies(bodies));
        app.add_systems(Update, probe);
        app.update();
        assert_eq!(app.world().resource::<Probe>().0, vec![true; 4], "no hole: the floor is whole for all");

        let mut overlay = app.world_mut().query::<&mut FeatureEcsWorldOverlay>();
        overlay.single_mut(app.world_mut()).expect("one overlay").portal_holes.push(
            ambition_platformer2d_shared_tangle::feature_overlay::PortalHolePass {
                face: ae::frame::FaceRect {
                    center: ae::Vec2::new(200.0, 400.0),
                    normal: ae::Vec2::new(0.0, -1.0),
                    half_along: 30.0,
                    half_out: 38.0,
                },
                bodies: vec![bodies[0]],
            },
        );
        app.update();
        assert_eq!(
            app.world().resource::<Probe>().0,
            vec![false, true, true, false],
            "the hole is open for the first body only"
        );
    }

    /// A body in a pair meets the far side with its part past the plane
    /// (`docs/planning/engine/portals-a-body-in-two-places.md`): behind a
    /// floor portal, inside the opening, is what is in front of its partner,
    /// a wall portal, and not the floor's own inside.
    #[test]
    fn a_body_in_a_pair_meets_what_is_in_front_of_its_far_end() {
        use ae::frame::{MapConvention, PortalAperture, PortalFrame};
        let floor = ae::Block::solid("floor", ae::Vec2::new(0.0, 300.0), ae::Vec2::new(400.0, 40.0));
        // A ledge 20 in front of the wall portal, inside its opening.
        let ledge = ae::Block::solid("ledge", ae::Vec2::new(460.0, 190.0), ae::Vec2::new(20.0, 40.0));
        let walls = ae::World::new("glue", ae::Vec2::new(800.0, 600.0), ae::Vec2::ZERO, vec![floor, ledge]);
        let glue = PortalGlue {
            enter: PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(100.0, 300.0), ae::Vec2::new(0.0, -1.0)), half_length: 30.0 },
            exit: PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(500.0, 200.0), ae::Vec2::new(-1.0, 0.0)), half_length: 30.0 },
            convention: MapConvention::Reflection,
            depth: 60.0,
        };
        let solid_at = |world: &ae::World, x: f32, y: f32| {
            let probe = ae::Aabb::new(ae::Vec2::new(x, y), ae::Vec2::splat(1.0));
            world.blocks.iter().any(|block| block.aabb.strict_intersects(probe))
        };
        let met = as_met_by(&walls, &[], &[], &[&glue]);
        // The ledge is under the floor portal, where it is from the far end:
        // 20 into the opening, 10 to 30 along it.
        assert!(solid_at(&met, 90.0, 330.0), "the far side's ledge is not met");
        // The floor's own inside behind the opening is not.
        assert!(!solid_at(&met, 120.0, 310.0), "the near side's floor is still met behind the opening");
        // Beside the opening the floor is the floor.
        assert!(solid_at(&met, 40.0, 310.0));
        // For a body in no pair, nothing changed.
        assert!(solid_at(&walls, 120.0, 310.0) && !solid_at(&walls, 90.0, 330.0) || solid_at(&walls, 90.0, 330.0));
    }

    /// A pair that turns what it carries by 45°: a square in front of the
    /// exit is a diamond behind the entry. The world's solids are boxes, so
    /// it is the box that bounds the diamond: solid wherever the diamond is,
    /// never a sliver. Mapping two opposite corners and boxing them (the
    /// first glue, review of 15d0cf99c) gave a box of no width: the two
    /// corners of a square turned by 45° share an x. And a block inside the
    /// box that bounds the turned region, but outside the region, is not
    /// carried.
    #[test]
    fn a_square_through_a_pair_that_turns_it_by_45_degrees_is_solid_where_it_is() {
        use ae::frame::{map_point, MapConvention, PortalAperture, PortalFrame};
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let glue = PortalGlue {
            enter: PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(100.0, 300.0), ae::Vec2::new(0.0, -1.0)), half_length: 60.0 },
            exit: PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(500.0, 200.0), ae::Vec2::new(s, -s)), half_length: 60.0 },
            convention: MapConvention::Reflection,
            depth: 60.0,
        };
        // In front of the exit, well inside its region.
        let inside = ae::Vec2::new(500.0, 200.0) + ae::Vec2::new(s, -s) * 30.0;
        let square = ae::Block::solid("square", inside - ae::Vec2::splat(8.0), ae::Vec2::splat(16.0));
        // In the box that bounds the turned region, out of the region.
        let corner = ae::Block::solid("corner", ae::Vec2::new(578.0, 228.0), ae::Vec2::splat(4.0));
        let walls = ae::World::new("glue", ae::Vec2::new(800.0, 600.0), ae::Vec2::ZERO, vec![square.clone(), corner]);
        let met = as_met_by(&walls, &[], &[], &[&glue]);
        let carried: Vec<&ae::Block> = met.blocks.iter().filter(|b| b.name == "square").collect();
        let square_at = carried.iter().find(|b| b.aabb.max.y > 300.0).expect("the square is carried behind the entry");
        let size = square_at.aabb.max - square_at.aabb.min;
        assert!(size.x > 16.0 && size.y > 16.0, "the carried square has the area of a turned square: {size:?}");
        let back = |p: ae::Vec2| map_point(&glue.exit.frame, &glue.enter.frame, glue.convention, p);
        for c in [square.aabb.min, square.aabb.max, ae::Vec2::new(square.aabb.min.x, square.aabb.max.y), ae::Vec2::new(square.aabb.max.x, square.aabb.min.y)] {
            let p = back(c);
            assert!(
                p.x >= square_at.aabb.min.x - 1e-3 && p.x <= square_at.aabb.max.x + 1e-3 && p.y >= square_at.aabb.min.y - 1e-3 && p.y <= square_at.aabb.max.y + 1e-3,
                "the carried box holds the turned square's corner {p:?}: {:?}",
                square_at.aabb
            );
        }
        assert_eq!(met.blocks.iter().filter(|b| b.name == "corner").count(), 1, "a block out of the turned region is not carried");
    }

    /// What a glued world costs to make (review of 15d0cf99c, finding D):
    /// `cargo test --release -p ambition_platformer2d_world glue_cost -- --ignored --nocapture`.
    /// A measurement, not a check.
    #[test]
    #[ignore]
    fn glue_cost() {
        use ae::frame::{MapConvention, PortalAperture, PortalFrame};
        for n in [100usize, 1000, 5000] {
            let blocks: Vec<ae::Block> = (0..n)
                .map(|i| ae::Block::solid(format!("b{i}"), ae::Vec2::new((i % 100) as f32 * 32.0, (i / 100) as f32 * 32.0), ae::Vec2::splat(16.0)))
                .collect();
            let walls = ae::World::new("cost", ae::Vec2::new(3200.0, 3200.0), ae::Vec2::ZERO, blocks);
            let glue = PortalGlue {
                enter: PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(400.0, 400.0), ae::Vec2::new(0.0, -1.0)), half_length: 48.0 },
                exit: PortalAperture { frame: PortalFrame::fixed(ae::Vec2::new(1200.0, 800.0), ae::Vec2::new(1.0, 0.0)), half_length: 48.0 },
                convention: MapConvention::Reflection,
                depth: 60.0,
            };
            let reps = 200;
            let start = std::time::Instant::now();
            let mut kept = 0;
            for _ in 0..reps {
                kept += as_met_by(&walls, &[], &[], &[&glue]).blocks.len();
            }
            let glued = start.elapsed() / reps;
            let start = std::time::Instant::now();
            for _ in 0..reps {
                kept += as_met_by(&walls, &[], &[], &[]).blocks.len();
            }
            let plain = start.elapsed() / reps;
            eprintln!("blocks={n}: glued {glued:?} per body per tick, unglued {plain:?} ({kept})");
        }
    }

    /// OW1 cut 3d: a reader in live room #1 collides with #1's walls, and
    /// a reader in #0 with #0's, when both are live at once. #1's overlay
    /// holds a lock wall, so the composed (not the borrowed) road is keyed
    /// too.
    ///
    /// The control is one live room: a reader that names no room gets it.
    /// With two, that reader gets nothing, and not one of them.
    #[test]
    fn a_reader_collides_with_the_live_room_it_is_in_and_no_other() {
        let mut app = App::new();
        app.init_resource::<KeyedProbe>();
        app.add_systems(Update, probe_keyed);
        let floor = |name: &str| ae::Block::solid(name, ae::Vec2::new(0.0, 380.0), ae::Vec2::new(400.0, 20.0));
        a_live_room(&mut app, LiveRoomInstance::ACTIVATION, vec![floor("floor #0")]);
        app.update();
        let names = |names: &[&str]| Some(names.iter().map(|name| name.to_string()).collect::<Vec<_>>());
        assert_eq!(
            app.world().resource::<KeyedProbe>().0,
            vec![names(&["floor #0"]), names(&["floor #0"]), None],
            "one live room: the unkeyed reader gets it, and #1 is not live"
        );

        a_live_room(&mut app, LiveRoomInstance::ACTIVATION.next(), vec![floor("floor #1")]);
        let second = app
            .world_mut()
            .query::<(&LiveRoomInstance, &mut FeatureEcsWorldOverlay)>()
            .iter_mut(app.world_mut())
            .find(|(live, _)| **live == LiveRoomInstance::ACTIVATION.next())
            .map(|(_, overlay)| overlay);
        second.expect("#1 has an overlay").gate_solids.push(gate_wall());
        app.update();
        assert_eq!(
            app.world().resource::<KeyedProbe>().0,
            vec![None, names(&["floor #0"]), names(&["floor #1", "lockwall:test_encounter"])],
            "two live rooms: each reader collides with its own room's walls"
        );
    }
}

#[cfg(test)]
mod contributed_block_order_tests {
    use super::*;

    /// Two contributed walls on the same spot, from two occurrences. A reader
    /// that takes the first block that matches gets one or the other.
    fn walls() -> [ae::Block; 2] {
        let wall = |occurrence: &str| ae::Block {
            id: ae::GeoId::placement(ae::PlacementId::new(occurrence), 0),
            name: format!("wall {occurrence}"),
            ..ae::Block::solid("", ae::Vec2::new(100.0, 0.0), ae::Vec2::new(16.0, 64.0))
        };
        [wall("crate_b"), wall("crate_a")]
    }

    fn names(world: &ae::World) -> Vec<String> {
        world.blocks.iter().map(|block| block.name.clone()).collect()
    }

    /// The contributors are unordered among themselves and each iterates a
    /// query in storage order, so the order a block list arrives in is not a
    /// fact about the world. Each composition states one order for it.
    #[test]
    fn a_composed_world_does_not_depend_on_the_order_its_blocks_were_contributed() {
        let room = ae::World::new(
            "test",
            ae::Vec2::new(400.0, 400.0),
            ae::Vec2::new(50.0, 50.0),
            vec![ae::Block::solid("floor", ae::Vec2::new(0.0, 380.0), ae::Vec2::new(400.0, 20.0))],
        );
        let [b, a] = walls();
        let forward = [b.clone(), a.clone()];
        let backward = [a, b];

        let sandbox = |blocks: &[ae::Block], gates: &[ae::Block]| {
            let overlay = FeatureEcsWorldOverlay {
                blocks: blocks.to_vec(),
                gate_solids: gates.to_vec(),
                ..Default::default()
            };
            names(&world_with_sandbox_solids(&room, &[], &overlay))
        };
        assert_eq!(sandbox(&forward, &[]), sandbox(&backward, &[]), "contributed blocks");
        assert_eq!(sandbox(&[], &forward), sandbox(&[], &backward), "gate solids");

        let projectile = |gates: &[ae::Block], contributed: &[ae::Block]| {
            names(&world_with_contributed_solids_and_carves(&room, gates, contributed, &[], &[]))
        };
        assert_eq!(projectile(&forward, &[]), projectile(&backward, &[]), "gate solids");
        assert_eq!(projectile(&[], &forward), projectile(&[], &backward), "contributed blocks");

        // The authored room keeps its own order, ahead of what is contributed.
        assert_eq!(
            sandbox(&forward, &[]),
            ["floor", "wall crate_a", "wall crate_b"],
        );
    }
}
