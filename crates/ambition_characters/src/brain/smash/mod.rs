//! Smash-brawl brain template — SSBB Subspace-Emissary feel.
//!
//! Each tick the brain runs a 5-stage pipeline:
//!
//! 1. **Observe**: snapshot the world into an [`ObservationFrame`]
//!    (self + target + stage + crowding + hazards).
//! 2. **Choose broad mode**: pick a [`BroadMode`] (Approach / Retreat
//!    / Engage / Reposition / Recover / Idle) with hysteresis so the
//!    actor doesn't oscillate.
//! 3. **Choose specific action**: pick a [`SpecificAction`] from the
//!    mode's allowed vocabulary, gated by the actor's [`ActionSet`]
//!    capability mask.
//! 4. **Apply difficulty filter**: reaction delay, commit
//!    probability, aim accuracy. Easier enemies "see late" and drop
//!    actions; harder enemies commit + aim cleanly.
//! 5. **Emit inputs**: translate the action into an
//!    [`crate::actor::control::ActorControlFrame`] the integration pipeline consumes.
//!
//! Every stage is a pure function of the previous one's output plus
//! [`SmashCfg`] / [`SmashState`]. This makes the pipeline trivially
//! unit-testable and keeps the brain backend swappable — a future
//! RL policy can replace any single stage without touching the
//! others.

use super::action_set::ActionSet;
use super::snapshot::BrainSnapshot;
// `ae` is used both by `maybe_substitute_ranged` (the ranged-verb emit) and the
// tests, so the import is no longer test-gated.
use ambition_engine_core as ae;

pub mod action;
pub mod difficulty;
pub mod emit;
pub mod mode;
pub mod observation;

#[cfg(test)]
mod arena;

pub use action::{choose_action, SpecificAction};
pub use difficulty::{apply_difficulty, DifficultyProfile};
pub use emit::emit_inputs;
pub use mode::{choose_mode, BroadMode};
pub use observation::{observe, CrowdingSignal, ObservationFrame, TerrainAwareness};

/// Tuning knobs for a [`StateMachineCfg::Smash`] brain. Per-actor
/// state lives in [`SmashState`]. Designer-facing today — eventually
/// migrates to data so per-archetype variants live in
/// `character_archetypes.ron`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmashCfg {
    /// Maximum sensing distance (px). Outside this radius the brain
    /// idles regardless of target presence.
    pub aggro_radius: f32,
    /// Distance the brain tries to settle at while in `Engage`.
    /// Slightly outside `attack_range` so the actor has room to
    /// burst forward into an attack.
    pub engage_distance: f32,
    /// Concrete melee attack range (px). When the target is closer
    /// than this AND the actor has melee capability, `Engage` emits
    /// a melee attempt. Authoritative — replaces the old hardcoded
    /// melee-engage range.
    pub attack_range: f32,
    /// Distance below which the actor retreats to avoid being
    /// pinned against a wall by the target.
    pub too_close_distance: f32,
    /// Minimum *upward* gap (px) to the target before the actor jumps to
    /// pursue it vertically. The grunt default (`60`) chases any target a
    /// short hop above; a duelist sets this **above a hop's apex** so it
    /// only climbs after a target genuinely standing on a platform, instead
    /// of leapfrogging an opponent that is merely mid-hop (the flat-ground
    /// air-juggle cascade). Replaces the former hardcoded threshold.
    pub vertical_chase_min: f32,
    /// Movement speed while in Approach / Chase (px/s).
    pub chase_speed: f32,
    /// Movement speed while in Retreat / Reposition (px/s).
    pub retreat_speed: f32,
    /// Crowding pressure (from same-faction allies) that triggers
    /// `Reposition` mode. `0.0` disables.
    pub crowding_threshold: f32,
    /// When true, the actor bursts a [`SpecificAction::Dash`] to close
    /// a *large* approach gap (on a cooldown) instead of only walking —
    /// a more aggressive, dynamic chase. Off by default; enabled per
    /// archetype (goblins) so it doesn't silently change every melee
    /// enemy's feel.
    pub dash_to_close: bool,
    /// Neutral-game footsies: amplitude (px) the actor weaves IN and OUT
    /// around [`Self::engage_distance`] while spacing against a live
    /// opponent. `0.0` (the grunt default) disables the weave entirely —
    /// the actor closes and holds like before. A positive value makes a
    /// *duelist*: it dips into poke range on a rhythm, then backs out to
    /// bait a whiff, instead of camping point-blank and mashing. Uses only
    /// the target-relative distance, so it's frame-agnostic.
    pub footsies_amplitude: f32,
    /// Seconds per full in→out footsies cycle. Ignored when
    /// [`Self::footsies_amplitude`] is `0.0`.
    pub footsies_period_s: f32,
    /// Minimum seconds between *neutral hops* — short jumps the duelist
    /// mixes into its approach to vary its attack vector and use vertical
    /// space. `0.0` (the grunt default) disables neutral hops.
    pub neutral_jump_cadence_s: f32,
    /// **Regroup** trigger: accumulated recent damage (as a fraction of max HP,
    /// decaying over a couple seconds) that makes the fighter break off and reset
    /// after taking a beating — instead of trading hits forever at one spacing. It
    /// retreats a real distance (dashing to cover ground, taking to the air for high
    /// ground if it can fly), then re-engages. `0.0` (the grunt default) disables it.
    pub regroup_damage_threshold: f32,
    /// How long a regroup lasts (s) before the fighter returns to neutral. Ignored
    /// when [`Self::regroup_damage_threshold`] is `0.0`.
    pub regroup_duration_s: f32,
    /// Target separation (px) a regroup opens up: once the fighter has backed off at
    /// least this far it has "regrouped" and re-engages early. Large enough that the
    /// retreat — and the re-approach — cross a real gap (so the dash/fly traversal
    /// actually fires). Ignored when regroup is disabled.
    pub regroup_distance: f32,
    /// **Poke-and-reset discipline** (whiff-punish footsies). Seconds the fighter
    /// suppresses offense after a melee swing completes — letting the neutral weave
    /// reset its spacing — before it may re-commit. `0.0` (the grunt default)
    /// disables it: the actor stays in range and re-swings the instant its cooldown
    /// clears (point-blank mashing). A positive value makes a real neutral game —
    /// poke, reset, re-approach — instead of two bodies glued together trading
    /// hits. Frame-agnostic (uses only target-relative spacing); the in/out weave,
    /// not a forced retreat, does the spacing so a cornered fighter never pins
    /// itself against a wall.
    pub poke_reset_s: f32,
    /// When true, the fighter may **blink-evade** a fast-closing opponent (a
    /// perceivable lunge, read from the lagged target history — never from a
    /// privileged attack flag). Capability gate only: the body still needs the
    /// blink ability for the emitted intent to resolve, exactly like the player.
    /// `false` for grunts.
    pub can_blink: bool,
    /// Minimum seconds between blink-evades. Ignored when [`Self::can_blink`]
    /// is `false`.
    pub blink_cooldown_s: f32,
    /// `[0, 1]` — probability the fighter actually reacts to a perceived threat
    /// this tick (blink or block). `< 1.0` models imperfect defense: it does NOT
    /// block every swing, so some attacks land and the fight never turtles into a
    /// stalemate. `0.0` (the grunt default) disables reactive defense entirely.
    /// This is the "no perfect reactions" knob layered on top of `reaction_delay_s`
    /// (which already makes it perceive the lunge late).
    pub defense_reactivity: f32,
    /// Perceived closing speed (px/s, toward the fighter) at or above which a
    /// threat is met with a **blink** (the mobile evade for a committed lunge /
    /// dash-in). Below it — but above [`Self::shield_closing_speed`] — the fighter
    /// **blocks** instead (the stand-ground option for ordinary approach pressure).
    /// Splitting the two is what gives the layered, readable defensive game.
    pub blink_closing_speed: f32,
    /// Minimum perceived closing speed (px/s) that counts as a threat worth
    /// reacting to at all. Above a slow drift, below a walk-in so the fighter
    /// guards an opponent stepping into poke range.
    pub shield_closing_speed: f32,
    /// When true, the (grounded) fighter may **reactive-block** a perceived lunge
    /// it can't or won't blink away from — it raises `shield_held` and stands its
    /// ground for a short window. Layered defense: blink is the mobile option,
    /// block the stand-ground one. `false` for grunts.
    pub can_shield: bool,
    /// When true, this is a **hybrid flyer**: a body that can both fight grounded
    /// (footsies + jump) and take flight (`fly_toggle_pressed`). The brain decides
    /// when to be airborne — to contest an elevated target, or to mount a proactive
    /// aerial foray — and lands again to footsie. `false` = the body never toggles
    /// (a pure grounded brawler, or a pure flyer driven by its `actor_aerial`
    /// body state). Capability gate: the body still needs the fly ability for the
    /// toggle intent to resolve, like the player.
    pub can_fly: bool,
    /// Hybrid flyer: seconds spent grounded between proactive aerial forays.
    /// Ignored unless [`Self::can_fly`].
    pub aerial_foray_cadence_s: f32,
    /// Hybrid flyer: seconds an aerial foray lasts before landing again.
    /// Ignored unless [`Self::can_fly`].
    pub aerial_foray_duration_s: f32,
    /// **Relentless** engagement: when true, the fighter never disengages while its
    /// foe lives — beyond [`Self::aggro_radius`] it CHASES (Approach) instead of
    /// idling out. This is the committed-duelist property: a platform-fighter
    /// opponent pursues across the whole stage and re-acquires after the player
    /// flings it away with gravity, rather than going inert at distance. `false`
    /// (the grunt default) keeps ambient enemies idling once the player leaves.
    pub relentless: bool,
    /// **Stale-fight re-aggression**: seconds of the fighter's own offense-drought
    /// (no swing / shot committed) after which it forces an offensive push —
    /// suppressing its reactive defense and neutral-game patience to close and
    /// attack, the way two platform-fighter players break a passive standoff. Resets
    /// whenever it attacks, so it only fires during a genuine lull, never mid-trade.
    /// `0.0` (the grunt default) disables it.
    pub stale_fight_s: f32,
    /// Difficulty profile applied at stage 4.
    pub difficulty: DifficultyProfile,
}

