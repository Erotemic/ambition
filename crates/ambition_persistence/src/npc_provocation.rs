//! Durable provocation facts and their saved-flag reducer.

/// A talkable body's durable provocation changed.
#[derive(bevy::prelude::Message, Clone, Debug, PartialEq, Eq)]
pub struct NpcProvocationChanged {
    /// The NPC's `ActorIdentity::id` — the placement identity construction
    /// looks the fate up by.
    pub id: String,
    pub provoked: bool,
}



/// Record each [`NpcProvocationChanged`] as the NPC's persisted-hostile flag.
pub fn record_npc_provocations(
    mut changes: bevy::prelude::MessageReader<NpcProvocationChanged>,
    mut save: bevy::prelude::ResMut<crate::save::AmbitionGameSave>,
    mut quests: bevy::prelude::ResMut<crate::quest::QuestRegistry>,
) {
    for change in changes.read() {
        crate::quest::write_flag(
            &mut save,
            &mut quests,
            crate::save_data::npc_hostile_flag(&change.id),
            change.provoked,
        );
    }
}
