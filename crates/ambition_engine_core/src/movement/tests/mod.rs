//! Movement engine tests, split by topic.
//!
//! Submodules:
//! - [`clock`] — sim-vs-control clock separation, tiny-dt safety.
//! - [`ability_gates`] — pure "ability flag controls behavior" sanity.
//! - [`blink`] — press / hold / precision aim / soft-wall pass / grace.
//! - [`glide_and_air`] — glide cap, fast-fall, fly toggle, pogo.
//! - [`wall_collision`] — one-way, wall-jump, wall-cling, side-contact.
//! - [`climbing`] — ladder regions and Climbing body mode.
//! - [`ledge_grab`] — ledge grab latch + climb completion.
//! - [`combat_actions`] — dodge roll and shield/parry.
//!
//! Shared fixtures (`step_scratch`, `test_world`) live here;
//! submodules reach them via `super::`. Each test constructs a
//! `BodyClusterScratch` via
//! `BodyClusterScratch::new_with_abilities(spawn, abilities)` and
//! drives it through the cluster-native `_scratch` entry points.

use super::*;
use crate::body_clusters::BodyClusterScratch;
#[allow(unused_imports)]
use crate::test_support::*;
use crate::{Vec2, World};

pub(super) fn step_scratch(
    world: &World,
    scratch: &mut BodyClusterScratch,
    input: InputState,
) -> FrameEvents {
    update_player_with_tuning_scratch(world, scratch, input, 1.0 / 60.0, TEST_TUNING)
}

pub(super) fn test_world() -> World {
    let w = 1600.0;
    let h = 900.0;
    World {
        name: "movement test world".to_string(),
        size: Vec2::new(w, h),
        spawn: Vec2::new(210.0, h - 95.0),
        blocks: vec![
            crate::world::Block::solid("floor", Vec2::new(0.0, h - 48.0), Vec2::new(w, 48.0)),
            crate::world::Block::solid("left wall", Vec2::new(0.0, 0.0), Vec2::new(36.0, h)),
            crate::world::Block::solid("right wall", Vec2::new(w - 36.0, 0.0), Vec2::new(36.0, h)),
            crate::world::Block::solid("ceiling", Vec2::new(0.0, 0.0), Vec2::new(w, 24.0)),
        ],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
        chains: Vec::new(),
    }
}

mod ability_gates;
mod blink;
mod c4_reaction_seams;
mod climbing;
mod clock;
mod combat_actions;
mod contacts;
mod glide_and_air;
mod ledge_grab;
mod sweep_sample;
mod wall_collision;