impl SmashCfg {
    /// "Standard melee striker" tuning — humanoid grunt that
    /// approaches, swings, and steps back. Used by MediumStriker,
    /// SmallSkitter, SmallLurker, PirateRaider.
    pub const STRIKER_DEFAULT: Self = Self {
        aggro_radius: 460.0,
        engage_distance: 70.0,
        attack_range: 56.0,
        too_close_distance: 30.0,
        vertical_chase_min: 60.0,
        chase_speed: 170.0,
        retreat_speed: 130.0,
        crowding_threshold: 0.65,
        dash_to_close: false,
        // Grunts don't play footsies — they close and hold. The neutral game
        // is opt-in (duelists / bosses) so this doesn't change every enemy.
        footsies_amplitude: 0.0,
        footsies_period_s: 1.4,
        neutral_jump_cadence_s: 0.0,
        regroup_damage_threshold: 0.0,
        regroup_duration_s: 0.0,
        regroup_distance: 0.0,
        poke_reset_s: 0.0,
        can_blink: false,
        blink_cooldown_s: 0.0,
        defense_reactivity: 0.0,
        blink_closing_speed: 175.0,
        shield_closing_speed: 175.0,
        can_shield: false,
        can_fly: false,
        aerial_foray_cadence_s: 0.0,
        aerial_foray_duration_s: 0.0,
        // Ambient grunt: idles out when the player leaves; no stale-fight push.
        relentless: false,
        stale_fight_s: 0.0,
        difficulty: DifficultyProfile::MEDIUM,
    };
    /// Heavy brute tuning — slower, longer reach, less retreat.
    pub const BRUTE_DEFAULT: Self = Self {
        aggro_radius: 380.0,
        engage_distance: 90.0,
        attack_range: 70.0,
        too_close_distance: 24.0,
        vertical_chase_min: 60.0,
        chase_speed: 118.0,
        retreat_speed: 80.0,
        crowding_threshold: 0.55,
        dash_to_close: false,
        footsies_amplitude: 0.0,
        footsies_period_s: 1.6,
        neutral_jump_cadence_s: 0.0,
        regroup_damage_threshold: 0.0,
        regroup_duration_s: 0.0,
        regroup_distance: 0.0,
        poke_reset_s: 0.0,
        can_blink: false,
        blink_cooldown_s: 0.0,
        defense_reactivity: 0.0,
        blink_closing_speed: 175.0,
        shield_closing_speed: 175.0,
        can_shield: false,
        can_fly: false,
        aerial_foray_cadence_s: 0.0,
        aerial_foray_duration_s: 0.0,
        relentless: false,
        stale_fight_s: 0.0,
        difficulty: DifficultyProfile::MEDIUM,
    };
    /// **Duelist** tuning — a 1v1 fighter with a real neutral game: it weaves
    /// in and out of poke range (footsies), mixes in neutral hops, and dashes
    /// to close large gaps. Aware of the whole arena (large aggro). This is the
    /// base the Perfect Cell-ular Automaton and other "platform fighter"
    /// opponents build on; grunts stay on [`Self::STRIKER_DEFAULT`].
    pub const DUELIST_DEFAULT: Self = Self {
        aggro_radius: 1100.0,
        engage_distance: 78.0,
        attack_range: 56.0,
        too_close_distance: 30.0,
        vertical_chase_min: 140.0,
        chase_speed: 200.0,
        retreat_speed: 175.0,
        crowding_threshold: 0.65,
        dash_to_close: true,
        footsies_amplitude: 60.0,
        footsies_period_s: 1.3,
        neutral_jump_cadence_s: 1.7,
        // After taking ~5% of max HP since the last break-off (a few clean hits),
        // regroup: dash/fly out to a real distance, then re-engage — spatial depth
        // instead of a glued trade.
        regroup_damage_threshold: 0.05,
        regroup_duration_s: 1.6,
        regroup_distance: 300.0,
        // After every poke, suppress offense and let the weave reset spacing before
        // re-committing — the heart of the neutral game (no point-blank mashing).
        poke_reset_s: 0.38,
        can_blink: true,
        blink_cooldown_s: 1.1,
        // A real defensive game: react to ~60% of perceived threats (imperfect —
        // some hits land), blink a committed lunge (≥230 px/s closing), block the
        // ordinary walk-in pressure (≥70 px/s).
        defense_reactivity: 0.6,
        blink_closing_speed: 230.0,
        shield_closing_speed: 70.0,
        can_shield: true,
        // Grounded duelist by default; hybrid flight is opt-in per fighter.
        can_fly: false,
        aerial_foray_cadence_s: 0.0,
        aerial_foray_duration_s: 0.0,
        // A committed 1v1 fighter: chases its foe across the whole stage (never idles
        // out at distance) and, after ~2.5 s of its own inaction, forces an offensive
        // push so the bout never stalls into a passive standoff.
        relentless: true,
        stale_fight_s: 2.5,
        // A fair human reaction lag (no frame-perfect counters). Competence is
        // expressed through the neutral game + layered defense above, NOT through
        // crisper reactions: a twitchier profile locks the fight into a shielding
        // standoff (the arena non-degeneracy harness catches it).
        difficulty: DifficultyProfile::MEDIUM,
    };
}

