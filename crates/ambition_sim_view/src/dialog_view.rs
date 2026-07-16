//! `DialogView` — the dialogue overlay's per-frame read-model (recon C3).
//!
//! The render layer's dialog UI used to read `ambition_dialog::DialogState`
//! live — the renderer's only reason to depend on the dialogue runtime. This
//! row snapshots exactly the five facts the overlay draws (visibility, title,
//! body, option labels, selection), rebuilt sim-side in the FeatureViewSync
//! tail like every other read-model; presentation is a pure consumer.

use ambition_dialog::DialogState;
use bevy::prelude::{DetectChanges, Res, ResMut, Resource};

/// Per-frame snapshot of the dialogue overlay's facts. Empty/inactive when no
/// dialogue is running.
#[derive(Resource, Default, Clone, Debug)]
pub struct DialogView {
    /// Whether the dialogue overlay should be visible at all.
    pub active: bool,
    /// The overlay's title line (speaker — npc).
    pub title: String,
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
    view.active = dialogue.active();
    if !view.active {
        view.title.clear();
        view.body.clear();
        view.option_labels.clear();
        view.selected_option = 0;
        return;
    }
    view.title = dialogue.title();
    view.body = dialogue.body();
    view.option_labels.clear();
    view.option_labels
        .extend(dialogue.options().iter().map(|o| o.label.clone()));
    view.selected_option = dialogue.selected_option();
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
            ambition_dialog::DialogueContext::default(),
        );
        app.update();
        let view = app.world().resource::<DialogView>();
        assert!(view.active);
        assert!(view.title.contains("Robo"), "title: {}", view.title);
        assert_eq!(view.selected_option, 0);

        app.world_mut().resource_mut::<DialogState>().close();
        app.update();
        let view = app.world().resource::<DialogView>();
        assert!(!view.active);
        assert!(view.body.is_empty() && view.title.is_empty());
    }
}
