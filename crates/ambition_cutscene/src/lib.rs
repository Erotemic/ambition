//! Cutscene scripting primitives.
//!
//! A cutscene is an ordered list of timed beats. The runtime owns playback
//! state; presentation consumes its derived dialogue, banner, camera, and fade state.

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Each game's cutscene skip timing. Playback keeps the value selected at its start.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CutsceneSettings {
    pub skip_hold_seconds: f32,
}

impl Default for CutsceneSettings {
    fn default() -> Self { Self { skip_hold_seconds: 0.9 } }
}

impl CutsceneSettings {
    pub fn problems(&self) -> Vec<String> {
        if self.skip_hold_seconds.is_finite() && self.skip_hold_seconds >= 0.0 {
            Vec::new()
        } else {
            vec!["skip_hold_seconds must be finite and nonnegative".into()]
        }
    }
}

#[cfg(feature = "content_pack")]
pub mod content_schema;

/// One beat in a cutscene script.
// `deny_unknown_fields`: a pack authors beats, and a misspelt field must be
// refused, not read as its default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum CutsceneBeat {
    /// Hold the current presentation state for `seconds`. Used for
    /// pacing between beats.
    Wait { seconds: f32 },
    /// Display a line of dialogue. The presentation layer is
    /// responsible for showing `text` and waiting for the dismiss
    /// button before advancing.
    Dialogue {
        /// Label for a voice with no character ID or a title for this scene.
        #[serde(default)]
        speaker: String,
        text: String,
        /// Character ID for the name, voice, and portrait.
        #[serde(default)]
        speaker_character_id: String,
    },
    /// Pan the camera to `target` (a point in the room, in the room's own
    /// pixels, as bodies are placed) over `seconds`, eased, and hold it there
    /// through the beats that follow until a [`Self::CameraRelease`] or the
    /// end of the cutscene. A pan after a pan goes from the first target.
    /// Zero seconds is a cut.
    CameraPan { target: [f32; 2], seconds: f32 },
    /// Give the camera back to the view's own follow over `seconds`, eased.
    /// The end of a cutscene gives it back at once.
    CameraRelease { seconds: f32 },
    /// Ramp the screen between two AUTHORED alphas over `seconds`
    /// (`0.0` = clear, `1.0` = solid black).
    ///
    /// ⭐⭐ **BOTH ENDS ARE AUTHORED, AND THAT IS THE WHOLE POINT** (`Q143`,
    /// ruled 2026-09-19). The beat used to carry only its target, and every
    /// shipped literal targeted `0.0` — a fade UP, from black. Nothing said the
    /// screen was black: a script that opens with a fade to clear was relying
    /// on a convention that existed in the author's head and nowhere in the
    /// data. ⛔ Do not reintroduce it. A beat that means "come up from black"
    /// says `from_alpha: 1.0`.
    ///
    /// ⚠ **A FADE IS NOW A RAMP.** `presentation()` interpolates by
    /// `elapsed / seconds`, so the projection moves across the beat instead of
    /// reading its endpoint at every instant. A zero-length fade is a CUT: it
    /// reads `to_alpha` immediately.
    Fade {
        from_alpha: f32,
        to_alpha: f32,
        seconds: f32,
    },
    /// Set a save-game world flag. Useful for one-shot triggers
    /// (`seen_intro_cutscene = true`) and for tying cutscenes to the
    /// quest system via `QuestStepCondition::FlagSet`.
    SetFlag { id: String, on: bool },
    /// Show a non-dialogue HUD banner (e.g. "Three years later…")
    /// for `seconds`.
    Banner { text: String, seconds: f32 },
    /// The character `character` (a catalog id) in the cutscene's room falls:
    /// a scripted death, as if its health ran out. Instant, like `SetFlag`,
    /// and like it a consequence a skip still applies.
    Fall { character: String },
    /// Run a prepared authored command (`actor.leave_through goblin_raider
    /// barn_door runs_where_sent`, `encounter.start encounter:raid`), the
    /// vocabulary a Yarn `<<command>>` and a room line use. Instant, and a
    /// consequence: a skip still runs it, in its place in the order. The line is prepared against the
    /// installed commands when the library loads, and refused there.
    Command { line: String },
    /// Hold until the prepared condition line is true (`actor.gone
    /// goblin_raider`), or for `seconds_at_most`, whichever is first: a
    /// scene that waits on the world cannot wait forever on a world that
    /// did something else. Shows nothing. A skip still waits on it (SS8): it
    /// is how a scene says its next beat depends on an arrival, a spawn, a
    /// departure or an impact, which a skip cannot hurry.
    WaitFor { condition: String, seconds_at_most: f32 },
    /// Wait until a required result is true.
    WaitUntil { condition: String },
    /// Let the bound role take damage for the rest of the scene.
    AllowDamage {
        role: String,
        #[serde(default)]
        minimum_health: Option<i32>,
    },
    /// Aim at the center of the live subjects named by the bound roles.
    CameraTrack { roles: Vec<String>, seconds: f32 },
    /// Include these beats only if the flag is unset when playback starts.
    BeforeFact { flag: String, beats: Vec<CutsceneBeat> },
}

/// Everything a cutscene is SHOWING right now, derived from where it is.
///
/// This projection is a pure function of `(script, beat_index, elapsed)`, the
/// state already carried by the rollback snapshot. Recomputing the whole
/// presentation avoids stale fields between beats and restores mid-beat state
/// without relying on `BeatEntered` firing again.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CutscenePresentation {
    /// `(speaker, text)` while a dialogue beat is current.
    pub dialogue: Option<(String, String)>,
    /// Stable identity of the character whose dialogue is shown.
    pub speaker_character_id: String,
    /// `(text, seconds_remaining)` while a banner beat is current.
    ///
    /// The countdown is `authored - elapsed`; it is not separate state.
    pub banner: Option<(String, f32)>,
    /// Where the cutscene points the camera, `None` while it leaves the view
    /// to its own follow. See [`CutsceneCamera`].
    pub camera: Option<CutsceneCamera>,
    /// Where a fade beat's ramp has reached, `0.0` when no fade beat is
    /// current. Drawn by `ambition_render`'s cutscene overlay as a full-screen
    /// black sheet at this alpha. See [`CutsceneBeat::Fade`].
    pub fade_alpha: f32,
}

/// The camera a cutscene directs: from one point to another, `progress` of
/// the way (`0.0..=1.0`, eased by the reader). `None` at an end is the view's
/// own follow, wherever that is on the frame it is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CutsceneCamera {
    pub from: Option<[f32; 2]>,
    pub to: Option<[f32; 2]>,
    pub progress: f32,
}

impl CutsceneCamera {
    /// The point to frame, given where the view's follow would put it.
    /// Smoothstep-eased.
    pub fn center(&self, follow: [f32; 2]) -> [f32; 2] {
        let from = self.from.unwrap_or(follow);
        let to = self.to.unwrap_or(follow);
        let t = self.progress.clamp(0.0, 1.0);
        let t = t * t * (3.0 - 2.0 * t);
        [from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t]
    }
}

impl CutsceneRuntime {
    /// What this cutscene is showing, from where it is. See
    /// [`CutscenePresentation`].
    ///
    /// A finished runtime or an out-of-range beat index has no presentation.
    pub fn presentation(&self) -> CutscenePresentation {
        if self.finished || self.skipping {
            return CutscenePresentation::default();
        }
        let Some(beat) = self.script.beats.get(self.beat_index) else {
            return CutscenePresentation::default();
        };
        // ⭐⛤ THE SCREEN DOES NOT UN-FADE BECAUSE THE BEAT ENDED. A fade DOWN
        // followed by a line of dialogue used to go black, advance, and snap
        // clear again — `fade_alpha` was `0.0` on every non-Fade arm, so the
        // primitive worked in exactly the direction the shipped scripts happen
        // to use and nowhere else. The standing value is the last fade's
        // TARGET, which keeps the projection a pure function of
        // `(script, beat_index, elapsed)`.
        let standing = self.standing_fade();
        let camera = self.camera_with(|_| None);
        let shown = match beat {
            CutsceneBeat::Dialogue { speaker, text, speaker_character_id } => CutscenePresentation {
                dialogue: Some((speaker.clone(), text.clone())),
                speaker_character_id: speaker_character_id.clone(),
                fade_alpha: standing,
                ..Default::default()
            },
            CutsceneBeat::Banner { text, seconds } => CutscenePresentation {
                banner: Some((text.clone(), (seconds - self.elapsed).max(0.0))),
                fade_alpha: standing,
                ..Default::default()
            },
            CutsceneBeat::CameraPan { .. } | CutsceneBeat::CameraTrack { .. } | CutsceneBeat::CameraRelease { .. } => CutscenePresentation {
                fade_alpha: standing,
                ..Default::default()
            },
            CutsceneBeat::Fade {
                from_alpha,
                to_alpha,
                seconds,
            } => CutscenePresentation {
                // A zero-length fade is a cut, not a division by zero.
                fade_alpha: {
                    let t = if *seconds > 0.0 {
                        (self.elapsed / seconds).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    (from_alpha + (to_alpha - from_alpha) * t).clamp(0.0, 1.0)
                },
                ..Default::default()
            },
            // A wait or a flag write shows nothing — and says so, rather than
            // leaving the previous beat's picture up. The fade is not a
            // picture a beat puts up; it is the state of the screen.
            CutsceneBeat::Wait { .. }
            | CutsceneBeat::SetFlag { .. }
            | CutsceneBeat::Fall { .. }
            | CutsceneBeat::Command { .. }
            | CutsceneBeat::WaitFor { .. }
            | CutsceneBeat::WaitUntil { .. }
            | CutsceneBeat::AllowDamage { .. }
            | CutsceneBeat::BeforeFact { .. } => CutscenePresentation {
                fade_alpha: standing,
                ..Default::default()
            },
        };
        CutscenePresentation { camera, ..shown }
    }

    /// Where the camera points now: the current pan or release in progress,
    /// else the target the last one before this beat left standing. Like
    /// [`Self::standing_fade`], a pure function of where the cutscene is.
    pub fn camera_with(&self, target_of: impl Fn(&[String]) -> Option<[f32; 2]>) -> Option<CutsceneCamera> {
        let standing = |before: usize| {
            self.script.beats.get(..before).unwrap_or_default().iter().rev().find_map(|beat| match beat {
                CutsceneBeat::CameraPan { target, .. } => Some(Some(*target)),
                CutsceneBeat::CameraTrack { roles, .. } => Some(target_of(roles)),
                CutsceneBeat::CameraRelease { .. } => Some(None),
                _ => None,
            })
        };
        let from = standing(self.beat_index).flatten();
        let progress = |seconds: f32| if seconds > 0.0 { (self.elapsed / seconds).clamp(0.0, 1.0) } else { 1.0 };
        let camera = match self.script.beats.get(self.beat_index)? {
            CutsceneBeat::CameraPan { target, seconds } => {
                CutsceneCamera { from, to: Some(*target), progress: progress(*seconds) }
            }
            CutsceneBeat::CameraTrack { roles, seconds } => {
                CutsceneCamera { from, to: target_of(roles), progress: progress(*seconds) }
            }
            CutsceneBeat::CameraRelease { seconds } => CutsceneCamera { from, to: None, progress: progress(*seconds) },
            _ => CutsceneCamera { from, to: from, progress: 1.0 },
        };
        (camera.from.is_some() || camera.to.is_some()).then_some(camera)
    }

    /// The alpha the screen is already holding when the current beat starts:
    /// the TARGET of the most recent fade before it, or clear if there is none.
    ///
    /// ⛔ IT STOPS AT THE END OF THE CUTSCENE, DELIBERATELY. `presentation()`
    /// answers `default()` once `finished`, so a script ending on a fade to
    /// black clears rather than leaving an opaque sheet over a game nobody can
    /// see through — there is no owner for that sheet after the runtime is
    /// gone, and a cutscene is not a way to turn the screen off. A script that
    /// wants to hand black to whatever comes next needs that thing to own it.
    fn standing_fade(&self) -> f32 {
        self.script
            .beats
            .get(..self.beat_index)
            .unwrap_or_default()
            .iter()
            .rev()
            .find_map(|beat| match beat {
                CutsceneBeat::Fade { to_alpha, .. } => Some(to_alpha.clamp(0.0, 1.0)),
                _ => None,
            })
            .unwrap_or(0.0)
    }
}

/// Visit every declared beat, including beats inside an admission guard.
pub fn authored_beats(beats: &[CutsceneBeat]) -> Vec<&CutsceneBeat> {
    let mut result = Vec::new();
    fn add<'a>(beats: &'a [CutsceneBeat], result: &mut Vec<&'a CutsceneBeat>) {
        for beat in beats {
            result.push(beat);
            if let CutsceneBeat::BeforeFact { beats, .. } = beat {
                add(beats, result);
            }
        }
    }
    add(beats, &mut result);
    result
}

