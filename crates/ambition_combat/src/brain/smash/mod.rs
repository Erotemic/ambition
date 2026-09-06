//! Smash-brawl brain pipeline: observe, choose a broad mode, choose a capability-gated
//! action, apply difficulty policy, then emit an [`ambition_characters::actor::control::ActorControlFrame`].
//! Each stage is pure over its input plus [`SmashCfg`] / [`SmashState`].

// ⭐ THE PINNED DATA, NAMED DOWNWARD. These live in `ambition_characters`
// because the `Brain` encoder there reads them and the orphan rule will not
// let them move up; this crate depends on that one, so naming them is legal
// and is the whole shape of the split.
use ambition_characters::brain::action_set::ActionSet;
use ambition_characters::brain::smash::{BroadMode, SmashCfg, SmashState};
use ambition_characters::brain::snapshot::BrainSnapshot;
// `ae` is used both by `maybe_substitute_ranged` (the ranged-verb emit) and the
// tests, so the import is no longer test-gated.
use ambition_platformer2d_core as ae;

pub mod action;
pub mod difficulty;
pub mod emit;
pub mod mode;
pub mod observation;

#[cfg(test)]
mod arena;

pub use action::{choose_action, SpecificAction};
pub use difficulty::apply_difficulty;
pub use emit::emit_inputs;
pub use mode::choose_mode;
pub use observation::{observe, ObservationFrame};

/// How long a reactive block is held once triggered (s) — long enough to span a
/// jab's active window.
const SHIELD_HOLD_S: f32 = 0.32;

/// Window (s) over which the perceived target velocity is estimated for the
/// blink-evade lunge detector. Short enough to read a burst, long enough to be
/// robust to a single-tick jitter.
const THREAT_WINDOW_S: f32 = 0.08;

/// Sprint-to-close cadence (seconds) — an actor commits a hard approach at
/// most once per this interval, so it punctuates the chase rather than
/// running flat out every frame.
const SPRINT_COOLDOWN_S: f32 = 2.0;

/// Fraction of `aggro_radius` beyond which a sprint-to-close fires. Only
/// *large* gaps are worth the commitment; inside this the actor walks (and, if
/// ranged-capable, pokes) so it doesn't overshoot its firing range.
const SPRINT_CLOSE_FRACTION: f32 = 0.55;

