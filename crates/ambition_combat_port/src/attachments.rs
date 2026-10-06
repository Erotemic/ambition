//! The named points of a body's rig: a jaw, a saddle, a grip, a muzzle, a
//! hand. The art package states each one on a joint of the body's rig
//! (`<target>_body_rig.ron`), the engine poses the rig each tick
//! (`ambition_combat::body_rig`), and a module reads where the point is.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The observation port marker for a body's attachment points.
///
/// Port card:
///
/// * **Operation** — where each named attachment point of the owner's body
///   rig is this tick.
/// * **Owner** — `ambition_combat::extension` (the body rig pose).
/// * **Scope** — the body the invocation ran for.
/// * **Time** — offered in `boss_conduct`. The pose is the one the body's
///   hurt parts have at the read cut of the phase.
/// * **Read model** — each point is an offset from the body's POSITION in
///   the body's local frame: +x the way it faces, +y toward its feet, world
///   units. That is the frame of a `BodyHold` offset and of a riding
///   hitbox's, so a module gives a point to those ports as it is. For a
///   world place, mirror x by the side the body faces and add its position.
/// * **Absence** — no value for a body with no rig, or whose rig is not
///   posed yet. A name the rig does not publish is not in the list. A module
///   does not keep a number in place of an absent point: the point is the
///   art's to state.
/// * **Replay** — derived each tick from rollback state.
pub struct BodyAttachmentsPort;

/// One named point.
#[derive(Clone, Debug, PartialEq)]
pub struct BodyAttachment {
    pub name: String,
    pub offset: [f32; 2],
}

/// The named points of one body's rig, in the rig's attachment order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BodyAttachments {
    pub points: Vec<BodyAttachment>,
}

impl BodyAttachments {
    /// The point `name`, when the rig publishes it.
    pub fn get(&self, name: &str) -> Option<[f32; 2]> {
        self.points.iter().find(|point| point.name == name).map(|point| point.offset)
    }
}

impl Port for BodyAttachmentsPort {
    const KEY: PortKey = PortKey::new("ambition.body.attachments", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = BodyAttachments;

    fn encode(v: &BodyAttachments, out: &mut Vec<u8>) {
        wire::put_u32(out, v.points.len() as u32);
        for point in &v.points {
            wire::put_str(out, &point.name);
            wire::put_vec2(out, point.offset);
        }
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BodyAttachments, WireError> {
        let count = r.u32()?;
        let mut points = Vec::new();
        for _ in 0..count {
            points.push(BodyAttachment {
                name: r.str()?.to_owned(),
                offset: r.vec2()?,
            });
        }
        Ok(BodyAttachments { points })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_attachment_points_survive_the_wire() {
        let v = BodyAttachments {
            points: vec![
                BodyAttachment { name: "jaw".into(), offset: [118.5, -52.25] },
                BodyAttachment { name: "saddle".into(), offset: [-4.0, -30.0] },
            ],
        };
        let mut out = Vec::new();
        BodyAttachmentsPort::encode(&v, &mut out);
        let back = BodyAttachmentsPort::decode(&mut WireReader::new(&out)).unwrap();
        assert_eq!(back, v);
        assert_eq!(back.get("saddle"), Some([-4.0, -30.0]));
        assert_eq!(back.get("grip"), None, "a name the rig does not publish is absent, not a default");
    }
}
