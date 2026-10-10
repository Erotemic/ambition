//! Overflow Crash: Attack while holding the dive gauntlet lunges the body up
//! to [`LUNGE`] along the aim, snapped to its larger body axis, walls
//! permitting, and hits everything in the corridor it crossed. Driven bodies
//! only. Migrated from the native system (fast-iteration I4, body motion).
//!
//! The module does not know where the lunge ends, so the corridor is a
//! `StrikeVolume::Span` from where the body was to `Place::Body`: the
//! transit port is lowered first, so that is the arrival.

use ambition_combat_port::{
    BodySound, BodySoundPort, Destination, Place, SpendManaPort, Strike, StrikeKnockback, StrikePort, StrikeVolume, Transit,
    TransitPort, WieldedUsePort, Wielder,
};
use ambition_extension_sdk::{Fault, Invocation, ModuleDescriptor, Port};

use crate::wielded;

/// The held item's id.
pub const ITEM: &str = "dive";

/// Mana per lunge (of 100): it cannot cross a room in a stream of lunges.
const MANA_COST: f32 = 26.0;
/// How far the body lunges along the aim, if no wall stops it.
pub const LUNGE: f32 = 140.0;
/// The corridor is this much wider than the line the body crossed.
const WIDTH: f32 = 48.0;
const DAMAGE: i32 = 4;
/// The push along the lunge, a multiple of the victim's feel-tuned launch.
const KNOCKBACK: f32 = 1.4;

pub fn module() -> ModuleDescriptor {
    wielded::module(
        "dive",
        ITEM,
        vec![SpendManaPort::KEY, TransitPort::KEY, StrikePort::KEY, BodySoundPort::KEY],
        4,
        dive,
    )
}

/// The lunge, body-local: a unit vector on the aim's larger axis. With no
/// aim, ahead of the body, so a plain Attack lunges too.
pub fn direction(aim: [f32; 2], facing: f32) -> [f32; 2] {
    let horizontal = aim == [0.0, 0.0] || aim[0].abs() >= aim[1].abs();
    if horizontal {
        let side = if aim[0].abs() > 0.001 { aim[0].signum() } else { facing.signum() };
        [side, 0.0]
    } else {
        [0.0, aim[1].signum()]
    }
}

fn dive(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    if !w.driven || !wielded::pay(inv, &w, MANA_COST)? {
        return Ok(());
    }
    let local = direction(w.aim_local, w.facing);
    let world = wielded::unit_or_zero(w.to_world(local));
    inv.submit::<TransitPort>(Transit {
        to: Destination::Along { direction: world, distance: LUNGE },
        // A sideways lunge turns the body to face along it.
        facing: (local[0].abs() > 0.001).then(|| local[0].signum()),
    })?;
    inv.submit::<StrikePort>(Strike {
        volume: StrikeVolume::Span {
            from: Place::World(w.position),
            to: Place::Body,
            pad: WIDTH,
        },
        damage: DAMAGE,
        knockback: Some(StrikeKnockback {
            dir: local[0].signum(),
            feel_scale: KNOCKBACK,
        }),
    })?;
    inv.submit::<BodySoundPort>(BodySound {
        cue: "player.blink".into(),
        at: Place::Body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lunge_snaps_to_the_aims_larger_axis() {
        assert_eq!(direction([0.0, 0.0], -1.0), [-1.0, 0.0], "no aim: ahead");
        assert_eq!(direction([0.2, -0.9], 1.0), [0.0, -1.0], "mostly up");
        assert_eq!(direction([0.7, 0.7], -1.0), [1.0, 0.0], "a tie is along the side axis");
        assert_eq!(direction([0.0005, 0.0], -1.0), [-1.0, 0.0], "a stick at rest: ahead");
    }
}
