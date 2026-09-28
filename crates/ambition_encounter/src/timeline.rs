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
//! impact gate) → `ForceKill`. A script is content: beats deserialize from
//! authored data, and a place is an authored prop of the encounter's room,
//! named by its kind, so a script needs no position written in code.
//!
//! The authored script and the live script are two types. An authored beat
//! names a prop by its kind ([`EncounterBeat`]); [`EncounterScript::prepare`]
//! resolves each kind against the room and each member index against the
//! encounter's members, and refuses a script that names something that is not
//! there. So a live script holds [`EncounterPlace`]s, and the fight does not
//! look up a prop by name or step over an effect that cannot run.

use bevy::prelude::*;

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
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
///
/// `P` is a place. Authored, it is the kind of a prop of the room (`String`);
/// prepared, it is that prop ([`EncounterPlace`]).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EncounterEffect<P = String> {
    /// Force the Nth member straight to defeat (an environmental kill).
    ForceKill(usize),
    /// Show a gameplay banner for `secs` seconds.
    Banner { text: String, secs: f32 },
    /// Set (`Some`) or clear (`None`) the encounter music request.
    SetMusic(Option<String>),
    /// Command the Nth member toward the centre x of the room's prop of kind
    /// `to_prop` at `speed` (stopping within `arrive_tolerance`). The host
    /// attaches its "commanded move" override.
    CommandMoveTo {
        member: usize,
        to_prop: P,
        speed: f32,
        arrive_tolerance: f32,
    },
    /// Drop a hazard the size of the room's prop of kind `prop`, hanging where
    /// that prop is: it waits until `target_member` is within
    /// `align_tolerance.x`, then falls under `gravity` (capped at `terminal`)
    /// and fires `EncounterGate(impact_gate)` on contact.
    DropHazard {
        prop: P,
        gravity: f32,
        terminal: f32,
        align_tolerance: f32,
        target_member: usize,
        impact_gate: String,
    },
}

/// One scripted beat: when `when` fires, apply `then` and advance the cursor.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EncounterBeat<P = String> {
    pub when: EncounterTrigger,
    pub then: Vec<EncounterEffect<P>>,
}

impl<P> EncounterBeat<P> {
    pub fn new(when: EncounterTrigger, then: Vec<EncounterEffect<P>>) -> Self {
        Self { when, then }
    }
}

/// An authored prop of the encounter's room, as a prepared effect holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct EncounterPlace {
    /// The kind the script named, kept for diagnostics.
    pub kind: String,
    /// World-space centre of the prop.
    pub pos: Vec2,
    /// The prop's authored size.
    pub size: Vec2,
}

/// Why an authored script cannot run in the encounter it is prepared for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncounterScriptError {
    /// A trigger or an effect names a member index the encounter does not have.
    NoSuchMember {
        beat: usize,
        member: usize,
        members: usize,
    },
    /// An effect names a prop kind the room does not author.
    NoSuchProp { beat: usize, kind: String },
}

impl std::fmt::Display for EncounterScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSuchMember {
                beat,
                member,
                members,
            } => write!(
                f,
                "beat {beat} names member {member}, and the encounter has {members} member(s)"
            ),
            Self::NoSuchProp { beat, kind } => write!(
                f,
                "beat {beat} names the prop `{kind}`, which the room does not author"
            ),
        }
    }
}

impl std::error::Error for EncounterScriptError {}

/// An ordered beat sequence attached to an encounter entity. Advances one beat
/// per fired trigger; inert once the cursor passes the last beat.
///
/// Its beats are prepared ([`Self::prepare`]), so every effect it yields can
/// run in the encounter that carries it.
#[derive(Component, Clone, Debug, Default)]
pub struct EncounterScript {
    pub beats: Vec<EncounterBeat<EncounterPlace>>,
    cursor: usize,
    /// Seconds in the current beat (for `Timer`).
    elapsed: f32,
}

impl EncounterScript {
    pub fn new(beats: Vec<EncounterBeat<EncounterPlace>>) -> Self {
        Self {
            beats,
            cursor: 0,
            elapsed: 0.0,
        }
    }

    /// Prepare an authored script for an encounter with `members` members in a
    /// room whose props `place` finds by kind (centre and size).
    ///
    /// Every member index and every prop the script names is checked here, so
    /// a script that names something the encounter or the room does not have
    /// is refused before the encounter exists. A refused effect at run time
    /// would still consume its beat, and the fight would continue from a beat
    /// whose precondition never happened.
    pub fn prepare(
        authored: &[EncounterBeat],
        members: usize,
        place: impl Fn(&str) -> Option<(Vec2, Vec2)>,
    ) -> Result<Self, EncounterScriptError> {
        let mut beats = Vec::with_capacity(authored.len());
        for (index, beat) in authored.iter().enumerate() {
            let member_of = |member: usize| {
                if member < members {
                    Ok(member)
                } else {
                    Err(EncounterScriptError::NoSuchMember {
                        beat: index,
                        member,
                        members,
                    })
                }
            };
            let place_of = |kind: &String| {
                place(kind)
                    .map(|(pos, size)| EncounterPlace {
                        kind: kind.clone(),
                        pos,
                        size,
                    })
                    .ok_or_else(|| EncounterScriptError::NoSuchProp {
                        beat: index,
                        kind: kind.clone(),
                    })
            };
            if let EncounterTrigger::MemberDied(member) = beat.when {
                member_of(member)?;
            }
            let mut then = Vec::with_capacity(beat.then.len());
            for effect in &beat.then {
                then.push(match effect {
                    EncounterEffect::ForceKill(member) => {
                        EncounterEffect::ForceKill(member_of(*member)?)
                    }
                    EncounterEffect::Banner { text, secs } => EncounterEffect::Banner {
                        text: text.clone(),
                        secs: *secs,
                    },
                    EncounterEffect::SetMusic(track) => EncounterEffect::SetMusic(track.clone()),
                    EncounterEffect::CommandMoveTo {
                        member,
                        to_prop,
                        speed,
                        arrive_tolerance,
                    } => EncounterEffect::CommandMoveTo {
                        member: member_of(*member)?,
                        to_prop: place_of(to_prop)?,
                        speed: *speed,
                        arrive_tolerance: *arrive_tolerance,
                    },
                    EncounterEffect::DropHazard {
                        prop,
                        gravity,
                        terminal,
                        align_tolerance,
                        target_member,
                        impact_gate,
                    } => EncounterEffect::DropHazard {
                        prop: place_of(prop)?,
                        gravity: *gravity,
                        terminal: *terminal,
                        align_tolerance: *align_tolerance,
                        target_member: member_of(*target_member)?,
                        impact_gate: impact_gate.clone(),
                    },
                });
            }
            beats.push(EncounterBeat::new(beat.when.clone(), then));
        }
        Ok(Self::new(beats))
    }

