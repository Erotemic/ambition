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
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(value, out);
    }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        ambition_extension_sdk::structure::read(reader)
    }
}
ambition_extension_sdk::script_port!(ConsumeContactPort);
