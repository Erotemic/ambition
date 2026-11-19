//! Authored bounds for native runtime construction in each live room.

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


/// Room-owned destructible creation bounds, shared by all controller kinds.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BreakableCreationLimits {
    pub max_breakables: u32,
    pub max_room_breakables: u32,
    pub max_label_bytes: u32,
}
impl Default for BreakableCreationLimits {
    fn default() -> Self { Self {max_breakables: 32, max_room_breakables: 512, max_label_bytes: 128} }
}
pub const BREAKABLE_CREATION_RULES_SCHEMA: &str = "breakable_creation_rules";

/// Bounds for reusable behaviors and their directed calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ObjectBehaviorLimits {
    pub max_bindings_per_object: u32,
    pub max_bindings_per_session: u32,
    pub max_methods: u32,
    pub max_references: u32,
    pub max_subscriptions: u32,
    pub max_event_history: u32,
    pub max_state_records: u32,
    pub max_state_bytes: u32,
    pub max_data_bytes: u32,
    pub max_calls_per_binding: u32,
    pub max_active_wait_ticks: u32,
}
impl Default for ObjectBehaviorLimits {
    fn default() -> Self {
        Self {max_bindings_per_object: 16, max_bindings_per_session: 1024, max_methods: 32, max_references: 32,
            max_subscriptions: 32, max_event_history: 64,
            max_state_records: 16, max_state_bytes: 16_384, max_data_bytes: 4096,
            max_calls_per_binding: 32, max_active_wait_ticks: 600}
    }
}
pub const OBJECT_BEHAVIOR_RULES_SCHEMA: &str = "object_behavior_rules";
