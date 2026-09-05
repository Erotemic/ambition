//! Body-generic ECS vocabulary shared by player, enemy, NPC, and boss simulation.

use bevy::prelude::Component;

use super::Health;

/// Optional coin/credit wallet carried by a body.
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

/// How the accumulated-damage meter relates to death.
/// `BodyHealth::damage_taken` is the meter; this policy decides whether filling the pool kills.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeathPolicy {
    /// The health pool kills the body when depleted.
    #[default]
    HpDepleted,
    /// The meter never kills by itself; world/OOB rules own death.
    /// The health pool stays full while accumulated damage may grow past 100%.
    Unbounded,
}

impl DeathPolicy {
    /// Whether filling the health pool kills this body.
    pub fn kills_at_max(self) -> bool {
        matches!(self, DeathPolicy::HpDepleted)
    }
}

/// The health component shared by every body.
/// The pool and accumulated-damage meter are distinct: under `Unbounded` the pool stays full
/// while the meter continues to grow.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyHealth {
    pub health: Health,
    /// Total damage this body has accumulated, uncapped — the smash-percent
    /// axis. Not derived from the pool: see the type docs for why it cannot be.
    accumulated: i32,
    /// Whether filling the pool kills this body. Travels WITH the health so
    /// every body has one, including the player.
    policy: DeathPolicy,
}

impl BodyHealth {
    /// Construct a fresh body under the default `HpDepleted` policy.
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

    /// Restore health, accumulated damage, and death policy exactly from rollback state.
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

    /// Change death policy without changing the current accumulated-damage meter.
    pub fn set_policy(&mut self, policy: DeathPolicy) {
        self.policy = policy;
    }

    pub fn current(self) -> i32 {
        self.health.current
    }

    pub fn max(self) -> i32 {
        self.health.max
    }

    /// Accumulated damage this body has taken, uncapped. Knockback growth
    /// scales off this, so a heavily-damaged body launches farther under the
    /// same hit — and keeps launching farther past 100%.
    pub fn damage_taken(self) -> i32 {
        self.accumulated
    }

    /// The meter as a fraction of the pool, UNCLAMPED. `1.88` is a legal
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

    /// Apply damage and report whether this call killed the body.
    /// Accumulated damage advances independently of whether the pool is the death condition.
    pub fn damage(&mut self, amount: i32) -> bool {
        if self.health.invulnerable.any() || amount <= 0 {
            return false;
        }
        self.accumulated = self.accumulated.saturating_add(amount);
        if self.policy.kills_at_max() {
            self.health.damage(amount)
        } else {
            // Under `Unbounded`, the pool is not the death condition and does not drain.
            false
        }
    }

    /// PAY health for something, never falling below `floor`.
    ///
    /// ⭐⭐ A COST IS NOT AN INJURY, and the whole reason this is not
    /// [`Self::damage`] is the list of things damage does that a price must not.
    /// Damage consults invulnerability — a fighter still holding respawn
    /// protection would get her special FREE, which is not a discount anybody
    /// authored. Damage reports a kill — a move that could end the match by
    /// being pressed is not a cost, it is a suicide button. And damage is
    /// attributed: something hit you.
    ///
    /// ⭐ WHAT IT KEEPS IS THE METER. Spending health advances `accumulated`
    /// exactly as an injury would, because in a platform fighter the meter is
    /// the real currency: a Medic who has bought three bursts of tempo launches
    /// farther for the rest of the stock, and that is the price. Charging the
    /// pool and not the meter would make the cost invisible at the only moment
    /// it matters.
    ///
    /// ⛔ IT PAYS WHAT IT CAN AFFORD. At or below the floor it takes nothing
    /// and the move still happens — deliberately. The alternative is a fighter
    /// whose special stops existing at low health, which is when she needs it;
    /// a body down to its last point of margin has already paid.
    ///
    /// Returns what was actually taken.
    pub fn spend(&mut self, amount: i32, floor: i32) -> i32 {
        if amount <= 0 {
            return 0;
        }
        let floor = floor.max(1);
        let affordable = (self.health.current - floor).max(0);
        let paid = amount.min(affordable);
        if paid == 0 {
            return 0;
        }
        self.health.current -= paid;
        self.accumulated = self.accumulated.saturating_add(paid);
        paid
    }

