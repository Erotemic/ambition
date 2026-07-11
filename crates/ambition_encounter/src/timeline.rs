//! Generic encounter TIMELINE (§6): ordered beats `{ when: Trigger, then:
//! [Effect] }` that advance as triggers fire.
//!
//! This is the ONE timeline authority — an encounter entity (boss fight, wave
//! arena, scripted set piece) may carry an [`EncounterScript`] for its bespoke
//! beats. Triggers OBSERVE the encounter (fired signals, participant deadness,
//! elapsed time); effects are neutral REQUESTS the host executes (defeat a
//! member, command a member, drop a hazard, banner, music). The DATA + trigger
//! evaluation live here (generic, headless-testable); the effect EXECUTION
//! (which touches actor bodies / spawns hazard entities) stays in the host that
//! owns those types.
//!
//! The cut-rope boss fight is expressed entirely as a script: `Gate("rope_cut")`
//! → `CommandMoveTo` (lure) + `DropHazard` (a falling hazard that fires its
//! impact gate) → `ForceKill`.

use bevy::prelude::*;

use ambition_engine_core as ae;

use crate::participants::EncounterParticipants;

/// A named gate fired by gameplay/content (rope cut, hazard impact, cutscene
/// cue, "all adds dead") to advance an [`EncounterScript`] beat waiting on it.
/// The external / scripted hook.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct EncounterGate {
    pub gate: String,
}

impl EncounterGate {
    pub fn new(gate: impl Into<String>) -> Self {
        Self { gate: gate.into() }
    }
}

/// A condition that advances the current script beat. Observes the encounter.
#[derive(Clone, Debug, PartialEq)]
pub enum EncounterTrigger {
    /// An external [`EncounterGate`] with this name fired this tick.
    Gate(String),
    /// The Nth member (by [`EncounterParticipants`] order) is defeated (or gone).
    MemberDied(usize),
    /// Every member is defeated (or gone).
    AllMembersDead,
    /// `secs` elapsed since this beat became current.
    Timer(f32),
}

impl EncounterTrigger {
    /// Whether this trigger holds this tick. `fired` are the gate names fired
    /// this tick; `participants` supplies member deadness (refreshed upstream);
    /// `beat_elapsed` is seconds since this beat became current. Pure — the host
    /// executes the resulting effects, but the DECISION is generic.
    pub fn holds(
        &self,
        participants: &EncounterParticipants,
        fired: &[String],
        beat_elapsed: f32,
    ) -> bool {
        match self {
            EncounterTrigger::Gate(g) => fired.iter().any(|f| f == g),
            EncounterTrigger::MemberDied(i) => {
                participants.members.get(*i).map_or(true, |m| !m.alive)
            }
            EncounterTrigger::AllMembersDead => {
                !participants.members.is_empty() && participants.members.iter().all(|m| !m.alive)
            }
            EncounterTrigger::Timer(secs) => beat_elapsed >= *secs,
        }
    }
}

/// A neutral effect a beat applies (§6 — effects are requests). Member indices
/// address [`EncounterParticipants`]; the host resolves them to entities and
/// executes. No actor types leak into the generic crate.
#[derive(Clone, Debug, PartialEq)]
pub enum EncounterEffect {
    /// Force the Nth member straight to defeat (an environmental kill).
    ForceKill(usize),
    /// Show a gameplay banner for `secs` seconds.
    Banner { text: String, secs: f32 },
    /// Set (`Some`) or clear (`None`) the encounter music request.
    SetMusic(Option<String>),
    /// Command the Nth member toward `target.x` at `speed` (stopping within
    /// `arrive_tolerance`). The host attaches its "commanded move" override.
    CommandMoveTo {
        member: usize,
        target: ae::Vec2,
        speed: f32,
        arrive_tolerance: f32,
    },
    /// Drop a hazard hanging at `anchor`: it waits until `target_member` is
    /// within `align_tolerance.x`, then falls under `gravity` (capped at
    /// `terminal`) and fires `EncounterGate(impact_gate)` on contact.
    DropHazard {
        anchor: ae::Vec2,
        size: ae::Vec2,
        gravity: f32,
        terminal: f32,
        align_tolerance: f32,
        target_member: usize,
        impact_gate: String,
    },
}

