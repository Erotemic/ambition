//! **THE CAPTURE VOCABULARY — grab, pummel, throw, authored once.**
//!
//! The sibling of [`smash_repertoire`](crate::smash_repertoire), and it lives
//! beside it for the same stated reason and under the same stated caveat: this
//! is SMASH's vocabulary, held here TRANSITIONALLY because the generic character
//! crate is where a fighter's authoring currently lands. `ForwardThrow` is no
//! more a universal character concept than `ForwardSmash` is.
//!
//! ⛔ **do not move it for purity.** The restitch point is the first real
//! character-owned `smash.fighter` facet: when that seam exists, the Smash
//! capability should own these schemas and character packages should author
//! their values. Moving the types before then costs a migration and buys
//! nothing — see the same note on [`crate::smash_repertoire`].
//!
//! ## What a capture IS, and why it is not a hit
//!
//! ```text
//! a HIT       spatial overlap → damage → knockback → over, inside one move
//! a CAPTURE   spatial acquisition → RELATIONSHIP → later moves target it → release
//! ```
//!
//! That difference is the whole reason this module exists rather than a
//! `HitVolume { damage: 0, grab: true }`. A grab beats a shield instead of being
//! stopped by one; a pummel affects an ALREADY-SELECTED counterpart instead of
//! everything overlapping a box; a throw ends the relationship at an authored
//! frame. None of those are expressible as a damage payload.
//!
//! ## How a fighter says it
//!
//! Three authored [`MoveSpec`]s, no new timeline vocabulary. The engine already
//! has the seam this needs — [`MoveWindow::sustain_effect`] and
//! [`MoveEventKind::Effect`] exist precisely so an authored move can invoke
//! gameplay semantics without another variant on the generic timeline — so a
//! capture is expressed through [`EffectRef`]s and the generic move runtime
//! never learns what a forward throw is.
//!
//! ⛔ **fighter files must not spell these keys or hand-write their params.**
//! The helpers below own both, so a fighter authors VALUES and the strings stay
//! in one module — which is also what makes these movesets relocatable into a
//! character package later without rewriting every fighter.
//!
//! ## ⚠ On startup validation, which does NOT exist here
//!
//! `ambition_entity_catalog::ParamSchemaRegistry` is the documented road for
//! *"a param typo fails at startup, not mid-fight"*. **It has no production
//! installer** — measured 2026-08-18: the type, one unit test, and zero callers
//! of `validate` anywhere in the tree. So registering a check for these keys
//! would register it with nobody.
//!
//! ⭐ **and it costs this road nothing, because a fighter never writes params by
//! hand.** `author_standing_grab(spec, CaptureAttemptParams { … })` takes a
//! typed struct, so a misspelled or missing field is a COMPILE error in the
//! fighter's own file — strictly earlier than startup. The registry matters for
//! hand-written RON, which no capture move uses.
//!
//! ⇒ if a capture param ever becomes authorable as loose RON, wiring the
//! registry is a precondition of that change rather than a follow-up to it.

use ambition_entity_catalog::{EffectRef, MoveSpec, ParamValue, VolumeShape};
use serde::{Deserialize, Serialize};

/// **The effect key an active grab window sustains.**
///
/// Sustained rather than one-shot on purpose: a grab is spatially live for a
/// window, so the handler is asked every active frame and acquires on the first
/// frame something eligible overlaps. That gives the correct behaviour for free
/// — frame 1 catches nobody, frame 2 catches a body that just walked in, and the
/// remaining frames see a captor that already holds somebody and do nothing.
pub const CAPTURE_ATTEMPT: &str = "smash.capture_attempt";
/// The effect key a pummel's impact frame emits, once.
pub const CAPTURE_PUMMEL: &str = "smash.capture_pummel";
/// The effect key a throw's authored RELEASE frame emits, once.
pub const CAPTURE_THROW: &str = "smash.capture_throw";

