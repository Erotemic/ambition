//! The ports of a contact rule: what a touch on a body does, decided by a
//! module in `body_contact` (`ambition_extension_sdk::phases::BODY_CONTACT`).
//!
//! A stomp that bounces the toucher and defeats the body, a roll that defeats
//! it, a shell that a touch kicks: each is a rule of its game about a touch,
//! and it runs before the shared contact damage, so a body that a rule
//! defeated hurts nobody in that tick.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The trigger port marker for a contact rule.
///
/// Port card:
///
/// * **Operation** — a live body whose brain is the bound key is touched by
///   one or more player bodies of its own live room this tick. The selector
///   is the `CharacterBrain::Custom` key the placement authors. A body a
///   player drives does not have a contact rule.
/// * **Owner** — `ambition_combat::extension::contact`.
/// * **Time** — `body_contact`: after the bodies integrated, and before the
///   shared contact damage. One invocation for each touched body, in the
///   order of the body's simulation identity.
/// * **Touch** — the two boxes overlap side to side, and top to bottom they
///   overlap or meet: a body that stands on the other's head touches it.
/// * **Read model** — world units, +Y down; each box and velocity as it is
///   at the start of the phase. A box is the one the body collides with
///   under its own DOWN: a body in turned gravity has a turned box. The touchers are in the order of their
///   simulation identity, and a request names a toucher by its index.
pub struct BodyContactPort;

/// A body in a contact: where its box is, and how it moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactBody {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub velocity: [f32; 2],
}

/// What a contact rule reads: the touched body and each body that touches it.
#[derive(Clone, Debug, PartialEq)]
pub struct BodyContact {
    pub body: ContactBody,
    pub touchers: Vec<ContactBody>,
}

fn put_body(out: &mut Vec<u8>, body: &ContactBody) {
    wire::put_vec2(out, body.position);
    wire::put_vec2(out, body.size);
    wire::put_vec2(out, body.velocity);
}

fn body(r: &mut WireReader<'_>) -> Result<ContactBody, WireError> {
    Ok(ContactBody { position: r.vec2()?, size: r.vec2()?, velocity: r.vec2()? })
}

impl Port for BodyContactPort {
    const KEY: PortKey = PortKey::new("ambition.body.contact", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = BodyContact;

    fn encode(v: &BodyContact, out: &mut Vec<u8>) {
        put_body(out, &v.body);
        wire::put_u32(out, v.touchers.len() as u32);
        for toucher in &v.touchers {
            put_body(out, toucher);
        }
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BodyContact, WireError> {
        let touched = body(r)?;
        let count = r.read_len()?;
        let touchers = (0..count).map(|_| body(r)).collect::<Result<_, _>>()?;
        Ok(BodyContact { body: touched, touchers })
    }
}

/// The request port marker for a toucher that bounces off the body.
///
/// Port card:
///
/// * **Operation** — the toucher `toucher` (an index into this invocation's
///   `touchers`) jumps up off the body at `speed`, against its gravity. With
///   `pogo_cue`, the toucher's pogo cue plays at the body: a bounce off a
///   body is the pogo verb.
/// * **Owner** — `ambition_combat::extension::contact`.
/// * **Scope** — a toucher of this invocation's contact. An index past the
///   touchers is refused with a warning.
/// * **Time** — `body_contact`, this tick: before the shared contact damage.
pub struct ContactBouncePort;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactBounce {
    pub toucher: u32,
    pub speed: f32,
    pub pogo_cue: bool,
}

impl Port for ContactBouncePort {
    const KEY: PortKey = PortKey::new("ambition.body.contact_bounce", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = ContactBounce;

    fn encode(v: &ContactBounce, out: &mut Vec<u8>) {
        wire::put_u32(out, v.toucher);
        wire::put_f32(out, v.speed);
        wire::put_bool(out, v.pogo_cue);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<ContactBounce, WireError> {
        Ok(ContactBounce { toucher: r.u32()?, speed: r.f32()?, pogo_cue: r.bool()? })
    }
}

/// The request port marker for the touched body's defeat.
///
/// Port card:
///
/// * **Operation** — the body is defeated now: its health is zero before
///   the shared contact damage reads it, so it hurts nobody this tick, and
///   it is removed from the world. No drops, no score, no death sequence:
///   the squash of a walker.
/// * **Owner** — `ambition_combat::extension::contact`.
/// * **Scope** — the body the invocation ran for.
/// * **Time** — `body_contact`, this tick.
pub struct BodyDefeatPort;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyDefeat;

impl Port for BodyDefeatPort {
    const KEY: PortKey = PortKey::new("ambition.body.defeat", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = BodyDefeat;

    fn encode(_: &BodyDefeat, _: &mut Vec<u8>) {}

    fn decode(_: &mut WireReader<'_>) -> Result<BodyDefeat, WireError> {
        Ok(BodyDefeat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round<P: Port>(v: &P::Value) -> P::Value {
        let mut out = Vec::new();
        P::encode(v, &mut out);
        let mut r = WireReader::new(&out);
        let back = P::decode(&mut r).unwrap();
        r.finish().unwrap();
        back
    }

    #[test]
    fn the_contact_values_survive_the_wire() {
        let contact = BodyContact {
            body: ContactBody { position: [400.0, 900.0], size: [40.0, 30.0], velocity: [-60.0, 0.0] },
            touchers: vec![
                ContactBody { position: [395.0, 870.5], size: [24.0, 44.0], velocity: [10.0, 210.0] },
                ContactBody { position: [430.0, 899.0], size: [24.0, 44.0], velocity: [-300.0, 0.0] },
            ],
        };
        assert_eq!(round::<BodyContactPort>(&contact), contact);
        let bounce = ContactBounce { toucher: 1, speed: 460.0, pogo_cue: true };
        assert_eq!(round::<ContactBouncePort>(&bounce), bounce);
        assert_eq!(round::<BodyDefeatPort>(&BodyDefeat), BodyDefeat);
    }
}
