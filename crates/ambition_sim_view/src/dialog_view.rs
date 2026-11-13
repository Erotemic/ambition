//! `DialogView` — the dialogue overlay's per-frame read-model (recon C3).
//!
//! This row snapshots presentation-neutral dialogue facts (visibility, dialogue id, stable speaker
//! identity, portrait clip, speaker / conversation labels, body, options, and selection), rebuilt
//! after the narrative frame work and before dialogue presentation. Presentation
//! consumes the snapshot.
//!
//! It is the one dialogue surface: a playing cutscene's line is published here
//! too, when no conversation is shown, so every line is drawn by the same box
//! with the same controls.

use ambition_dialog::DialogState;
use bevy::prelude::{DetectChanges, DetectChangesMut, Entity, Query, Res, ResMut, Resource};

/// The controls a dialogue box names, for the seat that answers it, in the
/// vocabulary of that seat's current device ("F" on a keyboard preset, "A" on
/// an Xbox pad, "Cross" on a DualShock). Empty when there is no control to
/// name (touch, or no bindings): the box names the action instead.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DialogControls {
    /// Advances the line or takes the selected choice (`MenuSelect`).
    pub confirm: String,
    /// Closes a conversation; held, skips a cutscene (`MenuBack`).
    pub back: String,
    /// The line is a cutscene's: back held skips the cutscene, and a tap does
    /// not close it.
    pub hold_to_skip: bool,
}

impl DialogControls {
    /// The controls of the seat `owner` names.
    pub fn of(
        owner: Option<ambition_input::InputOwner>,
        bindings: Option<&ambition_input::SeatBindings>,
        devices: Option<&ambition_input::SeatActiveDevices>,
        hold_to_skip: bool,
    ) -> Self {
        let (Some(owner), Some(bindings)) = (owner, bindings) else {
            return Self::default();
        };
        let slot = match owner {
            ambition_input::InputOwner::Participant(participant) => participant.0,
            ambition_input::InputOwner::Primary | ambition_input::InputOwner::AllParticipants => {
                ambition_input::ParticipantId::PRIMARY.0
            }
        };
        let device = devices.map_or_else(Default::default, |devices| devices.for_seat(slot));
        // On touch the on-screen buttons are their own glyphs; the box names
        // the actions instead.
        if device == ambition_input::ActiveDevice::Touch {
            return Self { hold_to_skip, ..Self::default() };
        }
        let seat = bindings.for_seat(slot);
        Self {
            confirm: seat.confirm_label(device).unwrap_or_default(),
            back: seat.back_label(device).unwrap_or_default(),
            hold_to_skip,
        }
    }
}

/// Per-frame snapshot of the dialogue overlay's facts. Empty/inactive when no
/// dialogue is running.
#[derive(Resource, Default, Clone, Debug, PartialEq)]
pub struct DialogView {
    /// The participant whose controls can advance this dialogue.
    pub input_owner: Option<ambition_input::InputOwner>,
    /// The simulation conversation from which this surface was built.
    pub instance: Option<ambition_conversation::ConversationInstanceId>,
    /// Whether a dialogue presenter should be visible at all: a conversation
    /// runs and no surface above it holds it (`DialogState::held`). A held
    /// conversation comes back from the same state when the surface closes.
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
    /// The controls the box names for its seat.
    pub controls: DialogControls,
}

impl DialogView {
    pub fn controls_seat(&self, seat: ambition_characters::control::PlayerSlot) -> bool {
        self.input_owner.is_some_and(|owner| owner.accepts(
            ambition_platformer2d_actor_monolith::participant_seat::participant_of(seat),
        ))
    }
}