/// Authored parameters of a grab attempt.
///
/// ⛔ **the reach is a RECT SPELLED OUT, not a [`VolumeShape`], and that is the
/// transport's rule rather than a preference.** `ParamValue` stores authored
/// params as a `ron::Value`, whose deserializer cannot carry an enum — the type
/// says so in as many words (*"Enum-valued params are unsupported … model those
/// as string tags"*). A `VolumeShape` field serialises to `Rect(offset: …)` and
/// hydrates back as a bare map with the variant lost:
/// `InvalidValueForType { expected: "enum VolumeShape", found: "a map" }`.
/// Found by the round-trip assertion in this module's tests rather than by a
/// fighter's data failing at startup.
///
/// ⇒ v1 grabs are rectangular. A circular grab is a real future want and the
/// documented shape for it is a string tag beside these fields; inventing that
/// tag now, for zero callers, is the generalisation nobody asked for.
/// [`Self::volume`] rebuilds the engine type at the consumer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureAttemptParams {
    /// Centre of the grab reach, body-local: `+x` = the captor's committed
    /// facing, `+y` = gravity-down. The SAME contract an authored `HitVolume`
    /// uses, so a grab box and an attack box rotate together under arbitrary
    /// gravity.
    pub offset: (f32, f32),
    /// Half-extents of the grab reach, body-local.
    pub half_extents: (f32, f32),
    /// Where a caught body is held, in the captor's body-local frame. The
    /// simulation's anchor — NOT a sprite offset. Presentation may draw the
    /// captive anywhere it likes relative to this; the constraint uses this.
    pub hold_offset: (f32, f32),
}

impl CaptureAttemptParams {
    /// The reach as the engine's own volume type, for the acquisition pass.
    pub fn volume(&self) -> VolumeShape {
        VolumeShape::Rect {
            offset: self.offset,
            half_extents: self.half_extents,
        }
    }
}

/// Authored parameters of one pummel impact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapturePummelParams {
    /// Damage committed to the captive. No knockback, no hitstun, no
    /// post-hit invulnerability — the acquisition already happened, and a
    /// pummel that armed a hit reaction would release the very grab it belongs
    /// to.
    pub damage: i32,
}

/// Authored parameters of one throw release.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureThrowParams {
    pub damage: i32,
    /// Base knockback, before the victim's damage and weight are applied. Fed
    /// to the SAME scaled-knockback road every authored launcher uses, so a
    /// throw inherits weight, percent scaling, DI and arbitrary gravity rather
    /// than growing a second launch engine.
    pub knockback: f32,
    /// How much the launch grows with the victim's accumulated damage.
    pub knockback_growth: f32,
    /// Launch direction, body-local: `+x` = the captor's facing, `+y` =
    /// gravity-down. Same contract as [`CaptureAttemptParams::volume`].
    pub launch_dir: (f32, f32),
}

/// **The three-window shell a grab needs**, so a fighter authors TIMINGS rather
/// than a window list.
///
/// ⭐ it lives here rather than in either game because both providers need the
/// identical shape and because [`author_standing_grab`] REFUSES a move with no
/// Active window — a module that enforces a shape should be able to hand you
/// one. A fighter still owns every number.
///
/// ⚠ no hit volume, ever. A grab's Active window carries a capture ATTEMPT, and
/// a volume beside it would make the same frames both grab and hit.
pub fn grab_shell(id: &str, clip: &str, startup_s: f32, active_s: f32, recover_s: f32) -> MoveSpec {
    let active_end = startup_s + active_s;
    MoveSpec {
        id: id.to_string(),
        clip: ambition_entity_catalog::ClipBinding {
            clip: clip.to_string(),
            fallbacks: vec!["attack".to_string(), "idle".to_string()],
        },
        duration_s: active_end + recover_s,
        windows: vec![
            window(ambition_entity_catalog::WindowTag::Startup, 0.0, startup_s),
            window(
                ambition_entity_catalog::WindowTag::Active,
                startup_s,
                active_end,
            ),
            window(
                ambition_entity_catalog::WindowTag::Recovery,
                active_end,
                active_end + recover_s,
            ),
        ],
        events: Vec::new(),
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        landing_lag_s: None,
        autocancel_after_s: None,
    }
}

