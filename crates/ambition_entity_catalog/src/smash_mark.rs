//! Authored payload for a delayed mark applied by a damaging hit.
//! The mark follows the victim. The ruleset owns its timer and detonation.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const MARK_BODY: &str = "smash.mark_body";

/// Authored parameters for one delayed mark. A new mark refreshes the existing mark rather than stacking another detonation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkBodyParams {
    /// Seconds from the hit to detonation.
    pub fuse_s: f32,
    /// Damage of the detonation.
    pub damage: i32,
    /// Half-extent of the blast, in world px, centred on the marked body.
    pub blast_radius: f32,
    /// Knockback feel multiplier, not a launch speed: values such as 1.0 or 1.6.
    /// The mine's own comment records the units error this field invites.
    pub knockback: f32,
}

/// Attach the mark payload to every damaging volume of a move.
///
/// # Panics
///
/// Panics if the move has no damaging volume that can carry the mark.
pub fn author_mark_on_hit(mut spec: MoveSpec, params: MarkBodyParams) -> MoveSpec {
    let payload = EffectRef {
        key: MARK_BODY.to_string(),
        params: ParamValue::from_typed(&params).expect("mark-body params serialize"),
    };
    let mut stamped = 0usize;
    for window in &mut spec.windows {
        for volume in &mut window.volumes {
            if volume.damage > 0 {
                volume.on_hit = Some(payload.clone());
                stamped += 1;
            }
        }
    }
    assert!(
        stamped > 0,
        "move `{}` authors a delayed mark and has no damaging volume to carry \
         it, so the mark can never be applied and the move would spend its \
         recovery to do nothing",
        spec.id,
    );
    spec
}


/// Attach a mark to one move in an already-lowered contract.
///
/// # Panics
///
/// Panics if the named move is absent.
pub fn mark_move_in(
    contract: &mut crate::MovesetContract,
    id: &str,
    params: MarkBodyParams,
) {
    let found = contract.moves.iter().position(|m| m.id == id).unwrap_or_else(|| {
        panic!(
            "no move `{id}` to carry a delayed mark; the ids this contract holds \
             are: {:?}",
            contract.moves.iter().map(|m| m.id.as_str()).collect::<Vec<_>>()
        )
    });
    let spec = contract.moves.remove(found);
    contract.moves.insert(found, author_mark_on_hit(spec, params));
}