/// Number of opponent-position samples retained for reaction latency.
/// At 60 fps this is ~0.53 s of history — comfortably longer than any
/// authored `reaction_delay_s` (EASY = 0.30 s), so the delayed lookup is
/// always covered once the buffer fills.
pub const OBS_HISTORY_LEN: usize = 32;

/// Ring buffer of recent opponent observations `(sim_time, target_pos)`,
/// used to apply **reaction latency**: each tick the brain perceives the
/// opponent as it was `reaction_delay_s` ago, so it can't frame-perfectly
/// counter a sudden move. The actor's OWN state is never delayed (you always
/// know where you are). Pure function of the tick stream → replay-safe and
/// deterministic. `Copy` so `SmashState` stays `Copy`.
#[derive(Clone, Copy, Debug)]
pub struct ObsHistory {
    samples: [(f32, ae::Vec2); OBS_HISTORY_LEN],
    /// Next write index (ring).
    write: usize,
    /// Number of valid samples (saturates at `OBS_HISTORY_LEN`).
    count: usize,
}

impl Default for ObsHistory {
    fn default() -> Self {
        Self {
            samples: [(0.0, ae::Vec2::ZERO); OBS_HISTORY_LEN],
            write: 0,
            count: 0,
        }
    }
}

impl ObsHistory {
    /// Record this tick's observed opponent position.
    fn push(&mut self, sim_time: f32, target_pos: ae::Vec2) {
        self.samples[self.write] = (sim_time, target_pos);
        self.write = (self.write + 1) % OBS_HISTORY_LEN;
        self.count = (self.count + 1).min(OBS_HISTORY_LEN);
    }

    /// Stable snapshot view of the reaction-history ring. The brain crate owns the
    /// representation; rollback consumers receive the exact values without exposing
    /// the fields for arbitrary mutation.
    pub fn snapshot_parts(&self) -> (&[(f32, ae::Vec2); OBS_HISTORY_LEN], usize, usize) {
        (&self.samples, self.write, self.count)
    }

    /// Restore the reaction-history ring from a validated snapshot cursor.
    pub fn restore_snapshot_parts(
        &mut self,
        samples: [(f32, ae::Vec2); OBS_HISTORY_LEN],
        write: usize,
        count: usize,
    ) -> Option<()> {
        if write >= OBS_HISTORY_LEN || count > OBS_HISTORY_LEN {
            return None;
        }
        self.samples = samples;
        self.write = write;
        self.count = count;
        Some(())
    }

    /// The opponent position the brain is allowed to perceive this tick: the
    /// most recent sample that is at least `delay` seconds old. Never returns
    /// anything newer than `now - delay`, so the brain truly cannot react
    /// faster than its latency. Until the buffer covers the window (fight
    /// start), returns the oldest sample it has; `None` only when no sample has
    /// been recorded yet.
    fn delayed(&self, now: f32, delay: f32) -> Option<ae::Vec2> {
        if self.count == 0 {
            return None;
        }
        let target_time = now - delay.max(0.0);
        let mut best_old: Option<(f32, ae::Vec2)> = None; // newest sample <= target_time
        let mut oldest: Option<(f32, ae::Vec2)> = None;
        for i in 0..self.count {
            let s = self.samples[i];
            if oldest.is_none_or(|o| s.0 < o.0) {
                oldest = Some(s);
            }
            if s.0 <= target_time && best_old.is_none_or(|b| s.0 > b.0) {
                best_old = Some(s);
            }
        }
        Some(best_old.or(oldest).map(|s| s.1).unwrap_or(ae::Vec2::ZERO))
    }
}

