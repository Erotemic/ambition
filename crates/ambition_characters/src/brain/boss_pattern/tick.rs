//! The pure boss-pattern brain tick: scripted-step advance, cycle/macro state
//! machines, front-wall standoff, retreat positioning, and desired-velocity emit.
//!
//! Split out of the former 1327-line `boss_pattern/mod.rs` (2026-06-15).

use super::*;

/// Pure brain tick: advance the cursor/clocks, write movement and action intent,
/// and mirror the live telegraph/active profile into [`BossAttackState`].
pub fn tick_boss_pattern(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    out: &mut crate::actor::control::ActorControlFrame,
    attack_state: &mut BossAttackState,
) {
    // Always start from a neutral frame so a leaked
    // `melee_pressed = true` from a previous (now-stale) state
    // can't survive into the next tick.
    *out = crate::actor::control::ActorControlFrame::neutral();

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

    // Phase change → reset the scripted cursor. Scripted patterns
    // anchor on step 0 of the new phase rather than carrying the
    // old phase's cursor in mid-step.
    if state.last_phase != Some(ctx.encounter_phase) {
        state.step_index = 0;
        state.step_elapsed = 0.0;
        state.cycle_phase = CyclePhase::Cooldown;
        state.cycle_phase_remaining = 0.0;
        state.last_phase = Some(ctx.encounter_phase);
        // Reset to Engage on phase change so the macro timer
        // doesn't carry stale duration across the music swap.
        state.macro_state = BossMacroState::Engage;
        state.engage_timer = 0.0;
    }

    // Advance the chase/engage/retreat macro state machine BEFORE
    // emitting desired_vel so the movement override (Approach
    // chases the player, Retreat pulls away) is in lockstep with
    // the current macro state.
    if cfg.macro_tuning.is_enabled() && ctx.encounter_phase.is_attacking() {
        advance_macro_state(cfg, state, ctx);
    }

    // Non-attacking phases (Dormant / Stagger / Death) emit no intent
    // and clear the mirror so rendering doesn't keep drawing a stale
    // telegraph through a stagger window.
    if !ctx.encounter_phase.is_attacking() {
        attack_state.clear();
        // Still emit desired_vel from the movement profile so a
        // boss in Dormant still keeps its sway phase (matches the
        // legacy behavior).
        emit_desired_vel(cfg, state, ctx, out, attack_state);
        return;
    }

    // Bosses with a standoff macro should not begin telegraph/strike
    // actions while closing distance or backing away. This keeps the
    // Smirking Behemoth from intentionally walking into the player;
    // it moves to its preferred ring, then spends that close-range
    // window idling or firing eye beams.
    if cfg.macro_tuning.suppress_attacks_while_moving
        && matches!(
            state.macro_state,
            BossMacroState::Approach { .. } | BossMacroState::Retreat { .. }
        )
    {
        attack_state.clear();
        emit_desired_vel(cfg, state, ctx, out, attack_state);
        return;
    }

    match &cfg.pattern {
        BossAttackPattern::Scripted { .. } => {
            advance_scripted(cfg, state, ctx, attack_state);
        }
        BossAttackPattern::Cycle => {
            advance_cycle(cfg, state, ctx, attack_state);
        }
    }

    // Edge tags into the ActorControlFrame: while a Strike is active,
    // emit melee_pressed for ordinary profiles and special_pressed
    // for profiles the EFFECTS consumer handles (apple rain today).
    // ActionSet binds special_pressed to `SpecialActionSpec::Special("apple_rain")`;
    // the resolver writes `ActorActionMessage::Special`; the consumer
    // spawns the apples.
    if cfg.aggressiveness > 0.0 {
        if let Some(profile) = attack_state.active_profile.as_ref() {
            if profile.is_special() {
                out.special_pressed = true;
            } else {
                out.melee_pressed = true;
            }
        }
    }

    emit_desired_vel(cfg, state, ctx, out, attack_state);
}

