//! Shared **body vocabulary** components — the health, combat-status, and wallet
//! every actor carries (the player, enemies, NPCs, and bosses alike).
//!
//! These re-homed down from `ambition_actors::actor` (unified-actors
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
/// trading NPCs) needs no `crate::avatar` dependency.
///
/// Decided (Jon): a coin/credits wallet, not item-as-currency.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BodyWallet {
    pub balance: i32,
}

/// Marks a body whose positive wallet balance absorbs one incoming hit.
///
/// The shared victim resolver spends the entire balance before HP or death is
/// evaluated, then publishes a `WalletShieldSpent` fact for game-specific
/// presentation. This is deliberately body vocabulary rather than a Sanic
/// special case: any persona may author a wallet-backed shield without adding
/// another damage pipeline.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BodyWalletShield;

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

/// **How a body's accumulated-damage meter relates to death.** (CM1)
///
/// Smash's percent and Ambition's HP are the SAME quantity read through two
/// policies. The meter itself is [`BodyHealth::damage_taken`]; this decides only
/// whether filling the pool KILLS.
///
/// ⚠ **it lives beside `BodyHealth` and travels with it**, which it did not use
/// to. It was authored per-archetype on `ActorTuning` and consulted in exactly
/// one place (`apply_actor_hit`), so the PLAYER's body had no death policy at
/// all — and a versus fighter is the adopted player. A policy the player cannot
/// carry cannot express "this fighter dies to the blast zone, not to the meter",
/// which is the entire point of the `Unbounded` variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeathPolicy {
    /// Dies when the meter fills the pool — Ambition today. THE DEFAULT, so
    /// every existing archetype is unchanged.
    #[default]
    HpDepleted,
    /// The meter never kills on its own; death comes from the WORLD — the
    /// blast-zone / OOB / fell-out gate the engine already owns.
    ///
    /// The body stays ALIVE and its pool stays full no matter how far the meter
    /// climbs, so `alive()` keeps answering `true`, hits keep landing, and
    /// knockback keeps growing — which is what makes a 188% body launchable off
    /// the stage by the one mechanism this variant reserves the right to kill it.
    Unbounded,
}

impl DeathPolicy {
    /// Whether filling the pool KILLS this body. `HpDepleted` (the default)
    /// does, so every existing kill path is byte-unchanged.
    pub fn kills_at_max(self) -> bool {
        matches!(self, DeathPolicy::HpDepleted)
    }
}

/// The ONE health component every body carries — the player, enemies, NPCs, and
/// bosses. Wraps the shared [`Health`]. This is the keystone collapse of the
/// identical parallel wrappers `PlayerHealth` / `ActorHealth` into one: every
/// damage / heal / HUD / save / respawn system reads and writes a single
/// component, so health is body vocabulary, not a per-actor-type concept.
///
/// ## The meter and the pool are two different quantities (S4)
///
/// They used to be one, and that was the defect. `damage_taken()` was
/// `max - current`, so it could not exceed `max` by construction — and
/// `Health::damage` both clamps `current` at zero AND returns early once the
/// body is not `alive()`, so a hit landing on an empty pool was DROPPED rather
/// than merely capped. An `Unbounded` body at 100% therefore stopped taking
/// damage, stopped growing its knockback, and could no longer be launched off
/// the stage. Selecting the variant bought an immortal punching bag, and the
/// shipped tests were green and silent about all of it.
///
/// So [`Self::accumulated`] is its own uncapped counter that EVERY landed hit
/// adds to, whatever the policy and whatever the pool is doing. The pool is
/// still the pool: under [`DeathPolicy::HpDepleted`] it drains and kills exactly
/// as before. Under [`DeathPolicy::Unbounded`] it simply never drains, and the
/// counter keeps climbing past 100%.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyHealth {
    pub health: Health,
    /// **Total damage this body has accumulated, uncapped** — the smash-percent
    /// axis. Not derived from the pool: see the type docs for why it cannot be.
    accumulated: i32,
    /// Whether filling the pool kills this body. Travels WITH the health so
    /// every body has one, including the player.
    policy: DeathPolicy,
}

impl BodyHealth {
    /// A body under the default policy: the pool kills when it empties.
    ///
    /// Signature unchanged on purpose — a dozen construction sites author a
    /// pool and nothing else, and every one of them means `HpDepleted`.
    pub fn new(health: Health) -> Self {
        Self {
            health,
            accumulated: 0,
            policy: DeathPolicy::default(),
        }
    }

