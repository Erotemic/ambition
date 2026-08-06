//! Cutscene scripting primitives.
//!
//! A cutscene is an ordered list of timed beats (`CutsceneBeat`) the
//! sandbox plays back: wait, show a line of dialogue, pan the camera,
//! fade in/out, set a world flag. Player input is suppressed for the
//! duration; canceling is allowed (defaulted to "skip" via a button).
//!
//! This module is Bevy-free so the same scripts can be tested
//! deterministically in headless and authored from data. Presentation
//! lives in the sandbox: rendering the dialogue text, easing the
//! camera target, drawing the fade overlay.

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// One beat in a cutscene script.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CutsceneBeat {
    /// Hold the current presentation state for `seconds`. Used for
    /// pacing between beats.
    Wait { seconds: f32 },
    /// Display a line of dialogue. The presentation layer is
    /// responsible for showing `text` and waiting for the dismiss
    /// button before advancing.
    Dialogue { speaker: String, text: String },
    /// Pan the camera to a world-space point over `seconds`. The
    /// presentation layer applies easing.
    CameraPan { target: [f32; 2], seconds: f32 },
    /// Fade screen to `alpha` (0.0 = clear, 1.0 = solid black) over
    /// `seconds`.
    Fade { to_alpha: f32, seconds: f32 },
    /// Set a save-game world flag. Useful for one-shot triggers
    /// (`seen_intro_cutscene = true`) and for tying cutscenes to the
    /// quest system via `QuestStepCondition::FlagSet`.
    SetFlag { id: String, on: bool },
    /// Show a non-dialogue HUD banner (e.g. "Three years later…")
    /// for `seconds`.
    Banner { text: String, seconds: f32 },
}

/// **Everything a cutscene is SHOWING right now, derived from where it is.**
///
/// ⛔⛔ **this exists because "derived" was a claim nobody implemented.**
/// `ActiveCutscene` carried four presentation fields, its rollback snapshot
/// encoded none of them, and the doc said the next simulation tick would
/// republish. It could not: `CutsceneRuntime::tick` emits `BeatEntered` only
/// when `elapsed == 0.0`, so a rollback landing MID-BEAT never saw the entry
/// event again and the banner, the camera target and the fade were gone for the
/// rest of that beat. (GPT 5.6, review through `32eb27a`, finding 1 — correct.)
///
/// ⛔ **and the same shape was visible without any rollback at all.** The fields
/// were mutated one at a time by whichever beat happened to use them, so a
/// `CameraPan` following a dialogue set `camera_target` and left
/// `current_dialogue` standing — the overlay kept showing a line nobody was
/// saying. (Finding 2.) A projection cannot do that: it is one beat's whole
/// answer, replaced every tick, so *dialogue from beat 3 during beat 4* stopped
/// being representable rather than stopping by convention.
///
/// ⭐ **it is a pure function of `(script, beat_index, elapsed)`**, which is
/// exactly the state the snapshot already carries. That is what makes the
/// snapshot honest rather than lucky.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CutscenePresentation {
    /// `(speaker, text)` while a dialogue beat is current.
    pub dialogue: Option<(String, String)>,
    /// `(text, seconds_remaining)` while a banner beat is current.
    ///
    /// ⭐ the countdown is `authored − elapsed`, not a separately ticked timer.
    /// A timer would be a second authority on the same clock and is precisely
    /// what could not survive a restore.
    pub banner: Option<(String, f32)>,
    /// Where a camera-pan beat is pointing.
    pub camera_target: Option<[f32; 2]>,
    /// The fade a fade beat is holding, `0.0` when no fade beat is current.
    pub fade_alpha: f32,
}

