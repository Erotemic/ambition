//! Shared **body vocabulary** components — the health, combat-status, and wallet
//! every actor carries (the player, enemies, NPCs, and bosses alike).
//!
//! These re-homed down from `ambition_gameplay_core::actor` (unified-actors
//! keystone, D2): they are leaf actor vocabulary — a body's hit points, its
//! combat/reaction status, and its coin balance — with no gameplay-shell deps,
//! so they belong beside [`super::Health`] on the reusable actor crate rather
//! than in the 95k game crate that everything imports just to name a body
//! component.

use bevy::prelude::Component;

use super::Health;

/// A body's coin/credits balance — the spendable currency a body carries, used
/// at merchants and credited by `PickupKind::Currency` collection. **Body
/// vocabulary, not player-only:** the player carries one (per-player in
/// multiplayer), and an NPC/enemy can carry one too (a body that drops currency
/// on death holds it here). Pay-for-use — most bodies simply never spawn with a
/// wallet. Was `PlayerWallet`; re-homed here so non-player economy (drops,
/// trading NPCs) needs no `crate::player` dependency.
///
/// Decided (Jon): a coin/credits wallet, not item-as-currency.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BodyWallet {
    pub balance: i32,
}

impl BodyWallet {
    /// Credit the wallet (clamped at zero so a negative `amount` can't drive it
    /// below zero).
    pub fn add(&mut self, amount: i32) {
        self.balance = (self.balance + amount).max(0);
    }

    /// Spend `amount` if affordable; returns `true` and debits on success.
    pub fn try_spend(&mut self, amount: i32) -> bool {
        if amount >= 0 && self.balance >= amount {
            self.balance -= amount;
            true
        } else {
            false
        }
    }
}

/// The ONE health component every body carries — the player, enemies, NPCs, and
/// bosses. Wraps the shared [`Health`]. This is the keystone collapse of the
/// identical parallel wrappers `PlayerHealth` / `ActorHealth` into one: every
/// damage / heal / HUD / save / respawn system reads and writes a single
/// component, so health is body vocabulary, not a per-actor-type concept.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyHealth {
    pub health: Health,
}

impl BodyHealth {
    pub fn new(health: Health) -> Self {
        Self { health }
    }

    pub fn current(self) -> i32 {
        self.health.current
    }

    pub fn max(self) -> i32 {
        self.health.max
    }

    /// Accumulated damage this body has taken — the smash-percent axis (CM1).
    /// It is `max - current` read through the existing pool; no parallel meter.
    /// Knockback growth scales off this so a heavily-damaged body launches
    /// farther under the same hit.
    pub fn damage_taken(self) -> i32 {
        (self.health.max - self.health.current).max(0)
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
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
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
    /// Mirrored each frame from `BodyMelee::is_active()`.
    pub attacking: bool,
    // ── Actor status / attack-timeline presentation ──
    /// Liveness MIRROR of the body's `BodyHealth` authority, written every frame:
    /// for an actor from its cluster `status.alive` (`sync_actor_components_from_cluster`),
    /// for the player from `health.current() > 0` (`write_player_ecs_components`).
    /// Read-model for presentation/AI; liveness-critical gameplay reads `BodyHealth`
    /// directly to avoid a tick of mirror lag.
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

    /// Advance the body-generic reaction timers one frame — the post-hit i-frame
    /// window, the damage-blink the renderer reads, and the §A2 stagger set
    /// (hitstun / recoil-lock / hitstop). ONE decay for every body: the actor tick
    /// and the boss tick both call this on their `BodyCombat`, retiring the two
    /// hand-copied five-line decay blocks (fable review §A1). Each clamps at zero.
    pub fn decay_reaction_timers(&mut self, dt: f32) {
        self.damage_invuln_timer = (self.damage_invuln_timer - dt).max(0.0);
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        self.hitstun_timer = (self.hitstun_timer - dt).max(0.0);
        self.recoil_lock_timer = (self.recoil_lock_timer - dt).max(0.0);
        self.hitstop_timer = (self.hitstop_timer - dt).max(0.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The shared reaction-timer decay (the ONE the actor tick AND the boss tick
    /// call, §A1) advances every reaction timer by `dt` and clamps at zero — so a
    /// nearly-expired window lands exactly on 0, not a small negative.
    #[test]
    fn decay_reaction_timers_advances_all_five_and_clamps_at_zero() {
        let mut combat = BodyCombat {
            hit_flash: 0.30,
            hitstop_timer: 0.02, // < dt → clamps to 0
            damage_invuln_timer: 0.50,
            hitstun_timer: 0.10,
            recoil_lock_timer: 0.05,
            ..Default::default()
        };
        combat.decay_reaction_timers(0.10);
        assert!((combat.hit_flash - 0.20).abs() < 1e-6);
        assert_eq!(combat.hitstop_timer, 0.0, "under-dt timer clamps to zero");
        assert!((combat.damage_invuln_timer - 0.40).abs() < 1e-6);
        assert_eq!(combat.hitstun_timer, 0.0);
        assert_eq!(combat.recoil_lock_timer, 0.0);
    }
}
