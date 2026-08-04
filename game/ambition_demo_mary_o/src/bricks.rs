//! Mary-O's breakable bricks — the SECOND consumer of the reactive-block primitive.
//!
//! The ?-block powerup ([`crate::powerups`]) was the FIRST consumer of
//! `ContactSource::Block { id: GeoId }`: a head-bonk answers "*which* authored block
//! did I strike" by the block's durable [`GeoId`](ae::GeoId), with no point-matching.
//! A brick reuses that exact seam for the OPPOSITE effect — where the ?-block ADDS a
//! wand pickup, a bonked brick SUBTRACTS itself from the world. Two consumers, one
//! primitive, add vs subtract: the engine-for-other-games oracle wants a second user
//! to prove the primitive generalizes past the powerup, and this is it, with **zero
//! engine edits** beyond the render reconcile the subtraction always wanted.
//!
//! The removal is a mid-run **World-mutation** done the elegant way: the authored
//! [`RoomGeometry`](ae::RoomGeometry) stays immutable (it is swapped at room
//! boundaries, never edited mid-room), and a broken brick's name is contributed to
//! the collision overlay's per-frame `removed_block_names` — the SAME immutable-base
//! subtraction seam encounter gates use to drop authored blocks. Collision and
//! render both honour that list, so a broken brick stops colliding AND stops drawing.

use bevy::prelude::*;

use ambition_platformer2d::actors::actor::PrimaryPlayer;
use ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput;
use ambition_platformer2d::actors::features::FeatureEcsWorldOverlay;
use ambition_platformer2d::actors::rooms::RoomLoaded;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::collision_semantics::{ContactKind, ContactSource};

use ambition_platformer2d::platformer::lifecycle::SessionWorldRef;

use crate::ldtk_vocabulary::{block_kind_of, MaryOBlockKind};
use crate::LEVEL_1_1_ROOM_ID;

/// Which bricks are broken this run, **by their authored NAME**.
///
/// ⛔ **this was a `u32` bitset over brick INDICES.** An index is a position in a
/// Rust column array, and the level is authored now — there is no array, the
/// author can add a tenth brick, and inserting one would have renumbered every
/// brick after it *including the ones already recorded broken*. The name is what
/// the collision overlay subtracts anyway (`removed_block_names`), so storing
/// anything else meant converting on the way out.
///
/// ⚠ **`BTreeSet`, not `HashSet`, and that is the determinism contract.**
/// [`contribute_broken_bricks_to_overlay`] ITERATES this every frame, and
/// std-hash iteration order is seeded per process — two peers would subtract the
/// same blocks in different orders. A `BTreeSet` iterates in name order,
/// everywhere, always.
///
/// ⚠ **`Clone` because it is ROLLBACK STATE.** Which bricks are broken decides
/// what the room is made of — the overlay subtracts them from collision every
/// frame — so a rewind across a bonk that does not restore this leaves a wall
/// with a hole in it, or a hole with a wall in it.
#[derive(Resource, Default, Clone)]
pub struct BrokenBricks(std::collections::BTreeSet<String>);

impl BrokenBricks {
    /// Mark this brick broken; `true` only on the FRESH break, so the caller
    /// shatters it exactly once rather than every frame the contact re-reports.
    fn mark(&mut self, name: &str) -> bool {
        self.0.insert(name.to_string())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn is_broken(&self, name: &str) -> bool {
        self.0.contains(name)
    }

    fn clear(&mut self) {
        self.0.clear();
    }

    /// The broken names, in a stable order.
    fn broken_names(&self) -> impl Iterator<Item = &String> + '_ {
        self.0.iter()
    }
}

/// **The brick-break.** A head contact (`ContactKind::Head`) against a brick —
/// identified by the durable `GeoId` the engine carries on `ContactSource::Block`,
/// NOT by point-matching — marks that brick broken and shatters it once. The SAME
/// contact seam [`crate::powerups::bonk_power_blocks`] reads; a bonk resolves to a
/// ?-block OR a brick (their `GeoId` base indices are disjoint), never both.
pub fn break_bricks(
    mut broken: ResMut<BrokenBricks>,
    mut vfx: MessageWriter<ambition_platformer2d::vfx::VfxMessage>,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    players: Query<&PlayerBodyFrameOutput, With<PrimaryPlayer>>,
    // A `GeoId` names a block; only the room can say which one.
    geometry: SessionWorldRef<ae::RoomGeometry>,
) {
    let Ok(frame) = players.single() else {
        return;
    };
    for contact in &frame.events.contacts {
        if contact.kind != ContactKind::Head {
            continue;
        }
        let ContactSource::Block { id, .. } = &contact.source else {
            continue;
        };
        // ⭐ **ask the ROOM, then ask the BLOCK what it is.** This looked the id
        // up in a table of ids reconstructed from a column array, so an authored
        // brick was simply not in it.
        let Some(block) = crate::authored_block_by_id(&geometry.0, id) else {
            continue;
        };
        if block_kind_of(&block.name) != Some(MaryOBlockKind::Brick) {
            continue;
        }
        let center = (block.aabb.min + block.aabb.max) * 0.5;
        if broken.mark(&block.name) {
            // A fresh break shatters into brick-red shards through the engine's
            // shared particle seam — the same `VfxMessage::Burst` the snake squash
            // pops, so a brick reads as breaking with no bespoke vfx.
            vfx.write(ambition_platformer2d::vfx::VfxMessage::Burst {
                pos: center,
                count: 14,
                speed: 155.0,
                color: [0.72, 0.35, 0.22, 1.0],
                kind: ambition_platformer2d::vfx::ParticleKind::Shard,
            });
            // ...and cracks, through the engine's shared cue seam. PLACEHOLDER
            // TIMBRE: this reuses the existing `Hit` cue rather than inventing a
            // brick-specific one, and the provider synthesizes it as a short noisy
            // thunk. It reads as a smash and needs no asset; a bespoke crumble can
            // replace the authored spec later without touching this call site,
            // because what is emitted here is the SEMANTIC cue, not a sound.
            // H2/I3: the COURSE's. A brick is authored room geometry, not a body,
            // so it has no character to sound like — but it is Mary-O's geometry,
            // and `write_global` would have credited whoever is hosting her. The
            // SMASHER's own cue (the head-bonk, the swing) is emitted by the
            // smasher.
            sfx.write_from(
                crate::provider::MARY_O_EXPERIENCE,
                ambition_platformer2d::sfx::SfxMessage::Hit { pos: center },
            );
        }
    }
}

