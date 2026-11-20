//! The custody requests: a body takes an item lying in its room, or lets go
//! of the one it holds (runtime authoring audit P4: the coconut swallows).
//!
//! The item owner (`ambition_held_items`) does the work, as it does for a
//! player's pickup and throw: the same take, the same release, the same
//! refusals. A request with a `reply` gets an action result: `applied` once
//! the item owner has done it, or `rejected` with the refusal's code. Without
//! one, a refusal is only logged; the next tick's observation (the member's
//! `held` and `supporter`) still shows what happened, never the submission.

use ambition_extension_sdk::action::{ActionSubmission, ReplyTo};
use ambition_extension_sdk::wire::{WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The action domain of a take.
pub const TAKE_DOMAIN: &str = "item.take";
/// The action domain of a release.
pub const RELEASE_DOMAIN: &str = "item.release";

/// The request port marker for taking an item.
///
/// Port card:
///
/// * **Operation** — `body` takes `item` into its hand. With a `supporter`,
///   the two carry it together: the supporter holds the other end of the
///   sling (`SupportedCarry`), and its hand stays empty.
/// * **Owner** — `ambition_abilities::group`, through
///   `ambition_held_items::take_ground_item`.
/// * **Time** — `actor_think`, applied before the control gate this tick.
/// * **Result** — with a `reply`: `applied` (`item_taken`, naming the body
///   and the item) or `rejected` with the refusal's code. Without one, a
///   refusal is warned and dropped.
/// * **Refusal** — `no_such_body`: the body is not an undriven body of the
///   invocation's room; `no_such_supporter`: the supporter is not another
///   one; `hand_not_free`: either already holds or supports something;
///   `no_such_item`: no item of that identity lies in that room, which is
///   also the answer when another take has it first this tick (requests apply
///   in the order they were submitted); `out_of_reach`: the item's box is
///   farther than `TAKE_REACH` from the body's.
pub struct TakeItemPort;

/// A take.
#[derive(Clone, Debug, PartialEq)]
pub struct TakeItem {
    pub body: String,
    pub item: String,
    pub supporter: Option<String>,
    pub reply: Option<ReplyTo>,
}

impl Port for TakeItemPort {
    const KEY: PortKey = PortKey::new("ambition.item.take", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = TakeItem;

    fn action(value: &TakeItem) -> Option<ActionSubmission<'_>> {
        Some(ActionSubmission::encoded::<Self>(value, TAKE_DOMAIN, value.reply.as_ref()?))
    }

    fn encode(v: &TakeItem, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<TakeItem, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}

ambition_extension_sdk::port_record! {
    TakeItem {
        /// The taker's identity (`GroupMember::id`).
        body: String,
        /// The item's identity (`GroupItem::id`).
        item: String,
        /// The member that carries it with the taker, or `()` to carry it
        /// alone.
        supporter: Option<String> = None,
        /// Where the action result goes, or `()` for none.
        reply: Option<ReplyTo> = None,
    }
}

ambition_extension_sdk::script_port!(TakeItemPort);

/// How far past its own box a body reaches for an item it takes on request
/// (world units): a flyer over a coconut reaches down to it. A player's
/// pickup asks for the boxes to overlap.
pub const TAKE_REACH: f32 = 32.0;

/// The request port marker for letting go of the held item.
///
/// Port card:
///
/// * **Operation** — `body` lets go of what it holds: thrown, or dropped,
///   with `velocity`. It leaves from under the body, or from under the
///   middle of a shared carry, and the sling ends.
/// * **Owner** — `ambition_abilities::group`, through
///   `ambition_held_items::release_carried_item`.
/// * **Time** — `actor_think`, applied before the control gate this tick.
/// * **Result** — with a `reply`: `applied` (`item_released`, naming the
///   body) or `rejected` with the refusal's code. Without one, a refusal is
///   warned and dropped.
/// * **Refusal** — `no_such_body`: the body is not an undriven body of the
///   invocation's room; `no_room_to_release`: the item owner has no room to
///   put the item; `nothing_held`: the body holds nothing (its hand is
///   emptied, as the item owner does).
pub struct ReleaseItemPort;

/// A release.
#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseItem {
    pub body: String,
    pub thrown: bool,
    pub velocity: [f32; 2],
    pub reply: Option<ReplyTo>,
}

impl Port for ReleaseItemPort {
    const KEY: PortKey = PortKey::new("ambition.item.release", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = ReleaseItem;

    fn action(value: &ReleaseItem) -> Option<ActionSubmission<'_>> {
        Some(ActionSubmission::encoded::<Self>(value, RELEASE_DOMAIN, value.reply.as_ref()?))
    }

    fn encode(v: &ReleaseItem, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<ReleaseItem, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}

ambition_extension_sdk::port_record! {
    ReleaseItem {
        /// The holder's identity (`GroupMember::id`).
        body: String,
        /// `true` to throw it (an attack: what it hits, it hurts), `false`
        /// to drop it. (`throw` is a word Rhai reserves.)
        thrown: bool,
        /// The item's velocity as it leaves (world units per second, +Y
        /// down).
        velocity: [f32; 2] = [0.0, 0.0],
        /// Where the action result goes, or `()` for none.
        reply: Option<ReplyTo> = None,
    }
}

ambition_extension_sdk::script_port!(ReleaseItemPort);

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_extension_sdk::script::{ScriptPort, ScriptValue};

    #[test]
    fn a_take_and_a_release_go_there_and_back() {
        let take = TakeItem { body: "placement:a".into(), item: "placement:nut".into(), supporter: Some("placement:b".into()),
            reply: Some(ReplyTo { request: 7, selector: "probe::m/taken".into() }) };
        let mut bytes = Vec::new();
        TakeItemPort::encode(&take, &mut bytes);
        assert_eq!(TakeItemPort::decode(&mut WireReader::new(&bytes)).unwrap(), take);
        let release = ReleaseItem { body: "placement:a".into(), thrown: true, velocity: [10.0, 0.0], reply: None };
        assert_eq!(ReleaseItemPort::from_script(&ReleaseItemPort::to_script(&release)).unwrap(), release);
        // A take alone leaves the supporter out.
        let alone = ScriptValue::Map(vec![
            ("body".into(), ScriptValue::Text("placement:a".into())),
            ("item".into(), ScriptValue::Text("placement:nut".into())),
        ]);
        assert_eq!(TakeItemPort::from_script(&alone).unwrap().supporter, None);
    }
}