impl CutsceneRuntime {
    /// **What this cutscene is showing, from where it is.** See
    /// [`CutscenePresentation`].
    ///
    /// ⚠ a FINISHED runtime shows nothing, and so does one whose index has run
    /// off the end — both are "no beat is current", which is a state the caller
    /// reaches by advancing rather than by an event it might have missed.
    pub fn presentation(&self) -> CutscenePresentation {
        if self.finished {
            return CutscenePresentation::default();
        }
        let Some(beat) = self.script.beats.get(self.beat_index) else {
            return CutscenePresentation::default();
        };
        match beat {
            CutsceneBeat::Dialogue { speaker, text } => CutscenePresentation {
                dialogue: Some((speaker.clone(), text.clone())),
                ..Default::default()
            },
            CutsceneBeat::Banner { text, seconds } => CutscenePresentation {
                banner: Some((text.clone(), (seconds - self.elapsed).max(0.0))),
                ..Default::default()
            },
            CutsceneBeat::CameraPan { target, .. } => CutscenePresentation {
                camera_target: Some(*target),
                ..Default::default()
            },
            CutsceneBeat::Fade { to_alpha, .. } => CutscenePresentation {
                fade_alpha: to_alpha.clamp(0.0, 1.0),
                ..Default::default()
            },
            // A wait or a flag write shows nothing — and says so, rather than
            // leaving the previous beat's picture up.
            CutsceneBeat::Wait { .. } | CutsceneBeat::SetFlag { .. } => {
                CutscenePresentation::default()
            }
        }
    }
}

impl CutsceneBeat {
    /// Whether the beat self-times (the runtime auto-advances after
    /// `seconds`) or whether it waits for a player dismiss
    /// (Dialogue / Fade-to-1.0 — currently only Dialogue).
    pub fn auto_advances(&self) -> bool {
        !matches!(self, Self::Dialogue { .. })
    }
}

/// A complete cutscene: ordered beats + an id + an optional "seen"
/// flag. Sandbox systems can skip a cutscene that's already been
/// played by checking `AmbitionGameSaveData::flag(seen_flag)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CutsceneScript {
    pub id: String,
    pub beats: Vec<CutsceneBeat>,
    /// Optional save flag that records whether this cutscene has been
    /// seen. When set, the runtime should refuse to play if the flag
    /// is already on.
    pub seen_flag: Option<String>,
}

