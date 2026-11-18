//! Exact inventory exchanges and observations over native item authority.

use ambition_extension_sdk::{
    Port, PortKey, PortRole,
    action::ReplyTo,
    wire::{WireError, WireReader},
};

#[derive(Clone, Debug, PartialEq)]
pub struct ItemAmount {
    pub id: String,
    pub count: u32,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InventoryTerms {
    pub costs: Vec<ItemAmount>,
    pub grants: Vec<ItemAmount>,
    pub pay_coins: u32,
    pub receive_coins: u32,
}
ambition_extension_sdk::port_record! { ItemAmount { id: String, count: u32, } }
ambition_extension_sdk::port_record! {
    InventoryTerms {
        costs: Vec<ItemAmount> = Vec::new(),
        grants: Vec<ItemAmount> = Vec::new(),
        pay_coins: u32 = 0,
        receive_coins: u32 = 0,
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryExchange {
    pub reply: ReplyTo,
    pub exchange: InventoryTerms,
}
ambition_extension_sdk::port_record! {
    InventoryExchange { reply: ReplyTo, exchange: InventoryTerms }
}
pub struct InventoryExchangePort;
impl Port for InventoryExchangePort {
    const KEY: PortKey = PortKey::new("ambition.world.inventory_exchange", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = InventoryExchange;
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(value, out);
    }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        ambition_extension_sdk::structure::read(reader)
    }
}
ambition_extension_sdk::script_port!(InventoryExchangePort);

#[derive(Clone, Debug, PartialEq)]
pub struct InventoryItem {
    pub id: String,
    pub count: u32,
    pub capacity: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryView {
    pub items: Vec<InventoryItem>,
    pub wallet: Option<i32>,
}
ambition_extension_sdk::port_record! { InventoryItem { id: String, count: u32, capacity: u32, } }
ambition_extension_sdk::port_record! { InventoryView { items: Vec<InventoryItem>, wallet: Option<i32>, } }
pub struct InventoryViewPort;
impl Port for InventoryViewPort {
    const KEY: PortKey = PortKey::new("ambition.world.inventory", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = InventoryView;
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(value, out);
    }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        ambition_extension_sdk::structure::read(reader)
    }
}
ambition_extension_sdk::script_port!(InventoryViewPort);
