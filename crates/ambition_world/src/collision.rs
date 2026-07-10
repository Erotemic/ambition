//! The composited collision world: the authored room folded together with the
//! per-frame dynamic contributions a running sim adds to it.
//!
//! [`CollisionWorld`] is the single collision read-API every actor sweep/raycast
//! should reach for instead of `Res<RoomGeometry>`: it composites the authored
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

use ambition_engine_core as ae;
use ambition_engine_core::geometry::subtract_aabb;
use ambition_engine_core::AabbExt;
use ambition_platformer_primitives::feature_overlay::FeatureEcsWorldOverlay;
use bevy_ecs::prelude::{Res, Resource};
use bevy_ecs::system::SystemParam;
use std::borrow::Cow;

use crate::platforms::{world_with_moving_platforms, MovingPlatformState};

/// The active room's live moving platforms.
///
/// Owned by the physics/rendering pipeline; the player tick advances each
/// platform per frame and carries the player by its delta. The physics plugin
/// registers this as a resource; the room-load path (setup, load_room, LDtk
/// hot-reload, sandbox reset) replaces the Vec when the active room changes.
///
/// Lives beside [`MovingPlatformState`] rather than a tier up: it is a newtype
/// over this crate's own vocabulary, and [`CollisionWorld`] reads it.
#[derive(Resource, Default)]
pub struct MovingPlatformSet(pub Vec<MovingPlatformState>);

/// The single collision read-API. Composites the authored
/// [`ambition_engine_core::RoomGeometry`] with the per-frame dynamic overlay —
/// moving platforms, ECS-owned solids, and portal carves — into the collision
/// world a sweep or raycast should see.
///
/// Every resource is optional so headless / minimal-app tests that insert only a
/// room (or nothing) still satisfy the param. The composite degrades to the bare
/// authored geometry *exactly* when there are no dynamics — which is precisely
/// when bare and composite are identical — so routing a former `Res<RoomGeometry>`
/// reader through here changes behaviour only in production rooms that actually
/// carry moving platforms / ECS solids / portal carves.
#[derive(SystemParam)]
pub struct CollisionWorld<'w> {
    room: Option<Res<'w, ae::RoomGeometry>>,
    platforms: Option<Res<'w, MovingPlatformSet>>,
    overlay: Option<Res<'w, FeatureEcsWorldOverlay>>,
}

impl CollisionWorld<'_> {
    /// The full collision world: authored room + moving platforms + ECS solids,
    /// with portal apertures carved. This is what actor sweeps and traversal
    /// raycasts (grapple / blink / dive / body-mode clearance / dropped items)
    /// read so they collide with everything solid this frame.
    ///
    /// Returns `None` when no room is loaded (minimal test apps), and borrows the
    /// base geometry on the no-dynamics fast path so the common case never clones.
    pub fn solids(&self) -> Option<Cow<'_, ae::World>> {
        let room = self.room.as_ref()?;
        let platforms = self.platforms.as_ref().map_or(&[][..], |p| &p.0);
        let overlay_empty = self.overlay.as_ref().map_or(true, |o| {
            o.blocks.is_empty()
                && o.gate_solids.is_empty()
                && o.portal_carves.is_empty()
                && o.removed_block_names.is_empty()
                && o.climbable_carves.is_empty()
                && o.water_regions.is_empty()
        });
        if platforms.is_empty() && overlay_empty {
            return Some(Cow::Borrowed(&room.0));
        }
        let default_overlay;
        let overlay = match self.overlay.as_ref() {
            Some(o) => &**o,
            None => {
                default_overlay = FeatureEcsWorldOverlay::default();
                &default_overlay
            }
        };
        Some(Cow::Owned(world_with_sandbox_solids(
            &room.0, platforms, overlay,
        )))
    }

    /// The room with ONLY portal apertures carved — moving platforms and ECS
    /// solids omitted. Projectiles pass through moving platforms, so they read
    /// this. Borrows when no carves are active (the common case).
    pub fn carves_only(&self) -> Option<Cow<'_, ae::World>> {
        let room = self.room.as_ref()?;
        let carves = self.overlay.as_ref().map_or(&[][..], |o| &o.portal_carves);
        Some(world_with_portal_carves(&room.0, carves))
    }

    /// The bare authored geometry, no overlay. For metadata / bounds / layout
    /// reads only — never for collision. Prefer `solids()` / `carves_only()`.
    pub fn base(&self) -> Option<&ae::World> {
        self.room.as_ref().map(|r| &r.0)
    }
}

pub fn world_with_sandbox_solids(
    world: &ae::World,
    platforms: &[MovingPlatformState],
    ecs_overlay: &FeatureEcsWorldOverlay,
) -> ae::World {
    let mut collision_world = world_with_moving_platforms(world, platforms);
    // Content gates may SUBTRACT authored geometry from the view (the immutable-
    // base inversion): drop named authored blocks and carve out suppressed
    // climbable regions before adding overlay solids / carving portals. A gate
    // contributes these instead of mutating the authored base mid-room.
    apply_overlay_subtractions(&mut collision_world, ecs_overlay);
    collision_world
        .blocks
        .extend(ecs_overlay.blocks.iter().cloned());
    // Gate solids (lock walls) are authored-equivalent statics: added alongside
    // the base/platform solids BEFORE the carve so a portal aperture splits them
    // exactly as it would a base wall.
    collision_world
        .blocks
        .extend(ecs_overlay.gate_solids.iter().cloned());
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

/// The room world with ONLY the portal apertures carved out — no moving-platform
/// or ECS-overlay solids added. Projectiles historically collided against the raw
/// room world (they pass through moving platforms); this preserves that exactly
/// while letting a shot sink into a portal opening and transit.
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
    if gate_solids.is_empty() && portal_carves.is_empty() && removed_block_names.is_empty() {
        return Cow::Borrowed(world);
    }
    let mut composed = world.clone();
    // Subtract authored blocks a gate has opened (the gnu_ton floor-gate) so a
    // shot passes through it exactly as the player does, then add gate solids +
    // carve.
    remove_named_blocks(&mut composed.blocks, removed_block_names);
    composed.blocks.extend(gate_solids.iter().cloned());
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
        let carvable = matches!(
            block.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. } | ae::BlockKind::OneWay
        );
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
            });
        }
    }
}

#[cfg(test)]
mod collision_world_tests {
    use super::*;
    use bevy_app::{App, Update};
    use bevy_ecs::prelude::ResMut;

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
        app.insert_resource(room_one_block());
        // No platforms, no overlay → borrow the base, identical block count.
        assert_eq!(run(&mut app), Some((false, 1)));
    }

    #[test]
    fn empty_overlay_still_borrows() {
        let mut app = App::new();
        app.init_resource::<SolidsProbe>();
        app.insert_resource(room_one_block());
        app.insert_resource(FeatureEcsWorldOverlay::default());
        // An empty overlay is still the no-dynamics fast path.
        assert_eq!(run(&mut app), Some((false, 1)));
    }

    #[test]
    fn overlay_solids_compose_owned() {
        let mut app = App::new();
        app.init_resource::<SolidsProbe>();
        app.insert_resource(room_one_block());
        app.insert_resource(FeatureEcsWorldOverlay {
            blocks: vec![ae::Block {
                id: ae::GeoId::anon(),
                name: "ecs-solid".into(),
                aabb: ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(10.0, 10.0)),
                kind: ae::BlockKind::Solid,
                velocity: ae::Vec2::ZERO,
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
        app.insert_resource(room_one_block());
        app.insert_resource(FeatureEcsWorldOverlay {
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
}
