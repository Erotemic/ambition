//! A sign that names controls.
//!
//! Authored sign text can name a control by the action it fires:
//! `{action:jump}`. The text shows the key or button the local primary seat
//! presses for that action, read from the one control prompt
//! (`ambition_sim_view::ControlPrompt::fill_legend`). So the sign follows a
//! rebind, a pad, and the body the player drives, and it cannot name a key
//! that does nothing.

use bevy::prelude::*;

use ambition_sim_view::{ControlPrompt, LEGEND_ACTION_OPEN};

use super::label_layout::MirroredWorldLabel;

/// The authored text of a sign that names controls by action.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ControlLegend {
    template: String,
}

impl ControlLegend {
    /// The legend for authored sign text, if the text names a control.
    pub fn for_text(text: &str) -> Option<Self> {
        text.contains(LEGEND_ACTION_OPEN).then(|| Self {
            template: text.to_owned(),
        })
    }

    /// The text before any prompt exists: every control is `?`.
    pub fn unresolved_text(&self) -> String {
        ControlPrompt::default().fill_legend(&self.template)
    }
}

/// Write each legend sign, and each view's copy of it, from the prompt.
///
/// It writes when the prompt changes and when a legend or a copy appears. A
/// copy is spawned with its root's text, but the root's text changes later.
pub fn write_control_legends(
    prompt: Option<Res<ControlPrompt>>,
    mut legends: Query<(Entity, Ref<ControlLegend>, &mut Text2d)>,
    mut copies: Query<(Ref<MirroredWorldLabel>, &mut Text2d), Without<ControlLegend>>,
) {
    let Some(prompt) = prompt else { return };
    let new_copy = copies.iter().any(|(copy, _)| copy.is_added());
    let mut written: Vec<(Entity, String)> = Vec::new();
    for (entity, legend, mut text) in &mut legends {
        if !(prompt.is_changed() || legend.is_added() || new_copy) {
            continue;
        }
        let wanted = prompt.fill_legend(&legend.template);
        if text.0 != wanted {
            text.0 = wanted.clone();
        }
        written.push((entity, wanted));
    }
    if written.is_empty() {
        return;
    }
    for (copy, mut text) in &mut copies {
        if let Some((_, wanted)) = written.iter().find(|(root, _)| *root == copy.root) {
            if &text.0 != wanted {
                text.0 = wanted.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_sim_view::{ActionId, ControlContextKind, ControlSlot, PromptEntry};

    fn prompt(jump: &str) -> ControlPrompt {
        ControlPrompt {
            context: ControlContextKind::Gameplay,
            entries: vec![PromptEntry {
                action: ActionId::new("jump"),
                slot: ControlSlot::Jump,
                label: "Jump".to_owned(),
                visual: None,
                binding: Some(jump.to_owned()),
                ready: true,
            }],
            menu_confirm: None,
        }
    }

    /// The sign and each view's copy of it name the key the prompt binds, and
    /// both follow a rebind.
    #[test]
    fn a_legend_sign_and_its_copies_follow_the_prompt() {
        let mut app = App::new();
        app.add_systems(Update, write_control_legends);
        app.insert_resource(prompt("Z"));
        let legend = ControlLegend::for_text("{action:jump}: JUMP").expect("names a control");
        let sign = app
            .world_mut()
            .spawn((Text2d::new(legend.unresolved_text()), legend))
            .id();
        app.update();
        let copy = app
            .world_mut()
            .spawn((
                Text2d::new("?: JUMP"),
                MirroredWorldLabel { root: sign },
            ))
            .id();
        app.update();
        let text = |app: &App, entity: Entity| app.world().get::<Text2d>(entity).unwrap().0.clone();
        assert_eq!(text(&app, sign), "Z: JUMP");
        assert_eq!(text(&app, copy), "Z: JUMP", "a new copy did not get the sign's text");

        app.insert_resource(prompt("K"));
        app.update();
        assert_eq!(text(&app, sign), "K: JUMP", "the sign kept the old key after a rebind");
        assert_eq!(text(&app, copy), "K: JUMP", "the copy kept the old key after a rebind");
    }

    /// Plain sign text is not a legend.
    #[test]
    fn text_that_names_no_control_is_not_a_legend() {
        assert_eq!(ControlLegend::for_text("LOOP"), None);
    }
}