fn admit_beats(beats: Vec<CutsceneBeat>, flag_set: &impl Fn(&str) -> bool, admitted: &mut Vec<CutsceneBeat>) {
    for beat in beats {
        match beat {
            CutsceneBeat::BeforeFact { flag, beats } if !flag_set(&flag) => admit_beats(beats, flag_set, admitted),
            CutsceneBeat::BeforeFact { .. } => {}
            beat => admitted.push(beat),
        }
    }
}

impl CutsceneBeat {
    pub fn condition(&self) -> Option<(&str, Option<f32>)> {
        match self {
            Self::WaitFor { condition, seconds_at_most } => Some((condition, Some(*seconds_at_most))),
            Self::WaitUntil { condition } => Some((condition, None)),
            _ => None,
        }
    }

    /// What this beat does to the world, apart from what it shows: the event
    /// the caller applies. `None` for a beat that only shows something.
    pub fn consequence(&self) -> Option<CutsceneEvent> {
        match self {
            Self::SetFlag { id, on } => Some(CutsceneEvent::FlagWritten { id: id.clone(), on: *on }),
            Self::Fall { character } => Some(CutsceneEvent::CharacterFell { character: character.clone() }),
            Self::Command { line } => Some(CutsceneEvent::CommandAsked { line: line.clone() }),
            _ => None,
        }
    }

    /// Whether the beat self-times (the runtime auto-advances after
    /// `seconds`) or whether it waits for a player dismiss
    /// (Dialogue / Fade-to-1.0 — currently only Dialogue).
    pub fn auto_advances(&self) -> bool {
        !matches!(self, Self::Dialogue { .. })
    }
}

/// Who plays one of a script's roles (SS7). The runtime binds the role to one
/// live body when the scene starts, and the scene's calls name it `@role`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleCast {
    /// The one live body of this character in the scene's room (in the
    /// world, for a scene of the whole world). None, or more than one, leaves
    /// the role unbound and every call naming it refused.
    Character(String),
    /// The body of this placement.
    Placement(String),
}

/// A complete cutscene: ordered beats + an id + an optional "seen"
/// flag. Sandbox systems can skip a cutscene that's already been
/// played by checking `AmbitionGameSaveData::flag(seen_flag)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CutsceneScript {
    pub id: String,
    pub beats: Vec<CutsceneBeat>,
    /// Optional save flag that records whether this cutscene has been
    /// seen. When set, the runtime should refuse to play if the flag
    /// is already on.
    pub seen_flag: Option<String>,
    /// A condition line (`actor.count_is npc_lab_raider 1`): the script
    /// plays, for the whole world, on the first tick the condition holds and
    /// it is unseen. It needs a seen flag, so it plays once. Read by the
    /// trigger, not by playback, so the rollback encoding of a playing
    /// script does not carry it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_when: Option<String>,
    /// The roles its calls name as `@role`, and who plays each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<(String, RoleCast)>,
}

impl CutsceneScript {
    pub fn admit(mut self, flag_set: impl Fn(&str) -> bool) -> Self {
        admit_beats(std::mem::take(&mut self.beats), &flag_set, &mut self.beats);
        self
    }

    pub fn new(id: impl Into<String>, beats: Vec<CutsceneBeat>) -> Self {
        Self {
            id: id.into(),
            beats,
            seen_flag: None,
            starts_when: None,
            roles: Vec::new(),
        }
    }

    pub fn with_seen_flag(mut self, flag: impl Into<String>) -> Self {
        self.seen_flag = Some(flag.into());
        self
    }
}

/// Live cutscene playback. Drains beats in order; `tick` advances the
/// timer for auto-advancing beats and surfaces side-effects for the
/// caller to apply (set flags, banners, dialogue lines).
#[derive(Clone, Debug, PartialEq)]
pub struct CutsceneRuntime {
    pub settings: CutsceneSettings,
    /// Shared, not owned: the rollback log of a room's cutscene
    /// ([`CutscenesInRooms`]) keeps a state for each tick it changes, and the
    /// script does not change while it plays.
    pub script: std::sync::Arc<CutsceneScript>,
    pub beat_index: usize,
    /// Seconds elapsed within the current beat.
    pub elapsed: f32,
    /// True after the last beat finishes. Caller drops the runtime.
    pub finished: bool,
    /// The presentation was skipped (SS8): no picture, no hold on the
    /// players, and every beat passes at once except a `WaitFor`, which
    /// still waits for its answer or its longest hold. The scene's gameplay
    /// work keeps its order and its dependencies.
    pub skipping: bool,
}

impl CutsceneRuntime {
    pub fn new(script: CutsceneScript) -> Self {
        Self {
            settings: CutsceneSettings::default(),
            script: std::sync::Arc::new(script),
            beat_index: 0,
            elapsed: 0.0,
            finished: false,
            skipping: false,
        }
    }

    /// Skip the presentation (SS8). A skip ends the show, not the story: the
    /// rest of the script still runs in order on the following ticks, its
    /// `WaitFor` beats still waiting on what they wait for, so a command is
    /// never asked before the arrival, spawn or departure it depends on. The
    /// scene completes (and is marked seen) when its last beat passes.
    pub fn begin_skip(&mut self) {
        if !self.finished {
            self.skipping = true;
        }
    }

    pub fn current_beat(&self) -> Option<&CutsceneBeat> {
        self.script.beats.get(self.beat_index)
    }

    /// Drive the cutscene forward by `dt`. Returns the events the
    /// caller should react to *this tick* (newly entered beats, flag
    /// toggles, completion). A `WaitFor` beat waits out its longest hold:
    /// [`Self::tick_asking`] answers its condition.
    pub fn tick(&mut self, dt: f32, advance_dialogue: bool) -> Vec<CutsceneEvent> {
        self.tick_asking(dt, advance_dialogue, |_| false)
    }

    /// [`Self::tick`], with `holds(condition)` answering a `WaitFor` beat's
    /// condition line this tick.
    pub fn tick_asking(&mut self, dt: f32, advance_dialogue: bool, holds: impl Fn(&str) -> bool) -> Vec<CutsceneEvent> {
        let mut out = Vec::new();
        if self.finished {
            return out;
        }
        if self.script.beats.is_empty() {
            self.finished = true;
            out.push(CutsceneEvent::Completed);
            return out;
        }
        let dt = dt.max(0.0);
        if self.skipping {
            return self.tick_skipped(dt, holds);
        }
        // First-frame entry into a beat: emit `BeatEntered`.
        if self.elapsed == 0.0 {
            if let Some(beat) = self.script.beats.get(self.beat_index).cloned() {
                out.extend(beat.consequence());
                out.push(CutsceneEvent::BeatEntered {
                    index: self.beat_index,
                    beat,
                });
            }
        }
        let Some(beat) = self.script.beats.get(self.beat_index).cloned() else {
            self.finished = true;
            return out;
        };
        let want_advance = match &beat {
            CutsceneBeat::Wait { seconds } => {
                self.elapsed += dt;
                self.elapsed >= *seconds
            }
            CutsceneBeat::Dialogue { .. } => {
                self.elapsed += dt;
                advance_dialogue
            },
            CutsceneBeat::CameraPan { seconds, .. }
            | CutsceneBeat::CameraTrack { seconds, .. }
            | CutsceneBeat::CameraRelease { seconds }
            | CutsceneBeat::Fade { seconds, .. } => {
                self.elapsed += dt;
                self.elapsed >= *seconds
            }
            CutsceneBeat::SetFlag { .. }
            | CutsceneBeat::Fall { .. }
            | CutsceneBeat::Command { .. }
            | CutsceneBeat::AllowDamage { .. }
            | CutsceneBeat::BeforeFact { .. } => true,
            CutsceneBeat::WaitUntil { condition } => holds(condition),
            CutsceneBeat::WaitFor { condition, seconds_at_most } => {
                self.elapsed += dt;
                holds(condition) || self.elapsed >= *seconds_at_most
            }
            CutsceneBeat::Banner { seconds, .. } => {
                self.elapsed += dt;
                self.elapsed >= *seconds
            }
        };
        if want_advance {
            self.beat_index += 1;
            self.elapsed = 0.0;
            if self.beat_index >= self.script.beats.len() {
                self.finished = true;
                out.push(CutsceneEvent::Completed);
            }
        }
        out
    }

