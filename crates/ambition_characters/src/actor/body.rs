//! Shared **body vocabulary** components — the health, combat-status, and wallet
//! every actor carries (the player, enemies, NPCs, and bosses alike).
//!
//! These re-homed down from `ambition_platformer2d_actor_monolith::actor` (unified-actors
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
        if self.health.invulnerable.any() || amount <= 0 {
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
/// The field sets were disjoint when they merged, and the union preserved both
/// vocabularies. AC3 is unpicking that: three of the actor-side status fields
/// (`strike_count`, `attack_windup_timer`, `attack_timer`) turned out to be
/// maintained and rewound for no reader at all and are gone, and the reaction
/// timers now have ONE decay and ONE reset for every body rather than a list per
/// actor family. `hit_flash` is the ONE damage-blink field, shared by both.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyCombat {
    /// Presentation flash (damage hit-blink) — the one field for every body,
    /// decayed for every body by [`Self::decay_reaction_timers`].
    pub hit_flash: f32,
    // ── Player reaction / control-lock timers ──
    /// Hitstop: freezes `time_scale` to 0 while positive.
    pub hitstop_timer: f32,
    /// **Landing lag: the authored recovery an aerial move owes for touching
    /// down before it finished.**
    ///
    /// A HARD control lock while positive, exactly like
    /// [`Self::recoil_lock_timer`] — and deliberately NOT that field. They are
    /// two different facts that happen to have the same effect: one says *you
    /// were just thrown*, the other says *you landed out of a move you had not
    /// finished*. Sharing a field would make the two indistinguishable in a
    /// trace and in F1, and reading one as the other is the defect class this
    /// campaign has been deleting all week.
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
    // ── Actor status / attack-timeline presentation ──
    pub training_dummy: bool,
}

// ⭐⭐ **THE `peaceful` / `hostile` CONSTRUCTORS ARE GONE** (AC3.1.A). They were
// the last thing making a body's DISPOSITION look like a fact about its combat
// STATE. Once AC3 removed the dead attack timeline, the liveness mirror and the
// melee mirror, the two differed by a single authored boolean — so a caller that
// wants a fresh body writes the struct and a caller that wants to update one
// writes the field. Neither needs to know which side the body is on, because
// `BodyCombat` no longer records it.
impl BodyCombat {
    /// **THE HARD CONTROL LOCK THIS FRAME, whichever fact produced it** — no
    /// steering authority at all while it is positive.
    ///
    /// ⭐ **named because two roads have to agree on it and one of them does
    /// not** (ledger D108). The player road already computes
    /// `recoil_lock.max(landing_lag)` inline; the ACTOR road passes only
    /// `(hitstun, recoil_lock)`, so a CPU lands clean out of an aerial that
    /// costs a human up to 0.28s. Two spellings of one rule is how they
    /// diverged, and one spelling is how they stop.
    ///
    /// ⚠ **this does NOT fix D108 by existing.** The actor road still does not
    /// call it — that is a difficulty decision, because applying it makes every
    /// CPU fighter commit to its aerials. What it does is make the fix a
    /// one-line change at the call site instead of a re-derivation, and make the
    /// divergence visible: one road calls this, the other spells half of it.
    ///
    /// ⛔ `hitstun_timer` is deliberately NOT part of it. Hitstun REDUCES
    /// movement authority; this is the set of facts that remove it entirely, and
    /// merging the two is the distinction `apply_post_hit_input_gates` exists to
    /// keep.
    pub fn hard_lock_timer(&self) -> f32 {
        self.recoil_lock_timer.max(self.landing_lag_timer)
    }

