//! The pure boss-pattern brain tick: scripted-step advance, cycle/macro state
//! machines, front-wall standoff, retreat positioning, and desired-velocity emit.

use super::control_flow;
use ambition_characters::brain::boss_pattern::*;
use ambition_platformer2d_core as ae;

/// Pure brain tick: advance the cursor/clocks and write movement plus
/// [`BossAttackIntent`]. Move execution and live attack timing remain downstream.
pub fn tick_boss_pattern(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    out: &mut ambition_characters::actor::control::ActorControlFrame,
    attack_intent: &mut BossAttackIntent,
) {
    // Both outputs are per-tick facts. Clear them before every early return so
    // a paused or suppressed brain cannot leak yesterday's attack request.
    *out = ambition_characters::actor::control::ActorControlFrame::neutral();
    attack_intent.clear();

    if ctx.dt <= 0.0 {
        return;
    }

    let facing_delta_x = ctx.target_pos.x - ctx.actor_pos.x;
    if facing_delta_x.abs() > 2.0 {
        out.facing = facing_delta_x.signum();
    }

    // Tick the free-running clocks the movement profile reads.
    state.movement_timer += ctx.dt;
    state.pattern_timer += ctx.dt;

    // Phase change: reset the scripted cursor to step 0 of the new phase.
    let phase_entered = if state.last_phase != Some(ctx.encounter_phase) {
        state.step_index = 0;
        state.step_elapsed = 0.0;
        state.cycle_rest_remaining = 0.0;
        state.last_phase = Some(ctx.encounter_phase);
        // Reset to Engage so the macro timer does not carry a stale duration
        // across the music change.
        state.macro_state = BossMacroState::Engage;
        state.engage_timer = 0.0;
        // A new phase is a new script. Drop the resolved timeline, leave any
        // stance of the old phase, and re-resolve this tick. Interrupt
        // cooldowns reset too, so a rule can fire on the phase-2 beat it was
        // authored for.
        state.timeline.clear();
        state.stance_stack.clear();
        state.stance = None;
        state.interrupt_cooldowns.clear();
        state.interrupt_timers.clear();
        Some(ctx.encounter_phase)
    } else {
        None
    };

    // Advance the chase/engage/retreat macro state before emitting
    // `desired_vel`, so the movement override (Approach chases, Retreat pulls
    // away) matches the current macro state.
    if cfg.macro_tuning.is_enabled() && ctx.encounter_phase.is_attacking() {
        advance_macro_state(cfg, state, ctx);
    }

    // Non-attacking phases (Dormant / Stagger / Death) emit no intent and
    // clear it, so rendering does not draw a stale telegraph during a stagger.
    if !ctx.encounter_phase.is_attacking() {
        attack_intent.clear();
        // Still emit `desired_vel` from the movement profile, so a dormant boss
        // keeps its sway phase.
        emit_desired_vel(cfg, state, ctx, out);
        return;
    }

    // Bosses with a standoff macro do not start telegraph/strike actions
    // while closing distance or backing away. So the Smirking Behemoth moves
    // to its preferred ring, then idles or fires eye beams there, and does not
    // walk into the player.
    if cfg.macro_tuning.suppress_attacks_while_moving
        && matches!(
            state.macro_state,
            BossMacroState::Approach { .. } | BossMacroState::Retreat { .. }
        )
    {
        attack_intent.clear();
        emit_desired_vel(cfg, state, ctx, out);
        return;
    }

    match &cfg.pattern {
        BossAttackPattern::Scripted { .. } => {
            advance_scripted(cfg, state, ctx, attack_intent, phase_entered);
        }
        BossAttackPattern::Cycle => {
            advance_cycle(cfg, state, ctx, attack_intent);
        }
    }

    // Aggressiveness gates the typed boss-action channel, because the moveset
    // trigger reads `BossAttackIntent` directly. Clear it here, so a peaceful
    // boss can still advance its cursor without starting attacks.
    if cfg.aggressiveness <= 0.0 {
        attack_intent.clear();
    }

    emit_desired_vel(cfg, state, ctx, out);
}

/// The boss's one deterministic random stream (ADR 0023: no ambient RNG). A
/// value type so the ticker can hand `&mut` to `resolve_timeline` and to
/// `enter_stance` without also handing them the whole `BossPatternState` twice.
/// Seeded from the encounter id, checkpointed back into `state.rng_seed`.
pub(super) struct PatternRng(u64);