impl CutsceneScript {
    pub fn new(id: impl Into<String>, beats: Vec<CutsceneBeat>) -> Self {
        Self {
            id: id.into(),
            beats,
            seen_flag: None,
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
    pub script: CutsceneScript,
    pub beat_index: usize,
    /// Seconds elapsed within the current beat.
    pub elapsed: f32,
    /// True after the last beat finishes. Caller drops the runtime.
    pub finished: bool,
}

impl CutsceneRuntime {
    pub fn new(script: CutsceneScript) -> Self {
        Self {
            script,
            beat_index: 0,
            elapsed: 0.0,
            finished: false,
        }
    }

    pub fn current_beat(&self) -> Option<&CutsceneBeat> {
        self.script.beats.get(self.beat_index)
    }

    /// Drive the cutscene forward by `dt`. Returns the events the
    /// caller should react to *this tick* (newly entered beats, flag
    /// toggles, completion).
    pub fn tick(&mut self, dt: f32, advance_dialogue: bool) -> Vec<CutsceneEvent> {
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
        // First-frame entry into a beat: emit `BeatEntered`.
        if self.elapsed == 0.0 {
            if let Some(beat) = self.script.beats.get(self.beat_index).cloned() {
                if let CutsceneBeat::SetFlag { id, on } = &beat {
                    out.push(CutsceneEvent::FlagWritten {
                        id: id.clone(),
                        on: *on,
                    });
                }
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
            CutsceneBeat::Dialogue { .. } => advance_dialogue,
            CutsceneBeat::CameraPan { seconds, .. } | CutsceneBeat::Fade { seconds, .. } => {
                self.elapsed += dt;
                self.elapsed >= *seconds
            }
            CutsceneBeat::SetFlag { .. } => true,
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

    /// Cancel and skip remaining beats. Emits `Skipped` so the caller
    /// can still apply terminal flags (e.g. mark the cutscene seen).
    pub fn skip(&mut self) -> Vec<CutsceneEvent> {
        if self.finished {
            return Vec::new();
        }
        self.finished = true;
        vec![CutsceneEvent::Skipped]
    }
}

/// Side effects emitted while a cutscene plays.
#[derive(Clone, Debug, PartialEq)]
pub enum CutsceneEvent {
    BeatEntered { index: usize, beat: CutsceneBeat },
    FlagWritten { id: String, on: bool },
    Skipped,
    Completed,
}

// ---------------------------------------------------------------------------
// Live playback state (Bevy Resources). The format + stepper above are bevy-free;
// these are the running-cutscene state the presentation player mutates each frame
// and gameplay/HUD systems read. Kept here so the cutscene runtime is one crate.

/// Live cutscene playback state. `runtime` is `Some` while a cutscene is running.
///
/// ⭐ **ONE authoritative field.** `runtime` is the whole state; everything a
/// consumer draws is [`CutsceneRuntime::presentation`], recomputed from it. The
/// four presentation fields that used to live here were mutated one at a time by
/// whichever beat used them and dropped entirely by the rollback snapshot, which
/// made *"derived, republished next tick"* a claim rather than a mechanism — see
/// [`CutscenePresentation`] for the two ways that failed.
#[derive(Resource, Default)]
pub struct ActiveCutscene {
    pub runtime: Option<CutsceneRuntime>,
    /// The current beat's whole picture, republished every tick from `runtime`.
    ///
    /// ⚠ **stored rather than computed on read for ONE reason**: consumers are
    /// Bevy systems that take `Res<ActiveCutscene>` and cannot allocate a
    /// projection per reader per frame. It is a cache of a pure function, and
    /// the tick that advances `runtime` is the only writer.
    pub presentation: CutscenePresentation,
}

impl ActiveCutscene {
    pub fn is_playing(&self) -> bool {
        self.runtime.is_some()
    }

    pub fn freezes_player_input(&self) -> bool {
        self.is_playing()
    }
}

/// Hold duration in seconds the player must keep the skip button held before the
/// cutscene actually skips. Long enough that an accidental tap can't burn through
/// scripted content.
pub const SKIP_HOLD_THRESHOLD_SECS: f32 = 1.2;

/// The input layer's advance/skip signal for the active cutscene (kept off the
/// gameplay `ControlFrame` so the sim half doesn't import keyboard state).
/// ⭐ **TWO EDGES, and nothing else crosses.** Each field is a decision the
/// participant made this frame, consumed by the sim with `mem::take`. The
/// partially-held skip lives in [`CutsceneSkipHold`] instead: it is an
/// accumulator the INPUT layer keeps and the HUD draws, and the sim never reads
/// it. Keeping it here made the crossing structure carry presentation state, and
/// a rewind that ever registered this request would have rewound a half-pressed
/// button along with the decisions.
#[derive(Resource, Default)]
pub struct CutsceneAdvanceRequest {
    pub dismiss_dialogue: bool,
    pub skip_cutscene: bool,
}

/// **How long the skip button has been held — input-local, never simulation.**
///
/// ⛔ this was a field on [`CutsceneAdvanceRequest`], the structure that crosses
/// into the sim. It is not a decision: it is the accumulation on the way to one,
/// which the HUD draws as a progress ring and which the sim has never read. The
/// cutscene-authority split (`tracks.md`) puts the completed EDGE on the
/// crossing and the accumulator on this side of it.
///
/// ⚠ it accumulates WALL time on purpose. A player holding a button for 1.2
/// seconds means 1.2 seconds of their life, not of a slow-motion world's.
#[derive(Resource, Default)]
pub struct CutsceneSkipHold {
    pub seconds: f32,
}

impl CutsceneSkipHold {
    /// Fraction of the way through the skip-hold window. Useful for HUD progress
    /// bars; clamped to `[0, 1]`.
    pub fn progress(&self) -> f32 {
        if SKIP_HOLD_THRESHOLD_SECS <= 0.0 {
            return 1.0;
        }
        (self.seconds / SKIP_HOLD_THRESHOLD_SECS).clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Runtime registries (content-free). The authored scripts/bindings live in a
// game's content crate; these are the reusable resources that hold them and the
// runtime systems look up. No named content here.

/// Registry of cutscene scripts keyed by id. A game's content installs scripts
/// here at startup; the trigger/playback systems look one up by id when a
/// trigger fires.
#[derive(Resource, Default)]
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
}

/// Mapping from room id → cutscene id to play the first time an actor enters
/// that room. Drained by the runtime auto-trigger system. Content populates the
/// pairs; this type carries no defaults of its own.
#[derive(Resource, Default)]
pub struct RoomCutsceneBindings {
    pub bindings: Vec<(String, String)>,
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

    #[test]
    fn skip_terminates_immediately() {
        let mut runtime = CutsceneRuntime::new(intro());
        let _ = runtime.tick(0.0, false);
        let evs = runtime.skip();
        assert!(evs.contains(&CutsceneEvent::Skipped));
        assert!(runtime.finished);
        // Subsequent ticks are no-ops.
        let evs = runtime.tick(1.0, true);
        assert!(evs.is_empty());
    }

    #[test]
    fn auto_advances_predicate_distinguishes_dialogue_from_others() {
        // Dialogue waits for player input.
        let dialogue = CutsceneBeat::Dialogue {
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

/// **The rollback wire format for playback state.**
///
/// ⛔ **`ActiveCutscene` is not presentation, and `rollback_coverage` waives the
/// whole `ambition_cutscene::` namespace as if it were.** `is_playing()` drives a
/// CAPTURING input-context claim, so while a cutscene plays the participant's
/// gameplay input is suppressed — whether the player can act is gameplay truth.
/// A rewind into a playing frame that did not restore this would let the
/// resimulation act through beats the original could not.
///
/// ⚠ **the SCRIPT is encoded, not looked up.** `SnapshotState::decode` has no
/// world, so resolving an id against `CutsceneLibrary` would need a follow-up
/// system — and "an authority that needs a second call" is a shape this repo has
/// been bitten by. A script is a handful of beats with short strings; paying
/// those bytes per snapshot buys a decode that is total.
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

    impl SnapshotState for CutsceneBeat {
        fn encode(&self, out: &mut Vec<u8>) {
            match self {
                Self::Wait { seconds } => {
                    put_u32(out, WAIT);
                    put_f32(out, *seconds);
                }
                Self::Dialogue { speaker, text } => {
                    put_u32(out, DIALOGUE);
                    put_str(out, speaker);
                    put_str(out, text);
                }
                Self::CameraPan { target, seconds } => {
                    put_u32(out, CAMERA_PAN);
                    put_f32(out, target[0]);
                    put_f32(out, target[1]);
                    put_f32(out, *seconds);
                }
                Self::Fade { to_alpha, seconds } => {
                    put_u32(out, FADE);
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
                },
                CAMERA_PAN => Self::CameraPan {
                    target: [reader.f32()?, reader.f32()?],
                    seconds: reader.f32()?,
                },
                FADE => Self::Fade {
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
                // ⚠ an unknown tag is a REFUSAL, not a default beat: a snapshot
                // written by a different build must not decode into a cutscene
                // that plays something else.
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
        }

        fn decode(reader: &mut Reader<'_>) -> Option<Self> {
            let id = reader.str()?.to_owned();
            // ⛔⛔ **this was `reader.bool()?.then(|| reader.str().map(str::to_owned))?`,
            // and it REFUSED every script without a seen flag.** `bool::then`
            // yields `None` when the bool is false, and the trailing `?` returns
            // that `None` from `decode` — so "this script has no seen flag"
            // decoded as "this snapshot is corrupt", and the whole cutscene was
            // dropped on restore rather than the flag being absent.
            //
            // ⚠ **found by writing the test the review asked for, not by the
            // review.** Every existing snapshot fixture happened to call
            // `.with_seen_flag(..)`, so the false branch had never been decoded
            // — a codec whose only tests exercise one side of its only branch.
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
            Some(Self {
                id,
                beats,
                seen_flag,
            })
        }
    }

    impl SnapshotState for ActiveCutscene {
        fn encode(&self, out: &mut Vec<u8>) {
            match &self.runtime {
                None => put_bool(out, false),
                Some(runtime) => {
                    put_bool(out, true);
                    runtime.script.encode(out);
                    put_u32(out, runtime.beat_index as u32);
                    put_f32(out, runtime.elapsed);
                    put_bool(out, runtime.finished);
                }
            }
        }

        fn decode(reader: &mut Reader<'_>) -> Option<Self> {
            // ⭐ **only `runtime` crosses, and the rest is DERIVED.** The
            // dialogue line, banner, camera target and fade alpha are what the
            // current beat has emitted — presentation the tick re-publishes as
            // beats enter. Encoding them would put four copies of the same fact
            // in every snapshot and invite them to disagree with the beat index
            // that produced them.
            if !reader.bool()? {
                return Some(Self::default());
            }
            let script = CutsceneScript::decode(reader)?;
            Some(Self {
                runtime: Some(CutsceneRuntime {
                    script,
                    beat_index: reader.u32()? as usize,
                    elapsed: reader.f32()?,
                    finished: reader.bool()?,
                }),
                ..Self::default()
            })
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

        /// **A script with NO seen flag decodes.**
        ///
        /// ⛔ **it did not, and the bug shipped.** `decode` read the flag as
        /// `reader.bool()?.then(|| reader.str().map(str::to_owned))?` —
        /// `bool::then` yields `None` when the bool is false and the trailing `?`
        /// returns it from the function, so *"this script has no seen flag"* was
        /// indistinguishable from *"this snapshot is corrupt"* and the whole
        /// cutscene was dropped on restore.
        ///
        /// ⚠ **every other fixture in this module calls `.with_seen_flag(..)`**,
        /// so the false branch of the only branch in this codec had never once
        /// been decoded. That is the shape to look for elsewhere: a test suite
        /// that exercises one side of a two-sided decision.
        #[test]
        fn a_script_without_a_seen_flag_still_decodes() {
            let script = CutsceneScript::new("no_flag", vec![CutsceneBeat::Wait { seconds: 0.25 }]);
            assert!(
                script.seen_flag.is_none(),
                "the fixture must have NO seen flag or it tests the branch that worked"
            );
            let before = ActiveCutscene {
                runtime: Some(CutsceneRuntime::new(script)),
                ..ActiveCutscene::default()
            };
            let after = round_trip(&before);
            assert_eq!(
                after.runtime, before.runtime,
                "a flagless script did not survive the wire, so every cutscene \
                 that does not write a save flag is dropped by any rollback"
            );
        }

        /// **A rollback into the MIDDLE of a beat restores its picture.**
        ///
        /// ⛔ **this is the test that was missing, and its absence let a false
        /// claim ship.** The snapshot encodes `runtime` only, and the four
        /// presentation fields it dropped were documented as *"derived, the next
        /// simulation tick republishes them"*. It could not:
        /// `CutsceneRuntime::tick` emits `BeatEntered` only when
        /// `elapsed == 0.0`, so a restore landing mid-beat never saw the entry
        /// event again — the banner, the camera target and the fade were gone
        /// for the rest of that beat. The old test round-tripped `runtime` and
        /// proved exactly that much. (GPT 5.6 through `32eb27a`, finding 1.)
        ///
        /// ⭐ **so this asserts the DERIVATION, not the encoding**: rebuild the
        /// picture from the decoded state and require it to equal the original's,
        /// mid-beat, where the event-driven version could not.
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

            let before = ActiveCutscene {
                presentation: runtime.presentation(),
                runtime: Some(runtime),
            };
            assert!(
                before.presentation.banner.is_some(),
                "the fixture is not showing anything, so restoring it proves nothing"
            );

            let after = round_trip(&before);
            let restored = after
                .runtime
                .as_ref()
                .expect("the runtime round-trips")
                .presentation();
            assert_eq!(
                restored, before.presentation,
                "the picture did not survive a mid-beat restore. This is the \
                 failure the old snapshot could not see: `runtime` round-tripped \
                 and the banner did not come back, because `BeatEntered` only \
                 fires on the tick a beat begins"
            );
            // ⚠ and the countdown is DERIVED, not a separately ticked timer —
            // 4.0 authored minus 1.5 elapsed. A timer would be a second
            // authority on the same clock and is exactly what could not survive.
            let (_, remaining) = restored.banner.expect("a banner beat is current");
            assert!(
                (remaining - 2.5).abs() < 1e-4,
                "the restored banner has {remaining}s left, not the 2.5 its \
                 authored duration minus its elapsed time says"
            );
        }

        /// **A beat that shows nothing does not leave the last beat's picture up.**
        ///
        /// ⛔ finding 2, and it needed no rollback to see: the presentation
        /// fields were mutated one at a time by whichever beat used them, so a
        /// `CameraPan` following a `Dialogue` set `camera_target` and left
        /// `dialogue` standing — the overlay kept showing a line nobody was
        /// saying. A projection cannot: it is one beat's whole answer.
        #[test]
        fn advancing_off_a_dialogue_takes_the_dialogue_with_it() {
            let script = CutsceneScript::new(
                "talk_then_pan",
                vec![
                    CutsceneBeat::Dialogue {
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
            assert_eq!(after.camera_target, Some([10.0, 20.0]));
        }

        /// **Every beat variant survives**, because a codec that silently loses
        /// one plays a different cutscene after a rewind.
        #[test]
        fn a_playing_cutscene_survives_the_wire() {
            let script = CutsceneScript::new(
                "intro",
                vec![
                    CutsceneBeat::Wait { seconds: 0.5 },
                    CutsceneBeat::Dialogue {
                        speaker: "Creator".into(),
                        text: "Wait.".into(),
                    },
                    CutsceneBeat::CameraPan {
                        target: [12.0, -3.5],
                        seconds: 1.25,
                    },
                    CutsceneBeat::Fade {
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
                ],
            )
            .with_seen_flag("seen_intro");
            let mut runtime = CutsceneRuntime::new(script);
            runtime.beat_index = 3;
            runtime.elapsed = 0.125;
            let before = ActiveCutscene {
                runtime: Some(runtime),
                ..ActiveCutscene::default()
            };
            let after = round_trip(&before);
            assert_eq!(after.runtime, before.runtime);
        }

        /// The absent case, which is what most frames encode.
        #[test]
        fn no_cutscene_survives_the_wire() {
            let before = ActiveCutscene::default();
            assert!(round_trip(&before).runtime.is_none());
        }

        /// ⛔ **an unknown beat tag REFUSES.** A snapshot from a build with a
        /// seventh variant must not decode into a cutscene that plays something
        /// else; `None` makes the mismatch visible instead of plausible.
        #[test]
        fn an_unknown_beat_tag_is_refused_rather_than_defaulted() {
            let mut bytes = Vec::new();
            put_bool(&mut bytes, true);
            put_str(&mut bytes, "intro");
            put_bool(&mut bytes, false);
            put_u32(&mut bytes, 1);
            put_u32(&mut bytes, 99);
            let mut reader = Reader::new(&bytes);
            assert!(ActiveCutscene::decode(&mut reader).is_none());
        }
    }
}
