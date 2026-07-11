//! **BD1 — the three authored-logic atoms, as pure functions.**
//!
//! `docs/planning/engine/boss-design.md` §1: *"today's `BossPattern` sequencing
//! covers timed/scripted beats; add the three authored-logic atoms fights keep
//! wanting: conditional selection, interrupts, and stances. No scripting
//! language — three enum arms."*
//!
//! Everything here is a pure function of `(pattern, state, context)`. The ticker
//! in [`super::tick`] calls it; nothing here touches Bevy, and every rule is a
//! unit test rather than a play session.
//!
//! ## Resolution, not interpretation
//!
//! A `Select` is rolled **when the timeline is resolved**, not when the cursor
//! reaches it. Resolution happens on phase change, on stance enter/leave, and
//! each time the cursor loops — so for a looping script, "roll once when reached"
//! and "roll once per pass" are the same thing.
//!
//! Two reasons it is done this way. The ticker's cursor arithmetic advances by
//! step DURATION, and a zero-duration step at the cursor is a foot-gun (an
//! unbounded advance loop is one authoring mistake away). And BD5's validator
//! wants to integrate a pass's total threat, which it cannot do against a step
//! that means "and then, maybe, some other steps."
//!
//! ## Determinism
//!
//! `Select` rolls off the boss's existing seeded RNG stream (`rng_seed`), the
//! same one the idle-attack gate uses. Interrupt bookkeeping is index-parallel to
//! the rule list, and stances live in a `BTreeMap` — nothing here iterates a hash
//! container (ADR 0023).

use super::{
    step_duration, BossEncounterPhase, BossPattern, BossPatternContext, BossPatternState,
    BossPatternStep, InterruptRule, InterruptTrigger, SituationBucket, StanceReturn, WeightedArm,
    PLAYER_NEAR_PX,
};

/// How deep a `Select` arm may nest another `Select`/`Stance` before resolution
/// stops recursing. Four is far past anything readable; the limit exists so a
/// self-referencing table cannot hang the sim, not because five would be wrong.
const MAX_RESOLVE_DEPTH: u8 = 4;

/// Does this bucket hold for the boss this tick? Pure over the context the boss
/// brain already has — no private queries, the same discipline the fighter brain's
/// no-cheat contract imposes.
pub fn bucket_holds(bucket: SituationBucket, ctx: &BossPatternContext) -> bool {
    let to_target = ctx.target_pos - ctx.actor_pos;
    match bucket {
        SituationBucket::PlayerNear => to_target.length() <= PLAYER_NEAR_PX,
        SituationBucket::PlayerFar => to_target.length() > PLAYER_NEAR_PX,
        // `+y` is down, so "above" is a negative delta.
        SituationBucket::PlayerAbove => to_target.y < 0.0,
        // A boss with no facing opinion (`0.0`) is never flanked: it has no back.
        SituationBucket::PlayerBehind => {
            ctx.actor_facing != 0.0 && to_target.x * ctx.actor_facing < 0.0
        }
        SituationBucket::HpBelow(frac) => ctx.hp_frac() < frac,
    }
}

/// The arms of `table` that may win this tick.
fn eligible<'a>(table: &'a [WeightedArm], ctx: &BossPatternContext) -> Vec<&'a WeightedArm> {
    table
        .iter()
        .filter(|arm| arm.weight > 0.0)
        .filter(|arm| arm.when.is_none_or(|b| bucket_holds(b, ctx)))
        .collect()
}

/// Pick one arm by weight from the eligible set, consuming one draw of `unit`
/// (a value in `[0, 1)`).
///
/// `None` when nothing is eligible — a legal, deliberate "do nothing here", and
/// the reason an authoring agent can write a table that is silent at long range
/// without also writing an empty arm.
pub fn pick_arm<'a>(
    table: &'a [WeightedArm],
    ctx: &BossPatternContext,
    unit: f32,
) -> Option<&'a WeightedArm> {
    let arms = eligible(table, ctx);
    let total: f32 = arms.iter().map(|a| a.weight).sum();
    if arms.is_empty() || total <= 0.0 {
        return None;
    }
    // `unit` is drawn from `[0,1)`; clamp defensively so a 1.0 cannot fall off
    // the end of the table and silently pick nothing.
    let mut target = (unit.clamp(0.0, 0.999_999) * total).min(total);
    for arm in &arms {
        target -= arm.weight;
        if target < 0.0 {
            return Some(arm);
        }
    }
    arms.last().copied()
}

/// Turn an authored step list into the concrete timeline the cursor walks: every
/// `Select` rolled away, `Stance` markers left in place as jumps.
///
/// `draw` yields one uniform in `[0, 1)` per `Select` encountered, in
/// depth-first order, so the caller's RNG stream advances deterministically.
pub fn resolve_timeline(
    steps: &[BossPatternStep],
    ctx: &BossPatternContext,
    draw: &mut impl FnMut() -> f32,
) -> Vec<BossPatternStep> {
    let mut out = Vec::with_capacity(steps.len());
    resolve_into(steps, ctx, draw, 0, &mut out);
    out
}

