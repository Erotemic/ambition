//! Grapple: Attack while holding the grapple casts a line along the aim.
//! What the line bites decides what it does:
//!
//! * a wall ABOVE the body, within [`BITE_CAP`] of straight up: the body
//!   hangs from the bite on a held rope (`ambition.motion.hang_on_rope`). The
//!   kernel swings it under gravity and the stick, reels it on up and down,
//!   and lets go on Jump with the swing's speed and a hop; Attack again lets
//!   go with the swing's speed alone. The rope is drawn from the body's motion
//!   facts while it holds.
//! * a wall level with or below the body (or a body that cannot hang): the
//!   body is pulled toward the bite at [`PULL_SPEED`] (a burst of velocity:
//!   collision settles the body at the surface), and the line is drawn.
//! * nothing within [`RANGE`]: the miss sound, and nothing is spent.
//!
//! Driven bodies only. A bite shares the movement cooldown with the blink.
//!
//! The module reads where the line meets a wall (`ambition.items.aim_cast`):
//! the host casts, and the module decides everything from the answer.

use ambition_combat_port::{
    AimCastPort, BodySound, BodySoundPort, Burst, BurstPort, HangOnRope, HangOnRopePort, HitMark, HitMarkPort,
    LetGoOfRope, LetGoOfRopePort, MovementCooldown, MovementCooldownPort, Place, SetVelocity, SetVelocityPort,
    WieldedUsePort, Wielder,
};
use ambition_extension_sdk::{Fault, Invocation, ModuleDescriptor, Port};

use crate::wielded;

/// The held item's id.
pub const ITEM: &str = "grapple";

/// How far the line reaches for a wall.
pub const RANGE: f32 = 300.0;
/// The speed of the pull toward the wall.
pub const PULL_SPEED: f32 = 620.0;
/// The farthest from straight up a bite may be and still hang the body
/// (radians): flatter bites zip.
pub const BITE_CAP: f32 = 1.2;
/// The rope a bite hangs the body on.
pub const ROPE: HangOnRope = HangOnRope {
    anchor: [0.0, 0.0],
    // Past the bite cap, so a pumped swing can rise above the side of the
    // bite before the fading drive holds it.
    max_angle: 1.75,
    swing_accel: 7.0,
    reel_speed: 220.0,
    min_length: 48.0,
    max_length: RANGE,
    release_hop: 280.0,
};
/// Only a bite uses it, so grappling is deliberate.
const COOLDOWN_S: f32 = 0.55;
const SOUND: &str = "player.dash";
/// The zip's line is drawn as tan sparks between the body and the wall, so
/// it reads as a rope that pulls the body in.
const LINE_SEGMENTS: u32 = 8;
const LINE_COLOR: [f32; 4] = [0.86, 0.78, 0.48, 0.95];

pub fn module() -> ModuleDescriptor {
    let mut module = wielded::module(
        "grapple",
        ITEM,
        vec![
            MovementCooldownPort::KEY,
            SetVelocityPort::KEY,
            HangOnRopePort::KEY,
            LetGoOfRopePort::KEY,
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

/// Whether a bite at `at` hangs a body at `from`: the bite is within
/// [`BITE_CAP`] of straight up in the body's gravity frame.
pub fn hangs_from(w: &Wielder, from: [f32; 2], at: [f32; 2]) -> bool {
    let to = [at[0] - from[0], at[1] - from[1]];
    let side = to[0] * w.frame_side[0] + to[1] * w.frame_side[1];
    let up = -(to[0] * w.frame_down[0] + to[1] * w.frame_down[1]);
    up > 0.0 && side.atan2(up).abs() <= BITE_CAP
}

fn grapple(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    if !w.driven {
        return Ok(());
    }
    let from = w.position;
    let sound = BodySound { cue: SOUND.into(), at: Place::World(from) };
    // A press on the rope lets go of it.
    if w.on_rope {
        inv.submit::<LetGoOfRopePort>(LetGoOfRope {})?;
        return inv.submit::<BodySoundPort>(sound);
    }
    // An aimless press casts no line.
    if wielded::unit_or_zero(w.to_world(w.aim_local)) == [0.0, 0.0] {
        return Ok(());
    }
    // The host casts farther than the line reaches: a wall past the range is
    // a miss.
    let Some(hit) = inv.observe::<AimCastPort>()?.hit.filter(|hit| hit.distance < RANGE) else {
        return inv.submit::<BodySoundPort>(sound);
    };
    if !w.cooldown_ready {
        return Ok(());
    }
    inv.submit::<MovementCooldownPort>(MovementCooldown { seconds: COOLDOWN_S })?;
    inv.submit::<BodySoundPort>(sound)?;
    if w.swept && hangs_from(&w, from, hit.at) {
        inv.submit::<HangOnRopePort>(HangOnRope { anchor: hit.at, ..ROPE })?;
        return inv.submit::<HitMarkPort>(HitMark { at: Place::World(hit.at) });
    }
    let pull = wielded::unit_or_zero([hit.at[0] - from[0], hit.at[1] - from[1]]);
    inv.submit::<SetVelocityPort>(SetVelocity { velocity: [pull[0] * PULL_SPEED, pull[1] * PULL_SPEED] })?;
    for i in 1..LINE_SEGMENTS {
        // The engine's `lerp`: `a * (1 - s) + b * s`.
        let s = i as f32 / LINE_SEGMENTS as f32;
        let at = [from[0] * (1.0 - s) + hit.at[0] * s, from[1] * (1.0 - s) + hit.at[1] * s];
        inv.submit::<BurstPort>(Burst { at, count: 2, speed: 28.0, color: LINE_COLOR, kind: "spark".into() })?;
    }
    inv.submit::<HitMarkPort>(HitMark { at: Place::World(hit.at) })
}
