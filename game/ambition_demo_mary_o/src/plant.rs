//! The Snake Plant: the plant that comes up out of a pipe.
//!
//! It is a snake that wears leaves. It stands in a pipe, comes up, bites the
//! air, and goes down again. To touch it from any side is a hit: no rule here
//! makes its head safe, so a stomp lands as body contact. A spark, a quasar or
//! a shell kills it, as they kill each body with one point of health.
//!
//! Its row is a body that stands still (`StandStill`). This module moves it:
//! the place of a plant on each tick is a function of the simulation tick and
//! of the pipe it stands in, so the cycle is no state of its own and a
//! rollback has nothing more to restore. It does not come up while a player is
//! on its pipe or beside it.

use bevy::prelude::*;

use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::combat::actor_tuning::{ActorConfig, ContactThreatWithdrawn};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};

use crate::ldtk_vocabulary::{pipe_of, MaryOPipeMouth};

/// The brain key a level placement carries beside `character_id`.
pub const SNAKE_PLANT_BRAIN_KEY: &str = "mary_o_snake_plant";
/// The catalog id, and the sheet target.
pub const SNAKE_PLANT_CHARACTER_ID: &str = "snake_plant";
/// The display name of its row.
pub const SNAKE_PLANT_DISPLAY_NAME: &str = "Snake Plant";

/// Ticks of one cycle: up, out, down, hidden.
const CYCLE_TICKS: u64 = 330;
/// Ticks to come all the way up, and to go all the way down.
const TRAVEL_TICKS: u64 = 36;
/// Ticks it stays all the way out.
const OUT_TICKS: u64 = 108;
/// A player this near the side of the pipe (px) keeps a hidden plant hidden.
const SHY_REACH: f32 = 40.0;
/// Below this part of its height out of the pipe, the plant hurts nobody: a
/// body that stands on the pipe, or rides down it, does not touch the plant.
const HARMLESS_BELOW: f32 = 0.12;

/// Is this body a Snake Plant?
pub fn is_snake_plant_brain(brain: &CharacterBrain) -> bool {
    matches!(brain, CharacterBrain::Custom(key) if key == SNAKE_PLANT_BRAIN_KEY)
}

/// How much of the plant the cycle wants out of the pipe at `tick`, from 0
/// (hidden) to 1 (all the way out). It changes by `1 / TRAVEL_TICKS` at most
/// from one tick to the next.
pub fn wanted_out(tick: u64) -> f32 {
    let at = tick % CYCLE_TICKS;
    let rise_end = TRAVEL_TICKS;
    let out_end = rise_end + OUT_TICKS;
    let sink_end = out_end + TRAVEL_TICKS;
    if at < rise_end {
        at as f32 / TRAVEL_TICKS as f32
    } else if at < out_end {
        1.0
    } else if at < sink_end {
        1.0 - (at - out_end) as f32 / TRAVEL_TICKS as f32
    } else {
        0.0
    }
}

/// The next part of the plant out of the pipe: it follows `wanted` at the
/// speed of the cycle, and a plant that is hidden stays hidden while `shy`.
pub fn next_out(now: f32, wanted: f32, shy: bool) -> f32 {
    if shy && now <= 0.0 {
        return 0.0;
    }
    let step = 1.0 / TRAVEL_TICKS as f32;
    now + (wanted - now).clamp(-step, step)
}

/// The pipe a plant at `pos` stands in: the pipe with its mouth up whose width
/// has the plant in it, and whose mouth is nearest to the plant.
fn pot_of(world: &ae::World, pos: ae::Vec2) -> Option<ae::Aabb> {
    world
        .blocks
        .iter()
        .filter(|block| {
            pipe_of(&block.name).is_some_and(|pipe| pipe.mouth == MaryOPipeMouth::Up)
                && block.aabb.min.x <= pos.x
                && block.aabb.max.x >= pos.x
        })
        .map(|block| block.aabb)
        .min_by(|a, b| (a.min.y - pos.y).abs().total_cmp(&(b.min.y - pos.y).abs()))
}

