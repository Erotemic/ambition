//! Reusable object behavior bindings and directed method calls.

use ambition_extension_sdk::{ObjectRef, Port, PortKey, PortRole, action::ReplyTo,
    script::ScriptValue, wire::{WireError, WireReader}};

#[derive(Clone, Debug, PartialEq)]
pub struct StateSeed { pub schema: String, pub value: ScriptValue }
ambition_extension_sdk::port_record! { StateSeed { schema: String, value: ScriptValue } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BehaviorMethod { pub name: String, pub entry: String }
ambition_extension_sdk::port_record! { BehaviorMethod { name: String, entry: String } }

#[derive(Clone, Debug, PartialEq)]
pub struct BehaviorSubscription { pub key: String, pub source: String, pub method: String }
ambition_extension_sdk::port_record! { BehaviorSubscription { key: String, source: String, method: String } }

#[derive(Clone, Debug, PartialEq)]
pub struct BehaviorBinding {
    pub key: String,
    pub methods: Vec<BehaviorMethod>,
    pub subscriptions: Vec<BehaviorSubscription>,
    pub references: Vec<(String, crate::object_target::ObjectTarget)>,
    pub params: ScriptValue,
    pub state: Vec<StateSeed>,
}
ambition_extension_sdk::port_record! {
    BehaviorBinding { key: String, methods: Vec<BehaviorMethod>,
        subscriptions: Vec<BehaviorSubscription> = Vec::new(),
        references: Vec<(String, crate::object_target::ObjectTarget)> as ambition_extension_sdk::structure::TextMap<crate::object_target::ObjectTarget> = Vec::new(),
        params: ScriptValue = ScriptValue::Unit, state: Vec<StateSeed> = Vec::new() }
}
#[derive(Clone, Debug, PartialEq)]
pub struct AttachBehaviors { pub reply: ReplyTo, pub target: ObjectRef, pub bindings: Vec<BehaviorBinding> }
ambition_extension_sdk::port_record! {
    AttachBehaviors { reply: ReplyTo, target: ObjectRef, bindings: Vec<BehaviorBinding> }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CallObject {
    pub reply: ReplyTo,
    pub target: ObjectRef,
    pub binding: String,
    pub method: String,
    pub data: ScriptValue,
}
ambition_extension_sdk::port_record! {
    CallObject { reply: ReplyTo, target: ObjectRef, binding: String, method: String,
        data: ScriptValue = ScriptValue::Unit }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectCall { pub sequence: u64, pub caller: ObjectRef, pub method: String, pub data: ScriptValue }
ambition_extension_sdk::port_record! {
    ObjectCall { sequence: u64, caller: ObjectRef, method: String, data: ScriptValue }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CompleteObjectCall { pub reply: ReplyTo, pub sequence: u64, pub accepted: bool, pub code: String, pub data: ScriptValue }
ambition_extension_sdk::port_record! {
    CompleteObjectCall { reply: ReplyTo, sequence: u64, accepted: bool = true, code: String, data: ScriptValue = ScriptValue::Unit }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectCallResult { pub ticket: ambition_extension_sdk::action::ActionTicket, pub data: ScriptValue }
ambition_extension_sdk::port_record! {
    ObjectCallResult { ticket: ambition_extension_sdk::action::ActionTicket, data: ScriptValue }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BehaviorContext { pub object: ObjectRef, pub binding: String, pub params: ScriptValue,
    pub references: Vec<(String, ObjectRef)> }
ambition_extension_sdk::port_record! {
    BehaviorContext { object: ObjectRef, binding: String, params: ScriptValue,
        references: Vec<(String, ObjectRef)> as ambition_extension_sdk::structure::TextMap<ObjectRef> }
}

macro_rules! port {
    ($name:ident, $key:literal, $role:ident, $value:ty) => {
        port!($name, $key, 1, $role, $value);
    };
    ($name:ident, $key:literal, $version:literal, $role:ident, $value:ty) => {
        pub struct $name;
        impl Port for $name {
            const KEY: PortKey = PortKey::new($key, $version);
            const ROLE: PortRole = PortRole::$role;
            type Value = $value;
            fn encode(value: &Self::Value, out: &mut Vec<u8>) { ambition_extension_sdk::structure::put(value, out); }
            fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> { ambition_extension_sdk::structure::read(reader) }
        }
        ambition_extension_sdk::script_port!($name);
    };
}
macro_rules! action_port {
    ($name:ident, $key:literal, $version:literal, $value:ty, $domain:literal) => {
        pub struct $name;
        impl Port for $name {
            const KEY: PortKey = PortKey::new($key, $version);
            const ROLE: PortRole = PortRole::Request;
            type Value = $value;
            fn action(value: &Self::Value) -> Option<ambition_extension_sdk::action::ActionSubmission<'_>> {
                Some(ambition_extension_sdk::action::ActionSubmission::encoded::<Self>(value, $domain, &value.reply))
            }
            fn encode(value: &Self::Value, out: &mut Vec<u8>) { ambition_extension_sdk::structure::put(value, out); }
            fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> { ambition_extension_sdk::structure::read(reader) }
        }
        ambition_extension_sdk::script_port!($name);
    };
}
action_port!(AttachBehaviorsPort, "ambition.objects.attach_behaviors", 3, AttachBehaviors, "objects.attach_behaviors");
action_port!(CallObjectPort, "ambition.objects.call", 1, CallObject, "objects.call");
port!(ObjectCallPort, "ambition.objects.method", Trigger, ObjectCall);
action_port!(CompleteObjectCallPort, "ambition.objects.complete_call", 2, CompleteObjectCall, "objects.complete_call");
port!(BehaviorContextPort, "ambition.objects.behavior", 2, Observation, Option<BehaviorContext>);

port!(ObjectCallResultPort, "ambition.objects.call_result", Observation, Option<ObjectCallResult>);

/// A completed native birth. All declared native and private state is present.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectBorn { pub object: ObjectRef }
ambition_extension_sdk::port_record! { ObjectBorn { object: ObjectRef } }
port!(ObjectBornPort, "ambition.objects.born", Trigger, ObjectBorn);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BehaviorEventDelivery {
    pub sequence: u64, pub subscription: String, pub source: ObjectRef,
    pub event: String, pub tick: u64, pub progress: ambition_extension_sdk::action::DeliveryProgress,
}
ambition_extension_sdk::port_record! {
    BehaviorEventDelivery { sequence: u64, subscription: String, source: ObjectRef,
        event: String, tick: u64, progress: ambition_extension_sdk::action::DeliveryProgress }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BehaviorEventHistory { pub deliveries: Vec<BehaviorEventDelivery>, pub dropped: u64 }
ambition_extension_sdk::port_record! { BehaviorEventHistory { deliveries: Vec<BehaviorEventDelivery>, dropped: u64 } }
port!(BehaviorEventsPort, "ambition.objects.events", Observation, BehaviorEventHistory);

/// Resolve authored placements in the caller's current live room instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolveAuthoredObjects { pub reply: ReplyTo, pub placements: Vec<String> }
ambition_extension_sdk::port_record! { ResolveAuthoredObjects { reply: ReplyTo, placements: Vec<String> } }
action_port!(ResolveAuthoredObjectsPort, "ambition.objects.resolve_authored", 1, ResolveAuthoredObjects, "objects.resolve_authored");


/// Native activation evidence. Revision and latest actor remain inspectable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectActivated { pub object: ObjectRef, pub actor: Option<ObjectRef>, pub revision: u64 }
ambition_extension_sdk::port_record! { ObjectActivated { object: ObjectRef, actor: Option<ObjectRef>, revision: u64 } }
port!(ObjectActivatedPort, "ambition.objects.activated", Trigger, ObjectActivated);
port!(ObjectActivationPort, "ambition.objects.activation", Observation, Option<ObjectActivated>);

/// A native defeat: the damage owner decided the hit killed `object`. `by` is
/// the attacker when the hit had one that is an addressed object. The fact is
/// committed; whatever owner takes the death afterwards (loot, a match, a fall)
/// cannot undo it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectDefeated { pub object: ObjectRef, pub by: Option<ObjectRef> }
ambition_extension_sdk::port_record! { ObjectDefeated { object: ObjectRef, by: Option<ObjectRef> } }
port!(ObjectDefeatedPort, "ambition.objects.defeated", Trigger, ObjectDefeated);

/// A breakable object was spent: struck to nothing or consumed by its own
/// exchange. Its owner commits it on the tick it breaks, whatever broke it;
/// a breakable that returns and breaks again is a second fact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectBroken { pub object: ObjectRef }
ambition_extension_sdk::port_record! { ObjectBroken { object: ObjectRef } }
port!(ObjectBrokenPort, "ambition.objects.broken", Trigger, ObjectBroken);

/// A pickup was taken: `by` is the body that collected it, when it has an
/// address. The pickup owner commits it on the tick of the take; a pickup
/// that regrows and is taken again is a second fact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectCollected { pub object: ObjectRef, pub by: Option<ObjectRef> }
ambition_extension_sdk::port_record! { ObjectCollected { object: ObjectRef, by: Option<ObjectRef> } }
port!(ObjectCollectedPort, "ambition.objects.collected", Trigger, ObjectCollected);

/// A body crossed into another room: the room it left and the room it
/// arrived in, by authored room id. The room owner commits it once the
/// destination published, so the body already stands in `to`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectCrossed { pub object: ObjectRef, pub from: String, pub to: String }
ambition_extension_sdk::port_record! { ObjectCrossed { object: ObjectRef, from: String, to: String } }
port!(ObjectCrossedPort, "ambition.objects.crossed", Trigger, ObjectCrossed);

/// A crossing the room owner refused after it began: the body is still in the
/// room it tried to leave, which is unchanged. `code` says why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectCrossingRefused { pub object: ObjectRef, pub to: String, pub code: String }
ambition_extension_sdk::port_record! { ObjectCrossingRefused { object: ObjectRef, to: String, code: String } }
port!(ObjectCrossingRefusedPort, "ambition.objects.crossing_refused", Trigger, ObjectCrossingRefused);

/// A script's named message from the object it runs for, to each method
/// subscribed to that object whose entry's selector is `topic`. The runtime
/// admits it to every such method or to none, and answers with the recipients.
#[derive(Clone, Debug, PartialEq)]
pub struct EmitSignal { pub reply: ReplyTo, pub topic: String, pub data: ScriptValue }
ambition_extension_sdk::port_record! {
    EmitSignal { reply: ReplyTo, topic: String, data: ScriptValue = ScriptValue::Unit }
}
action_port!(EmitSignalPort, "ambition.objects.emit_signal", 1, EmitSignal, "objects.emit_signal");
/// An admitted signal as its subscriber hears it. `from` is the emitting
/// object, named by the runtime, never by the script that emitted it.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectSignal { pub from: ObjectRef, pub topic: String, pub data: ScriptValue }
ambition_extension_sdk::port_record! { ObjectSignal { from: ObjectRef, topic: String, data: ScriptValue } }
port!(ObjectSignalPort, "ambition.objects.signal", Trigger, ObjectSignal);
/// Post a signal from the object this script runs for to its subscribers in
/// any room. Unlike [`EmitSignal`], a subscriber in another rollback island
/// hears it too: the world domain holds the post until the sending tick can
/// never be simulated again, then delivers it to every subscriber where each
/// is then, once, and again on that subscriber's room replay of the same tick.
/// The applied result says the post was held, not who will hear it.
#[derive(Clone, Debug, PartialEq)]
pub struct PostWorldSignal { pub reply: ReplyTo, pub topic: String, pub data: ScriptValue }
ambition_extension_sdk::port_record! {
    PostWorldSignal { reply: ReplyTo, topic: String, data: ScriptValue = ScriptValue::Unit }
}
action_port!(PostWorldSignalPort, "ambition.world.post_signal", 1, PostWorldSignal, "world.post_signal");
/// A signal topic: 1 to 48 bytes of `a-z`, `0-9`, `_`, `.` and `-`. The
/// subscriber's entry selector names it.
pub fn valid_signal_topic(topic: &str) -> bool {
    (1..=48).contains(&topic.len())
        && topic.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-'))
}

/// Write the world value `key`: a value the save keeps and every room reads.
/// The world domain applies it on a tick no room can simulate again, so a
/// write from a room that rewinds alone waits for its tick to be confirmed.
/// With `expect`, the write applies only if the value's revision is still
/// that one (`0`: no value yet); otherwise it is refused `revision_changed`.
#[derive(Clone, Debug, PartialEq)]
pub struct SetWorldValue { pub reply: ReplyTo, pub key: String, pub value: ScriptValue, pub expect: Option<u32> }
ambition_extension_sdk::port_record! {
    SetWorldValue { reply: ReplyTo, key: String, value: ScriptValue, expect: Option<u32> = None }
}
action_port!(SetWorldValuePort, "ambition.world.set_value", 1, SetWorldValue, "world.set_value");

/// Add `delta` to the world count `key` (an integer world value; absent is 0).
/// Applied as [`SetWorldValue`] is; two rooms that each add one make two.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddWorldCount { pub reply: ReplyTo, pub key: String, pub delta: i32 }
ambition_extension_sdk::port_record! { AddWorldCount { reply: ReplyTo, key: String, delta: i32 } }
action_port!(AddWorldCountPort, "ambition.world.add_count", 1, AddWorldCount, "world.add_count");

/// One world value and the revision of its last write (from 1).
#[derive(Clone, Debug, PartialEq)]
pub struct WorldValue { pub key: String, pub value: ScriptValue, pub revision: u32 }
ambition_extension_sdk::port_record! { WorldValue { key: String, value: ScriptValue, revision: u32 } }

/// Every world value, in key order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldValues { pub values: Vec<WorldValue> }
ambition_extension_sdk::port_record! { WorldValues { values: Vec<WorldValue> } }
port!(WorldValuesPort, "ambition.world.values", Observation, WorldValues);

// A world value changed. Entries whose selector is the key hear it, once per
// write, at the session, on the tick after the world domain applied it.
port!(WorldValueChangedPort, "ambition.world.value_changed", Trigger, WorldValue);

/// One authored gated wall of the invocation's room: its id, the condition
/// line it waits on (`gated_by`), and whether it stands now. A wall stands
/// while its condition is not satisfied (or cannot be answered).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockWallState { pub id: String, pub gated_by: String, pub standing: bool }
ambition_extension_sdk::port_record! { LockWallState { id: String, gated_by: String, standing: bool } }

/// The gated walls of the invocation's live room, in authored order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LockWalls { pub walls: Vec<LockWallState> }
ambition_extension_sdk::port_record! { LockWalls { walls: Vec<LockWallState> } }
port!(LockWallsPort, "ambition.world.lock_walls", Observation, LockWalls);
