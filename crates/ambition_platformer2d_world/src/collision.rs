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
//! `world_with_sandbox_solids` adds moving-platform + ECS-overlay solids and
//! carves portal apertures; `world_with_portal_carves` carves only the apertures
//! (borrowing when none are active, for the projectile path);
//! `world_with_gate_solids_and_carves` is the projectile view.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::geometry::subtract_aabb;
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
use bevy_ecs::prelude::{Component, Query, With};
use bevy_ecs::system::SystemParam;
use std::borrow::Cow;

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
            Option<&'static LiveRoomInstance>,
            &'static ae::RoomGeometry,
            Option<&'static MovingPlatformSet>,
            Option<&'static FeatureEcsWorldOverlay>,
        ),
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>,
    >,
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
        room_of(self.rooms.iter(), room)
    }

    /// [`RoomCollision::solids`] of the sole live room.
    pub fn solids(&self) -> Option<Cow<'_, ae::World>> {
        self.room(None)?.solids()
    }

    /// [`RoomCollision::carves_only`] of the sole live room.
    pub fn carves_only(&self) -> Option<Cow<'_, ae::World>> {
        self.room(None)?.carves_only()
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
    let state = world.try_query_filtered::<RoomParts, With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>>()?;
    let rooms = state.query_manual(world);
    room_of(rooms.iter(), room).map(read)
}

type RoomParts = (
    Option<&'static LiveRoomInstance>,
    &'static ae::RoomGeometry,
    Option<&'static MovingPlatformSet>,
    Option<&'static FeatureEcsWorldOverlay>,
);

fn room_of<'a>(
    rooms: impl Iterator<
        Item = (
            Option<&'a LiveRoomInstance>,
            &'a ae::RoomGeometry,
            Option<&'a MovingPlatformSet>,
            Option<&'a FeatureEcsWorldOverlay>,
        ),
    >,
    room: Option<&InRoomInstance>,
) -> Option<RoomCollision<'a>> {
    let mut matching = rooms.filter(|(live, ..)| match room {
        Some(room) => live.copied() == Some(room.0),
        None => true,
    });
    let (_, geometry, platforms, overlay) = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    Some(RoomCollision {
        geometry,
        platforms: platforms.map_or(&[][..], |platforms| &platforms.0),
        overlay,
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
    /// are open for it ([`GatePass`], Q54). With no gate open for the subject,
    /// which is every tick of a room with no per-actor gate, these are the
    /// same composed walls, borrowed.
    ///
    /// [`GatePass`]: ambition_platformer2d_shared_tangle::feature_overlay::GatePass
    pub fn solids_for(
        &mut self,
        collision: &'a CollisionWorld<'_, '_>,
        room: Option<&InRoomInstance>,
        subject: bevy_ecs::entity::Entity,
    ) -> Option<Cow<'_, ae::World>> {
        let open = collision
            .room(room)
            .map(|room| room.gates_open_for(subject))
            .unwrap_or_default();
        let walls = self.solids(collision, room)?;
        Some(without_gates(walls, &open))
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
                && o.gate_solids.is_empty()
                && o.portal_carves.is_empty()
                && o.removed_block_names.is_empty()
                && o.climbable_carves.is_empty()
                && o.water_regions.is_empty()
        });
        if self.platforms.is_empty() && overlay_empty {
            return Some(Cow::Borrowed(&self.geometry.0));
        }
        let default_overlay;
        let overlay = match self.overlay {
            Some(o) => o,
            None => {
                default_overlay = FeatureEcsWorldOverlay::default();
                &default_overlay
            }
        };
        Some(Cow::Owned(world_with_sandbox_solids(
            &self.geometry.0,
            self.platforms,
            overlay,
        )))
    }

    /// The room with ONLY portal apertures carved — moving platforms and ECS
    /// solids omitted. Projectiles pass through moving platforms, so they read
    /// this. Borrows when no carves are active (the common case).
    pub fn carves_only(self) -> Option<Cow<'a, ae::World>> {
        let carves = self.overlay.map_or(&[][..], |o| &o.portal_carves[..]);
        Some(world_with_portal_carves(&self.geometry.0, carves))
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
    /// reads only — never for collision. Prefer `solids()` / `carves_only()`.
    pub fn base(self) -> &'a ae::World {
        &self.geometry.0
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
    // Gate solids (lock walls) are authored-equivalent statics: added alongside
    // the base/platform solids BEFORE the carve so a portal aperture splits them
    // exactly as it would a base wall.
    extend_in_canonical_order(&mut collision_world.blocks, &ecs_overlay.gate_solids);
    // Additive liquid (falling-sand settled pools) folds in alongside the base
    // water regions — keeps the authored base immutable while the projection is a
    // per-frame overlay contribution like the solids above.
    collision_world
        .water_regions
        .extend(ecs_overlay.water_regions.iter().cloned());
    // Carve portal apertures out of the host surface so a body can sink into a
    // portal (the "feet in, feet out" transit). Only the solid host kinds are
    // carved; the portal rim and surrounding geometry stay solid.
    if !ecs_overlay.portal_carves.is_empty() {
        carve_portal_apertures(&mut collision_world.blocks, &ecs_overlay.portal_carves);
    }
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
