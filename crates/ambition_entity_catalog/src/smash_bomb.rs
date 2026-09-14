//! Authored payload for dropping a bomb.
//! The ruleset creates a ground item, so normal item custody and throwing apply to the bomb.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const DROP_BOMB: &str = "smash.drop_bomb";

/// Authored parameters of one dropped bomb.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DropBombParams {
    /// The held-item id this object becomes in somebody's hands. It must be a
    /// registered held item or nobody can pick the bomb up — which is half the
    /// move.
    pub item_id: String,
    /// Seconds until it goes off by itself.
    pub fuse_s: f32,
    /// Damage at the centre of the blast.
    pub damage: i32,
    /// How far the blast reaches, in world px.
    pub blast_radius: f32,
    /// Minimum contact speed that detonates the bomb. Slower impacts bounce and keep the fuse.
    pub impact_speed: f32,
    /// The object's own size in the world.
    pub half_extents: (f32, f32),
    /// Where it appears, body-local (`+x` toward facing, `+y` gravity-down).
    pub offset: (f32, f32),
}

/// Return the held-item id referenced by a bomb payload.
/// Malformed params return no references because parameter hydration is validated separately.
pub fn bomb_held_item_refs(effect: &crate::EffectRef) -> Vec<String> {
    effect
        .params
        .hydrate::<DropBombParams>()
        .map(|params| vec![params.item_id])
        .unwrap_or_default()
}

/// Author a bomb drop on a move timeline.
///
/// # Panics
///
/// Panics if `at_s` is after the move duration.
pub fn author_drop_bomb(mut spec: MoveSpec, at_s: f32, params: DropBombParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` drops its bomb at {at_s}s but only lasts {}s, so the bomb \
         would never appear and the move would spend a recovery to do nothing",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: DROP_BOMB.to_string(),
            params: ParamValue::from_typed(&params).expect("drop-bomb params serialize"),
        }),
    });
    spec
}
