//! The body-motion ports of a wielded item: a body moves itself, arms the
//! shared movement cooldown, strikes where it is, and shows an effect.
//!
//! A module cannot see the walls, so it cannot know where a transit ends.
//! The ports after the transit take a [`Place`]: [`Place::Body`] is where the
//! body is when the request is lowered. The transit port is lowered first in
//! its phase, so `Place::Body` is the arrival.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// Where a request happens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    /// A point in the world.
    World([f32; 2]),
    /// The body the invocation ran for, where it is when the request is
    /// lowered: after a transit of this tick.
    Body,
}

impl Place {
    pub fn put(self, out: &mut Vec<u8>) {
        match self {
            Place::World(at) => {
                wire::put_u8(out, 0);
                wire::put_vec2(out, at);
            }
            Place::Body => wire::put_u8(out, 1),
        }
    }

    pub fn read(r: &mut WireReader<'_>) -> Result<Place, WireError> {
        match r.u8()? {
            0 => Ok(Place::World(r.vec2()?)),
            1 => Ok(Place::Body),
            tag => Err(WireError::BadTag(tag)),
        }
    }

    /// The point, given where the body is now.
    pub fn resolve(self, body: [f32; 2]) -> [f32; 2] {
        match self {
            Place::World(at) => at,
            Place::Body => body,
        }
    }
}

/// The request port marker for a transit: the body moves at once along a
/// line.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — move the body up to `distance` along the unit
///   `direction` (world). The body stops a body-half short of the first wall
///   of its own live room and never ends inside a solid
///   (`ambition_abilities::traversal::blink::blink_target`); the half is of
///   the box the body has, turned to the DOWN of its last step. Its velocity
///   is kept. Then, if `facing` is given, the body faces that way
///   (body-local, `1.0` is +x). The transit is a scripted teleport for the
///   Class-B ranking.
/// * **Owner** — `ambition_abilities::extension` (`transit_body`, the
///   discrete-transit authority).
/// * **Scope** — the body the invocation ran for.
/// * **Time** — `wielded_use`, lowered first of the phase's request ports
///   and as a carry of the travelled path (`BodyPathSet::Carry`): the readers
///   of the path see the arrival this tick.
/// * **Result** — every body with a movement law transits: the transit
///   authority moves each kind. A rule about which bodies an item moves is
///   the module's (blink asks [`crate::Wielder::swept`]).
pub struct TransitPort;

#[derive(Clone, Debug, PartialEq)]
pub struct Transit {
    pub direction: [f32; 2],
    pub distance: f32,
    /// The facing to take at the arrival, or `None` to keep it.
    pub facing: Option<f32>,
}