impl PatternRng {
    fn seeded(cfg: &BossPatternCfg, state: &BossPatternState) -> Self {
        if state.rng_seed != 0 {
            return Self(state.rng_seed);
        }
        Self(
            hash_boss_pattern_seed(&cfg.encounter_id)
                ^ 0x9E37_79B9_7F4A_7C15
                ^ ((state.step_index as u64) << 32),
        )
    }

    /// One uniform in `[0, 1)`.
    pub(super) fn unit(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let n = (self.0 >> 33) as u32;
        n as f32 / (1u64 << 31) as f32
    }
}

/// Guard against an authored cycle of zero-duration control flow (a stance
/// that only enters itself). Far above any real script; it stops a typo from
/// hanging the sim.
const MAX_CURSOR_STEPS_PER_TICK: u32 = 64;

/// Advance the resolved scripted-pattern timeline.
///
/// Interrupts run before cursor movement, stance markers are control-flow jumps,
/// and reaching the end either returns from a stance or re-resolves the phase so
/// `Select` choices are rolled once per loop pass.
fn advance_scripted(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    attack_intent: &mut BossAttackIntent,
    phase_entered: Option<BossEncounterPhase>,
) {
    let pattern = match cfg.pattern.pattern_for(ctx.encounter_phase) {
        Some(pattern) if !pattern.steps.is_empty() => pattern.clone(),
        _ => {
            attack_intent.clear();
            return;
        }
    };
    let mut rng = PatternRng::seeded(cfg, state);

    if state.timeline.is_empty() {
        state.timeline = control_flow::resolve_timeline(&pattern.steps, ctx, &mut || rng.unit());
        state.step_index = 0;
        state.step_elapsed = 0.0;
    }
    if state.timeline.is_empty() {
        attack_intent.clear();
        state.rng_seed = rng.0;
        return;
    }

    // The brain remembers its own health, so `OnHitTaken` needs no damage
    // channel: a drop since last tick is a hit; a heal is not.
    let damage_taken = state
        .last_hp
        .map_or(0, |before| (before - ctx.hp_current).max(0));
    state.last_hp = Some(ctx.hp_current);

    if let Some(rule) =
        control_flow::tick_interrupts(&pattern.interrupts, state, ctx, phase_entered, damage_taken)
    {
        let enter = pattern.interrupts[rule].enter.clone();
        // An interrupt resumes the step it left, with its elapsed time: a boss
        // pulled out of a telegraph returns to that telegraph, so the punish
        // window the player was reading stays where it was.
        let resume = (state.step_index, state.step_elapsed);
        control_flow::enter_stance(&pattern, state, ctx, &enter, resume, &mut || rng.unit());
    }

    state.step_elapsed += ctx.dt;
    let mut guard = 0u32;
    loop {
        guard += 1;
        if guard > MAX_CURSOR_STEPS_PER_TICK {
            break;
        }
        match state.timeline.get(state.step_index).cloned() {
            Some(BossPatternStep::Stance { id }) => {
                let resume = (state.step_index + 1, 0.0);
                if !control_flow::enter_stance(&pattern, state, ctx, &id, resume, &mut || {
                    rng.unit()
                }) {
                    // Unknown or empty stance: step over the marker. The
                    // validator flags it; mid-fight it must not panic or stall.
                    state.step_index += 1;
                }
            }
            Some(current) => {
                let duration = step_duration(&current).max(0.01);
                if state.step_elapsed < duration {
                    break;
                }
                if !scripted_step_ready_to_advance(cfg, state, ctx, &current, duration, &mut rng) {
                    break;
                }
                state.step_elapsed -= duration;
                state.step_index += 1;
            }
            None => {
                // Off the end. A stance returns to whoever entered it; the phase's
                // own timeline loops, re-rolling its `Select`s for the new pass.
                if control_flow::leave_stance(state) {
                    continue;
                }
                state.timeline =
                    control_flow::resolve_timeline(&pattern.steps, ctx, &mut || rng.unit());
                state.step_index = 0;
                if state.timeline.is_empty() {
                    attack_intent.clear();
                    state.rng_seed = rng.0;
                    return;
                }
            }
        }
    }
    state.rng_seed = rng.0;

    let steps = &state.timeline;
    let Some(current) = steps.get(state.step_index).cloned() else {
        attack_intent.clear();
        return;
    };
    match &current {
        BossPatternStep::Telegraph { profile, .. } => {
            attack_intent.telegraph_profile = Some(profile.clone());
            attack_intent.active_profile = None;
        }
        BossPatternStep::Strike { profile, .. } => {
            attack_intent.telegraph_profile = None;
            attack_intent.active_profile = Some(profile.clone());
        }
        // Control-flow and rest steps emit no request rather than carrying a
        // stale profile into the move trigger.
        BossPatternStep::Rest { .. }
        | BossPatternStep::Stance { .. }
        | BossPatternStep::Select { .. } => attack_intent.clear(),
    }
}

