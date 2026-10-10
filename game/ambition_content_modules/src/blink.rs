//! Blink: Attack while holding the blink moves the body at once up to
//! [`DISTANCE`] along the aim, walls permitting, and strikes where it
//! arrives. Driven bodies only. It shares the movement cooldown with the
//! grapple. Migrated from the native system (fast-iteration I4, body motion).
//!
//! The module cannot see the walls, so it does not know where the body
//! arrives. The strike, the sound and the arrival effect are at
//! `Place::Body`: the transit port is lowered first, so that is the arrival.

use ambition_combat_port::{
    BodySound, BodySoundPort, Effect, EffectPort, MovementCooldown, MovementCooldownPort, Place, Strike, StrikePort,
    Transit, TransitPort, WieldedUsePort, Wielder,
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

/// `v` with length one, or zero when it has no direction. The same
/// arithmetic as the engine's `normalize_or_zero`: a multiply by the
/// reciprocal of the length.
fn unit_or_zero(v: [f32; 2]) -> [f32; 2] {
    let recip = 1.0 / (v[0] * v[0] + v[1] * v[1]).sqrt();
    if recip.is_finite() && recip > 0.0 {
        [v[0] * recip, v[1] * recip]
    } else {
        [0.0, 0.0]
    }
}

fn blink(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    if !w.driven || !w.transits {
        return Ok(());
    }
    let direction = unit_or_zero(w.to_world(w.aim_local));
    // An aimless press does not use the cooldown.
    if direction == [0.0, 0.0] || !w.cooldown_ready {
        return Ok(());
    }
    inv.submit::<MovementCooldownPort>(MovementCooldown { seconds: COOLDOWN_S })?;
    inv.submit::<TransitPort>(Transit { direction, distance: DISTANCE })?;
    inv.submit::<StrikePort>(Strike {
        at: Place::Body,
        radius: STRIKE_RADIUS,
        damage: STRIKE_DAMAGE,
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
