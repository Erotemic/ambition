//! Authored payload for a remotely triggered mine.
//! The ruleset represents the mine as a ground item. Normal item custody therefore applies while the placer keeps detonation authority.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const PLACE_MINE: &str = "smash.place_mine";

/// Authored parameters for one placed mine. The mine has no fuse; its owner triggers it after the arming delay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceMineParams {
    /// Held-item id used when the mine enters item custody. It must name a registered held item.
    pub item_id: String,
    /// Seconds before the mine accepts remote detonation.
    pub arm_s: f32,
    /// Damage at the centre of the blast.
    pub damage: i32,
    /// How far the blast reaches, in world px.
    pub blast_radius: f32,
    /// The object's own size in the world.
    pub half_extents: (f32, f32),
    /// Where it appears, body-local (`+x` toward facing, `+y` gravity-down).
    pub offset: (f32, f32),
}

/// Return the held-item id referenced by a mine payload.
/// Malformed params return no references because parameter hydration is validated separately.
pub fn mine_held_item_refs(effect: &crate::EffectRef) -> Vec<String> {
    effect
        .params
        .hydrate::<PlaceMineParams>()
        .map(|params| vec![params.item_id])
        .unwrap_or_default()
}

/// Author the mine technique on a move timeline. The ruleset decides whether the event places a mine or detonates the owner's armed mine.
///
/// # Panics
///
/// Panics if `at_s` is after the move duration.
pub fn author_place_mine(mut spec: MoveSpec, at_s: f32, params: PlaceMineParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` places its mine at {at_s}s but only lasts {}s, so the mine \
         would never appear and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: PLACE_MINE.to_string(),
            params: ParamValue::from_typed(&params).expect("place-mine params serialize"),
        }),
    });
    spec
}
