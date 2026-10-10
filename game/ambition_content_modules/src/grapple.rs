//! Grapple: Attack while holding the grapple casts a line along the aim. If
//! the line meets a wall within [`RANGE`], the body is pulled toward that
//! point at [`PULL_SPEED`] (a burst of velocity: collision settles the body
//! at the surface), and the line is drawn. A line into empty space plays
//! the miss sound and costs nothing. Driven bodies only. It shares the
//! movement cooldown with the blink. Migrated from the native system
//! (fast-iteration I4, body motion).
//!
//! The module reads where the line meets a wall (`ambition.items.aim_cast`):
//! the host casts, and the module decides everything from the answer.

use ambition_combat_port::{
    AimCastPort, BodySound, BodySoundPort, Burst, BurstPort, HitMark, HitMarkPort, MovementCooldown,
    MovementCooldownPort, Place, SetVelocity, SetVelocityPort, WieldedUsePort, Wielder,
};
use ambition_extension_sdk::{Fault, Invocation, ModuleDescriptor, Port};

use crate::wielded;

/// The held item's id.
pub const ITEM: &str = "grapple";

/// How far the line reaches for a wall.
pub const RANGE: f32 = 300.0;
/// The speed of the pull toward the wall.
pub const PULL_SPEED: f32 = 620.0;
/// Only a pull uses it, so grappling is deliberate.
const COOLDOWN_S: f32 = 0.55;
const SOUND: &str = "player.dash";
/// The line is drawn as tan sparks between the body and the wall, so it
/// reads as a rope that pulls the body in.
const LINE_SEGMENTS: u32 = 8;
const LINE_COLOR: [f32; 4] = [0.86, 0.78, 0.48, 0.95];

pub fn module() -> ModuleDescriptor {
    let mut module = wielded::module(
        "grapple",
        ITEM,
        vec![
            MovementCooldownPort::KEY,
            SetVelocityPort::KEY,
            BodySoundPort::KEY,
            BurstPort::KEY,
            HitMarkPort::KEY,
        ],
        // The cooldown, the pull, the sound, the line and the hit mark.
        3 + (LINE_SEGMENTS - 1) + 1,
        grapple,
    );
    module.entries[0].reads = vec![AimCastPort::KEY];
    module
}

fn grapple(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    if !w.driven {
        return Ok(());
    }
    // An aimless press casts no line.
    if wielded::unit_or_zero(w.to_world(w.aim_local)) == [0.0, 0.0] {
        return Ok(());
    }
    let from = w.position;
    let sound = BodySound { cue: SOUND.into(), at: Place::World(from) };
    // The host casts farther than the line reaches: a wall past the range is
    // a miss.
    let Some(hit) = inv.observe::<AimCastPort>()?.hit.filter(|hit| hit.distance < RANGE) else {
        return inv.submit::<BodySoundPort>(sound);
    };
    if !w.cooldown_ready {
        return Ok(());
    }
    inv.submit::<MovementCooldownPort>(MovementCooldown { seconds: COOLDOWN_S })?;
    let pull = wielded::unit_or_zero([hit.at[0] - from[0], hit.at[1] - from[1]]);
    inv.submit::<SetVelocityPort>(SetVelocity { velocity: [pull[0] * PULL_SPEED, pull[1] * PULL_SPEED] })?;
    inv.submit::<BodySoundPort>(sound)?;
    for i in 1..LINE_SEGMENTS {
        // The engine's `lerp`: `a * (1 - s) + b * s`.
        let s = i as f32 / LINE_SEGMENTS as f32;
        let at = [from[0] * (1.0 - s) + hit.at[0] * s, from[1] * (1.0 - s) + hit.at[1] * s];
        inv.submit::<BurstPort>(Burst { at, count: 2, speed: 28.0, color: LINE_COLOR, kind: "spark".into() })?;
    }
    inv.submit::<HitMarkPort>(HitMark { at: Place::World(hit.at) })
}
