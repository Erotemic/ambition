//! What Sanic's game tells a module about a badnik's contact: the badnik
//! rules of its pack, and each toucher's own facts. Both are observations the
//! game answers at each invocation, so a module keeps no copy of a value the
//! pack or a body owns.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The badnik rules of his pack (`sanic_rules.ron`): the stomp band, the
/// bounce, and the squash's bursts and sound.
pub struct BadnikRulesPort;

#[derive(Clone, Debug, PartialEq)]
pub struct BadnikRules {
    /// How far (px) a player's feet may be from a badnik's head and still be
    /// on it (`stomp.band`).
    pub band: f32,
    /// The upward speed off a stomped badnik (`badnik.bounce_speed`).
    pub bounce_speed: f32,
    /// The bursts where a badnik is squashed, in their order
    /// (`effects.badnik_squash`).
    pub squash: Vec<SquashBurst>,
    /// The badnik's own sound when it pops.
    pub cue: String,
}

/// One burst of the squash: how many, how fast (px per second), their colour
/// (sRGB and alpha, 0 to 1) and their kind, as the burst port names it.
#[derive(Clone, Debug, PartialEq)]
pub struct SquashBurst {
    pub count: u32,
    pub speed: f32,
    pub color: [f32; 4],
    pub kind: String,
}

impl Port for BadnikRulesPort {
    const KEY: PortKey = PortKey::new("sanic.rules.badnik", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = BadnikRules;

    fn encode(v: &BadnikRules, out: &mut Vec<u8>) {
        wire::put_f32(out, v.band);
        wire::put_f32(out, v.bounce_speed);
        wire::put_u32(out, v.squash.len() as u32);
        for burst in &v.squash {
            wire::put_u32(out, burst.count);
            wire::put_f32(out, burst.speed);
            for part in burst.color {
                wire::put_f32(out, part);
            }
            wire::put_str(out, &burst.kind);
        }
        wire::put_str(out, &v.cue);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BadnikRules, WireError> {
        let band = r.f32()?;
        let bounce_speed = r.f32()?;
        let len = r.u32()?;
        let squash = (0..len)
            .map(|_| {
                Ok(SquashBurst {
                    count: r.u32()?,
                    speed: r.f32()?,
                    color: [r.f32()?, r.f32()?, r.f32()?, r.f32()?],
                    kind: r.str()?.to_owned(),
                })
            })
            .collect::<Result<Vec<_>, WireError>>()?;
        Ok(BadnikRules { band, bounce_speed, squash, cue: r.str()?.to_owned() })
    }
}

/// Each toucher of the contact that triggered the invocation, in the
/// trigger's order: whether it rolls (the ball dash's `Rolling`, the game's
/// fact) and whether its touch harms (the empowerment its grant and its worn
/// character give it). A toucher the game cannot find is neither.
pub struct ContactTouchersPort;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ToucherFacts {
    pub rolling: bool,
    pub harms_on_contact: bool,
}

impl Port for ContactTouchersPort {
    const KEY: PortKey = PortKey::new("sanic.contact.touchers", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = Vec<ToucherFacts>;

    fn encode(v: &Vec<ToucherFacts>, out: &mut Vec<u8>) {
        wire::put_u32(out, v.len() as u32);
        for facts in v {
            wire::put_bool(out, facts.rolling);
            wire::put_bool(out, facts.harms_on_contact);
        }
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Vec<ToucherFacts>, WireError> {
        let len = r.u32()?;
        (0..len).map(|_| Ok(ToucherFacts { rolling: r.bool()?, harms_on_contact: r.bool()? })).collect()
    }
}