fn scripted_step_ready_to_advance(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    current: &BossPatternStep,
    duration: f32,
    rng: &mut PatternRng,
) -> bool {
    let chance_per_second = cfg.macro_tuning.idle_attack_chance_per_second.max(0.0);
    if chance_per_second <= 0.0 || !matches!(current, BossPatternStep::Rest { .. }) {
        return true;
    }

    // The Rest duration is the minimum idle time. After it, an optional
    // per-second chance decides whether the boss starts the next telegraph
    // now or keeps waiting ("idle, then maybe eye-beam" for the Smirking
    // Behemoth), without making every scripted boss probabilistic.
    let chance_this_tick = (chance_per_second * ctx.dt.max(0.0)).clamp(0.0, 1.0);
    if chance_this_tick >= 1.0 || rng.unit() < chance_this_tick {
        true
    } else {
        // Keep retrying the gate next tick without growing the elapsed value
        // without bound.
        state.step_elapsed = duration;
        false
    }
}

fn hash_boss_pattern_seed(id: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in id.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash.max(1)
}

fn clamp_world_lateral_approach_to_front_wall(
    cfg: &BossPatternCfg,
    ctx: &BossPatternContext,
    target: ae::Vec2,
) -> ae::Vec2 {
    let Some(clearance) = ctx.front_wall_clearance else {
        return target;
    };
    let dx = target.x - ctx.actor_pos.x;
    if dx.abs() <= 1e-3 {
        return target;
    }
    let allowed = (clearance - cfg.macro_tuning.front_wall_standoff.max(0.0)).max(0.0);
    if allowed <= 1.0 {
        return ae::Vec2::new(ctx.actor_pos.x, target.y);
    }
    if dx.abs() <= allowed {
        target
    } else {
        ae::Vec2::new(ctx.actor_pos.x + dx.signum() * allowed, target.y)
    }
}

/// Advance cycle-mode attack policy while the move runtime owns attack timing.
///
/// Sustain intent during windup so upstream suppression can still cancel it;
/// clear intent once the move strikes. The rest clock drains only while no move
/// is active, yielding `move duration + rest` cadence.
fn advance_cycle(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    attack_intent: &mut BossAttackIntent,
) {
    if let Some(live) = &ctx.live_attack {
        state.cycle_rest_remaining = cfg.cycle_attack_cooldown.max(0.05);
        if live.striking {
            attack_intent.clear();
        } else {
            attack_intent.telegraph_profile = Some(live.profile.clone());
            attack_intent.active_profile = None;
        }
        return;
    }
    if state.cycle_rest_remaining > 0.0 {
        state.cycle_rest_remaining = (state.cycle_rest_remaining - ctx.dt).max(0.0);
        attack_intent.clear();
        return;
    }

    // Rested and idle: request the rotation's current profile from windup.
    // An empty attack list falls back to `full_body_pulse`.
    let profile = if cfg.cycle_attacks.is_empty() {
        BossAttackProfile::Strike("full_body_pulse".to_string())
    } else {
        let cooldown = cfg.cycle_attack_cooldown.max(0.05);
        let idx = ((state.pattern_timer / cooldown) as usize) % cfg.cycle_attacks.len();
        cfg.cycle_attacks[idx].clone()
    };
    attack_intent.telegraph_profile = Some(profile);
    attack_intent.active_profile = None;
}

fn front_wall_standoff_reached(tuning: &BossMacroTuning, ctx: &BossPatternContext) -> bool {
    tuning.front_wall_standoff > 0.0
        && ctx
            .front_wall_clearance
            .is_some_and(|clearance| clearance <= tuning.front_wall_standoff + 1.0)
}

/// How much space between two body surfaces still counts as touching. Bodies
/// separated by integration tolerance are in contact for every visible
/// purpose. This is a skin on a real separation, not a substitute for the
/// bodies' size.
const CONTACT_SKIN: f32 = 4.0;

