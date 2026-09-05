//! The portal recovery: the authored vocabulary for "open a way up".
//!
//! ⭐⭐ JON'S MOVE, AND DELIBERATELY NOT A GENRE ONE, 2026-09-05: *"up b opens a
//! portal under him, and a portal at the very top of the stage, and when he falls
//! into it he comes out the higher portal … it's a portal so just use the portal
//! crate rules, we can even exercise angled portals with directional input on the
//! up b as a flavor that isn't actually in smash and is ours."*
//!
//! ⛔⛔ IT ADDS NO RECOVERY BEHAVIOUR, AND THAT IS THE WHOLE CLAIM.
//! `ambition_portal2d` already owns apertures, linking and transit;
//! `PlacedPortal` is a Component, so an aperture is a spawn, and
//! `PortalChannelColor::Indexed(n)` documents its own pairing — *"even = slot A,
//! odd = slot B; the partner is `Indexed(n ^ 1)`"*. A move that placed its own
//! transit rules would be a second portal implementation wearing a fighter's
//! name.
//!
//! ⭐ THE LIFETIME IS AUTHORED RATHER THAN DECIDED. Closing on move end, on the
//! first transit, or on a timer are three different MECHANICS — a recovery, a
//! one-shot escape, and a hole in the stage another fighter can use — and which
//! one this move is belongs to whoever authors it. Two fields express all three,
//! so the design question stays open in the data instead of being closed
//! silently by whichever was easiest to implement.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const PORTAL_PAIR: &str = "smash.portal_pair";

/// Authored parameters of one placed portal pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortalPairParams {
    /// How far ABOVE the fighter the exit aperture opens, in world px.
    ///
    /// ⛔ A RISE, NOT A DESTINATION. "The very top of the stage" is a fact about
    /// a stage, and a move that read one would be authored against a single
    /// room; a rise is a property of the MOVE and travels with the fighter to
    /// every stage. What "the top" means is then the stage's business, through
    /// the ordinary blast bounds.
    pub rise: f32,
    /// Half-extent of each aperture. Wide enough to fall into without aiming,
    /// or the recovery is a precision test rather than a route.
    pub half_extent: (f32, f32),
    /// Seconds both apertures stay open. `0` or less is refused at authoring.
    pub lifetime_s: f32,
    /// Close the pair the first time anything transits it.
    ///
    /// ⇒ `true` makes this a ONE-SHOT ESCAPE — the recovery closes behind you,
    /// which is what stops an opponent following. `false` leaves a route open
    /// for [`Self::lifetime_s`], which any fighter may use, and is the version
    /// that changes the stage rather than the fighter.
    pub close_on_transit: bool,
    /// Tilt of the pair from vertical, in DEGREES, for the angled variant.
    ///
    /// ⭐ NOT IN THE GENRE, and Jon's whole reason for the move. `0.0` is the
    /// straight version: enter downward, leave upward. Author the straight one
    /// first and let the angle be a second commit — it is also the cheapest
    /// possible test of whether the placement seam takes an orientation at all.
    #[serde(default)]
    pub tilt_degrees: f32,
    /// Which indexed channel pair to use. The partner is `index ^ 1`.
    ///
    /// ⛔ INDICES `8..` ONLY, which the colour table asks for in place: `0..=7`
    /// overlap the eight NAMED pairs in index space, and a move that quietly
    /// took one would fight whatever authored that colour in the room.
    pub channel_index: u16,
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

    /// A pair on a NAMED channel index is refused at authoring.
    ///
    /// ⛔ THE FAILURE IS INVISIBLE AT RUNTIME AND ROOM-DEPENDENT. Taking index
    /// `2` works perfectly until somebody plays the one stage that authored that
    /// colour, and then two unrelated portals link to each other — which reads as
    /// the recovery teleporting you somewhere absurd rather than as a channel
    /// collision.
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
