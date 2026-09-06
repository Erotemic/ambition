//! Putting a body to sleep: the authored vocabulary.
//!
//! ⭐⭐ THE MECHANIC WAS ALREADY THERE AND HAD NO NAME. `attack_support`'s
//! `hard_lock_timer` is a `max()` over NAMED causes of "this body cannot act" —
//! the knockback and landing locks a body owns, the dizzy a broken guard owes,
//! the shieldstun a blocked hit charges, shield-drop lag. A sleep is a fifth
//! cause, and `BodyCombat::sleep_timer` is where it lives.
//!
//! ⛔ SO THIS TECHNIQUE ADDS NO STATUS SYSTEM. It sets one timer that an
//! existing gate already reads, which is the difference between a move that
//! coordinates an authority and a move that becomes one.
//!
//! ⚠ IT IS A DISABLE, NOT YET A SLEEP, and the field says so too: no specific
//! pose and no mash escape. Both are what make a sleep richer than a disable and
//! neither is expressible as a timer in a `max` — so this is the honest half,
//! and the other half wants its own decision rather than a quiet extension.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key.
pub const SLEEP: &str = "smash.sleep";

/// Authored parameters of one sleep.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SleepParams {
    /// How long a caught body cannot act, in seconds.
    pub duration_s: f32,
    /// How far the sleep reaches from the singer, as a half-extent.
    ///
    /// ⭐ AN AREA, NOT A STRIKE. The genre's version catches everyone nearby
    /// rather than whoever is in front, and that is the whole shape of the move:
    /// it is a punish on a crowd and a suicide against one spaced opponent.
    pub half_extents: (f32, f32),
}

/// Author a sleep pulse onto `spec`, firing at `at_s`.
pub fn author_sleep(mut spec: MoveSpec, at_s: f32, params: SleepParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` sings at {at_s}s but only lasts {}s, so the pulse never fires",
        spec.id,
        spec.duration_s,
    );
    assert!(
        params.duration_s > 0.0,
        "move `{}` puts bodies to sleep for {}s, which is not a status at all",
        spec.id,
        params.duration_s,
    );
    assert!(
        params.half_extents.0 > 0.0 && params.half_extents.1 > 0.0,
        "move `{}` sings into a {:?} area, which reaches nobody",
        spec.id,
        params.half_extents,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: SLEEP.to_string(),
            params: ParamValue::from_typed(&params).expect("sleep params serialize"),
        }),
    });
    spec
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> MoveSpec {
        crate::moveset_authoring::hitless_special("test_sing", "special", 0.2, 0.8)
    }

    fn params() -> SleepParams {
        SleepParams {
            duration_s: 1.4,
            half_extents: (70.0, 40.0),
        }
    }

    #[test]
    fn sleep_params_survive_the_round_trip() {
        let carried = ParamValue::from_typed(&params()).expect("serialize");
        let back: SleepParams = carried.hydrate().expect("hydrate");
        assert_eq!(back, params());
    }

    /// A sleep of no duration is refused where it is authored.
    ///
    /// ⛔ AT RUNTIME IT IS INVISIBLE: the move plays its whole animation, the
    /// pulse fires, every caught body is put to sleep for zero seconds, and the
    /// result is indistinguishable from a move that missed.
    #[test]
    fn a_sleep_of_no_duration_is_refused() {
        let refused = std::panic::catch_unwind(|| {
            author_sleep(
                shell(),
                0.2,
                SleepParams {
                    duration_s: 0.0,
                    ..params()
                },
            )
        });
        assert!(refused.is_err(), "a zero-duration sleep was accepted");
    }

    /// A sleep that reaches nowhere is refused.
    #[test]
    fn a_sleep_with_no_area_is_refused() {
        let refused = std::panic::catch_unwind(|| {
            author_sleep(
                shell(),
                0.2,
                SleepParams {
                    half_extents: (0.0, 40.0),
                    ..params()
                },
            )
        });
        assert!(refused.is_err(), "a sleep with no reach was accepted");
    }
}