    /// Use a condition answer only for the beat that had it at tick start.
    /// This lets earlier commands apply before the next condition is read.
    fn tick_skipped(&mut self, dt: f32, holds: impl Fn(&str) -> bool) -> Vec<CutsceneEvent> {
        let answered_beat = self.beat_index;
        let mut out = Vec::new();
        while let Some(beat) = self.script.beats.get(self.beat_index).cloned() {
            if let Some((condition, timeout)) = beat.condition() {
                if self.beat_index != answered_beat {
                    return out;
                }
                self.elapsed += dt;
                if !(holds(condition) || timeout.is_some_and(|seconds| self.elapsed >= seconds)) {
                    return out;
                }
            } else {
                out.extend(beat.consequence());
                out.push(CutsceneEvent::BeatEntered { index: self.beat_index, beat });
            }
            self.beat_index += 1;
            self.elapsed = 0.0;
        }
        self.finished = true;
        out.push(CutsceneEvent::Completed);
        out
    }
}

/// Side effects emitted while a cutscene plays.
#[derive(Clone, Debug, PartialEq)]
pub enum CutsceneEvent {
    BeatEntered { index: usize, beat: CutsceneBeat },
    FlagWritten { id: String, on: bool },
    /// A `Fall` beat: the character falls in the cutscene's room.
    CharacterFell { character: String },
    /// A `Command` beat: run the prepared command line, in the cutscene's
    /// room.
    CommandAsked { line: String },
    Completed,
}

// ---------------------------------------------------------------------------
// Live playback state (Bevy Resources). The format + stepper above are bevy-free;
// these are the running-cutscene state the presentation player mutates each frame
// and gameplay/HUD systems read. Kept here so the cutscene runtime is one crate.

/// The ids of the rooms that were live when this trigger last looked, sorted
/// and without repeats — ROLLBACK STATE, not a system local.
///
/// This must be rollback state: a non-rewound memory can suppress the
/// trigger during resimulation and skip the cutscene entirely.
///
/// A set, not one id (OW1): with two live rooms (Alice and Bob apart), each
/// room becomes live on its own, and one id could remember only one of them.
/// With one live room the set has one member, and it flips as the one id did.
/// The same shape as the quest producer's `LastQuestRoom`.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq)]
pub struct LastCutsceneRoom(pub Vec<String>);

/// One playing cutscene: its playback, the room it plays for, and the skip
/// its answering seat is holding.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayingCutscene {
    /// Authoritative while the cutscene plays.
    pub runtime: CutsceneRuntime,
    /// The current picture, republished each tick from `runtime`.
    pub presentation: CutscenePresentation,
    /// The live room the cutscene plays for, by its ordinal
    /// (`LiveRoomInstance::ordinal`), when a room's entry started it; `None`
    /// for a cutscene of the whole world. A room's cutscene holds the players
    /// in that room and not the others (ISL-1b).
    pub room: Option<u32>,
    /// Its roles, each bound when it started to one body's identity (the
    /// `SimId` text the runtime minted). Rollback state: a rewind keeps the
    /// bodies the scene began with.
    pub roles: Vec<(String, String)>,
    /// How long the answering seat has held cancel over this cutscene, in
    /// SIMULATION seconds. The skip fires when it reaches
    /// [`CutsceneSettings::skip_hold_seconds`]; the overlay draws its progress.
    ///
    /// ⛔ ROLLBACK STATE. The hold is read from the seat's
    /// `ControlFrame::cancel_held` inside the timeline, so a rewind puts it
    /// back with the cutscene it belongs to; it ends with its cutscene.
    pub skip_hold: f32,
}

impl PlayingCutscene {
    /// `runtime` starting to play for `room`, its first beat's picture drawn
    /// before any tick, so a cutscene snapshotted the frame it starts is not
    /// blank.
    pub fn new(runtime: CutsceneRuntime, room: Option<u32>) -> Self {
        Self { presentation: runtime.presentation(), runtime, room, roles: Vec::new(), skip_hold: 0.0 }
    }

    /// This cutscene with its roles bound.
    pub fn with_roles(mut self, roles: Vec<(String, String)>) -> Self {
        self.roles = roles;
        self
    }

    /// Does this cutscene hold a body in the live room `room` (by its
    /// ordinal)? A cutscene of the whole world holds every body; a room's
    /// holds the bodies in its room, and a body whose room cannot be told.
    pub fn holds(&self, room: Option<u32>) -> bool {
        !self.runtime.skipping && (self.room.is_none() || room.is_none() || self.room == room)
    }

    /// The seat that answers this cutscene, out of `seats` (each seat with
    /// the live room of the body it drives): for a room's cutscene, the
    /// lowest seat in that room, so a seat in another room neither dismisses
    /// nor skips it (ISL-1b); otherwise, and when no seat is in the room,
    /// `primary`. The tick reads this seat's input and the dialogue box names
    /// this seat's controls, so the two agree on whose line it is.
    pub fn answering_seat<S: Ord>(&self, seats: impl IntoIterator<Item = (S, Option<u32>)>, primary: S) -> S {
        let Some(room) = self.room else {
            return primary;
        };
        seats
            .into_iter()
            .filter(|(_, at)| *at == Some(room))
            .map(|(seat, _)| seat)
            .min()
            .unwrap_or(primary)
    }

    /// The line this cutscene shows and waits on, with its speaker.
    pub fn line(&self) -> Option<(&str, &str)> {
        self.presentation.dialogue.as_ref().map(|(speaker, text)| (speaker.as_str(), text.as_str()))
    }

    /// The script's id.
    pub fn script_id(&self) -> &str {
        self.runtime.script.id.as_str()
    }

    /// Fraction of the way through the skip-hold window, for the overlay's
    /// bar; clamped to `[0, 1]`.
    pub fn skip_progress(&self) -> f32 {
        let window = self.runtime.settings.skip_hold_seconds;
        if window <= 0.0 {
            return if self.skip_hold > 0.0 { 1.0 } else { 0.0 };
        }
        (self.skip_hold / window).clamp(0.0, 1.0)
    }
}

/// The playing cutscenes (SIMULATION-ISLANDS N4): at most one per live room,
/// or one of the whole world, which plays alone. Each holds only its own
/// room's players, answers its own room's seat and ticks at its room's pace.
/// In room order, a cutscene of the whole world first.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ActiveCutscene {
    playing: Vec<PlayingCutscene>,
}

impl ActiveCutscene {
    /// One cutscene playing: `runtime`, for `room`.
    pub fn playing(runtime: CutsceneRuntime, room: Option<u32>) -> Self {
        Self { playing: vec![PlayingCutscene::new(runtime, room)] }
    }

    /// Is a cutscene running?
    ///
    /// ⚠ THIS IS A STATE QUESTION, NOT A POLICY ONE. Input during a cutscene
    /// is decided where every other input claim is decided: the participant
    /// declares `CUTSCENE_CONTEXT` at `context_priority::CUTSCENE` on the
    /// shared `ParticipantContexts` seam, the same way dialogue declares
    /// `DIALOGUE_CONTEXT`. A cutscene answers whether it is playing; it does
    /// not get an opinion about the pad.
    pub fn is_playing(&self) -> bool {
        !self.playing.is_empty()
    }

    /// Does a playing cutscene hold a body in the live room `room` (by its
    /// ordinal)? See [`PlayingCutscene::holds`].
    pub fn holds(&self, room: Option<u32>) -> bool {
        self.playing.iter().any(|cutscene| cutscene.holds(room))
    }

    /// The cutscene that holds a body in `room`: the world's, or that room's.
    pub fn of_room(&self, room: Option<u32>) -> Option<&PlayingCutscene> {
        self.playing.iter().find(|cutscene| cutscene.holds(room))
    }

    /// Each playing cutscene, in room order.
    pub fn iter(&self) -> impl Iterator<Item = &PlayingCutscene> + '_ {
        self.playing.iter()
    }

    /// Each playing cutscene, mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PlayingCutscene> + '_ {
        self.playing.iter_mut()
    }

    /// Is the script `id` playing anywhere?
    pub fn plays(&self, id: &str) -> bool {
        self.playing.iter().any(|cutscene| cutscene.script_id() == id)
    }

    /// Is the script `id` playing for `room` (SS7)? The same script may play
    /// in two live rooms at once: each run binds its own roles and answers its
    /// own waits, so one room's run does not hold back the other's.
    pub fn plays_in(&self, id: &str, room: Option<u32>) -> bool {
        self.playing.iter().any(|cutscene| cutscene.script_id() == id && cutscene.room == room)
    }

    /// May a cutscene for `room` start now? A room's waits while its room or
    /// the whole world has one; the world's waits while any plays.
    pub fn can_start(&self, room: Option<u32>) -> bool {
        match room {
            None => self.playing.is_empty(),
            Some(_) => !self.playing.iter().any(|cutscene| cutscene.room.is_none() || cutscene.room == room),
        }
    }

    /// Start `cutscene`, in its place in room order.
    pub fn start(&mut self, cutscene: PlayingCutscene) {
        let at = self.playing.partition_point(|playing| playing.room <= cutscene.room);
        self.playing.insert(at, cutscene);
    }

    /// Keep only the cutscenes `keep` returns true for.
    pub fn retain(&mut self, keep: impl FnMut(&PlayingCutscene) -> bool) {
        self.playing.retain(keep);
    }

    /// Take every playing cutscene out, to tick them; [`Self::put`] puts the
    /// ones that go on back.
    pub fn take_all(&mut self) -> Vec<PlayingCutscene> {
        std::mem::take(&mut self.playing)
    }

    /// Put cutscenes back, in room order.
    pub fn put(&mut self, cutscenes: impl IntoIterator<Item = PlayingCutscene>) {
        for cutscene in cutscenes {
            self.start(cutscene);
        }
    }

    /// Put back the cutscene `room` (ordinal) had: `state`, or none.
    pub fn restore_room(&mut self, room: u32, state: Option<CutsceneState>) {
        self.playing.retain(|cutscene| cutscene.room != Some(room));
        if let Some(state) = state {
            self.start(state.playing());
        }
    }
}

