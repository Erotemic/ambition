//! Platform-fighter capture vocabulary: grab, pummel, and throw.
//!
//! Captures are relationships rather than damage hits: a grab acquires a target, later
//! moves operate on that selected counterpart, and a throw releases it at an authored
//! frame. Fighter code authors typed values through the helpers here; effect keys and
//! parameter encoding stay centralized. The generic move timeline sees ordinary
//! `MoveSpec` effect windows/events and does not learn capture-specific variants.

use ambition_entity_catalog::{EffectRef, MoveSpec, ParamValue, VolumeShape};
use serde::{Deserialize, Serialize};

/// The effect key an active grab window sustains.
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

/// THE THREE CUES A CAPTURE SHOWS, authored per fighter.
///
///  they are not constants here, and three guards are the reason. Carl,
/// Emmy and Oiler each assert that every effect in their kit is drawn off their
/// OWN sheet — so a shared `classic_burst` from `generic_explosions` is a real
/// violation of a property somebody chose, not a test being fussy. The helper
/// owns WHEN a cue fires; the fighter owns WHICH effect it is. Same split as the
/// repertoire: centralise the vocabulary, never the design.
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureCues {
    /// Fires on the grab's first LIVE frame, so a whiff reads as an attempt at
    /// the moment it could have caught somebody.
    pub reach: &'static str,
    /// Fires on the pummel's own `at_s`.
    pub impact: &'static str,
    /// Fires on the throw's RELEASE frame.
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
///  the reach is a RECT SPELLED OUT, not a [`VolumeShape`], and that is the
/// transport's rule rather than a preference. `ParamValue` stores authored
/// params as a `ron::Value`, whose deserializer cannot carry an enum — the type
/// says so in as many words (*"Enum-valued params are unsupported … model those
/// as string tags"*). A `VolumeShape` field serialises to `Rect(offset: …)` and
/// hydrates back as a bare map with the variant lost:
/// `InvalidValueForType { expected: "enum VolumeShape", found: "a map" }`.
/// Found by the round-trip assertion in this module's tests rather than by a
/// fighter's data failing at startup.
///
///  v1 grabs are rectangular. A circular grab is a real future want and the
/// documented shape for it is a string tag beside these fields; inventing that
/// tag now, for zero callers, is the generalisation nobody asked for.
/// [`Self::volume`] rebuilds the engine type at the consumer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct CapturePummelParams {
    /// Damage committed to the captive. No knockback, no hitstun, no
    /// post-hit invulnerability — the acquisition already happened, and a
    /// pummel that armed a hit reaction would release the very grab it belongs
    /// to.
    pub damage: i32,
}

/// Authored parameters of one throw release.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// The three-window shell a grab needs, so a fighter authors TIMINGS rather
/// than a window list.
///
///  it lives here rather than in either game because both providers need the
/// identical shape and because [`author_standing_grab`] REFUSES a move with no
/// Active window — a module that enforces a shape should be able to hand you
/// one. A fighter still owns every number.
///
///  no hit volume, ever. A grab's Active window carries a capture ATTEMPT, and
/// a volume beside it would make the same frames both grab and hit.
pub fn grab_shell(id: &str, clip: &str, startup_s: f32, active_s: f32, recover_s: f32) -> MoveSpec {
    let active_end = startup_s + active_s;
    MoveSpec {
        display_name: None,
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
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
    }
}

/// The shell a pummel or a throw needs: a timeline and nothing else.
///
/// Neither reaches for anybody — the target is already established — so neither
/// has an Active window or a volume. What each has is an INSTANT, attached by
/// [`author_pummel`] or [`author_throw`].
pub fn capture_beat(id: &str, clip: &str, duration_s: f32) -> MoveSpec {
    MoveSpec {
        display_name: None,
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
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
    }
}

/// Extra STARTUP a running grab pays over the standing one — the wind-up of
/// reaching out while already moving. Two frames at 60Hz.
const RUNNING_GRAB_EXTRA_STARTUP_S: f32 = 2.0 / 60.0;
/// Extra RECOVERY a running grab pays. **This is the whole trade** and it is
/// why the genre's dash grab is a commitment: whiffing one out of a run is
/// punishable in a way whiffing a standing grab is not. Twelve frames at 60Hz.
const RUNNING_GRAB_EXTRA_RECOVERY_S: f32 = 12.0 / 60.0;

