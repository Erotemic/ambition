//! A strike that leaves a delayed mark on the body it hits: the authored vocabulary.
//!
//! ⭐ THE SAME SPLIT `smash_mine`, `smash_bomb` AND `smash_capture` USE. A key
//! and its params are what a MOVESET authors; the mark riding on the victim, its
//! clock and its detonation are a RULESET's, and that half is
//! `ambition_demo_smash::mark`.
//!
//! ⭐⭐ AND IT NEEDED NO NEW ENGINE AUTHORITY, which is the sixth time this
//! campaign has priced a rung and found it already built. Everything the mark
//! coordinates ships today:
//!
//! | the mark needs | what already answers it |
//! |---|---|
//! | to learn WHO was struck | `OnHitEffectMessage` carries `owner`, `victim`, `volume`, `contact` |
//! | an authored, keyed payload | `HitVolume::on_hit: Option<EffectRef>`, hydrated by the ruleset |
//! | a blast at a place | `vfx::Effect::DamageBox`, the mine's own detonation |
//! | per-body state that rewinds | a ruleset component, rollback-registered — `PlacedMine`'s precedent |
//!
//! ⇒ The whole capability is a key, four numbers, and a ruleset that spends them.
//!
//! ⛔ THE MARK IS ON THE VICTIM, NOT ON A PLACE, and that is the entire
//! difference from the mine. A mine asks the opponent to avoid a SPOT; a mark
//! follows them, so the pressure is on the clock rather than on the floor. A
//! player who has been marked has to change what they do, not where they stand —
//! which is what makes it worth authoring in a 1v1 match at all.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const MARK_BODY: &str = "smash.mark_body";

/// Authored parameters of one delayed mark.
///
/// ⛔ THERE IS NO `stacks` FIELD AND ITS ABSENCE IS THE DESIGN. A second mark on
/// an already-marked body REFRESHES the clock rather than adding a second
/// detonation: stacking turns one read ("how long have I got") into arithmetic,
/// and the move is selling the read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkBodyParams {
    /// Seconds from landing the hit to the mark going off.
    ///
    /// ⚠ LONG ENOUGH TO BE A DECISION. A fuse under roughly half a second is a
    /// delayed hit with extra steps — the victim cannot act on it, so it reads as
    /// damage arriving late rather than as pressure.
    pub fuse_s: f32,
    /// Damage of the detonation.
    pub damage: i32,
    /// Half-extent of the blast, in world px, centred on the marked body.
    pub blast_radius: f32,
    /// Knockback FEEL MULTIPLIER, not a launch speed — values such as 1.0 or 1.6.
    /// The mine's own comment records the units error this field invites.
    pub knockback: f32,
}

/// Stamp the mark onto every damaging volume of `spec`.
///
/// ⛔ EVERY VOLUME, DELIBERATELY, and the alternative is worse than it looks. A
/// move with an early weak window and a late strong one would otherwise mark on
/// only one of them, and which one is an invisible property of the volume order.
/// A move that wants a mark on one window authors that window as its own move.
///
/// ⚠ AND IT ASSERTS THE MOVE CAN ACTUALLY HIT, for the same reason
/// `author_place_mine` asserts its event fits inside the timeline: a mark
/// authored on a move with no damaging volume is a technique that can never fire,
/// and nothing downstream would ever say so.
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


/// Stamp the mark onto one move of an already-lowered contract, by id.
///
/// ⭐ THE VERB A FIGHTER ACTUALLY WANTS. `author_mark_on_hit` takes a `MoveSpec`,
/// which suits a special the fighter builds itself; a marking TILT or JAB is a
/// move the archetype already lowered, and re-authoring it from scratch to add
/// one payload would fork a shape somebody else maintains.
///
/// ⛔ AND IT ASSERTS THE MOVE EXISTS, because the id is a `String` and a typo is
/// otherwise a silent no-op — the fighter ships with a technique that never
/// happens and every test still passes. The same trap `when_refused` carries.
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
