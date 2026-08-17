//! The lib's generic boss-encounter base.
//!
//! The actor crate (`ambition_characters::boss_encounter`) owns the spec schema +
//! the phase state machine. Ambition's *named* boss encounter specs are
//! content: they live in `ambition_content/assets/data/boss_encounters/*.ron`
//! and are contributed through `ambition_content::bosses::register` into the
//! App-local `BossCatalog`. This module keeps only
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
            // The generic base carries no bespoke External gates.
            extra_phase_triggers: Vec::new(),
        }
    }
}
