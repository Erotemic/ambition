//! `DialogView` — the dialogue overlay's per-frame read-model (recon C3).
//!
//! This row snapshots presentation-neutral dialogue facts (visibility, dialogue id, stable speaker
//! identity, portrait clip, speaker / conversation labels, body, options, and selection), rebuilt
//! sim-side in the FeatureViewSync tail like every other read-model; presentation is a pure
//! consumer.

use ambition_dialog::DialogState;
use bevy::prelude::{DetectChanges, Res, ResMut, Resource};

/// Per-frame snapshot of the dialogue overlay's facts. Empty/inactive when no
/// dialogue is running.
#[derive(Resource, Default, Clone, Debug)]
pub struct DialogView {
    /// Whether a dialogue presenter should be visible at all.
    pub active: bool,
    /// Stable authored dialogue / Yarn node id. Presentation may use this to
    /// select game-owned framing, portraits, or other visual policy.
    pub dialogue_id: String,
    /// Stable character id whose portrait represents the current line. Empty
    /// only when the conversation has no identified endpoint and authored
    /// dialogue has not selected one explicitly.
    pub speaker_character_id: String,
    /// Optional named portrait clip requested by authored dialogue. Empty means
    /// the selected character's catalog default clip.
    pub portrait_clip: String,
    /// Human-facing speaker label for the current line. This is intentionally
    /// raw data rather than a renderer-formatted title.
    pub speaker_label: String,
    /// Human-facing label of the conversation endpoint that opened the
    /// dialogue. It may differ from `speaker_label` when Yarn changes speaker.
    pub conversation_label: String,
    /// The current (typewriter-revealed) body text.
    pub body: String,
    /// The currently REVEALED option labels, in presentation order. Empty for
    /// a non-branching line.
    pub option_labels: Vec<String>,
    /// Index of the selected option (into `option_labels`).
    pub selected_option: usize,
}

/// Rebuild [`DialogView`] from the live [`DialogState`]. Change-gated: an idle
/// frame (no dialogue mutation) does no string work; during an active dialogue
/// the state mutates every reveal tick anyway, which is exactly when the
/// overlay needs fresh text.
pub fn rebuild_dialog_view(dialogue: Res<DialogState>, mut view: ResMut<DialogView>) {
    if !dialogue.is_changed() && !view.active && !dialogue.active() {
        return;
    }
    // ⭐⭐ ONE DESTRUCTURE, NO `..`, COVERING BOTH BRANCHES. Every field of the
    // view was written by hand TWICE here — once to blank it when the dialogue
    // ends, once to refill it while one runs — so a TENTH field would have been
    // missed by both lists and leaked its previous dialogue's value into the
    // next. This row is what the overlay DRAWS, so that leak is a visible wrong
    // portrait or a stale line, not an internal inconsistency.
    //
    // ⇒ Adding a field is now `E0027` right here, and the author is asked the two
    // questions the two branches ask: what is this when idle, and where does it
    // come from when active?
    //
    // ⚠ BINDINGS, NOT A STRUCT LITERAL, and deliberately. `*view = DialogView
    // { .. }` would also be exhaustive (E0063) and would throw away every String
    // buffer on a row that rebuilds on EVERY REVEAL TICK of an active dialogue —
    // the allocation this function's change-gate exists to avoid.
    let DialogView {
        active,
        dialogue_id,
        speaker_character_id,
        portrait_clip,
        speaker_label,
        conversation_label,
        body,
        option_labels,
        selected_option,
    } = &mut *view;

    *active = dialogue.active();
    if !*active {
        dialogue_id.clear();
        speaker_character_id.clear();
        portrait_clip.clear();
        speaker_label.clear();
        conversation_label.clear();
        body.clear();
        option_labels.clear();
        *selected_option = 0;
        return;
    }
    dialogue_id.clear();
    dialogue_id.push_str(dialogue.dialogue_id());
    speaker_character_id.clear();
    speaker_character_id.push_str(dialogue.speaker_character_id());
    portrait_clip.clear();
    portrait_clip.push_str(dialogue.portrait_clip());
    speaker_label.clear();
    speaker_label.push_str(dialogue.speaker_label());
    conversation_label.clear();
    conversation_label.push_str(dialogue.conversation_label());
    *body = dialogue.body();
    option_labels.clear();
    option_labels.extend(dialogue.options().iter().map(|o| o.label.clone()));
    *selected_option = dialogue.selected_option();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    /// The view mirrors an active dialogue's facts and clears when it ends —
    /// the overlay reads THIS row, never the live `DialogState`.
    #[test]
    fn dialog_view_mirrors_state_and_clears_on_close() {
        let mut app = App::new();
        app.init_resource::<DialogState>();
        app.init_resource::<DialogView>();
        app.add_systems(Update, rebuild_dialog_view);

        // Pre-poison the view so the first rebuild must overwrite it.
        {
            let mut view = app.world_mut().resource_mut::<DialogView>();
            view.body = "stale".to_string();
            view.selected_option = 7;
        }
        app.world_mut().resource_mut::<DialogState>().start(
            "intro_greeting",
            "Robo",
            ambition_dialog::DialogueContext::between("player", "npc_robo"),
        );
        app.world_mut()
            .resource_mut::<DialogState>()
            .set_portrait_clip("speaking");
        app.update();
        let view = app.world().resource::<DialogView>();
        assert!(view.active);
        assert_eq!(view.dialogue_id, "intro_greeting");
        assert_eq!(view.speaker_character_id, "npc_robo");
        assert_eq!(view.portrait_clip, "speaking");
        assert_eq!(view.speaker_label, "Robo");
        assert_eq!(view.conversation_label, "Robo");
        assert_eq!(view.selected_option, 0);

        app.world_mut().resource_mut::<DialogState>().close();
        app.update();
        let view = app.world().resource::<DialogView>();
        assert!(!view.active);
        assert!(view.body.is_empty());
        assert!(view.dialogue_id.is_empty());
        assert!(view.speaker_character_id.is_empty());
        assert!(view.portrait_clip.is_empty());
        assert!(view.speaker_label.is_empty());
        assert!(view.conversation_label.is_empty());
    }
}