    /// STATE the meter outright — a ruleset placing a body at an authored
    /// starting damage."""
    ///
    /// ⛔⛔ NOT A HIT, AND THE DIFFERENCES ARE THE POINT. It does not consult
    /// invulnerability (nobody attacked), it does not drain the pool (a starting
    /// state is not an injury), and it does not report a death. Sudden death's
    /// 150% is the customer: both survivors are placed on the edge of a launch
    /// so the next clean connect ends the match, and routing that through
    /// [`Self::damage`] would refuse it for a body still holding respawn
    /// protection — which is exactly the body a level timeout can leave standing.
    ///
    /// ⛔ negative is clamped away: the meter is accumulated damage and there is
    /// no such thing as less than none.
    pub fn set_damage_taken(&mut self, damage: i32) {
        self.accumulated = damage.max(0);
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

/// Shared combat-reaction and presentation status for every body.
/// Reaction timers use one decay and reset path across all controller/body kinds.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyCombat {
    /// Presentation flash (damage hit-blink) — the one field for every body,
    /// decayed for every body by [`Self::decay_reaction_timers`].
    pub hit_flash: f32,
    // ── Player reaction / control-lock timers ──
    /// Hitstop: freezes `time_scale` to 0 while positive.
    pub hitstop_timer: f32,
    /// THIS BODY OWES ONE AUTOMATIC DISPLACEMENT — banked while its hitlag
    /// runs, spent on the first step after the freeze lifts
    /// ([`ambition_platformer2d_core::TraversalAbilityTuning::asdi_step`]).
    ///
    /// ⛔ A LATCH RATHER THAN A TIMER COMPARISON. "Is this the last tick of
    /// hitlag?" can be asked as `hitstop_timer <= dt`, but only by a reader
    /// that runs BEFORE the decay system — and the decay is a separate system
    /// whose order relative to the body step is not declared. A latch is
    /// answered by two consecutive steps of the same function and cannot be
    /// silenced by scheduling.
    ///
    /// A fresh hit that re-arms the freeze mid-flight simply banks it again,
    /// which is right: that is a new hit and it owes its own displacement.
    ///
    /// ⭐ ONE PAYMENT PER FREEZE EPISODE, NOT PER HIT, and the `bool` states that
    /// exactly. Hits arriving during hitlag EXTEND one freeze rather than
    /// queueing behind it, so the body is displaced once when that episode ends
    /// — the beat a player actually reads. A per-hit counter would pay a
    /// multihit several displacements out of a single freeze, which is a
    /// different mechanic and not this one.
    pub asdi_owed: bool,
    /// Landing lag: the authored recovery an aerial move owes for touching
    /// down before it finished.
    ///
    /// A HARD control lock while positive, exactly like [`Self::recoil_lock_timer`] — and
    /// deliberately NOT that field.
    ///
    /// `0.0` for a body whose move authored no landing lag, which is every move
    /// that has not opted in — an aerial that lands is an ordinary landing
    /// unless the move says otherwise.
    pub landing_lag_timer: f32,
    /// Invulnerability window after taking damage.
    pub damage_invuln_timer: f32,
    /// Partial-control penalty after knockback.
    pub hitstun_timer: f32,
    /// Short HARD control-lock at the start of a knockback (no input authority).
    pub recoil_lock_timer: f32,
    /// ASLEEP: a control status a MOVE put this body into, not a consequence of
    /// being hit.
    ///
    /// A HARD control lock while positive, exactly like
    /// [`Self::recoil_lock_timer`] — and deliberately NOT that field, for the
    /// reason the landing lag beside it gives: the LOCK is shared and the CAUSE
    /// is not. "Why is this fighter helpless" must keep one answer per cause, or
    /// presentation and a trace both have to guess.
    ///
    /// ⛔ NOT `BodyShieldState::break_timer` EITHER, which is the shortcut this
    /// field exists to refuse. That timer is the dizzy a broken guard owes and
    /// presentation draws it as one; a sleep borrowing it would render as a
    /// shield break and read in a trace as one.
    ///
    /// ⚠ THIS IS A DISABLE, NOT YET A SLEEP. It buys "cannot act for a
    /// duration" and wake-on-damage. What it does NOT buy is the specific POSE
    /// or the MASH escape, and those are what make a sleep richer than a
    /// disable — neither is expressible as a timer in a `max`.
    ///
    /// `0.0` for every body nothing has put to sleep, which is all of them
    /// until a move says otherwise.
    pub sleep_timer: f32,
    /// SUPER ARMOR: an authored `WindowTag::Armor` window on the move this body
    /// is playing is holding it through hits.
    ///
    /// Not invulnerability, and deliberately not carried as one: an armoured
    /// body IS hit and takes the damage, it simply does not answer for it — no
    /// launch, no hitstun, no recoil lock. It lives here rather than being
    /// threaded to the reaction because `apply_body_hit_reaction` already holds
    /// this component and both damage roads reach it; a parameter would have to
    /// be plumbed through two twelve-argument call chains and could be forgotten
    /// on one of them.
    ///
    /// DERIVED, republished every tick from the live `MovePlayback` by
    /// `ambition_platformer2d::combat::moveset::project_move_defense_windows` — so a move
    /// ending retracts it by being rewritten rather than by anyone remembering
    /// to clear it. Never write it from anywhere else.
    pub armored: bool,
    // ── Actor status / attack-timeline presentation ──
    pub training_dummy: bool,
}

impl BodyCombat {
    /// Hard control lock for this frame: the maximum of recoil lock and landing lag.
    /// `hitstun_timer` is excluded because hitstun reduces authority rather than removing it.
    pub fn hard_lock_timer(&self) -> f32 {
        // ⭐ THE FIFTH CAUSE, and the seam was already a max over named ones —
        // see `attack_support`'s `hard_lock_timer`, which folds this in beside
        // the guard break, the shieldstun and the shield-drop lag.
        self.recoil_lock_timer
            .max(self.landing_lag_timer)
            .max(self.sleep_timer)
    }

