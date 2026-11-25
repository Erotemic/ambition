//! Build a mechanism from parts of different native domains in one request:
//! lifts the platform domain moves and switches that send them. A switch
//! names its lift by the lift's local key in the same request, so the
//! binding is complete when the batch commits.

use ambition_extension_sdk::{
    action::ReplyTo,
    wire::{WireError, WireReader},
    Port, PortKey, PortRole,
};

#[derive(Clone, Debug, PartialEq)]
pub struct LiftNode {
    pub key: String,
    pub at: [f32; 2],
    pub size: [f32; 2],
    pub rise: f32,
    pub speed: f32,
}
ambition_extension_sdk::port_record! {
    LiftNode {
        /// Unique key within this request; switches name the lift by it.
        key: String,
        /// Centre of the lift at its first stop, in the caller's room chart, +Y down.
        at: [f32; 2],
        /// Full width and height, both positive.
        size: [f32; 2] = [96.0, 16.0],
        /// The signed distance to its second stop (negative travels UP).
        rise: f32,
        /// Travel speed, px/s.
        speed: f32 = 120.0,
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct SwitchNode {
    pub key: String,
    pub at: [f32; 2],
    pub size: [f32; 2],
    pub prompt: String,
    pub sends: crate::object_target::ObjectTarget,
}
ambition_extension_sdk::port_record! {
    SwitchNode {
        /// Unique key within this request.
        key: String,
        /// Centre in the caller's room chart, +Y down.
        at: [f32; 2],
        size: [f32; 2] = [24.0, 32.0],
        prompt: String = "Use".to_string(),
        /// The lift a press sends to its other stop: `#{local: "lift_key"}`.
        sends: crate::object_target::ObjectTarget,
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CreateMechanism {
    pub reply: ReplyTo,
    pub lifts: Vec<LiftNode>,
    pub switches: Vec<SwitchNode>,
}
ambition_extension_sdk::port_record! {
    CreateMechanism { reply: ReplyTo, lifts: Vec<LiftNode> = Vec::new(), switches: Vec<SwitchNode> = Vec::new() }
}
pub struct CreateMechanismPort;
impl Port for CreateMechanismPort {
    const KEY: PortKey = PortKey::new("ambition.world.create_mechanism", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = CreateMechanism;
    fn action(value: &Self::Value) -> Option<ambition_extension_sdk::action::ActionSubmission<'_>> {
        Some(ambition_extension_sdk::action::ActionSubmission::encoded::<Self>(value, "world.create_mechanism", &value.reply))
    }
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(value, out);
    }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        ambition_extension_sdk::structure::read(reader)
    }
}
ambition_extension_sdk::script_port!(CreateMechanismPort);
