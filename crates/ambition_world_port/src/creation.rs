//! Create a checked actor composition and initialize its declared behavior state.

use ambition_extension_sdk::{Port, PortKey, PortRole, action::ReplyTo,
    wire::{WireError, WireReader}};

ambition_extension_sdk::port_choices! {
    #[derive(Default)]
    pub enum SpawnAllegiance {
        #[default]
        Inherit = 0 => "inherit",
        Player = 1 => "player",
        Enemy = 2 => "enemy",
        Npc = 3 => "npc",
        Boss = 4 => "boss",
        Neutral = 5 => "neutral",
    }
}
ambition_extension_sdk::port_choices! {
    pub enum ActorLinkKind { Grudge = 1 => "grudge", Mount = 2 => "mount" }
}

use crate::behavior::StateSeed;
#[derive(Clone, Debug, PartialEq)]
pub struct ActorNode {
    pub key: String,
    pub character: String,
    pub at: [f32; 2],
    pub half_extents: [f32; 2],
    pub allegiance: SpawnAllegiance,
    pub face_left: bool,
    pub starting_wallet: Option<i32>,
    pub holds: Vec<String>,
    pub state: Vec<StateSeed>,
    pub bindings: Vec<crate::behavior::BehaviorBinding>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ActorLink { pub from: String, pub to: String, pub kind: ActorLinkKind }
#[derive(Clone, Debug, PartialEq)]
pub struct CreateActors { pub reply: ReplyTo, pub actors: Vec<ActorNode>, pub links: Vec<ActorLink> }

ambition_extension_sdk::port_record! {
    ActorNode {
        /// A unique key within this batch. Links name these keys.
        key: String,
        /// A prepared character in the active session's cast.
        character: String,
        /// World position of the placement box, +Y down.
        at: [f32; 2],
        half_extents: [f32; 2] = [16.0, 24.0],
        allegiance: SpawnAllegiance = SpawnAllegiance::Inherit,
        face_left: bool = false,
        /// Initial wallet balance for this occurrence. Absent uses the character.
        starting_wallet: Option<i32> = None,
        /// Status selectors held from birth, through the native status owner.
        holds: Vec<String> = Vec::new(),
        state: Vec<StateSeed> = Vec::new(),
        /// Private behavior instances initialized before object publication.
        bindings: Vec<crate::behavior::BehaviorBinding> = Vec::new(),
    }
}
ambition_extension_sdk::port_record! { ActorLink { from: String, to: String, kind: ActorLinkKind } }
ambition_extension_sdk::port_record! {
    CreateActors {
        /// Report completed construction or refusal to this entry.
        reply: ReplyTo,
        /// Build these roots and each character's required internal cluster.
        actors: Vec<ActorNode>,
        /// Native relations between actor keys in this batch.
        links: Vec<ActorLink> = Vec::new(),
    }
}
pub struct CreateActorsPort;
impl Port for CreateActorsPort {
    const KEY: PortKey = PortKey::new("ambition.world.create_actors", 5);
    const ROLE: PortRole = PortRole::Request;
    type Value = CreateActors;
    fn action(value: &Self::Value) -> Option<ambition_extension_sdk::action::ActionSubmission<'_>> {
        Some(ambition_extension_sdk::action::ActionSubmission::encoded::<Self>(value, "world.create_actors", &value.reply))
    }
    fn encode(value: &Self::Value, out: &mut Vec<u8>) { ambition_extension_sdk::structure::put(value, out); }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> { ambition_extension_sdk::structure::read(reader) }
}
ambition_extension_sdk::script_port!(CreateActorsPort);

/// Retire an object this caller created (`create_actors`, `create_breakables`):
/// its owner takes it out of the world. Only the creator may: the object's
/// provenance names the caller as its parent. The answer is terminal
/// `creation_retired`, or a refusal (`unknown_object`, `not_your_creation`).
/// Its subscribers hear `object_retired` as for any retirement.
#[derive(Clone, Debug, PartialEq)]
pub struct RetireCreation { pub reply: Option<ReplyTo>, pub object: ambition_extension_sdk::ObjectRef }
ambition_extension_sdk::port_record! {
    RetireCreation {
        /// Optional correlated reply.
        reply: Option<ReplyTo> = None,
        /// The created object.
        object: ambition_extension_sdk::ObjectRef,
    }
}
pub const RETIRE_CREATION_DOMAIN: &str = "world.retire_creation";
pub struct RetireCreationPort;
impl Port for RetireCreationPort {
    const KEY: PortKey = PortKey::new("ambition.world.retire_creation", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = RetireCreation;
    fn action(value: &Self::Value) -> Option<ambition_extension_sdk::action::ActionSubmission<'_>> {
        Some(ambition_extension_sdk::action::ActionSubmission::encoded::<Self>(value, RETIRE_CREATION_DOMAIN, value.reply.as_ref()?))
    }
    fn encode(value: &Self::Value, out: &mut Vec<u8>) { ambition_extension_sdk::structure::put(value, out); }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> { ambition_extension_sdk::structure::read(reader) }
}
ambition_extension_sdk::script_port!(RetireCreationPort);