/// **The shell a pummel or a throw needs**: a timeline and nothing else.
///
/// Neither reaches for anybody — the target is already established — so neither
/// has an Active window or a volume. What each has is an INSTANT, attached by
/// [`author_pummel`] or [`author_throw`].
pub fn capture_beat(id: &str, clip: &str, duration_s: f32) -> MoveSpec {
    MoveSpec {
        id: id.to_string(),
        clip: ambition_entity_catalog::ClipBinding {
            clip: clip.to_string(),
            fallbacks: vec!["attack".to_string(), "idle".to_string()],
        },
        duration_s,
        windows: Vec::new(),
        events: Vec::new(),
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        landing_lag_s: None,
        autocancel_after_s: None,
    }
}

fn window(
    tag: ambition_entity_catalog::WindowTag,
    start_s: f32,
    end_s: f32,
) -> ambition_entity_catalog::MoveWindow {
    ambition_entity_catalog::MoveWindow {
        start_s,
        end_s,
        tag,
        volumes: Vec::new(),
        motion_scale: 1.0,
        sustain_effect: None,
    }
}

/// Attach a grab attempt to `spec`'s ACTIVE window(s).
///
/// ⚠ it sustains rather than firing once, and it attaches to every window the
/// move tagged `Active` rather than a hand-picked index — a fighter that authors
/// a two-part grab gets both parts without this helper growing an argument.
///
/// # Panics
///
/// If `spec` has no `Active` window. A grab whose attempt is never live is a
/// recovery animation, and finding that out at authoring time is the point of
/// the helper existing at all.
pub fn author_standing_grab(mut spec: MoveSpec, params: CaptureAttemptParams) -> MoveSpec {
    let effect = EffectRef {
        key: CAPTURE_ATTEMPT.to_string(),
        params: ParamValue::from_typed(&params).expect("capture attempt params serialize"),
    };
    let mut attached = 0usize;
    for window in &mut spec.windows {
        if window.tag == ambition_entity_catalog::WindowTag::Active {
            window.sustain_effect = Some(effect.clone());
            attached += 1;
        }
    }
    assert!(
        attached > 0,
        "grab move `{}` has no Active window, so its capture attempt would never \
         be live — it would play, cost its recovery, and be unable to catch anybody",
        spec.id
    );
    spec
}

/// Attach a pummel impact to `spec` at `at_s` of its own timeline.
pub fn author_pummel(mut spec: MoveSpec, at_s: f32, params: CapturePummelParams) -> MoveSpec {
    spec.events.push(ambition_entity_catalog::MoveEvent {
        at_s,
        kind: ambition_entity_catalog::MoveEventKind::Effect(EffectRef {
            key: CAPTURE_PUMMEL.to_string(),
            params: ParamValue::from_typed(&params).expect("capture pummel params serialize"),
        }),
    });
    spec
}

/// Attach a throw RELEASE to `spec` at `at_s` of its own timeline.
///
/// ⭐ the release is a timeline instant, not the button press. The captive stays
/// constrained through the wind-up and leaves at this frame, which is what makes
/// a throw's wind-up readable and punishable rather than instantaneous.
pub fn author_throw(mut spec: MoveSpec, at_s: f32, params: CaptureThrowParams) -> MoveSpec {
    spec.events.push(ambition_entity_catalog::MoveEvent {
        at_s,
        kind: ambition_entity_catalog::MoveEventKind::Effect(EffectRef {
            key: CAPTURE_THROW.to_string(),
            params: ParamValue::from_typed(&params).expect("capture throw params serialize"),
        }),
    });
    spec
}