/// Lateral separation between the two body surfaces, negative once the boxes
/// overlap.
///
/// Measuring center distance instead would keep a wide body's contact chase
/// open forever, so the largest bodies would never engage (and, under
/// `suppress_attacks_while_moving`, never attack).
///
/// Lateral, not planar: a contact chase is the horizontal run-in a grounded
/// body performs, and the profiles that author it lock themselves to the arena
/// lane. A target directly overhead is not something this boss can walk into.
fn lateral_body_gap(cfg: &BossPatternCfg, ctx: &BossPatternContext) -> f32 {
    let centre_gap = (ctx.target_pos.x - ctx.actor_pos.x).abs();
    centre_gap - (cfg.combat_size.x + ctx.target_body_size.x) * 0.5
}

/// Advance the chase/engage/retreat macro state machine. Transitions:
///
/// - `Engage` → `Approach` if distance > too_far_distance, or in
///   contact-chase mode whenever the player is not yet horizontally
///   overlapping the boss.
/// - `Engage` → `Retreat` if distance < too_close_distance (anti-corner)
///   or engage_timer >= engage_max_duration_s (a periodic "preparing" beat).
/// - `Approach` → `Engage` if distance < engage_distance, if contact-chase
///   mode has horizontally closed, or if the timer expired.
/// - `Retreat` → `Engage` if timer expired
///
/// Retreat picks `retreat_pos` along the player→boss axis, so the boss
/// visibly retreats away from the player.
fn advance_macro_state(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
) {
    let movement = cfg.movement_for_phase(ctx.encounter_phase);
    // World-arena-lateral bosses measure standoff on the arena lane only.
    // Otherwise a player jumping over or under the boss would look "far away"
    // and the boss would slide into them. The intent is left/right spacing,
    // with collision handling the walls.
    let distance = if movement.world_arena_lateral_only() {
        (ctx.target_pos.x - ctx.actor_pos.x).abs()
    } else {
        (ctx.target_pos - ctx.actor_pos).length()
    };
    let tuning = &cfg.macro_tuning;
    let front_wall_blocked = front_wall_standoff_reached(tuning, ctx);
    let contact_chase_mode = tuning.contact_chase_mode();
    let contact_chase_closed = contact_chase_mode
        && lateral_body_gap(cfg, ctx) <= tuning.engage_distance.max(CONTACT_SKIN);
    match &mut state.macro_state {
        BossMacroState::Engage => {
            state.engage_timer += ctx.dt;
            let too_close = tuning.too_close_distance > 0.0 && distance < tuning.too_close_distance;
            let too_far = if contact_chase_mode {
                !contact_chase_closed
            } else {
                tuning.too_far_distance > 0.0 && distance > tuning.too_far_distance
            };
            let prep_due = tuning.engage_max_duration_s > 0.0
                && state.engage_timer >= tuning.engage_max_duration_s;
            if too_close || prep_due {
                state.macro_state = BossMacroState::Retreat {
                    remaining_s: tuning.retreat_duration_s.max(0.5),
                    retreat_pos: compute_retreat_pos(cfg, ctx),
                };
                state.engage_timer = 0.0;
            } else if too_far && !front_wall_blocked {
                state.macro_state = BossMacroState::Approach {
                    remaining_s: tuning.approach_duration_s.max(0.5),
                };
                state.engage_timer = 0.0;
            }
        }
        BossMacroState::Approach { remaining_s } => {
            *remaining_s -= ctx.dt;
            let close_enough = if contact_chase_mode {
                contact_chase_closed
            } else {
                tuning.engage_distance > 0.0 && distance < tuning.engage_distance
            };
            if close_enough || front_wall_blocked || *remaining_s <= 0.0 {
                state.macro_state = BossMacroState::Engage;
                state.engage_timer = 0.0;
            }
        }
        BossMacroState::Retreat { remaining_s, .. } => {
            *remaining_s -= ctx.dt;
            if *remaining_s <= 0.0 {
                state.macro_state = BossMacroState::Engage;
                state.engage_timer = 0.0;
            }
        }
    }
}

