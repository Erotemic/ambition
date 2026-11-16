//! The custody requests: a body takes an item lying in its room, or lets go
//! of the one it holds (runtime authoring audit P4: the coconut swallows).
//!
//! The item owner (`ambition_held_items`) does the work, as it does for a
//! player's pickup and throw: the same take, the same release, the same
//! refusals. A script sees the result in the next tick's observation (the
//! member's `held` and `supporter`), never on submission.

use ambition_extension_sdk::wire::{WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

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
/// * **Refusal** (warned and dropped) — the body or the supporter is not an
///   undriven body of the invocation's room, or already holds something; the
///   item is not lying in that room; the item's box is farther than
///   `TAKE_REACH` from the body's; or another take has the item first this
///   tick (requests apply in the order they were submitted).
pub struct TakeItemPort;

/// A take.
#[derive(Clone, Debug, PartialEq)]
pub struct TakeItem {
    pub body: String,
    pub item: String,
    pub supporter: Option<String>,
}

impl Port for TakeItemPort {
    const KEY: PortKey = PortKey::new("ambition.item.take", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = TakeItem;

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
/// * **Refusal** (warned and dropped) — the body is not an undriven body of
///   the invocation's room, or the release has no room to put the item
///   (the item owner's refusal). A body that holds nothing has its hand
///   emptied, as the item owner does.
pub struct ReleaseItemPort;

/// A release.
#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseItem {
    pub body: String,
    pub thrown: bool,
    pub velocity: [f32; 2],
}

impl Port for ReleaseItemPort {
    const KEY: PortKey = PortKey::new("ambition.item.release", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = ReleaseItem;

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
    }
}

ambition_extension_sdk::script_port!(ReleaseItemPort);

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_extension_sdk::script::{ScriptPort, ScriptValue};

    #[test]
    fn a_take_and_a_release_go_there_and_back() {
        let take = TakeItem { body: "placement:a".into(), item: "placement:nut".into(), supporter: Some("placement:b".into()) };
        let mut bytes = Vec::new();
        TakeItemPort::encode(&take, &mut bytes);
        assert_eq!(TakeItemPort::decode(&mut WireReader::new(&bytes)).unwrap(), take);
        let release = ReleaseItem { body: "placement:a".into(), thrown: true, velocity: [10.0, 0.0] };
        assert_eq!(ReleaseItemPort::from_script(&ReleaseItemPort::to_script(&release)).unwrap(), release);
        // A take alone leaves the supporter out.
        let alone = ScriptValue::Map(vec![
            ("body".into(), ScriptValue::Text("placement:a".into())),
            ("item".into(), ScriptValue::Text("placement:nut".into())),
        ]);
        assert_eq!(TakeItemPort::from_script(&alone).unwrap().supporter, None);
    }
}
