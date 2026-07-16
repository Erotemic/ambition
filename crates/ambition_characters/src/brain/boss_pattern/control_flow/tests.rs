//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

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

/// An unknown stance id is a no-op, not a panic. BD5 flags it as a diagnostic
/// finding; a fight already running must not die of a typo.
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
    let long = BossPatternContext {
        dt: 1.0,
        ..c.clone()
    };
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
