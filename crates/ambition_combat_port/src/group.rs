//! The ports of a coordinated group: the bodies of one room that share a tag
//! decide together, in one invocation (runtime authoring audit P4,
//! `docs/planning/engine/runtime-authoring-audit-2026-11-14/execution-plan.md`).
//!
//! A cooperative rule (two swallows lifting one coconut) needs every member
//! and every claim in one place: callbacks for each body would each assume
//! they own the same item. So the room is the invocation's scope, its members
//! are listed in simulation-identity order, and each member is named by that
//! identity, never by an entity's bits.

use ambition_extension_sdk::wire::{WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The trigger port marker for a coordinated group.
///
/// Port card:
///
/// * **Operation** — the members of one room's group decide this tick. The
///   selector is a character catalog tag; the members are the room's live
///   bodies whose character carries it. A room with no member is not invoked.
/// * **Owner** — `ambition_abilities::group`.
/// * **Time** — `actor_think`: after the brains published their control
///   frames, and before the control gate. One invocation for each room, in
///   the order of the room's identity.
/// * **Scope** — the room's root: a record attached to the scope is the
///   room's, and retires with it.
/// * **Read model** — world units, +Y down; each member's facts and each item
///   in the world as they are at the start of the phase, and each target where
///   it is now.
pub struct GroupThinkPort;

/// What a group reads: its members and the room's items.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupThink {
    pub members: Vec<GroupMember>,
    pub items: Vec<GroupItem>,
}

/// One member of a group.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupMember {
    pub id: String,
    pub form: String,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub size: [f32; 2],
    pub alive: bool,
    pub driven: bool,
    pub held: Option<String>,
    pub supporter: Option<String>,
    pub target: Option<[f32; 2]>,
    pub target_velocity: Option<[f32; 2]>,
}

/// One item lying in the room.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupItem {
    pub id: String,
    pub kind: String,
    pub position: [f32; 2],
    pub at_rest: bool,
}

impl Port for GroupThinkPort {
    const KEY: PortKey = PortKey::new("ambition.actor.group_think", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = GroupThink;

    fn encode(v: &GroupThink, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<GroupThink, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}

ambition_extension_sdk::port_record! {
    GroupThink {
        /// The room's members, in the order of their identity.
        members: Vec<GroupMember>,
        /// The items lying in the room (not held), in the order of their
        /// identity.
        items: Vec<GroupItem>,
    }
}

ambition_extension_sdk::port_record! {
    GroupMember {
        /// The body's simulation identity: what a request names it by.
        id: String,
        /// Its character (`european_swallow`).
        form: String,
        /// Its centre (world units, +Y down).
        position: [f32; 2],
        /// Its velocity (world units per second).
        velocity: [f32; 2],
        /// Its box, width and height.
        size: [f32; 2],
        /// Whether it is alive.
        alive: bool,
        /// Whether a player drives it: a driven member is listed, and a
        /// `steer` for it is refused.
        driven: bool,
        /// The kind of the item it holds, or `()`.
        held: Option<String>,
        /// The member that shares its carry, or `()`.
        supporter: Option<String>,
        /// Where its target is now, or `()` when it has none.
        target: Option<[f32; 2]>,
        /// Its target's velocity, or `()` when it has none.
        target_velocity: Option<[f32; 2]>,
    }
}

ambition_extension_sdk::port_record! {
    GroupItem {
        /// The item's simulation identity.
        id: String,
        /// What it is (`coconut`).
        kind: String,
        /// Its centre (world units, +Y down).
        position: [f32; 2],
        /// Whether it has come to rest.
        at_rest: bool,
    }
}

ambition_extension_sdk::script_port!(GroupThinkPort);

/// The request port marker for a member's steering intent.
///
/// Port card:
///
/// * **Operation** — steer a member at a velocity: a flying body's velocity
///   target, or a walking body's locomotion throttle in its own frame (the
///   velocity over its run speed), in its control frame,
///   and clear its attack and fire for the tick. The control gate, the body's
///   capabilities and its frame still apply: it is intent, not a position or
///   velocity write.
/// * **Owner** — `ambition_abilities::group`.
/// * **Time** — `actor_think`, consumed before the control gate this tick.
/// * **Refusal** — a body that is not a member of the invocation's room, or
///   that a player drives: warned and dropped.
pub struct SteerPort;

/// A member's steering intent.
#[derive(Clone, Debug, PartialEq)]
pub struct Steer {
    pub body: String,
    pub velocity: [f32; 2],
}

impl Port for SteerPort {
    const KEY: PortKey = PortKey::new("ambition.actor.steer", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = Steer;

    fn encode(v: &Steer, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Steer, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}

ambition_extension_sdk::port_record! {
    Steer {
        /// The member's identity (`GroupMember::id`).
        body: String,
        /// The velocity it aims for (world units per second, +Y down).
        velocity: [f32; 2],
    }
}

ambition_extension_sdk::script_port!(SteerPort);

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_extension_sdk::script::ScriptPort;

    #[test]
    fn a_group_and_a_steer_go_there_and_back() {
        let think = GroupThink {
            members: vec![GroupMember {
                id: "placement:a".into(),
                form: "african_swallow".into(),
                position: [1.0, 2.0],
                velocity: [0.0, -3.0],
                size: [24.0, 20.0],
                alive: true,
                driven: false,
                held: Some("coconut".into()),
                supporter: None,
                target: Some([5.0, 6.0]),
                target_velocity: None,
            }],
            items: vec![GroupItem { id: "placement:nut".into(), kind: "coconut".into(), position: [9.0, 9.0], at_rest: true }],
        };
        let mut bytes = Vec::new();
        GroupThinkPort::encode(&think, &mut bytes);
        assert_eq!(GroupThinkPort::decode(&mut WireReader::new(&bytes)).unwrap(), think);
        assert_eq!(GroupThinkPort::from_script(&GroupThinkPort::to_script(&think)).unwrap(), think);
        let steer = Steer { body: "placement:a".into(), velocity: [3.0, -4.0] };
        let mut bytes = Vec::new();
        SteerPort::encode(&steer, &mut bytes);
        assert_eq!(SteerPort::decode(&mut WireReader::new(&bytes)).unwrap(), steer);
    }
}