/// Per-actor runtime state for the Smash brain.
#[derive(Clone, Copy, Debug, Default)]
pub struct SmashState {
    /// Mode active last tick. Used by the hysteresis check in
    /// `choose_mode` so the brain doesn't flip Approach⇄Retreat
    /// when distance hovers at the threshold.
    pub mode: BroadMode,
    /// Seconds the current mode has been active. Incremented each
    /// tick from `snapshot.dt`; reset to 0 on mode change. Compared
    /// against `MODE_MIN_DWELL_S` for hysteresis.
    pub mode_dwell_s: f32,
    /// Random seed for difficulty jitter (commit probability,
    /// reaction delay variance). Set once at first tick from the
    /// actor id; survives reset_to_spawn via spawn-time init.
    pub rng_seed: u64,
    /// Seconds until the actor's *dash-to-close* burst is off cooldown
    /// (only used when [`SmashCfg::dash_to_close`]). Same brain-side
    /// cadence shape as the ranged cooldown: decremented each tick,
    /// gated against `> 0`, reset to [`DASH_COOLDOWN_S`] on a dash.
    pub dash_cooldown_remaining: f32,
    /// Rolling history of the opponent's recent positions, used to apply
    /// the difficulty profile's `reaction_delay_s` (the brain perceives a
    /// lagged opponent — it never reacts frame-perfectly). See [`ObsHistory`].
    pub obs_history: ObsHistory,
    /// Footsies weave phase (radians), advanced each tick when the cfg enables
    /// the neutral game. A per-actor offset (from `rng_seed`) is added at read
    /// time so two duelists desync rather than mirror-lock.
    pub spacing_phase: f32,
    /// Seconds until the next neutral hop is allowed. Decremented each tick;
    /// re-armed to `neutral_jump_cadence_s` when a hop fires.
    pub neutral_jump_cooldown: f32,
    /// Seconds until the next blink-evade is allowed. Decremented each tick;
    /// re-armed to `blink_cooldown_s` when a blink fires.
    pub blink_cooldown: f32,
    /// Hybrid flyer: seconds left in the current ground-dwell or aerial-foray
    /// phase. Drives the proactive take-off/land cadence with hysteresis so the
    /// fighter doesn't chatter the fly toggle every tick.
    pub foray_timer: f32,
    /// Seconds left holding a reactive block. Set when the fighter chooses to
    /// shield a perceived lunge; while positive it keeps `shield_held` up so the
    /// block spans the opponent's attack instead of flickering for one tick.
    pub shield_hold_timer: f32,
    /// Seconds left in the post-poke neutral reset (whiff-punish footsies). Armed
    /// to [`SmashCfg::poke_reset_s`] on the falling edge of a swing; while positive
    /// the fighter suppresses offense and weaves out to its outer spacing pocket
    /// instead of re-swinging point-blank. `0` outside the window / for grunts.
    pub neutral_reset_timer: f32,
    /// Whether the actor was mid-swing last tick. Used to detect the swing's
    /// falling edge (swing → not-swinging) so the neutral reset arms exactly once
    /// per poke. Pure tick-stream bookkeeping → replay-safe.
    pub was_attacking: bool,
    /// Seconds left in the current regroup (break-off-and-reset after a beating).
    /// While positive the fighter retreats a real distance — dashing, and taking to
    /// the air for high ground if able — instead of trading. `0` outside a regroup.
    pub regroup_timer: f32,
    /// Own health fraction observed last tick, to detect DROPS (damage taken).
    pub last_health_fraction: f32,
    /// Decaying memory of recent damage taken (sum of health-fraction drops, bled
    /// off over a couple seconds). Arms a regroup when it crosses the threshold.
    pub damage_accum: f32,
    /// Seconds since this fighter last committed an attack (swing or shot). Drives
    /// the **stale-fight re-aggression** ([`SmashCfg::stale_fight_s`]): once it
    /// exceeds the threshold the fighter forces an offensive push instead of waiting
    /// out a passive standoff. Reset to `0` on every attack, so it only grows during
    /// a genuine lull. Pure tick-stream bookkeeping → replay-safe.
    pub time_since_offense: f32,
}

/// How long a reactive block is held once triggered (s) — long enough to span a
/// jab's active window.
const SHIELD_HOLD_S: f32 = 0.32;

/// Window (s) over which the perceived target velocity is estimated for the
/// blink-evade lunge detector. Short enough to read a burst, long enough to be
/// robust to a single-tick jitter.
const THREAT_WINDOW_S: f32 = 0.08;

/// Dash-to-close cadence (seconds) — a dash-capable actor bursts at
/// most once per this interval, so it punctuates the chase rather than
/// dashing every frame.
const DASH_COOLDOWN_S: f32 = 2.0;

/// Fraction of `aggro_radius` beyond which a dash-to-close fires. Only
/// *large* gaps are worth a burst; inside this the actor walks (and, if
/// ranged-capable, pokes) so it doesn't overshoot its firing range.
const DASH_CLOSE_FRACTION: f32 = 0.55;

