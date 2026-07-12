//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod phase_mechanism_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module (a direct
//! sibling, so `super` path depth is unchanged) with `use super::*;`.

use super::*;
use BossEncounterPhase::{Death, Dormant, Enrage, Intro, Phase1, Phase2};

/// A boss with NO triggers never phases up — it just fights until its HP
/// (owned elsewhere) reaches zero. This is "a boss reused as a plain enemy".
#[test]
fn empty_triggers_never_phase_up() {
    let mut state = ActorPhaseState::new(Vec::new());
    assert_eq!(state.start_phase, Phase1, "no intro tell ⇒ start fighting");
    assert_eq!(
        state.wake(),
        vec![BossPhaseEvent::PhaseChanged {
            from: Dormant,
            to: Phase1
        }]
    );
    // Tick a long time at low HP — nothing happens without a trigger.
    for _ in 0..600 {
        assert!(state.tick(1.0 / 60.0, 0.01).is_empty());
    }
    assert_eq!(state.phase, Phase1);
}

/// An HpBelow trigger fires when HP crosses its threshold, gated by `from`.
#[test]
fn hp_below_trigger_fires_when_threshold_crossed() {
    let mut state = ActorPhaseState::new(vec![PhaseTrigger::hp_below(0.5, Phase1, Phase2, 0.0)]);
    state.wake();
    assert_eq!(state.phase, Phase1);
    // Above threshold: no transition.
    assert!(state.tick(0.016, 0.8).is_empty());
    assert_eq!(state.phase, Phase1);
    // Cross it: swap to Phase2.
    let evs = state.tick(0.016, 0.4);
    assert_eq!(
        evs,
        vec![BossPhaseEvent::PhaseChanged {
            from: Phase1,
            to: Phase2
        }]
    );
    assert_eq!(state.phase, Phase2);
}

/// `from` gating: a Phase2→Enrage trigger must not fire while in Phase1.
#[test]
fn trigger_respects_from_phase_gate() {
    let mut state = ActorPhaseState::new(vec![
        PhaseTrigger::hp_below(0.6, Phase1, Phase2, 0.0),
        PhaseTrigger::hp_below(0.2, Phase2, Enrage, 0.0),
    ]);
    state.wake();
    // Drop HP straight past BOTH thresholds. Only Phase1→Phase2 should fire
    // this tick (the Enrage trigger is gated to `from: Phase2`).
    let evs = state.tick(0.016, 0.1);
    assert_eq!(
        evs,
        vec![BossPhaseEvent::PhaseChanged {
            from: Phase1,
            to: Phase2
        }]
    );
    // Next tick, now in Phase2, the Enrage trigger fires.
    let evs = state.tick(0.016, 0.1);
    assert_eq!(
        evs,
        vec![BossPhaseEvent::PhaseChanged {
            from: Phase2,
            to: Enrage
        }]
    );
}

/// A trigger with a `lock` inserts an invulnerable tell beat before the
/// swap — its own mechanism, not the swap itself.
#[test]
fn lock_inserts_invulnerable_tell_before_swap() {
    let mut state = ActorPhaseState::new(vec![PhaseTrigger::hp_below(0.5, Phase1, Phase2, 0.30)]);
    state.wake();
    assert!(!state.boss_invulnerable(), "Phase1 is vulnerable");
    // Crossing the threshold starts the lock, but does NOT swap yet.
    let evs = state.tick(0.016, 0.4);
    assert_eq!(
        evs,
        vec![BossPhaseEvent::TransitionLockStarted { to: Phase2 }]
    );
    assert_eq!(state.phase, Phase1, "phase holds during the tell");
    assert!(
        state.boss_invulnerable(),
        "boss is invulnerable during the tell"
    );
    // Tick out the lock; the swap lands on expiry.
    let mut swapped = false;
    for _ in 0..40 {
        let evs = state.tick(0.016, 0.4);
        if evs
            == vec![BossPhaseEvent::PhaseChanged {
                from: Phase1,
                to: Phase2,
            }]
        {
            swapped = true;
            break;
        }
    }
    assert!(swapped, "the tell beat expires into the new phase");
    assert_eq!(state.phase, Phase2);
    assert!(!state.boss_invulnerable());
}

