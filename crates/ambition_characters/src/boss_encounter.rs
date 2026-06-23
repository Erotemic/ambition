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

// ===========================================================================
// Entity-local phase mechanism (boss-entity-local refactor, Stage R1)
//
// The trigger-driven replacement for the monolithic, registry-owned phase
// machine above. Where `BossEncounterState` bundles HP + phase + per-phase
// music + stagger + display thresholds into one blob keyed in a global map,
// this splits the ENTITY half out as its own mechanism:
//
//   * HP lives in `BossStatus.health` (entity).
//   * Phase progression lives in `BossPhaseState` (entity) and is driven by a
//     `Vec<PhaseTrigger>` of intrinsic, *optional* DATA — empty ⇒ the boss
//     never phases up and just fights to death (a boss reused as a plain
//     enemy); non-empty ⇒ it phases up on its own, with or without an
//     encounter wrapping it.
//   * Per-phase music / lock-walls / HUD / display thresholds stay encounter
//     concerns (the data catalog now; the encounter entity in R2).
//
// Phase transitions are their OWN parallel mechanism, deliberately NOT the
// hitstun/recoil code (Jon's decision #2): a trigger fires → a brief
// invulnerable `transition_lock` "tell/scream" beat → the exposed phase swaps.
// They merely *resemble* the "event → locked beat → controls change" shape.
//
// See `docs/planning/boss-entity-local-refactor.md` ("Phase model" + R1).
// ===========================================================================

/// The condition under which a [`PhaseTrigger`] fires.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PhaseTriggerCondition {
    /// Fire once the boss's HP fraction is at or below `frac` (the common
    /// "phase up at 66% / 20% HP" case).
    HpBelow(f32),
    /// Fire after `secs` spent in the current phase (the opt-in intro "tell";
    /// any timed beat). Replaces the old *forced* Intro invulnerability — a
    /// boss only gets an intro if it authors a `TimeInPhase` trigger.
    TimeInPhase(f32),
    /// Fire when a named external `gate` message arrives (room switch, "all
    /// adds dead", a scripted cutscene cue). This is the gauntlet / scripted
    /// hook — fired via [`BossPhaseState::notify_external`].
    External(String),
}

/// A pluggable, data-driven phase transition. Intrinsic triggers live as DATA
/// on the boss (a `Vec<PhaseTrigger>`), so flipping a boss between "has phases"
/// and "no phases" is editing data, never code — the key requirement from
/// Jon's resolved decisions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhaseTrigger {
    /// The condition that fires this trigger.
    pub when: PhaseTriggerCondition,
    /// Only evaluate while the boss is in one of these phases. Empty ⇒ any
    /// non-dormant, non-dead phase. Keeps a `Phase2 → Enrage` HP trigger from
    /// firing back in `Phase1`.
    pub from: Vec<BossEncounterPhase>,
    /// The phase to enter when the trigger fires.
    pub to: BossEncounterPhase,
    /// A brief invulnerable "tell/scream" beat (seconds) inserted before the
    /// new phase's controls go live. `0.0` ⇒ swap instantly.
    pub lock: f32,
}

impl PhaseTrigger {
    pub fn hp_below(
        frac: f32,
        from: BossEncounterPhase,
        to: BossEncounterPhase,
        lock: f32,
    ) -> Self {
        Self {
            when: PhaseTriggerCondition::HpBelow(frac),
            from: vec![from],
            to,
            lock,
        }
    }

    pub fn time_in_phase(
        secs: f32,
        from: BossEncounterPhase,
        to: BossEncounterPhase,
        lock: f32,
    ) -> Self {
        Self {
            when: PhaseTriggerCondition::TimeInPhase(secs),
            from: vec![from],
            to,
            lock,
        }
    }

    pub fn external(
        gate: impl Into<String>,
        from: BossEncounterPhase,
        to: BossEncounterPhase,
        lock: f32,
    ) -> Self {
        Self {
            when: PhaseTriggerCondition::External(gate.into()),
            from: vec![from],
            to,
            lock,
        }
    }

    /// True if this trigger may evaluate while the boss is in `phase`.
    pub fn matches_from(&self, phase: BossEncounterPhase) -> bool {
        self.from.is_empty() || self.from.contains(&phase)
    }

