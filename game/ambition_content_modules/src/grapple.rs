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
//! * a body before any wall: the WHIP. The line's end strikes it
//!   (`ambition.combat.strike`) and tugs it toward the wielder.
//! * a loose item before any wall: the GRAB. The item flies back to the hand
//!   (`ambition.items.reel_in`), and the body takes it as it takes any item
//!   at its feet.
//! * nothing within [`RANGE`]: the miss sound, and nothing is spent.
//!
//! Driven bodies only. A bite shares the movement cooldown with the blink.
//!
//! The module reads where the line meets a wall (`ambition.items.aim_cast`):
//! the host casts, and the module decides everything from the answer.

use ambition_combat_port::{
    AimCastPort, AimHit, BodySound, BodySoundPort, Burst, BurstPort, HangOnRope, HangOnRopePort, HitMark, HitMarkPort,
    LetGoOfRope, LetGoOfRopePort, MovementCooldown, MovementCooldownPort, Place, ReelInItem, ReelInItemPort,
    SetVelocity, SetVelocityPort, Strike, StrikeKnockback, StrikePort, StrikeVolume, WieldedUsePort, Wielder,
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
/// The whip's crack at the line's end: how wide, how hard, and how much of
/// the victim's launch the tug toward the wielder is.
const WHIP_RADIUS: f32 = 22.0;
const WHIP_DAMAGE: i32 = 1;
const WHIP_TUG: f32 = 0.55;
/// How long a grabbed item flies back to the hand.
const REEL_S: f32 = 0.4;
/// Only a bite, a whip or a grab uses it, so grappling is deliberate.
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
            StrikePort::KEY,
            ReelInItemPort::KEY,
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
    // The host casts farther than the line reaches: anything past the range
    // is a miss.
    let cast = inv.observe::<AimCastPort>()?;
    let Some(caught) = caught(cast.hit, cast.body, cast.item) else {
        return inv.submit::<BodySoundPort>(sound);
    };
    if !w.cooldown_ready {
        return Ok(());
    }
    inv.submit::<MovementCooldownPort>(MovementCooldown { seconds: COOLDOWN_S })?;
    inv.submit::<BodySoundPort>(sound)?;
    let hit = match caught {
        Caught::Body(hit) => {
            let side = (hit.at[0] - from[0]) * w.frame_side[0] + (hit.at[1] - from[1]) * w.frame_side[1];
            inv.submit::<StrikePort>(Strike {
                volume: StrikeVolume::Circle { at: Place::World(hit.at), radius: WHIP_RADIUS },
                damage: WHIP_DAMAGE,
                knockback: Some(StrikeKnockback { dir: if side >= 0.0 { -1.0 } else { 1.0 }, feel_scale: WHIP_TUG }),
            })?;
            return draw_line(inv, from, hit.at);
        }
        Caught::Item(hit) => {
            inv.submit::<ReelInItemPort>(ReelInItem { at: hit.at, flight_s: REEL_S })?;
            return draw_line(inv, from, hit.at);
        }
        Caught::Wall(hit) => hit,
    };
    if w.swept && hangs_from(&w, from, hit.at) {
        inv.submit::<HangOnRopePort>(HangOnRope { anchor: hit.at, ..ROPE })?;
        return inv.submit::<HitMarkPort>(HitMark { at: Place::World(hit.at) });
    }
    let pull = wielded::unit_or_zero([hit.at[0] - from[0], hit.at[1] - from[1]]);
    inv.submit::<SetVelocityPort>(SetVelocity { velocity: [pull[0] * PULL_SPEED, pull[1] * PULL_SPEED] })?;
    draw_line(inv, from, hit.at)
}

/// What the line caught first within [`RANGE`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Caught {
    Wall(AimHit),
    Body(AimHit),
    Item(AimHit),
}

/// The first of a wall, a body and an item the line meets within [`RANGE`]
/// (the host reports a body or an item only before the wall). A body before
/// an item at the same distance is the one struck.
pub fn caught(wall: Option<AimHit>, body: Option<AimHit>, item: Option<AimHit>) -> Option<Caught> {
    [body.map(Caught::Body), item.map(Caught::Item), wall.map(Caught::Wall)]
        .into_iter()
        .flatten()
        .filter(|caught| distance(caught) < RANGE)
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
}

fn distance(caught: &Caught) -> f32 {
    match caught {
        Caught::Wall(hit) | Caught::Body(hit) | Caught::Item(hit) => hit.distance,
    }
}

/// The line, as tan sparks from the body to where it caught, and a mark there.
fn draw_line(inv: &mut Invocation<'_>, from: [f32; 2], to: [f32; 2]) -> Result<(), Fault> {
    for i in 1..LINE_SEGMENTS {
        // The engine's `lerp`: `a * (1 - s) + b * s`.
        let s = i as f32 / LINE_SEGMENTS as f32;
        let at = [from[0] * (1.0 - s) + to[0] * s, from[1] * (1.0 - s) + to[1] * s];
        inv.submit::<BurstPort>(Burst { at, count: 2, speed: 28.0, color: LINE_COLOR, kind: "spark".into() })?;
    }
    inv.submit::<HitMarkPort>(HitMark { at: Place::World(to) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(distance: f32) -> Option<AimHit> {
        Some(AimHit { at: [distance, 0.0], distance })
    }

    /// The nearest thing within the range is caught; a body before an item
    /// at one distance is the one; anything past the range is a miss.
    #[test]
    fn the_line_catches_the_nearest_thing_within_its_range() {
        assert_eq!(caught(at(200.0), at(120.0), at(80.0)), Some(Caught::Item(AimHit { at: [80.0, 0.0], distance: 80.0 })));
        assert!(matches!(caught(at(200.0), at(120.0), None), Some(Caught::Body(_))));
        assert!(matches!(caught(at(200.0), at(90.0), at(90.0)), Some(Caught::Body(_))));
        assert!(matches!(caught(at(150.0), None, None), Some(Caught::Wall(_))));
        assert_eq!(caught(at(RANGE + 1.0), at(RANGE + 0.5), None), None);
        assert_eq!(caught(None, None, None), None);
    }

    /// A bite above the body within the cap hangs it; a level bite, a bite
    /// below, and one past the cap do not.
    #[test]
    fn only_a_bite_above_hangs_the_body() {
        let w = Wielder {
            pressed: true,
            driven: true,
            position: [0.0, 0.0],
            size: [28.0, 46.0],
            facing: 1.0,
            frame_side: [1.0, 0.0],
            frame_down: [0.0, 1.0],
            aim_local: [1.0, -1.0],
            mana: None,
            names_spawns: false,
            cooldown_ready: true,
            swept: true,
            on_rope: false,
        };
        assert!(hangs_from(&w, [0.0, 0.0], [100.0, -100.0]));
        assert!(hangs_from(&w, [0.0, 0.0], [0.0, -100.0]));
        assert!(!hangs_from(&w, [0.0, 0.0], [100.0, 0.0]));
        assert!(!hangs_from(&w, [0.0, 0.0], [100.0, 50.0]));
        assert!(!hangs_from(&w, [0.0, 0.0], [300.0, -50.0]), "past the cap (80° from up)");
        // Under gravity turned up (+Y up the screen is down), "above" turns too.
        let flipped = Wielder { frame_down: [0.0, -1.0], frame_side: [-1.0, 0.0], ..w };
        assert!(hangs_from(&flipped, [0.0, 0.0], [0.0, 100.0]));
        assert!(!hangs_from(&flipped, [0.0, 0.0], [0.0, -100.0]));
    }
}
