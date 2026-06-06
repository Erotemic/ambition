//! Unified body kinematics for every controllable body in the platformer.
//!
//! [`BodyKinematics`] is the single position / velocity / AABB-size / facing
//! component shared by the player, enemies/NPCs, and bosses. It replaces the
//! three historical parallel types (`PlayerKinematics`, `ActorKinematics`,
//! `BossKinematics`) so any code that operates on "a body" (orientation,
//! transit, vortex, brain effects, …) holds ONE query instead of branching
//! across three.
//!
//! ## Query-conflict discipline
//!
//! Because player, enemy, and boss entities now all carry `BodyKinematics`, any
//! single system that holds more than one `&mut BodyKinematics` query (or a
//! `&mut` query alongside another that can alias the same entity) must make the
//! queries provably disjoint with marker filters
//! (`With<PlayerEntity>` / `With<EnemyConfig>` / `With<BossConfig>`, plus
//! `Without<…>` guards where needed). Player / enemy / boss are mutually
//! exclusive archetypes, so those filters are sound. This is the same failure
//! mode that originally forced the boss onto its own type — handle it with
//! filters, never by re-splitting the component.
//!
//! Lives sandbox-side under the `platformer_runtime` facade for now; it batch-
//! moves into the `ambition_platformer_runtime` crate in a later pass (finish
//! cleanup first, extract later).

use bevy::prelude::Component;

use crate::engine_core::{Aabb, Vec2};

/// Position, velocity, AABB size, and facing direction of a body.
///
/// Shared by the player, enemies/NPCs, and bosses. Bosses float and never
/// integrate `vel` themselves (the brain emits a fresh `desired_vel` each tick
/// for `integrate_body`), so a boss simply leaves `vel` at [`Vec2::ZERO`].
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BodyKinematics {
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub facing: f32,
}

impl Default for BodyKinematics {
    /// Player-flavored default (the only `::default()` callers are player
    /// spawn helpers): a default-sized body at the origin, at rest, facing
    /// right. Matches the pre-unification `PlayerKinematics::default`.
    fn default() -> Self {
        let body = crate::engine_core::movement::default_player_body_size();
        Self {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            size: body,
            facing: 1.0,
        }
    }
}

impl BodyKinematics {
    /// The body's world-space AABB (centered on `pos`, half-extents `size/2`).
    pub fn aabb(self) -> Aabb {
        Aabb::new(self.pos, self.size * 0.5)
    }
}