/// Re-arm every brick when level 1-1 (re)loads, so a cyclic replay rebuilds the
/// wall. Mirrors [`crate::powerups::refill_power_blocks_on_room_loaded`].
pub fn refill_bricks_on_room_loaded(
    mut rooms: MessageReader<RoomLoaded>,
    mut broken: ResMut<BrokenBricks>,
) {
    for message in rooms.read() {
        if message.room_id == LEVEL_1_1_ROOM_ID {
            broken.clear();
        }
    }
}

/// Contribute each broken brick's authored NAME to the collision overlay's per-frame
/// `removed_block_names` — the engine's immutable-base SUBTRACTION seam (the same one
/// encounter gates use to DROP authored blocks without editing the base). This is
/// what actually removes a broken brick from every collision read AND, via the
/// render reconcile, from the drawn world. Runs AFTER
/// [`rebuild_feature_ecs_world_overlay`](ambition_platformer2d::actors::features::rebuild_feature_ecs_world_overlay)
/// clears the list (its clean-slate-per-frame contract), exactly as
/// `contribute_encounter_lock_walls` does for `gate_solids`.
pub fn contribute_broken_bricks_to_overlay(
    broken: Res<BrokenBricks>,
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
) {
    overlay
        .removed_block_names
        .extend(broken.broken_names().cloned());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two bricks the LEVEL authors — the population `break_bricks` serves.
    fn two_authored_bricks() -> (String, String) {
        let room = crate::level_1_1();
        let mut names: Vec<String> = room
            .world
            .blocks
            .iter()
            .filter(|b| block_kind_of(&b.name) == Some(MaryOBlockKind::Brick))
            .map(|b| b.name.clone())
            .collect();
        names.sort();
        assert!(names.len() >= 2, "the level authors a brick wall: {names:?}");
        (names[0].clone(), names[1].clone())
    }

    fn authored_brick_id(name: &str) -> ae::GeoId {
        crate::level_1_1()
            .world
            .blocks
            .iter()
            .find(|b| b.name == name)
            .expect("that brick is authored")
            .id
            .clone()
    }

    fn head_bonk_frame(id: ae::GeoId) -> PlayerBodyFrameOutput {
        let mut frame = PlayerBodyFrameOutput::default();
        frame
            .events
            .contacts
            .push(ae::collision_semantics::Contact {
                kind: ContactKind::Head,
                point: ae::Vec2::ZERO,
                normal: ae::Vec2::new(0.0, 1.0),
                toi: 0.0,
                surface_velocity: ae::Vec2::ZERO,
                source: ContactSource::Block {
                    kind: ae::BlockKind::Solid,
                    id,
                },
            });
        frame
    }

    fn break_app() -> App {
        let mut app = App::new();
        app.init_resource::<BrokenBricks>();
        app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        // ⚠ **the REAL level, because `break_bricks` asks the room what it hit.**
        // A fixture with no room answers nothing, which is a green test about an
        // empty world rather than a test about bricks.
        ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
            app.world_mut(),
            ae::RoomGeometry(crate::level_1_1().world.clone()),
        );
        app.add_systems(Update, break_bricks);
        app
    }

    fn drain_bursts(app: &mut App) -> usize {
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ambition_platformer2d::vfx::VfxMessage>>()
            .drain()
            .filter(|m| matches!(m, ambition_platformer2d::vfx::VfxMessage::Burst { .. }))
            .count()
    }

    #[test]
    fn a_head_bonk_breaks_the_struck_brick_and_shatters_once() {
        let (struck, neighbour) = two_authored_bricks();
        let mut app = break_app();
        let id = authored_brick_id(&struck);
        app.world_mut().spawn((PrimaryPlayer, head_bonk_frame(id)));

        app.update();
        assert!(
            app.world().resource::<BrokenBricks>().is_broken(&struck),
            "the bonked brick is broken"
        );
        assert!(
            !app.world().resource::<BrokenBricks>().is_broken(&neighbour),
            "only the struck brick breaks — the id match is specific"
        );
        assert_eq!(
            drain_bursts(&mut app),
            1,
            "a fresh break shatters exactly once"
        );

        // The same contact next frame must not re-shatter: the brick is spent.
        app.update();
        assert_eq!(
            drain_bursts(&mut app),
            0,
            "an already-broken brick does not re-shatter"
        );
    }

    #[test]
    fn a_head_bonk_on_a_non_brick_breaks_nothing() {
        let mut app = break_app();
        app.world_mut()
            .spawn((PrimaryPlayer, head_bonk_frame(ae::GeoId::anon())));
        app.update();
        assert_eq!(
            app.world()
                .resource::<BrokenBricks>()
                .broken_names()
                .count(),
            0,
            "a plain block is not a brick"
        );
    }

    /// **END TO END: a broken brick is gone from the world a sweep reads.**
    ///
    /// ⚠ every link in this chain already had a test — the bonk marks the bit,
    /// the bit becomes a `removed_block_names` entry, the level authors
    /// `brick_<i>` with the matching `GeoId` — and Jon can still stand on a
    /// broken brick. So the untested claim was the COMPOSITION: that the name
    /// contributed to the overlay actually subtracts from the world
    /// `world_with_sandbox_solids` hands to collision. This asserts that
    /// directly, against the REAL level rather than a fixture, because a chain
    /// of green links is not a green chain.
    #[test]
    fn a_broken_brick_leaves_the_collision_world_the_body_reads() {
        let room = crate::level_1_1();
        let mut overlay = FeatureEcsWorldOverlay::default();
        // ⭐ the AUTHORED bricks, whatever they are called and however many there
        // are. This used to name `brick_name(0)` and `brick_name(1)` — positions
        // in a Rust array that no longer exists.
        let bricks: Vec<String> = room
            .world
            .blocks
            .iter()
            .filter(|b| block_kind_of(&b.name) == Some(MaryOBlockKind::Brick))
            .map(|b| b.name.clone())
            .collect();
        assert!(
            bricks.len() >= 2,
            "the level authors a brick wall to break into: {bricks:?}"
        );
        let (broken_one, neighbour) = (&bricks[0], &bricks[1]);

        // Nothing broken yet: the brick is solid, or this proves nothing.
        let before = ambition_platformer2d::world::collision::world_with_sandbox_solids(
            &room.world,
            &[],
            &overlay,
        );
        assert!(
            before.blocks.iter().any(|b| b.name == *broken_one),
            "`{broken_one}` must start solid in the composed collision world",
        );

        // Break it exactly as `break_bricks` would, then contribute as the
        // scheduled system does.
        let mut broken = BrokenBricks::default();
        assert!(broken.mark(broken_one), "a fresh brick marks broken");
        overlay
            .removed_block_names
            .extend(broken.broken_names().cloned());

        let after = ambition_platformer2d::world::collision::world_with_sandbox_solids(
            &room.world,
            &[],
            &overlay,
        );
        assert!(
            !after.blocks.iter().any(|b| b.name == *broken_one),
            "a broken brick must not be in the world a sweep reads — this is the \
             'she can stand on a broken brick' report, asserted at the composition",
        );
        assert!(
            after.blocks.iter().any(|b| b.name == *neighbour),
            "only the broken brick leaves; its neighbours are untouched",
        );
    }

    #[test]
    fn a_broken_brick_is_subtracted_from_the_collision_overlay() {
        let mut app = App::new();
        app.init_resource::<FeatureEcsWorldOverlay>();
        let mut broken = BrokenBricks::default();
        broken.mark("brick_alpha");
        broken.mark("brick_gamma");
        app.insert_resource(broken);
        app.add_systems(Update, contribute_broken_bricks_to_overlay);

        app.update();
        let removed = &app
            .world()
            .resource::<FeatureEcsWorldOverlay>()
            .removed_block_names;
        assert!(
            removed.contains(&"brick_alpha".to_string())
                && removed.contains(&"brick_gamma".to_string()),
            "broken bricks are named in removed_block_names: {removed:?}"
        );
        assert!(
            !removed.contains(&"brick_beta".to_string()),
            "an intact brick is not subtracted"
        );
    }

    #[test]
    fn a_reload_rearms_the_bricks() {
        let mut app = App::new();
        let mut broken = BrokenBricks::default();
        broken.mark("brick_alpha");
        app.insert_resource(broken);
        app.add_message::<RoomLoaded>();
        app.add_systems(Update, refill_bricks_on_room_loaded);

        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<RoomLoaded>>()
            .write(RoomLoaded {
                room_id: LEVEL_1_1_ROOM_ID.to_string(),
            });
        app.update();
        assert_eq!(
            app.world()
                .resource::<BrokenBricks>()
                .broken_names()
                .count(),
            0,
            "a level (re)load rebuilds the wall for the next lap"
        );
    }
}
