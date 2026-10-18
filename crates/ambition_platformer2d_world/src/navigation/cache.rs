//! The surface graphs in use, kept by what each one is a function of.
//!
//! A [`NavGraph`] is a pure function of the geometry it is built over, the
//! exits it avoids, the body's tuning, abilities and size, and its motion
//! frame. [`NavGraphKey`] is that function's argument as a value, and
//! [`NavGraphCache`] keeps a bounded number of graphs by it, each built whole.
//! The simulation's navigation advisor keeps one cache, and the autopilot of a
//! player's seat keeps another: one keying rule, so two graphs for the same
//! argument are the same graph.

use ambition_platformer2d_core as ae;
use ae::movement::MotionModelSpec;
use ae::{AbilitySet, Aabb, BodyBaseSize, BodyClusterScratch, MotionFrame, Vec2, World};

use super::NavGraph;
use crate::collision::{GeometryRevision, RoomCollision};

/// What one graph is a function of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavGraphKey {
    /// The geometry and the exits, as a number ([`geometry_stamp`]).
    room: u64,
    spec: MotionModelSpec,
    abilities: AbilitySet,
    size: Vec2,
    base_size: BodyBaseSize,
    /// The whole frame, and not its net acceleration: the graph is built from
    /// which way is down, from the gravity a jump law can scale, and from the
    /// external acceleration it cannot. Two frames with one sum are two rooms
    /// to a body (found in review, 2026-10-09).
    frame: MotionFrame,
}

impl NavGraphKey {
    /// The key of the graph of `world`, avoiding `exits`, for a body of this
    /// tuning under `frame`.
    pub fn new(
        world: &World,
        exits: &[Aabb],
        spec: MotionModelSpec,
        abilities: AbilitySet,
        size: Vec2,
        base_size: BodyBaseSize,
        frame: MotionFrame,
    ) -> Self {
        Self::from_stamp(geometry_stamp(world, exits), spec, abilities, size, base_size, frame)
    }

    /// The key of a graph whose world and exits have the stamp `room`
    /// ([`geometry_stamp`]): for a caller that keys many bodies in one world
    /// and stamps the world once.
    pub fn from_stamp(
        room: u64,
        spec: MotionModelSpec,
        abilities: AbilitySet,
        size: Vec2,
        base_size: BodyBaseSize,
        frame: MotionFrame,
    ) -> Self {
        Self { room, spec, abilities, size, base_size, frame }
    }

    /// The probe body the graph is built for: the tuning with no history. A
    /// graph is not a fact about what a body was doing when it was asked for.
    pub fn probe_body(&self) -> BodyClusterScratch {
        let mut body = BodyClusterScratch::new_with_abilities(Vec2::ZERO, self.abilities);
        ae::movement::switch_motion_model(&mut body.model, self.spec);
        body.kinematics.size = self.size;
        body.base_size = self.base_size;
        body
    }
}

/// Graphs by [`NavGraphKey`], at most `capacity`; the oldest goes first. A
/// key with no graph (the body has none there) is kept too, so it is not
/// built again.
pub struct NavGraphCache {
    graphs: Vec<(NavGraphKey, Option<NavGraph>)>,
    capacity: usize,
}

impl NavGraphCache {
    pub fn with_capacity(capacity: usize) -> Self {
        Self { graphs: Vec::with_capacity(capacity), capacity: capacity.max(1) }
    }

    pub fn len(&self) -> usize {
        self.graphs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.graphs.is_empty()
    }

    /// The kept answer for `key`: `Some(None)` when it has no graph, `None`
    /// when it was never built.
    pub fn get(&self, key: &NavGraphKey) -> Option<Option<&NavGraph>> {
        self.graphs.iter().find(|(known, _)| known == key).map(|(_, graph)| graph.as_ref())
    }

    /// Keep `graph` as the answer for `key`.
    pub fn insert(&mut self, key: NavGraphKey, graph: Option<NavGraph>) {
        if let Some(slot) = self.graphs.iter_mut().find(|(known, _)| *known == key) {
            slot.1 = graph;
            return;
        }
        if self.graphs.len() >= self.capacity {
            self.graphs.remove(0);
        }
        self.graphs.push((key, graph));
    }

    /// The kept answer for `key`, or `build()` kept and returned.
    pub fn get_or_build(&mut self, key: NavGraphKey, build: impl FnOnce() -> Option<NavGraph>) -> Option<&NavGraph> {
        let index = match self.graphs.iter().position(|(known, _)| *known == key) {
            Some(index) => index,
            None => {
                self.insert(key, build());
                self.graphs.len() - 1
            }
        };
        self.graphs[index].1.as_ref()
    }

    /// Each kept key and its graph.
    pub fn iter(&self) -> impl Iterator<Item = (&NavGraphKey, Option<&NavGraph>)> {
        self.graphs.iter().map(|(key, graph)| (key, graph.as_ref()))
    }
}