    /// True once every beat has fired.
    pub fn done(&self) -> bool {
        self.cursor >= self.beats.len()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Seconds the CURRENT beat has been waiting.
    ///
    /// ⭐ Public because it is the field a rewind arm can read. `cursor` only
    /// moves when a trigger holds, so a script whose beat waits on a gate looks
    /// frozen while its clock is running — and the clock is the half that
    /// inflates when a resimulated tick advances state nothing restored.
    pub fn beat_elapsed(&self) -> f32 {
        self.elapsed
    }

    /// The two fields a rewind has to restore, as one deterministic word.
    ///
    /// It lives here because `cursor` and `elapsed` are private to this module,
    /// and the rollback declaration is the only caller that needs both at once.
    pub(crate) fn progress_bits(&self) -> u64 {
        ((self.cursor as u64) << 32) ^ u64::from(self.elapsed.to_bits())
    }

    /// Advance the beat clock and, if the current beat's trigger holds, return
    /// its effects for the host to execute + step the cursor. The generic step
    /// of the script; the host applies the returned effects.
    pub fn advance(
        &mut self,
        dt: f32,
        participants: &EncounterParticipants,
        fired: &[String],
    ) -> Vec<EncounterEffect<EncounterPlace>> {
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

    fn lure_and_drop(member: usize, prop: &str) -> Vec<EncounterBeat> {
        vec![
            EncounterBeat::new(
                EncounterTrigger::Gate("rope_cut".into()),
                vec![
                    EncounterEffect::CommandMoveTo {
                        member,
                        to_prop: prop.into(),
                        speed: 150.0,
                        arrive_tolerance: 42.0,
                    },
                    EncounterEffect::DropHazard {
                        prop: prop.into(),
                        gravity: 1400.0,
                        terminal: 920.0,
                        align_tolerance: 42.0,
                        target_member: member,
                        impact_gate: "impact".into(),
                    },
                ],
            ),
            EncounterBeat::new(
                EncounterTrigger::Gate("impact".into()),
                vec![EncounterEffect::ForceKill(member)],
            ),
        ]
    }

    fn anvil_room(kind: &str) -> Option<(Vec2, Vec2)> {
        (kind == "anvil").then_some((Vec2::new(300.0, 80.0), Vec2::new(40.0, 30.0)))
    }

    /// A prepared effect holds the prop the room authors, so the effect the
    /// fight yields carries the place and no name to look up.
    #[test]
    fn a_prepared_script_holds_the_props_it_names() {
        let mut script = EncounterScript::prepare(&lure_and_drop(0, "anvil"), 1, anvil_room)
            .expect("every name resolves");
        let effects = script.advance(0.0, &one_member(true), &["rope_cut".to_string()]);
        let anvil = EncounterPlace {
            kind: "anvil".into(),
            pos: Vec2::new(300.0, 80.0),
            size: Vec2::new(40.0, 30.0),
        };
        assert!(matches!(
            &effects[0],
            EncounterEffect::CommandMoveTo { to_prop, .. } if *to_prop == anvil
        ));
        assert!(matches!(
            &effects[1],
            EncounterEffect::DropHazard { prop, .. } if *prop == anvil
        ));
    }

    /// A prop the room does not author and a member the encounter does not
    /// have are refused before a script exists, each at the beat that names it.
    #[test]
    fn a_script_that_names_what_is_not_there_is_refused() {
        assert_eq!(
            EncounterScript::prepare(&lure_and_drop(0, "piano"), 1, anvil_room).unwrap_err(),
            EncounterScriptError::NoSuchProp {
                beat: 0,
                kind: "piano".into()
            }
        );
        assert_eq!(
            EncounterScript::prepare(&lure_and_drop(1, "anvil"), 1, anvil_room).unwrap_err(),
            EncounterScriptError::NoSuchMember {
                beat: 0,
                member: 1,
                members: 1
            }
        );
        // A trigger names a member too. `MemberDied` of a member that does not
        // exist holds at once, so it would fire its beat on the first tick.
        let died = [EncounterBeat::new(
            EncounterTrigger::MemberDied(2),
            vec![EncounterEffect::SetMusic(None)],
        )];
        assert_eq!(
            EncounterScript::prepare(&died, 2, anvil_room).unwrap_err(),
            EncounterScriptError::NoSuchMember {
                beat: 0,
                member: 2,
                members: 2
            }
        );
    }
}