    /// The same body under a declared death policy.
    pub fn with_policy(mut self, policy: DeathPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// **Rebuild a body's health EXACTLY, meter and policy included.**
    ///
    /// For the rollback decoder, and named for it: every other construction site
    /// authors a fresh body, where a zero meter and the default policy are the
    /// right answer. A RESTORE is the opposite — the values are the whole point,
    /// and `new()` silently substituting its own is how a fighter at 188% under
    /// `Unbounded` came back at 0% under `HpDepleted` (GPT 5.6, 2026-07-31).
    pub fn restored(health: Health, damage_taken: i32, policy: DeathPolicy) -> Self {
        Self {
            health,
            accumulated: damage_taken,
            policy,
        }
    }

    pub fn policy(self) -> DeathPolicy {
        self.policy
    }

    /// Change the policy of a LIVE body, leaving the meter where it is.
    ///
    /// The case this exists for is match activation: a fighter adopted into a
    /// versus match plays under the match's rules, and its body already exists.
    pub fn set_policy(&mut self, policy: DeathPolicy) {
        self.policy = policy;
    }

    pub fn current(self) -> i32 {
        self.health.current
    }

    pub fn max(self) -> i32 {
        self.health.max
    }

    /// **Accumulated damage this body has taken, uncapped.** Knockback growth
    /// scales off this, so a heavily-damaged body launches farther under the
    /// same hit — and keeps launching farther past 100%.
    pub fn damage_taken(self) -> i32 {
        self.accumulated
    }

    /// **The meter as a fraction of the pool, UNCLAMPED.** `1.88` is a legal
    /// answer and is how a HUD prints `188%`; `Health::ratio` cannot express it
    /// because it clamps to `0..=1` and is about the POOL, not the meter.
    pub fn damage_percent(self) -> f32 {
        if self.health.max <= 0 {
            0.0
        } else {
            self.accumulated as f32 / self.health.max as f32
        }
    }

    /// Healing repays the meter as well as refilling the pool — otherwise a
    /// healed body would keep launching as if it were still hurt.
    pub fn heal(&mut self, amount: i32) {
        if amount > 0 {
            self.accumulated = (self.accumulated - amount).max(0);
        }
        self.health.heal(amount);
    }

    /// **Apply `amount` of damage; returns `true` if this call killed the body.**
    ///
    /// THE damage authority for any body. The meter always advances on a landed
    /// hit; whether the pool follows it down, and whether emptying the pool
    /// kills, is the policy's business.
    pub fn damage(&mut self, amount: i32) -> bool {
        if self.health.invulnerable || amount <= 0 {
            return false;
        }
        // ⚠ NOT gated on `alive()`. That gate is what made the meter saturate:
        // a hit landing on an empty pool returned early and the accumulated
        // total never moved again.
        self.accumulated = self.accumulated.saturating_add(amount);
        if self.policy.kills_at_max() {
            self.health.damage(amount)
        } else {
            // The pool is not this body's death condition, so it does not drain
            // at all — `alive()` stays true and every later hit still lands.
            false
        }
    }

    /// Back to full: an empty meter and a full pool. The policy is a property of
    /// the body, not of its current state, so it survives.
    pub fn reset(&mut self) {
        self.accumulated = 0;
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

/// A body's ECS-owned animation signal timers.
///
/// **Body vocabulary, not player-only** — despite the anim rows it gates being
/// authored on the player's sheet first, every brain-driven body that plays a
/// slash, a landing, or a dash pre-roll carries one. It re-homed here from
/// `ambition_actors::avatar::components` (the S5/S6 player fold, refactor-chain
/// R6): it was the single biggest reason `crate::avatar` was still a universal
/// dependency sink — 18 non-player modules imported that module solely to name
/// this component.
///
/// All fields are presentation-only: they gate which sprite row plays and
/// decay independent of gameplay timers like hitstop or invulnerability.
/// Written directly by `cleanup_timers_system` / the melee swing / the dash;
/// the animation picker reads them. This is the authoritative source —
/// `write_player_ecs_components` does not touch it.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct BodyAnimFacts {
    /// Time remaining for the slash animation row.
    pub slash_anim_timer: f32,
    /// Time remaining for the post-touchdown landing pose.
    pub land_anim_timer: f32,
    /// True when the landing was fast enough for the hard-impact row.
    pub land_anim_hard: bool,
    /// Time remaining for the brief dash pre-roll pose.
    pub dash_startup_timer: f32,
    /// Previous frame's `dashing` fact; used to detect the dash rising edge.
    pub anim_prev_dashing: bool,
    /// Time remaining for the projectile-release `Shoot` pose. Armed by
    /// `update_projectiles` whenever a projectile body is spawned (any
    /// kind — Fireball/Hadouken/HadoukenSuper). Single-shot, short.
    pub shoot_anim_timer: f32,
    /// Set each frame by `update_projectiles` to mirror
    /// `PlayerProjectileState.charging.is_some()`. While true the
    /// player is holding a charge and the `Aim` row plays.
    pub aim_anim_active: bool,
    /// Time remaining for the wall-jump push-off pose. Armed by
    /// `handle_player_events` on a `MovementOp::WallJump` op. Distinct
    /// from `Jump` so the wall departure reads as a kick-off rather
    /// than a ground arc.
    pub wall_jump_anim_timer: f32,
    /// Time remaining for the interact-gesture pose. Armed when an
    /// interaction (door, NPC, pickup) consumes
    /// `interact_buffer_timer`.
    pub interact_anim_timer: f32,
    /// The body is curled into a persistent rolling ball (spin dash roll,
    /// morph ball). A STATE mirror like `aim_anim_active`, not a timer:
    /// whatever verb owns the curl re-derives it every frame, and the picker
    /// plays the looping `Roll` row while it holds.
    pub rolling: bool,
    /// Time remaining on the DEATH pose.
    ///
    /// A player body respawns the instant it dies, so its liveness is true again
    /// before anything could have drawn it dead — which is why the `Death` row
    /// was unreachable for the player no matter what a sheet published. This is
    /// the fact that makes a death VISIBLE: whatever owns the death beat arms it,
    /// and the picker plays `Death` above every other row until it runs out.
    ///
    /// A timer rather than a liveness read, precisely because the body is alive
    /// again by then. It is presentation state on an already-presentation
    /// cluster, so nothing about the sim depends on it.
    pub death_anim_timer: f32,
}

impl BodyAnimFacts {
    pub fn reset(&mut self) {
        *self = Self::default();
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
