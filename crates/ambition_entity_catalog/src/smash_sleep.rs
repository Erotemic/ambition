//! Authored payload for a temporary action lock presented as sleep.
//! The ruleset writes the existing combat sleep timer; this module does not define a separate status system.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue};

/// The authored effect key.
pub const SLEEP: &str = "smash.sleep";

/// Authored parameters of one sleep.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SleepParams {
    /// How long a caught body cannot act, in seconds.
    pub duration_s: f32,
    /// Half-extents of the sleep area around the singer.
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
        crate::authoring::hitless_special("test_sing", "special", 0.2, 0.8)
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

    /// A zero-duration sleep is refused because it has no mechanical effect.
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