/// Tick the Smash brain pipeline. Pure function modulo `state`
/// (which the difficulty stage mutates for its RNG advance + the
/// mode stage mutates for hysteresis bookkeeping).
pub fn tick_smash(
    cfg: &SmashCfg,
    state: &mut SmashState,
    actions: &ActionSet,
    snapshot: &BrainSnapshot,
    perception: Option<&crate::perception::WorldView>,
    out: &mut crate::actor::control::ActorControlFrame,
) {
    *out = crate::actor::control::ActorControlFrame::neutral();
    if !snapshot.alive {
        state.mode = BroadMode::Idle;
        return;
    }
    // Advance the dwell accumulator before any mode-flip check.
    state.mode_dwell_s += snapshot.dt;
    // Tick the brain-side cadences down (clamped at 0) before this
    // frame's verb selection can re-arm them. Fire-rate is NOT among them:
    // the body owns the ranged refire cooldown (invariant I3), so the brain
    // attempts a shot whenever it wants one and the body enforces the rate.
    state.dash_cooldown_remaining = (state.dash_cooldown_remaining - snapshot.dt).max(0.0);
    state.neutral_jump_cooldown = (state.neutral_jump_cooldown - snapshot.dt).max(0.0);
    state.blink_cooldown = (state.blink_cooldown - snapshot.dt).max(0.0);
    state.neutral_reset_timer = (state.neutral_reset_timer - snapshot.dt).max(0.0);
    state.regroup_timer = (state.regroup_timer - snapshot.dt).max(0.0);
    // Grows during an offense-drought; reset at the end of the tick on any attack.
    state.time_since_offense += snapshot.dt;
    // Advance the spacing phase — it drives the grounded footsies weave AND the
    // aerial dive/perch cycle, so a flyer needs it even with footsies disabled.
    if (cfg.footsies_amplitude > 0.0 || snapshot.actor_aerial) && cfg.footsies_period_s > 0.0 {
        state.spacing_phase += snapshot.dt * std::f32::consts::TAU / cfg.footsies_period_s;
        if state.spacing_phase > std::f32::consts::TAU {
            state.spacing_phase -= std::f32::consts::TAU;
        }
    }
    // --- Reaction latency ---
    // Record the opponent's true position this tick, then build the snapshot the
    // brain is actually allowed to perceive: the opponent as it was
    // `reaction_delay_s` ago. Only the OPPONENT is delayed — the actor's own
    // pos/vel/ground/timers are read live. This is what stops the brain from
    // frame-perfectly countering a sudden dash or jump; it's also the single
    // place that makes the difficulty knob fair instead of omniscient.
    state
        .obs_history
        .push(snapshot.sim_time, snapshot.target_pos);
    let perceived = {
        let mut s = *snapshot;
        if let Some(delayed_target) = state
            .obs_history
            .delayed(snapshot.sim_time, cfg.difficulty.reaction_delay_s)
        {
            s.target_pos = delayed_target;
        }
        s
    };
    let obs = observe(&perceived);
    // Poke-and-reset: arm the neutral-reset window on the swing's falling edge
    // (mid-swing last tick, done this tick). The fighter then disengages to its
    // outer spacing pocket before re-committing, instead of re-swinging in place.
    if state.was_attacking && !obs.self_attacking {
        state.neutral_reset_timer = cfg.poke_reset_s;
    }
    state.was_attacking = obs.self_attacking;
    // Regroup trigger: accumulate recent damage (health-fraction DROPS), bleed it
    // off over ~2s, and break off when it crosses the threshold. Health is a scalar,
    // so this is gravity-frame-agnostic. The first tick (last == 0.0 default) reads
    // as a rise, not a drop, so it never false-triggers.
    let hp = obs.self_health_fraction;
    let drop = (state.last_health_fraction - hp).max(0.0);
    state.last_health_fraction = hp;
    // Accumulate damage taken SINCE the last regroup (reset on trigger). The bleed
    // is deliberately tiny — far below the real in-fight damage rate (good defense
    // means hits are sparse) — so a "bunch of hits" actually accumulates instead of
    // being cancelled; it only forgives ancient chip damage over minutes.
    state.damage_accum = (state.damage_accum - snapshot.dt * 0.001).max(0.0) + drop;
    if cfg.regroup_damage_threshold > 0.0
        && state.regroup_timer <= 0.0
        && state.damage_accum >= cfg.regroup_damage_threshold
    {
        state.regroup_timer = cfg.regroup_duration_s;
        state.damage_accum = 0.0;
    }
    // Regrouped: once we've opened up the target separation, re-engage early.
    if state.regroup_timer > 0.0 && obs.distance_to_target >= cfg.regroup_distance {
        state.regroup_timer = 0.0;
    }
    // Stale-fight re-aggression: after a long enough drought of our OWN offense,
    // force an offensive push this tick — drop the reactive defense and the
    // neutral-game patience (footsies hold / post-poke reset) and just close and
    // swing, the way two platform-fighter players break a passive standoff instead
    // of both waiting forever. A regroup (deliberate break-off) outranks it. Resets
    // when we attack (end of tick), so it only fires during a genuine lull.
    let force_offense = cfg.stale_fight_s > 0.0
        && state.regroup_timer <= 0.0
        && state.time_since_offense >= cfg.stale_fight_s;
    let mode = choose_mode(&obs, cfg, state);
    let action = choose_action(&obs, mode, cfg, actions);
    // Verb selection by range (the player/enemy unification flex): a
    // ranged-capable actor closing on a mid-range target fires ranged
    // on its own cadence before committing to the melee finish.
    // Substituted *before* difficulty so the shot inherits the same
    // accuracy jitter / commit roll as a melee swing.
    let ranged = maybe_substitute_ranged(action, &obs, mode, cfg, actions);
    // Line-of-fire gate (S5, perception-driven): keep a substituted ranged shot
    // only if the body can actually land it — if a solid occludes the path to the
    // target, fall back to the movement action so the refiners below close /
    // reposition into a clear line instead of firing into a wall. The check reuses
    // the body's `WorldView` (the headless world-out port), so "do I have a shot"
    // is answered over the SAME geometry a shot would physically fly through. With
    // no perception (pure-stage tests) the gate is inert and the shot stands.
    let action = match ranged {
        SpecificAction::RangedAttack { .. }
            if !perception.map_or(true, |view| view.line_of_fire(obs.target_pos)) =>
        {
            action
        }
        other => other,
    };
    // The grounded movement refiners (dash-to-close, footsies weave, neutral hop)
    // only make sense for a body that walks + jumps. A flyer skips them — its 2D
    // motion is steered below — but keeps the dimension-agnostic ranged poke.
    let action = if obs.self_aerial {
        action
    } else if state.regroup_timer > 0.0 {
        // REGROUP (grounded): break off and cover ground — dash away if the burst is
        // ready (exercises the body dash), else walk away. Taking to the air for high
        // ground is decided below (after a ground dash). Frame-agnostic: "away" is the
        // sign along the gravity-perpendicular side axis.
        regroup_ground_action(&obs, cfg, state)
    } else if state.neutral_reset_timer > 0.0 && !force_offense {
        // Post-poke neutral reset (duelist whiff-punish footsies): suppress all
        // offense (start from Idle, ignoring this tick's melee / ranged / dash) and
        // let the in/out neutral weave reset the spacing — then allow a spacing hop.
        // This is what stops point-blank mashing and opens the approach phase where
        // the opponent's re-entry becomes a perceivable, defendable threat, without
        // a forced retreat that would wall-pin a cornered fighter. SKIPPED while
        // forcing offense — a stalled fighter re-commits its poke immediately rather
        // than patiently resetting — but the footsies weave below still runs in BOTH
        // branches, so a forced push never collapses the spacing into a wall (it's
        // the loss of footsies, not the reset, that corner-pins a fighter).
        let action = maybe_apply_footsies(SpecificAction::Idle, &obs, mode, cfg, state);
        maybe_neutral_jump(action, &obs, cfg, state)
    } else {
        // Then, if still just closing a *large* gap, burst a dash. Runs after
        // ranged so a mid-range poke wins over a dash (shoot, then dash to close
        // while the shot reloads).
        let action = maybe_substitute_dash(action, &obs, mode, cfg, state);
        // Neutral game (duelists only — no-op when footsies are disabled): weave
        // the spacing in/out around the engage band instead of camping point-blank,
        // then mix in a neutral hop. Runs last among the movement refiners so it
        // governs only the residual plain Walk/Idle; a committed poke / dash /
        // ranged shot is never overridden.
        let action = maybe_apply_footsies(action, &obs, mode, cfg, state);
        maybe_neutral_jump(action, &obs, cfg, state)
    };
    let action = apply_difficulty(action, &cfg.difficulty, state);
    emit_inputs(action, &obs, out);
    if obs.self_aerial {
        // Flyer: the grounded motor outputs (locomotion throttle, jump edge) don't
        // apply — discard them and steer a 2D velocity toward a dive/perch spacing
        // point. The attack verbs emit_inputs wrote (melee / ranged / special) are
        // dimension-agnostic and stay.
        out.locomotion = ae::Vec2::ZERO;
        out.jump_pressed = false;
        out.velocity_target = if state.regroup_timer > 0.0 {
            // Regrouping in the air: peel AWAY and UP to a high, far perch — the
            // "gain high ground while resetting" the design calls for. Frame-agnostic
            // (side / up axes).
            regroup_aerial_steer(&obs, cfg)
        } else {
            aerial_steer(&obs, mode, cfg, state)
        };
    }
    // Reactive defense (capability-gated). Reacts to a *perceivable* lunge — the
    // opponent closing fast — not a privileged read of its attack flag, so a human
    // could make the same read. The perceived target velocity comes from the SAME
    // lagged history that enforces reaction latency, so the defense can't beat the
    // opponent's commitment frame-perfectly. Layered: blink away if able (mobile),
    // else stand and block (shield). Attack verbs already emitted are left intact.
    state.shield_hold_timer = (state.shield_hold_timer - snapshot.dt).max(0.0);
    // A forced offensive push drops reactive defense — go in rather than turtle.
    if !obs.self_attacking && !force_offense {
        if let Some((away, closing)) = perceived_threat(&obs, cfg, state, snapshot.sim_time) {
            // Imperfect reaction (the "no perfect blocks" knob): only commit to a
            // defense some of the time, so some swings land and the bout doesn't
            // turtle into a stalemate. Layered on top of the reaction latency that
            // already makes the lunge perceived late.
            if difficulty::roll_unit(state) < cfg.defense_reactivity {
                // A committed lunge (fast closing) gets the mobile blink; ordinary
                // walk-in pressure gets the stand-ground block. Splitting the two
                // is the layered defensive game.
                let is_lunge = closing >= cfg.blink_closing_speed;
                if cfg.can_blink && is_lunge && state.blink_cooldown <= 0.0 {
                    // Emit a one-frame quick-blink TAP: the body's blink limb arms
                    // on `blink_pressed` but only commits on `blink_released`, and
                    // cancels in-frame if it sees neither held nor released. A human
                    // taps press→release across frames; the AI compresses that to a
                    // single frame by emitting BOTH edges, so the body actually
                    // teleports instead of arming-then-cancelling.
                    out.blink_pressed = true;
                    out.blink_released = true;
                    out.blink_quick_dir = away;
                    out.locomotion = ae::Vec2::ZERO;
                    out.velocity_target = ae::Vec2::ZERO;
                    state.blink_cooldown = cfg.blink_cooldown_s;
                } else if cfg.can_shield && obs.self_on_ground {
                    state.shield_hold_timer = SHIELD_HOLD_S;
                }
            }
        }
        // Hold the block up across its window: shield + stand ground.
        if state.shield_hold_timer > 0.0 && obs.self_on_ground && !out.blink_pressed {
            out.shield_held = true;
            out.locomotion = ae::Vec2::ZERO;
        }
    }
    // Hybrid flight: decide whether to be airborne and emit the fly toggle when
    // that differs from the body's current mode. Movement this tick still runs in
    // the *current* mode (above); the toggle takes effect next tick. No-op for a
    // pure grounded brawler or a pure flyer (cfg.can_fly == false).
    if cfg.can_fly {
        // During a regroup, take to the air for HIGH GROUND — but only once the
        // ground dash has fired (dash on cooldown), so the break-off reads as
        // "dash out, then rise" rather than launching on frame one.
        let want_air = if state.regroup_timer > 0.0 && state.dash_cooldown_remaining > 0.0 {
            true
        } else {
            decide_flight(&obs, cfg, state)
        };
        if want_air != obs.self_aerial {
            out.fly_toggle_pressed = true;
        }
    }
    // Stale-fight bookkeeping: any committed attack (this tick's swing/shot, or a
    // swing still in progress) resets the offense-drought clock, so `force_offense`
    // only ever triggers during a real lull — never mid-trade.
    if out.melee_pressed || out.fire.is_some() || obs.self_attacking {
        state.time_since_offense = 0.0;
    }
}

