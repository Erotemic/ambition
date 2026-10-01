//! Shockwave Slam: Attack while holding the shockwave gauntlet slams a damage
//! box around the wielder. Any wielder, player or brain. Migrated from the
//! native system (fast-iteration I7).

use ambition_combat_port::{DamageBox, DamageBoxPort, WieldedUsePort, Wielder};
use ambition_extension_sdk::{Fault, Invocation, ModuleDescriptor, Port};

use crate::wielded;

/// The held item's id.
pub const ITEM: &str = "shockwave";

const MANA_COST: f32 = 25.0;
const HALF: [f32; 2] = [120.0, 52.0];
const DAMAGE: i32 = 4;
const LIFETIME_S: f32 = 0.18;
const KNOCKBACK: f32 = 1.3;

pub fn module() -> ModuleDescriptor {
    wielded::module("shockwave", ITEM, wielded::requests(DamageBoxPort::KEY), 3, slam)
}

fn slam(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    if !wielded::pay(inv, &w, MANA_COST)? {
        return Ok(());
    }
    inv.submit::<DamageBoxPort>(DamageBox {
        center: w.position,
        // The body's own frame: a slam on a wall spreads along the wall.
        half_extent: w.to_world_half(HALF),
        damage: DAMAGE,
        knockback: KNOCKBACK,
        lifetime_s: LIFETIME_S,
    })?;
    wielded::rock_hit(inv, &w)
}