/// **A fighter's capture kit.**
///
/// ⚠ the three throws beyond forward are `Option` DURING THE MIGRATION. The
/// first implementation proves the relationship architecture on two fighters;
/// forcing all fourteen to invent four throws each before that is settled would
/// be authoring against a shape that is still moving. When the roster is
/// migrated and capture is part of the required Smash contract, these lose their
/// `Option` the same way any other slot would.
///
/// ⛔ **an unauthored throw does NOTHING**, and deliberately does not fall back
/// to a pummel. A player who presses up+attack and gets a pummel has been told
/// the fighter has an up-throw that is bad; a player who gets nothing has been
/// told it has none. The second is true.
pub struct SmashCaptureRepertoire {
    /// The standing grab. Its Active window sustains the capture attempt.
    pub grab: MoveSpec,
    /// The pummel: neutral Attack while holding somebody. Repeatable — the
    /// relationship outlives it.
    pub pummel: MoveSpec,
    /// Forward + Attack while holding somebody.
    pub forward_throw: MoveSpec,
    pub back_throw: Option<MoveSpec>,
    pub up_throw: Option<MoveSpec>,
    pub down_throw: Option<MoveSpec>,
}

/// The verb a capture move answers to. **The one place these strings exist.**
///
/// ⛔ they are not a directional family of `grab`. `capture_throw_forward` is
/// selected by the ATTACK press inside a capture relationship, not by a
/// directional grab press — so naming them `grab_forward` would invite the
/// action scheme's directional-verb matcher to light the Grab slot up for a
/// fighter that authored only throws.
pub mod verbs {
    //! ⚠ **re-exports, not a second definition.** The strings live beside
    //! `ATTACK_VERB` and `SMASH_VERB` in `ambition_entity_catalog` because the
    //! move SELECTOR has to resolve them and that crate is the one both the
    //! selector and this authoring module can see. Spelling them again here
    //! would be two places for a typo to become a press that does nothing.
    pub use ambition_entity_catalog::{
        CAPTURE_PUMMEL_VERB as PUMMEL, CAPTURE_THROW_BACK_VERB as THROW_BACK,
        CAPTURE_THROW_DOWN_VERB as THROW_DOWN, CAPTURE_THROW_FORWARD_VERB as THROW_FORWARD,
        CAPTURE_THROW_UP_VERB as THROW_UP, GRAB_VERB as GRAB,
    };
}

