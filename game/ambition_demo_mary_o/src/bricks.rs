//! Mary-O's breakable bricks — the SECOND consumer of the reactive-block primitive.
//!
//! The ?-block powerup ([`crate::powerups`]) was the FIRST consumer of
//! `ContactSource::Block { id: GeoId }`: a head-bonk answers "*which* authored block
//! did I strike" by the block's durable [`GeoId`](ae::GeoId), with no point-matching.
//! A brick reuses that exact seam for the OPPOSITE effect — where the ?-block ADDS a
//! wand pickup, a bonked brick SUBTRACTS itself from the world. Two consumers, one
//! primitive, add vs subtract: the engine-for-other-games oracle wants a second user
//! to prove the primitive generalizes past the powerup, and this is it, with zero
//! engine edits beyond the render reconcile the subtraction always wanted.
//!
//! The removal is a mid-run World-mutation done the elegant way: the authored
//! [`RoomGeometry`](ae::RoomGeometry) stays immutable (it is swapped at room
//! boundaries, never edited mid-room), and a broken brick's name is contributed to
//! the collision overlay's per-frame `removed_block_names` — the SAME immutable-base
//! subtraction seam encounter gates use to drop authored blocks. Collision and
//! render both honour that list, so a broken brick stops colliding AND stops drawing.

use bevy::prelude::*;

use ambition_platformer2d::actors::actor::PrimaryPlayer;
use ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput;
use ambition_platformer2d::actors::features::FeatureEcsWorldOverlay;
use ambition_platformer2d::world::rooms::RoomLoaded;
use ambition_platformer2d::characters::equipment::WornEquipment;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::collision_semantics::{ContactKind, ContactSource};

use ambition_platformer2d::platformer::lifecycle::SessionWorldRef;

use crate::ldtk_vocabulary::{block_of, MaryOBlockLook};
use ambition_platformer2d::actors::session::reset::RoomReplayRequested;

/// Which bricks are broken this run, by their authored NAME.
///
/// this was a `u32` bitset over brick INDICES. An index is a position in a
/// Rust column array, and the level is authored now — there is no array, the
/// author can add a tenth brick, and inserting one would have renumbered every
/// brick after it *including the ones already recorded broken*. The name is what
/// the collision overlay subtracts anyway (`removed_block_names`), so storing
/// anything else meant converting on the way out.
///
/// `BTreeSet`, not `HashSet`, and that is the determinism contract.
/// [`contribute_broken_bricks_to_overlay`] ITERATES this every frame, and
/// std-hash iteration order is seeded per process — two peers would subtract the
/// same blocks in different orders. A `BTreeSet` iterates in name order,
/// everywhere, always.
///
/// `Clone` because it is ROLLBACK STATE. Which bricks are broken decides
/// what the room is made of — the overlay subtracts them from collision every
/// frame — so a rewind across a bonk that does not restore this leaves a wall
/// with a hole in it, or a hole with a wall in it.
#[derive(Resource, Default, Clone)]
pub struct BrokenBricks(std::collections::BTreeSet<String>);

impl BrokenBricks {
    /// A checksum over WHICH bricks are broken, order-independent.
    ///
    /// XOR of per-name hashes, not a hash of the iteration. The set is a
    /// `BTreeSet` so its order is already deterministic — but the projection is
    /// what two peers compare, and making it independent of container choice
    /// means a later switch to a `HashSet` cannot turn a matching pair of
    /// timelines into a reported desync. (Its sibling `SpentPowerBlocks` IS a
    /// `HashSet`, which is how this came up.)
    pub fn checksum(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        self.0.iter().fold(0u64, |acc, name| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            name.hash(&mut hasher);
            acc ^ hasher.finish()
        })
    }

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