/// Hybrid-flight decision: should the fighter be airborne right now?
///
/// **The body PREFERS grounded.** It takes to the air only to cover a long
/// traversal gap — closing on a distant target faster than it could on foot — or
/// to reach a target far overhead that a jump can't contest; once it has closed
/// in, it lands and fights on the ground. Distance hysteresis (a higher take-off
/// than landing threshold) keeps the toggle from chattering at the boundary.
///
/// This is pure *policy* (invariant I4): flight here is free, so the preference
/// is the only thing keeping the fighter grounded; a resource cost will reinforce
/// it later, and a learned policy could rediscover the same trade-off. Returns the
/// DESIRED airborne state; the caller toggles when it differs from `self_aerial`.
fn decide_flight(obs: &ObservationFrame, cfg: &SmashCfg, _state: &mut SmashState) -> bool {
    // No live target in sensing range → no reason to leave the ground.
    if !obs.target_alive || obs.distance_to_target > cfg.aggro_radius {
        return false;
    }
    // Take off only for a genuinely long gap; once closed inside the (lower)
    // landing band, come back down and brawl. Hysteresis via the two thresholds.
    let threshold = if obs.self_aerial {
        cfg.aggro_radius * 0.42
    } else {
        cfg.aggro_radius * 0.60
    };
    let long_traversal = obs.distance_to_target > threshold;
    // A target far overhead (well beyond a jump) is also a fly case.
    let high_overhead = obs.to_target_up() > cfg.vertical_chase_min * 2.5;
    long_traversal || high_overhead
}

/// Detect an incoming lunge worth defending against, returning the WORLD-space
/// "away" direction (used by the blink). Threat = the opponent is *perceived* to
/// be closing on us faster than a walk while already in danger range. Perception
/// uses the lagged `obs_history` (reaction latency applies to defense too), so
/// it's fair. Shared by the blink-evade and the reactive block.
fn perceived_threat(
    obs: &ObservationFrame,
    cfg: &SmashCfg,
    state: &SmashState,
    now: f32,
) -> Option<(ae::Vec2, f32)> {
    let delay = cfg.difficulty.reaction_delay_s;
    let p_now = state.obs_history.delayed(now, delay)?;
    let p_prev = state.obs_history.delayed(now, delay + THREAT_WINDOW_S)?;
    let target_vel = (p_now - p_prev) / THREAT_WINDOW_S;
    let to_me = obs.self_pos - p_now;
    let dist = to_me.length();
    // Danger range: react as the opponent steps into ~2.5× poke range (wide enough
    // to guard an approach, not so wide it flinches at nothing).
    if dist < 1.0 || dist > cfg.attack_range * 2.5 {
        return None;
    }
    let closing = target_vel.dot(to_me / dist); // +ve = approaching us
    if closing < cfg.shield_closing_speed {
        return None;
    }
    // Evade UP-and-away, framed against local gravity (I10). The side component
    // (along the gravity-perpendicular axis) breaks the opponent's line; the
    // strong "up" bias (against gravity) sends the dodge into the open vertical
    // space rather than risking a blink straight into a side wall — wall-safe
    // without needing wall geometry, under any gravity orientation. For a flyer
    // this also resets it to a fresh perch; for a grounded body it's an evasive
    // air-reposition. Under screen-down gravity this is byte-identical to the old
    // `(to_me.x/dist * 0.5, -1)`.
    let side = (to_me.dot(obs.side_axis()) / dist) * 0.5;
    Some((
        (side * obs.side_axis() + obs.up_axis()).normalize_or_zero(),
        closing,
    ))
}

