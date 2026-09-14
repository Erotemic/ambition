//! Authored payload for reeling a fighter toward a ledge.
//! The technique only delivers the fighter to an anchor. The normal ledge-grab authority decides whether a ledge is actually acquired.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key.
pub const TETHER_PULL: &str = "smash.tether_pull";

/// Authored parameters of one tether reel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TetherPullParams {
    /// Maximum ledge-search reach in world pixels.
    pub reach: f32,
    /// How fast the reel carries her, in world px per second.
    pub speed: f32,
    /// Maximum reel time in seconds. The reel normally ends when the fighter reaches the latched anchor.
    pub timeout_s: f32,
}

/// Author a tether reel.
///
/// # Panics
///
/// Panics if the event or parameters are invalid, including a speed and timeout that cannot cross the authored reach.
pub fn author_tether_pull(mut spec: MoveSpec, at_s: f32, params: TetherPullParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` throws a tether at {at_s}s but only lasts {}s",
        spec.id,
        spec.duration_s,
    );
    assert!(
        params.reach > 0.0,
        "move `{}` authors a tether with {}px of reach, so the line can never \
         bite anything",
        spec.id,
        params.reach,
    );
    assert!(
        params.speed > 0.0,
        "move `{}` authors a tether that reels at {}px/s, so it latches a ledge \
         and then never arrives",
        spec.id,
        params.speed,
    );
    assert!(
        params.timeout_s > 0.0,
        "move `{}` authors a {}s reel, which expires on the frame it starts",
        spec.id,
        params.timeout_s,
    );
    // The reel must be able to cross its full authored reach before timeout.
    assert!(
        params.speed * params.timeout_s >= params.reach,
        "move `{}` reels at {}px/s for {}s — {}px — but its line bites out to \
         {}px, so a tether thrown at full reach expires before it arrives",
        spec.id,
        params.speed,
        params.timeout_s,
        params.speed * params.timeout_s,
        params.reach,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: TETHER_PULL.to_string(),
            params: ParamValue::from_typed(&params).expect("tether-pull params serialize"),
        }),
    });
    spec
}
