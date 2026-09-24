//! Platform-fighter capture vocabulary: grab, pummel, and throw.
//!
//! Captures are relationships rather than damage hits: a grab acquires a target, later
//! moves operate on that selected counterpart, and a throw releases it at an authored
//! frame. Fighter code authors typed values through the helpers here; effect keys and
//! parameter encoding stay centralized. The generic move timeline sees ordinary
//! `MoveSpec` effect windows/events and does not learn capture-specific variants.

use crate::{EffectRef, MoveEvent, MoveEventKind, MoveSpec, ParamValue, VolumeShape};
use serde::{Deserialize, Serialize};

/// The effect key an active grab window sustains.
///
/// Sustained rather than one-shot on purpose: a grab is spatially live for a
/// window, so the handler is asked every active frame and acquires on the first
/// frame something eligible overlaps. So frame 1 catches nobody, frame 2
/// catches a body that just walked in, and later frames see a captor that
/// already holds somebody and do nothing.
pub const CAPTURE_ATTEMPT: &str = "smash.capture_attempt";
/// The effect key a pummel's impact frame emits, once.
pub const CAPTURE_PUMMEL: &str = "smash.capture_pummel";
/// The effect key a carry emits, once, on the frame the captor takes the
/// weight.
///
/// A carry is entered from a throw-shaped move, not from the grab: you grab
/// normally, then one direction puts the captive on your shoulders. So the
/// grab params (`CaptureAttemptParams`) do not need a carry field.
///
/// It is not a throw and must not release. `apply_capture_throws` ends the
/// relationship; a carry keeps it and changes its terms.
pub const CAPTURE_CARRY: &str = "smash.capture_carry";
/// The effect key a throw's authored release frame emits, once.
pub const CAPTURE_THROW: &str = "smash.capture_throw";

/// The three cues a capture shows, authored per fighter.
///
/// Not constants here: some fighters' tests require every effect in their kit
/// to come from their own sheet. The helper owns when a cue fires; the
/// fighter owns which effect it is.
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureCues {
    /// Fires on the grab's first live frame, so a whiff reads as an attempt at
    /// the moment it could have caught somebody.
    pub reach: &'static str,
    /// Fires on the pummel's own `at_s`.
    pub impact: &'static str,
    /// Fires on the throw's release frame.
    pub release: &'static str,
}

impl CaptureCues {
    /// The shipped generic rows, for a fighter whose art is generic anyway.
    pub const GENERIC: Self = Self {
        reach: "smoke_burst",
        impact: "classic_burst",
        release: "shockwave",
    };
}

/// Authored parameters of a grab attempt.
///
/// The reach is a rect spelled out, not a [`VolumeShape`]. `ParamValue` stores
/// params as a `ron::Value`, which cannot carry an enum: a `VolumeShape`
/// hydrates back as a bare map with the variant lost
/// (`InvalidValueForType { expected: "enum VolumeShape", found: "a map" }`).
/// The round-trip test in this module guards this.
///
/// Grabs are rectangular for now. A circular grab would use a string tag beside
/// these fields. [`Self::volume`] rebuilds the engine type at the consumer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureAttemptParams {
    /// Centre of the grab reach, body-local: `+x` = the captor's committed
    /// facing, `+y` = gravity-down. This is the contract of an authored
    /// `HitVolume`, so a grab box and an attack box rotate together under any
    /// gravity.
    pub offset: (f32, f32),
    /// Half-extents of the grab reach, body-local.
    pub half_extents: (f32, f32),
    /// Where a caught body is held, in the captor's body-local frame. This is
    /// the simulation's anchor, not a sprite offset; presentation may draw the
    /// captive anywhere relative to it.
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

    /// How far forward this grab reaches, body-local: the leading edge of the
    /// box, not its center.
    ///
    /// Use this one formula. The brain's `AttackCandidate::reach` (the distance
    /// a fighter closes to before it grabs) and the tether line the player sees
    /// must agree; separate copies of the sum can drift apart silently. This
    /// matters most for long grabs (Projectile Polygon's reaches 150 px).
    ///
    /// Body-local and unsigned by facing. The caller applies the captor's
    /// committed facing, which a move locks at start.
    pub fn reach_x(&self) -> f32 {
        self.offset.0 + self.half_extents.0
    }

    /// The reach's vertical center, body-local: the other half of where a line
    /// to this grab is drawn.
    pub fn reach_y(&self) -> f32 {
        self.offset.1
    }

    /// The grab box as `(min, max)` corners, body-local: where this move
    /// covers, for a planner.
    ///
    /// `max.0` is [`Self::reach_x`] by construction. Callers must use this and
    /// not compute the corners themselves.
    pub fn coverage(&self) -> ((f32, f32), (f32, f32)) {
        (
            (
                self.offset.0 - self.half_extents.0,
                self.offset.1 - self.half_extents.1,
            ),
            (self.reach_x(), self.offset.1 + self.half_extents.1),
        )
    }
}