/// Pick a retreat anchor `retreat_distance` px from the player,
/// along the player→boss axis (with a fallback when the boss and
/// player are coincident). Clamped to the world bounds upstream by
/// `emit_desired_vel`.
fn compute_retreat_pos(cfg: &BossPatternCfg, ctx: &BossPatternContext) -> ae::Vec2 {
    let movement = cfg.movement_for_phase(ctx.encounter_phase);
    if movement.world_arena_lateral_only() {
        let dx = ctx.actor_pos.x - ctx.target_pos.x;
        let dir_x = if dx.abs() < 1e-3 { 1.0 } else { dx.signum() };
        // `BossRuntime::integrate_body` still runs through `step_motion`, so
        // walls and platforms stop the body if this target is beyond reachable
        // floor.
        let target_x = ctx.actor_pos.x + dir_x * cfg.macro_tuning.retreat_distance.max(60.0);
        return ae::Vec2::new(target_x * 0.6 + cfg.spawn.x * 0.4, ctx.actor_pos.y);
    }

    let away = ctx.actor_pos - ctx.target_pos;
    let dir = if away.length_squared() < 1e-3 {
        ae::Vec2::new(1.0, 0.0)
    } else {
        away.normalize()
    };
    // Anchor near the boss spawn, so retreat curves back toward the spawn and
    // does not drift the boss into arena edges over many encounters.
    let target = ctx.actor_pos + dir * cfg.macro_tuning.retreat_distance.max(60.0);
    target * 0.6 + cfg.spawn * 0.4
}

/// Movement-profile → frame.desired_vel translation. Runs even in
/// non-attacking phases so a dormant boss keeps its sway phase.
fn emit_desired_vel(
    cfg: &BossPatternCfg,
    state: &BossPatternState,
    ctx: &BossPatternContext,
    out: &mut ambition_characters::actor::control::ActorControlFrame,
) {
    if ctx.dt <= 0.0 {
        return;
    }

    // Phase-aware movement: Phase 2 / Enrage may override the default
    // movement profile (for example a slow sway escalating to a wide
    // AirSwoop).
    let movement = cfg.movement_for_phase(ctx.encounter_phase);
    // Macro state overrides the movement target: Approach chases the player,
    // Retreat heads to the retreat anchor, and Engage uses the normal
    // sway/swoop target. Speed scaling for Approach/Retreat is applied below
    // via `macro_speed_scale`.
    let mut target = match state.macro_state {
        BossMacroState::Approach { .. } => {
            // Bosses that author a `too_close_distance` keep a standoff ring.
            // Contact-chase bosses disable that ring and author
            // `engage_distance = 0`, so the target is the player's x and body
            // contact stops the run-in.
            let standoff = if cfg.macro_tuning.too_close_distance > 0.0 {
                cfg.macro_tuning
                    .engage_distance
                    .max(cfg.macro_tuning.too_close_distance + 12.0)
                    .max(48.0)
            } else {
                cfg.macro_tuning.engage_distance.max(0.0)
            };
            if movement.world_arena_lateral_only() {
                let dx = ctx.actor_pos.x - ctx.target_pos.x;
                let dir_x = if dx.abs() < 1e-3 { 1.0 } else { dx.signum() };
                ae::Vec2::new(ctx.target_pos.x + dir_x * standoff, ctx.actor_pos.y)
            } else {
                let away = ctx.actor_pos - ctx.target_pos;
                let dir = if away.length_squared() < 1e-3 {
                    ae::Vec2::new(1.0, 0.0)
                } else {
                    away.normalize()
                };
                ctx.target_pos + dir * standoff
            }
        }
        BossMacroState::Retreat { retreat_pos, .. } => retreat_pos,
        BossMacroState::Engage if cfg.macro_tuning.hold_position_while_engaged => ctx.actor_pos,
        BossMacroState::Engage => movement.target(cfg.spawn, state.movement_timer, ctx.target_pos),
    };

    // While a strike is live, a self-dodging boss adds a horizontal dodge to
    // the sway, so it reads as stepping aside from its own attack (GNU-ton
    // weaving out of its apple rain).
    let self_dodge_active = matches!(cfg.movement, BossMovementProfile::StationaryGiant { .. })
        && cfg.self_dodge_amp > 0.0
        && ctx.encounter_phase.is_attacking();
    if self_dodge_active {
        // This function cannot tell whether DebrisRain is active without the
        // `BossAttackState` mirror. It relies on the sway oscillator in
        // `state.movement_timer` running every tick.
        let _ = state.movement_timer;
    }

    // Soft world-bounds clamp: collision owns the hard stop, but the brain
    // does not ask to walk into it.
    let half = cfg.combat_size * 0.5;
    let margin = 8.0;
    let max_x = (ctx.world_size.x - half.x - margin).max(half.x + margin);
    let max_y = (ctx.world_size.y - half.y - margin).max(half.y + margin);
    let mut clamped_target = ae::Vec2::new(
        target.x.clamp(half.x + margin, max_x),
        target.y.clamp(half.y + margin, max_y),
    );
    if movement.world_arena_lateral_only() {
        // The profile has no authored vertical arena travel, so the macro
        // standoff/retreat steering must not add one.
        clamped_target.y = ctx.actor_pos.y;
    }
    target = clamped_target;

    if matches!(state.macro_state, BossMacroState::Approach { .. })
        && movement.world_arena_lateral_only()
        && cfg.macro_tuning.front_wall_standoff > 0.0
    {
        target = clamp_world_lateral_approach_to_front_wall(cfg, ctx, target);
    }

    let delta = target - ctx.actor_pos;
    // A strike's motion lock is the move's `MoveWindow::motion_scale`,
    // enforced at body integration for any controller; it is not applied
    // here.
    //
    // Macro-state speed scaling: Approach uses `> 1.0`, Retreat `< 1.0`,
    // Engage 1.0.
    let macro_scale = match state.macro_state {
        BossMacroState::Approach { .. } => cfg.macro_tuning.approach_speed_scale.max(0.0),
        BossMacroState::Retreat { .. } => cfg.macro_tuning.retreat_speed_scale.max(0.0),
        BossMacroState::Engage => 1.0,
    };
    let speed = movement.speed() * macro_scale;
    let max_step = speed * ctx.dt;
    // `delta` is a world-space position difference, so the command is
    // world-space, as `velocity_target`'s type states.
    out.velocity_target = ae::WorldVec2(if delta.length() > max_step && max_step > 0.0 {
        delta.normalize_or_zero() * speed
    } else if ctx.dt > 0.0 {
        delta / ctx.dt
    } else {
        ae::Vec2::ZERO
    });
}

