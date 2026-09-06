//! What stayed behind when the wielded ability kit was carved out (D33,
//! 2026-09-03). The kit itself is [`ambition_abilities`].
//!
//! ⛔⛔ THESE ARE NOT "THE LEFTOVER ABILITIES". They are two groups that share a
//! directory name with abilities and nothing else:
//!
//! * [`traversal`] — `possession`, `teleport`, `trapdoor`, `flyline`. Registered
//!   by `ambition_platformer2d_runtime`, not by the item-pickup family, and
//!   `possession` is named 87 times outside this directory (`teleport` 61) by (counted within
//!   the actor monolith, minus its own `abilities/`; across `crates` and `game`
//!   the same grep gives 256 and 603, so the scope is part of the figure)
//!   `crate::body_custody`, `crate::control::authority`,
//!   `crate::features::ecs::dormancy` and `crate::control::input_systems`. That
//!   is control authority; carving it into an abilities crate would rename the
//!   kernel's coupling rather than reduce it.
//! * [`thrown`] — the puppy-slug gun, which SPAWNS A BODY through the
//!   crate-private `features::spawn_runtime_minion`. The cross-crate seam for
//!   that already exists (`ambition_vfx::Effect::Summon` → `SummonSpec` →
//!   `ActorConstructionParams::SummonedMinion`); the gun is the one caller
//!   bypassing it, so moving it is a behaviour change and not a file move.
//!
//! `docs/planning/engine/actor-monolith-decomposition.md` carries both
//! arguments with their numbers, so neither group gets carved by line count.
//!
//! Possession's `PossessionState` + `ControlledSubject` resources are still
//! initialized by [`AmbitionAbilitiesPlugin`]; its systems stay chained inside
//! `crate::schedule::plugins::register_player_simulation_systems` alongside the
//! player tick, because possession is a pure SEAT REDIRECT.

pub mod thrown;
pub mod traversal;

use bevy::prelude::*;

/// Registers the `App` state the kernel-resident half owns.
pub struct AmbitionAbilitiesPlugin;

impl Plugin for AmbitionAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<traversal::possession::PossessionState>();
        app.init_resource::<ambition_platformer2d_shared_tangle::markers::ControlledSubject>();
        // Its sibling: what to frame when nothing is driving a body.
        app.init_resource::<ambition_platformer2d_shared_tangle::markers::FramedCast>();
    }
}
