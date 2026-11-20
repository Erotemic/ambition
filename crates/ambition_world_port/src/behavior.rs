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
/// A signal topic: 1 to 48 bytes of `a-z`, `0-9`, `_`, `.` and `-`. The
/// subscriber's entry selector names it.
pub fn valid_signal_topic(topic: &str) -> bool {
    (1..=48).contains(&topic.len())
        && topic.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-'))
}