// ===== The state-machine-shaped entry =====
//
// This adapter calls `tick_boss_pattern`, so it lives beside it and not in
// `ambition_characters` (which would be an upward dependency).

// ===== BossPattern =====
//
// The boss tick fills the BossPattern fields the pattern needs
// (`boss_encounter_phase` / `world_size` / `front_wall_clearance`) onto the shared
// snapshot, so a `BossPattern` brain ticks through the universal `Brain::tick`
// path as every other body. A snapshot without those fields (a non-boss
// caller that holds a BossPattern brain) ticks under a Dormant phase, which
// emits only the idle sway, never a strike.
pub fn tick_boss_pattern_via_state_machine(
    cfg: &ambition_characters::brain::boss_pattern::BossPatternCfg,
    state: &mut ambition_characters::brain::boss_pattern::BossPatternState,
    snapshot: &ambition_characters::brain::BrainSnapshot,
    out: &mut ambition_characters::actor::control::ActorControlFrame,
) {
    let ctx = ambition_characters::brain::boss_pattern::BossPatternContext {
        encounter_phase: snapshot.boss_encounter_phase.unwrap_or_default(),
        actor_pos: snapshot.actor_pos,
        target_pos: snapshot.target_pos,
        // A point target: the shared snapshot has no body box. This path
        // ticks Dormant (see above), so it never reaches contact reasoning.
        // The ECS boss tick passes the real body.
        target_body_size: ae::Vec2::ZERO,
        world_size: snapshot.world_size,
        front_wall_clearance: snapshot.front_wall_clearance,
        dt: snapshot.dt,
        // Situation buckets. The snapshot has a health fraction, not a pool,
        // so hp is on a 0..100 scale here: `HpBelow` reads the ratio, and
        // `OnHitTaken`'s min_damage is percent-of-max on this path. The ECS
        // boss tick passes the real pool.
        actor_facing: snapshot.actor_facing,
        hp_current: (snapshot.health_fraction.clamp(0.0, 1.0) * 100.0).round() as i32,
        hp_max: 100,
        // This path ticks under a Dormant phase (see above) and never reaches
        // the attack patterns, so there is no live move to observe.
        live_attack: None,
    };
    // The universal brain API has one generic control-frame output. Keep the
    // boss-specific profile request as a transient cache on the pattern state so
    // the ECS adapter can publish it without making `ActorControlFrame` boss-aware.
    let mut attack_intent = core::mem::take(&mut state.attack_intent);
    super::tick_boss_pattern(cfg, state, &ctx, out, &mut attack_intent);
    state.attack_intent = attack_intent;
}
