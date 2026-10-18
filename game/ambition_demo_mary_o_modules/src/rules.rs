//! Mary-O's rules that a module reads: an observation the game answers from
//! her pack (`stomp` in `mary_o_rules.ron`), at each invocation. A module that
//! kept these as constants would be a second authority, and an edit to the
//! pack would not reach it.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The stomp rules of her enemies.
pub struct StompRulesPort;

/// The stomp rules of her enemies, as her pack states them.
#[derive(Clone, Debug, PartialEq)]
pub struct StompRules {
    /// The vertical band (px) within which a player's feet count as on a
    /// body's head ([`crate::stomp::player_touch`]).
    pub band: f32,
    /// The upward speed off a stomped enemy.
    pub bounce_speed: f32,
    /// The dust where an enemy is stomped (`stomp.dust`).
    pub dust: StompDust,
}

/// The dust of a stomp: how many, how fast (px per second), their colour
/// (sRGB and alpha, 0 to 1) and their kind, as the burst port names it.
#[derive(Clone, Debug, PartialEq)]
pub struct StompDust {
    pub count: u32,
    pub speed: f32,
    pub color: [f32; 4],
    pub kind: String,
}

impl Port for StompRulesPort {
    const KEY: PortKey = PortKey::new("mary_o.rules.stomp", 2);
    const ROLE: PortRole = PortRole::Observation;
    type Value = StompRules;

    fn encode(v: &StompRules, out: &mut Vec<u8>) {
        wire::put_f32(out, v.band);
        wire::put_f32(out, v.bounce_speed);
        wire::put_u32(out, v.dust.count);
        wire::put_f32(out, v.dust.speed);
        for part in v.dust.color {
            wire::put_f32(out, part);
        }
        wire::put_str(out, &v.dust.kind);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<StompRules, WireError> {
        Ok(StompRules {
            band: r.f32()?,
            bounce_speed: r.f32()?,
            dust: StompDust {
                count: r.u32()?,
                speed: r.f32()?,
                color: [r.f32()?, r.f32()?, r.f32()?, r.f32()?],
                kind: r.str()?.to_owned(),
            },
        })
    }
}
