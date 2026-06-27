//! The neutral **actor vocabulary** home for shared sim-state — the components
//! every actor carries, the player included.
//!
//! Establishing this module is step 4 (the keystone) of the unified-actors plan
//! (`docs/planning/engine/unified-actors.md` / `engine/architecture.md`): the
//! shared body/sim-state was historically surfaced through `crate::player`, which
//! made `crate::player` a universal dependency sink — ~20 of the non-player
//! modules imported it just to name a body component. Re-homing the shared types
//! here dissolves those back-edges so the runtime domains can extract into leaf
//! crates.
//!
//! **Rule:** new *shared* sim-state (state every actor has) lands here on the
//! actor vocabulary, never on a `Player*`-named component. Genuinely player-only
//! state (camera, HUD, device input, wallet) stays in `crate::player`.
//!
//! Slice 0 re-homed [`BodyKinematics`] (the single position / velocity / size /
//! facing component the player, enemies, NPCs, and bosses all share). Slice 0b
//! re-homes the entity markers [`PlayerEntity`] / [`PrimaryPlayer`] (already
//! foundation types) + the [`PrimaryPlayerOnly`] filter. Subsequent slices move
//! the combat/economy sim-state here.

use bevy::prelude::With;

pub use crate::platformer_runtime::body::BodyKinematics;
pub use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};

/// Query filter for "the one camera/HUD-owning player body" — `With<PlayerEntity>`
/// + `With<PrimaryPlayer>`. The neutral home for the filter every non-player system
/// uses to find the primary player (e.g. targeting, camera follow, HUD readouts).
pub type PrimaryPlayerOnly = (With<PlayerEntity>, With<PrimaryPlayer>);

/// The ONE health component every body carries — the player, enemies, NPCs, and
/// bosses. Wraps the shared [`ambition_characters::actor::Health`]. This is the
/// keystone collapse of the identical parallel wrappers `PlayerHealth` /
/// `ActorHealth` into one: every damage / heal / HUD / save / respawn system
/// reads and writes a single component, so health is body vocabulary, not a
/// per-actor-type concept.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyHealth {
    pub health: ambition_characters::actor::Health,
}

impl BodyHealth {
    pub fn new(health: ambition_characters::actor::Health) -> Self {
        Self { health }
    }

    pub fn current(self) -> i32 {
        self.health.current
    }

    pub fn max(self) -> i32 {
        self.health.max
    }

    pub fn heal(&mut self, amount: i32) {
        self.health.heal(amount);
    }

    /// Apply `amount` of damage; returns `true` if this killed the body.
    pub fn damage(&mut self, amount: i32) -> bool {
        self.health.damage(amount)
    }

    pub fn reset(&mut self) {
        self.health.reset();
    }

    pub fn alive(self) -> bool {
        self.health.alive()
    }
}
