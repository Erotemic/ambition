//! What a body may aim at: the facts a module's targeting reads.
//!
//! The engine answers who is a foe (the targeting relation, which no module
//! should restate) and leaves the aiming to the module: a cone, a nearest, a
//! lock-on are each a move's own rule over [`Foes`].

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The observation port marker for a body's foes.
///
/// Port card:
///
/// * **Operation** — the foes of the body the invocation ran for: every
///   other body of that body's live room that it may target and can hit.
///   "Target" is the targeting relation, not the damage one: a match team
///   outranks a faction, so a teammate is not a foe and an opponent is; a
///   neutral bystander is damageable and still not a foe; friendly fire does
///   not take part. "Can hit" is in play, alive and on the playable plane.
/// * **Owner** — `ambition_combat::extension::targets`.
/// * **Scope** — the body the invocation ran for.
/// * **Time** — offered in `technique_execution`; read when the entry runs.
/// * **Read model** — world units, +Y down; each foe's `position` is its
///   centre. Foes are in the order of their simulation identity (a body with
///   none after every body with one, by entity), so a module that keeps the
///   first of equals breaks a tie the same way on every peer and across a
///   rewind.
/// * **Absence** — an empty list when there is no foe; absent for a scope
///   that is not a body.
pub struct FoesPort;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Foes {
    pub foes: Vec<Foe>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Foe {
    pub position: [f32; 2],
}

impl Port for FoesPort {
    const KEY: PortKey = PortKey::new("ambition.combat.foes", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = Foes;

    fn encode(v: &Foes, out: &mut Vec<u8>) {
        wire::put_u32(out, v.foes.len() as u32);
        for foe in &v.foes {
            wire::put_f32(out, foe.position[0]);
            wire::put_f32(out, foe.position[1]);
        }
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Foes, WireError> {
        let count = r.u32()?;
        let foes = (0..count)
            .map(|_| Ok(Foe { position: [r.f32()?, r.f32()?] }))
            .collect::<Result<_, WireError>>()?;
        Ok(Foes { foes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Foes decode to what was encoded, in order.
    #[test]
    fn foes_cross_the_wire_unchanged() {
        let foes = Foes { foes: vec![Foe { position: [1.0, -2.0] }, Foe { position: [30.5, 4.0] }] };
        let mut bytes = Vec::new();
        FoesPort::encode(&foes, &mut bytes);
        assert_eq!(FoesPort::decode(&mut WireReader::new(&bytes)).unwrap(), foes);
    }
}