/// Scripted-pattern cursor advancement.
fn advance_scripted(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    attack_state: &mut BossAttackState,
) {
    let steps: Vec<BossPatternStep> = match cfg.pattern.pattern_for(ctx.encounter_phase) {
        Some(pattern) if !pattern.steps.is_empty() => pattern.steps.clone(),
        _ => {
            attack_state.clear();
            return;
        }
    };

    state.step_elapsed += ctx.dt;
    // Wrap the cursor if a phase transition shrunk the script under
    // our feet, then advance through any completed steps this frame.
    if state.step_index >= steps.len() {
        state.step_index = 0;
        state.step_elapsed = 0.0;
    }
    loop {
        let current = &steps[state.step_index];
        let duration = step_duration(current).max(0.01);
        if state.step_elapsed < duration {
            break;
        }
        if !scripted_step_ready_to_advance(cfg, state, ctx, current, duration) {
            break;
        }
        state.step_elapsed -= duration;
        state.step_index = (state.step_index + 1) % steps.len();
    }

    let current = &steps[state.step_index];
    let current_duration = step_duration(current).max(0.01);
    let elapsed = state.step_elapsed.clamp(0.0, current_duration);
    let remaining = (current_duration - elapsed).max(0.0);
    match current {
        BossPatternStep::Telegraph { profile, .. } => {
            attack_state.telegraph_profile = Some(profile.clone());
            attack_state.telegraph_remaining = remaining;
            attack_state.telegraph_elapsed = elapsed;
            attack_state.active_profile = None;
            attack_state.active_remaining = 0.0;
            attack_state.active_elapsed = 0.0;
        }
        BossPatternStep::Strike { profile, .. } => {
            let previous_telegraph_elapsed = if state.step_index > 0 {
                match &steps[state.step_index - 1] {
                    BossPatternStep::Telegraph {
                        profile: prev_profile,
                        duration,
                    } if prev_profile == profile => (*duration).max(0.0),
                    _ => 0.0,
                }
            } else {
                0.0
            };
            attack_state.telegraph_profile = None;
            attack_state.telegraph_remaining = 0.0;
            attack_state.telegraph_elapsed = 0.0;
            attack_state.active_profile = Some(profile.clone());
            attack_state.active_remaining = remaining;
            attack_state.active_elapsed = previous_telegraph_elapsed + elapsed;
        }
        BossPatternStep::Rest { .. } => attack_state.clear(),
    }
}

fn scripted_step_ready_to_advance(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    current: &BossPatternStep,
    duration: f32,
) -> bool {
    let chance_per_second = cfg.macro_tuning.idle_attack_chance_per_second.max(0.0);
    if chance_per_second <= 0.0 || !matches!(current, BossPatternStep::Rest { .. }) {
        return true;
    }

    // The Rest duration is the minimum idle time. After that, an
    // optional per-second chance gates whether the boss starts the next
    // telegraph now or keeps waiting. This gives Smirking Behemoth an
    // "idle, then maybe eye-beam" feel without making every scripted
    // boss probabilistic.
    let chance_this_tick = (chance_per_second * ctx.dt.max(0.0)).clamp(0.0, 1.0);
    if chance_this_tick >= 1.0 || roll_boss_pattern_chance(cfg, state, chance_this_tick) {
        true
    } else {
        // Keep retrying the gate next tick without accumulating an
        // arbitrarily huge elapsed value.
        state.step_elapsed = duration;
        false
    }
}

