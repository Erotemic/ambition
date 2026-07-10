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
mod tests {
    use super::*;
    use crate::brain::boss_pattern::BossAttackProfile;
    use ambition_engine_core as ae;

    fn ctx() -> BossPatternContext {
        BossPatternContext {
            actor_pos: ae::Vec2::ZERO,
            target_pos: ae::Vec2::new(100.0, 0.0),
            actor_facing: 1.0,
            hp_current: 100,
            hp_max: 100,
            dt: 1.0 / 60.0,
            ..Default::default()
        }
    }

    fn strike(id: &str, duration: f32) -> BossPatternStep {
        BossPatternStep::Strike {
            profile: BossAttackProfile::Strike(id.to_string()),
            duration,
        }
    }

    fn arm(weight: f32, when: Option<SituationBucket>, id: &str) -> WeightedArm {
        WeightedArm {
            weight,
            when,
            steps: vec![strike(id, 0.5)],
        }
    }

    /// A fixed sequence of draws, so a `Select` test asserts on the ARM, not on
    /// the LCG. The RNG's own determinism is the ticker's business.
    fn draws(values: &[f32]) -> impl FnMut() -> f32 + '_ {
        let mut i = 0;
        move || {
            let v = values[i.min(values.len() - 1)];
            i += 1;
            v
        }
    }

    // ── buckets ──────────────────────────────────────────────────────────────

    #[test]
    fn near_and_far_split_at_the_authored_radius() {
        let mut c = ctx();
        c.target_pos = ae::Vec2::new(PLAYER_NEAR_PX - 1.0, 0.0);
        assert!(bucket_holds(SituationBucket::PlayerNear, &c));
        assert!(!bucket_holds(SituationBucket::PlayerFar, &c));

        c.target_pos = ae::Vec2::new(PLAYER_NEAR_PX + 1.0, 0.0);
        assert!(!bucket_holds(SituationBucket::PlayerNear, &c));
        assert!(bucket_holds(SituationBucket::PlayerFar, &c));

        // Exactly on the radius reads as NEAR — the boundary belongs to the arm
        // that can reach, so a sweep authored at its own reach never whiffs by a
        // float.
        c.target_pos = ae::Vec2::new(PLAYER_NEAR_PX, 0.0);
        assert!(bucket_holds(SituationBucket::PlayerNear, &c));
    }

    /// `+y` is down. A player ABOVE the boss is at a negative delta.
    #[test]
    fn above_respects_the_engines_downward_y() {
        let mut c = ctx();
        c.target_pos = ae::Vec2::new(0.0, -50.0);
        assert!(bucket_holds(SituationBucket::PlayerAbove, &c));
        c.target_pos = ae::Vec2::new(0.0, 50.0);
        assert!(!bucket_holds(SituationBucket::PlayerAbove, &c));
    }

    /// A boss with no facing opinion has no back to be behind.
    #[test]
    fn behind_needs_a_facing_to_be_behind_of() {
        let mut c = ctx();
        c.target_pos = ae::Vec2::new(-100.0, 0.0);
        c.actor_facing = 1.0;
        assert!(bucket_holds(SituationBucket::PlayerBehind, &c));
        c.actor_facing = -1.0;
        assert!(!bucket_holds(SituationBucket::PlayerBehind, &c));
        c.actor_facing = 0.0;
        assert!(!bucket_holds(SituationBucket::PlayerBehind, &c));
    }

    #[test]
    fn hp_below_reads_the_fraction_and_an_unknown_pool_is_full_health() {
        let mut c = ctx();
        c.hp_current = 30;
        assert!(bucket_holds(SituationBucket::HpBelow(0.5), &c));
        assert!(!bucket_holds(SituationBucket::HpBelow(0.2), &c));
        c.hp_max = 0;
        assert!(
            !bucket_holds(SituationBucket::HpBelow(0.99), &c),
            "a context that never learned the pool must not trip an enrage"
        );
    }

    // ── Select ───────────────────────────────────────────────────────────────

    /// The weighted roll walks the eligible arms in authored order, so a table's
    /// priority is readable off the RON.
    #[test]
    fn the_weighted_roll_partitions_the_unit_interval_in_authored_order() {
        let c = ctx();
        let table = vec![arm(1.0, None, "a"), arm(3.0, None, "b")];
        // total = 4; `a` owns [0, 0.25), `b` owns [0.25, 1).
        assert_eq!(
            pick_arm(&table, &c, 0.0).unwrap().steps,
            vec![strike("a", 0.5)]
        );
        assert_eq!(
            pick_arm(&table, &c, 0.24).unwrap().steps,
            vec![strike("a", 0.5)]
        );
        assert_eq!(
            pick_arm(&table, &c, 0.26).unwrap().steps,
            vec![strike("b", 0.5)]
        );
        assert_eq!(
            pick_arm(&table, &c, 0.999).unwrap().steps,
            vec![strike("b", 0.5)]
        );
    }

    /// An ineligible arm is not in the denominator. Without this, a table that is
    /// half far-range arms would silently under-weight its near-range ones the
    /// moment the player closed in — the exact bug a "roll then filter" order has.
    #[test]
    fn an_ineligible_arm_leaves_the_denominator_too() {
        let mut c = ctx();
        c.target_pos = ae::Vec2::new(10.0, 0.0); // near
        let table = vec![
            arm(1.0, Some(SituationBucket::PlayerFar), "far"),
            arm(1.0, Some(SituationBucket::PlayerNear), "near"),
        ];
        // `far` is out, so `near` owns the WHOLE interval.
        for unit in [0.0, 0.5, 0.99] {
            assert_eq!(
                pick_arm(&table, &c, unit).unwrap().steps,
                vec![strike("near", 0.5)]
            );
        }
    }

    /// A table with nothing eligible resolves to zero beats — a legal, deliberate
    /// "do nothing in this situation", not a panic and not a fallback.
    #[test]
    fn a_table_with_no_eligible_arm_resolves_to_nothing() {
        let mut c = ctx();
        c.target_pos = ae::Vec2::new(10.0, 0.0); // near
        let table = vec![arm(1.0, Some(SituationBucket::PlayerFar), "far")];
        assert!(pick_arm(&table, &c, 0.5).is_none());

        let steps = vec![BossPatternStep::Select { table }];
        assert!(resolve_timeline(&steps, &c, &mut draws(&[0.5])).is_empty());
    }

    #[test]
    fn a_zero_or_negative_weight_never_wins() {
        let c = ctx();
        let table = vec![arm(0.0, None, "never"), arm(-3.0, None, "also_never")];
        assert!(pick_arm(&table, &c, 0.5).is_none());
    }

    /// Resolution splices the winning arm IN PLACE, keeping the beats around it.
    #[test]
    fn resolution_splices_the_winning_arm_between_its_neighbours() {
        let c = ctx();
        let steps = vec![
            strike("before", 0.1),
            BossPatternStep::Select {
                table: vec![arm(1.0, None, "chosen")],
            },
            strike("after", 0.2),
        ];
        let out = resolve_timeline(&steps, &c, &mut draws(&[0.5]));
        assert_eq!(
            out,
            vec![
                strike("before", 0.1),
                strike("chosen", 0.5),
                strike("after", 0.2)
            ]
        );
        assert!(
            !out.iter()
                .any(|s| matches!(s, BossPatternStep::Select { .. })),
            "a resolved timeline contains no Select: the ticker never sees one"
        );
    }

    /// The RNG stream advances once per `Select` **whether or not an arm wins**, so
    /// two bosses that diverge in position stay in lockstep on the stream itself.
    #[test]
    fn a_select_consumes_one_draw_even_when_nothing_is_eligible() {
        let mut c = ctx();
        c.target_pos = ae::Vec2::new(10.0, 0.0); // near: the far arm is out
        let mut consumed = 0;
        let mut draw = || {
            consumed += 1;
            0.5
        };
        let steps = vec![
            BossPatternStep::Select {
                table: vec![arm(1.0, Some(SituationBucket::PlayerFar), "far")],
            },
            BossPatternStep::Select {
                table: vec![arm(1.0, None, "always")],
            },
        ];
        let out = resolve_timeline(&steps, &c, &mut draw);
        assert_eq!(out, vec![strike("always", 0.5)]);
        assert_eq!(consumed, 2, "one draw per Select, eligible or not");
    }

    /// Nested `Select`s resolve depth-first, and the depth limit stops a
    /// self-referencing table from hanging the sim rather than pretending it is
    /// authored correctly.
    #[test]
    fn nested_selects_resolve_depth_first_and_bottom_out() {
        let c = ctx();
        let inner = BossPatternStep::Select {
            table: vec![WeightedArm {
                weight: 1.0,
                when: None,
                steps: vec![strike("inner", 0.3)],
            }],
        };
        let outer = BossPatternStep::Select {
            table: vec![WeightedArm {
                weight: 1.0,
                when: None,
                steps: vec![strike("outer", 0.1), inner],
            }],
        };
        let out = resolve_timeline(&[outer], &c, &mut draws(&[0.5, 0.5]));
        assert_eq!(out, vec![strike("outer", 0.1), strike("inner", 0.3)]);
    }

    #[test]
    fn a_self_referencing_select_terminates_at_the_depth_limit() {
        let c = ctx();
        // Build a Select whose arm contains a Select whose arm contains ... 8 deep.
        let mut step = strike("bottom", 0.1);
        for _ in 0..8 {
            step = BossPatternStep::Select {
                table: vec![WeightedArm {
                    weight: 1.0,
                    when: None,
                    steps: vec![step],
                }],
            };
        }
        let out = resolve_timeline(&[step], &c, &mut draws(&[0.5]));
        assert!(
            out.is_empty(),
            "past MAX_RESOLVE_DEPTH the tail is dropped, not recursed: {out:?}"
        );
    }

    // ── stances ──────────────────────────────────────────────────────────────

    fn pattern_with_stance() -> BossPattern {
        let mut stances = std::collections::BTreeMap::new();
        stances.insert("panic".to_string(), vec![strike("panic_swipe", 0.4)]);
        BossPattern {
            steps: vec![strike("normal", 1.0)],
            stances,
            interrupts: Vec::new(),
        }
    }

    #[test]
    fn entering_a_stance_saves_the_cursor_and_leaving_restores_it_exactly() {
        let c = ctx();
        let pattern = pattern_with_stance();
        let mut state = BossPatternState {
            timeline: vec![strike("normal", 1.0)],
            step_index: 0,
            step_elapsed: 0.37,
            ..Default::default()
        };

        assert!(enter_stance(
            &pattern,
            &mut state,
            &c,
            "panic",
            (0, 0.37),
            &mut draws(&[0.5])
        ));
        assert_eq!(state.stance.as_deref(), Some("panic"));
        assert_eq!(state.timeline, vec![strike("panic_swipe", 0.4)]);
        assert_eq!(state.step_index, 0);
        assert_eq!(state.step_elapsed, 0.0);

        assert!(leave_stance(&mut state));
        assert_eq!(state.stance, None);
        assert_eq!(state.timeline, vec![strike("normal", 1.0)]);
        assert_eq!(
            (state.step_index, state.step_elapsed),
            (0, 0.37),
            "an interrupt resumes the telegraph it stole, elapsed and all"
        );
        assert!(!leave_stance(&mut state), "nothing left to pop");
    }

    /// An unknown stance id is a no-op, not a panic. BD5 rejects it at install
    /// time; a fight already running must not die of a typo.
    #[test]
    fn an_unknown_or_empty_stance_is_a_no_op() {
        let c = ctx();
        let mut pattern = pattern_with_stance();
        pattern.stances.insert("hollow".to_string(), Vec::new());
        let mut state = BossPatternState {
            timeline: vec![strike("normal", 1.0)],
            ..Default::default()
        };
        assert!(!enter_stance(
            &pattern,
            &mut state,
            &c,
            "nope",
            (0, 0.0),
            &mut draws(&[0.5])
        ));
        assert!(!enter_stance(
            &pattern,
            &mut state,
            &c,
            "hollow",
            (0, 0.0),
            &mut draws(&[0.5])
        ));
        assert!(state.stance_stack.is_empty());
        assert_eq!(state.timeline, vec![strike("normal", 1.0)]);
    }

    /// A stance may enter a stance. The stack is why the way home survives.
    #[test]
    fn stances_nest_and_unwind_in_order() {
        let c = ctx();
        let mut pattern = pattern_with_stance();
        pattern
            .stances
            .insert("deeper".to_string(), vec![strike("deep", 0.2)]);
        let mut state = BossPatternState {
            timeline: vec![strike("normal", 1.0)],
            ..Default::default()
        };
        enter_stance(
            &pattern,
            &mut state,
            &c,
            "panic",
            (1, 0.0),
            &mut draws(&[0.5]),
        );
        enter_stance(
            &pattern,
            &mut state,
            &c,
            "deeper",
            (0, 0.1),
            &mut draws(&[0.5]),
        );
        assert_eq!(state.stance.as_deref(), Some("deeper"));
        assert_eq!(state.stance_stack.len(), 2);

        leave_stance(&mut state);
        assert_eq!(state.stance.as_deref(), Some("panic"));
        leave_stance(&mut state);
        assert_eq!(state.stance, None);
        assert_eq!((state.step_index, state.step_elapsed), (1, 0.0));
    }

    // ── interrupts ───────────────────────────────────────────────────────────

    fn rule(on: InterruptTrigger, cooldown_s: f32) -> InterruptRule {
        InterruptRule {
            on,
            cooldown_s,
            enter: "panic".to_string(),
        }
    }

    #[test]
    fn on_hit_taken_fires_at_or_above_its_threshold_and_respects_its_cooldown() {
        let c = ctx();
        let rules = vec![rule(InterruptTrigger::OnHitTaken { min_damage: 5 }, 1.0)];
        let mut s = BossPatternState::default();

        assert_eq!(tick_interrupts(&rules, &mut s, &c, None, 4), None, "4 < 5");
        assert_eq!(tick_interrupts(&rules, &mut s, &c, None, 5), Some(0));
        assert_eq!(
            tick_interrupts(&rules, &mut s, &c, None, 99),
            None,
            "still on cooldown"
        );
        // Burn the cooldown down.
        let long = BossPatternContext { dt: 1.0, ..c };
        assert_eq!(tick_interrupts(&rules, &mut s, &long, None, 0), None);
        assert_eq!(tick_interrupts(&rules, &mut s, &c, None, 5), Some(0));
    }

    /// A `min_damage` of zero still means "a hit", not "every tick".
    #[test]
    fn on_hit_taken_never_fires_on_zero_damage() {
        let c = ctx();
        let rules = vec![rule(InterruptTrigger::OnHitTaken { min_damage: 0 }, 0.0)];
        let mut s = BossPatternState::default();
        assert_eq!(tick_interrupts(&rules, &mut s, &c, None, 0), None);
        assert_eq!(tick_interrupts(&rules, &mut s, &c, None, 1), Some(0));
    }

    #[test]
    fn on_phase_enter_fires_on_the_rising_edge_only() {
        let c = ctx();
        let rules = vec![rule(
            InterruptTrigger::OnPhaseEnter {
                phase: BossEncounterPhase::Enrage,
            },
            0.0,
        )];
        let mut s = BossPatternState::default();
        assert_eq!(tick_interrupts(&rules, &mut s, &c, None, 0), None);
        assert_eq!(
            tick_interrupts(&rules, &mut s, &c, Some(BossEncounterPhase::Phase2), 0),
            None,
            "a different phase is a different rule"
        );
        assert_eq!(
            tick_interrupts(&rules, &mut s, &c, Some(BossEncounterPhase::Enrage), 0),
            Some(0)
        );
        assert_eq!(tick_interrupts(&rules, &mut s, &c, None, 0), None);
    }

    /// **The trap this rule exists to avoid.** A 1s timer behind a 5s cooldown must
    /// not bank five firings and spend them all at t=5. The accumulator resets when
    /// the trigger CONDITION holds, not when the interrupt is allowed to fire.
    #[test]
    fn a_timer_behind_a_long_cooldown_does_not_bank_its_firings() {
        let c = BossPatternContext { dt: 0.5, ..ctx() };
        let rules = vec![rule(InterruptTrigger::OnTimer { every_s: 1.0 }, 5.0)];
        let mut s = BossPatternState::default();

        let mut fires = 0;
        // 20 ticks of 0.5s = 10 seconds. The timer wants to fire 10×; the cooldown
        // allows one at t=1 and one at t=6.
        for _ in 0..20 {
            if tick_interrupts(&rules, &mut s, &c, None, 0).is_some() {
                fires += 1;
            }
        }
        assert_eq!(fires, 2, "one at t=1, one after the 5s cooldown expires");
    }

    /// Only ONE rule fires per tick, and it is the first in authored order — so a
    /// fight's interrupt priority is readable off the RON rather than emergent.
    #[test]
    fn at_most_one_rule_fires_per_tick_and_authored_order_is_the_priority() {
        let c = ctx();
        let rules = vec![
            rule(InterruptTrigger::OnHitTaken { min_damage: 1 }, 0.0),
            rule(InterruptTrigger::OnHitTaken { min_damage: 1 }, 0.0),
        ];
        let mut s = BossPatternState::default();
        assert_eq!(tick_interrupts(&rules, &mut s, &c, None, 10), Some(0));
    }

    #[test]
    fn a_zero_period_timer_never_fires() {
        let c = ctx();
        let rules = vec![rule(InterruptTrigger::OnTimer { every_s: 0.0 }, 0.0)];
        let mut s = BossPatternState::default();
        for _ in 0..600 {
            assert_eq!(tick_interrupts(&rules, &mut s, &c, None, 0), None);
        }
    }
}
