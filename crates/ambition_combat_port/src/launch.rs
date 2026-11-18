//! The axes of an authored hit's launch direction.

use ambition_extension_sdk::wire::{self, WireError, WireReader};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LaunchDirection {
    /// A world direction, with positive Y down.
    World([f32; 2]),
    /// X points away from the source; positive Y points down.
    AwayFromSource([f32; 2]),
}

impl LaunchDirection {
    pub(crate) fn put(self, out: &mut Vec<u8>) {
        let (tag, direction) = match self {
            Self::World(direction) => (0, direction),
            Self::AwayFromSource(direction) => (1, direction),
        };
        wire::put_u8(out, tag);
        wire::put_vec2(out, direction);
    }

    pub(crate) fn read(r: &mut WireReader<'_>) -> Result<Self, WireError> {
        match r.u8()? {
            0 => Ok(Self::World(r.vec2()?)),
            1 => Ok(Self::AwayFromSource(r.vec2()?)),
            tag => Err(WireError::BadTag(tag)),
        }
    }
}

impl ambition_extension_sdk::structure::Field<LaunchDirection> for LaunchDirection {
    fn shape() -> ambition_extension_sdk::structure::Shape {
        use ambition_extension_sdk::structure::{CaseShape, Shape};
        Shape::Choice(vec![
            CaseShape { name: "world", payload: Some(Shape::Vec2), script: "#{ world: [x, y] }" },
            CaseShape { name: "away", payload: Some(Shape::Vec2), script: "#{ away: [x, y] }: x points away from the source" },
        ])
    }
    fn put(v: &LaunchDirection, out: &mut Vec<u8>) { (*v).put(out); }
    fn read(r: &mut WireReader<'_>) -> Result<LaunchDirection, WireError> { LaunchDirection::read(r) }
    fn to_script(v: &LaunchDirection) -> ambition_extension_sdk::script::ScriptValue {
        use ambition_extension_sdk::script::ScriptValue;
        match v {
            LaunchDirection::World(at) => ScriptValue::map([("world", ScriptValue::vec2(*at))]),
            LaunchDirection::AwayFromSource(at) => ScriptValue::map([("away", ScriptValue::vec2(*at))]),
        }
    }
    fn from_script(v: &ambition_extension_sdk::script::ScriptValue) -> Result<LaunchDirection, String> {
        use ambition_extension_sdk::structure::record_fields;
        let fields = record_fields(v, &["world", "away"])?;
        let [(kind, value)] = fields else { return Err("a launch needs exactly one of world or away".into()); };
        let direction = <[f32; 2]>::from_script(value)?;
        Ok(match kind.as_str() { "world" => Self::World(direction), "away" => Self::AwayFromSource(direction), _ => unreachable!() })
    }
}
