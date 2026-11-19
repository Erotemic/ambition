//! Consume a destructible from this invocation's native world contacts.

use ambition_extension_sdk::{Port, PortKey, PortRole, action::ReplyTo,
    wire::{WireError, WireReader}};
use crate::inventory::InventoryTerms;

/// The contact index is local to the current body_contact invocation.
/// The native owner commits consumption and the exchange together.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsumeContact {
    pub reply: ReplyTo,
    pub contact: u32,
    pub exchange: InventoryTerms,
}
ambition_extension_sdk::port_record! {
    ConsumeContact {
        reply: ReplyTo,
        /// Index in this.trigger.world. Do not retain this index for another step.
        contact: u32,
        exchange: InventoryTerms = InventoryTerms::default(),
    }
}
pub struct ConsumeContactPort;
impl Port for ConsumeContactPort {
    const KEY: PortKey = PortKey::new("ambition.world.consume_contact", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = ConsumeContact;
    fn action(value: &Self::Value) -> Option<ambition_extension_sdk::action::ActionSubmission<'_>> {
        Some(ambition_extension_sdk::action::ActionSubmission::encoded::<Self>(value, "world.consume_contact", &value.reply))
    }
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(value, out);
    }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        ambition_extension_sdk::structure::read(reader)
    }
}
ambition_extension_sdk::script_port!(ConsumeContactPort);


/// Spend this behavior's destructible and exchange with an exact local body.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsumeSelf { pub reply: ReplyTo, pub recipient: ambition_extension_sdk::ObjectRef, pub exchange: InventoryTerms }
ambition_extension_sdk::port_record! { ConsumeSelf { reply: ReplyTo, recipient: ambition_extension_sdk::ObjectRef,
    exchange: InventoryTerms = InventoryTerms::default() } }
pub struct ConsumeSelfPort;
impl Port for ConsumeSelfPort {
    const KEY: PortKey = PortKey::new("ambition.world.consume_self", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = ConsumeSelf;
    fn action(value: &Self::Value) -> Option<ambition_extension_sdk::action::ActionSubmission<'_>> {
        Some(ambition_extension_sdk::action::ActionSubmission::encoded::<Self>(value, "world.consume_self", &value.reply))
    }
    fn encode(value: &Self::Value, out: &mut Vec<u8>) { ambition_extension_sdk::structure::put(value, out); }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> { ambition_extension_sdk::structure::read(reader) }
}
ambition_extension_sdk::script_port!(ConsumeSelfPort);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DestructibleState { pub health: i32, pub maximum: i32, pub consumed: bool }
ambition_extension_sdk::port_record! { DestructibleState { health: i32, maximum: i32, consumed: bool } }
pub struct DestructibleStatePort;
impl Port for DestructibleStatePort {
    const KEY: PortKey = PortKey::new("ambition.world.destructible", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = Option<DestructibleState>;
    fn encode(value: &Self::Value, out: &mut Vec<u8>) { ambition_extension_sdk::structure::put(value, out); }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> { ambition_extension_sdk::structure::read(reader) }
}
ambition_extension_sdk::script_port!(DestructibleStatePort);
