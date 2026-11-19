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
pub struct BehaviorBinding {
    pub key: String,
    pub methods: Vec<BehaviorMethod>,
    pub params: ScriptValue,
    pub state: Vec<StateSeed>,
}
ambition_extension_sdk::port_record! {
    BehaviorBinding { key: String, methods: Vec<BehaviorMethod>,
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
pub struct BehaviorContext { pub object: ObjectRef, pub binding: String, pub params: ScriptValue }
ambition_extension_sdk::port_record! {
    BehaviorContext { object: ObjectRef, binding: String, params: ScriptValue }
}

macro_rules! port {
    ($name:ident, $key:literal, $role:ident, $value:ty) => {
        pub struct $name;
        impl Port for $name {
            const KEY: PortKey = PortKey::new($key, 1);
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
action_port!(AttachBehaviorsPort, "ambition.objects.attach_behaviors", 1, AttachBehaviors, "objects.attach_behaviors");
action_port!(CallObjectPort, "ambition.objects.call", 1, CallObject, "objects.call");
port!(ObjectCallPort, "ambition.objects.method", Trigger, ObjectCall);
action_port!(CompleteObjectCallPort, "ambition.objects.complete_call", 2, CompleteObjectCall, "objects.complete_call");
port!(BehaviorContextPort, "ambition.objects.behavior", Observation, Option<BehaviorContext>);

port!(ObjectCallResultPort, "ambition.objects.call_result", Observation, Option<ObjectCallResult>);
