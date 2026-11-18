//! Create fresh room-owned destructibles through the native world owner.

use ambition_extension_sdk::{
    action::ReplyTo,
    wire::{WireError, WireReader},
    Port, PortKey, PortRole,
};

ambition_extension_sdk::port_choices! {
    #[derive(Default)]
    pub enum BreakableCollision {
        None = 0 => "none",
        #[default]
        Solid = 1 => "solid",
        OneWay = 2 => "one_way",
    }
}
ambition_extension_sdk::port_choices! {
    #[derive(Default)]
    pub enum BreakableTrigger {
        #[default]
        Hit = 0 => "hit",
        Stand = 1 => "stand",
        Either = 2 => "either",
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BreakableNode {
    pub key: String,
    pub label: Option<String>,
    pub at: [f32; 2],
    pub size: [f32; 2],
    pub health: i32,
    pub collision: BreakableCollision,
    pub trigger: BreakableTrigger,
    pub respawn_after: Option<f32>,
    pub pogo_refresh: bool,
    pub look: Option<String>,
}
ambition_extension_sdk::port_record! {
    BreakableNode {
        /// Unique key within this batch. Replies use request order.
        key: String,
        label: Option<String> = None,
        /// Centre in the caller's room chart, +Y down.
        at: [f32; 2],
        /// Full width and height, both positive.
        size: [f32; 2],
        health: i32 = 1,
        collision: BreakableCollision = BreakableCollision::Solid,
        trigger: BreakableTrigger = BreakableTrigger::Hit,
        /// Respawn after consumption while the room remains live. Positive seconds.
        respawn_after: Option<f32> = None,
        pogo_refresh: bool = false,
        /// Prop sheet identity. Missing art uses the normal breakable fallback.
        look: Option<String> = None,
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CreateBreakables {
    pub reply: ReplyTo,
    pub objects: Vec<BreakableNode>,
}
ambition_extension_sdk::port_record! {
    CreateBreakables { reply: ReplyTo, objects: Vec<BreakableNode> }
}
pub struct CreateBreakablesPort;
impl Port for CreateBreakablesPort {
    const KEY: PortKey = PortKey::new("ambition.world.create_breakables", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = CreateBreakables;
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(value, out);
    }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        ambition_extension_sdk::structure::read(reader)
    }
}
ambition_extension_sdk::script_port!(CreateBreakablesPort);