fn resolve_into(
    steps: &[BossPatternStep],
    ctx: &BossPatternContext,
    draw: &mut impl FnMut() -> f32,
    depth: u8,
    out: &mut Vec<BossPatternStep>,
) {
    for step in steps {
        match step {
            BossPatternStep::Select { table } => {
                // The draw happens whether or not an arm wins, so the RNG stream
                // does not depend on which buckets held — two bosses that diverge
                // in position stay in lockstep on the stream itself.
                let unit = draw();
                if depth >= MAX_RESOLVE_DEPTH {
                    continue;
                }
                if let Some(arm) = pick_arm(table, ctx, unit) {
                    resolve_into(&arm.steps, ctx, draw, depth + 1, out);
                }
            }
            other => out.push(other.clone()),
        }
    }
}

/// The rule that should fire this tick, as an index into `pattern.interrupts`.
///
/// Ticks every rule's cooldown and `OnTimer` accumulator by `dt` first, so this
/// is the single place interrupt time advances. At most one rule fires per tick —
/// the FIRST eligible one in authored order, which makes the priority visible in
/// the RON rather than emergent.
///
/// `phase_entered` is the rising edge the caller detects (`state.last_phase`
/// differs from `ctx.encounter_phase`), because by the time this runs the ticker
/// has already had to know.
pub fn tick_interrupts(
    interrupts: &[InterruptRule],
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    phase_entered: Option<BossEncounterPhase>,
    damage_taken: i32,
) -> Option<usize> {
    state.interrupt_cooldowns.resize(interrupts.len(), 0.0);
    state.interrupt_timers.resize(interrupts.len(), 0.0);

    let mut fired = None;
    for (i, rule) in interrupts.iter().enumerate() {
        state.interrupt_cooldowns[i] = (state.interrupt_cooldowns[i] - ctx.dt).max(0.0);
        if let InterruptTrigger::OnTimer { every_s } = rule.on {
            if every_s > 0.0 {
                state.interrupt_timers[i] += ctx.dt;
            }
        }

        let triggered = match rule.on {
            InterruptTrigger::OnHitTaken { min_damage } => damage_taken >= min_damage.max(1),
            InterruptTrigger::OnPhaseEnter { phase } => phase_entered == Some(phase),
            InterruptTrigger::OnTimer { every_s } => {
                every_s > 0.0 && state.interrupt_timers[i] >= every_s
            }
        };
        if !triggered {
            continue;
        }
        // The timer resets on TRIGGER, not on fire: a rule whose cooldown swallows
        // its own tick must not then fire immediately on the next one. Otherwise a
        // 1s timer behind a 5s cooldown would fire five times in a row at t=5.
        if let InterruptTrigger::OnTimer { every_s } = rule.on {
            state.interrupt_timers[i] -= every_s;
        }
        if state.interrupt_cooldowns[i] > 0.0 || fired.is_some() {
            continue;
        }
        state.interrupt_cooldowns[i] = rule.cooldown_s.max(0.0);
        fired = Some(i);
    }
    fired
}

/// Enter `stance_id`, saving where to come back to. No-op (returns `false`) when
/// the pattern has no such stance — an authored typo must not panic mid-fight; it
/// is BD5's job to reject it at install time.
///
/// `resume_at` is the step to return to. A `Stance` step passes the step AFTER
/// itself (the marker is consumed); an INTERRUPT passes the current step and its
/// elapsed, so a boss yanked out of a telegraph resumes that telegraph rather
/// than restarting it — the punish window the player was already reading stays
/// where it was.
pub fn enter_stance(
    pattern: &BossPattern,
    state: &mut BossPatternState,
    ctx: &BossPatternContext,
    stance_id: &str,
    resume_at: (usize, f32),
    draw: &mut impl FnMut() -> f32,
) -> bool {
    let Some(steps) = pattern.stances.get(stance_id) else {
        return false;
    };
    let resolved = resolve_timeline(steps, ctx, draw);
    if resolved.is_empty() {
        return false;
    }
    state.stance_stack.push(StanceReturn {
        timeline: std::mem::take(&mut state.timeline),
        stance: state.stance.take(),
        step_index: resume_at.0,
        step_elapsed: resume_at.1,
    });
    state.timeline = resolved;
    state.stance = Some(stance_id.to_string());
    state.step_index = 0;
    state.step_elapsed = 0.0;
    true
}

/// Leave the current stance, restoring the saved cursor. `false` when there is
/// nothing to pop (we are at the phase's own timeline).
pub fn leave_stance(state: &mut BossPatternState) -> bool {
    let Some(back) = state.stance_stack.pop() else {
        return false;
    };
    state.timeline = back.timeline;
    state.stance = back.stance;
    state.step_index = back.step_index;
    state.step_elapsed = back.step_elapsed;
    true
}

/// Total authored time of a resolved timeline. `0.0` for a timeline of pure
/// control flow — which is how the ticker knows a stance is a no-op rather than a
/// zero-length loop to spin on.
pub fn timeline_duration(timeline: &[BossPatternStep]) -> f32 {
    timeline.iter().map(step_duration).sum()
}

#[cfg(test)]
mod tests;