impl Port for TransitPort {
    const KEY: PortKey = PortKey::new("ambition.motion.transit", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = Transit;

    fn encode(v: &Transit, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.direction);
        wire::put_f32(out, v.distance);
        wire::put_opt(out, v.facing, wire::put_f32);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Transit, WireError> {
        Ok(Transit {
            direction: r.vec2()?,
            distance: r.f32()?,
            facing: r.opt(WireReader::f32)?,
        })
    }
}

/// The request port marker for the shared movement cooldown.
///
/// Port card:
///
/// * **Operation** — arm the body's movement-ability cooldown for `seconds`
///   (`ambition_abilities::ability_cooldown`, which blink and grapple share).
/// * **Owner** — `ambition_abilities::extension`.
/// * **Time** — `wielded_use`, after the cooldown ticks this tick.
/// * **Result** — a cooldown that runs is not armed again, and the refusal is
///   logged. A module that asked [`crate::Wielder::cooldown_ready`] is never
///   refused.
pub struct MovementCooldownPort;

#[derive(Clone, Debug, PartialEq)]
pub struct MovementCooldown {
    pub seconds: f32,
}

impl Port for MovementCooldownPort {
    const KEY: PortKey = PortKey::new("ambition.abilities.movement_cooldown", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = MovementCooldown;

    fn encode(v: &MovementCooldown, out: &mut Vec<u8>) {
        wire::put_f32(out, v.seconds);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<MovementCooldown, WireError> {
        Ok(MovementCooldown { seconds: r.f32()? })
    }
}

/// The request port marker for a strike: one hit, this tick, on what is in
/// a volume.
///
/// Port card:
///
/// * **Operation** — hit what is in `volume` for `damage`. With no
///   `knockback`, the standard knockback; with one, a stunning hit pushed
///   along `dir` (body-local sign) at `feel_scale` of the victim's feel-tuned
///   launch, from the volume's centre. The attacker is the body, so its side
///   decides whom the strike hurts (`HitEvent`, melee source).
/// * **Owner** — `ambition_combat::extension`.
/// * **Time** — `wielded_use`, before the hit resolution of the combat
///   phase: the strike lands this tick.
pub struct StrikePort;

/// Where a strike hits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrikeVolume {
    /// A circle of `radius` at `at`.
    Circle { at: Place, radius: f32 },
    /// The axis-aligned box that holds `from` and `to`, each side `pad`
    /// wider: what a body crossed from one point to the other.
    Span { from: Place, to: Place, pad: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrikeKnockback {
    pub dir: f32,
    pub feel_scale: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Strike {
    pub volume: StrikeVolume,
    pub damage: i32,
    pub knockback: Option<StrikeKnockback>,
}

impl Port for StrikePort {
    const KEY: PortKey = PortKey::new("ambition.combat.strike", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = Strike;

    fn encode(v: &Strike, out: &mut Vec<u8>) {
        match v.volume {
            StrikeVolume::Circle { at, radius } => {
                wire::put_u8(out, 0);
                at.put(out);
                wire::put_f32(out, radius);
            }
            StrikeVolume::Span { from, to, pad } => {
                wire::put_u8(out, 1);
                from.put(out);
                to.put(out);
                wire::put_f32(out, pad);
            }
        }
        wire::put_i32(out, v.damage);
        wire::put_opt(out, v.knockback, |out, k| {
            wire::put_f32(out, k.dir);
            wire::put_f32(out, k.feel_scale);
        });
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Strike, WireError> {
        let volume = match r.u8()? {
            0 => StrikeVolume::Circle {
                at: Place::read(r)?,
                radius: r.f32()?,
            },
            1 => StrikeVolume::Span {
                from: Place::read(r)?,
                to: Place::read(r)?,
                pad: r.f32()?,
            },
            tag => return Err(WireError::BadTag(tag)),
        };
        Ok(Strike {
            volume,
            damage: r.i32()?,
            knockback: r.opt(|r| {
                Ok(StrikeKnockback {
                    dir: r.f32()?,
                    feel_scale: r.f32()?,
                })
            })?,
        })
    }
}

/// The request port marker for an authored effect.
///
/// Port card:
///
/// * **Operation** — show the authored effect `fx` (an effect id, for
///   example `classic_burst`) at `at`, upright, at `scale`, in the live room
///   of the body. Presentation: no simulation state reads it.
/// * **Owner** — `ambition_abilities::extension`.
/// * **Time** — `wielded_use`.
pub struct EffectPort;

#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub at: Place,
    pub fx: String,
    pub scale: f32,
}

impl Port for EffectPort {
    const KEY: PortKey = PortKey::new("ambition.feedback.effect", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = Effect;

    fn encode(v: &Effect, out: &mut Vec<u8>) {
        v.at.put(out);
        wire::put_str(out, &v.fx);
        wire::put_f32(out, v.scale);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Effect, WireError> {
        Ok(Effect {
            at: Place::read(r)?,
            fx: r.str()?.to_owned(),
            scale: r.f32()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<P: Port>(v: &P::Value) -> P::Value {
        let mut out = Vec::new();
        P::encode(v, &mut out);
        let mut r = WireReader::new(&out);
        let back = P::decode(&mut r).expect("decodes");
        r.finish().expect("no trailing bytes");
        back
    }

    #[test]
    fn each_body_motion_value_survives_the_wire() {
        let transit = Transit { direction: [0.6, -0.8], distance: 150.0, facing: Some(-1.0) };
        assert_eq!(round_trip::<TransitPort>(&transit), transit);
        let cooldown = MovementCooldown { seconds: 0.45 };
        assert_eq!(round_trip::<MovementCooldownPort>(&cooldown), cooldown);
        let strike = Strike {
            volume: StrikeVolume::Circle { at: Place::Body, radius: 36.0 },
            damage: 2,
            knockback: None,
        };
        assert_eq!(round_trip::<StrikePort>(&strike), strike);
        let span = Strike {
            volume: StrikeVolume::Span { from: Place::World([3.0, 4.0]), to: Place::Body, pad: 48.0 },
            damage: 4,
            knockback: Some(StrikeKnockback { dir: -1.0, feel_scale: 1.4 }),
        };
        assert_eq!(round_trip::<StrikePort>(&span), span);
        let effect = Effect { at: Place::World([1.0, 2.0]), fx: "classic_burst".into(), scale: 0.35 };
        assert_eq!(round_trip::<EffectPort>(&effect), effect);
    }
}
