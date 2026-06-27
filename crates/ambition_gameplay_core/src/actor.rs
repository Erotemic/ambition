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

/// The shared **movement-cluster components** every body carries — the 18
/// ancillary clusters (ground contact, wall, jump, dash, flight, blink, ledge,
/// dodge, shield, body-mode, environment contact, mana, offense, action buffer,
/// lifetime, combo trace, base size, ability mask) that, together with
/// [`BodyKinematics`], form the authoritative movement aggregate the shared
/// pipeline (`ae::update_body_with_tuning_clusters`) reads and writes.
///
/// These were historically named `Player*` and surfaced through `crate::player`,
/// which made every non-player module that names a body component import the
/// player. They are not player-specific — enemies, NPCs, and bosses all carry
/// them — so they are re-homed here on the neutral actor vocabulary under the
/// `Body*` convention (matching [`BodyKinematics`] / [`BodyHealth`] /
/// [`BodyCombat`]). The types `#[derive(Component)]` in `ambition_engine_core`;
/// this is the single import surface for them.
pub use ambition_engine_core::{
    BodyAbilities, BodyActionBuffer, BodyBaseSize, BodyBlinkState, BodyComboTrace, BodyDashState,
    BodyDodgeState, BodyEnvironmentContact, BodyFlightState, BodyGroundState, BodyJumpState,
    BodyLedgeState, BodyLifetime, BodyMana, BodyModeState, BodyOffense, BodyShieldState,
    BodyWallState,
};

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

/// The ONE combat / presentation-status component every body carries — the
/// player, enemies, NPCs, and bosses. The keystone collapse of the parallel
/// `PlayerCombatState` (the player's authoritative reaction/hit timers) and
/// `ActorCombatState` (the actor presentation read-model) into a single type, so
/// the HUD, nameplates, and animation read ONE component for any body.
///
/// The field sets were disjoint, so the union preserves both vocabularies: the
/// player fills the reaction timers (`hitstop_timer` / `damage_invuln_timer` /
/// `hitstun_timer` / `recoil_lock_timer` / `attacking`), while an actor fills the
/// status/attack fields (`alive` / `strike_count` / `attack_windup_timer` /
/// `attack_timer` / `training_dummy`, synced each frame from its authoritative
/// cluster state). `hit_flash` is the ONE damage-blink field, shared by both.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyCombat {
    /// Presentation flash (damage hit-blink) — the one field for every body.
    /// Decays in the player `cleanup_timers_system`; for an actor it is synced
    /// from the cluster each frame.
    pub hit_flash: f32,
    // ── Player reaction / control-lock timers ──
    /// Hitstop: freezes `time_scale` to 0 while positive.
    pub hitstop_timer: f32,
    /// Invulnerability window after taking damage.
    pub damage_invuln_timer: f32,
    /// Partial-control penalty after knockback.
    pub hitstun_timer: f32,
    /// Short HARD control-lock at the start of a knockback (no input authority).
    pub recoil_lock_timer: f32,
    /// Mirrored each frame from `ActivePlayerAttack::is_active()`.
    pub attacking: bool,
    // ── Actor status / attack-timeline presentation ──
    pub alive: bool,
    pub strike_count: i32,
    pub attack_windup_timer: f32,
    pub attack_timer: f32,
    pub training_dummy: bool,
}

impl BodyCombat {
    pub fn vulnerable(&self) -> bool {
        self.damage_invuln_timer <= 0.0
    }

    /// Reset the player reaction timers + attacking mirror (the actor status
    /// fields are owned by the per-frame sync from the cluster).
    pub fn reset(&mut self) {
        self.hit_flash = 0.0;
        self.hitstop_timer = 0.0;
        self.damage_invuln_timer = 0.0;
        self.hitstun_timer = 0.0;
        self.recoil_lock_timer = 0.0;
        self.attacking = false;
    }

    /// Presentation state for a peaceful actor (the former `ActorCombatState::peaceful`).
    pub fn peaceful(strike_count: i32, hit_flash: f32) -> Self {
        Self {
            alive: true,
            hit_flash,
            strike_count,
            ..Default::default()
        }
    }

    /// Presentation state for a hostile actor (the former `ActorCombatState::hostile`).
    pub fn hostile(
        alive: bool,
        hit_flash: f32,
        attack_windup_timer: f32,
        attack_timer: f32,
        training_dummy: bool,
    ) -> Self {
        Self {
            alive,
            hit_flash,
            attack_windup_timer,
            attack_timer,
            training_dummy,
            ..Default::default()
        }
    }
}