    /// Whether this body is currently in hitlag.
    pub fn is_in_hitlag(&self) -> bool {
        self.hitstop_timer > 0.0
    }

    pub fn vulnerable(&self) -> bool {
        self.damage_invuln_timer <= 0.0
    }

    /// Decay every body reaction timer by caller-supplied `dt`, clamping at zero.
    /// The caller owns the clock domain used for `dt`.
    pub fn decay_reaction_timers(&mut self, dt: f32) {
        self.damage_invuln_timer = (self.damage_invuln_timer - dt).max(0.0);
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        self.hitstun_timer = (self.hitstun_timer - dt).max(0.0);
        self.recoil_lock_timer = (self.recoil_lock_timer - dt).max(0.0);
        self.sleep_timer = (self.sleep_timer - dt).max(0.0);
        self.hitstop_timer = (self.hitstop_timer - dt).max(0.0);
        self.landing_lag_timer = (self.landing_lag_timer - dt).max(0.0);
    }

    /// Clear reaction timers while preserving construction-owned `training_dummy`.
    ///
    /// `armored` is NOT cleared here and must not be: it is republished from the
    /// live move every tick, so clearing it would be undone within the frame and
    /// would read as a rule this function does not own.
    pub fn reset(&mut self) {
        self.hit_flash = 0.0;
        self.hitstop_timer = 0.0;
        self.asdi_owed = false;
        self.damage_invuln_timer = 0.0;
        self.hitstun_timer = 0.0;
        self.recoil_lock_timer = 0.0;
        self.landing_lag_timer = 0.0;
        // A fighter who respawns still asleep from the stock before is helpless
        // on arrival with nothing on screen explaining why.
        self.sleep_timer = 0.0;
    }
}

/// Authoritative body-generic animation facts.
/// These fields are presentation-only and independent of gameplay reaction timers.
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
    pub anim_prev_dashing: bool,
    /// Time remaining for the projectile-release `Shoot` pose. Armed by
    /// `charge_projectile_input` whenever a projectile request is accepted (any
    /// kind — Fireball/Hadouken/HadoukenSuper). Single-shot, short.
    pub shoot_anim_timer: f32,
    /// Set each frame by `charge_projectile_input` to mirror
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
    /// Whether this body is currently held, re-derived each frame from the capture relation.
    /// The sim publishes this fact so presentation does not query combat relations directly.
    pub held: bool,
    /// Whether this body currently holds another body.
    pub holding: bool,
    /// Time remaining on the death pose, which may outlive the liveness transition itself.
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

    /// Shared reaction decay advances every timer and clamps at zero.
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

#[cfg(test)]
mod hard_lock_tests {
    use super::*;

