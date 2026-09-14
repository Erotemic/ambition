//! Authored payload for placing a linked portal pair as a recovery.
//! Portal linking and transit remain owned by `ambition_portal2d`; this module only supplies placement, lifetime, and orientation parameters.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const PORTAL_PAIR: &str = "smash.portal_pair";

/// Authored parameters of one placed portal pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortalPairParams {
    /// Vertical separation between entry and exit apertures, in world pixels. This is relative movement, not a stage-specific destination.
    pub rise: f32,
    /// Half-extent of each aperture. Wide enough to fall into without aiming,
    /// or the recovery is a precision test rather than a route.
    pub half_extent: (f32, f32),
    /// Seconds both apertures stay open. `0` or less is refused at authoring.
    pub lifetime_s: f32,
    /// If true, close the pair after the first transit. If false, keep it open until `lifetime_s` expires.
    pub close_on_transit: bool,
    /// Authored tilt from vertical, in degrees.
    #[serde(default)]
    pub tilt_degrees: f32,
    /// Maximum player-controlled tilt around `tilt_degrees`, in degrees. `0.0` disables aiming.
    #[serde(default)]
    pub aim_tilt_degrees: f32,
    /// Indexed portal channel. Values below 8 are reserved for named room channels and are rejected.
    pub channel_index: u8,
}

/// Author a portal pair onto `spec`, opening at `at_s`.
pub fn author_portal_pair(mut spec: MoveSpec, at_s: f32, params: PortalPairParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` opens a portal pair at {at_s}s but only lasts {}s, so the \
         apertures would never appear and the move would spend a recovery to do \
         nothing",
        spec.id,
        spec.duration_s,
    );
    assert!(
        params.rise > 0.0,
        "move `{}` opens its exit {}px above the fighter, which is at or below \
         where they already are — a recovery that goes nowhere",
        spec.id,
        params.rise,
    );
    assert!(
        params.lifetime_s > 0.0,
        "move `{}` opens apertures for {}s, so they close on the frame they \
         open and nothing can ever transit them",
        spec.id,
        params.lifetime_s,
    );
    assert!(
        params.half_extent.0 > 0.0 && params.half_extent.1 > 0.0,
        "move `{}` opens a portal with a non-positive aperture {:?}, which \
         nothing can fall into",
        spec.id,
        params.half_extent,
    );
    assert!(
        params.channel_index >= 8,
        "move `{}` takes channel pair {}, and indices 0..=7 overlap the eight \
         NAMED authored pairs — a room that authored that colour would find its \
         portals fighting this move. Use 8 or above.",
        spec.id,
        params.channel_index,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: PORTAL_PAIR.to_string(),
            params: ParamValue::from_typed(&params).expect("portal pair params serialize"),
        }),
    });
    spec
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> MoveSpec {
        ron::from_str(
            r#"(id: "test_portal_up_b", clip: (clip: "special", fallbacks: ["idle"]), duration_s: 0.5, windows: [], events: [])"#,
        )
        .expect("the fixture move parses")
    }

    fn params() -> PortalPairParams {
        PortalPairParams {
            rise: 320.0,
            half_extent: (26.0, 6.0),
            lifetime_s: 2.5,
            close_on_transit: false,
            tilt_degrees: 0.0,
            aim_tilt_degrees: 0.0,
            channel_index: 8,
        }
    }

    /// The authored pair round-trips through `ParamValue`.
    #[test]
    fn portal_params_survive_the_round_trip() {
        let carried = ParamValue::from_typed(&params()).expect("serialize");
        let back: PortalPairParams = carried.hydrate().expect("hydrate");
        assert_eq!(back, params());
    }

    /// A pair on a named room channel is refused because unrelated portals could link.
    #[test]
    fn a_named_channel_index_is_refused() {
        let refused = std::panic::catch_unwind(|| {
            author_portal_pair(
                shell(),
                0.1,
                PortalPairParams {
                    channel_index: 2,
                    ..params()
                },
            )
        });
        assert!(
            refused.is_err(),
            "a move took one of the eight NAMED channel pairs, so a room that \
             authored that colour would find its portals linked to a fighter's"
        );
    }

    /// Apertures that close on the frame they open are refused.
    #[test]
    fn a_pair_that_closes_immediately_is_refused() {
        let refused = std::panic::catch_unwind(|| {
            author_portal_pair(
                shell(),
                0.1,
                PortalPairParams {
                    lifetime_s: 0.0,
                    ..params()
                },
            )
        });
        assert!(
            refused.is_err(),
            "a portal pair with no lifetime was accepted, so the move plays its \
             whole animation and nothing can ever transit"
        );
    }

    /// A recovery that rises nowhere is refused.
    #[test]
    fn a_rise_of_nothing_is_refused() {
        let refused = std::panic::catch_unwind(|| {
            author_portal_pair(shell(), 0.1, PortalPairParams { rise: 0.0, ..params() })
        });
        assert!(
            refused.is_err(),
            "a portal recovery whose exit is level with its entrance was \
             accepted — falling in returns you where you started"
        );
    }
}
