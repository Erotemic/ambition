//! Smash-brawl brain template — SSBM Subspace-Emissary feel.
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
/// `enemy_archetypes.ron`.
#[derive(Clone, Copy, Debug)]
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
    /// When true, the fighter may **blink-evade** a fast-closing opponent (a
    /// perceivable lunge, read from the lagged target history — never from a
    /// privileged attack flag). Capability gate only: the body still needs the
    /// blink ability for the emitted intent to resolve, exactly like the player.
    /// `false` for grunts.
    pub can_blink: bool,
    /// Minimum seconds between blink-evades. Ignored when [`Self::can_blink`]
    /// is `false`.
    pub blink_cooldown_s: f32,
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
        can_blink: false,
        blink_cooldown_s: 0.0,
        can_shield: false,
        can_fly: false,
        aerial_foray_cadence_s: 0.0,
        aerial_foray_duration_s: 0.0,
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
        can_blink: false,
        blink_cooldown_s: 0.0,
        can_shield: false,
        can_fly: false,
        aerial_foray_cadence_s: 0.0,
        aerial_foray_duration_s: 0.0,
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
        can_blink: true,
        blink_cooldown_s: 1.2,
        can_shield: true,
        // Grounded duelist by default; hybrid flight is opt-in per fighter.
        can_fly: false,
        aerial_foray_cadence_s: 0.0,
        aerial_foray_duration_s: 0.0,
        difficulty: DifficultyProfile::MEDIUM,
    };
}

/// Number of opponent-position samples retained for reaction latency.
/// At 60 fps this is ~0.53 s of history — comfortably longer than any
/// authored `reaction_delay_s` (EASY = 0.30 s), so the delayed lookup is
/// always covered once the buffer fills.
const OBS_HISTORY_LEN: usize = 32;

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

    /// The opponent position the brain is allowed to perceive this tick: the
    /// most recent sample that is at least `delay` seconds old. Never returns
    /// anything newer than `now - delay`, so the brain truly can't react
    /// faster than its latency. Until the buffer covers the window (fight
    /// start), returns the oldest sample it has — lag ramps up rather than
    /// snapping on. `None` only when no sample has been recorded yet.
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
}

/// How long a reactive block is held once triggered (s) — long enough to span a
/// jab's active window.
const SHIELD_HOLD_S: f32 = 0.32;

