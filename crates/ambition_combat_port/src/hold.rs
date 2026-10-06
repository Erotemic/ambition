//! A body held by another: seized, carried at a point, pummelled, thrown,
//! released. The engine's capture relation (`ambition_combat::capture`),
//! offered to a module.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The request port marker for a hold.
///
/// Port card:
///
/// * **Operation** — one of:
///   * `Seize` — catch the nearest body that the owner's damage can land on
///     whose box meets the reach volume (`reach_offset`, `reach_half`: a box
///     in the owner's local frame, +x the way it faces, +y down), and hold it
///     at `hold_offset` (local the same way) for at most `hold_s` seconds or
///     until it mashes free. The held body's move ends, its control is held,
///     gravity leaves it. A body already held, in hitstun, out of play or in
///     another live room is not caught. The owner holds one body at a time.
///   * `Carry` — move the held body to `hold_offset` this tick (a shake is a
///     carry every tick).
///   * `Pummel` — `damage` to the held body, no knockback: it stays held.
///   * `Throw` — `damage`, then release, then launch it along `launch_dir`
///     (owner-local) at a launch speed from `knockback` and `growth`, with
///     the hit reaction a strike gives (hitstun).
///   * `Release` — let go, no launch.
///   With nothing held, `Carry`, `Pummel`, `Throw` and `Release` do nothing.
/// * **Owner** — `ambition_combat::extension` (the capture relation).
/// * **Scope** — the owner is the body the invocation ran for.
/// * **Time** — offered in `boss_conduct`. A seize, carry, pummel and throw are
///   applied this tick, in the combat phase. The trigger reports what is held
///   (`BossConduct::holding`) on the next tick.
/// * **Replay** — the relation is rollback state on the held body.
pub struct BodyHoldPort;

#[derive(Clone, Debug, PartialEq)]
pub enum BodyHold {
    Seize { reach_offset: [f32; 2], reach_half: [f32; 2], hold_offset: [f32; 2], hold_s: f32 },
    Carry { hold_offset: [f32; 2] },
    Pummel { damage: i32 },
    Throw { damage: i32, knockback: f32, growth: f32, launch_dir: [f32; 2] },
    Release,
}

impl Port for BodyHoldPort {
    const KEY: PortKey = PortKey::new("ambition.combat.body_hold", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = BodyHold;

    fn encode(v: &BodyHold, out: &mut Vec<u8>) {
        match v {
            BodyHold::Seize { reach_offset, reach_half, hold_offset, hold_s } => {
                wire::put_u8(out, 0);
                wire::put_vec2(out, *reach_offset);
                wire::put_vec2(out, *reach_half);
                wire::put_vec2(out, *hold_offset);
                wire::put_f32(out, *hold_s);
            }
            BodyHold::Carry { hold_offset } => {
                wire::put_u8(out, 1);
                wire::put_vec2(out, *hold_offset);
            }
            BodyHold::Pummel { damage } => {
                wire::put_u8(out, 2);
                wire::put_i32(out, *damage);
            }
            BodyHold::Throw { damage, knockback, growth, launch_dir } => {
                wire::put_u8(out, 3);
                wire::put_i32(out, *damage);
                wire::put_f32(out, *knockback);
                wire::put_f32(out, *growth);
                wire::put_vec2(out, *launch_dir);
            }
            BodyHold::Release => wire::put_u8(out, 4),
        }
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BodyHold, WireError> {
        Ok(match r.u8()? {
            0 => BodyHold::Seize { reach_offset: r.vec2()?, reach_half: r.vec2()?, hold_offset: r.vec2()?, hold_s: r.f32()? },
            1 => BodyHold::Carry { hold_offset: r.vec2()? },
            2 => BodyHold::Pummel { damage: r.i32()? },
            3 => BodyHold::Throw { damage: r.i32()?, knockback: r.f32()?, growth: r.f32()?, launch_dir: r.vec2()? },
            4 => BodyHold::Release,
            tag => return Err(WireError::BadTag(tag)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_hold_survives_the_wire() {
        for v in [
            BodyHold::Seize { reach_offset: [90.0, 20.0], reach_half: [60.0, 40.0], hold_offset: [120.0, -10.0], hold_s: 2.5 },
            BodyHold::Carry { hold_offset: [100.0, -30.0] },
            BodyHold::Pummel { damage: 1 },
            BodyHold::Throw { damage: 2, knockback: 1.6, growth: 0.4, launch_dir: [-0.6, -0.8] },
            BodyHold::Release,
        ] {
            let mut out = Vec::new();
            BodyHoldPort::encode(&v, &mut out);
            assert_eq!(BodyHoldPort::decode(&mut WireReader::new(&out)).unwrap(), v);
        }
    }
}