/// One number for the world a graph is built from, and the exits it avoids.
///
/// A graph is built by stepping a probe body through the WHOLE world (its
/// rollouts call the movement kernel), so the number folds every field of the
/// world the kernel can read: each block whole, the water and climbable
/// regions, the surface chains, the edges, the size. It folds them through
/// serde, so a field added to the world later is in the number without an
/// edit here. Only `name` fields are left out: a label is not geometry. Two
/// worlds with one number have the same graph.
pub fn geometry_stamp(world: &World, exits: &[Aabb]) -> u64 {
    use serde::Serialize as _;
    let mut fold = fingerprint::Fold(0x6e61_765f_6772_6170);
    // The fold cannot fail: its error type has no value.
    let _ = world.serialize(&mut fold);
    fold.fold(exits.len() as u64);
    for exit in exits {
        fold.fold(((exit.min.x.to_bits() as u64) << 32) | exit.min.y.to_bits() as u64);
        fold.fold(((exit.max.x.to_bits() as u64) << 32) | exit.max.y.to_bits() as u64);
    }
    ae::navigation::mix(fold.0)
}

/// The stamps of live rooms' authored geometry ([`geometry_stamp`] with no
/// exits), kept across ticks by [`GeometryRevision`]: a room's stamp is folded
/// once for each write to its geometry, not once each tick. The fold reads the
/// whole world: 5 us for the median shipped room and 21 us for the largest
/// (`hall_of_characters`), in a debug build (measured 2026-10-18).
#[derive(Default)]
pub struct GeometryStamps {
    /// The stamps asked for before the last [`Self::retire_unused`].
    kept: Vec<(GeometryRevision, u64)>,
    /// The stamps asked for since.
    used: Vec<(GeometryRevision, u64)>,
    folds: u64,
}

impl GeometryStamps {
    /// The stamp of `room`'s authored geometry ([`RoomCollision::base`]).
    pub fn of(&mut self, room: RoomCollision<'_>) -> u64 {
        let revision = room.geometry_revision();
        if let Some((_, stamp)) = self.used.iter().find(|(known, _)| *known == revision) {
            return *stamp;
        }
        let stamp = match self.kept.iter().find(|(known, _)| *known == revision) {
            Some((_, stamp)) => *stamp,
            None => {
                self.folds += 1;
                geometry_stamp(room.base(), &[])
            }
        };
        self.used.push((revision, stamp));
        stamp
    }

    /// Forget each stamp not asked for since the last call: of a room that
    /// is gone, or of geometry written since.
    pub fn retire_unused(&mut self) {
        self.kept = std::mem::take(&mut self.used);
    }

    /// How many times a world was folded. For tests and instruments.
    pub fn folds(&self) -> u64 {
        self.folds
    }
}

/// A serde serializer that folds each value into one number, with no
/// allocation: floats by their bits, strings by their bytes, an enum by its
/// variant, a sequence by its length and an end mark. Each value is folded by
/// a multiply and a rotate (as FxHash does), and the result is mixed once at
/// the end: a world has a few values for each block, and the stamp is asked
/// for each tick.
mod fingerprint {
    use serde::ser::{self, Serialize};

    pub(super) struct Fold(pub u64);

    impl Fold {
        pub(super) fn fold(&mut self, value: u64) {
            self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x517c_c1b7_2722_0a95);
        }

