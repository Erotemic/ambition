//! A field of force a module holds over a region of its room: a wind, a
//! tide, the low weight of space.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The request port marker for a held field of force.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — every body of the room whose box overlaps the box
///   `min`..`max` changes its velocity this tick by two parts:
///   * `lift` (0..1) takes that part of the body's own gravity away: a lift
///     of 0.8 leaves a fifth of its weight, the low weight of space. Gravity
///     is an acceleration, so it is the same for a light and a heavy body.
///   * `force` (world units per second per second on a body of mass 1, +Y
///     down) pushes it: a body of mass `m` takes `force / m` (Newton's second
///     law). A tide that throws the player across the room moves the giant
///     an eighth as far.
///
///   * `well` (v2), when there is one, pulls each body toward its point
///     `at`: an acceleration of `pull` at `near` or nearer, and of
///     `pull * near / r` at a distance `r` past it (a flat world's gravity
///     falls off as `1 / r`). Gravity is an acceleration, so the well pulls
///     a light and a heavy body the same: the giant falls in as the player
///     does.
///
///   It does not turn which way a body's feet point.
/// * **Owner** — `ambition_abilities::extension`.
/// * **Scope** — the bodies of the requester's own live room. `slot` names
///   the field among the requester's fields.
/// * **Time** — offered in `boss_conduct`. The field is HELD: it acts on each
///   tick its requester asks for it, and on no other. The change goes into
///   the body's velocity on that tick. The field has no state of its own, so
///   a rewind and a replay give the same velocities.
/// * **Read model** — world units, +Y down.
pub struct HeldForcePort;

#[derive(Clone, Debug, PartialEq)]
pub struct HeldForce {
    pub slot: u32,
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub lift: f32,
    pub force: [f32; 2],
    /// A well of gravity in the field (v2).
    pub well: Option<GravityWell>,
}

/// A point that pulls each body of a field toward it (see the port card).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GravityWell {
    pub at: [f32; 2],
    /// World units per second per second, at `near` or nearer.
    pub pull: f32,
    pub near: f32,
}

impl GravityWell {
    /// The well's acceleration on a body at `from` (+Y down).
    pub fn acceleration(&self, from: [f32; 2]) -> [f32; 2] {
        let (dx, dy) = (self.at[0] - from[0], self.at[1] - from[1]);
        let r = (dx * dx + dy * dy).sqrt();
        if r < 1e-3 {
            return [0.0, 0.0];
        }
        let a = self.pull * (self.near.max(0.0) / r).min(1.0);
        [dx / r * a, dy / r * a]
    }
}

impl Port for HeldForcePort {
    const KEY: PortKey = PortKey::new("ambition.world.held_force", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = HeldForce;

    fn encode(v: &HeldForce, out: &mut Vec<u8>) {
        wire::put_u32(out, v.slot);
        wire::put_vec2(out, v.min);
        wire::put_vec2(out, v.max);
        wire::put_f32(out, v.lift);
        wire::put_vec2(out, v.force);
        wire::put_opt(out, v.well, |out, well| {
            wire::put_vec2(out, well.at);
            wire::put_f32(out, well.pull);
            wire::put_f32(out, well.near);
        });
    }

    fn decode(r: &mut WireReader<'_>) -> Result<HeldForce, WireError> {
        Ok(HeldForce {
            slot: r.u32()?,
            min: r.vec2()?,
            max: r.vec2()?,
            lift: r.f32()?,
            force: r.vec2()?,
            well: r.opt(|r| Ok(GravityWell { at: r.vec2()?, pull: r.f32()?, near: r.f32()? }))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_force_survives_the_wire() {
        let well = GravityWell { at: [900.0, 1200.0], pull: 2400.0, near: 400.0 };
        for well in [None, Some(well)] {
            let force = HeldForce { slot: 2, min: [0.0, -10.0], max: [1800.0, 1280.0], lift: 0.8, force: [600.0, 0.0], well };
            let mut out = Vec::new();
            HeldForcePort::encode(&force, &mut out);
            let mut r = WireReader::new(&out);
            assert_eq!(HeldForcePort::decode(&mut r).unwrap(), force);
            r.finish().unwrap();
        }
    }

    /// A well pulls toward its point: its full pull near it, and less as
    /// `1 / r` past `near`.
    #[test]
    fn a_well_pulls_toward_its_point_and_less_far_from_it() {
        let well = GravityWell { at: [0.0, 0.0], pull: 2000.0, near: 100.0 };
        assert_eq!(well.acceleration([-50.0, 0.0]), [2000.0, 0.0]);
        let far = well.acceleration([0.0, -400.0]);
        assert!(far[0].abs() < 1e-3 && (far[1] - 500.0).abs() < 1e-3, "{far:?}");
        assert_eq!(well.acceleration([0.0, 0.0]), [0.0, 0.0]);
    }
}