/// Authored parameters of one pummel impact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturePummelParams {
    /// Damage committed to the captive. No knockback, no hitstun, no post-hit
    /// invulnerability: a pummel that armed a hit reaction would release its
    /// own grab.
    pub damage: i32,
}

/// Authored parameters of one throw release.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureThrowParams {
    pub damage: i32,
    /// Base knockback, before the victim's damage and weight apply. It goes
    /// through the same scaled-knockback road as every authored launcher, so a
    /// throw gets weight, percent scaling, DI and any gravity.
    pub knockback: f32,
    /// How much the launch grows with the victim's accumulated damage.
    pub knockback_growth: f32,
    /// Launch direction, body-local: `+x` = the captor's facing, `+y` =
    /// gravity-down. Same contract as [`CaptureAttemptParams::volume`].
    pub launch_dir: (f32, f32),
}

/// The three-window shell a grab needs, so a fighter authors timings, not a
/// window list.
///
/// It lives here because [`author_standing_grab`] refuses a move with no
/// Active window, so the module that enforces the shape also provides it. The
/// fighter owns every number.
///
/// No hit volume: the Active window carries a capture attempt, and a volume
/// there would make the same frames both grab and hit.
pub fn grab_shell(id: &str, clip: &str, startup_s: f32, active_s: f32, recover_s: f32) -> MoveSpec {
    let active_end = startup_s + active_s;
    MoveSpec {
        display_name: None,
        id: id.to_string(),
        clip: crate::ClipBinding {
            clip: clip.to_string(),
            fallbacks: vec!["attack".to_string(), "idle".to_string()],
        },
        duration_s: active_end + recover_s,
        windows: vec![
            window(crate::WindowTag::Startup, 0.0, startup_s),
            window(
                crate::WindowTag::Active,
                startup_s,
                active_end,
            ),
            window(
                crate::WindowTag::Recovery,
                active_end,
                active_end + recover_s,
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

/// The shell a pummel or a throw needs: a timeline and nothing else.
///
/// Neither reaches for anybody (the target is already established), so neither
/// has an Active window or a volume. Each has an instant, attached by
/// [`author_pummel`] or [`author_throw`].
pub fn capture_beat(id: &str, clip: &str, duration_s: f32) -> MoveSpec {
    MoveSpec {
        display_name: None,
        id: id.to_string(),
        clip: crate::ClipBinding {
            clip: clip.to_string(),
            fallbacks: vec!["attack".to_string(), "idle".to_string()],
        },
        duration_s,
        windows: Vec::new(),
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

/// Extra startup a running grab pays over the standing one: the windup of
/// reaching out while moving. Two frames at 60Hz.
const RUNNING_GRAB_EXTRA_STARTUP_S: f32 = 2.0 / 60.0;
/// Extra recovery a running grab pays. This is the trade: a whiffed grab from
/// a run is punishable in a way a standing whiff is not. Twelve frames at
/// 60Hz.
const RUNNING_GRAB_EXTRA_RECOVERY_S: f32 = 12.0 / 60.0;

/// A fighter's running grab, derived from its own standing grab.
///
/// Derived, not authored. A dash attack is a different move, so each fighter
/// authors one. A dash grab is the same reach-out from a run: same clip, same
/// catch, slower to start and much slower to end. Deriving it gives every
/// fighter one in its own timing, with no slot to forget.
///
/// The extra time is in seconds, not a ratio: a multiplier would punish a
/// fast grab less than a slow one.
///
/// The windows shift together: everything after the startup moves later by
/// the added windup, and only recovery stretches by the added endlag.
fn running_grab_from(standing: &MoveSpec) -> MoveSpec {
    // Exhaustive on purpose. This rewrites the move's timeline, and every
    // absolute time must move together. A new `MoveSpec` field stops this
    // compiling, and its author must decide whether it is a point on the
    // timeline (shift it) or a duration owed elsewhere (leave it).
    let MoveSpec {
        id,
        display_name: _,
        clip,
        duration_s,
        windows,
        events,
        gates,
        start_impulse,
        smash_charge_mult,
        // A charge policy has both kinds: `hold_at_s` is a point and shifts
        // with the added startup; `max_hold_s` is a duration and does not.
        smash_charge,
        // Neither: which button holds the charge does not move.
        charge_gesture,
        // A loop is a stretch of the timeline: both ends are points and shift.
        repeat,
        // A duration owed after landing, not a point: it does not move.
        landing_lag_s,
        // A point measured from the move's start, so it moves with the rest.
        autocancel_after_s,
        // Neither: a rate, and presentation. The running grab spins like the
        // standing one.
        sprite_spin_hz,
        // Neither: what the move holds. The running grab brandishes the same
        // item for as long as its own (longer) clock runs.
        equips,
        // Inherited, like `windows`: a running grab makes the same decision
        // from a run. The clock stretches; the sequence does not change.
        flow,
    } = standing.clone();
    let mut running = MoveSpec {
        // A derived move never inherits a hand-written label: the standing
        // grab's label would name the wrong beat.
        display_name: None,
        id: format!("{id}_dash"),
        clip,
        charge_gesture,
        duration_s: duration_s + RUNNING_GRAB_EXTRA_STARTUP_S + RUNNING_GRAB_EXTRA_RECOVERY_S,
        windows,
        events,
        gates,
        start_impulse,
        smash_charge_mult,
        smash_charge: smash_charge.map(|policy| crate::SmashChargeSpec {
            hold_at_s: policy.hold_at_s + RUNNING_GRAB_EXTRA_STARTUP_S,
            ..policy
        }),
        repeat: repeat.map(|l| crate::MoveLoop {
            from_s: l.from_s + RUNNING_GRAB_EXTRA_STARTUP_S,
            to_s: l.to_s + RUNNING_GRAB_EXTRA_STARTUP_S,
            // A duration, not a point: the loop's maximum does not change.
            ..l
        }),
        landing_lag_s,
        autocancel_after_s: autocancel_after_s.map(|at| at + RUNNING_GRAB_EXTRA_STARTUP_S),
        sprite_spin_hz,
        equips,
        flow,
    };
    // Events happen at the same point in the swing, which is now later. An
    // unshifted event would fire during the added startup.
    for event in &mut running.events {
        event.at_s += RUNNING_GRAB_EXTRA_STARTUP_S;
    }
    for w in &mut running.windows {
        match w.tag {
            crate::WindowTag::Startup => {
                w.end_s += RUNNING_GRAB_EXTRA_STARTUP_S;
            }
            crate::WindowTag::Recovery => {
                w.start_s += RUNNING_GRAB_EXTRA_STARTUP_S;
                w.end_s += RUNNING_GRAB_EXTRA_STARTUP_S + RUNNING_GRAB_EXTRA_RECOVERY_S;
            }
            // Active and anything else the author placed: the catch happens at
            // the same point in the swing, just later.
            _ => {
                w.start_s += RUNNING_GRAB_EXTRA_STARTUP_S;
                w.end_s += RUNNING_GRAB_EXTRA_STARTUP_S;
            }
        }
    }
    running
}

fn window(
    tag: crate::WindowTag,
    start_s: f32,
    end_s: f32,
) -> crate::MoveWindow {
    crate::MoveWindow {
        start_s,
        end_s,
        tag,
        volumes: Vec::new(),
        motion_scale: 1.0,
        sustain_effect: None,
    }
}

/// Attach a grab attempt to `spec`'s Active window(s).
///
/// It sustains, not fires once, and it attaches to every window tagged
/// `Active`, so a two-part grab gets both parts.
///
/// # Panics
///
/// If `spec` has no `Active` window. A grab whose attempt is never live is a
/// recovery animation; this catches it at authoring time.
pub fn author_standing_grab(mut spec: MoveSpec, params: CaptureAttemptParams) -> MoveSpec {
    let effect = EffectRef {
        key: CAPTURE_ATTEMPT.to_string(),
        params: ParamValue::from_typed(&params).expect("capture attempt params serialize"),
    };
    let mut attached = 0usize;
    for window in &mut spec.windows {
        if window.tag == crate::WindowTag::Active {
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

/// The cue a capture beat carries. Fighters author values; the strings stay
/// in this module. Naming the effect names the cue.
fn burst(mut spec: MoveSpec, at_s: f32, effect: &str, scale: f32) -> MoveSpec {
    spec.events.push(crate::MoveEvent {
        at_s,
        kind: crate::MoveEventKind::Vfx {
            effect: effect.to_string(),
            at: (0.0, 0.0),
            scale,
            sfx: None,
        },
    });
    spec
}

/// The cue for a beat that already carries a gameplay `Effect`. It uses the
/// same instant, so retuning the beat moves its flash too.
fn cue_at_effect(spec: MoveSpec, effect: &str, scale: f32) -> MoveSpec {
    let at = spec
        .events
        .iter()
        .find(|e| matches!(e.kind, crate::MoveEventKind::Effect(_)))
        .map(|e| e.at_s)
        .unwrap_or(0.0);
    burst(spec, at, effect, scale)
}

/// The grab's cue, on the first live frame rather than at zero.
fn cue_at_reach(spec: MoveSpec, effect: &str) -> MoveSpec {
    let at = spec
        .windows
        .iter()
        .find(|w| w.tag == crate::WindowTag::Active)
        .map(|w| w.start_s)
        .unwrap_or(0.0);
    burst(spec, at, effect, 0.45)
}

/// Attach a pummel impact to `spec` at `at_s` of its own timeline.
pub fn author_pummel(mut spec: MoveSpec, at_s: f32, params: CapturePummelParams) -> MoveSpec {
    spec.events.push(crate::MoveEvent {
        at_s,
        kind: crate::MoveEventKind::Effect(EffectRef {
            key: CAPTURE_PUMMEL.to_string(),
            params: ParamValue::from_typed(&params).expect("capture pummel params serialize"),
        }),
    });
    spec
}

/// Authored parameters of a carry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureCarryParams {
    /// Where the captive rides once carried, in the captor's body-local frame.
    ///
    /// A carry moves the captive. The grab's `hold_offset` is in front at
    /// arm's length (for a throw); a carried body goes up and over. The
    /// difference must be visible, or the carry looks like a grab that did
    /// not end.
    pub hold_offset: (f32, f32),
}

/// Author a carry onto a throw-shaped move: the captor takes the weight and
/// keeps it.
///
/// # Panics
///
/// If `at_s` is past the move's duration: the carry would never happen and
/// the captor would spend the beat to keep an ordinary hold.
pub fn author_carry(mut spec: MoveSpec, at_s: f32, params: CaptureCarryParams) -> MoveSpec {
    assert!(
        at_s <= spec.duration_s,
        "move `{}` takes the weight at {at_s}s but only lasts {}s, so the carry \
         never happens and the beat is spent on nothing",
        spec.id,
        spec.duration_s,
    );
    spec.events.push(MoveEvent {
        at_s,
        kind: MoveEventKind::Effect(EffectRef {
            key: CAPTURE_CARRY.to_string(),
            params: ParamValue::from_typed(&params).expect("capture-carry params serialize"),
        }),
    });
    spec
}

/// Attach a throw release to `spec` at `at_s` of its own timeline.
///
/// The release is a timeline instant, not the button press. The captive stays
/// held through the windup and leaves at this frame, so a throw's windup is
/// readable and punishable.
pub fn author_throw(mut spec: MoveSpec, at_s: f32, params: CaptureThrowParams) -> MoveSpec {
    spec.events.push(crate::MoveEvent {
        at_s,
        kind: crate::MoveEventKind::Effect(EffectRef {
            key: CAPTURE_THROW.to_string(),
            params: ParamValue::from_typed(&params).expect("capture throw params serialize"),
        }),
    });
    spec
}

/// A fighter's capture kit.
///
/// The three throws beyond forward are `Option` while the roster migrates.
/// When capture is part of the required Smash contract, they stop being
/// `Option`.
///
/// An unauthored throw does nothing; it does not fall back to a pummel. A
/// pummel on up+attack tells the player the fighter has a bad up-throw; doing
/// nothing tells them it has none, which is true.
pub struct SmashCaptureRepertoire {
    /// The standing grab. Its Active window sustains the capture attempt.
    pub grab: MoveSpec,
    /// The pummel: neutral Attack while holding somebody. Repeatable; the
    /// relationship outlives it.
    pub pummel: MoveSpec,
    /// Forward + Attack while holding somebody.
    pub forward_throw: MoveSpec,
    pub back_throw: Option<MoveSpec>,
    pub up_throw: Option<MoveSpec>,
    pub down_throw: Option<MoveSpec>,
    /// What this fighter's capture shows. [`CaptureCues::GENERIC`] for a
    /// fighter with generic art; its own rows for a fighter whose tests require
    /// every effect to come from its own sheet.
    pub cues: CaptureCues,
}

/// The verb a capture move answers to.
///
/// These are not a directional family of `grab`. `capture_throw_forward` is
/// selected by the Attack press inside a capture, not by a directional grab
/// press. Names such as `grab_forward` would make the action scheme's
/// directional-verb matcher light the Grab slot for a fighter with only
/// throws.
pub mod verbs {
    //! Re-exports, not a second definition. The strings live beside
    //! `ATTACK_VERB` and `SMASH_VERB` in the crate root, because the move
    //! selector must resolve them.
    pub use crate::{
        CAPTURE_PUMMEL_VERB as PUMMEL, CAPTURE_THROW_BACK_VERB as THROW_BACK,
        CAPTURE_THROW_DOWN_VERB as THROW_DOWN, CAPTURE_THROW_FORWARD_VERB as THROW_FORWARD,
        CAPTURE_THROW_UP_VERB as THROW_UP, GRAB_DASH_VERB as GRAB_DASH, GRAB_VERB as GRAB,
    };
}

/// The vocabulary's sprite row for a capture beat, asked for first.
///
/// Applied in [`SmashCaptureRepertoire::bound`], which knows the verb of each
/// beat, so a fighter cannot author a pummel and forget the pummel row. The
/// rows are the ones fighter rigs draw: `grab`, `pummel`,
/// `throw_forward`/`_back`/`_up`/`_down`.
///
/// The character's own clip is kept one step down the chain: a sheet with a
/// bespoke row still draws it, and a sheet with only `attack` still lands
/// there.
fn row_first(mut spec: MoveSpec, rows: &[&str]) -> MoveSpec {
    let mut chain: Vec<String> = rows.iter().map(|r| (*r).to_string()).collect();
    chain.push(spec.clip.clip);
    chain.append(&mut spec.clip.fallbacks);
    let mut seen = Vec::new();
    chain.retain(|row| {
        let fresh = !seen.contains(row);
        if fresh {
            seen.push(row.clone());
        }
        fresh
    });
    let mut chain = chain.into_iter();
    spec.clip = crate::ClipBinding {
        clip: chain.next().unwrap_or_default(),
        fallbacks: chain.collect(),
    };
    spec
}

impl SmashCaptureRepertoire {
    /// The `(verb, spec)` rows this kit contributes to a moveset contract.
    ///
    /// Every capture move is grounded for now; aerial and command grabs are
    /// future techniques.
    ///
    /// Public, because a contract built by hand still needs this verb mapping.
    /// Otherwise that table would copy the verb names.
    pub fn bound(self) -> Vec<(&'static str, MoveSpec)> {
        let Self {
            grab,
            pummel,
            forward_throw,
            back_throw,
            up_throw,
            down_throw,
            cues,
        } = self;
        // The cues are applied here, where every beat is already walked, so a
        // fighter cannot author a throw and forget its release flash.
        // The running grab is derived here from the authored standing grab,
        // before the cues are applied, so it gets the fighter's timing and
        // its own reach flash.
        let running_grab = running_grab_from(&grab);
        let mut out = vec![
            (
                verbs::GRAB,
                cue_at_reach(row_first(grab, &["grab"]), cues.reach),
            ),
            (
                verbs::GRAB_DASH,
                cue_at_reach(row_first(running_grab, &["grab"]), cues.reach),
            ),
            (
                verbs::PUMMEL,
                cue_at_effect(row_first(pummel, &["pummel"]), cues.impact, 0.55),
            ),
            (
                verbs::THROW_FORWARD,
                cue_at_effect(
                    row_first(forward_throw, &["throw_forward", "throw"]),
                    cues.release,
                    1.15,
                ),
            ),
        ];
        for (verb, rows, spec) in [
            (verbs::THROW_BACK, &["throw_back", "throw"], back_throw),
            (verbs::THROW_UP, &["throw_up", "throw"], up_throw),
            (verbs::THROW_DOWN, &["throw_down", "throw"], down_throw),
        ] {
            if let Some(spec) = spec {
                out.push((
                    verb,
                    cue_at_effect(row_first(spec, rows), cues.release, 1.15),
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClipBinding, MoveWindow, WindowTag};

    fn spec(id: &str, windows: Vec<MoveWindow>) -> MoveSpec {
        MoveSpec {
            display_name: None,
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

    /// The attempt is live for the whole Active window, and only there.
    ///
    /// Live in Startup, a grab would catch before the tell finished; live in
    /// Recovery, a whiffed grab would be free.
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

    /// A grab with no Active window is caught at authoring time. Otherwise it
    /// would play and cost its recovery without being able to catch anybody.
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

    /// The running grab is the fighter's own grab, later and longer.
    ///
    /// Every window keeps its shape, the catch happens at the same point in
    /// the swing, and the only additions are the windup and the endlag.
    #[test]
    fn a_running_grab_is_the_standing_one_later_and_longer() {
        let standing = grab_shell("grab", "grab", 0.07, 0.05, 0.2);
        let running = super::running_grab_from(&standing);

        assert_eq!(running.id, "grab_dash", "the derived id must not collide");
        assert_eq!(
            running.clip.clip, standing.clip.clip,
            "a derived grab asks for the fighter's own clip"
        );
        assert!(
            (running.duration_s
                - (standing.duration_s
                    + super::RUNNING_GRAB_EXTRA_STARTUP_S
                    + super::RUNNING_GRAB_EXTRA_RECOVERY_S))
                .abs()
                < 1e-6,
            "the running grab must cost exactly the two stated deltas, got {}",
            running.duration_s
        );

        let find = |spec: &MoveSpec, tag: crate::WindowTag| {
            spec.windows
                .iter()
                .find(|w| w.tag == tag)
                .map(|w| (w.start_s, w.end_s))
                .expect("window present")
        };
        use crate::WindowTag;
        let (_, s_end) = find(&standing, WindowTag::Startup);
        let (r_s_start, r_s_end) = find(&running, WindowTag::Startup);
        assert!(r_s_start.abs() < 1e-6, "the wind-up still begins at zero");
        assert!(
            (r_s_end - (s_end + super::RUNNING_GRAB_EXTRA_STARTUP_S)).abs() < 1e-6,
            "the wind-up did not lengthen by the stated startup"
        );

        // The Active window keeps its length: a running grab catches for as
        // long, only later.
        let (a_start, a_end) = find(&standing, WindowTag::Active);
        let (ra_start, ra_end) = find(&running, WindowTag::Active);
        assert!(
            ((ra_end - ra_start) - (a_end - a_start)).abs() < 1e-6,
            "the catch window changed length"
        );
        assert!(
            (ra_start - (a_start + super::RUNNING_GRAB_EXTRA_STARTUP_S)).abs() < 1e-6,
            "the catch did not move later by the stated startup"
        );

        // And recovery carries the whole extra commitment.
        let (rec_start, rec_end) = find(&standing, WindowTag::Recovery);
        let (rr_start, rr_end) = find(&running, WindowTag::Recovery);
        assert!(
            ((rr_end - rr_start) - ((rec_end - rec_start) + super::RUNNING_GRAB_EXTRA_RECOVERY_S))
                .abs()
                < 1e-6,
            "recovery did not absorb the endlag the genre charges"
        );
        assert!(
            (rr_start - (rec_start + super::RUNNING_GRAB_EXTRA_STARTUP_S)).abs() < 1e-6,
            "recovery did not shift with the swing"
        );
        assert!(
            (rr_end - running.duration_s).abs() < 1e-6,
            "the move outlives its own last window"
        );

        // Events and autocancel move too. A grab's effect fires at a point on
        // its timeline; unshifted, it would fire during the added windup.
        let mut timed = grab_shell("grab", "grab", 0.07, 0.05, 0.2);
        timed.events.push(crate::MoveEvent {
            at_s: 0.09,
            kind: crate::MoveEventKind::Sfx {
                cue: "reach".to_string(),
            },
        });
        timed.autocancel_after_s = Some(0.15);
        let derived = super::running_grab_from(&timed);
        assert!(
            (derived.events[0].at_s - (0.09 + super::RUNNING_GRAB_EXTRA_STARTUP_S)).abs() < 1e-6,
            "an authored event stayed where it was while the swing moved later,              so it now fires during the added wind-up; got {}",
            derived.events[0].at_s
        );
        assert!(
            (derived.autocancel_after_s.expect("carried")
                - (0.15 + super::RUNNING_GRAB_EXTRA_STARTUP_S))
                .abs()
                < 1e-6,
            "the autocancel point is measured from the move's start and did not              move with it"
        );
        // A move that authored neither keeps neither: the shift must not
        // create an autocancel from `None`.
        assert!(
            super::running_grab_from(&standing)
                .autocancel_after_s
                .is_none(),
            "a grab with no autocancel acquired one"
        );
    }

    /// An unauthored throw contributes no verb. A fighter with only a forward
    /// throw offers three capture verbs, not six. A press for a missing throw
    /// finds nothing; it does not fall through to a pummel.
    #[test]
    fn an_unauthored_throw_is_absent_rather_than_substituted() {
        let kit = SmashCaptureRepertoire {
            cues: CaptureCues::GENERIC,
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
            vec![
                verbs::GRAB,
                verbs::GRAB_DASH,
                verbs::PUMMEL,
                verbs::THROW_FORWARD
            ],
            "an absent throw invented a verb, or an authored one lost its"
        );
    }

    /// The verb names the row; the fighter's own clip stays behind it.
    ///
    /// Asking for the row must not replace the character's clip, or a sheet
    /// without the row would fall past its own art to `idle`.
    #[test]
    fn a_capture_beat_asks_for_its_verbs_row_before_the_authored_one() {
        let kit = SmashCaptureRepertoire {
            cues: CaptureCues::GENERIC,
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
        // `spec()` authors clip == id, which stands in for the fighter's choice.
        let chains: Vec<(String, Vec<String>)> = kit
            .bound()
            .into_iter()
            .map(|(_, m)| (m.clip.clip, m.clip.fallbacks))
            .collect();
        assert_eq!(
            chains
                .iter()
                .map(|(head, _)| head.as_str())
                .collect::<Vec<_>>(),
            // Two "grab" heads: the running grab is derived from the standing
            // one, so it asks for the same row.
            vec!["grab", "grab", "pummel", "throw_forward"],
            "a capture beat asked the sheet for a row its verb does not name"
        );
        // Found by name, not index, so inserting a beat cannot make the
        // assertion check a different beat.
        let throw = chains
            .iter()
            .find(|(head, _)| head == "throw_forward")
            .expect("the forward throw is bound");
        assert!(
            throw
                .1
                .starts_with(&["throw".to_string(), "fthrow".to_string()]),
            "the generic throw row or the fighter's own clip fell out of the chain: {:?}",
            throw.1
        );
    }
}

#[cfg(test)]
mod capture_cue_tests {
    use super::*;
    use crate::MoveEventKind;

    fn effects(spec: &MoveSpec) -> Vec<(f32, String)> {
        spec.events
            .iter()
            .filter_map(|e| match &e.kind {
                MoveEventKind::Vfx { effect, .. } => Some((e.at_s, effect.clone())),
                _ => None,
            })
            .collect()
    }

    fn kit(cues: CaptureCues) -> SmashCaptureRepertoire {
        SmashCaptureRepertoire {
            cues,
            grab: author_standing_grab(
                grab_shell("g", "attack", 0.07, 0.05, 0.2),
                CaptureAttemptParams {
                    offset: (12.0, 1.0),
                    half_extents: (18.0, 15.0),
                    hold_offset: (13.0, 3.0),
                },
            ),
            pummel: author_pummel(
                capture_beat("p", "attack", 0.18),
                0.08,
                CapturePummelParams { damage: 3 },
            ),
            forward_throw: author_throw(
                capture_beat("t", "attack", 0.26),
                0.14,
                CaptureThrowParams {
                    damage: 8,
                    knockback: 120.0,
                    knockback_growth: 2.0,
                    launch_dir: (0.85, -0.55),
                },
            ),
            back_throw: None,
            up_throw: None,
            down_throw: None,
        }
    }

    /// Every beat of a capture shows something, from the fighter's own sheet.
    #[test]
    fn every_capture_beat_carries_the_fighters_own_cue() {
        let bound = kit(CaptureCues {
            reach: "mine_reach",
            impact: "mine_impact",
            release: "mine_release",
        })
        .bound();

        let by_verb = |v: &str| -> Vec<(f32, String)> {
            effects(&bound.iter().find(|(verb, _)| *verb == v).unwrap().1)
        };

        // The grab's cue is on the reach (the first live frame, not zero), so
        // a whiff reads as an attempt at the moment it could have caught
        // somebody.
        assert_eq!(by_verb(verbs::GRAB), vec![(0.07, "mine_reach".to_string())]);
        // The pummel and the throw use the same instant as their gameplay
        // effect, so retuning a beat moves its flash too.
        assert_eq!(
            by_verb(verbs::PUMMEL),
            vec![(0.08, "mine_impact".to_string())]
        );
        assert_eq!(
            by_verb(verbs::THROW_FORWARD),
            vec![(0.14, "mine_release".to_string())]
        );
    }

    /// A generic kit gets the shipped rows, so fighters without a bespoke
    /// sheet still show their capture.
    #[test]
    fn a_generic_kit_still_shows_its_capture() {
        let bound = kit(CaptureCues::GENERIC).bound();
        for (verb, spec) in &bound {
            assert_eq!(
                effects(spec).len(),
                1,
                "`{verb}` carries {} cues, not one",
                effects(spec).len()
            );
        }
    }
}
