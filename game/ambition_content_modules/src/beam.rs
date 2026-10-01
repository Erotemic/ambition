//! Focus Beam: Attack while holding the beam fires a short line of damage
//! along the aim, snapped to the body's horizontal or vertical axis. Driven
//! bodies only. Migrated from the native system (fast-iteration I7).

use ambition_combat_port::{DamageBox, DamageBoxPort, WieldedUsePort, Wielder};
use ambition_extension_sdk::{Fault, Invocation, ModuleDescriptor, Port};

use crate::wielded;

/// The held item's id.
pub const ITEM: &str = "beam";

const MANA_COST: f32 = 30.0;
const LENGTH: f32 = 300.0;
const WIDTH: f32 = 30.0;
const DAMAGE: i32 = 5;
const LIFETIME_S: f32 = 0.12;
const KNOCKBACK: f32 = 1.1;

pub fn module() -> ModuleDescriptor {
    wielded::module("beam", ITEM, wielded::requests(DamageBoxPort::KEY), 3, fire)
}

/// The beam's body-local (centre offset, half size): along the aim's larger
/// axis; with no aim, ahead of the body.
pub fn geometry(aim: [f32; 2], facing: f32) -> ([f32; 2], [f32; 2]) {
    let (half_len, half_wid) = (LENGTH * 0.5, WIDTH * 0.5);
    let horizontal = aim == [0.0, 0.0] || aim[0].abs() >= aim[1].abs();
    if horizontal {
        let dir = if aim[0].abs() > 0.001 { aim[0].signum() } else { facing.signum() };
        ([dir * half_len, 0.0], [half_len, half_wid])
    } else {
        ([0.0, aim[1].signum() * half_len], [half_wid, half_len])
    }
}

fn fire(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    if !w.driven || !wielded::pay(inv, &w, MANA_COST)? {
        return Ok(());
    }
    let (offset_local, half_local) = geometry(w.aim_local, w.facing);
    let offset = w.to_world(offset_local);
    inv.submit::<DamageBoxPort>(DamageBox {
        center: [w.position[0] + offset[0], w.position[1] + offset[1]],
        half_extent: w.to_world_half(half_local),
        damage: DAMAGE,
        knockback: KNOCKBACK,
        lifetime_s: LIFETIME_S,
    })?;
    wielded::rock_hit(inv, &w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_beam_runs_along_the_aims_larger_axis() {
        assert_eq!(geometry([0.0, 0.0], -1.0), ([-150.0, 0.0], [150.0, 15.0]), "no aim: ahead");
        assert_eq!(geometry([0.2, -0.9], 1.0), ([0.0, -150.0], [15.0, 150.0]), "mostly up: tall");
        assert_eq!(geometry([0.7, 0.7], -1.0), ([150.0, 0.0], [150.0, 15.0]), "a tie is horizontal");
    }
}
