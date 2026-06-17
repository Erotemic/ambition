//! Boss encounter state machine.
//!
//! Composes `BossPatternSchedule` (per-phase attack data) with a
//! coarse phase progression: Dormant → Intro → Phase1 → Transition →
//! Phase2 → Stagger → Enrage → Death. Each phase has an HP threshold
//! that fires the next transition; the runtime walks through them
//! deterministically based on the boss's current health fraction.
//!
//! This module owns the *phase logic* only — how the boss attacks in
//! each phase lives in `BossPatternSchedule`, and how the boss
//! actually moves / hits the player lives in the sandbox's
//! `BossRuntime`. Keeping these layered means a future enemy boss can
//! reuse the phase machinery with different patterns.
//!
//! The phase enum is also surfaced game-side as a seldom_state
//! component so HUD / debug overlays read from one source of truth.
//!
//! The GAME's boss roster (the named `BossEncounterSpec` constructors)
//! lives in `ambition_gameplay_core::boss_encounter::roster` — machinery
//! owns the schema + state machine; the game owns the data.

use serde::{Deserialize, Serialize};

// The phase enum lives in `crate::brain::boss_pattern` (bosses are
// actors, ADR 0016); re-exported here so the encounter machinery is
// importable as one module.
pub use crate::brain::BossEncounterPhase;

/// Authored thresholds + timings driving phase transitions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BossEncounterSpec {
    pub id: String,
    /// Boss display name. HUD shows this above the health bar.
    pub name: String,
    pub max_hp: i32,
    /// HP fraction at which Phase1 ends and Transition begins.
    /// Default 0.66.
    pub phase1_to_transition_hp: f32,
    /// HP fraction at which Phase2 begins (after Transition).
    /// Default same as phase1_to_transition (Transition is an
    /// invulnerable beat, HP doesn't drop further during it).
    pub transition_to_phase2_hp: f32,
    /// HP fraction at which Enrage triggers from Phase2. Default
    /// 0.20.
    pub phase2_to_enrage_hp: f32,
    pub intro_seconds: f32,
    pub transition_seconds: f32,
    pub stagger_seconds: f32,
    pub death_seconds: f32,
    /// HP fraction window where damage builds up "stagger pressure";
    /// hitting the boss for `stagger_threshold` HP within this window
    /// triggers a Stagger. Defaults disable stagger by setting
    /// threshold to a large number.
    pub stagger_threshold: i32,
    pub stagger_window_seconds: f32,
    /// Music track ids per phase. Empty disables the swap.
    pub music_intro: String,
    pub music_phase1: String,
    pub music_phase2: String,
    pub music_enrage: String,
}

/// Live encounter state.
#[derive(Clone, Debug, PartialEq)]
pub struct BossEncounterState {
    pub spec: BossEncounterSpec,
    pub phase: BossEncounterPhase,
    /// Current HP (clamped to [0, max_hp]).
    pub hp: i32,
    /// Seconds spent in the current phase. Resets on every transition.
    pub phase_elapsed: f32,
    /// Damage accumulated in the rolling stagger window. Decays over
    /// `stagger_window_seconds`.
    pub stagger_pressure: i32,
    pub stagger_window: f32,
}

impl BossEncounterState {
    pub fn new(spec: BossEncounterSpec) -> Self {
        let hp = spec.max_hp.max(1);
        Self {
            spec,
            phase: BossEncounterPhase::Dormant,
            hp,
            phase_elapsed: 0.0,
            stagger_pressure: 0,
            stagger_window: 0.0,
        }
    }

    pub fn hp_fraction(&self) -> f32 {
        if self.spec.max_hp <= 0 {
            return 0.0;
        }
        (self.hp as f32 / self.spec.max_hp as f32).clamp(0.0, 1.0)
    }

    pub fn enter_intro(&mut self) -> Vec<BossEncounterEvent> {
        if !matches!(self.phase, BossEncounterPhase::Dormant) {
            return Vec::new();
        }
        self.set_phase(BossEncounterPhase::Intro)
    }

    /// Apply player damage to the boss. Returns the events the caller
    /// should react to (phase changes, music swaps, death).
    pub fn apply_player_damage(&mut self, damage: i32) -> Vec<BossEncounterEvent> {
        if damage <= 0 || self.phase.boss_invulnerable() {
            return Vec::new();
        }
        let mut events = Vec::new();
        self.hp = (self.hp - damage).max(0);
        self.stagger_pressure = self.stagger_pressure.saturating_add(damage);
        self.stagger_window = self.spec.stagger_window_seconds.max(0.0);
        events.push(BossEncounterEvent::DamageApplied {
            amount: damage,
            hp_remaining: self.hp,
            hp_fraction: self.hp_fraction(),
        });
        if self.hp == 0 {
            events.extend(self.set_phase(BossEncounterPhase::Death));
            return events;
        }
        // HP-driven phase transitions take precedence over stagger
        // — Transition / Enrage are plot-required beats and must
        // not be skipped because the player happened to land a big
        // hit on the threshold. Stagger only fires when the HP
        // didn't cross a threshold this hit.
        let frac = self.hp_fraction();
        let mut crossed_plot_threshold = false;
        if matches!(self.phase, BossEncounterPhase::Phase1)
            && frac <= self.spec.phase1_to_transition_hp
        {
            events.extend(self.set_phase(BossEncounterPhase::Transition));
            crossed_plot_threshold = true;
            self.stagger_pressure = 0;
        }
        if matches!(self.phase, BossEncounterPhase::Phase2) && frac <= self.spec.phase2_to_enrage_hp
        {
            events.extend(self.set_phase(BossEncounterPhase::Enrage));
            crossed_plot_threshold = true;
            self.stagger_pressure = 0;
        }
        if crossed_plot_threshold {
            return events;
        }
        // Stagger trigger only fires from the actively-attacking
        // phases (no double-stagger during Transition / Stagger
        // / Death).
        if matches!(
            self.phase,
            BossEncounterPhase::Phase1 | BossEncounterPhase::Phase2 | BossEncounterPhase::Enrage
        ) && self.stagger_pressure >= self.spec.stagger_threshold.max(1)
        {
            self.stagger_pressure = 0;
            events.extend(self.set_phase(BossEncounterPhase::Stagger));
        }
        events
    }

