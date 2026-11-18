//! Authored bounds for character compositions in each live room.

/// Games can declare this rule for their own rooms.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ActorCreationLimits {
    pub max_actors: u32,
    pub max_rows: u32,
    pub max_room_bodies: u32,
    pub max_links: u32,
    pub max_holds: u32,
    pub max_state_bytes: u32,
    pub max_state_records: u32,
}
impl Default for ActorCreationLimits {
    fn default() -> Self {
        Self { max_actors: 32, max_rows: 96, max_room_bodies: 512,
            max_links: 64, max_holds: 16, max_state_bytes: 16_384, max_state_records: 16 }
    }
}


/// Content schema for these room rules.
pub const ACTOR_CREATION_RULES_SCHEMA: &str = "actor_creation_rules";