    /// **IS THIS BODY IN HITLAG** — the shared freeze a landed hit puts on BOTH
    /// parties, which the damage path states as *"a landed hit is one event"*.
    ///
    /// ⭐ **named for the same reason as [`Self::hard_lock_timer`], and BOTH
    /// movement roads now ask it** (ledger D114, closed 2026-08-17). The timer
    /// is armed on the victim AND the attacker, whoever they are; the avatar
    /// road (`integrate_player_body`) and the actor road (`integrate_body`)
    /// each take `sim_dt = 0` off this one predicate.
    ///
    /// ⛔ **what it was before, because the shape recurs.** Only the avatar road
    /// branched, so a hit whose two parties were BOTH actors froze neither of
    /// them — on a platform-fighter stage that is every CPU-versus-CPU exchange
    /// and every seat past slot 0. Factoring the predicate out first is what
    /// made closing it a call rather than a re-derivation.
    ///
    /// ⚠ **a `With<PrimaryPlayer>` clock request is still slot-0 only.** That is
    /// the PRESENTATION freeze (the whole screen hitching), a different question
    /// from whether a struck body advances, and it remains open.
    pub fn is_in_hitlag(&self) -> bool {
        self.hitstop_timer > 0.0
    }

    pub fn vulnerable(&self) -> bool {
        self.damage_invuln_timer <= 0.0
    }

    /// **THE decay for every body's reaction timers** — the post-hit i-frame
    /// window, the damage-blink the renderer reads, the §A2 stagger set
    /// (hitstun / recoil-lock / hitstop), and the landing lag an aerial owes.
    /// Each clamps at zero.
    ///
    /// ⛔⛔ **it is THE decay now, and it was not before** (AC3.3, closing
    /// D108's decay site). This method was written to retire two hand-copied
    /// decay blocks — and a THIRD survived on the player road, spelling its own
    /// five lines. The two lists disagreed in BOTH directions: the shared one
    /// decayed `hit_flash` and forgot `landing_lag_timer`; the player's decayed
    /// `landing_lag_timer` and forgot `hit_flash`, which a separate
    /// home-avatar-only system did instead. So a CPU kept its landing lag
    /// forever, and a co-op/clone body — a `PlayerEntity` that is not the
    /// `PrimaryPlayer` — kept its damage blink forever, because the system that
    /// decayed the blink queried only the home avatar.
    ///
    /// ⚠ **the caller supplies its own `dt` deliberately.** The actor and boss
    /// ticks pass the SIM delta; the player road passes the frame delta. That
    /// difference is a time-domain question this method does not decide, and
    /// collapsing it here would change feel while pretending to consolidate a
    /// list.
    pub fn decay_reaction_timers(&mut self, dt: f32) {
        self.damage_invuln_timer = (self.damage_invuln_timer - dt).max(0.0);
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        self.hitstun_timer = (self.hitstun_timer - dt).max(0.0);
        self.recoil_lock_timer = (self.recoil_lock_timer - dt).max(0.0);
        self.hitstop_timer = (self.hitstop_timer - dt).max(0.0);
        self.landing_lag_timer = (self.landing_lag_timer - dt).max(0.0);
    }

    /// Reset every reaction timer a body reset clears.
    ///
    /// ⛔ **it said the remaining fields "are owned by the per-frame sync from
    /// the cluster"**, which stopped being true in AC3.2 —
    /// `sync_actor_components_from_cluster` writes no `BodyCombat` field now —
    /// and stopped being possible in AC6.2, where the seed carries this
    /// component. The one non-timer field, `training_dummy`, is the character's
    /// `practice_target` written once at construction: a reset restores a body's
    /// reaction history, it does not re-decide what the body IS.
    ///
    /// ⛔ **`landing_lag_timer` is in it now** — D108's fourth site. It cleared
    /// six fields and not that one, so a body reset mid-landing-lag kept up to
    /// 0.28s of input lock through the reset. Three production callers reach
    /// this (`sandbox_reset`, `features::ecs::reset`, `session::reset`); the
    /// room-transition and lifecycle-commit paths cleared it by hand, which is
    /// why the symptom stayed hidden.
    pub fn reset(&mut self) {
        self.hit_flash = 0.0;
        self.hitstop_timer = 0.0;
        self.damage_invuln_timer = 0.0;
        self.hitstun_timer = 0.0;
        self.recoil_lock_timer = 0.0;
        self.landing_lag_timer = 0.0;
    }

}

/// A body's ECS-owned animation signal timers.
///
/// **Body vocabulary, not player-only** — despite the anim rows it gates being
/// authored on the player's sheet first, every brain-driven body that plays a
/// slash, a landing, or a dash pre-roll carries one. It re-homed here from
/// `ambition_platformer2d_actor_monolith::avatar::components` (the S5/S6 player fold, refactor-chain
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

