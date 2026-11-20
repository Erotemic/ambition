//! Bridge conversation simulation authority and the presentation text runner.
//!
//! Simulation-owned [`ActiveConversation`] projects openings to presentation.
//! Narrative completion is external input and is stamped into
//! [`super::NarrativeInputLedger`] at the tick where it applies so rollback can
//! replay it deterministically. Presentation state itself is not simulation
//! authority.

use bevy::prelude::*;
use ambition_platformer2d_core::replay_messages::ReplayMessageReader;

use super::authority::ActiveConversation;
use super::instance::ConversationInstanceId;
use super::ledger::NarrativeInputWriter;

/// Narrative completion for one conversation instance.
///
/// Carrying the instance prevents an end observed for one conversation from
/// closing a later conversation.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct ConversationEnded {
    pub instance: ConversationInstanceId,
}

/// The conversation the text runner is running: presentation state, set by
/// [`project_the_dialog_ui_from_the_conversation`]. A narrative command or a
/// narrative end the runner reports is that conversation's. One runner
/// presents one conversation (SIMULATION-ISLANDS N2 gives each its own).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct PresentedConversation(pub Option<ConversationInstanceId>);

impl PresentedConversation {
    /// The live conversation the runner presents: the one an authored command
    /// the runner fires acts on.
    pub fn live<'a>(&self, conversation: &'a ActiveConversation) -> Option<&'a super::authority::LiveConversation> {
        conversation.of_instance(self.0.as_ref()?)
    }
}

/// The conversation the runner presents, for a Yarn command or function the
/// runner fires: that conversation's talker, initiator and seat.
#[derive(bevy::ecs::system::SystemParam)]
pub struct RunnerConversation<'w> {
    conversation: Option<Res<'w, ActiveConversation>>,
    presented: Option<Res<'w, PresentedConversation>>,
}

impl RunnerConversation<'_> {
    /// The live conversation the runner presents.
    pub fn live(&self) -> Option<&super::authority::LiveConversation> {
        self.presented.as_deref()?.live(self.conversation.as_deref()?)
    }

    /// The body being talked to.
    pub fn talker(&self) -> Option<Entity> {
        self.live().and_then(|live| live.talker)
    }

    /// The body that started it.
    pub fn initiator(&self) -> Option<Entity> {
        self.live().and_then(|live| live.initiator)
    }

    /// Whose input drives it.
    pub fn input_owner(&self) -> Option<ambition_input::InputOwner> {
        self.live().map(|live| live.input_owner)
    }

    /// The seat whose dialogue history it is.
    pub fn history_seat(&self) -> u8 {
        self.live().map_or(0, super::authority::LiveConversation::history_seat)
    }
}

/// Run condition of the dialog projection and the end stamp: a conversation
/// is live or the dialog UI is shown, or one was last frame, the run that
/// forgets the conversation the projection had attached.
pub fn a_conversation_is_live_or_shown(
    conversation: Res<ActiveConversation>,
    dialog: Res<ambition_dialog::DialogState>,
    mut was: Local<bool>,
) -> bool {
    let now = conversation.is_live() || dialog.active();
    let run = now || *was;
    *was = now;
    run
}

/// Run condition of the sim passes over a live conversation.
pub fn a_conversation_is_live(conversation: Res<ActiveConversation>) -> bool {
    conversation.is_live()
}

/// Stamp narrative completion into simulation input once per conversation.
///
/// The runner remains finished until simulation closes the authority, so
/// idempotent stamping is required to preserve the original simulation tick
/// across rollback.
pub fn publish_the_narrative_end(
    conversation: Res<ActiveConversation>,
    presented: Res<PresentedConversation>,
    dialog: Res<ambition_dialog::DialogState>,
    mut narrative: NarrativeInputWriter<ConversationEnded>,
) {
    let Some(instance) = presented.0.clone().filter(|instance| conversation.of_instance(instance).is_some()) else {
        return;
    };
    if dialog.active() {
        return;
    }
    narrative.write_once(ConversationEnded { instance });
}

/// Close simulation authority from the stamped narrative-completion input.
///
/// The ledger replays completion on the same `SimTick`; Yarn execution itself remains presentation
/// work outside rollback. This runs before continuity checks so an ended conversation is not barked.
pub fn close_conversation_on_narrative_end(
    mut conversation: ResMut<ActiveConversation>,
    mut ended: ReplayMessageReader<ConversationEnded>,
) {
    for end in ended.read() {
        conversation.close_instance(&end.instance);
    }
}

/// Project the current simulation conversation into the presentation text runner.
///
/// No authority closes/detaches the box; a different instance starts the runner; the same instance
/// leaves it untouched. `attached` is presentation-local state and is intentionally not rewound.
pub fn project_the_dialog_ui_from_the_conversation(
    conversation: Res<ActiveConversation>,
    mut dialog: ResMut<ambition_dialog::DialogState>,
    mut presented: ResMut<PresentedConversation>,
    settings: ambition_platformer2d_world::governing_rules::RulesOf<ambition_dialog::DialogSettings>,
) {
    let attached = &mut presented.0;
    // The one runner presents the earliest conversation still live.
    let Some(live) = conversation.conversations().next() else {
        // No conversation authority means the shared dialog surface must be closed and detached.
        *attached = None;
        if dialog.active() {
            dialog.close();
        }
        return;
    };
    if dialog.input_owner() != Some(live.input_owner) {
        dialog.set_input_owner(live.input_owner);
    }
    if attached.as_ref() == Some(&live.instance) {
        return;
    }
    *attached = Some(live.instance.clone());
    // Instance identity includes dialogue context, so corrected participants restart Yarn state.
    dialog.start_with_settings(live.dialogue_id(), &live.speaker_name, live.context(), live.input_owner,
        settings.of_subject(live.initiator.or(live.talker)).copied().unwrap_or_default());
}