/// Move each Snake Plant to its place in its cycle.
///
/// A plant that stands in no pipe is not moved: it is a plant that is always
/// out. The plant is put on its path after the movement step of the tick, so
/// the path wins, and it says that its pose is not its own
/// (`PoseOwnedExternally`), so the movement step does not pull it down or put
/// it out of the pipe.
pub fn run_snake_plants(
    mut commands: Commands,
    tick: Option<Res<ambition_platformer2d::time::SimTick>>,
    geometry: ambition_platformer2d::platformer::lifecycle::LiveRoomOf<ae::RoomGeometry>,
    players: Query<&ae::BodyKinematics, With<PrimaryPlayer>>,
    mut plants: Query<
        (
            Entity,
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
            &ActorConfig,
            &BodyHealth,
            &mut ContactThreatWithdrawn,
            Has<ae::PoseOwnedExternally>,
        ),
        (Without<PrimaryPlayer>, Without<PlayerEntity>),
    >,
) {
    // A world with no simulation clock (a test of one rule) has no cycle.
    let Some(tick) = tick else {
        return;
    };
    let player = players.single().ok().map(|kin| kin.aabb());
    for (entity, clusters, mut model, config, health, mut withdrawn, held) in &mut plants {
        if !is_snake_plant_brain(&config.brain) || !health.alive() {
            continue;
        }
        let mut item = clusters;
        let mut clusters = item.as_clusters_mut();
        let (pos, half_height) = (clusters.kinematics.pos, clusters.kinematics.size.y * 0.5);
        let Some(pot) = geometry.of(entity).and_then(|room| pot_of(&room.0, pos)) else {
            continue;
        };
        // Out: its feet on the mouth. Hidden: its head under the mouth.
        let (out_y, hidden_y) = (pot.min.y - half_height, pot.min.y + half_height + 1.0);
        let now = ((hidden_y - pos.y) / (hidden_y - out_y)).clamp(0.0, 1.0);
        let shy = player.is_some_and(|player| {
            player.max.x > pot.min.x - SHY_REACH
                && player.min.x < pot.max.x + SHY_REACH
                && player.max.y <= pot.max.y
        });
        // Each pipe has its own place in the cycle, from where it stands, so a
        // row of plants does not come up as one.
        let offset = (pot.min.x.max(0.0) as u64 / 16) * 53;
        let out = next_out(now, wanted_out(tick.get() + offset), shy);
        ambition_platformer2d::engine_core::movement::transit_body(
            &mut model,
            &mut clusters,
            ae::Vec2::new((pot.min.x + pot.max.x) * 0.5, hidden_y + (out_y - hidden_y) * out),
            ambition_platformer2d::engine_core::movement::TransitVelocity::Zero,
        );
        let harmless = out < HARMLESS_BELOW;
        if withdrawn.0 != harmless {
            withdrawn.0 = harmless;
        }
        if !held {
            commands.entity(entity).try_insert(ae::PoseOwnedExternally);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plant is put where the cycle says on each tick. A cycle that jumps
    /// is a plant that is in a body it was not near one tick before.
    #[test]
    fn the_cycle_is_hidden_then_out_and_has_no_jump() {
        let step = 1.0 / TRAVEL_TICKS as f32 + 1e-4;
        let (mut hidden, mut out) = (0, 0);
        for tick in 0..CYCLE_TICKS * 2 {
            let (a, b) = (wanted_out(tick), wanted_out(tick + 1));
            assert!((a - b).abs() <= step, "the cycle jumps from {a} to {b} at tick {tick}");
            hidden += usize::from(a == 0.0);
            out += usize::from(a == 1.0);
        }
        assert!(hidden > 0 && out > 0, "the plant is hidden for {hidden} ticks and out for {out}");
    }

    /// The classic rule: it does not come up under a player. A plant that is
    /// out goes on with its cycle, so it is not held out either.
    #[test]
    fn a_hidden_plant_stays_hidden_beside_a_player_and_a_plant_that_is_out_goes_down() {
        assert_eq!(next_out(0.0, 1.0, true), 0.0);
        assert!(next_out(0.0, 1.0, false) > 0.0);
        assert!(next_out(1.0, 0.0, true) < 1.0);
    }
}
