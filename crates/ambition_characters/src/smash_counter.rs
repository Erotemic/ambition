//! The counter stance: the authored vocabulary for "if you hit me here, this
//! happens".
//!
//! ⭐⭐ THE SAME SPLIT `smash_capture` AND `smash_teleport` USE. A key and its
//! params are what a MOVESET authors, so they live where movesets can name them;
//! holding the parry window open and dispatching the response are the GAME's
//! job, and that half sits in the smash demo beside the capture adapter.
//!
//! ⛔⛔ THIS TECHNIQUE ADDS NO DEFENSIVE MECHANIC, AND THAT IS THE WHOLE POINT.
//! The perfect shield already denies a qualifying attack, decides it
//! deterministically, and now says who it denied (`ParriedBodyHit`). A counter
//! is that fact plus an authored consequence. The three counters a platform
//! fighter wants differ ONLY in the consequence:
//!
//! * an ordinary counter answers with an attack;
//! * a Revenge-style counter answers with a lasting character modifier;
//! * a Witch-Time-style counter answers by slowing the attacker.
//!
//! ⇒ So there is no `CounterKind` here and there must never be one. The kind is
//! whichever technique the author names in [`CounterParams::response`], which
//! means a counter can answer with anything the game can already do, and a
//! technique added for some other reason becomes a counter response for free.

use serde::{Deserialize, Serialize};

use ambition_entity_catalog::{EffectRef, MoveSpec, MoveWindow, ParamValue, WindowTag};

/// The authored effect key. Namespaced like every other smash technique so an
/// unrecognised key falls through other rulesets untouched.
pub const COUNTER: &str = "smash.counter";

/// Authored parameters of one counter stance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterParams {
    /// How long the parry window stays open, in seconds.
    ///
    /// ⭐ REFRESHED EVERY FRAME THE AUTHORING WINDOW IS ACTIVE, not set once.
    /// `BodyShieldState::parrying()` is the timer alone — `parry_window_timer >
    /// 0.0` — and it counts down, so a stance that set it once would decay
    /// mid-window and the counter would stop catching part-way through its own
    /// authored frames. ⇒ Author this as roughly one tick of slack rather than
    /// the stance's length: it is a "still open" heartbeat, not a duration.
    pub window_s: f32,
    /// The technique fired on a successful interception — the counter's answer.
    ///
    /// ⛔ A KEY, NOT A KIND. See the module note: what makes this a retaliation
    /// or a buff or a slow is which technique is named here, and nothing in the
    /// engine needs to know which of those it is.
    pub response: String,
    /// Aim [`Self::response`] at the body that SWUNG rather than at the
    /// stance's owner. `false` — the default, and what every stance authored
    /// before this field existed meant.
    ///
    /// ⭐⭐ THE THIRD COUNTER THIS MODULE NAMES NEEDED A TARGET, NOT AN
    /// AUTHORITY. The header lists the three a platform fighter wants — an
    /// ordinary counter answers with an attack, a Revenge-style one with a
    /// lasting modifier, and a **Witch-Time-style one by SLOWING THE ATTACKER**.
    /// The first two shipped; the third could not be written, because every
    /// response was dispatched to the stance's OWNER and no authored effect
    /// could name the body that swung.
    ///
    /// ⛔ THE FACT WAS ALREADY PUBLISHED, which is the rule every signal here
    /// obeys: `ParriedBodyHit::attacker` exists and its own doc says why the
    /// parry road has it — *"known here, unlike on the block road, because a
    /// parry is resolved at the strike rather than at a damage resolution that
    /// may have lost the striker."*
    ///
    /// ⛔⛔ A `bool` AND NOT AN ENUM, AND THAT IS NOT A STYLE CHOICE. This was a
    /// two-variant enum for an afternoon and **every counter in the game
    /// silently stopped working**: `ParamValue` is a `ron::Value`, and a unit
    /// variant does not survive the round trip (`InvalidValueForType { expected:
    /// "enum …", found: "a unit value" }`). Hydration failed, the stance
    /// resolved to `None`, and the only signal was a `warn!` nobody reads. ⇒ See
    /// `round_trip_probe` at the bottom of this file: the guard that turns that
    /// into a compile-fast failure instead of a playtest.
    #[serde(default)]
    pub answers_the_attacker: bool,
    /// The response technique's own authored params, passed through untouched.
    ///
    /// ⚠ NESTED, WHICH THE TELEPORT'S PARAMS DOC SAYS TO BE CAREFUL ABOUT.
    /// That warning is about ENUMS — a `ron::Value` cannot round-trip a struct
    /// variant — and not about maps. A `ParamValue` IS a `ron::Value`, so it
    /// nests inside another one exactly as a map, which
    /// `nested_response_params_survive_a_round_trip` holds to.
    #[serde(default)]
    pub response_params: ParamValue,
    /// This stance ABSORBS projectiles instead of returning them.
    ///
    /// ⭐ A COUNTER ALREADY REFLECTS SHOTS FOR FREE — the projectile road gates
    /// on the same `parrying()` window this stance opens, which is how a
    /// reflector arrived without anyone authoring one. This flips that: the shot
    /// is consumed instead, and the stance's `response` is what the absorption
    /// was WORTH.
    ///
    /// ⛔ THE RESPONSE IS STILL THE AUTHOR'S. Absorbing that heals, that fills a
    /// gauge, and that simply deletes the shot are one interception and three
    /// consequences; this field chooses the interception, not the consequence.
    #[serde(default)]
    pub absorbs_projectiles: bool,
}

/// A complete counter move: startup, the stance, and recovery.
///
/// ⛔ THE STANCE WINDOW CARRIES NO VOLUMES, DELIBERATELY. A move that defended
/// and swung in the same frames would put its own strike into the set of things
/// its parry could catch, and "what did I counter" would stop having one answer.
/// The retaliation is the RESPONSE's business and happens after the catch.
///
/// ⭐ `motion_scale: 0.0` ON THE STANCE. A counter is a commitment — standing
/// still is what the window costs — and the engine enforces motion scale
/// body-side for any controller, so a player and a brain pay it alike.
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
        clip: ambition_entity_catalog::ClipBinding {
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
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
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

    /// The response's params survive being carried inside the counter's params.
    ///
    /// ⛔ THE ONE THING THIS DESIGN RESTS ON. If a nested `ParamValue` did not
    /// round-trip, every counter would dispatch its response with empty params
    /// and the failure would look like a response technique misbehaving rather
    /// than like the counter losing its argument.
    #[test]
    fn nested_response_params_survive_a_round_trip() {
        let params = CounterParams {
            window_s: 0.05,
            // Its own answer, as every counter but the clerk's is.
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
    /// AUTHORED rather than silently never catching anything.
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
    use ambition_entity_catalog::ParamValue;

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