    /// Drive the encounter's timed transitions forward.
    pub fn tick(&mut self, dt: f32) -> Vec<BossEncounterEvent> {
        let dt = dt.max(0.0);
        let mut events = Vec::new();
        if matches!(self.phase, BossEncounterPhase::Dormant) {
            return events;
        }
        self.phase_elapsed += dt;
        if self.stagger_window > 0.0 {
            self.stagger_window = (self.stagger_window - dt).max(0.0);
            if self.stagger_window == 0.0 {
                self.stagger_pressure = 0;
            }
        }
        match self.phase {
            BossEncounterPhase::Intro if self.phase_elapsed >= self.spec.intro_seconds => {
                events.extend(self.set_phase(BossEncounterPhase::Phase1));
            }
            BossEncounterPhase::Transition
                if self.phase_elapsed >= self.spec.transition_seconds =>
            {
                events.extend(self.set_phase(BossEncounterPhase::Phase2));
            }
            BossEncounterPhase::Stagger if self.phase_elapsed >= self.spec.stagger_seconds => {
                // Recover into the right "attacking" phase based on HP.
                let frac = self.hp_fraction();
                let next = if frac <= self.spec.phase2_to_enrage_hp {
                    BossEncounterPhase::Enrage
                } else if frac <= self.spec.phase1_to_transition_hp {
                    BossEncounterPhase::Phase2
                } else {
                    BossEncounterPhase::Phase1
                };
                events.extend(self.set_phase(next));
            }
            _ => {}
        }
        events
    }

    /// Whether the death outro has elapsed and the boss can be
    /// considered fully resolved (caller transitions the persisted
    /// state to `Cleared`).
    pub fn death_complete(&self) -> bool {
        matches!(self.phase, BossEncounterPhase::Death)
            && self.phase_elapsed >= self.spec.death_seconds
    }

    /// Force the encounter directly into Death.
    ///
    /// Environmental win conditions use this instead of
    /// `apply_player_damage`: the boss may be immune to ordinary player
    /// hits, but room-authored mechanics (e.g. a falling anvil) still need
    /// to drive the same death events, music clearing, and save resolution
    /// as normal lethal damage.
    pub fn force_death(&mut self) -> Vec<BossEncounterEvent> {
        self.hp = 0;
        self.stagger_pressure = 0;
        self.stagger_window = 0.0;
        self.set_phase(BossEncounterPhase::Death)
    }

    /// Reset the encounter so a fresh attempt becomes available.
    pub fn reset_for_retry(&mut self) {
        self.phase = BossEncounterPhase::Dormant;
        self.hp = self.spec.max_hp.max(1);
        self.phase_elapsed = 0.0;
        self.stagger_pressure = 0;
        self.stagger_window = 0.0;
    }

    fn set_phase(&mut self, phase: BossEncounterPhase) -> Vec<BossEncounterEvent> {
        if phase == self.phase {
            return Vec::new();
        }
        let from = self.phase;
        self.phase = phase;
        self.phase_elapsed = 0.0;
        let mut events = vec![BossEncounterEvent::PhaseChanged { from, to: phase }];
        let track = match phase {
            BossEncounterPhase::Intro => Some(self.spec.music_intro.clone()),
            BossEncounterPhase::Phase1 | BossEncounterPhase::Transition => {
                Some(self.spec.music_phase1.clone())
            }
            BossEncounterPhase::Phase2 | BossEncounterPhase::Stagger => {
                Some(self.spec.music_phase2.clone())
            }
            BossEncounterPhase::Enrage => Some(self.spec.music_enrage.clone()),
            BossEncounterPhase::Death | BossEncounterPhase::Dormant => None,
        };
        if let Some(track) = track.filter(|t| !t.is_empty()) {
            events.push(BossEncounterEvent::MusicRequested { track });
        }
        if matches!(phase, BossEncounterPhase::Death) {
            events.push(BossEncounterEvent::Defeated);
        }
        events
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BossEncounterEvent {
    PhaseChanged {
        from: BossEncounterPhase,
        to: BossEncounterPhase,
    },
    DamageApplied {
        amount: i32,
        hp_remaining: i32,
        hp_fraction: f32,
    },
    MusicRequested {
        track: String,
    },
    /// Boss reached HP=0; the runtime will play the death animation
    /// and `death_complete` returns true once it's fully resolved.
    Defeated,
}