    /// Derive the intrinsic phase-up triggers a legacy authored
    /// [`BossEncounterSpec`] implies, so existing bosses keep their phases as
    /// pure DATA once the registry-driven machine is retired (R3).
    ///
    /// The old forced `Intro` / `Transition` invulnerable beats become opt-in:
    ///   * `Intro` is authored only when `intro_seconds > 0` (a `TimeInPhase`
    ///     trigger out of `Intro`); a spec with `intro_seconds == 0` starts
    ///     fighting immediately.
    ///   * The old explicit `Transition` phase collapses into the
    ///     `Phase1 → Phase2` trigger's `lock` (the same invulnerable tell beat,
    ///     now `transition_lock`).
    ///
    /// Stagger (the damage-pressure beat) is intentionally NOT modelled here —
    /// it is a combat-feel / encounter concern, not part of the HpBelow /
    /// TimeInPhase / External trigger vocabulary.
    pub fn intrinsic_from_spec(spec: &BossEncounterSpec) -> Vec<PhaseTrigger> {
        use BossEncounterPhase::{Enrage, Intro, Phase1, Phase2};
        let mut triggers = Vec::new();
        if spec.intro_seconds > 0.0 {
            triggers.push(PhaseTrigger::time_in_phase(
                spec.intro_seconds,
                Intro,
                Phase1,
                0.0,
            ));
        }
        triggers.push(PhaseTrigger::hp_below(
            spec.phase1_to_transition_hp,
            Phase1,
            Phase2,
            spec.transition_seconds.max(0.0),
        ));
        triggers.push(PhaseTrigger::hp_below(
            spec.phase2_to_enrage_hp,
            Phase2,
            Enrage,
            0.0,
        ));
        triggers
    }
}

/// A phase-mechanism event. Deliberately distinct from [`BossEncounterEvent`]:
/// the encounter owns the bespoke music/banner reactions, the entity owns the
/// raw phase mechanics. R3 bridges these into the existing publisher.
#[derive(Clone, Debug, PartialEq)]
pub enum BossPhaseEvent {
    /// The invulnerable tell beat started; `to` is entered when it expires.
    TransitionLockStarted { to: BossEncounterPhase },
    /// The exposed phase changed (after any tell beat).
    PhaseChanged {
        from: BossEncounterPhase,
        to: BossEncounterPhase,
    },
}

/// Entity-local boss phase state + its trigger-driven transition mechanism.
///
/// The ENTITY half of the old `BossEncounterState`. Carries the current phase,
/// a `transition_lock` tell timer, and the intrinsic phase triggers as DATA.
/// HP is NOT here (it lives in `BossStatus.health`); `tick` takes the HP
/// fraction as an argument so the two stay decoupled.
#[derive(Clone, Debug, PartialEq)]
pub struct BossPhaseState {
    /// Current exposed phase the brain reads. `Dormant` until [`wake`]d.
    ///
    /// [`wake`]: BossPhaseState::wake
    pub phase: BossEncounterPhase,
    /// Seconds in the current phase; resets on every swap.
    pub phase_elapsed: f32,
    /// Brief invulnerable "tell/scream" beat between phases. Its OWN timer —
    /// NOT the player's `recoil_lock`; boss-phase code stays decoupled from
    /// combat-feel code. While `> 0` the boss is invulnerable and `pending`
    /// holds the phase entered on expiry.
    pub transition_lock: f32,
    /// Intrinsic phase triggers as DATA. Empty ⇒ the boss never phases up; it
    /// fights until `health == 0` (a boss reused as a plain enemy).
    pub triggers: Vec<PhaseTrigger>,
    /// The phase entered on [`wake`]: `Intro` when an intro tell is authored,
    /// else `Phase1` (fight immediately — no forced intro invulnerability).
    ///
    /// [`wake`]: BossPhaseState::wake
    pub start_phase: BossEncounterPhase,
    /// Phase queued behind an active `transition_lock`.
    pending: Option<BossEncounterPhase>,
}

impl BossPhaseState {
    /// Build from an explicit trigger list. An empty list is valid (and means
    /// "no phases — fight to death").
    pub fn new(triggers: Vec<PhaseTrigger>) -> Self {
        // Opt-in intro: start in `Intro` iff a trigger leaves `Intro` (an
        // intro tell was authored), else fight immediately from `Phase1`.
        let start_phase = if triggers
            .iter()
            .any(|t| t.from.contains(&BossEncounterPhase::Intro))
        {
            BossEncounterPhase::Intro
        } else {
            BossEncounterPhase::Phase1
        };
        Self {
            phase: BossEncounterPhase::Dormant,
            phase_elapsed: 0.0,
            transition_lock: 0.0,
            triggers,
            start_phase,
            pending: None,
        }
    }

    /// Build the intrinsic triggers a legacy [`BossEncounterSpec`] implies.
    pub fn from_spec(spec: &BossEncounterSpec) -> Self {
        Self::new(PhaseTrigger::intrinsic_from_spec(spec))
    }

    /// Whether the boss currently rejects player damage. True during the
    /// `transition_lock` tell beat, plus the phases the vocabulary marks
    /// invulnerable (`Dormant` / `Intro` / `Transition` / `Death`) — so the
    /// intro title-card + the legacy transition beat stay invulnerable.
    pub fn boss_invulnerable(&self) -> bool {
        self.transition_lock > 0.0 || self.phase.boss_invulnerable()
    }

