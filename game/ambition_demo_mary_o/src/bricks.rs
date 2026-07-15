//! Mary-O's breakable bricks — the SECOND consumer of the reactive-block primitive.
//!
//! The ?-block powerup ([`crate::powerups`]) was the FIRST consumer of
//! `ContactSource::Block { id: GeoId }`: a head-bonk answers "*which* authored block
//! did I strike" by the block's durable [`GeoId`](ae::GeoId), with no point-matching.
//! A brick reuses that exact seam for the OPPOSITE effect — where the ?-block ADDS a
//! milk pickup, a bonked brick SUBTRACTS itself from the world. Two consumers, one
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

use ambition::actors::actor::PrimaryPlayer;
use ambition::actors::avatar::PlayerBodyFrameOutput;
use ambition::actors::features::FeatureEcsWorldOverlay;
use ambition::actors::rooms::RoomLoaded;
use ambition::engine_core as ae;
use ambition::engine_core::collision_semantics::{ContactKind, ContactSource};

use crate::{brick_index_for, brick_min, brick_name, BRICK_COUNT, LEVEL_1_1_ROOM_ID, T};

// One bit per brick in [`BrokenBricks`]; the level authors far fewer than 32.
const _: () = assert!(
    BRICK_COUNT <= 32,
    "BrokenBricks packs each brick into a u32 bit"
);

/// Which bricks are broken this run. A fixed bitset over the level's brick indices,
/// NOT a `HashSet`: [`contribute_broken_bricks_to_overlay`] ITERATES it every frame,
/// and the sim determinism contract bans std-hash iteration (whose order is seeded
/// per process). A positional bitset iterates in a stable index order.
/// [`refill_bricks_on_room_loaded`] clears it on every (re)load so a cyclic replay
/// re-arms the wall — the brick twin of [`crate::powerups::SpentPowerBlocks`].
#[derive(Resource, Default)]
pub struct BrokenBricks(u32);

impl BrokenBricks {
    fn is_broken(&self, i: usize) -> bool {
        self.0 & (1 << i) != 0
    }

    /// Mark brick `i` broken; returns `true` only on the FRESH break (so the caller
    /// shatters it exactly once, never every frame the bonk contact re-reports).
    fn mark(&mut self, i: usize) -> bool {
        let bit = 1 << i;
        let newly = self.0 & bit == 0;
        self.0 |= bit;
        newly
    }

    fn clear(&mut self) {
        self.0 = 0;
    }

    fn broken_indices(&self) -> impl Iterator<Item = usize> + '_ {
        (0..BRICK_COUNT).filter(move |&i| self.is_broken(i))
    }
}

/// **The brick-break.** A head contact (`ContactKind::Head`) against a brick —
/// identified by the durable `GeoId` the engine carries on `ContactSource::Block`,
/// NOT by point-matching — marks that brick broken and shatters it once. The SAME
/// contact seam [`crate::powerups::bonk_power_blocks`] reads; a bonk resolves to a
/// ?-block OR a brick (their `GeoId` base indices are disjoint), never both.
pub fn break_bricks(
    mut broken: ResMut<BrokenBricks>,
    mut vfx: MessageWriter<ambition::vfx::VfxMessage>,
    players: Query<&PlayerBodyFrameOutput, With<PrimaryPlayer>>,
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
        let Some(i) = brick_index_for(id) else {
            continue;
        };
        if broken.mark(i) {
            // A fresh break shatters into brick-red shards through the engine's
            // shared particle seam — the same `VfxMessage::Burst` the crony squash
            // pops, so a brick reads as breaking with no bespoke vfx.
            let center = brick_min(i) + ae::Vec2::splat(T * 0.5);
            vfx.write(ambition::vfx::VfxMessage::Burst {
                pos: center,
                count: 14,
                speed: 155.0,
                color: [0.72, 0.35, 0.22, 1.0],
                kind: ambition::vfx::ParticleKind::Shard,
            });
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
/// [`rebuild_feature_ecs_world_overlay`](ambition::actors::features::rebuild_feature_ecs_world_overlay)
/// clears the list (its clean-slate-per-frame contract), exactly as
/// `contribute_encounter_lock_walls` does for `gate_solids`.
pub fn contribute_broken_bricks_to_overlay(
    broken: Res<BrokenBricks>,
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
) {
    overlay
        .removed_block_names
        .extend(broken.broken_indices().map(brick_name));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brick_id;

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
        app.add_message::<ambition::vfx::VfxMessage>();
        app.add_systems(Update, break_bricks);
        app
    }

    fn drain_bursts(app: &mut App) -> usize {
        app.world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ambition::vfx::VfxMessage>>()
            .drain()
            .filter(|m| matches!(m, ambition::vfx::VfxMessage::Burst { .. }))
            .count()
    }

    #[test]
    fn a_head_bonk_breaks_the_struck_brick_and_shatters_once() {
        let mut app = break_app();
        app.world_mut()
            .spawn((PrimaryPlayer, head_bonk_frame(brick_id(1))));

        app.update();
        assert!(
            app.world().resource::<BrokenBricks>().is_broken(1),
            "the bonked brick is broken"
        );
        assert!(
            !app.world().resource::<BrokenBricks>().is_broken(0),
            "only the struck brick breaks — the GeoId match is specific"
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
                .broken_indices()
                .count(),
            0,
            "a plain block is not a brick"
        );
    }

    #[test]
    fn a_broken_brick_is_subtracted_from_the_collision_overlay() {
        let mut app = App::new();
        app.init_resource::<FeatureEcsWorldOverlay>();
        let mut broken = BrokenBricks::default();
        broken.mark(0);
        broken.mark(2);
        app.insert_resource(broken);
        app.add_systems(Update, contribute_broken_bricks_to_overlay);

        app.update();
        let removed = &app
            .world()
            .resource::<FeatureEcsWorldOverlay>()
            .removed_block_names;
        assert!(
            removed.contains(&brick_name(0)) && removed.contains(&brick_name(2)),
            "broken bricks are named in removed_block_names: {removed:?}"
        );
        assert!(
            !removed.contains(&brick_name(1)),
            "an intact brick is not subtracted"
        );
    }

    #[test]
    fn a_reload_rearms_the_bricks() {
        let mut app = App::new();
        let mut broken = BrokenBricks::default();
        broken.mark(0);
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
                .broken_indices()
                .count(),
            0,
            "a level (re)load rebuilds the wall for the next lap"
        );
    }
}