    /// Exhaustive destructuring forces every `BodyCombat` field to declare decay ownership.
    #[allow(dead_code)]
    fn every_timer_declares_whether_the_shared_decay_ticks_it(combat: &BodyCombat) {
        let BodyCombat {
            // Decayed by `decay_reaction_timers`.
            damage_invuln_timer: _,
            hit_flash: _,
            hitstun_timer: _,
            recoil_lock_timer: _,
            // Ticked with the rest: a sleep runs down on the same clock as
            // every other reaction, so nothing has to remember it separately.
            sleep_timer: _,
            hitstop_timer: _,
            landing_lag_timer: _,

            // Not a timer: it is a WINDOW the live move republishes every tick,
            // so it expires by being rewritten rather than by counting down.
            armored: _,

            // Not a timer: a LATCH, banked while the freeze runs and spent by
            // the body step on the far side of it. Decaying it would spend the
            // displacement on whichever system happened to run first.
            asdi_owed: _,

            // Not a timer.
            training_dummy: _,
        } = combat;
    }

    /// Exhaustive destructuring forces every `BodyCombat` field to declare reset ownership.
    #[allow(dead_code)]
    fn every_field_declares_whether_reset_clears_it(combat: &BodyCombat) {
        let BodyCombat {
            // Cleared by `reset()`.
            hit_flash: _,
            hitstop_timer: _,
            damage_invuln_timer: _,
            hitstun_timer: _,
            recoil_lock_timer: _,
            landing_lag_timer: _,
            // ⭐ CLEARED, and it must be: a fighter who respawns still asleep
            // from the stock before is helpless on arrival with nothing on
            // screen explaining why.
            sleep_timer: _,

            // NOT cleared, deliberately: `project_move_defense_windows`
            // republishes it from the live move every tick, so clearing it here
            // would be undone inside the frame and would read as a rule this
            // function does not own.
            armored: _,

            // Cleared by `reset()`: a body put back to spawn owes nothing from
            // a freeze that is no longer happening.
            asdi_owed: _,

            // Construction-owned; reset does not change body identity.
            training_dummy: _,
        } = combat;
    }

    /// Hitlag is body state independent of controller kind.
    #[test]
    fn hitlag_is_a_body_question_and_both_roads_ask_it() {
        let mut combat = BodyCombat::default();
        assert!(
            !combat.is_in_hitlag(),
            "a body with no hitstop reports hitlag, so the assertion below cannot \
             tell a freeze from a default"
        );
        combat.hitstop_timer = 0.07;
        assert!(
            combat.is_in_hitlag(),
            "a body carrying hitstop does not report hitlag — the predicate stopped \
             reading the timer the damage path arms"
        );
    }

    /// the expression was `recoil_lock.max(landing_lag)` written at the call
    /// site, so reducing it to `recoil_lock` alone would have stopped landing lag
    /// locking anything and no test would have said so. That was not
    /// hypothetical: the ACTOR road passed exactly that reduced form.
    ///
    /// it cannot any more, and the type is why —
    /// `engine_input_from_actor_control` takes `&BodyCombat` instead of two
    /// loose `f32`s, so there is no parameter a caller can fill with the wrong
    /// field. This test guards the remaining half: that the method itself keeps
    /// asking for both.
    #[test]
    fn landing_lag_alone_still_locks_control() {
        let mut combat = BodyCombat::default();
        assert_eq!(
            combat.hard_lock_timer(),
            0.0,
            "a body with no stagger reports a lock, so the assertions below \
             cannot tell a lock from a default"
        );

        combat.landing_lag_timer = 0.28;
        assert_eq!(
            combat.hard_lock_timer(),
            0.28,
            "landing lag alone does not lock control — an aerial's authored cost \
             is being dropped, which is precisely what the actor road does"
        );

        // The larger of the two wins, both ways round.
        combat.recoil_lock_timer = 0.40;
        assert_eq!(combat.hard_lock_timer(), 0.40);
        combat.landing_lag_timer = 0.55;
        assert_eq!(combat.hard_lock_timer(), 0.55);

        // hitstun is a DIFFERENT gate — it reduces authority rather than
        // removing it, and folding it in here would silently harden it.
        combat.recoil_lock_timer = 0.0;
        combat.landing_lag_timer = 0.0;
        combat.hitstun_timer = 1.0;
        assert_eq!(
            combat.hard_lock_timer(),
            0.0,
            "hitstun became a HARD lock; it is supposed to leave reduced movement \
             authority, which is the distinction the input gate keeps"
        );
    }
}