    /// True once a dead boss's death outro has elapsed `death_seconds` — the
    /// caller (the death-resolution system) reads this to write the save +
    /// quest event. `phase_elapsed` keeps advancing during `Death` (see
    /// [`tick`](Self::tick)) so this can fire.
    pub fn death_outro_complete(&self, death_seconds: f32) -> bool {
        matches!(self.phase, BossEncounterPhase::Death) && self.phase_elapsed >= death_seconds
    }

    /// Force the boss straight to `Death` (environmental kills / lethal damage
    /// bypassing the tell beat). Returns the phase event for the caller to
    /// bridge.
    pub fn kill(&mut self) -> Vec<BossPhaseEvent> {
        self.transition_lock = 0.0;
        self.pending = None;
        self.enter(BossEncounterPhase::Death)
    }

    /// Wake the boss: `Dormant → start_phase`. No-op if already awake.
    pub fn wake(&mut self) -> Vec<BossPhaseEvent> {
        if !matches!(self.phase, BossEncounterPhase::Dormant) {
            return Vec::new();
        }
        self.enter(self.start_phase)
    }

    /// Advance the mechanism by `dt`, reading the boss's current `hp_fraction`
    /// (from `BossStatus.health`). Returns the phase events to react to.
    pub fn tick(&mut self, dt: f32, hp_fraction: f32) -> Vec<BossPhaseEvent> {
        let dt = dt.max(0.0);
        if matches!(self.phase, BossEncounterPhase::Dormant) {
            return Vec::new();
        }
        self.phase_elapsed += dt;
        // Death advances its outro timer (for `death_outro_complete`) but fires
        // no further triggers — it is terminal.
        if matches!(self.phase, BossEncounterPhase::Death) {
            return Vec::new();
        }
        // Resolve an in-flight tell beat first; stay invulnerable + ignore
        // fresh triggers until it expires.
        if self.transition_lock > 0.0 {
            self.transition_lock = (self.transition_lock - dt).max(0.0);
            if self.transition_lock == 0.0 {
                if let Some(to) = self.pending.take() {
                    return self.enter(to);
                }
            }
            return Vec::new();
        }
        // Fire the first auto (HP / time) trigger whose `from` matches.
        let fired = self.triggers.iter().find_map(|trig| {
            if !trig.matches_from(self.phase) {
                return None;
            }
            let hit = match &trig.when {
                PhaseTriggerCondition::HpBelow(frac) => hp_fraction <= *frac,
                PhaseTriggerCondition::TimeInPhase(secs) => self.phase_elapsed >= *secs,
                // External triggers fire via `notify_external`, not here.
                PhaseTriggerCondition::External(_) => false,
            };
            hit.then_some((trig.to, trig.lock))
        });
        match fired {
            Some((to, lock)) => self.fire(to, lock),
            None => Vec::new(),
        }
    }

    /// Fire any `External(gate)` trigger eligible from the current phase. The
    /// gauntlet / scripted hook (R2's encounter script drives this).
    pub fn notify_external(&mut self, gate: &str) -> Vec<BossPhaseEvent> {
        if self.transition_lock > 0.0
            || matches!(
                self.phase,
                BossEncounterPhase::Dormant | BossEncounterPhase::Death
            )
        {
            return Vec::new();
        }
        let hit = self.triggers.iter().find_map(|trig| {
            if !trig.matches_from(self.phase) {
                return None;
            }
            match &trig.when {
                PhaseTriggerCondition::External(g) if g == gate => Some((trig.to, trig.lock)),
                _ => None,
            }
        });
        match hit {
            Some((to, lock)) => self.fire(to, lock),
            None => Vec::new(),
        }
    }

    fn fire(&mut self, to: BossEncounterPhase, lock: f32) -> Vec<BossPhaseEvent> {
        if lock > 0.0 {
            self.transition_lock = lock;
            self.pending = Some(to);
            vec![BossPhaseEvent::TransitionLockStarted { to }]
        } else {
            self.enter(to)
        }
    }

    fn enter(&mut self, to: BossEncounterPhase) -> Vec<BossPhaseEvent> {
        if to == self.phase {
            return Vec::new();
        }
        let from = self.phase;
        self.phase = to;
        self.phase_elapsed = 0.0;
        vec![BossPhaseEvent::PhaseChanged { from, to }]
    }
}

#[cfg(test)]
mod phase_mechanism_tests {
    use super::*;
    use BossEncounterPhase::{Death, Dormant, Enrage, Intro, Phase1, Phase2};

