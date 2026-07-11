//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

/// The whole model is a fixed-size table. `Situation × Choice` and nothing else,
/// so there is no history to prune and no unbounded growth to fear.
#[test]
fn the_model_is_bounded_by_the_product_of_two_closed_enums() {
    let mut m = HabitModel::new(0.9);
    for _ in 0..10_000 {
        for s in [
            Situation::Neutral,
            Situation::Advantage,
            Situation::EdgeGuard,
            Situation::Disadvantage,
            Situation::Recovery,
        ] {
            for c in Choice::ALL {
                m.observe(s, c);
            }
        }
    }
    assert_eq!(m.len(), 5 * Choice::ALL.len());
}

/// **Ignorance is not knowledge of absence.** An unseen situation reads as the
/// uniform prior, not as zero — otherwise a level-9 brain would conclude that
/// its opponent will never shield out of a juggle, on the evidence of never
/// having juggled them.
#[test]
fn an_unseen_situation_reads_as_the_uniform_prior() {
    let m = HabitModel::new(0.9);
    let uniform = 1.0 / Choice::ALL.len() as f32;
    for c in Choice::ALL {
        assert_eq!(m.frequency(Situation::Recovery, c), uniform);
    }
    assert!(m.read(Situation::Recovery).is_none());
    assert_eq!(m.read_bonus(Situation::Recovery, Choice::Jump, 1.0), 0.0);
}

/// **Decay is what makes it a read rather than a census.** An opponent who
/// spot-dodged nine times and then stopped is not a spot-dodger.
#[test]
fn a_habit_fades_when_the_opponent_changes_their_mind() {
    let mut m = HabitModel::new(0.5);
    for _ in 0..9 {
        m.observe(Situation::Disadvantage, Choice::Shield);
    }
    assert_eq!(m.read(Situation::Disadvantage).unwrap().0, Choice::Shield);

    // They stop shielding and start jumping. It takes a handful of observations,
    // not nine, because the old evidence is halved each time.
    for _ in 0..3 {
        m.observe(Situation::Disadvantage, Choice::Jump);
    }
    assert_eq!(
        m.read(Situation::Disadvantage).unwrap().0,
        Choice::Jump,
        "three fresh jumps outweigh nine stale shields at decay 0.5"
    );
}

/// `decay = 1.0` never forgets, and is therefore a census, not a read. Pinned so
/// the difference is a number a profile chooses rather than a hidden constant.
#[test]
fn a_decay_of_one_never_forgets() {
    let mut m = HabitModel::new(1.0);
    for _ in 0..9 {
        m.observe(Situation::Disadvantage, Choice::Shield);
    }
    for _ in 0..3 {
        m.observe(Situation::Disadvantage, Choice::Jump);
    }
    assert_eq!(m.read(Situation::Disadvantage).unwrap().0, Choice::Shield);
    assert_eq!(m.count(Situation::Disadvantage, Choice::Shield), 9.0);
}

/// Observing one situation does not decay another. Being edge-guarded rarely
/// does not make what they do there any less known.
#[test]
fn observing_one_situation_leaves_the_others_alone() {
    let mut m = HabitModel::new(0.5);
    m.observe(Situation::EdgeGuard, Choice::Attack);
    let before = m.count(Situation::EdgeGuard, Choice::Attack);
    for _ in 0..20 {
        m.observe(Situation::Neutral, Choice::Wait);
    }
    assert_eq!(m.count(Situation::EdgeGuard, Choice::Attack), before);
}

/// `read_weight` is the whole of §1's *"Level-9 reads = sampling the model;
/// lower levels ignore it."* A level-1 profile passes zero and the model, however
/// confident, contributes nothing.
#[test]
fn a_brain_that_does_not_read_gets_nothing_from_a_confident_model() {
    let mut m = HabitModel::new(0.9);
    for _ in 0..50 {
        m.observe(Situation::Neutral, Choice::Approach);
    }
    assert!(m.frequency(Situation::Neutral, Choice::Approach) > 0.9);
    assert_eq!(m.read_bonus(Situation::Neutral, Choice::Approach, 0.0), 0.0);
    assert!(m.read_bonus(Situation::Neutral, Choice::Approach, 1.0) > 0.0);
}

/// An opponent who does the expected thing exactly as often as chance tells you
/// nothing, and the bonus says so: it is measured against the uniform prior, not
/// against zero.
#[test]
fn a_perfectly_random_opponent_is_worth_no_read() {
    let mut m = HabitModel::new(1.0);
    for c in Choice::ALL {
        m.observe(Situation::Neutral, c);
    }
    for c in Choice::ALL {
        assert!(m.read_bonus(Situation::Neutral, c, 1.0).abs() < 1e-6);
    }
}

/// **Determinism.** Rows walk in a stable order and ties break on the `Choice`
/// enum, so a read is a function of the evidence and never of insertion history
/// (ADR 0023 — the counts are a `BTreeMap`, not the sketch's `HashMap`).
#[test]
fn reads_and_rows_are_stable_under_insertion_order() {
    let build = |order: [Choice; 3]| {
        let mut m = HabitModel::new(1.0);
        for c in order {
            m.observe(Situation::Neutral, c);
        }
        m
    };
    let a = build([Choice::Attack, Choice::Jump, Choice::Shield]);
    let b = build([Choice::Shield, Choice::Attack, Choice::Jump]);
    assert_eq!(a.read(Situation::Neutral), b.read(Situation::Neutral));
    assert_eq!(
        a.rows().collect::<Vec<_>>(),
        b.rows().collect::<Vec<_>>(),
        "row order must not depend on what was seen first"
    );
    // Three-way tie: the earliest `Choice` wins, and `Jump` precedes `Attack`.
    assert_eq!(a.read(Situation::Neutral).unwrap().0, Choice::Jump);
}

/// A decay outside `0..=1` is clamped rather than trusted. A model whose counts
/// grew would saturate an `f32` and stop learning, silently.
#[test]
fn a_nonsense_decay_is_clamped_not_obeyed() {
    assert_eq!(HabitModel::new(3.0).decay, 1.0);
    assert_eq!(HabitModel::new(-1.0).decay, 0.0);
}
