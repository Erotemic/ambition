//! The trigger port of a technique a body's action scheme puts on a control
//! slot: what pressing that slot does, decided by a module.
//!
//! A body's scheme can route a slot to a technique (`ActionGate::Technique`):
//! the body's own technique, the rules' technique for the driven body, or the
//! technique of a row it wears. An ability variant equipped from the inventory
//! is such a row. When a module binds the technique's id, the module decides
//! what the slot does.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

use crate::wielded::{WieldedUsePort, Wielder};

/// The trigger port marker for a body's technique.
///
/// Port card:
///
/// * **Operation** — a body's action scheme routes a control slot to the
///   technique whose id is the selector. The value is the slot's edge this
///   tick and the body's facts as a [`Wielder`]. `user.pressed` is the press
///   of the technique's slot, not of Attack.
/// * **Owner** — `ambition_abilities::extension`, which reads
///   `ResolvedTechniqueEdges`.
/// * **Scope** — one invocation for EACH body whose scheme carries a bound
///   technique, EACH tick the phase runs. An invocation is IDLE when the slot
///   is not pressed, held or released.
/// * **Time** — `technique_execution`; every value is this tick's settled
///   value.
/// * **Replay** — derived each tick from rollback state.
pub struct TechniqueUsePort;

/// What a technique module reads.
#[derive(Clone, Debug, PartialEq)]
pub struct TechniqueUse {
    /// The slot is down this tick.
    pub held: bool,
    /// The slot went up this tick.
    pub released: bool,
    pub user: Wielder,
}

impl TechniqueUse {
    /// The slot was pressed, held or released this tick.
    pub fn is_active(&self) -> bool {
        self.user.pressed || self.held || self.released
    }
}

impl Port for TechniqueUsePort {
    const KEY: PortKey = PortKey::new("ambition.body.technique", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = TechniqueUse;

    fn encode(v: &TechniqueUse, out: &mut Vec<u8>) {
        wire::put_bool(out, v.held);
        wire::put_bool(out, v.released);
        WieldedUsePort::encode(&v.user, out);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<TechniqueUse, WireError> {
        Ok(TechniqueUse {
            held: r.bool()?,
            released: r.bool()?,
            user: WieldedUsePort::decode(r)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_technique_use_round_trips() {
        let used = TechniqueUse {
            held: true,
            released: false,
            user: Wielder {
                pressed: false,
                driven: true,
                position: [10.0, -4.0],
                size: [28.0, 32.0],
                facing: -1.0,
                frame_side: [1.0, 0.0],
                frame_down: [0.0, 1.0],
                aim_local: [0.0, 0.0],
                mana: Some(40.0),
                names_spawns: true,
                cooldown_ready: true,
                swept: true,
            },
        };
        let mut out = Vec::new();
        TechniqueUsePort::encode(&used, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(TechniqueUsePort::decode(&mut r).unwrap(), used);
        r.finish().unwrap();
        assert!(used.is_active());
    }
}
