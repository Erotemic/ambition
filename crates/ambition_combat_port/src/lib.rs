//! The combat domain's extension ports.
//!
//! Port card for [`DamageBoxPort`] (`docs/planning/engine/extension-domain-contracts.md`):
//!
//! * **Operation** — put a short-lived damage box into the world.
//! * **Owner** — `ambition_combat::extension`, which lowers it into
//!   `EffectRequest` → `Effect::DamageBox`, the effect executor's one road.
//! * **Scope and grant** — the box's owner is the body the invocation ran
//!   for, and its FACTION is that body's own faction. A module cannot choose
//!   whom a box hurts; a body with no faction has its boxes refused.
//! * **Time** — offered in `technique_execution`, before the effect executor:
//!   the box exists this tick.
//! * **Read model** — world units, +Y down.
//! * **Result** — submitted is not applied. No acknowledgement port yet.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The request port marker.
pub struct DamageBoxPort;

/// A damage box a module asks for.
#[derive(Clone, Debug, PartialEq)]
pub struct DamageBox {
    pub center: [f32; 2],
    pub half_extent: [f32; 2],
    pub damage: i32,
    /// A multiplier over the victim's standard feel-tuned launch.
    pub knockback: f32,
    pub lifetime_s: f32,
}

impl Port for DamageBoxPort {
    const KEY: PortKey = PortKey::new("ambition.combat.damage_box", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = DamageBox;

    fn encode(v: &DamageBox, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.center);
        wire::put_vec2(out, v.half_extent);
        wire::put_i32(out, v.damage);
        wire::put_f32(out, v.knockback);
        wire::put_f32(out, v.lifetime_s);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<DamageBox, WireError> {
        Ok(DamageBox {
            center: r.vec2()?,
            half_extent: r.vec2()?,
            damage: r.i32()?,
            knockback: r.f32()?,
            lifetime_s: r.f32()?,
        })
    }
}
