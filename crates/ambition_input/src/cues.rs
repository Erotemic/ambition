//! Resolved UI cues — what the submit-functional controls DO right now, in
//! the owning surface's own words.
//!
//! A surface that owns (or may own) an input context publishes a [`UiCue`]
//! keyed by its [`InputContextId`] — the shell's startup cards publish
//! "Continue", the launcher publishes the focused row's verb ("Play",
//! "Exit"), the app's inventory publishes the focused item's verb ("Equip" /
//! "Use"). Presenters never read these directly: the cue for the ACTIVE
//! context is folded into the one presenter-facing read-model
//! (`ambition_sim_view::ControlPrompt`) by whichever provider owns the
//! active context. This keys cues by context identity instead of hardcoding
//! a menu bridge, so gameplay's `ActionSchemeContract` labels and any future
//! surface (dialogue, vehicles) join the same vocabulary rather than a
//! parallel prompt system.
//!
//! A cue is presentation data only. It never carries device state and never
//! routes input; deleting every cue changes labels, not behavior.

use bevy::prelude::*;

use crate::participant::InputContextId;

/// One surface's published cue: the submit verb for its context.
///
/// Deliberately label-first today (matching `ControlPrompt`); glyphs, icons,
/// enabled state, hold/tap presentation, and accessibility descriptions grow
/// HERE as fields when a consumer exists for them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiCue {
    pub context: InputContextId,
    /// Ordering among simultaneously published cues; higher wins. Mirrors
    /// the owning context's claim priority.
    pub priority: i32,
    /// The submit-functional controls' verb: "Continue", "Play", "Equip".
    pub submit_label: String,
}

/// The published cues, keyed by context. Surfaces `sync` their own cue;
/// readers ask for [`ActiveUiCues::top`] or a specific context's cue.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct ActiveUiCues {
    cues: Vec<UiCue>,
}

impl ActiveUiCues {
    /// Upsert a cue by context. Idempotent for an unchanged cue.
    pub fn declare(&mut self, cue: UiCue) {
        match self.cues.iter_mut().find(|c| c.context == cue.context) {
            Some(existing) => *existing = cue,
            None => self.cues.push(cue),
        }
    }

    /// Remove a context's cue. Idempotent when absent.
    pub fn retract(&mut self, context: InputContextId) {
        self.cues.retain(|c| c.context != context);
    }

    /// Declare when `active`, retract when not. Returns whether the stored
    /// cues changed, so callers can avoid change-detection churn.
    pub fn sync(&mut self, cue: UiCue, active: bool) -> bool {
        let before = self.cues.clone();
        if active {
            self.declare(cue);
        } else {
            self.retract(cue.context);
        }
        before != self.cues
    }

    pub fn for_context(&self, context: InputContextId) -> Option<&UiCue> {
        self.cues.iter().find(|c| c.context == context)
    }

    /// The highest-priority published cue (ties break by context id, so the
    /// answer is deterministic regardless of publication order).
    pub fn top(&self) -> Option<&UiCue> {
        self.cues.iter().max_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then(b.context.0.cmp(a.context.0))
        })
    }
}

#[cfg(test)]
mod cue_tests {
    use super::*;

    const A: InputContextId = InputContextId("test.a");
    const B: InputContextId = InputContextId("test.b");

    fn cue(context: InputContextId, priority: i32, label: &str) -> UiCue {
        UiCue {
            context,
            priority,
            submit_label: label.to_owned(),
        }
    }

    #[test]
    fn the_top_cue_wins_by_priority_and_is_deterministic_on_ties() {
        let mut cues = ActiveUiCues::default();
        cues.declare(cue(A, 10, "Continue"));
        cues.declare(cue(B, 20, "Play"));
        assert_eq!(cues.top().map(|c| c.submit_label.as_str()), Some("Play"));

        let mut fwd = ActiveUiCues::default();
        fwd.declare(cue(A, 10, "one"));
        fwd.declare(cue(B, 10, "two"));
        let mut rev = ActiveUiCues::default();
        rev.declare(cue(B, 10, "two"));
        rev.declare(cue(A, 10, "one"));
        assert_eq!(fwd.top(), rev.top());
    }

    #[test]
    fn sync_updates_in_place_and_reports_real_changes_only() {
        let mut cues = ActiveUiCues::default();
        assert!(cues.sync(cue(A, 10, "Play"), true));
        assert!(!cues.sync(cue(A, 10, "Play"), true), "unchanged re-declare");
        assert!(cues.sync(cue(A, 10, "Exit"), true), "label change is real");
        assert_eq!(
            cues.for_context(A).map(|c| c.submit_label.as_str()),
            Some("Exit")
        );
        assert!(cues.sync(cue(A, 10, "Exit"), false));
        assert!(cues.top().is_none());
    }
}