/// Window (s) over which the perceived target velocity is estimated for the
/// blink-evade lunge detector. Short enough to read a burst, long enough to be
/// robust to a single-tick jitter.
const THREAT_WINDOW_S: f32 = 0.08;
/// Perceived closing speed (px/s) toward the fighter that counts as a "lunge"
/// worth evading. Above a walk (~110) but a dash-in (~260) clears it easily.
const THREAT_CLOSING_SPEED: f32 = 175.0;

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
        out.velocity_target = aerial_steer(&obs, mode, cfg, state);
    }
    // Reactive defense (capability-gated). Reacts to a *perceivable* lunge — the
    // opponent closing fast — not a privileged read of its attack flag, so a human
    // could make the same read. The perceived target velocity comes from the SAME
    // lagged history that enforces reaction latency, so the defense can't beat the
    // opponent's commitment frame-perfectly. Layered: blink away if able (mobile),
    // else stand and block (shield). Attack verbs already emitted are left intact.
    state.shield_hold_timer = (state.shield_hold_timer - snapshot.dt).max(0.0);
    if !obs.self_attacking {
        if let Some(away) = perceived_threat(&obs, cfg, state, snapshot.sim_time) {
            if cfg.can_blink && state.blink_cooldown <= 0.0 {
                out.blink_pressed = true;
                out.blink_quick_dir = away;
                out.locomotion = ae::Vec2::ZERO;
                out.velocity_target = ae::Vec2::ZERO;
                state.blink_cooldown = cfg.blink_cooldown_s;
            } else if cfg.can_shield && obs.self_on_ground {
                state.shield_hold_timer = SHIELD_HOLD_S;
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
    if cfg.can_fly && decide_flight(&obs, cfg, state) != obs.self_aerial {
        out.fly_toggle_pressed = true;
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
) -> Option<ae::Vec2> {
    let delay = cfg.difficulty.reaction_delay_s;
    let p_now = state.obs_history.delayed(now, delay)?;
    let p_prev = state.obs_history.delayed(now, delay + THREAT_WINDOW_S)?;
    let target_vel = (p_now - p_prev) / THREAT_WINDOW_S;
    let to_me = obs.self_pos - p_now;
    let dist = to_me.length();
    if dist < 1.0 || dist > cfg.attack_range * 2.2 {
        return None;
    }
    let closing = target_vel.dot(to_me / dist); // +ve = approaching us
    if closing <= THREAT_CLOSING_SPEED {
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
    Some((side * obs.side_axis() + obs.up_axis()).normalize_or_zero())
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
    let desired_gap = cfg.engage_distance + cfg.footsies_amplitude * phase.sin();
    let toward = if obs.to_target_x.abs() < 0.001 {
        obs.self_facing
    } else {
        obs.to_target_x.signum()
    };
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
    state.neutral_jump_cooldown = cfg.neutral_jump_cadence_s;
    SpecificAction::Jump
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
    let dir = if obs.to_target_x.abs() < 0.001 {
        obs.self_facing
    } else {
        obs.to_target_x.signum()
    };
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
/// `ActorAttackState::try_fire_ranged`). A blocked attempt simply spawns
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
    let dir_x = if obs.to_target_side().abs() < 0.001 {
        obs.self_facing
    } else {
        obs.to_target_side().signum()
    };
    SpecificAction::RangedAttack {
        dir: ae::Vec2::new(dir_x, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap_with_target_at_x(target_x: f32) -> BrainSnapshot {
        let mut s = BrainSnapshot::idle();
        s.actor_pos = ae::Vec2::new(0.0, 0.0);
        s.target_pos = ae::Vec2::new(target_x, 0.0);
        s.actor_on_ground = true;
        s.target_alive = true;
        s
    }

    #[test]
    fn idles_when_target_out_of_range() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let actions = ActionSet::peaceful();
        let snap = snap_with_target_at_x(2000.0);
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
        assert_eq!(
            frame.locomotion.x, 0.0,
            "actor outside aggro_radius should not move"
        );
        assert!(!frame.melee_pressed);
    }

    #[test]
    fn approaches_when_target_in_aggro_but_out_of_attack() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let actions = ActionSet::peaceful();
        // Target at 300 px — inside aggro (460), outside engage (70).
        let snap = snap_with_target_at_x(300.0);
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
        assert!(
            frame.locomotion.x > 0.0,
            "actor should approach a target to its right; got vel={:?}",
            frame.locomotion,
        );
    }

    #[test]
    fn melee_smash_swings_when_target_is_point_blank() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let actions = ActionSet {
            melee: Some(crate::brain::MeleeActionSpec::Swipe(
                crate::brain::SwipeSpec::STRIKER_DEFAULT,
            )),
            ..ActionSet::peaceful()
        };
        // 20px is inside STRIKER_DEFAULT.too_close_distance, but a
        // melee-capable Smash actor should take the point-blank swing
        // instead of backing away forever. This pins the cove-pirate
        // regression where provoked NPCs approached, then held range
        // without ever swinging when the player was beside them.
        let snap = snap_with_target_at_x(20.0);
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
        assert!(frame.melee_pressed, "point-blank melee actor should swing");
    }

    /// Difficulty profile that always commits and never jitters, so the
    /// ranged-cadence tests are deterministic regardless of rng seed.
    fn crisp_striker_cfg() -> SmashCfg {
        SmashCfg {
            difficulty: DifficultyProfile {
                reaction_delay_s: 0.0,
                commit_probability: 1.0,
                accuracy: 1.0,
                ..DifficultyProfile::HARD
            },
            ..SmashCfg::STRIKER_DEFAULT
        }
    }

    fn ranged_actions() -> ActionSet {
        ActionSet {
            ranged: Some(crate::brain::RangedActionSpec::Rock {
                speed: 300.0,
                damage: 2,
            }),
            ..ActionSet::peaceful()
        }
    }

    #[test]
    fn ranged_capable_actor_fires_at_mid_range() {
        // A Smash actor with a ranged verb, at mid-range (inside aggro
        // 460, outside melee reach 56), fires ranged rather than silently
        // walking closer — the player/enemy "verb selection by range" flex.
        let cfg = crisp_striker_cfg();
        let mut state = SmashState::default();
        let actions = ranged_actions();
        let snap = snap_with_target_at_x(300.0);
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
        assert!(
            frame.fire.is_some(),
            "ranged actor should attempt fire at mid-range"
        );
        assert!(!frame.melee_pressed, "should not also melee at mid-range");
        // The brain no longer rate-limits: it attempts a shot on every in-band
        // tick. A second tick still emits `fire` (the BODY throttles, not the
        // brain — invariant I3). This is what a spam controller would also do.
        let mut frame2 = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame2);
        assert!(
            frame2.fire.is_some(),
            "brain keeps attempting fire every in-band tick; the body enforces the rate"
        );
    }

    /// A `WorldView` whose terrain is the given solids, with self at the origin —
    /// the perception a body at (0,0) would have.
    fn view_with_terrain(terrain: Vec<crate::perception::PerceivedSolid>) -> crate::perception::WorldView {
        use crate::perception::{SelfView, Viewport, WorldView};
        WorldView {
            self_view: SelfView {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::ZERO,
                facing: 1.0,
                half_extent: ae::Vec2::new(10.0, 16.0),
                gravity_down: ae::Vec2::new(0.0, 1.0),
                on_ground: true,
                aerial: false,
                alive: true,
                faction: crate::actor::ActorFaction::Enemy,
                can_fire: true,
                can_blink: false,
                can_dash: false,
                can_shield: false,
            },
            viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(800.0)),
            actors: vec![],
            projectiles: vec![],
            terrain,
            portals: vec![],
            sim_time: 0.0,
        }
    }

    /// Line-of-fire gate (S5): the same mid-range body that fires with a clear
    /// shot must NOT fire when a solid wall occludes the path to the target — it
    /// falls back to closing instead of firing into a wall. Proven against the
    /// REAL brain pipeline + REAL `WorldView::line_of_fire` over the carried solids.
    #[test]
    fn ranged_shot_suppressed_when_line_of_fire_blocked() {
        use crate::perception::{PerceivedSolid, SolidKind};
        let cfg = crisp_striker_cfg();
        let actions = ranged_actions();
        let snap = snap_with_target_at_x(300.0); // body (0,0) → target (300,0)

        // Clear view: the body fires (matches the None-perception behavior).
        let clear = view_with_terrain(vec![]);
        let mut state = SmashState::default();
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, Some(&clear), &mut frame);
        assert!(
            frame.fire.is_some(),
            "with a clear line of fire the body still shoots"
        );

        // Wall at x=150 squarely between the body and the target → no shot, and
        // the body keeps closing (a movement intent) toward a clear line.
        let blocked = view_with_terrain(vec![PerceivedSolid {
            aabb: ae::Aabb::new(ae::Vec2::new(150.0, 0.0), ae::Vec2::new(8.0, 60.0)),
            kind: SolidKind::Solid,
        }]);
        let mut state = SmashState::default();
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, Some(&blocked), &mut frame);
        assert!(
            frame.fire.is_none(),
            "a wall between body and target suppresses the ranged shot (no firing into walls)"
        );
        assert!(
            frame.locomotion.length() > 0.0,
            "with the shot blocked the body falls back to closing for a clear line"
        );
    }

    #[test]
    fn melee_takes_precedence_over_ranged_in_reach() {
        // With BOTH verbs, a point-blank target gets the melee swing,
        // not a ranged shot — ranged only substitutes for *closing*
        // actions outside melee range.
        let cfg = crisp_striker_cfg();
        let mut state = SmashState::default();
        let actions = ActionSet {
            melee: Some(crate::brain::MeleeActionSpec::Swipe(
                crate::brain::SwipeSpec::STRIKER_DEFAULT,
            )),
            ..ranged_actions()
        };
        let snap = snap_with_target_at_x(20.0); // inside attack_range
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
        assert!(frame.melee_pressed, "in-reach actor swings");
        assert!(frame.fire.is_none(), "does not fire ranged in melee reach");
    }

    #[test]
    fn brain_does_not_self_rate_limit_fire_body_owns_the_rate() {
        // Invariant I3: the brain no longer gates its own fire rate — it attempts
        // a ranged shot on EVERY in-band tick. The body (`try_fire_ranged`) is
        // the floor that turns those attempts into the weapon's rate. So back-to-
        // back ticks both emit `fire`; nothing in the brain throttles them. (The
        // body-side throttle is proven over real systems in the fighter harness.)
        let cfg = crisp_striker_cfg();
        let mut state = SmashState::default();
        let actions = ranged_actions();
        let mut snap = snap_with_target_at_x(300.0);
        snap.dt = 0.2;

        for tick in 0..8 {
            let mut frame = crate::actor::control::ActorControlFrame::neutral();
            tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
            assert!(
                frame.fire.is_some(),
                "tick {tick}: brain keeps attempting fire — the body, not the brain, enforces cadence"
            );
        }
    }

    fn dash_striker_cfg() -> SmashCfg {
        SmashCfg {
            dash_to_close: true,
            ..crisp_striker_cfg()
        }
    }

    #[test]
    fn dash_capable_actor_bursts_to_close_a_large_gap() {
        // A dash-capable Smash actor closing a large gap (beyond
        // DASH_CLOSE_FRACTION * aggro ≈ 0.55 * 460 ≈ 253) bursts a Dash
        // (260 px/s) instead of plodding at walk speed (170).
        let cfg = dash_striker_cfg();
        let mut state = SmashState::default();
        let actions = ActionSet::peaceful(); // no ranged → dash, not a poke
        let snap = snap_with_target_at_x(300.0);
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
        assert!(
            frame.locomotion.x > 0.8,
            "dash burst should exceed walk speed; got {}",
            frame.locomotion.x
        );
        assert!(
            state.dash_cooldown_remaining > 0.0,
            "dash cadence armed on commit"
        );
    }

    #[test]
    fn dash_is_only_for_large_gaps() {
        // Inside the dash fraction (120 < 253) the actor walks, not dashes.
        let cfg = dash_striker_cfg();
        let mut state = SmashState::default();
        let actions = ActionSet::peaceful();
        let snap = snap_with_target_at_x(120.0);
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
        assert!(
            frame.locomotion.x > 0.0 && frame.locomotion.x < 0.8,
            "a small gap walks, not dashes; got {}",
            frame.locomotion.x
        );
        assert_eq!(
            state.dash_cooldown_remaining, 0.0,
            "no dash armed for a small gap"
        );
    }

    #[test]
    fn non_dash_actor_walks_the_same_large_gap() {
        // The SAME large gap, but dash_to_close OFF → a plain walk: the
        // capability is gated on the cfg flag, not on by default.
        let cfg = crisp_striker_cfg(); // dash_to_close = false
        let mut state = SmashState::default();
        let actions = ActionSet::peaceful();
        let snap = snap_with_target_at_x(300.0);
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
        assert!(
            frame.locomotion.x > 0.0 && frame.locomotion.x < 0.8,
            "no dash capability → a walk; got {}",
            frame.locomotion.x
        );
    }

    /// Crisp difficulty (always commit, no jitter) with an explicit reaction
    /// delay, so latency tests aren't confounded by the commit roll.
    fn crisp_cfg_with_delay(delay_s: f32) -> SmashCfg {
        SmashCfg {
            difficulty: DifficultyProfile {
                reaction_delay_s: delay_s,
                commit_probability: 1.0,
                accuracy: 1.0,
                ..DifficultyProfile::HARD
            },
            ..SmashCfg::STRIKER_DEFAULT
        }
    }

    fn run_tick(
        cfg: &SmashCfg,
        state: &mut SmashState,
        actions: &ActionSet,
        target_x: f32,
        t: f32,
    ) -> ae::Vec2 {
        let mut snap = snap_with_target_at_x(target_x);
        snap.sim_time = t;
        snap.dt = 1.0 / 60.0;
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(cfg, state, actions, &snap, None, &mut frame);
        frame.locomotion
    }

    #[test]
    fn reaction_latency_delays_response_to_a_sudden_move() {
        // The never-cheats guarantee: after the opponent suddenly teleports
        // from far-right to far-left, the brain keeps pursuing the STALE
        // (right) position for ~reaction_delay_s before it perceives the new
        // one and flips. This is the headless proof the AI can't frame-
        // perfectly counter.
        let dt = 1.0 / 60.0;
        let delay = 0.15;
        let cfg = crisp_cfg_with_delay(delay);
        let mut state = SmashState::default();
        let actions = ActionSet::peaceful();
        let mut t = 0.0;
        // Settle approaching the right-hand target so the buffer fills.
        for _ in 0..30 {
            let loco = run_tick(&cfg, &mut state, &actions, 300.0, t);
            assert!(loco.x > 0.0, "should approach the right-hand target");
            t += dt;
        }
        // Opponent teleports to the LEFT. The very next tick still pursues
        // right (perceiving the lagged position).
        let loco = run_tick(&cfg, &mut state, &actions, -300.0, t);
        assert!(
            loco.x > 0.0,
            "right after the teleport the brain still chases the stale position; got {loco:?}",
        );
        t += dt;
        // Within the reaction window the brain must NOT have flipped yet.
        let mut flipped_at: Option<f32> = None;
        let teleport_t = t - dt;
        for _ in 0..40 {
            let loco = run_tick(&cfg, &mut state, &actions, -300.0, t);
            if loco.x < 0.0 {
                flipped_at = Some(t - teleport_t);
                break;
            }
            t += dt;
        }
        let elapsed = flipped_at.expect("brain eventually pursues the new position");
        assert!(
            elapsed >= delay - dt,
            "brain flipped after {elapsed:.3}s — faster than its {delay:.3}s reaction delay (cheating)",
        );
        assert!(
            elapsed <= delay + 6.0 * dt,
            "brain flipped after {elapsed:.3}s — far later than its {delay:.3}s reaction delay",
        );
    }

    #[test]
    fn zero_reaction_delay_responds_immediately() {
        // Control: with reaction_delay_s == 0 the brain has no perception lag
        // and flips the very next tick after the opponent moves.
        let dt = 1.0 / 60.0;
        let cfg = crisp_cfg_with_delay(0.0);
        let mut state = SmashState::default();
        let actions = ActionSet::peaceful();
        let mut t = 0.0;
        for _ in 0..30 {
            run_tick(&cfg, &mut state, &actions, 300.0, t);
            t += dt;
        }
        let loco = run_tick(&cfg, &mut state, &actions, -300.0, t);
        assert!(
            loco.x < 0.0,
            "with zero reaction delay the brain pursues the new position immediately; got {loco:?}",
        );
    }

    #[test]
    fn dead_actor_emits_neutral_frame() {
        let cfg = SmashCfg::STRIKER_DEFAULT;
        let mut state = SmashState::default();
        let actions = ActionSet::peaceful();
        let mut snap = snap_with_target_at_x(100.0);
        snap.alive = false;
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        // Pre-poison: if `tick_smash` early-returns without writing,
        // the assertion below would catch a leak from the caller's
        // pre-existing frame state.
        frame.melee_pressed = true;
        frame.locomotion = ae::Vec2::new(999.0, 999.0);
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
        assert!(!frame.melee_pressed, "dead actor must not emit melee");
        assert_eq!(frame.locomotion, ae::Vec2::ZERO);
    }

    // --- S2: frame-agnostic motor / perception (invariant I10) ---

    /// Build an idle snapshot with rotated gravity. `down` is the world gravity
    /// direction; `target` the world target position.
    fn snap_rotated(down: ae::Vec2, target: ae::Vec2) -> BrainSnapshot {
        let mut s = BrainSnapshot::idle();
        s.actor_pos = ae::Vec2::ZERO;
        s.control_down = down;
        s.target_pos = target;
        s.target_alive = true;
        s.actor_on_ground = false;
        s.actor_aerial = true;
        s
    }

    /// Under gravity rotated 90° (down = screen `+x`), the reactive evade points
    /// AGAINST gravity (screen `-x`), not screen `-y`. The old code hard-coded
    /// `-y`, which would dodge sideways into a wall under this orientation. The
    /// dodge must climb the open vertical space whatever the gravity frame.
    #[test]
    fn evade_dodges_against_gravity_under_rotated_gravity() {
        let cfg = crisp_striker_cfg(); // reaction_delay_s = 0
        let down = ae::Vec2::new(1.0, 0.0);
        // Target closing fast on the actor from the side, along screen -y.
        let near = ae::Vec2::new(0.0, 40.0);
        let far = ae::Vec2::new(0.0, 60.0);
        let mut snap = snap_rotated(down, near);
        let now = 1.0;
        snap.sim_time = now;
        let obs = observe(&snap);
        let mut state = SmashState::default();
        state.obs_history.push(now - THREAT_WINDOW_S, far);
        state.obs_history.push(now, near);

        let away = perceived_threat(&obs, &cfg, &state, now).expect("a fast lunge is a threat");
        assert!(
            away.dot(obs.up_axis()) > 0.5,
            "evade must climb against gravity (up = -down); got {away:?}"
        );
        assert!(
            away.dot(down) < 0.0,
            "evade must never dive into gravity; got {away:?}"
        );
    }

    /// `to_target_up` is frame-correct: a target offset against gravity reads as
    /// "above" regardless of screen orientation. Under down = screen `+x`, a
    /// target at screen `-x` is above.
    #[test]
    fn target_above_is_gravity_relative() {
        // down = +x ⇒ up = -x. Target at screen -x (200 left) is "above".
        let snap = snap_rotated(ae::Vec2::new(1.0, 0.0), ae::Vec2::new(-200.0, 0.0));
        let obs = observe(&snap);
        assert!(
            obs.to_target_up() > 100.0,
            "target opposite gravity must read as above; got {}",
            obs.to_target_up()
        );
        // And a target *along* gravity (screen +x) reads as below.
        let snap_below = snap_rotated(ae::Vec2::new(1.0, 0.0), ae::Vec2::new(200.0, 0.0));
        assert!(
            observe(&snap_below).to_target_up() < -100.0,
            "target along gravity must read as below"
        );
    }

    /// The aerial dive/perch steers into the gravity-relative "up" space: a flyer
    /// engaging a target perches against gravity, not toward screen `-y`. Under
    /// down = screen `+x`, the steered velocity carries the flyer to the up side.
    #[test]
    fn aerial_perch_climbs_against_gravity() {
        let cfg = SmashCfg::DUELIST_DEFAULT; // a real neutral game / flyer cfg
        let down = ae::Vec2::new(1.0, 0.0);
        // Flyer sitting ON the target's gravity-line so the only steer is up/down.
        let target = ae::Vec2::new(0.0, 0.0);
        let mut snap = snap_rotated(down, target);
        snap.actor_pos = ae::Vec2::new(0.0, 0.0);
        let obs = observe(&snap);
        let state = SmashState::default();
        // Engage mode rides the dive→perch arc; perch sits above-and-beside.
        let vel = aerial_steer(&obs, BroadMode::Engage, &cfg, &state);
        // The desired point biases against gravity, so the steer has a positive
        // up-component (it is not allowed to be a screen-`-y`-only push).
        assert!(
            vel.dot(obs.up_axis()) >= 0.0,
            "aerial steer must not drive into gravity; got {vel:?} under down={down:?}"
        );
    }

    // --- S3b: hybrid flight prefers grounded, flies to traverse ---

    fn hybrid_obs(distance_x: f32, currently_aerial: bool) -> ObservationFrame {
        let mut snap = snap_with_target_at_x(distance_x);
        snap.actor_aerial = currently_aerial;
        snap.actor_on_ground = !currently_aerial;
        observe(&snap)
    }

    /// The hybrid PREFERS grounded: with a target close in, it does not take to
    /// the air; with a target a long traversal away, it does. (Brain *policy* —
    /// flight is free for now, so this preference is the only thing keeping it
    /// grounded.)
    #[test]
    fn hybrid_flight_prefers_grounded_flies_to_traverse() {
        let mut cfg = SmashCfg::DUELIST_DEFAULT;
        cfg.can_fly = true;
        cfg.aggro_radius = 500.0; // take-off > 300, land > 210
        let mut state = SmashState::default();

        // Close target, on the ground → stay grounded.
        assert!(
            !decide_flight(&hybrid_obs(120.0, false), &cfg, &mut state),
            "a grounded hybrid should NOT fly to a target it can just walk to"
        );
        // Distant target, on the ground → take off to cover the gap.
        assert!(
            decide_flight(&hybrid_obs(420.0, false), &cfg, &mut state),
            "a grounded hybrid SHOULD fly to close a long traversal gap"
        );
        // A target beyond sensing range is not chased into the air.
        assert!(
            !decide_flight(&hybrid_obs(900.0, false), &cfg, &mut state),
            "no live target in range → no reason to leave the ground"
        );
    }

    /// Hysteresis: once airborne it keeps flying through the mid-band (so the
    /// toggle doesn't chatter at the boundary), but lands once it has closed in.
    #[test]
    fn hybrid_flight_has_landing_hysteresis() {
        let mut cfg = SmashCfg::DUELIST_DEFAULT;
        cfg.can_fly = true;
        cfg.aggro_radius = 500.0; // take-off 300, land 210
        let mut state = SmashState::default();

        // Mid-band (between land=210 and take-off=300): keep flying if already up…
        assert!(
            decide_flight(&hybrid_obs(250.0, true), &cfg, &mut state),
            "an airborne hybrid keeps flying through the mid-band (hysteresis)"
        );
        // …but a grounded one would NOT have taken off at the same distance.
        assert!(
            !decide_flight(&hybrid_obs(250.0, false), &cfg, &mut state),
            "a grounded hybrid does not take off in the mid-band"
        );
        // Closed all the way in → land and brawl.
        assert!(
            !decide_flight(&hybrid_obs(150.0, true), &cfg, &mut state),
            "once closed inside the landing band, the hybrid comes down to fight"
        );
    }
}
