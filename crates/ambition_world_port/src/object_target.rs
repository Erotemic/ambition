//! Typed occurrence targets and local names in a native construction batch.

use ambition_extension_sdk::{ObjectRef, script::ScriptValue,
    structure::{CaseShape, Field, Shape}, wire::{self, WireError, WireReader}};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectTarget { Live(ObjectRef), Local(String) }

impl Field<ObjectTarget> for ObjectTarget {
    fn shape() -> Shape {
        Shape::Choice(vec![
            CaseShape {name: "live", payload: Some(ObjectRef::shape()), script: "an object reference"},
            CaseShape {name: "local", payload: Some(Shape::Text), script: "#{local: \"node_key\"}"},
        ])
    }
    fn put(value: &Self, out: &mut Vec<u8>) {
        match value {
            Self::Live(reference) => { wire::put_u8(out, 0); ObjectRef::put(reference, out); }
            Self::Local(key) => { wire::put_u8(out, 1); wire::put_str(out, key); }
        }
    }
    fn read(reader: &mut WireReader<'_>) -> Result<Self, WireError> {
        match reader.u8()? {
            0 => ObjectRef::read(reader).map(Self::Live),
            1 => reader.str().map(|key| Self::Local(key.into())),
            tag => Err(WireError::BadTag(tag)),
        }
    }
    fn to_script(value: &Self) -> ScriptValue {
        match value {
            Self::Live(reference) => ObjectRef::to_script(reference),
            Self::Local(key) => ScriptValue::map([("local", ScriptValue::Text(key.clone()))]),
        }
    }
    fn from_script(value: &ScriptValue) -> Result<Self, String> {
        if let ScriptValue::Map(fields) = value {
            if let [(key, value)] = fields.as_slice() {
                if key == "local" { return value.as_text().map(|key| Self::Local(key.into())); }
            }
        }
        ObjectRef::from_script(value).map(Self::Live)
    }
}