/// Rebuild [`DialogView`] from the live [`DialogState`]. Change-gated: an idle
/// frame (no dialogue mutation) does no string work; during an active dialogue
/// the state mutates every reveal tick anyway, which is exactly when the
/// overlay needs fresh text.
///
/// `Option`: a composition without the dialogue capability (`Capability::Dialogue`)
/// has no `DialogState`, and the view stays at its idle default.
pub fn rebuild_dialog_view(
    dialogue: Option<Res<DialogState>>,
    conversation: Option<Res<ambition_conversation::ActiveConversation>>,
    // A cutscene's line, and the seat that answers it: the lowest seat in the
    // cutscene's room (`ActiveCutscene::answering_seat`, which the cutscene
    // tick reads too).
    (cutscene, drivers, rooms): (
        Option<Res<ambition_cutscene::ActiveCutscene>>,
        Query<(&ambition_characters::control::DrivingParticipant, Entity)>,
        ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    ),
    // What the box names for confirm and back.
    (bindings, devices): (
        Option<Res<ambition_input::SeatBindings>>,
        Option<Res<ambition_input::SeatActiveDevices>>,
    ),
    mut view: ResMut<DialogView>,
) {
    let conversation_shown = dialogue.as_deref().is_some_and(|dialogue| dialogue.active() && !dialogue.held());
    let line = cutscene
        .as_deref()
        .filter(|_| !conversation_shown)
        .and_then(|cutscene| cutscene.line().map(|line| (cutscene, line)));
    if let Some((cutscene, (speaker, text))) = line {
        let seat = cutscene.answering_seat(
            drivers.iter().map(|(driver, body)| (driver.0, rooms.of(body).map(|at| at.ordinal()))),
            ambition_characters::control::PlayerSlot::PRIMARY,
        );
        let owner = Some(ambition_input::InputOwner::Participant(
            ambition_platformer2d_actor_monolith::participant_seat::participant_of(seat),
        ));
        // Written only when it differs: the cutscene republishes its
        // presentation each tick, and a write here rebuilds the box.
        view.set_if_neq(DialogView {
            input_owner: owner,
            instance: None,
            active: true,
            dialogue_id: cutscene.script_id().unwrap_or_default().to_owned(),
            speaker_character_id: String::new(),
            portrait_clip: String::new(),
            speaker_label: speaker.to_owned(),
            conversation_label: speaker.to_owned(),
            body: text.to_owned(),
            option_labels: Vec::new(),
            selected_option: 0,
            controls: DialogControls::of(owner, bindings.as_deref(), devices.as_deref(), true),
        });
        return;
    }
    // A cutscene line that ended leaves no conversation behind it.
    if view.controls.hold_to_skip {
        *view = DialogView::default();
    }
    mirror_conversation(dialogue, conversation, &mut view);
    let controls = DialogControls::of(view.input_owner, bindings.as_deref(), devices.as_deref(), false);
    if view.controls != controls {
        view.controls = controls;
    }
}

/// Mirror the live conversation into the view, change-gated.
fn mirror_conversation(
    dialogue: Option<Res<DialogState>>,
    conversation: Option<Res<ambition_conversation::ActiveConversation>>,
    view: &mut ResMut<DialogView>,
) {
    let Some(dialogue) = dialogue else {
        if view.active || view.input_owner.is_some() || view.instance.is_some() {
            **view = DialogView::default();
        }
        return;
    };
    // Nothing to mirror while the state is unchanged and the view already
    // agrees it is open or closed. An open dialogue that waits for the player
    // writes nothing, so its box is not rebuilt each tick.
    let shown = dialogue.active() && !dialogue.held();
    if !dialogue.is_changed() && view.active == shown
        && conversation.as_ref().is_none_or(|conversation| !conversation.is_changed()) {
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
        input_owner,
        instance,
        active,
        dialogue_id,
        speaker_character_id,
        portrait_clip,
        speaker_label,
        conversation_label,
        body,
        option_labels,
        selected_option,
        // Written by the caller, from the seat this writes.
        controls: _,
    } = &mut **view;

    *active = shown;
    *input_owner = dialogue.input_owner();
    let next_instance = conversation.as_deref().and_then(|conversation| conversation.instance());
    if instance.as_ref() != next_instance {
        *instance = next_instance.cloned();
    }
    if !*active {
        *input_owner = None;
        *instance = None;
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
            ambition_input::InputOwner::Primary,
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

    /// An open dialogue that writes nothing leaves the view unchanged, so the
    /// box that draws it is not rebuilt each tick.
    #[test]
    fn a_waiting_dialogue_does_not_rewrite_the_view() {
        let mut app = App::new();
        app.init_resource::<DialogState>();
        app.init_resource::<DialogView>();
        app.add_systems(Update, rebuild_dialog_view);
        app.world_mut().resource_mut::<DialogState>().start(
            "intro_greeting",
            "Robo",
            ambition_dialog::DialogueContext::between("player", "npc_robo"),
            ambition_input::InputOwner::Primary,
        );
        app.update();
        assert!(app.world().resource::<DialogView>().active);
        // The tick of the last write. `is_changed` read from outside a system
        // is false after every `update`, which clears the trackers.
        let before = app.world().resource_ref::<DialogView>().last_changed();
        app.update();
        let view = app.world().resource_ref::<DialogView>();
        assert_eq!(view.last_changed(), before, "an unchanged open dialogue rewrote the view");
    }
}