/// One scripted beat: when `when` fires, apply `then` and advance the cursor.
#[derive(Clone, Debug, PartialEq)]
pub struct EncounterBeat {
    pub when: EncounterTrigger,
    pub then: Vec<EncounterEffect>,
}

impl EncounterBeat {
    pub fn new(when: EncounterTrigger, then: Vec<EncounterEffect>) -> Self {
        Self { when, then }
    }
}

/// An ordered beat sequence attached to an encounter entity. Advances one beat
/// per fired trigger; inert once the cursor passes the last beat.
#[derive(Component, Clone, Debug, Default)]
pub struct EncounterScript {
    pub beats: Vec<EncounterBeat>,
    cursor: usize,
    /// Seconds in the current beat (for `Timer`).
    elapsed: f32,
}

impl EncounterScript {
    pub fn new(beats: Vec<EncounterBeat>) -> Self {
        Self {
            beats,
            cursor: 0,
            elapsed: 0.0,
        }
    }

    /// True once every beat has fired.
    pub fn done(&self) -> bool {
        self.cursor >= self.beats.len()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Advance the beat clock and, if the current beat's trigger holds, return
    /// its effects for the host to execute + step the cursor. The generic step
    /// of the script; the host applies the returned effects.
    pub fn advance(
        &mut self,
        dt: f32,
        participants: &EncounterParticipants,
        fired: &[String],
    ) -> Vec<EncounterEffect> {
        if self.done() {
            return Vec::new();
        }
        self.elapsed += dt;
        let beat = &self.beats[self.cursor];
        if !beat.when.holds(participants, fired, self.elapsed) {
            return Vec::new();
        }
        let effects = beat.then.clone();
        self.cursor += 1;
        self.elapsed = 0.0;
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::participants::{EncounterParticipant, EncounterRole};

    fn one_member(alive: bool) -> EncounterParticipants {
        EncounterParticipants::new(vec![EncounterParticipant {
            id: "m0".into(),
            entity: None,
            role: EncounterRole::PrimaryTarget,
            alive,
            ownership: crate::participants::Ownership::Adopted,
        }])
    }

    #[test]
    fn a_gate_beat_fires_only_when_its_gate_is_fired() {
        let mut script = EncounterScript::new(vec![EncounterBeat::new(
            EncounterTrigger::Gate("impact".into()),
            vec![EncounterEffect::ForceKill(0)],
        )]);
        let parts = one_member(true);
        // No gate → nothing, cursor unmoved.
        assert!(script.advance(0.1, &parts, &[]).is_empty());
        assert!(!script.done());
        // Gate fired → the effect comes out + cursor advances.
        let effects = script.advance(0.1, &parts, &["impact".to_string()]);
        assert_eq!(effects, vec![EncounterEffect::ForceKill(0)]);
        assert!(script.done());
    }

    #[test]
    fn timer_and_all_members_dead_triggers() {
        let mut timer = EncounterScript::new(vec![EncounterBeat::new(
            EncounterTrigger::Timer(0.25),
            vec![EncounterEffect::Banner {
                text: "go".into(),
                secs: 1.0,
            }],
        )]);
        let parts = one_member(true);
        assert!(timer.advance(0.1, &parts, &[]).is_empty());
        assert!(!timer.advance(0.2, &parts, &[]).is_empty(), "0.3s ≥ 0.25s");

        let mut all_dead = EncounterScript::new(vec![EncounterBeat::new(
            EncounterTrigger::AllMembersDead,
            vec![EncounterEffect::SetMusic(None)],
        )]);
        assert!(all_dead.advance(0.1, &one_member(true), &[]).is_empty());
        assert!(!all_dead.advance(0.1, &one_member(false), &[]).is_empty());
    }
}
