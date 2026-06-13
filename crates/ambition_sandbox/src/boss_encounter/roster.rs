//! The lib's generic boss-encounter base.
//!
//! The actor crate (`ambition_actor::boss_encounter`) owns the spec schema +
//! the phase state machine. Ambition's *named* boss encounter specs are
//! content: they live in `ambition_content/assets/data/boss_encounters/*.ron`
//! and are installed via `ambition_content::bosses::install_boss_roster`
//! (see `specs::boss_encounter_specs`). This module keeps only
//! `gradient_sentinel` — the in-lib generic fallback that `BossProfile::generic`
//! clones for an unknown boss id. It has no RON of its own (it IS the default),
//! so it is not a content duplicate.

use super::BossEncounterSpec;

/// The generic boss-encounter base, as an extension trait over the machinery
/// schema. Only `gradient_sentinel` remains in the lib; the named boss specs
/// moved to content (`boss_encounters/*.ron`).
pub trait BossSpecRoster: Sized {
    fn gradient_sentinel() -> Self;
}

impl BossSpecRoster for BossEncounterSpec {
    fn gradient_sentinel() -> Self {
        Self {
            id: "gradient_sentinel".into(),
            name: "Gradient Sentinel".into(),
            max_hp: 36,
            phase1_to_transition_hp: 0.66,
            transition_to_phase2_hp: 0.66,
            phase2_to_enrage_hp: 0.22,
            intro_seconds: 2.4,
            transition_seconds: 1.6,
            stagger_seconds: 1.8,
            death_seconds: 2.4,
            stagger_threshold: 6,
            stagger_window_seconds: 1.5,
            // Gradient Sentinel: violin track from the first beat of
            // every phase, including Intro. Previously the intro used
            // pulse_drift_voyage as a "calmer escalation bed" but the
            // 2.4-second intro window read as "wrong track for two
            // seconds before snapping into the boss music." Per-phase
            // ids still swap end-to-end at runtime; future audio
            // changes only need to retune these strings.
            music_intro: "fast_paced_violin_boss".into(),
            music_phase1: "fast_paced_violin_boss".into(),
            music_phase2: "fast_paced_violin_boss".into(),
            music_enrage: "fast_paced_violin_boss".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boss_encounter::{BossEncounterEvent, BossEncounterPhase, BossEncounterState};

    #[test]
    fn enter_intro_then_phase1_after_intro_seconds() {
        let mut s = BossEncounterState::new(BossEncounterSpec::gradient_sentinel());
        let evs = s.enter_intro();
        assert!(matches!(s.phase, BossEncounterPhase::Intro));
        assert!(evs.iter().any(|e| matches!(
            e,
            BossEncounterEvent::PhaseChanged {
                to: BossEncounterPhase::Intro,
                ..
            }
        )));
        s.tick(s.spec.intro_seconds + 0.05);
        assert!(matches!(s.phase, BossEncounterPhase::Phase1));
    }

    #[test]
    fn hp_threshold_triggers_transition_and_phase2() {
        let mut s = BossEncounterState::new(BossEncounterSpec::gradient_sentinel());
        s.enter_intro();
        s.tick(s.spec.intro_seconds + 0.01);
        // Damage to push under the phase1_to_transition threshold.
        let to_transition =
            (s.spec.max_hp as f32 * (1.0 - s.spec.phase1_to_transition_hp + 0.05)).ceil() as i32;
        s.apply_player_damage(to_transition);
        assert!(matches!(s.phase, BossEncounterPhase::Transition));
        s.tick(s.spec.transition_seconds + 0.01);
        assert!(matches!(s.phase, BossEncounterPhase::Phase2));
    }

    #[test]
    fn stagger_pressure_triggers_stagger_then_recovers() {
        let mut spec = BossEncounterSpec::gradient_sentinel();
        spec.stagger_threshold = 4;
        spec.stagger_window_seconds = 5.0;
        let mut s = BossEncounterState::new(spec);
        s.enter_intro();
        s.tick(s.spec.intro_seconds + 0.01);
        assert!(matches!(s.phase, BossEncounterPhase::Phase1));
        s.apply_player_damage(2);
        s.apply_player_damage(2);
        // Crossed the threshold → stagger.
        assert!(matches!(s.phase, BossEncounterPhase::Stagger));
        // Tick past stagger duration → back to Phase1.
        s.tick(s.spec.stagger_seconds + 0.01);
        assert!(matches!(s.phase, BossEncounterPhase::Phase1));
    }

    #[test]
    fn enrage_triggers_under_low_hp() {
        let mut s = BossEncounterState::new(BossEncounterSpec::gradient_sentinel());
        s.enter_intro();
        s.tick(s.spec.intro_seconds + 0.01);
        // Force into Phase2 by big chunk of damage.
        let to_phase2 =
            (s.spec.max_hp as f32 * (1.0 - s.spec.phase1_to_transition_hp + 0.05)).ceil() as i32;
        s.apply_player_damage(to_phase2);
        s.tick(s.spec.transition_seconds + 0.01);
        assert!(matches!(s.phase, BossEncounterPhase::Phase2));
        // Damage down to enrage threshold.
        let to_enrage = (s.spec.max_hp as f32
            * (s.spec.phase1_to_transition_hp - s.spec.phase2_to_enrage_hp))
            .ceil() as i32
            + 1;
        s.apply_player_damage(to_enrage);
        assert!(matches!(s.phase, BossEncounterPhase::Enrage));
    }

    #[test]
    fn final_damage_transitions_to_death() {
        let mut s = BossEncounterState::new(BossEncounterSpec::gradient_sentinel());
        s.enter_intro();
        s.tick(100.0);
        assert!(matches!(s.phase, BossEncounterPhase::Phase1));
        let evs = s.apply_player_damage(s.spec.max_hp);
        assert!(matches!(s.phase, BossEncounterPhase::Death));
        assert!(evs.contains(&BossEncounterEvent::Defeated));
        assert!(!s.death_complete());
        s.tick(s.spec.death_seconds + 0.01);
        assert!(s.death_complete());
    }

    /// Drive a full encounter from `Dormant` all the way through
    /// `Death`, verifying every phase transition fires in order with
    /// the right music swap. This is the integration-style guard
    /// for the boss state machine — single test, single source of
    /// truth.
    #[test]
    fn full_encounter_progression_intro_to_death() {
        let mut s = BossEncounterState::new(BossEncounterSpec::gradient_sentinel());
        let mut transitions: Vec<(BossEncounterPhase, BossEncounterPhase)> = Vec::new();
        let mut music_track_changes: Vec<String> = Vec::new();
        let mut record = |evs: &[BossEncounterEvent]| {
            for ev in evs {
                match ev {
                    BossEncounterEvent::PhaseChanged { from, to } => {
                        transitions.push((*from, *to));
                    }
                    BossEncounterEvent::MusicRequested { track } => {
                        music_track_changes.push(track.clone());
                    }
                    _ => {}
                }
            }
        };

        // Dormant → Intro
        let evs = s.enter_intro();
        record(&evs);

        // Intro → Phase1
        let evs = s.tick(s.spec.intro_seconds + 0.05);
        record(&evs);
        assert!(matches!(s.phase, BossEncounterPhase::Phase1));

        // Damage in 2hp chunks (under the stagger threshold of 6)
        // and tick past stagger_window between hits so pressure
        // resets cleanly. Stop before crossing the transition
        // threshold so the next "big hit" is the one that flips us.
        let mut hits = 0;
        while s.hp_fraction() > s.spec.phase1_to_transition_hp + 0.05 {
            let evs = s.apply_player_damage(2);
            record(&evs);
            let evs = s.tick(s.spec.stagger_window_seconds + 0.05);
            record(&evs);
            hits += 1;
            if hits > 50 {
                panic!("too many small hits — pressure reset is broken");
            }
        }
        // One small hit to cross the transition threshold without
        // building stagger pressure to 6.
        let evs = s.apply_player_damage(2);
        record(&evs);
        assert!(
            matches!(s.phase, BossEncounterPhase::Transition),
            "expected Transition, got {:?}",
            s.phase
        );

        // Transition → Phase2
        let evs = s.tick(s.spec.transition_seconds + 0.05);
        record(&evs);
        assert!(matches!(s.phase, BossEncounterPhase::Phase2));

        // Damage to enrage threshold.
        let evs = s.apply_player_damage(s.spec.max_hp / 2);
        record(&evs);
        // The enrage threshold may have been crossed in one big hit.
        // If not, take another bite.
        if !matches!(s.phase, BossEncounterPhase::Enrage) {
            let evs = s.apply_player_damage(2);
            record(&evs);
        }
        // Walk through any stagger.
        let evs = s.tick(s.spec.stagger_seconds + 0.05);
        record(&evs);
        assert!(
            matches!(
                s.phase,
                BossEncounterPhase::Enrage | BossEncounterPhase::Phase2
            ),
            "expected Enrage or Phase2, got {:?}",
            s.phase
        );

        // Final damage to kill.
        let evs = s.apply_player_damage(s.spec.max_hp);
        record(&evs);
        assert!(matches!(s.phase, BossEncounterPhase::Death));
        assert!(!s.death_complete());

        // Tick past death animation.
        let evs = s.tick(s.spec.death_seconds + 0.05);
        record(&evs);
        assert!(s.death_complete());

        // Verify each canonical transition fired at least once.
        let saw = |from: BossEncounterPhase, to: BossEncounterPhase| {
            transitions.iter().any(|(f, t)| *f == from && *t == to)
        };
        assert!(saw(BossEncounterPhase::Dormant, BossEncounterPhase::Intro));
        assert!(saw(BossEncounterPhase::Intro, BossEncounterPhase::Phase1));
        assert!(saw(
            BossEncounterPhase::Phase1,
            BossEncounterPhase::Transition
        ));
        assert!(saw(
            BossEncounterPhase::Transition,
            BossEncounterPhase::Phase2,
        ));
        // Music swaps fired in order. Tightened from a bare non-empty
        // check (tech-debt "Boss music swap requests aren't asserted"):
        // the recorded sequence must equal the per-phase track each
        // PhaseChanged requests, derived from the spec's own fields +
        // the actual transitions. This is content-agnostic — it stays
        // valid when the real per-phase tracks land and diverge from
        // today's shared placeholder — and it catches a music request
        // silently dropping at any phase boundary.
        let expected_music: Vec<String> = transitions
            .iter()
            .filter_map(|(_, to)| match to {
                BossEncounterPhase::Intro => Some(s.spec.music_intro.clone()),
                BossEncounterPhase::Phase1 | BossEncounterPhase::Transition => {
                    Some(s.spec.music_phase1.clone())
                }
                BossEncounterPhase::Phase2 | BossEncounterPhase::Stagger => {
                    Some(s.spec.music_phase2.clone())
                }
                BossEncounterPhase::Enrage => Some(s.spec.music_enrage.clone()),
                BossEncounterPhase::Death | BossEncounterPhase::Dormant => None,
            })
            .filter(|t| !t.is_empty())
            .collect();
        assert!(
            !expected_music.is_empty(),
            "test should drive at least one music-emitting phase"
        );
        assert_eq!(
            music_track_changes, expected_music,
            "boss music swap sequence drifted from the per-phase spec tracks"
        );
    }

    #[test]
    fn invulnerable_phases_ignore_damage() {
        let mut s = BossEncounterState::new(BossEncounterSpec::gradient_sentinel());
        // Dormant: damage no-op.
        let evs = s.apply_player_damage(10);
        assert!(evs.is_empty());
        assert_eq!(s.hp, s.spec.max_hp);
        s.enter_intro();
        // Intro: damage no-op too.
        let evs = s.apply_player_damage(10);
        assert!(evs.is_empty());
        assert_eq!(s.hp, s.spec.max_hp);
    }
}