/// 2D steering for an aerial (free-mover) Smash fighter. Instead of grounded
/// footsies, it runs a **dive / perch** oscillation: it perches diagonally above-
/// and-beside the target to bait + reset, then dives onto it to land a strike,
/// using the vertical stage space a grounded brawler can't. Reuses the spacing
/// phase (no extra state). Fully frame-agnostic (I10): "above" and "beside" are
/// the local `up_axis` / `side_axis`, so the dive/perch arc is correct under any
/// gravity orientation — under screen-down gravity it is byte-identical to the
/// old screen-space offsets. Returns a desired world velocity for
/// `velocity_target`.
fn aerial_steer(
    obs: &ObservationFrame,
    mode: BroadMode,
    cfg: &SmashCfg,
    state: &SmashState,
) -> ae::Vec2 {
    // Hold position through a swing so the strike connects rather than drifting
    // back out of range mid-attack.
    if obs.self_attacking {
        return ae::Vec2::ZERO;
    }
    let side = obs.side_axis();
    let up = obs.up_axis();
    // Aerial steering is velocity-target based (not grounded locomotion), so it uses
    // the TIGHT alignment test, not the grounded run/facing deadzone: a flyer wants
    // its perch side to track the target's true side even at small offsets, and the
    // wider grounded deadzone would freeze its perch on one wall.
    let toward = if obs.to_target_side().abs() < 0.001 {
        obs.self_facing
    } else {
        obs.to_target_side().signum()
    };
    let phase = state.spacing_phase + seed_phase_offset(state.rng_seed);
    // Dive/perch parameter in [0, 1]: 0 = dive onto the target (enter attack
    // range), 1 = perch above-and-beside it.
    let t = 0.5 + 0.5 * phase.sin();
    // Cross-up: the perch side flips on a slower phase, so between dives the flyer
    // crosses over the target (left-perch → dive → right-perch) instead of camping
    // one side. Falls back toward the target's side when it has no momentum.
    let cross = (phase * 0.5).sin();
    let perch_side = if cross.abs() < 0.05 {
        toward
    } else {
        cross.signum()
    };
    let perch = obs.target_pos
        + side * (perch_side * cfg.engage_distance)
        + up * (cfg.engage_distance * 0.85);
    let dive = obs.target_pos;
    let (desired, speed) = match mode {
        // Pressured / crowded: peel off to a higher, farther perch.
        BroadMode::Retreat | BroadMode::Reposition => (
            obs.target_pos
                + side * (-toward * cfg.engage_distance * 1.3)
                + up * (cfg.engage_distance * 1.4),
            cfg.retreat_speed,
        ),
        // No engagement: hold station.
        BroadMode::Idle => (obs.self_pos, 0.0),
        // Neutral / engage: ride the dive→perch arc.
        _ => (dive.lerp(perch, t), cfg.chase_speed),
    };
    let to_desired = desired - obs.self_pos;
    // Ease into the target point so the flyer settles instead of overshooting and
    // oscillating around it.
    let throttle = (to_desired.length() / 22.0).min(1.0);
    to_desired.normalize_or_zero() * speed * throttle
}

/// Per-actor footsies phase offset (radians) derived from the stable RNG seed,
/// so two duelists with the same cfg weave out of phase instead of mirror-locking
/// into a symmetric stalemate. Pure function of the seed → replay-safe.
fn seed_phase_offset(rng_seed: u64) -> f32 {
    ((rng_seed >> 40) & 0xFFFF) as f32 / 65535.0 * std::f32::consts::TAU
}

/// Footsies weave (duelist neutral game). Replaces a plain neutral `Walk`/`Idle`
/// with movement that settles the actor around a *weaving* desired gap: it dips
/// into poke range on a rhythm (where `choose_action` will commit a swing), then
/// backs out to bait a whiff — instead of collapsing to point-blank and mashing.
///
/// Frame-agnostic: reads only the target-relative `distance_to_target` /
/// `to_target_x`. Never overrides a committed attack, jump, dash, or ranged shot
/// (those aren't `Walk`/`Idle`), so it can't suppress offense. No-op unless
/// `cfg.footsies_amplitude > 0.0`.
fn maybe_apply_footsies(
    action: SpecificAction,
    obs: &ObservationFrame,
    mode: BroadMode,
    cfg: &SmashCfg,
    state: &SmashState,
) -> SpecificAction {
    if cfg.footsies_amplitude <= 0.0 {
        return action;
    }
    // Only govern grounded neutral movement; leave attacks/jumps/dashes and the
    // airborne / retreat-too-close / reposition / recover cases alone.
    if !matches!(action, SpecificAction::Walk { .. } | SpecificAction::Idle)
        || obs.self_attacking
        || !obs.self_on_ground
        || !matches!(mode, BroadMode::Approach | BroadMode::Engage)
    {
        return action;
    }
    let phase = state.spacing_phase + seed_phase_offset(state.rng_seed);
    // Weave in and out on the sine to bait and whiff-punish. The in-half of the
    // cycle is what keeps a cornered fighter from pinning itself against a wall —
    // a pure outward retreat would drift the pressured fighter into the corner and
    // freeze it (the brain has no wall geometry to back away from).
    let desired_gap = cfg.engage_distance + cfg.footsies_amplitude * phase.sin();
    // Weave direction along the local SIDE axis (I10) so footsies stay correct under
    // rotated gravity. Byte-identical to `to_target_x` under screen-down. Uses the
    // HELD facing inside the alignment deadzone so the weave keeps a stable in/out
    // direction (the gap-band logic, not a jittering sign, governs in/out) — and a
    // grounded fighter doesn't rapid-flip when the target stacks on the gravity axis.
    let toward = obs.side_face_toward_target();
    // Small deadzone so the actor settles (holds, facing the foe) at the pocket
    // rather than jittering one frame in / one frame out. Kept tight so the
    // weave keeps the actor micro-repositioning rather than camping a spot.
    let deadzone = 6.0;
    if obs.distance_to_target > desired_gap + deadzone {
        SpecificAction::Walk { dir: toward }
    } else if obs.distance_to_target < desired_gap - deadzone {
        SpecificAction::Walk { dir: -toward }
    } else {
        SpecificAction::Idle
    }
}

