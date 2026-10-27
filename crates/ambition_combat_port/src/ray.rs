//! A ray: a strike along a line. Lightning, a laser, any move that hurts
//! along a line is one.
//!
//! The module says where the line starts, which way it goes, how far it may
//! reach, how wide it is and how it looks. The engine casts it: it stops at
//! the first wall and goes on through each portal pair it enters, so its
//! length is its length unfolded. It hurts as one volume from where it
//! starts, and the portal pairs cut that volume as they cut every other
//! (`PortalPassages::pieces`): what reaches through a pair is out of its
//! other end. Its look is a name the presentation resolves, drawn from the
//! ray's published geometry; the module never draws it.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// Where something a module asks for is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spot {
    /// A world point, +Y down.
    World([f32; 2]),
    /// An offset from the body the invocation ran for: it follows the body.
    Body([f32; 2]),
}

impl Spot {
    pub fn put(self, out: &mut Vec<u8>) {
        match self {
            Spot::World(at) => {
                wire::put_u8(out, 0);
                wire::put_vec2(out, at);
            }
            Spot::Body(offset) => {
                wire::put_u8(out, 1);
                wire::put_vec2(out, offset);
            }
        }
    }

    pub fn read(r: &mut WireReader<'_>) -> Result<Spot, WireError> {
        match r.u8()? {
            0 => Ok(Spot::World(r.vec2()?)),
            1 => Ok(Spot::Body(r.vec2()?)),
            tag => Err(WireError::BadTag(tag)),
        }
    }

    /// The world point, for a body at `body`.
    pub fn at(self, body: [f32; 2]) -> [f32; 2] {
        match self {
            Spot::World(at) => at,
            Spot::Body([x, y]) => [body[0] + x, body[1] + y],
        }
    }
}

/// The request port marker for a held ray.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — hold a ray from `from` along `toward` (any length; it
///   is the direction), as far as `reach` or the first wall, `width` wide,
///   through the portal pairs it enters. It hurts what it meets for
///   `damage`, launching by `knockback` (a multiplier over the victim's
///   feel-tuned launch). `launch_dir` selects the axes; absence launches away
///   from the source. It is drawn as `look`.
/// * **Owner** — `ambition_combat::extension`, with the held damage boxes:
///   the same slots, generations and lifetimes. Held and asked again, it is
///   cast again: it follows its `from` and `toward`.
/// * **Scope and grant** — the body the invocation ran for owns it, on its
///   effective side; a body with no faction has its rays refused.
/// * **Time** — offered in `technique_execution` and `boss_conduct`.
/// * **Read model** — world units, +Y down.
/// * **Replay** — as the held damage boxes: the adapter's record is rollback
///   state on the body, and the ray is a hitbox.
pub struct HeldRayPort;

#[derive(Clone, Debug, PartialEq)]
pub struct HeldRay {
    /// The module's name for the ray. Slots are per entry, shared with the
    /// held damage boxes.
    pub slot: u32,
    /// A new generation in a slot replaces the ray.
    pub generation: u32,
    pub from: Spot,
    pub toward: [f32; 2],
    pub reach: f32,
    pub width: f32,
    pub damage: i32,
    pub knockback: f32,
    pub launch_dir: Option<crate::LaunchDirection>,
    pub lifetime_s: f32,
    /// How it is drawn: a name of the presentation's ray looks (`"strike"`,
    /// `"force"`, `"laser"`).
    pub look: String,
}

impl Port for HeldRayPort {
    const KEY: PortKey = PortKey::new("ambition.combat.held_ray", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = HeldRay;

    fn encode(v: &HeldRay, out: &mut Vec<u8>) {
        wire::put_u32(out, v.slot);
        wire::put_u32(out, v.generation);
        v.from.put(out);
        wire::put_vec2(out, v.toward);
        wire::put_f32(out, v.reach);
        wire::put_f32(out, v.width);
        wire::put_i32(out, v.damage);
        wire::put_f32(out, v.knockback);
        wire::put_opt(out, v.launch_dir, |out, direction| direction.put(out));
        wire::put_f32(out, v.lifetime_s);
        wire::put_str(out, &v.look);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<HeldRay, WireError> {
        Ok(HeldRay {
            slot: r.u32()?,
            generation: r.u32()?,
            from: Spot::read(r)?,
            toward: r.vec2()?,
            reach: r.f32()?,
            width: r.f32()?,
            damage: r.i32()?,
            knockback: r.f32()?,
            launch_dir: r.opt(crate::LaunchDirection::read)?,
            lifetime_s: r.f32()?,
            look: r.str()?.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ray_round_trips() {
        let ray = HeldRay {
            slot: 5,
            generation: 2,
            from: Spot::Body([3.0, -27.0]),
            toward: [0.6, -0.8],
            reach: 460.0,
            width: 30.0,
            damage: 1,
            knockback: 1.3,
            launch_dir: Some(crate::LaunchDirection::World([-1.0, -0.4])),
            lifetime_s: 0.3,
            look: "force".into(),
        };
        let mut out = Vec::new();
        HeldRayPort::encode(&ray, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(HeldRayPort::decode(&mut r).unwrap(), ray);
        r.finish().unwrap();
    }

    #[test]
    fn a_spot_is_a_world_point_or_rides_its_body() {
        assert_eq!(Spot::World([5.0, 6.0]).at([100.0, 100.0]), [5.0, 6.0]);
        assert_eq!(Spot::Body([5.0, 6.0]).at([100.0, 100.0]), [105.0, 106.0]);
    }
}
