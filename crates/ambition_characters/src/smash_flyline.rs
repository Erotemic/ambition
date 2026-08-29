//! Being lifted out of the scene on a wire: the authored vocabulary.
//!
//! ⭐ THE SAME SPLIT `smash_teleport`, `smash_trapdoor`, `smash_vitality` AND
//! `smash_ride` USE. A key and its params are what a MOVESET authors; hanging a
//! body off a pendulum on a winch, integrating it, and deciding what it is
//! travelling at when the rope lets go is engine work.
//!
//! ⭐⭐ JON'S DESIGN, 2026-08-29: *"It is not a teleport and should not get the
//! teleport sound. It needs to be a rope or wire that reaches down from the sky
//! (it can instantly appear as if it went from visible to invisible), but she
//! doesn't teleport up, she gets lifted up by the wire, a fairly large vertical
//! distance, and while she is being lifted by the wire her motion controls
//! should let her swing like a pendulum so she has a bit of horizontal recovery
//! with it too."*
//!
//! ⛔⛔ SO IT IS NOT `smash.teleport` WITH DIFFERENT NUMBERS, and that is the
//! whole reason this module exists. `apply_authored_teleports` emits
//! `PLAYER_BLINK` at every transit — the cue Jon is complaining about comes from
//! the EXECUTOR, not from any timeline, so a move that runs the teleport
//! executor IS a teleport however it is commented. The fix is a different
//! technique, and this is it.
//!
//! ⛔ AND THE MOTION IS NOT AUTHORED HERE. What "on a wire" means — no gravity,
//! a position of `(anchor, length, angle)`, a stick that buys ANGULAR
//! acceleration, and one release that writes one exit velocity — is a property
//! of the movement kernel, stated once in `integrate_wire_clusters`. This module
//! only says WHEN, and with what rope.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const FLYLINE: &str = "smash.flyline";

/// One catch of a flyline: the rope, the lift, and what the swing is allowed to
/// buy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlylineParams {
    /// How far above her the wire's anchor is when it catches, in world px.
    ///
    /// ⭐⭐ THIS IS THE SWING RADIUS, not the travel. A LONG rope swings slowly
    /// through a wide arc and carries her a long way sideways for a small angle;
    /// a short one snaps back fast and barely moves her. It is the one number
    /// that decides what the pendulum FEELS like, and it is separate from
    /// [`Self::rise`] because how far she goes up and how far she can swing while
    /// going there are two different questions.
    ///
    /// ⛔ IT MUST EXCEED [`Self::rise`], or the winch reels the rope past its own
    /// pulley and the lift stops short at the kernel's minimum length. The
    /// content test that walks her table is what checks it.
    pub rope_length: f32,
    /// How far the winch lifts her over the whole beat, in world px.
    ///
    /// ⭐ THE CLAUSE JON STATED AS *"a fairly large vertical distance"*, and the
    /// only honest way to hold him to it is a number measured against a stage.
    pub rise: f32,
    /// How long the lift takes, in seconds. The winch speed is `rise / lift_s`,
    /// so these two together are the whole of the climb.
    ///
    /// ⛔⛔ THE MOVE'S OWN TIMELINE MUST OUTLAST IT. A wire still reeling when
    /// the move ends is a body being flown by a maneuver nothing is animating;
    /// the guard is in the content test, because this function cannot see the
    /// rest of the timeline any more than `author_trapdoor` can.
    pub lift_s: f32,
    /// How far the swing may reach from straight down, in DEGREES.
    ///
    /// ⛔ DEGREES IN THE AUTHORING, radians in the kernel. A moveset is written
    /// by a person and the conversion is one call; the alternative is authored
    /// content carrying `0.4363` and nobody able to see that it is 25°.
    pub max_swing_deg: f32,
    /// What a held stick contributes, in radians per second squared.
    pub swing_accel: f32,
    /// How fast she is still rising when the wire lets go, in px/s.
    ///
    /// ⛔⛔ IT LIVES HERE BECAUSE THE RELEASE IS THE ONE WRITER OF EXIT VELOCITY,
    /// and the trapdoor is why that sentence is written down: `LEAP_OUT_SPEED`
    /// was authored as an `Impulse` AND as a technique beat on the same frame,
    /// the later system overwrote the impulse every single time, and the move
    /// leapt nowhere for as long as the constant existed. The wire has one
    /// writer, in `integrate_wire_clusters`, and this is its input.
    ///
    /// `0.0` cuts her loose at whatever the swing was doing and nothing more.
    pub release_rise: f32,
    /// The effect drawn where the wire takes hold.
    ///
    /// ⛔ NOT THE WIRE ITSELF. The rope is a persistent object for the length of
    /// the lift and is drawn from the read model, the way the trapdoor is; this
    /// is the one-shot at the catch. The distinction cost `rendering/submerged.rs`
    /// its own paragraph: an FX-atlas row plays once and ends, and a thing that
    /// has to stay on screen while a state holds is not that.
    pub vfx: String,
    /// The cue played at the catch. ⛔ NOT `player.blink`.
    pub sfx: String,
}

/// Author one flyline catch onto a move's timeline.
///
/// # Panics
///
/// If `at_s` is past the move's own duration — a catch scheduled after the move
/// ends never fires, which is an up-B that does nothing at all.
pub fn author_flyline(mut spec: MoveSpec, at_s: f32, params: FlylineParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` catches a flyline at {at_s}s but only lasts {}s, so the beat \
         would never fire",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: FLYLINE.to_string(),
            params: ParamValue::from_typed(&params).expect("flyline params serialize"),
        }),
    });
    spec
}