/// What a room's rewind puts back of the room's playing cutscene: its
/// playback and its skip hold (ISL-1b).
#[derive(Clone, Debug, PartialEq)]
pub struct CutsceneState {
    pub runtime: CutsceneRuntime,
    pub room: u32,
    pub roles: Vec<(String, String)>,
    pub skip_hold: f32,
}

impl CutsceneState {
    /// The state of a room's playing cutscene; `None` for the world's.
    pub fn of(cutscene: &PlayingCutscene) -> Option<Self> {
        Some(Self {
            runtime: cutscene.runtime.clone(),
            room: cutscene.room?,
            roles: cutscene.roles.clone(),
            skip_hold: cutscene.skip_hold,
        })
    }

    /// The cutscene this state plays again, its picture with it.
    pub fn playing(self) -> PlayingCutscene {
        PlayingCutscene { skip_hold: self.skip_hold, roles: self.roles, ..PlayingCutscene::new(self.runtime, Some(self.room)) }
    }
}

/// Each room's cutscene changes within the rewind horizon, oldest first: the
/// room (ordinal) and tick of each, and the room's cutscene it replaced
/// (SIMULATION-ISLANDS ISL-1b, N4). A room rewinds alone, so its rewind puts
/// back the cutscene it had at the save and leaves every other room's. A
/// cutscene of the whole world is not logged.
///
/// Rollback state with a real value: a playing cutscene changes it each tick.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct CutscenesInRooms {
    /// Each room's cutscene as last seen, in room order.
    seen: Vec<CutsceneState>,
    changes: Vec<(u32, u64, Option<CutsceneState>)>,
}

impl CutscenesInRooms {
    /// The cutscenes are `now` on tick `tick`: log each room's change, and
    /// drop the changes older than `horizon` ticks.
    pub fn observe(&mut self, now: &ActiveCutscene, tick: u64, horizon: u64) {
        let now: Vec<CutsceneState> = now.iter().filter_map(CutsceneState::of).collect();
        if self.seen == now {
            return;
        }
        let mut rooms: Vec<u32> = self.seen.iter().chain(&now).map(|state| state.room).collect();
        rooms.sort_unstable();
        rooms.dedup();
        for room in rooms {
            let before = self.seen.iter().find(|state| state.room == room);
            if before != now.iter().find(|state| state.room == room) {
                self.changes.push((room, tick, before.cloned()));
            }
        }
        self.seen = now;
        self.changes.retain(|(_, at, _)| at.saturating_add(horizon) >= tick);
    }

    /// The room `room` rewinds to `saved_at`: take out its changes after that
    /// tick and return the cutscene to put back (`Some(None)`: none plays).
    /// `None` when the room's cutscene did not change.
    pub fn retract_room(&mut self, room: u32, saved_at: u64) -> Option<Option<CutsceneState>> {
        let mut put_back = None;
        for at in (0..self.changes.len()).rev() {
            let (by, tick, _) = &self.changes[at];
            if *by == room && *tick > saved_at {
                put_back = Some(self.changes.remove(at).2);
            }
        }
        if let Some(state) = &put_back {
            self.seen.retain(|seen| seen.room != room);
            if let Some(state) = state {
                let at = self.seen.partition_point(|seen| seen.room < room);
                self.seen.insert(at, state.clone());
            }
        }
        put_back
    }
}

// ---------------------------------------------------------------------------
// Runtime registries (content-free). The authored scripts/bindings live in a
// game's content crate; these are the reusable resources that hold them and the
// runtime systems look up. No named content here.

/// Registry of cutscene scripts keyed by id. A game's content installs scripts
/// here at startup; the trigger/playback systems look one up by id when a
/// trigger fires.
#[derive(Resource, Default, Clone)]
pub struct CutsceneLibrary {
    pub scripts: BTreeMap<String, CutsceneScript>,
}

impl CutsceneLibrary {
    pub fn insert(&mut self, script: CutsceneScript) {
        self.scripts.insert(script.id.clone(), script);
    }

    pub fn get(&self, id: &str) -> Option<&CutsceneScript> {
        self.scripts.get(id)
    }

    /// State one provider's scripts again: remove the `previous` ids it owned
    /// and insert `scripts`. A script id another provider holds is refused,
    /// and nothing changes. Returns the ids the provider owns now.
    pub fn restate<'a>(
        &mut self,
        previous: impl IntoIterator<Item = &'a str>,
        scripts: &[CutsceneScript],
    ) -> Result<Vec<String>, String> {
        let previous: Vec<&str> = previous.into_iter().collect();
        let taken: Vec<&str> = scripts
            .iter()
            .map(|script| script.id.as_str())
            .filter(|id| self.scripts.contains_key(*id) && !previous.contains(id))
            .collect();
        if !taken.is_empty() {
            return Err(format!("cutscene id(s) {taken:?} belong to another provider"));
        }
        for id in previous {
            self.scripts.remove(id);
        }
        for script in scripts {
            self.insert(script.clone());
        }
        Ok(scripts.iter().map(|script| script.id.clone()).collect())
    }
}

/// The cutscene TRIGGER channel — a presentation-neutral request queue.
///
/// Gameplay systems (a boss dying, a room entry, a dialogue node) decide *that*
/// a cutscene should play by pushing its id here; the PLAYBACK runtime drains
/// the queue and starts the matching [`CutsceneScript`], while the overlay
/// presentation lives in `ambition_render::cutscene`. Splitting the trigger out
/// lets sim code request a cutscene without depending on the renderer — the
/// same request-channel seam used for VFX/SFX.
///
/// It lives here, beside the script format and the playback resources, so that
/// any gameplay domain can REQUEST a cutscene without reaching up into whatever
/// crate happens to host the playback systems. That is what it is for: the boss
/// domain asks for `boss_intro_<id>` and knows nothing else about cutscenes.
///
/// ⛔⛔ **EVERY PRODUCER MUST RUN INSIDE THE REWINDING SIMULATION SCHEDULE.**
/// The queue HOLDS ACROSS FRAMES (`drain_cutscene_triggers` returns without
/// draining while a cutscene plays), so it is rollback state and rewinds with
/// `ActiveCutscene` and `LastCutsceneRoom` (`cutscene.trigger_queue`). A
/// producer outside that schedule therefore loses its trigger on every rewind:
/// the restore puts back the queue from before it, and nothing re-produces it.
/// A cutscene dismiss raised on the host side had exactly that defect
/// (`Q136`); it now rides the seat's `ControlFrame`, which a replay
/// re-delivers. The shipped producers are `auto_trigger_room_cutscenes` in the
/// monolith's `Cutscene` phase and `update_boss_encounters` in
/// `ProgressionSet::BossAdvance`, and
/// `scripts/checks/check_sim_consumed_request_writers.py` ratchets the WRITER
/// SET: a new writer fails the maintenance lane until somebody reads its body
/// and records which schedule it runs in.
///
/// ⚠ The ratchet is not an attribution. A schedule's per-system access set is
/// `pub(crate)` in Bevy 0.19 and `System::name()` is the debug placeholder in
/// this build, so no mechanical answer to *"which schedule writes this"* is
/// available; the guard makes the question unavoidable instead of answering it.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct CutsceneTriggerQueue(pub Vec<CutsceneTrigger>);

/// A queued cutscene: its id, and the live room it plays for (by ordinal;
/// `None` for the whole world).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CutsceneTrigger {
    pub id: String,
    pub room: Option<u32>,
}

impl CutsceneTriggerQueue {
    /// Queue a cutscene of the whole world.
    pub fn request(&mut self, id: impl Into<String>) {
        self.push(CutsceneTrigger { id: id.into(), room: None });
    }

    /// Queue a cutscene that plays for the live room with ordinal `room`.
    pub fn request_in(&mut self, id: impl Into<String>, room: u32) {
        self.push(CutsceneTrigger { id: id.into(), room: Some(room) });
    }

