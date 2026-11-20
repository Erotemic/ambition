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
    fn action(value: &Self::Value) -> Option<ambition_extension_sdk::action::ActionSubmission<'_>> {
        Some(ambition_extension_sdk::action::ActionSubmission::encoded::<Self>(value, "inventory.exchange", &value.reply))
    }
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

/// Equip an owned item on a body a seat drives: a held item (a weapon, the
/// portal gun) into its hand, putting down what the hand carried; an ability
/// into its worn set, where an exclusive slot replaces the variant that held
/// it. The bag keeps the item: equipping spends nothing.
///
/// * **Owner** — the hand (`ambition_held_items::change_hand`) or the worn
///   set (`ambition_platformer2d_items::narrative::wear_item`), the same
///   applier as a menu's equip. `actor_think`.
/// * **Result** — terminal `item_equipped` (into the hand) or `item_worn`
///   (into the worn set), naming the body. Refusals: `no_such_body` (no body
///   of that identity in the invocation's room), `not_a_party_body` (no seat
///   drives it), `unknown_item`, `not_owned`, `not_equippable` (neither held
///   nor worn), `no_hand`, `no_room_to_release` (what the hand carries cannot
///   be put down).
#[derive(Clone, Debug, PartialEq)]
pub struct EquipItem {
    pub reply: ReplyTo,
    pub body: String,
    pub item: String,
}
ambition_extension_sdk::port_record! {
    EquipItem {
        /// Where the action result goes.
        reply: ReplyTo,
        /// The body's identity (its `SimId`).
        body: String,
        /// The catalog item's id.
        item: String,
    }
}
pub struct EquipItemPort;
impl Port for EquipItemPort {
    const KEY: PortKey = PortKey::new("ambition.world.equip", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = EquipItem;
    fn action(value: &Self::Value) -> Option<ambition_extension_sdk::action::ActionSubmission<'_>> {
        Some(ambition_extension_sdk::action::ActionSubmission::encoded::<Self>(value, "inventory.equip", &value.reply))
    }
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(value, out);
    }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        ambition_extension_sdk::structure::read(reader)
    }
}
ambition_extension_sdk::script_port!(EquipItemPort);

/// Empty the hand of a body a seat drives, putting down what it carries.
///
/// * **Owner** — the hand (`ambition_held_items::change_hand`). `actor_think`.
/// * **Result** — terminal `hand_stowed`, naming the body. Refusals:
///   `no_such_body`, `not_a_party_body`, `no_hand`, `no_room_to_release`, and
///   `nothing_held` for a hand already empty.
#[derive(Clone, Debug, PartialEq)]
pub struct StowHand {
    pub reply: ReplyTo,
    pub body: String,
}
ambition_extension_sdk::port_record! {
    StowHand {
        /// Where the action result goes.
        reply: ReplyTo,
        /// The body's identity (its `SimId`).
        body: String,
    }
}
pub struct StowHandPort;
impl Port for StowHandPort {
    const KEY: PortKey = PortKey::new("ambition.world.stow", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = StowHand;
    fn action(value: &Self::Value) -> Option<ambition_extension_sdk::action::ActionSubmission<'_>> {
        Some(ambition_extension_sdk::action::ActionSubmission::encoded::<Self>(value, "inventory.stow", &value.reply))
    }
    fn encode(value: &Self::Value, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(value, out);
    }
    fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
        ambition_extension_sdk::structure::read(reader)
    }
}
ambition_extension_sdk::script_port!(StowHandPort);
