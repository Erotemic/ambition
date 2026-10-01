//! The combat domain's extension ports, and the held-item ports of the
//! [`wielded`] module.
//!
//! Port card for [`DamageBoxPort`] (`docs/planning/engine/extension-domain-contracts.md`):
//!
//! * **Operation** — put a short-lived damage box into the world.
//! * **Owner** — `ambition_combat::extension`, which lowers it into
//!   `EffectRequest` → `Effect::DamageBox`, the effect executor's one road.
//! * **Scope and grant** — the box's owner is the body the invocation ran
//!   for, and its FACTION is that body's own faction. A module cannot choose
//!   whom a box hurts; a body with no faction has its boxes refused.
//! * **Time** — offered in `technique_execution` and `wielded_use`, before the
//!   effect executor: the box exists this tick.
//! * **Read model** — world units, +Y down.
//! * **Result** — submitted is not applied. No acknowledgement port yet.

pub mod module_entity;
pub mod wielded;
pub use module_entity::{ModuleEntitySpawn, ModuleEntityTick, ModuleEntityTickPort, SpawnModuleEntityPort};
pub use wielded::{BodySound, BodySoundPort, SpendMana, SpendManaPort, WieldedUsePort, Wielder};

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

/// The request port marker for a HELD damage box.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — keep a damage box in the world for as long as the module
///   asks for it. The box is named by its SLOT and its GENERATION.
/// * **Owner** — `ambition_combat::extension`, which spawns, keeps and
///   despawns the box entity. The module never holds an entity.
/// * **Scope and grant** — as [`DamageBoxPort`]: the owner is the body the
///   invocation ran for, and the box is on that body's faction.
/// * **Time** — offered in `technique_execution`. Each tick the adapter runs,
///   for each body: a held box that the body's entry did NOT submit this tick
///   is despawned; a submitted (slot, generation) that is not held is spawned
///   (a held box of the same slot with another generation is despawned first);
///   a submitted (slot, generation) that is held is kept as it is (it does not
///   move, and its values are not read again). A box that ran out of lifetime
///   is not spawned again for the same generation.
/// * **Read model** — world units, +Y down.
/// * **Replay** — the adapter's record of what is held is rollback state on
///   the body.
/// * **Result** — submitted is not applied. No acknowledgement port yet.
pub struct HeldDamageBoxPort;

/// A held damage box a module asks for.
#[derive(Clone, Debug, PartialEq)]
pub struct HeldDamageBox {
    /// The module's name for the box. Slots are per entry.
    pub slot: u32,
    /// A new generation in a slot replaces the box.
    pub generation: u32,
    pub center: [f32; 2],
    pub half_extent: [f32; 2],
    pub damage: i32,
    /// A multiplier over the victim's standard feel-tuned launch.
    pub knockback: f32,
    /// The box's own lifetime: an upper bound on how long it stays held.
    pub lifetime_s: f32,
}

impl Port for HeldDamageBoxPort {
    const KEY: PortKey = PortKey::new("ambition.combat.held_damage_box", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = HeldDamageBox;

    fn encode(v: &HeldDamageBox, out: &mut Vec<u8>) {
        wire::put_u32(out, v.slot);
        wire::put_u32(out, v.generation);
        wire::put_vec2(out, v.center);
        wire::put_vec2(out, v.half_extent);
        wire::put_i32(out, v.damage);
        wire::put_f32(out, v.knockback);
        wire::put_f32(out, v.lifetime_s);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<HeldDamageBox, WireError> {
        Ok(HeldDamageBox {
            slot: r.u32()?,
            generation: r.u32()?,
            center: r.vec2()?,
            half_extent: r.vec2()?,
            damage: r.i32()?,
            knockback: r.f32()?,
            lifetime_s: r.f32()?,
        })
    }
}