        fn bytes(&mut self, bytes: &[u8]) {
            self.fold(bytes.len() as u64);
            for chunk in bytes.chunks(8) {
                let mut word = [0u8; 8];
                word[..chunk.len()].copy_from_slice(chunk);
                self.fold(u64::from_le_bytes(word));
            }
        }
    }

    /// The end of a sequence, a tuple or a struct, so `[a][b]` and `[a, b]`
    /// fold apart.
    const END: u64 = 0x656e_645f_6d61_726b;

    #[derive(Debug)]
    pub(super) enum Never {}

    impl std::fmt::Display for Never {
        fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match *self {}
        }
    }

    impl std::error::Error for Never {}

    impl ser::Error for Never {
        fn custom<T: std::fmt::Display>(_: T) -> Self {
            unreachable!("the fold never fails")
        }
    }

    type Done = Result<(), Never>;

    impl ser::Serializer for &mut Fold {
        type Ok = ();
        type Error = Never;
        type SerializeSeq = Self;
        type SerializeTuple = Self;
        type SerializeTupleStruct = Self;
        type SerializeTupleVariant = Self;
        type SerializeMap = Self;
        type SerializeStruct = Self;
        type SerializeStructVariant = Self;

        fn serialize_bool(self, v: bool) -> Done { self.fold(u64::from(v)); Ok(()) }
        fn serialize_i8(self, v: i8) -> Done { self.fold(v as u64); Ok(()) }
        fn serialize_i16(self, v: i16) -> Done { self.fold(v as u64); Ok(()) }
        fn serialize_i32(self, v: i32) -> Done { self.fold(v as u64); Ok(()) }
        fn serialize_i64(self, v: i64) -> Done { self.fold(v as u64); Ok(()) }
        fn serialize_u8(self, v: u8) -> Done { self.fold(u64::from(v)); Ok(()) }
        fn serialize_u16(self, v: u16) -> Done { self.fold(u64::from(v)); Ok(()) }
        fn serialize_u32(self, v: u32) -> Done { self.fold(u64::from(v)); Ok(()) }
        fn serialize_u64(self, v: u64) -> Done { self.fold(v); Ok(()) }
        fn serialize_f32(self, v: f32) -> Done { self.fold(u64::from(v.to_bits())); Ok(()) }
        fn serialize_f64(self, v: f64) -> Done { self.fold(v.to_bits()); Ok(()) }
        fn serialize_char(self, v: char) -> Done { self.fold(u64::from(v)); Ok(()) }
        fn serialize_str(self, v: &str) -> Done { self.bytes(v.as_bytes()); Ok(()) }
        fn serialize_bytes(self, v: &[u8]) -> Done { self.bytes(v); Ok(()) }
        fn serialize_none(self) -> Done { self.fold(0); Ok(()) }
        fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Done { self.fold(1); value.serialize(self) }
        fn serialize_unit(self) -> Done { self.fold(2); Ok(()) }
        fn serialize_unit_struct(self, _: &'static str) -> Done { self.fold(2); Ok(()) }
        fn serialize_unit_variant(self, _: &'static str, index: u32, _: &'static str) -> Done {
            self.fold(u64::from(index));
            Ok(())
        }
        fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _: &'static str, value: &T) -> Done {
            value.serialize(self)
        }
        fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _: &'static str, index: u32, _: &'static str, value: &T) -> Done {
            self.fold(u64::from(index));
            value.serialize(self)
        }
        fn serialize_seq(self, len: Option<usize>) -> Result<Self, Never> {
            self.fold(len.map_or(u64::MAX, |len| len as u64));
            Ok(self)
        }
        fn serialize_tuple(self, _: usize) -> Result<Self, Never> { Ok(self) }
        fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self, Never> { Ok(self) }
        fn serialize_tuple_variant(self, _: &'static str, index: u32, _: &'static str, _: usize) -> Result<Self, Never> {
            self.fold(u64::from(index));
            Ok(self)
        }
        fn serialize_map(self, len: Option<usize>) -> Result<Self, Never> {
            self.fold(len.map_or(u64::MAX, |len| len as u64));
            Ok(self)
        }
        fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, Never> { Ok(self) }
        fn serialize_struct_variant(self, _: &'static str, index: u32, _: &'static str, _: usize) -> Result<Self, Never> {
            self.fold(u64::from(index));
            Ok(self)
        }
    }

    impl ser::SerializeSeq for &mut Fold {
        type Ok = ();
        type Error = Never;
        fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Done { value.serialize(&mut **self) }
        fn end(self) -> Done { self.fold(END); Ok(()) }
    }

    impl ser::SerializeTuple for &mut Fold {
        type Ok = ();
        type Error = Never;
        fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Done { value.serialize(&mut **self) }
        fn end(self) -> Done { self.fold(END); Ok(()) }
    }

    impl ser::SerializeTupleStruct for &mut Fold {
        type Ok = ();
        type Error = Never;
        fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Done { value.serialize(&mut **self) }
        fn end(self) -> Done { self.fold(END); Ok(()) }
    }

    impl ser::SerializeTupleVariant for &mut Fold {
        type Ok = ();
        type Error = Never;
        fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Done { value.serialize(&mut **self) }
        fn end(self) -> Done { self.fold(END); Ok(()) }
    }

    impl ser::SerializeMap for &mut Fold {
        type Ok = ();
        type Error = Never;
        fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Done { key.serialize(&mut **self) }
        fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Done { value.serialize(&mut **self) }
        fn end(self) -> Done { self.fold(END); Ok(()) }
    }

    impl ser::SerializeStruct for &mut Fold {
        type Ok = ();
        type Error = Never;
        fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Done {
            // A label is not geometry.
            if key == "name" {
                return Ok(());
            }
            value.serialize(&mut **self)
        }
        fn end(self) -> Done { self.fold(END); Ok(()) }
    }

    impl ser::SerializeStructVariant for &mut Fold {
        type Ok = ();
        type Error = Never;
        fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Done {
            if key == "name" {
                return Ok(());
            }
            value.serialize(&mut **self)
        }
        fn end(self) -> Done { self.fold(END); Ok(()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(blocks: Vec<ae::Block>) -> World {
        World::new("room", Vec2::new(800.0, 600.0), Vec2::ZERO, blocks)
    }

    fn floor() -> ae::Block {
        ae::Block::solid("floor", Vec2::new(0.0, 500.0), Vec2::new(800.0, 100.0))
    }

    /// A graph's rollouts step a body through the whole world, so each part of
    /// the world the movement kernel reads moves the stamp: water, a climbable
    /// region, a surface chain, a block's kind and its velocity, the edges.
    /// The control: a new label on a block is the same geometry, and the same
    /// stamp. (Found in review, 2026-10-17: the stamp read blocks, a few kind
    /// predicates and the exits, so a dry room's graph answered for a wet one.)
    #[test]
    fn each_part_of_the_world_the_kernel_reads_moves_the_stamp() {
        let dry = room(vec![floor()]);
        let stamp = |world: &World| geometry_stamp(world, &[]);
        let base = stamp(&dry);
        let region = ae::Aabb::new(Vec2::new(300.0, 300.0), Vec2::new(200.0, 200.0));

        let mut renamed = floor();
        renamed.name = "ground".into();
        assert_eq!(stamp(&room(vec![renamed])), base, "control: a label is not geometry");

        let mut wet = dry.clone();
        wet.water_regions.push(ae::WaterRegion::new(region, ae::WaterKind::Clear, Default::default()));
        let mut ladder = dry.clone();
        ladder.climbable_regions.push(ae::ClimbableRegion::new(region, ae::ClimbableKind::Ladder, Default::default()));
        let mut sloped = dry.clone();
        sloped.chains = vec![ae::SurfaceChain::open("slope", vec![Vec2::new(0.0, 500.0), Vec2::new(400.0, 300.0)])].into();
        let mut blink = floor();
        blink.kind = ae::BlockKind::BlinkWall { tier: ae::BlinkWallTier::Soft };
        let mut moving = floor();
        moving.velocity = Vec2::new(1.0, 0.0);
        let mut moving_faster = floor();
        moving_faster.velocity = Vec2::new(2.0, 0.0);
        let mut deeper = dry.clone();
        deeper.edges.fall += 100.0;
        for (what, world) in [
            ("water", wet),
            ("a ladder", ladder),
            ("a surface chain", sloped),
            ("a block's kind", room(vec![blink])),
            ("a block's velocity", room(vec![moving.clone()])),
            ("the edges", deeper),
        ] {
            assert_ne!(stamp(&world), base, "{what} did not move the stamp");
        }
        assert_ne!(
            stamp(&room(vec![moving])),
            stamp(&room(vec![moving_faster])),
            "two speeds of one moving block have one stamp"
        );
    }

    #[derive(bevy_ecs::resource::Resource, Default)]
    struct Stamped(GeometryStamps, Vec<u64>);

    fn stamp_the_room(collision: crate::collision::CollisionWorld, mut out: bevy_ecs::system::ResMut<Stamped>) {
        let out = &mut *out;
        let stamp = out.0.of(collision.room(None).expect("one live room"));
        // A second reader in one run reads the kept stamp.
        assert_eq!(out.0.of(collision.room(None).expect("one live room")), stamp);
        out.0.retire_unused();
        out.1.push(stamp);
    }

    /// A room's stamp is folded once for each write to its geometry, and not
    /// each tick; a write that keeps the count of blocks is a write.
    #[test]
    fn a_room_is_stamped_again_only_when_its_geometry_is_written() {
        use bevy_ecs::world::World as EcsWorld;
        let mut world = EcsWorld::new();
        world.init_resource::<Stamped>();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(&mut world, ae::RoomGeometry(room(vec![floor()])));
        let run = world.register_system(stamp_the_room);
        world.run_system(run).unwrap();
        world.run_system(run).unwrap();
        let stamps = |world: &EcsWorld| (world.resource::<Stamped>().0.folds(), world.resource::<Stamped>().1.clone());
        let (folds, seen) = stamps(&world);
        assert_eq!(folds, 1, "an unchanged room was folded again");
        assert_eq!(seen[0], geometry_stamp(&room(vec![floor()]), &[]), "the kept stamp is the geometry's");

        let mut lower = floor();
        lower.aabb = ae::Aabb::new(Vec2::new(400.0, 560.0), Vec2::new(400.0, 50.0));
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(&mut world, ae::RoomGeometry(room(vec![lower.clone()])));
        world.run_system(run).unwrap();
        let (folds, seen) = stamps(&world);
        assert_eq!(folds, 2, "a write to the geometry, with the count of blocks kept, was not seen");
        assert_eq!(seen[2], geometry_stamp(&room(vec![lower]), &[]), "the stamp of the geometry before the write");
    }
}