/// Intro is opt-in DATA: a `TimeInPhase` trigger out of Intro makes the
/// boss start there and advance to Phase1 on its own.
#[test]
fn intro_is_opt_in_time_trigger() {
    let mut state =
        ActorPhaseState::new(vec![PhaseTrigger::time_in_phase(0.5, Intro, Phase1, 0.0)]);
    assert_eq!(state.start_phase, Intro);
    state.wake();
    assert_eq!(state.phase, Intro);
    // Before the timer: still Intro.
    assert!(state.tick(0.1, 1.0).is_empty());
    assert_eq!(state.phase, Intro);
    // After: advance to Phase1.
    let mut advanced = false;
    for _ in 0..10 {
        if state.tick(0.1, 1.0)
            == vec![BossPhaseEvent::PhaseChanged {
                from: Intro,
                to: Phase1,
            }]
        {
            advanced = true;
            break;
        }
    }
    assert!(advanced);
    assert_eq!(state.phase, Phase1);
}

/// External triggers fire via `notify_external`, never from `tick`.
#[test]
fn external_trigger_only_fires_on_notify() {
    let mut state = ActorPhaseState::new(vec![PhaseTrigger::external(
        "all_adds_dead",
        Phase1,
        Enrage,
        0.0,
    )]);
    state.wake();
    // Ticking does nothing — external triggers don't auto-fire.
    assert!(state.tick(1.0, 0.01).is_empty());
    assert_eq!(state.phase, Phase1);
    // The wrong gate is ignored.
    assert!(state.notify_external("some_other_gate").is_empty());
    assert_eq!(state.phase, Phase1);
    // The right gate fires it.
    let evs = state.notify_external("all_adds_dead");
    assert_eq!(
        evs,
        vec![BossPhaseEvent::PhaseChanged {
            from: Phase1,
            to: Enrage
        }]
    );
    assert_eq!(state.phase, Enrage);
}

/// Death is terminal: no trigger fires once dead.
#[test]
fn death_is_terminal() {
    let mut state = ActorPhaseState::new(vec![PhaseTrigger::hp_below(0.9, Phase1, Phase2, 0.0)]);
    state.wake();
    state.phase = Death;
    assert!(state.tick(1.0, 0.0).is_empty());
    assert!(state.notify_external("anything").is_empty());
    assert_eq!(state.phase, Death);
}

/// The intrinsic derivation reproduces a legacy spec's phase graph as data:
/// opt-in intro, Phase1→Phase2 (with the transition tell), Phase2→Enrage.
#[test]
fn intrinsic_from_spec_models_the_legacy_graph() {
    let mut spec = legacy_spec();
    spec.intro_seconds = 2.0;
    spec.transition_seconds = 1.0;
    spec.phase1_to_transition_hp = 0.66;
    spec.phase2_to_enrage_hp = 0.20;
    let triggers = PhaseTrigger::intrinsic_from_spec(&spec);
    assert_eq!(triggers.len(), 3, "intro + phase1→2 + phase2→enrage");
    assert_eq!(
        triggers[0],
        PhaseTrigger::time_in_phase(2.0, Intro, Phase1, 0.0)
    );
    assert_eq!(
        triggers[1],
        PhaseTrigger::hp_below(0.66, Phase1, Phase2, 1.0)
    );
    assert_eq!(
        triggers[2],
        PhaseTrigger::hp_below(0.20, Phase2, Enrage, 0.0)
    );
    // A spec with no intro beat drops the intro trigger (no forced intro).
    spec.intro_seconds = 0.0;
    assert_eq!(PhaseTrigger::intrinsic_from_spec(&spec).len(), 2);
}

fn legacy_spec() -> BossEncounterSpec {
    BossEncounterSpec {
        id: "test".into(),
        name: "Test".into(),
        max_hp: 100,
        phase1_to_transition_hp: 0.66,
        transition_to_phase2_hp: 0.66,
        phase2_to_enrage_hp: 0.20,
        intro_seconds: 2.0,
        transition_seconds: 1.0,
        stagger_seconds: 1.0,
        death_seconds: 1.0,
        stagger_threshold: 9999,
        stagger_window_seconds: 1.0,
        music_intro: String::new(),
        music_phase1: String::new(),
        music_phase2: String::new(),
        music_enrage: String::new(),
        extra_phase_triggers: Vec::new(),
    }
}
