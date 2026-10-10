//! Blink: Attack while holding the blink moves the body at once up to
//! [`DISTANCE`] along the aim, walls permitting, and strikes where it
//! arrives. Driven bodies only. It shares the movement cooldown with the
//! grapple. Migrated from the native system (fast-iteration I4, body motion).
//!
//! The module cannot see the walls, so it does not know where the body
//! arrives. The strike, the sound and the arrival effect are at
//! `Place::Body`: the transit port is lowered first, so that is the arrival.

use ambition_combat_port::{
    BodySound, BodySoundPort, Destination, Effect, EffectPort, MovementCooldown, MovementCooldownPort, Place, Strike, StrikePort,
    StrikeVolume, Transit, TransitPort, WieldedUsePort, Wielder,
};
use ambition_extension_sdk::{Fault, Invocation, ModuleDescriptor, Port};

use crate::wielded;

/// The held item's id.
pub const ITEM: &str = "blink";

/// How far a blink carries the body along the aim, walls permitting.
pub const DISTANCE: f32 = 150.0;
/// A deliberate move, not a stream of moves.
const COOLDOWN_S: f32 = 0.45;
/// The arrival strike: you can blink into a group of enemies to hit them.
/// Mobility first, so the hit is light.
const STRIKE_RADIUS: f32 = 36.0;
const STRIKE_DAMAGE: i32 = 2;
const EFFECT: &str = "classic_burst";

pub fn module() -> ModuleDescriptor {
    wielded::module(
        "blink",
        ITEM,
        vec![
            TransitPort::KEY,
            MovementCooldownPort::KEY,
            StrikePort::KEY,
            BodySoundPort::KEY,
            EffectPort::KEY,
        ],
        6,
        blink,
    )
}

fn blink(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    // A body that does not move by the swept kernel does not blink.
    if !w.driven || !w.swept {
        return Ok(());
    }
    let direction = wielded::unit_or_zero(w.to_world(w.aim_local));
    // An aimless press does not use the cooldown.
    if direction == [0.0, 0.0] || !w.cooldown_ready {
        return Ok(());
    }
    inv.submit::<MovementCooldownPort>(MovementCooldown { seconds: COOLDOWN_S })?;
    inv.submit::<TransitPort>(Transit {
        to: Destination::Along { direction, distance: DISTANCE },
        facing: None,
    })?;
    inv.submit::<StrikePort>(Strike {
        volume: StrikeVolume::Circle { at: Place::Body, radius: STRIKE_RADIUS },
        damage: STRIKE_DAMAGE,
        knockback: None,
    })?;
    inv.submit::<BodySoundPort>(BodySound {
        cue: "player.blink".into(),
        at: Place::Body,
    })?;
    // A small burst where the body was, a larger one where it arrives.
    inv.submit::<EffectPort>(Effect {
        at: Place::World(w.position),
        fx: EFFECT.into(),
        scale: 0.35,
    })?;
    inv.submit::<EffectPort>(Effect {
        at: Place::Body,
        fx: EFFECT.into(),
        scale: 0.5,
    })
}