/// The brick-break. A head contact (`ContactKind::Head`) against a brick —
/// identified by the durable `GeoId` the engine carries on `ContactSource::Block`,
/// NOT by point-matching — marks that brick broken and shatters it once, and
/// only if she is big enough to. The SAME contact seam
/// [`crate::powerups::bonk_power_blocks`] reads; a bonk resolves to a ?-block OR
/// a brick (their `GeoId` base indices are disjoint), never both.
///
/// Only tall or fire should be able to."*). Breaking took exactly two conditions — a `Head` contact
/// and an empty `Brick` — so a small Mary-O demolished a wall with her scalp. The value was already
/// in hand one file over: [`crate::powerups::reward_for`] threads the same `WornEquipment` through
/// to decide what a block PAYS, and this is the same question about what a block SUFFERS. A missing
/// argument, not a missing concept.
pub fn break_bricks(
    mut broken: ResMut<BrokenBricks>,
    mut vfx: MessageWriter<ambition_platformer2d::vfx::VfxMessage>,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    // her FORM rides the same query, `Option` because a body with no
    // equipment component at all is small — that is what small IS, not a bug.
    players: Query<(&PlayerBodyFrameOutput, Option<&WornEquipment>), With<PrimaryPlayer>>,
    // A `GeoId` names a block; only the room can say which one.
    geometry: SessionWorldRef<ae::RoomGeometry>,
) {
    let Ok((frame, worn)) = players.single() else {
        return;
    };
    // a system-wide `return`, and here that is honest. Guarding a whole
    // system on a singleton is usually a smell — the guarded value normally
    // feeds one call among several — but breaking masonry is the ENTIRE body of
    // this function. There is no other effect for a small Mary-O to still get,
    // so hoisting the read out of the contact loop costs nothing and says the
    // rule once. Her form cannot change mid-frame either.
    //
    // this asks [`crate::powerups::is_small`] rather than testing two
    // equipment ids, so the ladder stays the single authority on what a form is
    // — see its doc for why `wears(STAR_WAND_ID)` would have muted fire.
    if crate::powerups::is_small(worn) {
        return;
    }
    for contact in &frame.events.contacts {
        if contact.kind != ContactKind::Head {
            continue;
        }
        let ContactSource::Block { id, .. } = &contact.source else {
            continue;
        };
        // ask the ROOM, then ask the BLOCK what it is. This looked the id
        // up in a table of ids reconstructed from a column array, so an authored
        // brick was simply not in it.
        let Some(block) = crate::authored_block_by_id(&geometry.0, id) else {
            continue;
        };
        // See `MaryOBlockContents::breaks_when_empty`.
        if !block_of(&block.name).is_some_and(|authored| {
            authored.look == MaryOBlockLook::Brick && authored.contents.breaks_when_empty()
        }) {
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

/// Re-arm every brick when the room (re)loads or replays, so the next lap —
/// and the next LIFE — starts against a whole wall. Mirrors
/// [`crate::powerups::rearm_power_blocks_for_a_fresh_attempt`].
///
/// A death emits [`RoomReplayRequested`], which the host answers with `ResetRoomFeaturesEvent`;
/// `RoomLoaded` is written from exactly one place, an actual room load. The two are different
/// events on purpose, and per-attempt CONTENT state has to answer the replay — that is what
/// `ContentRoomReplayResetSet` is for, and the bosses were already using it.
///
/// the `room_id == LEVEL_1_1_ROOM_ID` gate is gone too. It predates 1-2,
/// and it meant a wall smashed in 1-2 stayed smashed through every reload of it.
/// Broken names are per-room authored and you can only stand in one room, so
/// "any room boundary re-arms everything" is both simpler and correct.
pub fn rearm_bricks_for_a_fresh_attempt(
    mut rooms: MessageReader<RoomLoaded>,
    mut replays: MessageReader<RoomReplayRequested>,
    mut broken: ResMut<BrokenBricks>,
) {
    // Both are drained every frame regardless of the other: a `||` would
    // short-circuit the second reader and leave its message queued to fire again.
    let reloaded = rooms.read().count() > 0;
    let replayed = replays.read().count() > 0;
    if reloaded || replayed {
        broken.clear();
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
    ///
    /// breakable bricks, not brick-LOOKING blocks, and the difference is
    /// now load-bearing: 1-1 authors a `Brick` holding a quasar (the folded
    /// stack at px 1600,288), which by design does not shatter. Selecting on the
    /// look alone picked it as a victim and this test failed the moment the
    /// level used the feature it was written to support. The filter mirrors
    /// `break_bricks`'s own predicate so the fixture cannot drift from it again.
    fn two_authored_bricks() -> (String, String) {
        let room = crate::level_1_1();
        let mut names: Vec<String> = room
            .world
            .blocks
            .iter()
            .filter(|b| {
                block_of(&b.name).is_some_and(|a| {
                    a.look == MaryOBlockLook::Brick && a.contents.breaks_when_empty()
                })
            })
            .map(|b| b.name.clone())
            .collect();
        names.sort();
        assert!(
            names.len() >= 2,
            "the level authors a brick wall: {names:?}"
        );
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

    /// The form every "a bonk breaks it" fixture below must wear.
    ///
    /// `a_head_bonk_on_a_non_brick_breaks_nothing` is the sharpest case: it would have passed
    /// for the wrong reason forever.
    ///
    /// Tall rather than fire because tall is the WEAKER of the two forms that
    /// may break masonry — a gate that accidentally demanded the beacon would
    /// still be caught here, and `only_a_tall_or_fire_mary_o_breaks_a_brick`
    /// covers the fire side explicitly.
    fn tall() -> WornEquipment {
        WornEquipment::new(vec![crate::powerups::star_wand()])
    }

    fn head_bonk_frame(id: ae::GeoId) -> PlayerBodyFrameOutput {
        let mut frame = PlayerBodyFrameOutput::default();
        frame
            .events
            .contacts
            .push(ae::collision_semantics::Contact {
                // A hand-built fixture contact: nothing arrived at this surface.
                impact_speed: 0.0,
                involuntary: false,
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
        // the REAL level, because `break_bricks` asks the room what it hit.
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
        app.world_mut()
            .spawn((PrimaryPlayer, head_bonk_frame(id), tall()));

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

    /// A LOADED brick does not break.
    ///
    /// breakability is DERIVED from the contents
    /// (`MaryOBlockContents::breaks_when_empty`), so the author never has to
    /// keep two fields agreeing. `bonk_power_blocks` takes the loaded brick and
    /// `break_bricks` declines it, off the same authored field.
    #[test]
    fn a_brick_with_something_in_it_is_not_broken_by_a_bonk() {
        use crate::ldtk_vocabulary::{reactive_block, MaryOBlock, MaryOBlockContents, MaryOPickup};

        let loaded = reactive_block(
            MaryOBlock::new(
                MaryOBlockLook::Brick,
                MaryOBlockContents::Always(MaryOPickup::Lantern),
            ),
            "loaded_brick",
            ae::Vec2::new(64.0, 64.0),
            ae::Vec2::splat(32.0),
        );
        let empty = reactive_block(
            MaryOBlock::new(MaryOBlockLook::Brick, MaryOBlockContents::Empty),
            "empty_brick",
            ae::Vec2::new(128.0, 64.0),
            ae::Vec2::splat(32.0),
        );
        let (loaded_id, empty_id) = (loaded.id.clone(), empty.id.clone());
        let (loaded_name, empty_name) = (loaded.name.clone(), empty.name.clone());

        let mut app = App::new();
        app.init_resource::<BrokenBricks>();
        app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
            app.world_mut(),
            ae::RoomGeometry(ae::World::new(
                "loaded brick fixture",
                ae::Vec2::new(640.0, 480.0),
                ae::Vec2::new(32.0, 400.0),
                vec![loaded, empty],
            )),
        );
        app.add_systems(Update, break_bricks);

        // ONE player, whose contact is rewritten between frames.  Spawning a
        // second `PrimaryPlayer` makes `break_bricks`' `players.single()` fail
        // and the system silently do nothing — which is how the control case
        // below first "passed" as a failure.
        let player = app
            .world_mut()
            .spawn((PrimaryPlayer, head_bonk_frame(loaded_id), tall()))
            .id();
        app.update();
        assert!(
            !app.world()
                .resource::<BrokenBricks>()
                .is_broken(&loaded_name),
            "a brick that holds a lantern must survive the bonk that pops it"
        );

        // and the empty one still breaks, in the same world and off the
        // same code path — without this the test would pass over a
        // `break_bricks` that had simply stopped working at all.
        *app.world_mut()
            .entity_mut(player)
            .get_mut::<PlayerBodyFrameOutput>()
            .expect("the player carries its frame") = head_bonk_frame(empty_id);
        app.update();
        assert!(
            app.world()
                .resource::<BrokenBricks>()
                .is_broken(&empty_name),
            "an empty brick is still ordinary breakable masonry"
        );
    }

    /// Only tall or fire should be able to."
    ///
    /// `break_bricks` did not take her form at all — it gated on the
    /// contact being a `Head` and the block being an empty `Brick`, and there
    /// was no third condition. A small Mary-O smashed masonry with her scalp.
    ///
    /// TWO-SIDED on purpose, and the second half is the load-bearing one.
    /// "Small cannot break" passes trivially against a `break_bricks` that has
    /// stopped breaking anything at all — a form gate written slightly wrong
    /// (reading the beacon only, or inverting the rank test) would look green
    /// from the small side alone. So the same brick, in the same world, through
    /// the same system, must still shatter for tall AND for fire.
    #[test]
    fn only_a_tall_or_fire_mary_o_breaks_a_brick() {
        use crate::powerups::{cinder_beacon, star_wand};
        use ambition_platformer2d::characters::equipment::WornEquipment;

        // ONE brick, three forms, three FRESH worlds. 1-1 authors exactly
        // two breakable bricks, so a form-per-brick fixture would have had to
        // invent a third — and it would also have left "did the right brick get
        // picked" as a live confound. Each `strike` builds its own `App` with
        // its own `BrokenBricks`, so the same masonry is whole again every time
        // and the ONLY thing that differs between the three calls is her form.
        let (brick, _) = two_authored_bricks();

        // ONE `PrimaryPlayer` per app — `break_bricks` uses `single()`, so a
        // second body makes the system silently do nothing and every assertion
        // about "did not break" passes for the wrong reason.
        let strike = |worn: Option<WornEquipment>| -> bool {
            let mut app = break_app();
            let mut body = app
                .world_mut()
                .spawn((PrimaryPlayer, head_bonk_frame(authored_brick_id(&brick))));
            if let Some(worn) = worn {
                body.insert(worn);
            }
            app.update();
            app.world().resource::<BrokenBricks>().is_broken(&brick)
        };

        assert!(
            !strike(None),
            "SMALL Mary-O must not headbutt `{brick}` apart — she wears no form \
             row at all, which is exactly what being small IS"
        );
        assert!(
            strike(Some(WornEquipment::new(vec![star_wand()]))),
            "TALL Mary-O (the star wand) must still break `{brick}` — without \
             this the gate above is satisfied by a system that broke nothing"
        );
        assert!(
            strike(Some(WornEquipment::new(vec![cinder_beacon()]))),
            "FIRE Mary-O (the cinder beacon) must still break `{brick}`. The \
             beacon is worn ALONE here: it is the top of the ladder and does \
             not imply the wand, so a gate that only asked `wears(STAR_WAND)` \
             would mute the strongest form in the game"
        );
    }

    /// she is TALL here on purpose. A small Mary-O breaks nothing for a
    /// reason that has nothing to do with the block not being a brick, so this
    /// test would still pass while having stopped testing its own claim.
    #[test]
    fn a_head_bonk_on_a_non_brick_breaks_nothing() {
        let mut app = break_app();
        app.world_mut()
            .spawn((PrimaryPlayer, head_bonk_frame(ae::GeoId::anon()), tall()));
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

    /// END TO END: a broken brick is gone from the world a sweep reads.
    #[test]
    fn a_broken_brick_leaves_the_collision_world_the_body_reads() {
        let room = crate::level_1_1();
        let mut overlay = FeatureEcsWorldOverlay::default();
        let bricks: Vec<String> = room
            .world
            .blocks
            .iter()
            .filter(|b| block_of(&b.name).is_some_and(|a| a.look == MaryOBlockLook::Brick))
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
        app.add_message::<RoomReplayRequested>();
        app.add_systems(Update, rearm_bricks_for_a_fresh_attempt);

        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<RoomLoaded>>()
            .write(RoomLoaded {
                room_id: crate::LEVEL_1_1_ROOM_ID.to_string(),
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

    #[test]
    fn a_death_replay_rearms_the_bricks() {
        let mut app = App::new();
        let mut broken = BrokenBricks::default();
        broken.mark("brick_alpha");
        app.insert_resource(broken);
        app.add_message::<RoomLoaded>();
        app.add_message::<RoomReplayRequested>();
        app.add_systems(Update, rearm_bricks_for_a_fresh_attempt);

        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<RoomReplayRequested>>()
            .write(RoomReplayRequested);
        app.update();
        assert_eq!(
            app.world()
                .resource::<BrokenBricks>()
                .broken_names()
                .count(),
            0,
            "a death replays the room, and the wall it rebuilds must be whole"
        );
    }
}
