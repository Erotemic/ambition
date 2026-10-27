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