    /// A trigger already waiting is not queued twice: two producers on one
    /// tick (a room entry and a volume) ask for the same cutscene.
    fn push(&mut self, trigger: CutsceneTrigger) {
        if !self.0.contains(&trigger) {
            self.0.push(trigger);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intro() -> CutsceneScript {
        CutsceneScript::new(
            "intro",
            vec![
                CutsceneBeat::Banner {
                    text: "Long ago...".into(),
                    seconds: 1.0,
                },
                CutsceneBeat::Dialogue {
                    speaker_character_id: String::new(),
                    speaker: "Warden".into(),
                    text: "You are an instance.".into(),
                },
                CutsceneBeat::SetFlag {
                    id: "intro_seen".into(),
                    on: true,
                },
            ],
        )
        .with_seen_flag("intro_seen")
    }

    /// **A FADE MOVES ACROSS ITS BEAT, AND IT STARTS WHERE THE AUTHOR SAID.**
    ///
    /// ⛔⛤ This arm replaces the one that CHARACTERIZED the defect, and the
    /// defect was not the missing consumer. `presentation()` returned
    /// `to_alpha` ignoring `elapsed`, and every shipped literal targeted `0.0`,
    /// so an 0.8 s fade projected 0.0 at every instant — the number a clear
    /// screen reads. A consumer built against that would have drawn nothing and
    /// closed the row while the player still waited. ⇒ The ramp is the subject;
    /// the consumer is downstream of it.
    #[test]
    fn a_fade_ramps_from_its_authored_start_to_its_authored_target() {
        // Up from black over 0.8 s, sampled every 0.2 s.
        let fade_up = CutsceneScript::new(
            "fade_up",
            vec![CutsceneBeat::Fade {
                from_alpha: 1.0,
                to_alpha: 0.0,
                seconds: 0.8,
            }],
        );
        let mut runtime = CutsceneRuntime::new(fade_up);
        let mut readings = Vec::new();
        for _ in 0..4 {
            readings.push(runtime.presentation().fade_alpha);
            let _ = runtime.tick(0.2, false);
        }
        assert_eq!(readings, vec![1.0, 0.75, 0.5, 0.25]);

        // ⭐ THE CONTROL IS THE OTHER DIRECTION, not another instance of this
        // one: a projection that simply counted down would satisfy the readings
        // above forever.
        let fade_down = CutsceneScript::new(
            "fade_down",
            vec![CutsceneBeat::Fade {
                from_alpha: 0.0,
                to_alpha: 1.0,
                seconds: 0.8,
            }],
        );
        let mut runtime = CutsceneRuntime::new(fade_down);
        assert_eq!(
            runtime.presentation().fade_alpha,
            0.0,
            "a fade DOWN starts clear, which is what its authored start says"
        );
        let _ = runtime.tick(0.4, false);
        assert_eq!(runtime.presentation().fade_alpha, 0.5);

        // ⭐ AND THE TARGET STANDS AFTER THE BEAT ENDS. A fade down followed by
        // a line used to snap clear the instant the beat advanced, which is the
        // primitive working in one direction only.
        let hold = CutsceneScript::new(
            "hold",
            vec![
                CutsceneBeat::Fade {
                    from_alpha: 0.0,
                    to_alpha: 1.0,
                    seconds: 0.4,
                },
                CutsceneBeat::Dialogue {
                    speaker_character_id: String::new(),
                    speaker: "V".into(),
                    text: "in the dark".into(),
                },
            ],
        );
        let mut runtime = CutsceneRuntime::new(hold);
        let _ = runtime.tick(0.4, false);
        assert_eq!(runtime.beat_index, 1, "the fade did not advance");
        assert_eq!(
            runtime.presentation().fade_alpha,
            1.0,
            "the screen un-faded because the beat ended"
        );
        // ⛔ AND IT ENDS WITH THE CUTSCENE, which is the other half: nothing
        // owns an opaque sheet once the runtime is gone.
        let _ = runtime.tick(0.0, true);
        assert!(runtime.finished);
        assert_eq!(runtime.presentation().fade_alpha, 0.0);

        // ⚠ AND A ZERO-LENGTH FADE IS A CUT. The ramp divides by `seconds`;
        // this is the arm that says what happens when an author writes 0.
        let cut = CutsceneScript::new(
            "cut",
            vec![CutsceneBeat::Fade {
                from_alpha: 0.0,
                to_alpha: 1.0,
                seconds: 0.0,
            }],
        );
        assert_eq!(
            CutsceneRuntime::new(cut).presentation().fade_alpha,
            1.0,
            "a fade with no duration reads its target, not its start"
        );
    }

    #[test]
    fn auto_beats_advance_with_dt() {
        let mut runtime = CutsceneRuntime::new(intro());
        // First tick enters Banner, no advance yet.
        let events = runtime.tick(0.5, false);
        assert!(events
            .iter()
            .any(|e| matches!(e, CutsceneEvent::BeatEntered { index: 0, .. })));
        assert_eq!(runtime.beat_index, 0);
        // Push past the banner duration.
        let _ = runtime.tick(0.7, false);
        assert_eq!(runtime.beat_index, 1);
    }

    #[test]
    fn dialogue_waits_for_input() {
        let mut runtime = CutsceneRuntime::new(intro());
        // Skip past banner.
        let _ = runtime.tick(0.0, false);
        let _ = runtime.tick(2.0, false);
        assert!(matches!(
            runtime.current_beat(),
            Some(CutsceneBeat::Dialogue { .. })
        ));
        // Time alone doesn't advance dialogue.
        let _ = runtime.tick(10.0, false);
        assert!(matches!(
            runtime.current_beat(),
            Some(CutsceneBeat::Dialogue { .. })
        ));
        // Dismiss = advance.
        let _ = runtime.tick(0.0, true);
        assert!(matches!(
            runtime.current_beat(),
            Some(CutsceneBeat::SetFlag { .. })
        ));
    }

    #[test]
    fn set_flag_emits_flag_written() {
        let mut runtime = CutsceneRuntime::new(intro());
        // Walk to the SetFlag beat.
        let _ = runtime.tick(0.0, false);
        let _ = runtime.tick(2.0, false); // out of banner
        let _ = runtime.tick(0.0, true); // out of dialogue
        let events = runtime.tick(0.0, false); // enter SetFlag
        assert!(events.iter().any(
            |e| matches!(e, CutsceneEvent::FlagWritten { id, on: true } if id == "intro_seen")
        ));
        // SetFlag self-advances and the runtime completes.
        assert!(runtime.finished);
    }

    /// A skip passes over the show and keeps the story: the flags and falls
    /// of the beats it skipped are still emitted, in order; the beats
    /// already played are not emitted again.
    #[test]
    fn a_skip_keeps_the_flags_and_falls_it_skipped() {
        let script = CutsceneScript::new(
            "last_words",
            vec![
                CutsceneBeat::SetFlag { id: "before".into(), on: true },
                CutsceneBeat::Dialogue { speaker_character_id: String::new(), speaker: "A".into(), text: "B".into() },
                CutsceneBeat::Fall { character: "npc_creator".into() },
                CutsceneBeat::SetFlag { id: "after".into(), on: true },
            ],
        );
        let mut runtime = CutsceneRuntime::new(script);
        let played = runtime.tick(0.0, false);
        assert!(played.contains(&CutsceneEvent::FlagWritten { id: "before".into(), on: true }));
        let _ = runtime.tick(0.0, false);
        assert!(matches!(runtime.current_beat(), Some(CutsceneBeat::Dialogue { .. })));
        runtime.begin_skip();
        let consequences: Vec<CutsceneEvent> = runtime
            .tick(0.1, false)
            .into_iter()
            .filter(|event| !matches!(event, CutsceneEvent::BeatEntered { .. }))
            .collect();
        assert_eq!(
            consequences,
            vec![
                CutsceneEvent::CharacterFell { character: "npc_creator".into() },
                CutsceneEvent::FlagWritten { id: "after".into(), on: true },
                CutsceneEvent::Completed,
            ]
        );
        assert!(runtime.finished);
        assert!(runtime.tick(1.0, true).is_empty(), "a finished runtime is a no-op");
    }

    /// A skipped presentation still runs the story in order (SS8): the
    /// commands before a `WaitFor` are asked at once, the `WaitFor` still
    /// waits for its answer, and nothing after it is asked early.
    #[test]
    fn a_skipped_scene_still_waits_for_what_it_waits_for() {
        let script = CutsceneScript::new(
            "doorway",
            vec![
                CutsceneBeat::Dialogue { speaker_character_id: String::new(), speaker: "A".into(), text: "B".into() },
                CutsceneBeat::Command { line: "walk".into() },
                CutsceneBeat::WaitFor { condition: "arrived".into(), seconds_at_most: 8.0 },
                CutsceneBeat::Dialogue { speaker_character_id: String::new(), speaker: "C".into(), text: "D".into() },
                CutsceneBeat::Command { line: "release".into() },
                CutsceneBeat::WaitUntil { condition: "settled".into() },
            ],
        );
        let asked = |events: &[CutsceneEvent]| -> Vec<String> {
            events
                .iter()
                .filter_map(|event| match event {
                    CutsceneEvent::CommandAsked { line } => Some(line.clone()),
                    _ => None,
                })
                .collect()
        };
        let mut runtime = CutsceneRuntime::new(script);
        let _ = runtime.tick(0.0, false);
        runtime.begin_skip();
        assert_eq!(runtime.presentation(), CutscenePresentation::default(), "a skipped scene shows nothing");
        let events = runtime.tick_asking(0.1, false, |_| false);
        assert_eq!(asked(&events), ["walk"]);
        assert!(!runtime.finished, "the skip waits for the arrival");
        let events = runtime.tick_asking(0.1, false, |_| false);
        assert!(asked(&events).is_empty(), "nothing after the wait is asked before it is answered");
        let events = runtime.tick_asking(0.1, false, |condition| condition == "arrived");
        assert_eq!(asked(&events), ["release"]);
        assert!(!runtime.finished, "the arrival answer does not also settle the release");
        let events = runtime.tick_asking(60.0, false, |_| false);
        assert!(!runtime.finished, "required waits do not expire");
        assert!(events.is_empty());
        assert!(runtime.tick_asking(0.1, false, |condition| condition == "settled")
            .contains(&CutsceneEvent::Completed));
        assert!(runtime.finished);
    }

    #[test]
    fn auto_advances_predicate_distinguishes_dialogue_from_others() {
        // Dialogue waits for player input.
        let dialogue = CutsceneBeat::Dialogue {
            speaker_character_id: String::new(),
            speaker: "X".into(),
            text: "Y".into(),
        };
        assert!(!dialogue.auto_advances());
        // All other beat kinds auto-advance.
        assert!(CutsceneBeat::Wait { seconds: 1.0 }.auto_advances());
        assert!(CutsceneBeat::Banner {
            text: "T".into(),
            seconds: 0.5,
        }
        .auto_advances());
        assert!(CutsceneBeat::Fade {
            from_alpha: 1.0,
            to_alpha: 0.0,
            seconds: 1.0,
        }
        .auto_advances());
        assert!(CutsceneBeat::CameraPan {
            target: [0.0, 0.0],
            seconds: 1.0,
        }
        .auto_advances());
        assert!(CutsceneBeat::SetFlag {
            id: "flag".into(),
            on: true,
        }
        .auto_advances());
    }

    #[test]
    fn cutscene_script_with_seen_flag_round_trips() {
        let script = CutsceneScript::new("test", vec![]).with_seen_flag("test_seen");
        assert_eq!(script.seen_flag.as_deref(), Some("test_seen"));
    }
}

/// Rollback wire format for playback state.
///
/// Playback state affects input suppression, so it is rollback state. The script is
/// encoded directly because decoding has no `CutsceneLibrary` or world lookup.
mod snapshot {
    use super::*;
    use ambition_platformer2d_core::snapshot::{
        put_bool, put_f32, put_str, put_u32, Reader, SnapshotState,
    };

    const WAIT: u32 = 0;
    const DIALOGUE: u32 = 1;
    const CAMERA_PAN: u32 = 2;
    const FADE: u32 = 3;
    const SET_FLAG: u32 = 4;
    const BANNER: u32 = 5;
    const FALL: u32 = 6;
    const COMMAND: u32 = 7;
    const WAIT_FOR: u32 = 8;
    const CAMERA_RELEASE: u32 = 9;
    const WAIT_UNTIL: u32 = 10;
    const ALLOW_DAMAGE: u32 = 11;
    const CAMERA_TRACK: u32 = 12;
    const BEFORE_FACT: u32 = 13;

    impl SnapshotState for CutsceneBeat {
        fn encode(&self, out: &mut Vec<u8>) {
            match self {
                Self::Wait { seconds } => {
                    put_u32(out, WAIT);
                    put_f32(out, *seconds);
                }
                Self::Dialogue { speaker, text, speaker_character_id } => {
                    put_u32(out, DIALOGUE);
                    put_str(out, speaker);
                    put_str(out, text);
                    put_str(out, speaker_character_id);
                }
                Self::CameraPan { target, seconds } => {
                    put_u32(out, CAMERA_PAN);
                    put_f32(out, target[0]);
                    put_f32(out, target[1]);
                    put_f32(out, *seconds);
                }
                Self::Fade {
                    from_alpha,
                    to_alpha,
                    seconds,
                } => {
                    put_u32(out, FADE);
                    put_f32(out, *from_alpha);
                    put_f32(out, *to_alpha);
                    put_f32(out, *seconds);
                }
                Self::SetFlag { id, on } => {
                    put_u32(out, SET_FLAG);
                    put_str(out, id);
                    put_bool(out, *on);
                }
                Self::Banner { text, seconds } => {
                    put_u32(out, BANNER);
                    put_str(out, text);
                    put_f32(out, *seconds);
                }
                Self::Fall { character } => {
                    put_u32(out, FALL);
                    put_str(out, character);
                }
                Self::Command { line } => {
                    put_u32(out, COMMAND);
                    put_str(out, line);
                }
                Self::WaitFor { condition, seconds_at_most } => {
                    put_u32(out, WAIT_FOR);
                    put_str(out, condition);
                    put_f32(out, *seconds_at_most);
                }
                Self::WaitUntil { condition } => {
                    put_u32(out, WAIT_UNTIL);
                    put_str(out, condition);
                }
                Self::AllowDamage { role, minimum_health } => {
                    put_u32(out, ALLOW_DAMAGE);
                    put_str(out, role);
                    put_bool(out, minimum_health.is_some());
                    if let Some(floor) = minimum_health {
                        put_u32(out, *floor as u32);
                    }
                }
                Self::CameraTrack { roles, seconds } => {
                    put_u32(out, CAMERA_TRACK);
                    put_f32(out, *seconds);
                    put_u32(out, roles.len() as u32);
                    for role in roles { put_str(out, role); }
                }
                Self::BeforeFact { flag, beats } => {
                    put_u32(out, BEFORE_FACT);
                    put_str(out, flag);
                    put_u32(out, beats.len() as u32);
                    for beat in beats { beat.encode(out); }
                }
                Self::CameraRelease { seconds } => {
                    put_u32(out, CAMERA_RELEASE);
                    put_f32(out, *seconds);
                }
            }
        }

        fn decode(reader: &mut Reader<'_>) -> Option<Self> {
            Some(match reader.u32()? {
                WAIT => Self::Wait {
                    seconds: reader.f32()?,
                },
                DIALOGUE => Self::Dialogue {
                    speaker: reader.str()?.to_owned(),
                    text: reader.str()?.to_owned(),
                    speaker_character_id: reader.str()?.to_owned(),
                },
                CAMERA_PAN => Self::CameraPan {
                    target: [reader.f32()?, reader.f32()?],
                    seconds: reader.f32()?,
                },
                FADE => Self::Fade {
                    from_alpha: reader.f32()?,
                    to_alpha: reader.f32()?,
                    seconds: reader.f32()?,
                },
                SET_FLAG => Self::SetFlag {
                    id: reader.str()?.to_owned(),
                    on: reader.bool()?,
                },
                BANNER => Self::Banner {
                    text: reader.str()?.to_owned(),
                    seconds: reader.f32()?,
                },
                FALL => Self::Fall {
                    character: reader.str()?.to_owned(),
                },
                COMMAND => Self::Command {
                    line: reader.str()?.to_owned(),
                },
                WAIT_FOR => Self::WaitFor {
                    condition: reader.str()?.to_owned(),
                    seconds_at_most: reader.f32()?,
                },
                WAIT_UNTIL => Self::WaitUntil { condition: reader.str()?.to_owned() },
                ALLOW_DAMAGE => Self::AllowDamage {
                    role: reader.str()?.to_owned(),
                    minimum_health: if reader.bool()? { Some(reader.u32()? as i32) } else { None },
                },
                CAMERA_TRACK => Self::CameraTrack {
                    seconds: reader.f32()?,
                    roles: (0..reader.u32()?).map(|_| reader.str().map(str::to_owned)).collect::<Option<_>>()?,
                },
                BEFORE_FACT => Self::BeforeFact {
                    flag: reader.str()?.to_owned(),
                    beats: (0..reader.u32()?).map(|_| Self::decode(reader)).collect::<Option<_>>()?,
                },
                CAMERA_RELEASE => Self::CameraRelease {
                    seconds: reader.f32()?,
                },
                // Refuse unknown tags rather than decoding a different beat.
                _ => return None,
            })
        }
    }

    impl SnapshotState for CutsceneScript {
        fn encode(&self, out: &mut Vec<u8>) {
            put_str(out, &self.id);
            match &self.seen_flag {
                Some(flag) => {
                    put_bool(out, true);
                    put_str(out, flag);
                }
                None => put_bool(out, false),
            }
            put_u32(out, self.beats.len() as u32);
            for beat in &self.beats {
                beat.encode(out);
            }
            put_u32(out, self.roles.len() as u32);
            for (role, cast) in &self.roles {
                put_str(out, role);
                match cast {
                    RoleCast::Character(character) => {
                        put_u32(out, 0);
                        put_str(out, character);
                    }
                    RoleCast::Placement(placement) => {
                        put_u32(out, 1);
                        put_str(out, placement);
                    }
                }
            }
        }

        fn decode(reader: &mut Reader<'_>) -> Option<Self> {
            let id = reader.str()?.to_owned();
            // Decode absence as `None`; it is valid for a script to have no seen flag.
            let seen_flag = if reader.bool()? {
                Some(reader.str()?.to_owned())
            } else {
                None
            };
            let count = reader.u32()?;
            let mut beats = Vec::with_capacity(count as usize);
            for _ in 0..count {
                beats.push(CutsceneBeat::decode(reader)?);
            }
            let count = reader.u32()?;
            let mut roles = Vec::with_capacity((count as usize).min(16));
            for _ in 0..count {
                let role = reader.str()?.to_owned();
                let cast = match reader.u32()? {
                    0 => RoleCast::Character(reader.str()?.to_owned()),
                    1 => RoleCast::Placement(reader.str()?.to_owned()),
                    _ => return None,
                };
                roles.push((role, cast));
            }
            Some(Self {
                id,
                beats,
                seen_flag,
                starts_when: None,
                roles,
            })
        }
    }

    impl SnapshotState for CutsceneTriggerQueue {
        fn encode(&self, out: &mut Vec<u8>) {
            put_u32(out, self.0.len() as u32);
            for trigger in &self.0 {
                put_str(out, &trigger.id);
                match trigger.room {
                    Some(room) => {
                        put_bool(out, true);
                        put_u32(out, room);
                    }
                    None => put_bool(out, false),
                }
            }
        }

        fn decode(reader: &mut Reader<'_>) -> Option<Self> {
            let count = reader.u32()? as usize;
            let mut triggers = Vec::with_capacity(count.min(64));
            for _ in 0..count {
                let id = reader.str()?.to_owned();
                let room = if reader.bool()? { Some(reader.u32()?) } else { None };
                triggers.push(CutsceneTrigger { id, room });
            }
            Some(Self(triggers))
        }
    }

    impl SnapshotState for LastCutsceneRoom {
        fn encode(&self, out: &mut Vec<u8>) {
            put_u32(out, self.0.len() as u32);
            for room in &self.0 {
                put_str(out, room);
            }
        }

        fn decode(reader: &mut Reader<'_>) -> Option<Self> {
            // A count of 0 is a valid empty memory, not a decode failure.
            let count = reader.u32()? as usize;
            let mut rooms = Vec::with_capacity(count.min(64));
            for _ in 0..count {
                rooms.push(reader.str()?.to_owned());
            }
            Some(Self(rooms))
        }
    }

    fn put_roles(roles: &[(String, String)], out: &mut Vec<u8>) {
        put_u32(out, roles.len() as u32);
        for (role, id) in roles {
            put_str(out, role);
            put_str(out, id);
        }
    }

    fn roles(reader: &mut Reader<'_>) -> Option<Vec<(String, String)>> {
        let count = reader.u32()? as usize;
        let mut roles = Vec::with_capacity(count.min(16));
        for _ in 0..count {
            roles.push((reader.str()?.to_owned(), reader.str()?.to_owned()));
        }
        Some(roles)
    }

    fn put_runtime_state(runtime: &CutsceneRuntime, out: &mut Vec<u8>) {
        put_u32(out, runtime.beat_index as u32);
        put_f32(out, runtime.elapsed);
        put_bool(out, runtime.finished);
        put_bool(out, runtime.skipping);
        put_f32(out, runtime.settings.skip_hold_seconds);
    }

    fn runtime_state(reader: &mut Reader<'_>, script: std::sync::Arc<CutsceneScript>) -> Option<CutsceneRuntime> {
        Some(CutsceneRuntime {
            script,
            beat_index: reader.u32()? as usize,
            elapsed: reader.f32()?,
            finished: reader.bool()?,
            skipping: reader.bool()?,
            settings: CutsceneSettings { skip_hold_seconds: reader.f32()? },
        })
    }

    impl SnapshotState for ActiveCutscene {
        fn encode(&self, out: &mut Vec<u8>) {
            put_u32(out, self.playing.len() as u32);
            for cutscene in &self.playing {
                cutscene.runtime.script.encode(out);
                put_runtime_state(&cutscene.runtime, out);
                match cutscene.room {
                    Some(room) => {
                        put_bool(out, true);
                        put_u32(out, room);
                    }
                    None => put_bool(out, false),
                }
                put_roles(&cutscene.roles, out);
                put_f32(out, cutscene.skip_hold);
            }
        }

        fn decode(reader: &mut Reader<'_>) -> Option<Self> {
            // Presentation is derived from the restored runtime and is not encoded.
            let count = reader.u32()? as usize;
            let mut playing = Vec::with_capacity(count.min(64));
            for _ in 0..count {
                let script = std::sync::Arc::new(CutsceneScript::decode(reader)?);
                let runtime = runtime_state(reader, script)?;
                let room = if reader.bool()? { Some(reader.u32()?) } else { None };
                let roles = roles(reader)?;
                playing.push(PlayingCutscene { roles, skip_hold: reader.f32()?, ..PlayingCutscene::new(runtime, room) });
            }
            Some(Self { playing })
        }
    }

    impl CutsceneState {
        /// The playback by its script's place in the log's table, its room
        /// and the skip hold.
        fn encode(state: Option<&Self>, script: Option<u32>, out: &mut Vec<u8>) {
            match (state, script) {
                (Some(state), Some(script)) => {
                    put_bool(out, true);
                    put_u32(out, script);
                    put_runtime_state(&state.runtime, out);
                    put_u32(out, state.room);
                    put_roles(&state.roles, out);
                    put_f32(out, state.skip_hold);
                }
                _ => put_bool(out, false),
            }
        }

        fn decode(reader: &mut Reader<'_>, scripts: &[std::sync::Arc<CutsceneScript>]) -> Option<Option<Self>> {
            if !reader.bool()? {
                return Some(None);
            }
            let script = scripts.get(reader.u32()? as usize)?.clone();
            let runtime = runtime_state(reader, script)?;
            let room = reader.u32()?;
            let roles = roles(reader)?;
            Some(Some(Self { runtime, room, roles, skip_hold: reader.f32()? }))
        }
    }

    /// Each script the log holds, once: a playing cutscene's state changes
    /// each tick and its script does not, so the log names it by its place in
    /// this table.
    fn script_table(log: &CutscenesInRooms) -> Vec<&std::sync::Arc<CutsceneScript>> {
        let mut scripts: Vec<&std::sync::Arc<CutsceneScript>> = Vec::new();
        let states = log.seen.iter().chain(log.changes.iter().filter_map(|(_, _, state)| state.as_ref()));
        for runtime in states.map(|state| &state.runtime) {
            if !scripts.iter().any(|known| std::sync::Arc::ptr_eq(known, &runtime.script) || ***known == *runtime.script) {
                scripts.push(&runtime.script);
            }
        }
        scripts
    }

    impl SnapshotState for CutscenesInRooms {
        fn encode(&self, out: &mut Vec<u8>) {
            let scripts = script_table(self);
            let index_of = |state: Option<&CutsceneState>| {
                state.and_then(|state| {
                    scripts
                        .iter()
                        .position(|known| std::sync::Arc::ptr_eq(known, &state.runtime.script) || ***known == *state.runtime.script)
                        .map(|at| at as u32)
                })
            };
            put_u32(out, scripts.len() as u32);
            for script in &scripts {
                script.encode(out);
            }
            put_u32(out, self.seen.len() as u32);
            for state in &self.seen {
                CutsceneState::encode(Some(state), index_of(Some(state)), out);
            }
            put_u32(out, self.changes.len() as u32);
            for (room, tick, replaced) in &self.changes {
                put_u32(out, *room);
                ambition_platformer2d_core::snapshot::put_u64(out, *tick);
                CutsceneState::encode(replaced.as_ref(), index_of(replaced.as_ref()), out);
            }
        }

        fn decode(reader: &mut Reader<'_>) -> Option<Self> {
            let count = reader.u32()? as usize;
            let mut scripts = Vec::with_capacity(count.min(64));
            for _ in 0..count {
                scripts.push(std::sync::Arc::new(CutsceneScript::decode(reader)?));
            }
            let count = reader.u32()? as usize;
            let mut seen = Vec::with_capacity(count.min(64));
            for _ in 0..count {
                seen.push(CutsceneState::decode(reader, &scripts)??);
            }
            let count = reader.u32()? as usize;
            let mut changes = Vec::with_capacity(count.min(1024));
            for _ in 0..count {
                changes.push((reader.u32()?, reader.u64()?, CutsceneState::decode(reader, &scripts)?));
            }
            Some(Self { seen, changes })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn round_trip(before: &ActiveCutscene) -> ActiveCutscene {
            let mut bytes = Vec::new();
            before.encode(&mut bytes);
            let mut reader = Reader::new(&bytes);
            ActiveCutscene::decode(&mut reader).expect("a snapshot this crate wrote decodes")
        }

        /// The trigger's live rooms are rollback state; none, one and two
        /// rooms round-trip.
        #[test]
        fn the_last_triggered_room_survives_the_wire_including_absent() {
            for rooms in [
                Vec::new(),
                vec!["intro_wake_room".to_string()],
                vec!["central_hub_complex".to_string(), "switch_lab".to_string()],
            ] {
                let before = LastCutsceneRoom(rooms.clone());
                let mut bytes = Vec::new();
                before.encode(&mut bytes);
                let mut reader = Reader::new(&bytes);
                let after = LastCutsceneRoom::decode(&mut reader)
                    .expect("a snapshot this crate wrote decodes");
                assert_eq!(
                    after, before,
                    "the trigger's room memory did not survive a rewind, so a \
                     resimulation re-entering that room emits nothing and its \
                     cutscene is skipped"
                );
            }
        }

        /// A missing `seen_flag` is valid and must survive decoding.
        #[test]
        fn a_script_without_a_seen_flag_still_decodes() {
            let script = CutsceneScript::new("no_flag", vec![CutsceneBeat::Wait { seconds: 0.25 }]);
            assert!(
                script.seen_flag.is_none(),
                "the fixture must have NO seen flag or it tests the branch that worked"
            );
            let before = ActiveCutscene::playing(CutsceneRuntime::new(script), None);
            let after = round_trip(&before);
            assert_eq!(
                after, before,
                "a flagless script did not survive the wire, so every cutscene \
                 that does not write a save flag is dropped by any rollback"
            );
        }

        /// The room a cutscene plays for survives the wire: a rewind that lost
        /// it would hold every room's players for one room's cutscene. And two
        /// rooms' cutscenes, each with its own skip hold (N4).
        #[test]
        fn the_room_a_cutscene_plays_for_survives_the_wire() {
            let scene = |id: &str| CutsceneRuntime::new(CutsceneScript::new(id, vec![CutsceneBeat::Wait { seconds: 1.0 }]));
            for room in [None, Some(3)] {
                let before = ActiveCutscene::playing(scene("room_scene"), room);
                assert_eq!(round_trip(&before).iter().next().unwrap().room, room, "the cutscene's room did not survive the wire");
            }
            let mut two = ActiveCutscene::playing(scene("alices"), Some(1));
            two.start(PlayingCutscene { skip_hold: 0.25, ..PlayingCutscene::new(scene("bobs"), Some(2)) });
            assert_eq!(round_trip(&two), two, "two rooms' cutscenes did not survive the wire");
            assert_eq!(round_trip(&ActiveCutscene::default()), ActiveCutscene::default());
        }

        /// The log of a room's cutscene changes survives the wire.
        #[test]
        fn the_room_cutscene_log_survives_the_wire() {
            let scene = CutsceneRuntime::new(CutsceneScript::new("logged", vec![CutsceneBeat::Wait { seconds: 2.0 }]));
            let mut active = ActiveCutscene::playing(scene, Some(4));
            let mut log = CutscenesInRooms::default();
            log.observe(&active, 10, 600);
            active.iter_mut().next().unwrap().skip_hold = 0.75;
            log.observe(&active, 11, 600);
            active.retain(|_| false);
            log.observe(&active, 12, 600);
            let mut bytes = Vec::new();
            log.encode(&mut bytes);
            let after = CutscenesInRooms::decode(&mut Reader::new(&bytes)).expect("the log decodes");
            assert_eq!(after, log, "the cutscene log did not survive the wire");
        }

        /// N4: two rooms' cutscenes play at once, and a rewind of one room puts
        /// back only its own: Bob's room rewinds to before his scene started
        /// and Alice's scene, which moved on meanwhile, keeps its place.
        #[test]
        fn a_rooms_rewind_puts_back_only_its_own_cutscene() {
            let scene = |id: &str| CutsceneRuntime::new(CutsceneScript::new(id, vec![CutsceneBeat::Wait { seconds: 9.0 }]));
            let mut active = ActiveCutscene::playing(scene("alices"), Some(1));
            let mut log = CutscenesInRooms::default();
            log.observe(&active, 10, 600);
            assert!(!active.can_start(Some(1)) && active.can_start(Some(2)) && !active.can_start(None));
            active.start(PlayingCutscene::new(scene("bobs"), Some(2)));
            log.observe(&active, 11, 600);
            for cutscene in active.iter_mut() {
                cutscene.runtime.elapsed = 0.5;
            }
            log.observe(&active, 12, 600);
            let put_back = log.retract_room(2, 10).expect("Bob's room changed after the save");
            assert_eq!(put_back, None, "Bob's room had no cutscene at the save");
            active.restore_room(2, put_back);
            let left: Vec<_> = active.iter().map(|cutscene| (cutscene.script_id().to_owned(), cutscene.runtime.elapsed)).collect();
            assert_eq!(left, vec![("alices".to_owned(), 0.5)], "Alice's scene went back with Bob's room");
            assert_eq!(log.retract_room(2, 10), None, "the retracted changes are gone");
            assert_eq!(log.retract_room(1, 11).flatten().map(|state| state.runtime.elapsed), Some(0.0));
        }

        /// Review 2026-11-11 (P2): a playing cutscene changes each tick and
        /// its script does not, so the log holds and writes each script
        /// once, and a second script keeps its own place.
        #[test]
        fn the_room_cutscene_log_keeps_one_copy_of_each_script() {
            let long = CutsceneRuntime::new(CutsceneScript::new(
                "long_scene_with_a_distinct_name",
                vec![CutsceneBeat::Wait { seconds: 9.0 }],
            ));
            let other = CutsceneRuntime::new(CutsceneScript::new("other", vec![CutsceneBeat::Wait { seconds: 1.0 }]));
            let mut log = CutscenesInRooms::default();
            for tick in 0..20u64 {
                let runtime = CutsceneRuntime { elapsed: tick as f32 / 60.0, ..long.clone() };
                log.observe(&ActiveCutscene::playing(runtime, Some(1)), tick, 600);
            }
            log.observe(&ActiveCutscene::playing(other, Some(2)), 20, 600);
            assert!(
                log.changes.iter().filter_map(|(_, _, state)| state.as_ref().map(|state| &state.runtime)).skip(1).all(|runtime| {
                    runtime.script.id != long.script.id || std::sync::Arc::ptr_eq(&runtime.script, &long.script)
                }),
                "a logged tick copied its script"
            );
            let mut bytes = Vec::new();
            log.encode(&mut bytes);
            let name = b"long_scene_with_a_distinct_name";
            let copies = bytes.windows(name.len()).filter(|window| *window == name).count();
            assert_eq!(copies, 1, "the log wrote its script once per tick");
            let after = CutscenesInRooms::decode(&mut Reader::new(&bytes)).expect("the log decodes");
            assert_eq!(after, log, "the log with two scripts did not survive the wire");
        }

        /// A rollback into the MIDDLE of a beat restores its picture.
        ///
        /// Rebuild presentation from decoded runtime state and require it to
        /// match mid-beat, where no `BeatEntered` event will fire again.
        #[test]
        fn a_restore_mid_beat_restores_the_picture_it_was_showing() {
            let script = CutsceneScript::new(
                "banner_scene",
                vec![
                    CutsceneBeat::Banner {
                        text: "CHAPTER ONE".into(),
                        seconds: 4.0,
                    },
                    CutsceneBeat::CameraPan {
                        target: [120.0, 340.0],
                        seconds: 2.0,
                    },
                ],
            );
            let mut runtime = CutsceneRuntime::new(script);
            // Two ticks in: PAST the entry frame, which is the whole point.
            runtime.tick(1.0, false);
            runtime.tick(0.5, false);
            assert!(
                runtime.elapsed > 0.0,
                "the fixture must be mid-beat or it tests the case that already worked"
            );

            let before = ActiveCutscene::playing(runtime, None);
            let before = before.iter().next().unwrap();
            assert!(
                before.presentation.banner.is_some(),
                "the fixture is not showing anything, so restoring it proves nothing"
            );

            let after = round_trip(&ActiveCutscene::playing(before.runtime.clone(), None));
            let restored = after
                .iter()
                .next()
                .expect("the runtime round-trips")
                .runtime
                .presentation();
            assert_eq!(
                restored, before.presentation,
                "the picture did not survive a mid-beat restore. This is the \
                 failure the old snapshot could not see: `runtime` round-tripped \
                 and the banner did not come back, because `BeatEntered` only \
                 fires on the tick a beat begins"
            );
            // Countdown is derived from authored duration minus restored elapsed time.
            let (_, remaining) = restored.banner.expect("a banner beat is current");
            assert!(
                (remaining - 2.5).abs() < 1e-4,
                "the restored banner has {remaining}s left, not the 2.5 its \
                 authored duration minus its elapsed time says"
            );
        }

        /// Advancing to a beat with no dialogue clears the previous dialogue presentation.
        #[test]
        fn advancing_off_a_dialogue_takes_the_dialogue_with_it() {
            let script = CutsceneScript::new(
                "talk_then_pan",
                vec![
                    CutsceneBeat::Dialogue {
                        speaker_character_id: String::new(),
                        speaker: "Alice".into(),
                        text: "Look at that.".into(),
                    },
                    CutsceneBeat::CameraPan {
                        target: [10.0, 20.0],
                        seconds: 1.0,
                    },
                ],
            );
            let mut runtime = CutsceneRuntime::new(script);
            runtime.tick(0.0, false);
            assert!(
                runtime.presentation().dialogue.is_some(),
                "the dialogue beat is not current, so the advance proves nothing"
            );
            // Dismiss it: dialogue does not auto-advance.
            runtime.tick(0.016, true);
            let after = runtime.presentation();
            assert!(
                after.dialogue.is_none(),
                "the dialogue survived into the camera beat — the overlay would \
                 keep showing a line nobody is saying"
            );
            assert_eq!(after.camera.and_then(|camera| camera.to), Some([10.0, 20.0]));
        }

        /// A pan holds through the beats after it, a second pan goes from the
        /// first target, and a release eases back to the follow.
        #[test]
        fn a_pan_holds_until_it_is_released() {
            let script = CutsceneScript::new(
                "look_over_there",
                vec![
                    CutsceneBeat::CameraPan { target: [100.0, 50.0], seconds: 1.0 },
                    CutsceneBeat::Dialogue { speaker_character_id: "npc_creator".into(), speaker: "Creator".into(), text: "There.".into() },
                    CutsceneBeat::CameraPan { target: [300.0, 50.0], seconds: 0.0 },
                    CutsceneBeat::CameraRelease { seconds: 1.0 },
                ],
            );
            let mut runtime = CutsceneRuntime::new(script);
            runtime.tick(0.5, false);
            let halfway = runtime.presentation().camera.expect("the pan directs the camera");
            assert_eq!((halfway.from, halfway.to, halfway.progress), (None, Some([100.0, 50.0]), 0.5));
            assert_eq!(halfway.center([0.0, 50.0]), [50.0, 50.0], "eased halfway from the follow");
            runtime.tick(0.6, false);
            let held = runtime.presentation();
            assert!(held.dialogue.is_some(), "the dialogue beat is current");
            assert_eq!(held.camera.map(|camera| camera.center([0.0, 0.0])), Some([100.0, 50.0]), "the pan holds");
            runtime.tick(0.016, true);
            // A zero-second pan is a cut: it passes on the tick it starts, and
            // the release starts from where it cut to.
            runtime.tick(0.0, false);
            let release = runtime.presentation().camera.expect("the release directs the camera");
            assert_eq!((release.from, release.to, release.progress), (Some([300.0, 50.0]), None, 0.0));
            runtime.tick(0.5, false);
            assert_eq!(runtime.presentation().camera.map(|camera| camera.center([100.0, 50.0])), Some([200.0, 50.0]));
            runtime.tick(0.6, false);
            assert_eq!(runtime.presentation().camera, None, "released, and the cutscene is over");
        }

        /// Every beat variant survives, because a codec that silently loses
        /// one plays a different cutscene after a rewind.
        #[test]
        fn a_playing_cutscene_survives_the_wire() {
            let script = CutsceneScript::new(
                "intro",
                vec![
                    CutsceneBeat::Wait { seconds: 0.5 },
                    CutsceneBeat::Dialogue {
                        speaker_character_id: "npc_creator".into(),
                        speaker: "Creator".into(),
                        text: "Wait.".into(),
                    },
                    CutsceneBeat::CameraPan {
                        target: [12.0, -3.5],
                        seconds: 1.25,
                    },
                    CutsceneBeat::Fade {
                        from_alpha: 0.0,
                        to_alpha: 1.0,
                        seconds: 0.4,
                    },
                    CutsceneBeat::SetFlag {
                        id: "seen_intro".into(),
                        on: true,
                    },
                    CutsceneBeat::Banner {
                        text: "Three years later…".into(),
                        seconds: 2.0,
                    },
                    CutsceneBeat::Fall { character: "npc_creator".into() },
                    CutsceneBeat::Command { line: "actor.release npc_lab_raider".into() },
                    CutsceneBeat::WaitFor { condition: "actor.gone npc_lab_raider".into(), seconds_at_most: 6.0 },
                    CutsceneBeat::CameraRelease { seconds: 0.75 },
                ],
            )
            .with_seen_flag("seen_intro");
            let mut runtime = CutsceneRuntime::new(script);
            runtime.settings.skip_hold_seconds = 0.37;
            runtime.beat_index = 3;
            runtime.elapsed = 0.125;
            let before = ActiveCutscene::playing(runtime, None);
            let after = round_trip(&before);
            assert_eq!(after, before);
        }

        /// The absent case, which is what most frames encode.
        #[test]
        fn no_cutscene_survives_the_wire() {
            let before = ActiveCutscene::default();
            assert!(!round_trip(&before).is_playing());
        }

        /// Unknown beat tags are refused rather than defaulted.
        #[test]
        fn an_unknown_beat_tag_is_refused_rather_than_defaulted() {
            let mut bytes = Vec::new();
            put_u32(&mut bytes, 1);
            put_str(&mut bytes, "intro");
            put_bool(&mut bytes, false);
            put_u32(&mut bytes, 1);
            put_u32(&mut bytes, 99);
            let mut reader = Reader::new(&bytes);
            assert!(ActiveCutscene::decode(&mut reader).is_none());
        }
    }
}

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
