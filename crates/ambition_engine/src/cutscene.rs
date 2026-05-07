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
    CameraPan {
        target: [f32; 2],
        seconds: f32,
    },
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
/// played by checking `SandboxSaveData::flag(seen_flag)`.
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
    BeatEntered {
        index: usize,
        beat: CutsceneBeat,
    },
    FlagWritten {
        id: String,
        on: bool,
    },
    Skipped,
    Completed,
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
