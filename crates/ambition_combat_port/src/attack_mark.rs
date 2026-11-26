use ambition_extension_sdk::wire::{WireError, WireReader};
use ambition_extension_sdk::{structure, Port, PortKey, PortRole};

/// Show a rectangular attack mark in the body's room during `boss_conduct`.
/// Submit again to keep it visible. Each entry and body has its own slots.
pub struct AttackMarkPort;

#[derive(Clone, Debug, PartialEq)]
pub struct AttackMark {
    pub slot: u32,
    pub center: [f32; 2],
    pub half_extent: [f32; 2],
    pub progress: f32,
    pub active: bool,
}

ambition_extension_sdk::port_record! {
    AttackMark {
        slot: u32,
        /// World center in world units, with +Y down.
        center: [f32; 2],
        half_extent: [f32; 2],
        /// Progress within the warning or strike, from zero to one.
        progress: f32,
        /// Show the strike treatment when true. This has no effect on damage.
        active: bool,
    }
}

impl Port for AttackMarkPort {
    const KEY: PortKey = PortKey::new("ambition.feedback.attack_mark", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = AttackMark;

    fn encode(value: &AttackMark, out: &mut Vec<u8>) {
        structure::put(value, out);
    }

    fn decode(reader: &mut WireReader<'_>) -> Result<AttackMark, WireError> {
        structure::read(reader)
    }
}
ambition_extension_sdk::script_port!(AttackMarkPort);
