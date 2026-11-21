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

/// The conversation the main text runner is running: presentation state, set
/// by [`project_the_dialog_ui_from_the_conversation`]. A narrative command or a
/// narrative end the main runner reports is that conversation's. A
/// conversation beside it runs on a [`ambition_dialog::DialogSlot`] entity,
/// which names its own in [`SlotConversation`] (SIMULATION-ISLANDS N2).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct PresentedConversation(pub Option<ConversationInstanceId>);

/// The conversation a [`ambition_dialog::DialogSlot`]'s runner is running: the
/// slot's [`PresentedConversation`]. `None` while the slot is free.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotConversation(pub Option<ConversationInstanceId>);

/// The conversation of the runner whose command or function is running: a
/// slot's, when a slot runner speaks ([`ambition_dialog::SpeakingRunner`]), and
/// otherwise the main runner's.
#[derive(bevy::ecs::system::SystemParam)]
pub struct SpeakingConversation<'w, 's> {
    presented: Option<Res<'w, PresentedConversation>>,
    speaking: Option<Res<'w, ambition_dialog::SpeakingRunner>>,
    slots: Query<'w, 's, &'static SlotConversation>,
}

impl SpeakingConversation<'_, '_> {
    pub fn instance(&self) -> Option<&ConversationInstanceId> {
        let slot = self.speaking.as_deref().and_then(|speaking| speaking.0).and_then(|runner| self.slots.get(runner).ok());
        match slot {
            Some(slot) => slot.0.as_ref(),
            None => self.presented.as_deref()?.0.as_ref(),
        }
    }
}

/// [`SpeakingConversation::instance`] for code that holds the `World` (a Yarn
/// function with world access).
pub fn speaking_conversation_instance(world: &World) -> Option<ConversationInstanceId> {
    let speaking = world.get_resource::<ambition_dialog::SpeakingRunner>().and_then(|speaking| speaking.0);
    if let Some(slot) = speaking.and_then(|runner| world.get::<SlotConversation>(runner)) {
        return slot.0.clone();
    }
    world.get_resource::<PresentedConversation>()?.0.clone()
}

impl PresentedConversation {
    /// The live conversation the runner presents: the one an authored command
    /// the runner fires acts on.
    pub fn live<'a>(&self, conversation: &'a ActiveConversation) -> Option<&'a super::authority::LiveConversation> {
        conversation.of_instance(self.0.as_ref()?)
    }
}

/// The conversation of the runner that fires a Yarn command or function:
/// that conversation's talker, initiator and seat.
#[derive(bevy::ecs::system::SystemParam)]
pub struct RunnerConversation<'w, 's> {
    conversation: Option<Res<'w, ActiveConversation>>,
    speaking: SpeakingConversation<'w, 's>,
}

impl RunnerConversation<'_, '_> {
    /// The live conversation of the runner that fired the command.
    pub fn live(&self) -> Option<&super::authority::LiveConversation> {
        self.conversation.as_deref()?.of_instance(self.speaking.instance()?)
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
    slots: Query<&ambition_dialog::DialogSlot>,
    mut was: Local<bool>,
) -> bool {
    let now = conversation.is_live() || dialog.active() || slots.iter().any(|slot| slot.0.active());
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
    slots: Query<(&SlotConversation, &ambition_dialog::DialogSlot)>,
    mut narrative: NarrativeInputWriter<ConversationEnded>,
) {
    let runners = std::iter::once((presented.0.as_ref(), &*dialog))
        .chain(slots.iter().map(|(attached, slot)| (attached.0.as_ref(), &slot.0)));
    for (attached, dialog) in runners {
        let Some(instance) = attached.filter(|instance| conversation.of_instance(instance).is_some()) else {
            continue;
        };
        if !dialog.active() {
            narrative.write_once_for(instance.clone(), ConversationEnded { instance: instance.clone() });
        }
    }
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

/// Project the live simulation conversations onto the text runners.
///
/// A runner keeps its conversation until it ends: the main runner (the
/// [`ambition_dialog::DialogState`] resource) takes the first that needs one,
/// and each further live conversation takes a free [`ambition_dialog::DialogSlot`]
/// or a new one. A runner whose conversation ended and that takes no other is
/// closed. The same instance leaves its runner untouched; `attached` is
/// presentation-local state and is intentionally not rewound.
pub fn project_the_dialog_ui_from_the_conversation(
    mut commands: Commands,
    conversation: Res<ActiveConversation>,
    mut dialog: ResMut<ambition_dialog::DialogState>,
    mut presented: ResMut<PresentedConversation>,
    mut slots: Query<(Entity, &mut SlotConversation, &mut ambition_dialog::DialogSlot)>,
    settings: ambition_platformer2d_world::governing_rules::RulesOf<ambition_dialog::DialogSettings>,
) {
    let live = |instance: &Option<ConversationInstanceId>| {
        instance.as_ref().and_then(|instance| conversation.of_instance(instance))
    };
    // Keep each runner's conversation while it lives, following its seat.
    if let Some(kept) = live(&presented.0) {
        if dialog.input_owner() != Some(kept.input_owner) {
            dialog.set_input_owner(kept.input_owner);
        }
    } else {
        presented.0 = None;
    }
    for (_, mut attached, mut slot) in &mut slots {
        match live(&attached.0) {
            Some(kept) => {
                if slot.0.input_owner() != Some(kept.input_owner) {
                    slot.0.set_input_owner(kept.input_owner);
                }
            }
            None if attached.0.is_some() => attached.0 = None,
            None => {}
        }
    }
    // Give each conversation no runner holds the first free one, in the order
    // the conversations opened.
    let held: Vec<ConversationInstanceId> = presented.0.iter().cloned()
        .chain(slots.iter().filter_map(|(_, attached, _)| attached.0.clone()))
        .collect();
    let mut free_slots: Vec<Entity> = slots.iter()
        .filter(|(_, attached, _)| attached.0.is_none())
        .map(|(entity, ..)| entity)
        .collect();
    free_slots.reverse();
    for live in conversation.conversations().filter(|live| !held.contains(&live.instance)) {
        // Instance identity includes dialogue context, so corrected participants restart Yarn state.
        let start = |state: &mut ambition_dialog::DialogState| state.start_with_settings(
            live.dialogue_id(), &live.speaker_name, live.context(), live.input_owner,
            settings.of_subject(live.initiator.or(live.talker)).copied().unwrap_or_default());
        if presented.0.is_none() {
            presented.0 = Some(live.instance.clone());
            start(&mut dialog);
        } else if let Some(entity) = free_slots.pop() {
            let (_, mut attached, mut slot) = slots.get_mut(entity).expect("a free slot listed this frame");
            attached.0 = Some(live.instance.clone());
            start(&mut slot.0);
        } else {
            let mut slot = ambition_dialog::DialogSlot::default();
            start(&mut slot.0);
            commands.spawn((slot, SlotConversation(Some(live.instance.clone()))));
        }
    }
    // A runner left with nothing to present closes its box.
    if presented.0.is_none() && dialog.active() {
        dialog.close();
    }
    for entity in free_slots {
        if let Ok((_, _, mut slot)) = slots.get_mut(entity) {
            if slot.0.active() {
                slot.0.close();
            }
        }
    }
}
