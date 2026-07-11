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
//! lives in `ambition_actors::boss_encounter::roster` — machinery
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
    /// Authored phase triggers appended to the intrinsic HP / time-in-phase
    /// ones (`intrinsic_from_spec`). This is the content seam for `External`
    /// gates a boss can't derive from HP alone — e.g. a mounted boss's
    /// `External("mount_died")` flip into an on-foot mini-phase (ADR 0020; G2).
    /// Empty (the serde default) for every existing boss, so their phase graphs
    /// are byte-unchanged; a rider boss authors one row here.
    #[serde(default)]
    pub extra_phase_triggers: Vec<PhaseTrigger>,
}

// ===========================================================================
// Entity-local phase mechanism (boss-entity-local refactor, Stage R1)
//
// The trigger-driven replacement for the monolithic, registry-owned phase
// machine above. Where `BossEncounterState` bundles HP + phase + per-phase
// music + stagger + display thresholds into one blob keyed in a global map,
// this splits the ENTITY half out as its own mechanism:
//
//   * HP lives on the body's shared `BodyHealth` component (entity, §A1).
//   * Phase progression lives in `ActorPhaseState` (entity) and is driven by a
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
// See `docs/planning/engine/encounter-orchestration.md` ("Phase model" + R1).
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
    /// hook — fired via [`ActorPhaseState::notify_external`].
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
        // Authored External / bespoke triggers (ADR 0020 mount_died flip, etc.)
        // ride on top of the intrinsic HP graph.
        triggers.extend(spec.extra_phase_triggers.iter().cloned());
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
/// HP is NOT here (it lives on the body's shared `BodyHealth`); `tick` takes the HP
/// fraction as an argument so the two stay decoupled.
#[derive(Clone, Debug, PartialEq)]
pub struct ActorPhaseState {
    /// Current exposed phase the brain reads. `Dormant` until [`wake`]d.
    ///
    /// [`wake`]: ActorPhaseState::wake
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
    /// [`wake`]: ActorPhaseState::wake
    pub start_phase: BossEncounterPhase,
    /// Phase queued behind an active `transition_lock`.
    pending: Option<BossEncounterPhase>,
}

impl ActorPhaseState {
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
    /// (from the body's `BodyHealth`). Returns the phase events to react to.
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
mod phase_mechanism_tests;