fn roll_boss_pattern_chance(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    chance: f32,
) -> bool {
    if state.rng_seed == 0 {
        state.rng_seed = hash_boss_pattern_seed(&cfg.encounter_id)
            ^ 0x9E37_79B9_7F4A_7C15
            ^ ((state.step_index as u64) << 32);
    }
    state.rng_seed = state
        .rng_seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let n = (state.rng_seed >> 33) as u32;
    let unit = n as f32 / (u32::MAX >> 1) as f32;
    unit < chance.clamp(0.0, 1.0)
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

/// Cycle-mode (legacy rhythm) phase advancement. Picks the active
/// attack profile from `BossPatternStep`-less bosses by rotating
/// through their authored `attacks` list — see `BossRuntime::cycle_pattern_volumes`
/// for the rotation rule. The brain emits melee_pressed during the
/// Active phase; volume rendering still reads `pattern_timer` directly.
fn advance_cycle(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    attack_state: &mut BossAttackState,
) {
    if state.cycle_phase_remaining > 0.0 {
        state.cycle_phase_remaining = (state.cycle_phase_remaining - ctx.dt).max(0.0);
    }
    if state.cycle_phase_remaining <= 0.0 {
        state.cycle_phase = match state.cycle_phase {
            CyclePhase::Cooldown => CyclePhase::Windup,
            CyclePhase::Windup => CyclePhase::Active,
            CyclePhase::Active => CyclePhase::Cooldown,
        };
        state.cycle_phase_remaining = match state.cycle_phase {
            CyclePhase::Cooldown => cfg.cycle_attack_cooldown.max(0.05),
            CyclePhase::Windup => cfg.cycle_attack_windup.max(0.01),
            CyclePhase::Active => cfg.cycle_attack_active.max(0.01),
        };
    }

    // Pick the active profile from the cycle rotation. Matches the
    // historic `BossRuntime::cycle_pattern_volumes` math
    // `(pattern_timer / attack_cooldown).floor() % attacks.len()`
    // — preserved for parity. Cfg with an empty `cycle_attacks`
    // (defensively) falls back to the `full_body_pulse` strike.
    let profile = if cfg.cycle_attacks.is_empty() {
        BossAttackProfile::Strike("full_body_pulse".to_string())
    } else {
        let cooldown = cfg.cycle_attack_cooldown.max(0.05);
        let idx = ((state.pattern_timer / cooldown) as usize) % cfg.cycle_attacks.len();
        cfg.cycle_attacks[idx].clone()
    };
    match state.cycle_phase {
        CyclePhase::Windup => {
            // Cycle telegraph and strike share the same profile —
            // the legacy `attack_telegraph_volumes` and
            // `attack_volumes` both routed through
            // `cycle_pattern_volumes`. Mirror that here by writing
            // the rotation's current profile into telegraph_profile.
            let total = cfg.cycle_attack_windup.max(0.01);
            attack_state.telegraph_profile = Some(profile);
            attack_state.telegraph_remaining = state.cycle_phase_remaining;
            attack_state.telegraph_elapsed = (total - state.cycle_phase_remaining).max(0.0);
            attack_state.active_profile = None;
            attack_state.active_remaining = 0.0;
            attack_state.active_elapsed = 0.0;
        }
        CyclePhase::Active => {
            let total = cfg.cycle_attack_active.max(0.01);
            let windup_total = cfg.cycle_attack_windup.max(0.01);
            attack_state.telegraph_profile = None;
            attack_state.telegraph_remaining = 0.0;
            attack_state.telegraph_elapsed = 0.0;
            attack_state.active_profile = Some(profile);
            attack_state.active_remaining = state.cycle_phase_remaining;
            attack_state.active_elapsed =
                windup_total + (total - state.cycle_phase_remaining).max(0.0);
        }
        CyclePhase::Cooldown => attack_state.clear(),
    }
}

fn front_wall_standoff_reached(tuning: &BossMacroTuning, ctx: &BossPatternContext) -> bool {
    tuning.front_wall_standoff > 0.0
        && ctx
            .front_wall_clearance
            .is_some_and(|clearance| clearance <= tuning.front_wall_standoff + 1.0)
}

/// Advance the chase/engage/retreat macro state machine. Transitions:
///
/// - `Engage` → `Approach` if distance > too_far_distance, or in
///   contact-chase mode whenever the player is not yet horizontally
///   overlapping the boss.
/// - `Engage` → `Retreat` if distance < too_close_distance (anti-corner)
///   OR engage_timer >= engage_max_duration_s (periodic "preparing"
///   beat).
/// - `Approach` → `Engage` if distance < engage_distance, if contact-chase
///   mode has horizontally closed, or if the timer expired.
/// - `Retreat` → `Engage` if timer expired
///
/// Retreat picks `retreat_pos` along the player→boss axis (so the
/// boss visibly retreats *away* from the player rather than just
/// drifting toward an arbitrary anchor).
fn advance_macro_state(
    cfg: &BossPatternCfg,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
) {
    let movement = cfg.movement_for_phase(ctx.encounter_phase);
    // World-arena-lateral bosses reason about standoff on the authored arena lane only.
    // Otherwise a player jumping over/under the boss would look "far away"
    // and make the boss slide into them, even though the desired behavior is
    // YHTBTR-style left/right spacing with collision handling the walls.
    let distance = if movement.world_arena_lateral_only() {
        (ctx.target_pos.x - ctx.actor_pos.x).abs()
    } else {
        (ctx.target_pos - ctx.actor_pos).length()
    };
    let tuning = &cfg.macro_tuning;
    let front_wall_blocked = front_wall_standoff_reached(tuning, ctx);
    let contact_chase_mode = tuning.contact_chase_mode();
    let contact_chase_closed = contact_chase_mode && distance <= tuning.engage_distance.max(4.0);
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
        // For world-arena-lateral bosses, retreat is a
        // fixed-arena-lane desired velocity only. `BossRuntime::integrate_body`
        // still runs through `step_kinematic`, so solid walls and platforms
        // are the authority that stops the body if this target lies beyond
        // reachable floor.
        let target_x = ctx.actor_pos.x + dir_x * cfg.macro_tuning.retreat_distance.max(60.0);
        return ae::Vec2::new(target_x * 0.6 + cfg.spawn.x * 0.4, ctx.actor_pos.y);
    }

    let away = ctx.actor_pos - ctx.target_pos;
    let dir = if away.length_squared() < 1e-3 {
        ae::Vec2::new(1.0, 0.0)
    } else {
        away.normalize()
    };
    // Anchor near the boss spawn so retreat doesn't drift the boss
    // toward arena edges over many encounters. Blend the away-dir
    // with the spawn offset so the retreat curves back toward the
    // spawn anchor rather than off into a wall.
    let target = ctx.actor_pos + dir * cfg.macro_tuning.retreat_distance.max(60.0);
    target * 0.6 + cfg.spawn * 0.4
}