/// Tick the Smash brain pipeline. Pure function modulo `state`
/// (which the difficulty stage mutates for its RNG advance + the
/// mode stage mutates for hysteresis bookkeeping).
pub fn tick_smash(
    cfg: &SmashCfg,
    state: &mut SmashState,
    actions: &ActionSet,
    snapshot: &BrainSnapshot,
    perception: Option<&ambition_characters::perception::WorldView>,
    out: &mut ambition_characters::actor::control::ActorControlFrame,
) {
    *out = ambition_characters::actor::control::ActorControlFrame::neutral();
    if !snapshot.alive {
        state.mode = BroadMode::Idle;
        return;
    }
    // ── Capture context ─────────────────────────────────────────────────────
    //
    // BEFORE everything, and it RETURNS. A fighter in a capture — at
    // either end — is not a fighter with extra options; the ordinary decision
    // does not apply and running it would ask "should I approach?" of a body
    // that cannot walk.
    //
    // and the whole point is what these arms emit: nothing capture-shaped.
    // `SpecificAction::CaptureAttack` writes the ordinary `melee_pressed` and an
    // attack direction — the same two fields a person's Attack button writes —
    // and `trigger_moveset_moves` turns them into a pummel or a throw by reading
    // the SAME relationship. There is no capture API a brain can call, which is
    // what keeps a CPU and a human on one road rather than two that agree today.
    let obs = observe(snapshot);
    if snapshot.captured {
        // Held — and struggling, which is the whole of a captive's agency. The
        // note that stood here said escape did not exist yet and that this would
        // be its arm when it did; it does, and this is.
        state.mode = BroadMode::Idle;
        if ambition_characters::control::struggling_this_tick(snapshot.captured_for, snapshot.dt) {
            emit_inputs(SpecificAction::CaptureStruggle, &obs, out);
        }
        return;
    }
    if snapshot.holding_captive {
        // deliberately the simplest policy that proves the road: pummel once,
        // then throw. It is not grab AI — opponent percent, stage edge, kill
        // potential and escape risk are all real inputs it does not read. What it
        // proves is that a CPU reaches a pummel and a throw through the ordinary
        // control surface, which is the thing that would be expensive to retrofit.
        emit_inputs(
            SpecificAction::CaptureAttack {
                forward: snapshot.pummels_landed >= 1,
            },
            &obs,
            out,
        );
        return;
    }
    // Advance the dwell accumulator before any mode-flip check.
    state.mode_dwell_s += snapshot.dt;
    // Fire-rate is NOT among them: the body owns the ranged refire cooldown (invariant I3), so
    // the brain attempts a shot whenever it wants one and the body enforces the rate.
    state.sprint_cooldown_remaining = (state.sprint_cooldown_remaining - snapshot.dt).max(0.0);
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
    // frame-perfectly countering a sudden sprint or jump; it's also the single
    // place that makes the difficulty knob fair instead of omniscient.
    state
        .obs_history
        .push(snapshot.sim_time, snapshot.target_pos);
    let perceived = {
        // `clone` since FB4b: `BrainSnapshot` carries the attack kit and is no
        // longer `Copy`. One clone per smash brain per tick, on a path that
        // already allocates its observation history.
        let mut s = snapshot.clone();
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
    // The grounded movement refiners (sprint-to-close, footsies weave, neutral hop)
    // only make sense for a body that walks + jumps. A flyer skips them — its 2D
    // motion is steered below — but keeps the dimension-agnostic ranged poke.
    let action = if obs.self_aerial {
        action
    } else if state.regroup_timer > 0.0 {
        // REGROUP (grounded): break off and cover ground — sprint away if the cadence
        // is ready, else walk away. Taking to the air for high ground is decided
        // below (after a ground sprint). Frame-agnostic: "away" is the
        // sign along the gravity-perpendicular side axis.
        regroup_ground_action(&obs, cfg, state)
    } else if state.neutral_reset_timer > 0.0 && !force_offense {
        // Post-poke neutral reset (duelist whiff-punish footsies): suppress all
        // offense (start from Idle, ignoring this tick's melee / ranged / sprint) and
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
        // Then, if still just closing a *large* gap, sprint. Runs after ranged
        // so a mid-range poke wins over a sprint (shoot, then close hard while
        // the shot reloads).
        let action = maybe_substitute_sprint(action, &obs, mode, cfg, state);
        // Neutral game (duelists only — no-op when footsies are disabled): weave
        // the spacing in/out around the engage band instead of camping point-blank,
        // then mix in a neutral hop. Runs last among the movement refiners so it
        // governs only the residual plain Walk/Idle; a committed poke / sprint /
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
        out.locomotion = ae::LocalAxes::ZERO;
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
                    out.blink_quick_dir = ae::WorldVec2(away);
                    out.locomotion = ae::LocalAxes::ZERO;
                    out.velocity_target = ae::WorldVec2::ZERO;
                    state.blink_cooldown = cfg.blink_cooldown_s;
                } else if cfg.can_shield && (!cfg.shield_requires_ground || obs.self_on_ground) {
                    state.shield_hold_timer = SHIELD_HOLD_S;
                }
            }
        }
        // Hold the block up across its window: shield + stand ground.
        //
        // A CPU now requests the SAME semantic action a human's shield button resolves to, and
        // `emit_inputs` owns what that means on a frame.
        if state.shield_hold_timer > 0.0
            && (!cfg.shield_requires_ground || obs.self_on_ground)
            && !out.blink_pressed
        {
            // Reactive defense LAYERS a commitment onto an already chosen action; it does not
            // replace it.
            let facing = out.facing;
            emit_inputs(SpecificAction::Shield, &obs, out);
            out.facing = facing;
        }
    }
    // Hybrid flight: decide whether to be airborne and emit the fly toggle when
    // that differs from the body's current mode. Movement this tick still runs in
    // the *current* mode (above); the toggle takes effect next tick. No-op for a
    // pure grounded brawler or a pure flyer (cfg.can_fly == false).
    if cfg.can_fly {
        // During a regroup, take to the air for HIGH GROUND — but only once the
        // ground sprint has fired (cadence armed), so the break-off reads as
        // "run out, then rise" rather than launching on frame one.
        let want_air = if state.regroup_timer > 0.0 && state.sprint_cooldown_remaining > 0.0 {
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
/// The body PREFERS grounded. It takes to the air only to cover a long
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
/// footsies, it runs a dive / perch oscillation: it perches diagonally above-
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
) -> ae::WorldVec2 {
    // Hold position through a swing so the strike connects rather than drifting
    // back out of range mid-attack.
    if obs.self_attacking {
        return ae::WorldVec2::ZERO;
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
    ae::WorldVec2(to_desired.normalize_or_zero() * speed * throttle)
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
/// `to_target_x`. Never overrides a committed attack, jump, sprint, or ranged shot
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
    // Only govern grounded neutral movement; leave attacks/jumps/sprints and the
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
    // opponent. Beyond the band the actor closes on the ground (walk / sprint).
    let neutral_band = cfg.engage_distance + cfg.footsies_amplitude * 1.5;
    if obs.distance_to_target > neutral_band {
        return action;
    }
    state.neutral_jump_cooldown = cfg.neutral_jump_cadence_s;
    SpecificAction::Jump
}

/// Grounded regroup movement: retreat AWAY from the target — sprinting to cover
/// ground when the cadence is ready, else
/// walking. Re-arms the sprint cadence on a sprint, which the fly toggle then keys
/// off to rise to high ground. Frame-agnostic: "away" is the sign along the
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
    if cfg.sprint_to_close && obs.self_on_ground && state.sprint_cooldown_remaining <= 0.0 {
        state.sprint_cooldown_remaining = SPRINT_COOLDOWN_S;
        SpecificAction::Sprint { dir: away }
    } else {
        SpecificAction::Walk { dir: away }
    }
}

/// Aerial regroup steering: drive AWAY from the target and UP, to a high far perch —
/// gaining high ground while resetting. Frame-agnostic via the gravity-relative
/// side / up axes (byte-identical to screen `away`+`up` under screen-down gravity).
fn regroup_aerial_steer(obs: &ObservationFrame, cfg: &SmashCfg) -> ae::WorldVec2 {
    let toward = if obs.to_target_side().abs() < 0.001 {
        obs.self_facing
    } else {
        obs.to_target_side().signum()
    };
    let desired = obs.target_pos
        + obs.side_axis() * (-toward * cfg.regroup_distance)
        + obs.up_axis() * (cfg.engage_distance * 1.6);
    let to_desired = desired - obs.self_pos;
    ae::WorldVec2(to_desired.normalize_or_zero() * cfg.chase_speed)
}

/// Replace a *closing walk* over a large approach gap with a
/// [`SpecificAction::Sprint`] — full throttle — when the policy allows it
/// ([`SmashCfg::sprint_to_close`]), the actor is grounded, not mid-swing, and
/// the cadence is ready. Only fires beyond [`SPRINT_CLOSE_FRACTION`] of the
/// aggro radius, so the actor doesn't run *through* its ideal melee / firing
/// distance. Re-arms the cadence on commit. A ranged poke (run earlier) or a
/// melee swing already wins — only a plain Walk converts.
///
/// the gate is entirely a DECISION about cadence and distance. Nothing here
/// asks what the body can do, because running is not a capability.
fn maybe_substitute_sprint(
    action: SpecificAction,
    obs: &ObservationFrame,
    mode: BroadMode,
    cfg: &SmashCfg,
    state: &mut SmashState,
) -> SpecificAction {
    if !cfg.sprint_to_close
        || obs.self_attacking
        || !obs.self_on_ground
        || state.sprint_cooldown_remaining > 0.0
    {
        return action;
    }
    let closing_walk = matches!(action, SpecificAction::Walk { .. });
    let approaching = matches!(mode, BroadMode::Approach | BroadMode::Engage);
    let big_gap = obs.distance_to_target > cfg.aggro_radius * SPRINT_CLOSE_FRACTION;
    if !(closing_walk && approaching && big_gap) {
        return action;
    }
    state.sprint_cooldown_remaining = SPRINT_COOLDOWN_S;
    // Run along the local SIDE axis (I10) toward the target — correct under any
    // gravity; byte-identical to `to_target_x` under screen-down. Held facing inside
    // the alignment deadzone so the direction doesn't flip on a stacked target.
    let dir = obs.side_face_toward_target();
    SpecificAction::Sprint { dir }
}

/// Replace a *closing* action (`Walk`/`Idle` toward the target) with a
/// ranged shot when the actor has a ranged verb, is at mid-range
/// (inside aggro, outside melee reach), is approaching/holding (not
/// retreating), and isn't mid-swing. Melee swings already in reach and
/// retreats are never overridden — the actor still closes for the
/// melee finish once the shot lands.
///
/// The brain does NOT rate-limit here: it attempts a ranged shot on every
/// in-band tick and the body enforces the fire rate (invariant I3,
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
