//! Physical contact facts and responses for a body's bound rules.

use ambition_extension_sdk::wire::{WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// Contact rules bind a held status or a custom brain key. Every controller
/// uses the same path. Each active recipient reads one sample in body_contact,
/// after movement and before shared contact damage, including an empty sample.
pub struct BodyContactPort;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ContactBody {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub velocity: [f32; 2],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ContactSurface {
    pub point: [f32; 2],
    pub normal: [f32; 2],
    pub distance: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ContactParticipant {
    pub id: String,
    pub body: ContactBody,
    pub driven: bool,
    pub relation: Option<crate::targets::TargetRelation>,
    pub surfaces: Vec<ContactSurface>,
    pub surfaces_truncated: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactRefusal {
    pub id: String,
    pub reason: String,
}
ambition_extension_sdk::port_record! { ContactRefusal { id: String, reason: String } }

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BodyContact {
    pub observed_at: u64,
    pub room: Option<u32>,
    pub body_id: String,
    pub driven: bool,
    pub body: ContactBody,
    pub down: Option<[f32; 2]>,
    pub touchers: Vec<ContactParticipant>,
    pub truncated: bool,
    pub incomplete: bool,
    pub refusals: Vec<ContactRefusal>,
    pub refusals_truncated: bool,
}

ambition_extension_sdk::port_record! {
    ContactBody {
        /// The physical body's centre in its root room chart, with +Y down.
        position: [f32; 2],
        /// The full physical envelope under the body's current material axes.
        size: [f32; 2],
        velocity: [f32; 2],
    }
}
ambition_extension_sdk::port_record! {
    ContactSurface {
        /// A point on the recipient in the recipient's root room chart.
        point: [f32; 2],
        /// The recipient's outward unit normal in that same chart.
        normal: [f32; 2],
        /// Signed surface separation in the recipient's chart. Penetration is negative.
        distance: f32,
    }
}
ambition_extension_sdk::port_record! {
    ContactParticipant {
        id: String,
        body: ContactBody,
        driven: bool,
        relation: Option<crate::targets::TargetRelation>,
        /// Native physical contacts, including contacts through room apertures.
        surfaces: Vec<ContactSurface>,
        surfaces_truncated: bool,
    }
}
ambition_extension_sdk::port_record! {
    BodyContact {
        observed_at: u64,
        room: Option<u32>,
        body_id: String,
        driven: bool,
        body: ContactBody,
        down: Option<[f32; 2]>,
        /// Same-session and same-room touchers in simulation identity order.
        touchers: Vec<ContactParticipant>,
        /// More touching bodies existed than the game's contact budget.
        truncated: bool,
        /// At least one same-scope body could not supply accepted geometry.
        incomplete: bool,
        refusals: Vec<ContactRefusal>,
        refusals_truncated: bool,
    }
}

/// Bounce a toucher from this invocation, against that toucher's own gravity.
/// The index refers to the current sample, not a retained identity.
pub struct ContactBouncePort;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactBounce {
    pub toucher: u32,
    pub speed: f32,
    pub pogo_cue: bool,
}
ambition_extension_sdk::port_record! {
    ContactBounce { toucher: u32, speed: f32, pogo_cue: bool = false }
}

/// Defeat this invocation's recipient before shared contact damage.
/// This immediate retirement has no drop, score, or death sequence.
pub struct BodyDefeatPort;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyDefeat;
ambition_extension_sdk::port_record! { BodyDefeat {} }

macro_rules! port {
    ($port:ty, $key:literal, $version:literal, $role:ident, $value:ty) => {
        impl Port for $port {
            const KEY: PortKey = PortKey::new($key, $version);
            const ROLE: PortRole = PortRole::$role;
            type Value = $value;
            fn encode(value: &Self::Value, out: &mut Vec<u8>) {
                ambition_extension_sdk::structure::put(value, out);
            }
            fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
                ambition_extension_sdk::structure::read(reader)
            }
        }
        ambition_extension_sdk::script_port!($port);
    };
}
port!(
    BodyContactPort,
    "ambition.body.contact",
    2,
    Trigger,
    BodyContact
);
port!(
    ContactBouncePort,
    "ambition.body.contact_bounce",
    1,
    Request,
    ContactBounce
);
port!(
    BodyDefeatPort,
    "ambition.body.defeat",
    1,
    Request,
    BodyDefeat
);

#[cfg(test)]
mod tests {
    use super::*;

    fn round<P: Port>(v: &P::Value) -> P::Value {
        let mut out = Vec::new();
        P::encode(v, &mut out);
        let mut r = WireReader::new(&out);
        let back = P::decode(&mut r).unwrap();
        r.finish().unwrap();
        back
    }

    #[test]
    fn the_contact_values_survive_the_wire() {
        let contact = BodyContact {
            body: ContactBody {
                position: [400.0, 900.0],
                size: [40.0, 30.0],
                velocity: [-60.0, 0.0],
            },
            touchers: vec![ContactParticipant {
                id: "body:test".into(),
                driven: true,
                body: ContactBody {
                    position: [395.0, 870.5],
                    size: [24.0, 44.0],
                    velocity: [10.0, 210.0],
                },
                surfaces: vec![ContactSurface {
                    point: [400.0, 885.0],
                    normal: [0.0, -1.0],
                    distance: 0.0,
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        assert_eq!(round::<BodyContactPort>(&contact), contact);
        let bounce = ContactBounce {
            toucher: 1,
            speed: 460.0,
            pogo_cue: true,
        };
        assert_eq!(round::<ContactBouncePort>(&bounce), bounce);
        assert_eq!(round::<BodyDefeatPort>(&BodyDefeat), BodyDefeat);
    }
}