impl SmashCaptureRepertoire {
    /// The `(verb, spec)` rows this kit contributes to a moveset contract.
    ///
    /// Every capture move is GROUNDED for v1: aerial and command grabs are named
    /// future techniques, and a grab that answered an airborne press would be one
    /// of them by accident.
    pub(crate) fn bound(self) -> Vec<(&'static str, MoveSpec)> {
        let Self {
            grab,
            pummel,
            forward_throw,
            back_throw,
            up_throw,
            down_throw,
        } = self;
        let mut out = vec![
            (verbs::GRAB, grab),
            (verbs::PUMMEL, pummel),
            (verbs::THROW_FORWARD, forward_throw),
        ];
        for (verb, spec) in [
            (verbs::THROW_BACK, back_throw),
            (verbs::THROW_UP, up_throw),
            (verbs::THROW_DOWN, down_throw),
        ] {
            if let Some(spec) = spec {
                out.push((verb, spec));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::{ClipBinding, MoveWindow, WindowTag};

    fn spec(id: &str, windows: Vec<MoveWindow>) -> MoveSpec {
        MoveSpec {
            id: id.to_string(),
            clip: ClipBinding {
                clip: id.to_string(),
                fallbacks: Vec::new(),
            },
            duration_s: 0.4,
            windows,
            events: Vec::new(),
            gates: Default::default(),
            start_impulse: None,
            smash_charge_mult: 1.0,
            landing_lag_s: None,
            autocancel_after_s: None,
        }
    }

    fn window(tag: WindowTag, start_s: f32, end_s: f32) -> MoveWindow {
        MoveWindow {
            start_s,
            end_s,
            tag,
            volumes: Vec::new(),
            motion_scale: 1.0,
            sustain_effect: None,
        }
    }

    fn attempt() -> CaptureAttemptParams {
        CaptureAttemptParams {
            offset: (14.0, 0.0),
            half_extents: (10.0, 12.0),
            hold_offset: (16.0, -2.0),
        }
    }

    /// **THE ATTEMPT IS LIVE FOR THE WHOLE ACTIVE WINDOW, AND ONLY THERE.**
    ///
    /// A grab that sustained through Startup would catch a body before the tell
    /// finished, which is the frame data a shield read is made against; one that
    /// sustained through Recovery would make a whiffed grab free.
    #[test]
    fn a_grab_sustains_its_attempt_on_the_active_window_alone() {
        let grab = author_standing_grab(
            spec(
                "grab",
                vec![
                    window(WindowTag::Startup, 0.0, 0.1),
                    window(WindowTag::Active, 0.1, 0.16),
                    window(WindowTag::Recovery, 0.16, 0.4),
                ],
            ),
            attempt(),
        );
        let live: Vec<WindowTag> = grab
            .windows
            .iter()
            .filter(|w| w.sustain_effect.is_some())
            .map(|w| w.tag.clone())
            .collect();
        assert_eq!(live, vec![WindowTag::Active], "{live:?}");

        let carried = grab.windows[1].sustain_effect.as_ref().unwrap();
        assert_eq!(carried.key, CAPTURE_ATTEMPT);
        assert_eq!(
            carried.params.hydrate::<CaptureAttemptParams>().unwrap(),
            attempt(),
            "the authored params did not survive the round trip through ParamValue"
        );
    }

    /// **A GRAB WITH NO ACTIVE WINDOW IS CAUGHT AT AUTHORING TIME.**
    ///
    /// It would otherwise play, cost its recovery, and be unable to catch
    /// anybody — a fighter that looks like it has a grab and does not. The whole
    /// reason the helper exists rather than fighters writing the `EffectRef`
    /// themselves is that it can refuse this.
    #[test]
    #[should_panic(expected = "no Active window")]
    fn a_grab_that_is_never_live_refuses_to_be_authored() {
        author_standing_grab(
            spec(
                "grab",
                vec![
                    window(WindowTag::Startup, 0.0, 0.2),
                    window(WindowTag::Recovery, 0.2, 0.4),
                ],
            ),
            attempt(),
        );
    }

    /// **AN UNAUTHORED THROW CONTRIBUTES NO VERB.**
    ///
    /// The v1 migration rule, stated as a test: a fighter with only a forward
    /// throw offers three capture verbs and not six. A press for a throw it does
    /// not have must find nothing — NOT fall through to a pummel, which would
    /// tell the player the fighter has a bad up-throw when it has none.
    #[test]
    fn an_unauthored_throw_is_absent_rather_than_substituted() {
        let kit = SmashCaptureRepertoire {
            grab: author_standing_grab(
                spec("g", vec![window(WindowTag::Active, 0.0, 0.1)]),
                attempt(),
            ),
            pummel: author_pummel(spec("p", vec![]), 0.05, CapturePummelParams { damage: 2 }),
            forward_throw: author_throw(
                spec("fthrow", vec![]),
                0.12,
                CaptureThrowParams {
                    damage: 9,
                    knockback: 60.0,
                    knockback_growth: 0.8,
                    launch_dir: (1.0, -0.4),
                },
            ),
            back_throw: None,
            up_throw: None,
            down_throw: None,
        };
        let verbs: Vec<&str> = kit.bound().into_iter().map(|(v, _)| v).collect();
        assert_eq!(
            verbs,
            vec![verbs::GRAB, verbs::PUMMEL, verbs::THROW_FORWARD],
            "an absent throw invented a verb, or an authored one lost its"
        );
    }
}
