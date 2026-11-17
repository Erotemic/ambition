//! Durable provocation facts and their saved-flag reducer.

/// A talkable body's durable provocation changed.
#[derive(bevy::prelude::Message, Clone, Debug, PartialEq, Eq)]
pub struct NpcProvocationChanged {
    /// The NPC's `ActorIdentity::id` — the placement identity construction
    /// looks the fate up by.
    pub id: String,
    pub provoked: bool,
    /// The ordinal of the live room the NPC is in, when it is in one: the
    /// runtime logs the flag write in that room, so a rewind of the room
    /// takes it back (ISL-2d).
    pub room: Option<u32>,
}



/// Record each [`NpcProvocationChanged`] as the NPC's persisted-hostile flag,
/// unlogged: the reducer for a composition without room rewinds. The runtime
/// installs `apply_npc_provocations`, which also logs each write in its room.
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
