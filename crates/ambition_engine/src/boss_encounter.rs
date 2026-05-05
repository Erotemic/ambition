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
//! The phase enum is also surfaced as the player-facing `BossPhase`
//! seldom_state component (`crate::state_machines`), so HUD / debug
//! overlays read from one source of truth.

use serde::{Deserialize, Serialize};

/// Where the boss is in the encounter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BossEncounterPhase {
    #[default]
    Dormant,
    /// Pre-fight intro: title card, boss roar, camera-pan.
    Intro,
    /// First phase of attacks.
    Phase1,
    /// Brief transition between Phase1 and Phase2 — boss is
    /// invulnerable, plays a tell. Patterns from neither phase fire.
    Transition,
    /// Second phase of attacks (faster patterns, more variety).
    Phase2,
    /// Boss is staggered and vulnerable to a punish window. Triggered
    /// by hitting a stagger HP threshold. Auto-recovers after a fixed
    /// duration.
    Stagger,
    /// Final low-HP phase: tighter, faster patterns. Visible "enraged"
    /// presentation cue.
    Enrage,
    /// Boss is dead, playing outro logic.
    Death,
}

impl BossEncounterPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dormant => "dormant",
            Self::Intro => "intro",
            Self::Phase1 => "phase1",
            Self::Transition => "transition",
            Self::Phase2 => "phase2",
            Self::Stagger => "stagger",
            Self::Enrage => "enrage",
            Self::Death => "death",
        }
    }

    pub fn boss_invulnerable(self) -> bool {
        matches!(
            self,
            Self::Dormant | Self::Intro | Self::Transition | Self::Death
        )
    }

    /// True while the boss should be running its attack patterns.
    /// Stagger is not an attacking phase.
    pub fn is_attacking(self) -> bool {
        matches!(self, Self::Phase1 | Self::Phase2 | Self::Enrage)
    }

    /// Phase number for the seldom_state `BossPhase` component.
    /// Stagger / Transition / Death share Phase1's ordinal so the
    /// state component carries a stable "which patterns to run"
    /// hint; consumers should branch on the full
    /// `BossEncounterPhase` for transitions.
    pub fn pattern_phase(self) -> u8 {
        match self {
            Self::Dormant | Self::Intro | Self::Phase1 | Self::Transition | Self::Stagger => 1,
            Self::Phase2 => 2,
            Self::Enrage => 3,
            Self::Death => 0,
        }
    }
}

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

impl BossEncounterSpec {
    pub fn gradient_sentinel() -> Self {
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
            music_intro: "boss_intro".into(),
            music_phase1: "boss_phase1".into(),
            music_phase2: "boss_phase2".into(),
            music_enrage: "boss_enrage".into(),
        }
    }
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
        if matches!(self.phase, BossEncounterPhase::Phase2)
            && frac <= self.spec.phase2_to_enrage_hp
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_intro_then_phase1_after_intro_seconds() {
        let mut s = BossEncounterState::new(BossEncounterSpec::gradient_sentinel());
        let evs = s.enter_intro();
        assert!(matches!(s.phase, BossEncounterPhase::Intro));
        assert!(evs.iter().any(|e| matches!(e, BossEncounterEvent::PhaseChanged { to: BossEncounterPhase::Intro, .. })));
        s.tick(s.spec.intro_seconds + 0.05);
        assert!(matches!(s.phase, BossEncounterPhase::Phase1));
    }

    #[test]
    fn hp_threshold_triggers_transition_and_phase2() {
        let mut s = BossEncounterState::new(BossEncounterSpec::gradient_sentinel());
        s.enter_intro();
        s.tick(s.spec.intro_seconds + 0.01);
        // Damage to push under the phase1_to_transition threshold.
        let to_transition = (s.spec.max_hp as f32 * (1.0 - s.spec.phase1_to_transition_hp + 0.05))
            .ceil() as i32;
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
        let to_enrage =
            (s.spec.max_hp as f32 * (s.spec.phase1_to_transition_hp - s.spec.phase2_to_enrage_hp))
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