    /// A boss with NO triggers never phases up — it just fights until its HP
    /// (owned elsewhere) reaches zero. This is "a boss reused as a plain enemy".
    #[test]
    fn empty_triggers_never_phase_up() {
        let mut state = BossPhaseState::new(Vec::new());
        assert_eq!(state.start_phase, Phase1, "no intro tell ⇒ start fighting");
        assert_eq!(state.wake(), vec![BossPhaseEvent::PhaseChanged { from: Dormant, to: Phase1 }]);
        // Tick a long time at low HP — nothing happens without a trigger.
        for _ in 0..600 {
            assert!(state.tick(1.0 / 60.0, 0.01).is_empty());
        }
        assert_eq!(state.phase, Phase1);
    }

    /// An HpBelow trigger fires when HP crosses its threshold, gated by `from`.
    #[test]
    fn hp_below_trigger_fires_when_threshold_crossed() {
        let mut state = BossPhaseState::new(vec![PhaseTrigger::hp_below(0.5, Phase1, Phase2, 0.0)]);
        state.wake();
        assert_eq!(state.phase, Phase1);
        // Above threshold: no transition.
        assert!(state.tick(0.016, 0.8).is_empty());
        assert_eq!(state.phase, Phase1);
        // Cross it: swap to Phase2.
        let evs = state.tick(0.016, 0.4);
        assert_eq!(evs, vec![BossPhaseEvent::PhaseChanged { from: Phase1, to: Phase2 }]);
        assert_eq!(state.phase, Phase2);
    }

    /// `from` gating: a Phase2→Enrage trigger must not fire while in Phase1.
    #[test]
    fn trigger_respects_from_phase_gate() {
        let mut state = BossPhaseState::new(vec![
            PhaseTrigger::hp_below(0.6, Phase1, Phase2, 0.0),
            PhaseTrigger::hp_below(0.2, Phase2, Enrage, 0.0),
        ]);
        state.wake();
        // Drop HP straight past BOTH thresholds. Only Phase1→Phase2 should fire
        // this tick (the Enrage trigger is gated to `from: Phase2`).
        let evs = state.tick(0.016, 0.1);
        assert_eq!(evs, vec![BossPhaseEvent::PhaseChanged { from: Phase1, to: Phase2 }]);
        // Next tick, now in Phase2, the Enrage trigger fires.
        let evs = state.tick(0.016, 0.1);
        assert_eq!(evs, vec![BossPhaseEvent::PhaseChanged { from: Phase2, to: Enrage }]);
    }

    /// A trigger with a `lock` inserts an invulnerable tell beat before the
    /// swap — its own mechanism, not the swap itself.
    #[test]
    fn lock_inserts_invulnerable_tell_before_swap() {
        let mut state = BossPhaseState::new(vec![PhaseTrigger::hp_below(0.5, Phase1, Phase2, 0.30)]);
        state.wake();
        assert!(!state.boss_invulnerable(), "Phase1 is vulnerable");
        // Crossing the threshold starts the lock, but does NOT swap yet.
        let evs = state.tick(0.016, 0.4);
        assert_eq!(evs, vec![BossPhaseEvent::TransitionLockStarted { to: Phase2 }]);
        assert_eq!(state.phase, Phase1, "phase holds during the tell");
        assert!(state.boss_invulnerable(), "boss is invulnerable during the tell");
        // Tick out the lock; the swap lands on expiry.
        let mut swapped = false;
        for _ in 0..40 {
            let evs = state.tick(0.016, 0.4);
            if evs == vec![BossPhaseEvent::PhaseChanged { from: Phase1, to: Phase2 }] {
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
            BossPhaseState::new(vec![PhaseTrigger::time_in_phase(0.5, Intro, Phase1, 0.0)]);
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
                == vec![BossPhaseEvent::PhaseChanged { from: Intro, to: Phase1 }]
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
        let mut state =
            BossPhaseState::new(vec![PhaseTrigger::external("all_adds_dead", Phase1, Enrage, 0.0)]);
        state.wake();
        // Ticking does nothing — external triggers don't auto-fire.
        assert!(state.tick(1.0, 0.01).is_empty());
        assert_eq!(state.phase, Phase1);
        // The wrong gate is ignored.
        assert!(state.notify_external("some_other_gate").is_empty());
        assert_eq!(state.phase, Phase1);
        // The right gate fires it.
        let evs = state.notify_external("all_adds_dead");
        assert_eq!(evs, vec![BossPhaseEvent::PhaseChanged { from: Phase1, to: Enrage }]);
        assert_eq!(state.phase, Enrage);
    }

    /// Death is terminal: no trigger fires once dead.
    #[test]
    fn death_is_terminal() {
        let mut state = BossPhaseState::new(vec![PhaseTrigger::hp_below(0.9, Phase1, Phase2, 0.0)]);
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
        }
    }
}