#[cfg(test)]
mod hard_lock_tests {
    use super::*;

    /// **EVERY REACTION TIMER SAYS WHETHER [`BodyCombat::decay_reaction_timers`]
    /// TICKS IT.**
    ///
    /// ⭐ **this guard did its job, and AC3 collected.** It was added because the
    /// consolidation that produced `decay_reaction_timers` solved the
    /// DUPLICATION and not the ROT — `landing_lag_timer` joined the struct later
    /// and never joined the list, and a comment cannot fail. A destructure can:
    /// adding a timer to `BodyCombat` is a compile error here until somebody
    /// says whether it decays.
    ///
    /// It now records a list with nothing left in the "should be and is not"
    /// bucket. Keep it that way by answering the compile error rather than
    /// adding a field to the bottom group.
    #[allow(dead_code)]
    fn every_timer_declares_whether_the_shared_decay_ticks_it(combat: &BodyCombat) {
        let BodyCombat {
            // ── DECAYED by `decay_reaction_timers` (6) ─────────────────────
            damage_invuln_timer: _,
            hit_flash: _,
            hitstun_timer: _,
            recoil_lock_timer: _,
            hitstop_timer: _,
            // Joined the list in AC3.3. It is set by the moveset runtime on any
            // body that lands mid-move, and before AC3 only the player road
            // decremented it — so a CPU never paid the landing lag its own
            // authored aerial owed.
            landing_lag_timer: _,

            // ── NOT A TIMER — nothing to decay ─────────────────────────────
            training_dummy: _,
        } = combat;
    }

    /// **LANDING LAG IS PART OF THE HARD LOCK, NOT ONLY RECOIL** — ledger D108,
    /// and the assertion the player road's inline expression never had.
    ///
    /// **AND `reset()` WAS THE FOURTH LIST WITH THE SAME OMISSION.** It cleared
    /// six fields and not `landing_lag_timer`, so a body reset while
    /// mid-landing-lag kept the lock — for the PLAYER, whose road actually reads
    /// the timer, up to 0.28s of input lock carried through a reset. AC3.3
    /// closed it; this destructure is what will notice the next omission.
    #[allow(dead_code)]
    fn every_field_declares_whether_reset_clears_it(combat: &BodyCombat) {
        let BodyCombat {
            // ── CLEARED by `reset()` (6) ───────────────────────────────────
            hit_flash: _,
            hitstop_timer: _,
            damage_invuln_timer: _,
            hitstun_timer: _,
            recoil_lock_timer: _,
            landing_lag_timer: _,

            // ── NOT A TIMER, and CONSTRUCTION owns it ──────────────────────
            //
            // ⛔ **this said "rebuilt by the per-frame sync from the cluster"
            // and that has been false twice over.** AC3.2 deleted the sync's
            // rebuild — `sync_actor_components_from_cluster` is one string
            // comparison and writes no `BodyCombat` field at all — and AC6.2
            // made the seed carry this component, so the flag is the character's
            // `practice_target` written ONCE at construction. A reset restores a
            // body's reaction history; it does not re-decide what the body IS.
            training_dummy: _,
        } = combat;
    }

    /// **AND HITLAG IS THE SAME SHAPE ONE LAYER OVER** — D114.
    ///
    /// ⛔ the freeze is armed on the victim AND the attacker, from a law the
    /// damage path states as *"a landed hit is one event"*. It USED to be read by
    /// the avatar road alone, so a hit between two bodies that are neither
    /// produced no freeze — CPU versus CPU on a platform-fighter stage, which is
    /// what Smash is made of. Both roads take the branch as of 2026-08-17; this
    /// pins the predicate they share, and
    /// `a_hit_between_two_actors_freezes_them_both` pins that the actor road
    /// actually spends it.
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

    /// ⛔ the expression was `recoil_lock.max(landing_lag)` written at the call
    /// site, so reducing it to `recoil_lock` alone would have stopped landing lag
    /// locking anything and no test would have said so. That was not
    /// hypothetical: the ACTOR road passed exactly that reduced form.
    ///
    /// ⭐ **it cannot any more, and the type is why** —
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

        // ⛔ hitstun is a DIFFERENT gate — it reduces authority rather than
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
