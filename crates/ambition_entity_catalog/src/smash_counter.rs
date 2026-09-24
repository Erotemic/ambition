//! Authored payload for a counter stance.
//! The active window reuses the existing parry authority. The response key selects the consequence, so the counter does not define a separate counter-kind hierarchy.

use serde::{Deserialize, Serialize};

use crate::{EffectRef, MoveSpec, MoveWindow, ParamValue, WindowTag};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const COUNTER: &str = "smash.counter";

/// Authored parameters of one counter stance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterParams {
    /// Parry heartbeat in seconds. The active stance refreshes it every frame, so this is slack for the existing parry timer rather than the stance duration.
    pub window_s: f32,
    /// Technique key dispatched after a successful interception. The key, not a counter kind, defines the consequence.
    pub response: String,
    /// If true, dispatch the response to the attacker instead of the stance owner.
    /// This stays a boolean because `ParamValue` does not round-trip enum variants reliably.
    #[serde(default)]
    pub answers_the_attacker: bool,
    /// Authored parameters forwarded to the response technique.
    #[serde(default)]
    pub response_params: ParamValue,
    /// If true, consume intercepted projectiles instead of reflecting them. The response key still defines the consequence.
    #[serde(default)]
    pub absorbs_projectiles: bool,
}

/// Build a counter with startup, a stationary active stance, and recovery.
/// The active window has no hit volumes; retaliation is supplied by the response technique.
pub fn counter_move(
    id: &str,
    clip: &str,
    startup_s: f32,
    stance_s: f32,
    recover_s: f32,
    params: CounterParams,
) -> MoveSpec {
    assert!(
        stance_s > 0.0,
        "counter move `{id}` holds its stance for {stance_s}s, which is never open",
    );
    assert!(
        params.window_s > 0.0,
        "counter move `{id}` authors a {}s parry window, which never opens — \
         `parrying()` requires a timer strictly above zero",
        params.window_s,
    );
    let stance_end = startup_s + stance_s;
    MoveSpec {
        display_name: None,
        id: id.to_string(),
        clip: crate::ClipBinding {
            clip: clip.to_string(),
            fallbacks: vec!["attack".to_string(), "idle".to_string()],
        },
        duration_s: stance_end + recover_s,
        windows: vec![
            plain_window(WindowTag::Startup, 0.0, startup_s, 1.0, None),
            plain_window(
                WindowTag::Active,
                startup_s,
                stance_end,
                0.0,
                Some(EffectRef {
                    key: COUNTER.to_string(),
                    params: ParamValue::from_typed(&params).expect("counter params serialize"),
                }),
            ),
            plain_window(
                WindowTag::Recovery,
                stance_end,
                stance_end + recover_s,
                1.0,
                None,
            ),
        ],
        events: Vec::new(),
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: crate::ChargeGesture::default(),
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
    }
}

fn plain_window(
    tag: WindowTag,
    start_s: f32,
    end_s: f32,
    motion_scale: f32,
    sustain_effect: Option<EffectRef>,
) -> MoveWindow {
    MoveWindow {
        start_s,
        end_s,
        tag,
        volumes: Vec::new(),
        motion_scale,
        sustain_effect,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nested response parameters must survive the counter's `ParamValue` round trip.
    #[test]
    fn nested_response_params_survive_a_round_trip() {
        let params = CounterParams {
            window_s: 0.05,
            answers_the_attacker: false,
            response: "smash.capture_attempt".to_string(),
            response_params: ParamValue::parse(
                "(offset: (28.0, 0.0), half_extents: (20.0, 24.0), hold_offset: (24.0, 0.0))",
            )
            .expect("the response params are valid RON"),
            absorbs_projectiles: false,
        };
        let carried = ParamValue::from_typed(&params).expect("counter params serialize");
        let back: CounterParams = carried.hydrate().expect("counter params hydrate");
        assert_eq!(
            back, params,
            "a counter's response params did not survive the nesting, so every \
             counter would fire its answer with the wrong arguments"
        );
    }

    /// A stance whose window is open for zero time is refused where it is
    /// authored, not silently never catching anything.
    #[test]
    fn a_counter_window_that_never_opens_is_refused() {
        let refused = std::panic::catch_unwind(|| {
            counter_move(
                "test_counter",
                "special",
                0.1,
                0.2,
                0.3,
                CounterParams {
                    window_s: 0.0,
                    // Its own answer, as every counter but the clerk's is.
                    answers_the_attacker: false,
                    response: "whatever".to_string(),
                    response_params: ParamValue::default(),
                    absorbs_projectiles: false,
                },
            )
        });
        assert!(
            refused.is_err(),
            "a counter stance with a zero parry window was accepted, so the \
             move would stand there defending nothing and look like a timing \
             problem to whoever played it"
        );
    }
}

#[cfg(test)]
mod round_trip_probe {
    use super::*;
    use crate::ParamValue;

    #[test]
    fn counter_params_survive_a_param_value_round_trip() {
        let params = CounterParams {
            window_s: 0.05,
            answers_the_attacker: true,
            response: "smash.time_dilation".to_string(),
            response_params: ParamValue::default(),
            absorbs_projectiles: false,
        };
        let value = ParamValue::from_typed(&params).expect("serialize");
        let back = value.hydrate::<CounterParams>();
        assert!(
            back.is_ok(),
            "CounterParams did not survive a ParamValue round trip: {:?}",
            back.err()
        );
        assert!(back.unwrap().answers_the_attacker);
    }
}