/// Neutral hop (duelist mix-up). Converts an approach `Walk` into a `Jump` on a
/// cadence so the actor varies its approach vector and uses vertical stage space
/// rather than only shuffling on the floor. No-op unless
/// `cfg.neutral_jump_cadence_s > 0.0`. Re-arms the cadence on commit.
fn maybe_neutral_jump(
    action: SpecificAction,
    obs: &ObservationFrame,
    cfg: &SmashCfg,
    state: &mut SmashState,
) -> SpecificAction {
    if cfg.neutral_jump_cadence_s <= 0.0
        || state.neutral_jump_cooldown > 0.0
        || !obs.self_on_ground
        || obs.self_attacking
        || !matches!(action, SpecificAction::Walk { .. })
    {
        return action;
    }
    // Only hop within the neutral band — a spacing hop that inherits the weave
    // direction (often a back-hop), NOT a leap across the stage straight into the
    // opponent. Beyond the band the actor closes on the ground (walk / dash).
    let neutral_band = cfg.engage_distance + cfg.footsies_amplitude * 1.5;
    if obs.distance_to_target > neutral_band {
        return action;
    }
    state.neutral_jump_cooldown = cfg.neutral_jump_cadence_s;
    SpecificAction::Jump
}

/// Grounded regroup movement: retreat AWAY from the target — dashing to cover
/// ground when the burst is ready (the body enforces the dash capability), else
/// walking. Re-arms the dash cadence on a dash, which the fly toggle then keys off
/// to rise to high ground. Frame-agnostic: "away" is the sign along the
/// gravity-perpendicular side axis (`to_target_side`), so it's correct under any
/// gravity orientation — a duel where the player flips gravity stays sensible.
fn regroup_ground_action(
    obs: &ObservationFrame,
    cfg: &SmashCfg,
    state: &mut SmashState,
) -> SpecificAction {
    let toward = if obs.to_target_side().abs() < 0.001 {
        obs.self_facing
    } else {
        obs.to_target_side().signum()
    };
    let away = -toward;
    if cfg.dash_to_close && obs.self_on_ground && state.dash_cooldown_remaining <= 0.0 {
        state.dash_cooldown_remaining = DASH_COOLDOWN_S;
        SpecificAction::Dash { dir: away }
    } else {
        SpecificAction::Walk { dir: away }
    }
}

/// Aerial regroup steering: drive AWAY from the target and UP, to a high far perch —
/// gaining high ground while resetting. Frame-agnostic via the gravity-relative
/// side / up axes (byte-identical to screen `away`+`up` under screen-down gravity).
fn regroup_aerial_steer(obs: &ObservationFrame, cfg: &SmashCfg) -> ae::Vec2 {
    let toward = if obs.to_target_side().abs() < 0.001 {
        obs.self_facing
    } else {
        obs.to_target_side().signum()
    };
    let desired = obs.target_pos
        + obs.side_axis() * (-toward * cfg.regroup_distance)
        + obs.up_axis() * (cfg.engage_distance * 1.6);
    let to_desired = desired - obs.self_pos;
    to_desired.normalize_or_zero() * cfg.chase_speed
}

/// Replace a *closing walk* over a large approach gap with a
/// [`SpecificAction::Dash`] burst when the actor is dash-capable
/// ([`SmashCfg::dash_to_close`]), grounded, not mid-swing, and the dash
/// cadence is ready. Only fires beyond [`DASH_CLOSE_FRACTION`] of the
/// aggro radius, so the actor doesn't dash *through* its ideal melee /
/// firing distance. Re-arms the cadence on commit. A ranged poke (run
/// earlier) or a melee swing already wins — only a plain Walk converts.
fn maybe_substitute_dash(
    action: SpecificAction,
    obs: &ObservationFrame,
    mode: BroadMode,
    cfg: &SmashCfg,
    state: &mut SmashState,
) -> SpecificAction {
    if !cfg.dash_to_close
        || obs.self_attacking
        || !obs.self_on_ground
        || state.dash_cooldown_remaining > 0.0
    {
        return action;
    }
    let closing_walk = matches!(action, SpecificAction::Walk { .. });
    let approaching = matches!(mode, BroadMode::Approach | BroadMode::Engage);
    let big_gap = obs.distance_to_target > cfg.aggro_radius * DASH_CLOSE_FRACTION;
    if !(closing_walk && approaching && big_gap) {
        return action;
    }
    state.dash_cooldown_remaining = DASH_COOLDOWN_S;
    // Dash along the local SIDE axis (I10) toward the target — correct under any
    // gravity; byte-identical to `to_target_x` under screen-down. Held facing inside
    // the alignment deadzone so the burst direction doesn't flip on a stacked target.
    let dir = obs.side_face_toward_target();
    SpecificAction::Dash { dir }
}

/// Replace a *closing* action (`Walk`/`Idle` toward the target) with a
/// ranged shot when the actor has a ranged verb, is at mid-range
/// (inside aggro, outside melee reach), is approaching/holding (not
/// retreating), and isn't mid-swing. Melee swings already in reach and
/// retreats are never overridden — the actor still closes for the
/// melee finish once the shot lands.
///
/// The brain does NOT rate-limit here: it attempts a ranged shot on every
/// in-band tick and the **body** enforces the fire rate (invariant I3,
/// `BodyMelee::try_fire_ranged`). A blocked attempt simply spawns
/// nothing; the controller never beats the weapon's rate by attempting faster.
fn maybe_substitute_ranged(
    action: SpecificAction,
    obs: &ObservationFrame,
    mode: BroadMode,
    cfg: &SmashCfg,
    actions: &ActionSet,
) -> SpecificAction {
    if actions.ranged.is_none() || obs.self_attacking {
        return action;
    }
    let closing = matches!(action, SpecificAction::Walk { .. } | SpecificAction::Idle);
    let approaching = matches!(mode, BroadMode::Approach | BroadMode::Engage);
    let in_band =
        obs.distance_to_target > cfg.attack_range && obs.distance_to_target <= cfg.aggro_radius;
    if !(closing && approaching && in_band) {
        return action;
    }
    // Aim along the body-local side axis toward the target. `emit_inputs` wraps
    // this as a `controlled_body_local` fire request whose `x` is the body's side
    // axis, so the sign must come from the gravity-perpendicular `to_target_side`,
    // not screen `x` (I10). Under screen-down gravity this equals `to_target_x`.
    // Held facing when the target aligns on the gravity axis (a shot always needs a
    // direction — at the ranged band the deadzone effectively never applies).
    let dir_x = obs.side_face_toward_target();
    SpecificAction::RangedAttack {
        dir: ae::Vec2::new(dir_x, 0.0),
    }
}

#[cfg(test)]
mod tests;