/// Movement-profile → frame.desired_vel translation. Runs even in
/// non-attacking phases so a dormant boss keeps its sway phase.
fn emit_desired_vel(
    cfg: &BossPatternCfg,
    state: &BossPatternState,
    ctx: &BossPatternContext,
    out: &mut crate::actor::control::ActorControlFrame,
    attack_state: &BossAttackState,
) {
    if ctx.dt <= 0.0 {
        return;
    }

    // Phase-aware movement: Phase 2 / Enrage may override the
    // default movement profile so a boss can escalate from a slow
    // anchored sway to a wide AirSwoop without growing the profile
    // enum.
    let movement = cfg.movement_for_phase(ctx.encounter_phase);
    // Macro state overrides the movement target: Approach chases
    // the player directly, Retreat heads toward the chosen retreat
    // anchor. `Engage` falls through to the normal sway/swoop
    // target. The speed scaling for Approach/Retreat is applied
    // farther down via `macro_speed_scale`.
    let mut target = match state.macro_state {
        BossMacroState::Approach { .. } => {
            // Bosses that author a `too_close_distance` keep the older
            // standoff-ring behavior. Contact-chase bosses disable the
            // too-close ring and author `engage_distance = 0`, which makes
            // the target the player's current x so collision/body contact
            // is the thing that stops the run-in.
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

    // While a strike is live, a self-dodging boss layers a horizontal dodge
    // on top of the baseline sway so it reads as stepping aside to avoid its
    // own experiment (GNU-ton weaving out of its apple rain).
    let self_dodge_active = matches!(cfg.movement, BossMovementProfile::StationaryGiant { .. })
        && cfg.self_dodge_amp > 0.0
        && ctx.encounter_phase.is_attacking();
    if self_dodge_active {
        // Cheap proxy for "is DebrisRain active right now?": we
        // can't tell from inside this fn without reading the
        // BossAttackState mirror; rely on the boss tick system to
        // have already populated state.movement_timer + the
        // tick_boss_pattern dispatch order so the sway oscillator
        // runs every tick regardless.
        let _ = state.movement_timer;
    }

    // Soft world-bounds clamp matches the previous BossRuntime
    // `build_control_frame` behavior so collision still owns the
    // hard stop but the brain doesn't ask to walk into it.
    let half = cfg.combat_size * 0.5;
    let margin = 8.0;
    let max_x = (ctx.world_size.x - half.x - margin).max(half.x + margin);
    let max_y = (ctx.world_size.y - half.y - margin).max(half.y + margin);
    let mut clamped_target = ae::Vec2::new(
        target.x.clamp(half.x + margin, max_x),
        target.y.clamp(half.y + margin, max_y),
    );
    if movement.world_arena_lateral_only() {
        // The profile declares no authored world-arena vertical travel, so do not let the
        // macro standoff/retreat steering add one. Preserve the
        // current integrated y so collision remains authoritative if
        // the boss was previously nudged by the world.
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
    // Scale speed during ANY active strike so the boss doesn't
    // outrun its own attack. Two reasons:
    //
    // 1. Specials anchor World-space hitboxes at the boss's pos
    //    (saddle cross, minima pit, cascade origin). Sliding
    //    sideways after the strike started would visually
    //    misalign the hazards from the boss.
    // 2. Melee FollowOwner hitboxes (FloorSlam, SideSweep, etc.)
    //    track the boss every tick. If the boss is chasing the
    //    player at `approach_speed_scale × movement.speed`
    //    during the 0.4 s Strike beat, a player who's still
    //    running outpaces the strike. Holding the boss roughly
    //    still during the active window lets the strike actually
    //    *land* — the player gets a real telegraph-and-dodge
    //    window instead of "the boss is moving so the strike
    //    follows them everywhere I run."
    //
    // The previous behavior (special-only scaling) made
    // Gradient-Sentinel-during-Approach feel like the boss never
    // attacked — it WAS attacking, but the melee strikes whiffed
    // because the boss kept chasing at 1.5× speed.
    let in_active_strike = attack_state.active_profile.is_some();
    // Macro-state speed scaling. Approach commits visually with
    // `> 1.0` speed; Retreat backs off deliberately with `< 1.0`.
    // Engage keeps the legacy speed (1.0).
    let macro_scale = match state.macro_state {
        BossMacroState::Approach { .. } => cfg.macro_tuning.approach_speed_scale.max(0.0),
        BossMacroState::Retreat { .. } => cfg.macro_tuning.retreat_speed_scale.max(0.0),
        BossMacroState::Engage => 1.0,
    };
    let strike_scale = if in_active_strike {
        cfg.strike_speed_scale.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let speed = movement.speed() * macro_scale * strike_scale;
    let max_step = speed * ctx.dt;
    out.velocity_target = if delta.length() > max_step && max_step > 0.0 {
        delta.normalize_or_zero() * speed
    } else if ctx.dt > 0.0 {
        delta / ctx.dt
    } else {
        ae::Vec2::ZERO
    };
}