/// **A fighter's RUNNING grab, derived from its own standing grab.**
///
/// ⭐ **derived rather than authored, and the genre is the reason.** A dash
/// ATTACK is a different move — a shoulder charge where the jab was — so each
/// fighter authors one and `SmashRepertoire` makes it a required slot. A dash
/// GRAB is the same reach-out performed while running: same clip, same catch,
/// slower to start and much slower to end. Deriving it means every fighter gets
/// one in its own timing, with no per-fighter number anybody had to invent and
/// no slot for a new fighter to forget.
///
/// ⚠ **it takes the extra time in SECONDS, not as a ratio.** The genre states
/// these as frame counts, and a multiplier would punish a fast grab less than a
/// slow one — the opposite of a commitment that is supposed to cost the same
/// wherever you spend it.
///
/// The windows shift as a body: everything after the startup moves later by the
/// added wind-up, and recovery alone stretches by the added endlag.
fn running_grab_from(standing: &MoveSpec) -> MoveSpec {
    // ⛔ EXHAUSTIVE on purpose. This derivation rewrites a move's TIMELINE, and
    // every absolute time on it has to move together or the move desynchronises
    // from itself. Destructuring means a `MoveSpec` that grows a new field
    // stops this compiling and asks whether the new field is a POINT on the
    // timeline (shift it) or a DURATION owed elsewhere (leave it) — rather than
    // being silently left behind, which is how `events` and `autocancel_after_s`
    // were missed the first time.
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
        // A charge policy carries BOTH kinds of value: `hold_at_s` is a point
        // on the timeline and shifts with the added startup, `max_hold_s` is a
        // duration owed at that point and does not.
        smash_charge,
        // NEITHER a point nor a duration: which BUTTON holds the charge does
        // not move when the timeline does.
        charge_gesture,
        // A LOOP is a stretch of the timeline, so both of its ends are points
        // and both shift with the added startup.
        repeat,
        // A duration OWED after landing, not a point in the move. It does not
        // move when the move gets longer.
        landing_lag_s,
        // A point measured from the move's start, so it moves with the rest.
        autocancel_after_s,
        // NEITHER: a RATE. How fast the sprite mirrors while the move plays does
        // not change because the move got a longer windup, and it is
        // presentation besides — a derived running grab draws the way the
        // standing one does.
        sprite_spin_hz,
    } = standing.clone();
    let mut running = MoveSpec {
        // A derived move never inherits a hand-written label: the standing
        // grab's would name the wrong beat in a prompt.
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
        smash_charge: smash_charge.map(|policy| ambition_entity_catalog::SmashChargeSpec {
            hold_at_s: policy.hold_at_s + RUNNING_GRAB_EXTRA_STARTUP_S,
            ..policy
        }),
        repeat: repeat.map(|l| ambition_entity_catalog::MoveLoop {
            from_s: l.from_s + RUNNING_GRAB_EXTRA_STARTUP_S,
            to_s: l.to_s + RUNNING_GRAB_EXTRA_STARTUP_S,
            // A DURATION, not a point: how long the loop may run does not
            // change because the move got a longer windup.
            ..l
        }),
        landing_lag_s,
        autocancel_after_s: autocancel_after_s.map(|at| at + RUNNING_GRAB_EXTRA_STARTUP_S),
        sprite_spin_hz,
    };
    // Whatever the author placed on the swing happens at the same point IN the
    // swing, which is now later. An event left at its original time would fire
    // during the startup this derivation added.
    for event in &mut running.events {
        event.at_s += RUNNING_GRAB_EXTRA_STARTUP_S;
    }
    for w in &mut running.windows {
        match w.tag {
            ambition_entity_catalog::WindowTag::Startup => {
                w.end_s += RUNNING_GRAB_EXTRA_STARTUP_S;
            }
            ambition_entity_catalog::WindowTag::Recovery => {
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
///  it sustains rather than firing once, and it attaches to every window the
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

/// The cue a capture beat carries, owned here for the reason the effect keys
/// are: a fighter authors VALUES, and the strings stay in one module.
///
/// Naming the effect is naming the cue.
fn burst(mut spec: MoveSpec, at_s: f32, effect: &str, scale: f32) -> MoveSpec {
    spec.events.push(ambition_entity_catalog::MoveEvent {
        at_s,
        kind: ambition_entity_catalog::MoveEventKind::Vfx {
            effect: effect.to_string(),
            at: (0.0, 0.0),
            scale,
            sfx: None,
        },
    });
    spec
}

/// The cue for a beat that already carries a gameplay `Effect`: it rides the
/// SAME instant, so retuning the beat cannot leave its flash behind.
fn cue_at_effect(spec: MoveSpec, effect: &str, scale: f32) -> MoveSpec {
    let at = spec
        .events
        .iter()
        .find(|e| matches!(e.kind, ambition_entity_catalog::MoveEventKind::Effect(_)))
        .map(|e| e.at_s)
        .unwrap_or(0.0);
    burst(spec, at, effect, scale)
}

/// The grab's cue, on the first LIVE frame rather than at zero.
fn cue_at_reach(spec: MoveSpec, effect: &str) -> MoveSpec {
    let at = spec
        .windows
        .iter()
        .find(|w| w.tag == ambition_entity_catalog::WindowTag::Active)
        .map(|w| w.start_s)
        .unwrap_or(0.0);
    burst(spec, at, effect, 0.45)
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
///  the release is a timeline instant, not the button press. The captive stays
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

/// why this is not on [`CapturedBy`](ambition_platformer2d::combat::capture::CapturedBy) any more. That
/// component is the RELATION: who holds whom, where, and what physical state release must give
/// back. Every field of it is answerable without knowing what genre is being played.
///
///  they were fine on the relation while the mechanic was being proven, and
/// they are not convincing final owners. The split is not cosmetic: it is why a
/// capture in another game does not pay to rewind a pummel counter it has no
/// rule for.
///
///  it rides BESIDE `CapturedBy` on the captive, and its lifetime is that
/// component's. A hold with no `SmashHoldState` is a hold this ruleset has no
/// opinion about, which is the honest reading for a game that constrains bodies
/// without pummelling them.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct SmashHoldState {
    pub pummels_landed: u8,
    /// How long this hold has lasted, in the same scaled seconds a move
    /// timeline advances in — so a capture does not age during hitstop.
    ///
    /// Without an age, a fighter who grabs and then does nothing holds a body for the rest of
    /// the match.
    pub held_for: f32,
    /// What the captive's OWN input has bought toward getting out, in the
    /// same seconds [`Self::held_for`] counts.
    ///
    ///  the shape matters more than the number. A captive is not a body
    /// whose input ceased to exist — it is a body whose input reaches a
    /// restricted channel, and this is that channel's accumulator.
    pub mash_credit: f32,
    /// How long THIS hold lasts, decided when it began.
    ///
    ///  stored rather than recomputed, and that is the genre's rule rather
    /// than a caching trick. Ultimate reads the captive's percent AT THE GRAB;
    /// a hold that re-read it every tick would grow every time its captor
    /// pummelled, which turns a pummel from a decision into a free extension of
    /// the advantage you already have.
    pub escape_seconds: f32,
    /// Has the captor's stick returned to NEUTRAL since this hold began?
    ///
    /// ⛔⛔ A DIRECTION ALONE THROWS, SO IT HAS TO BE A NEW DIRECTION. You walk
    /// into a grab, so the stick that reached it is usually already pointing
    /// somewhere — reading the live axis on the first held tick threw the
    /// victim instantly, before the captor could pummel or choose.
    ///
    /// ⭐ ARMED BY NEUTRAL rather than by remembering the direction at capture:
    /// a captor who grabs holding forward and keeps holding forward has not
    /// pressed anything, and one who centres and pushes forward again has —
    /// same final direction, different input.
    ///
    /// ⛔ AND IT LIVES HERE, NOT ON `CapturedBy`, which is the whole reason this
    /// component exists. "Centre the stick before a direction throws" is a
    /// platform-fighter INPUT rule, not a fact about who holds whom or what
    /// release must restore. A game that constrains bodies without a throw
    /// vocabulary should not pay to rewind this, exactly as it does not pay to
    /// rewind `pummels_landed`.
    pub throw_armed: bool,
}

impl SmashHoldState {
    /// A fresh hold that lasts `escape_seconds`.
    ///
    ///  the only way to start one, and `Default` is not it. A default row
    /// has `escape_seconds == 0.0`, which [`Self::escaped`] correctly reads as a
    /// hold already over — so a fixture that reached for `default()` would watch
    /// its capture end on tick one and call that a timeout.
    pub fn lasting(escape_seconds: f32) -> Self {
        Self {
            escape_seconds,
            ..Default::default()
        }
    }

    /// Is this hold over? The ONE place the two clocks are compared, so no
    /// caller can end a hold by half the rule.
    pub fn escaped(&self) -> bool {
        self.held_for + self.mash_credit >= self.escape_seconds
    }
}

/// A fighter's capture kit.
///
///  the three throws beyond forward are `Option` DURING THE MIGRATION. When the roster is
/// migrated and capture is part of the required Smash contract, these lose their `Option` the
/// same way any other slot would.
///
///  an unauthored throw does NOTHING, and deliberately does not fall back
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
    /// What this fighter's capture SHOWS. [`CaptureCues::GENERIC`] for a
    /// fighter whose art is generic; its own rows for one whose kit guards that
    /// every effect comes off its own sheet.
    pub cues: CaptureCues,
}

/// The verb a capture move answers to. The one place these strings exist.
///
///  they are not a directional family of `grab`. `capture_throw_forward` is
/// selected by the ATTACK press inside a capture relationship, not by a
/// directional grab press — so naming them `grab_forward` would invite the
/// action scheme's directional-verb matcher to light the Grab slot up for a
/// fighter that authored only throws.
pub mod verbs {
    //!  re-exports, not a second definition. The strings live beside
    //! `ATTACK_VERB` and `SMASH_VERB` in `ambition_entity_catalog` because the
    //! move SELECTOR has to resolve them and that crate is the one both the
    //! selector and this authoring module can see. Spelling them again here
    //! would be two places for a typo to become a press that does nothing.
    pub use ambition_entity_catalog::{
        CAPTURE_PUMMEL_VERB as PUMMEL, CAPTURE_THROW_BACK_VERB as THROW_BACK,
        CAPTURE_THROW_DOWN_VERB as THROW_DOWN, CAPTURE_THROW_FORWARD_VERB as THROW_FORWARD,
        CAPTURE_THROW_UP_VERB as THROW_UP, GRAB_DASH_VERB as GRAB_DASH, GRAB_VERB as GRAB,
    };
}

/// The VOCABULARY's sprite row for a capture beat, asked for FIRST.
///
///  here rather than inside each `author_*` helper, for exactly the reason the
/// cues are: [`SmashCaptureRepertoire::bound`] is the one place that already
/// knows which VERB a beat answers to, so a fighter cannot author a pummel and
/// forget to ask for the pummel row. The rows are the ones the fighter rigs
/// draw — `grab`, `pummel`, `throw_forward`/`_back`/`_up`/`_down` — and every
/// character asking for `attack` instead is why a throw and a jab looked the
/// same.
///
///  the character's own clip is KEPT, one step down the chain: a sheet with a
/// bespoke row still draws it, and a sheet with only `attack` still lands there.
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
    spec.clip = ambition_entity_catalog::ClipBinding {
        clip: chain.next().unwrap_or_default(),
        fallbacks: chain.collect(),
    };
    spec
}

impl SmashCaptureRepertoire {
    /// The `(verb, spec)` rows this kit contributes to a moveset contract.
    ///
    /// Every capture move is GROUNDED for v1: aerial and command grabs are named
    /// future techniques, and a grab that answered an airborne press would be one
    /// of them by accident.
    ///  public because a contract assembled BY HAND still needs the one
    /// verb mapping. `SmashRepertoire::into_contract` is the usual road, and a
    /// table that builds its `MovesetContract` directly would otherwise copy the
    /// verb names — the copy that drifts the day one is renamed.
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
        //  the cues land HERE, in the one place that already walks every
        // beat, rather than inside each `author_*` helper. A fighter cannot
        // author a throw and forget its release flash, and there is no second
        // site to keep in agreement.
        // The running grab is DERIVED here, from the standing grab this fighter
        // authored, so it picks up that fighter's own timing and — being built
        // before the cue is applied below — its own reach flash too.
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
    use ambition_entity_catalog::{ClipBinding, MoveWindow, WindowTag};

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
            charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
            repeat: None,
            landing_lag_s: None,
            autocancel_after_s: None,
            sprite_spin_hz: None,
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

    /// THE ATTEMPT IS LIVE FOR THE WHOLE ACTIVE WINDOW, AND ONLY THERE.
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

    /// A GRAB WITH NO ACTIVE WINDOW IS CAUGHT AT AUTHORING TIME.
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

    /// AN UNAUTHORED THROW CONTRIBUTES NO VERB.
    ///
    /// The v1 migration rule, stated as a test: a fighter with only a forward
    /// throw offers three capture verbs and not six. A press for a throw it does
    /// not have must find nothing — NOT fall through to a pummel, which would
    /// tell the player the fighter has a bad up-throw when it has none.
    /// **The running grab is the fighter's OWN grab, later and longer.**
    ///
    /// ⛔ this is the assertion that makes the derivation honest rather than a
    /// second invented move: every window keeps its shape, the catch happens at
    /// the same point in the swing, and the ONLY additions are the wind-up and
    /// the endlag — the endlag being the trade the genre actually charges for
    /// grabbing out of a run.
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

        let find = |spec: &MoveSpec, tag: ambition_entity_catalog::WindowTag| {
            spec.windows
                .iter()
                .find(|w| w.tag == tag)
                .map(|w| (w.start_s, w.end_s))
                .expect("window present")
        };
        use ambition_entity_catalog::WindowTag;
        let (_, s_end) = find(&standing, WindowTag::Startup);
        let (r_s_start, r_s_end) = find(&running, WindowTag::Startup);
        assert!(r_s_start.abs() < 1e-6, "the wind-up still begins at zero");
        assert!(
            (r_s_end - (s_end + super::RUNNING_GRAB_EXTRA_STARTUP_S)).abs() < 1e-6,
            "the wind-up did not lengthen by the stated startup"
        );

        // ⛔ the ACTIVE window keeps its LENGTH — a running grab catches for just
        // as long, it simply catches later. A derivation that stretched it would
        // be a better grab, not a committed one.
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

        // ⛔ **EVENTS AND AUTOCANCEL MOVE TOO**, and they are the half the
        // derivation forgot. A grab's own effect fires at a point on its
        // timeline; leaving that point where it was while the swing slides later
        // fires it during the wind-up this derivation ADDED — the attempt going
        // live before the hand has reached. Nothing ships that shape today only
        // because the cue is applied after derivation, which makes this a trap
        // for the next author rather than a live bug.
        let mut timed = grab_shell("grab", "grab", 0.07, 0.05, 0.2);
        timed.events.push(ambition_entity_catalog::MoveEvent {
            at_s: 0.09,
            kind: ambition_entity_catalog::MoveEventKind::Sfx {
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
        //  a move that authored NEITHER keeps neither -- the shift must not
        // invent an autocancel out of `None`.
        assert!(
            super::running_grab_from(&standing)
                .autocancel_after_s
                .is_none(),
            "a grab with no autocancel acquired one"
        );
    }

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

    /// THE VERB NAMES THE ROW; THE FIGHTER'S OWN CLIP SURVIVES BEHIND IT.
    ///
    ///  every shipped fighter authored `attack` for its pummel and its throws,
    /// so a throw and a jab drew the same picture — while the sheets have carried
    /// `pummel` and `throw_forward` the whole time.  both halves: asking for the
    /// row is worthless if it REPLACES what a character chose, since a sheet
    /// without the row would then fall past its own art to `idle`.
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
            // ⚠ TWO "grab" heads: the running grab is derived from the standing
            // one, so it asks the sheet for the same row — which is the point of
            // deriving it rather than making every fighter author a second clip.
            vec!["grab", "grab", "pummel", "throw_forward"],
            "a capture beat asked the sheet for a row its verb does not name"
        );
        // ⛔ found BY NAME, not by index. This read `chains[2]` until the running
        // grab was inserted ahead of it, at which point a positional assertion
        // would have quietly started checking a different beat's chain.
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
    use ambition_entity_catalog::MoveEventKind;

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

    /// EVERY BEAT OF A CAPTURE SHOWS SOMETHING, AND SHOWS THE FIGHTER'S OWN.
    ///
    /// Capture beats carry authored cues from the fighter's own sheet so grab
    /// state is visible without introducing shared presentation rows.
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

        //  the grab's cue sits on the REACH — the first LIVE frame, not zero —
        // so a whiff reads as an attempt at the moment it could have caught
        // somebody, which is the punish window a player has to learn.
        assert_eq!(by_verb(verbs::GRAB), vec![(0.07, "mine_reach".to_string())]);
        // The pummel and the throw ride the SAME instant as their own gameplay
        // effect, so retuning a beat cannot leave its flash behind.
        assert_eq!(
            by_verb(verbs::PUMMEL),
            vec![(0.08, "mine_impact".to_string())]
        );
        assert_eq!(
            by_verb(verbs::THROW_FORWARD),
            vec![(0.14, "mine_release".to_string())]
        );
    }

    /// AND A GENERIC KIT GETS THE SHIPPED ROWS, so the eleven fighters with
    /// no bespoke sheet are not left silent to keep the three honest.
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
