//! Device-agnostic per-frame control vocabulary.
//!
//! `ControlFrame` is the brain-facing snapshot a controller, replay, netcode
//! peer, touch bridge, or scripted test emits for one actor-control slot. The
//! device adapters that *build* it live above this crate (`ambition_input`,
//! touch, replay, etc.); the body/brain contracts that *consume* it live here
//! and in `ambition_characters`. Keeping the struct at the engine foundation
//! prevents controller vocabulary from leaking upward into reusable character
//! brains.

use bevy_ecs::prelude::Resource;

use crate::RawDirectionEdges;

/// What a device asks the attack interpreter to make of this press.
///
/// ⭐⭐ THREE-VALUED BECAUSE THE OLD BOOL WAS ONE-WAY. `attack_strong_hint` could
/// force a Smash and nothing could force a Tilt, so on a tilt-stick — a
/// right-stick mode whose whole purpose is throwing tilts at full deflection —
/// the deflection itself registered as a flick and the interpreter returned
/// Smash anyway. A full deflection could never be a tilt.
///
/// ⛔ THE ALTERNATIVE WAS SPOOFING STICK HISTORY, and it is the reason this is a
/// hint rather than a device trick: a right-stick adapter that wanted a tilt
/// would have had to feed the interpreter a deflection small enough not to arm
/// the flick, which throws away the direction the player actually pushed.
///
/// Device-independent. A C-stick adapter, a dedicated key, a replay, a remote
/// peer or an RL policy may set it; the multi-tick flick history stays sim-side
/// and is never streamed as resolved state.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum AttackStrengthHint {
    /// Let the interpreter decide from the stick's own flick history. What every
    /// ordinary attack button asks for.
    #[default]
    Auto,
    /// This press is a TILT however hard the stick is pushed.
    Tilt,
    /// This press is a SMASH even with no flick behind it.
    Smash,
}

impl AttackStrengthHint {
    /// Fold two samples of one tick.
    ///
    /// ⭐ AN EXPLICIT HINT BEATS `Auto`, and `Smash` beats `Tilt` — which is the
    /// old `|` on the bool this replaced, extended rather than reinterpreted. A
    /// tick that saw both sticks is a tick the player pressed both, and the
    /// stronger reading is the one that cannot be recovered by pressing again.
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Smash, _) | (_, Self::Smash) => Self::Smash,
            (Self::Tilt, _) | (_, Self::Tilt) => Self::Tilt,
            (Self::Auto, Self::Auto) => Self::Auto,
        }
    }
}

/// Per-frame snapshot of semantic controller input feeding actor brains.
///
/// This is intentionally device-free: there are no keyboards, gamepads,
/// touch events, or Leafwing actions here. Visible builds populate it from
/// `ambition_input`; headless tests, replay, and future netcode can populate it
/// directly.
///
/// Local-primary adapter: the host still keeps one global `ControlFrame` resource at the
/// device edge for the local primary input, then publishes that finalized value into
/// `ambition_platformer2d::characters::control::SlotControls`. The type itself is slot-neutral; no copy is
/// stored on the controlled body. This is why adding a `ControlFrame` field does not bump
/// `INPUT_STREAM_VERSION`.
#[derive(
    Resource, Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize,
)]
#[serde(default)]
pub struct ControlFrame {
    pub axis_x: f32,
    pub axis_y: f32,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub jump_released: bool,
    /// Rising edge on the SHARED burst press — the dodge/dash channel. What a
    /// press BUYS is the body's answer (`BurstManeuver`), not this field's.
    #[serde(rename = "dash_pressed")]
    pub burst_pressed: bool,
    /// Movement-left input was newly pressed this frame in the raw input/screen
    /// frame. Directional gameplay gestures resolve this through
    /// `AccelerationFrame` before treating it as local left/right/up/down.
    pub left_pressed: bool,
    /// Movement-right input was newly pressed this frame in the raw input/screen
    /// frame.
    pub right_pressed: bool,
    /// Movement-up input was newly pressed this frame in the raw input/screen
    /// frame. The sandbox resolves this with the current controlled-body frame
    /// before using it for local-up gestures.
    pub up_pressed: bool,
    /// Movement-down input was newly pressed this frame in the raw input/screen
    /// frame. The sandbox resolves this with the current controlled-body frame
    /// before using it for local-down gestures.
    pub down_pressed: bool,
    /// Double-tap-down recognized by the sandbox input gesture detector.
    pub fast_fall_pressed: bool,
    pub blink_pressed: bool,
    pub blink_held: bool,
    pub blink_released: bool,
    /// Dedicated signature-SPECIAL slot (`Platformer2dInputActionMonolith::Special`). Distinct from
    /// blink: the player brain sources `special_pressed` from THIS, retiring the
    /// historical `special_pressed = blink_pressed` alias.
    pub special_pressed: bool,
    /// Special button LEVEL, preserved beside the edge for the same reason
    /// [`Self::attack_held`] is: a move can be held.
    ///
    /// A chargeable neutral special — the genre's charge shot — freezes its own
    /// timeline while this is down and fires on the release. The edge alone
    /// cannot say that: by the time the charge is accruing, the press it started
    /// from is several ticks old.
    pub special_held: bool,
    /// Attack button rising edge.
    pub attack_pressed: bool,
    /// Attack button level. Preserved independently from the edge so a future
    /// move interpreter can distinguish tap, charge, and sustained attacks.
    pub attack_held: bool,
    /// Attack button falling edge.
    pub attack_released: bool,
    /// What this press asks the attack interpreter to make of it — see
    /// [`AttackStrengthHint`].
    pub attack_strength_hint: AttackStrengthHint,
    /// This attack's DIRECTION came from the aim stick, not the movement stick.
    ///
    /// ⭐ A C-STICK ATTACK POINTS WHERE THE RIGHT STICK WENT, and the left stick
    /// keeps meaning "walk". Without this the two roles collide: `attack_axis`
    /// is the movement axis, so a right-stick attack would come out in whatever
    /// direction the player happened to be running.
    ///
    /// ⛔ A BOOL BECAUSE THE QUESTION REALLY HAS TWO ANSWERS — which of the two
    /// sticks aimed this press. A third source would make it a three-state
    /// field, and the field would have to change shape rather than alias.
    ///
    /// ⚠ MEANINGLESS WITHOUT `attack_pressed`. It qualifies a press; a frame
    /// with no press carries `false` and says nothing.
    pub attack_from_aim_stick: bool,
    /// The direction the C-stick was flicked, in SCREEN axes — the press's own
    /// direction, carried WITH the press.
    ///
    /// ⛔⛔ NOT `aim_x`/`aim_y`, AND THAT IS THE WHOLE POINT. The aim pair is a
    /// LEVEL: `merge_sample` takes the newest sample, so a stick already back at
    /// rest by the next device frame zeroed it — while `attack_pressed`,
    /// `attack_strength_hint` and `attack_from_aim_stick` are EDGES and survived.
    /// The press arrived armed and pointing nowhere, and `attack_axis` then fell
    /// back to the MOVEMENT axis: a flick right while running left threw the
    /// attack LEFT. A direction that qualifies an edge has to be latched like
    /// one.
    ///
    /// ⚠ MEANINGLESS WITHOUT `attack_from_aim_stick`, which is what says a press
    /// took its direction from this pair rather than from the movement stick.
    pub attack_aim_x: f32,
    /// See [`Self::attack_aim_x`]. Screen axes, so +Y is DOWN like every other
    /// axis pair on this frame.
    pub attack_aim_y: f32,
    /// How this seat's sticks map onto the controlled body's local frame.
    ///
    /// ⛔⛤ **IT TRAVELS WITH THE INPUT BECAUSE IT IS PART OF WHAT THE INPUT
    /// MEANS.** It lived in `SeatControlFrameModes`, a resource written from
    /// `Update` and never rollback-registered, so a resimulation of frame N read
    /// whatever the settings UI held NOW — "projected" is not "admitted", and a
    /// projection narrows who reads a mutable value while leaving the timeline
    /// question untouched.
    ///
    /// ⚠ THE OTHER REPAIR WAS TO RESOLVE THE DIRECTION AT CAPTURE and let the
    /// simulation see no mode at all. It is still available after this change
    /// and was NOT chosen here, because it is not behaviour-preserving:
    /// `resolve_input` reads the CURRENT gravity basis for two of its three
    /// modes (`ScreenRelative` projects onto `side`/`down`, `BodyRelativeAssist`
    /// reads `down.y`), and both shipped defaults are `ScreenRelative` under a
    /// `WorldFixed` camera. Baking a direction at capture would resolve the
    /// default player's gestures against a stale basis at every gravity flip,
    /// against that mode's own contract — "the body moves the way the stick
    /// points ON SCREEN at any gravity". Carrying the MODE keeps the basis live
    /// and changes no behaviour; it only makes the mode deterministic.
    pub control_frame_modes: crate::ControlFrameModes,
    pub pogo_pressed: bool,
    pub fly_toggle_pressed: bool,
    /// Generic context interaction. This is a dedicated interact action plus
    /// the sandbox double-tap-up gesture, not raw held/up movement.
    pub interact_pressed: bool,
    /// Interact button currently HELD (sustain), distinct from the
    /// `interact_pressed` rising edge. Hold gestures (e.g. possession's
    /// ~2s Down+Interact) accumulate on this; single-shot interactions
    /// (doors / heal-shrine) use the edge so one press = one action.
    pub interact_held: bool,
    pub reset_pressed: bool,
    pub start_pressed: bool,
    /// Player projectile / spell action — newly pressed this frame.
    pub projectile_pressed: bool,
    /// Player projectile button is currently held. Used by the
    /// fireball charge mechanic to accumulate hold time. Whenever
    /// the button is held, the charge timer ticks; release-edge
    /// (`projectile_released`) commits the charged shot.
    pub projectile_held: bool,
    /// Player projectile button was released this frame. Triggers
    /// the actual fireball spawn when a charge was in progress.
    pub projectile_released: bool,
    /// Shield button is currently held. Maps to the input adapter's quick-action
    /// verb. While held with the `shield` ability active, the engine deploys the
    /// bubble and tracks the parry window.
    pub shield_held: bool,
    /// Grab button rising edge. One press = one capture attempt.
    ///
    /// an EDGE and not a level, unlike [`Self::shield_held`] beside it, and
    /// the asymmetry is the mechanic: a shield is a state you sustain, a grab is
    /// an attempt you commit. Holding the button must not re-attempt every tick
    /// — the authored grab move owns how long the attempt stays active, and its
    /// recovery is what a whiffed grab costs.
    pub grab_pressed: bool,
    /// Rising edge: this body wants to TAUNT this tick. A taunt costs the
    /// body its footing and buys it nothing, which is the whole point.
    pub taunt_pressed: bool,
    /// Modifier slot currently HELD (sustain). The device layer reports the raw
    /// button state and assigns it NO meaning: content decides what sustaining
    /// this slot does to a body (a locomotion technique, a stance, a guard).
    /// Carried as a level so a sustained technique survives the frame→tick latch
    /// and reads identically under replay and rollback.
    pub modifier_held: bool,
    /// Modifier slot newly pressed this frame — the rising edge of the same
    /// button whose sustain is [`ControlFrame::modifier_held`]. Content may bind a
    /// momentary action to the edge while the hold drives a technique.
    pub modifier_pressed: bool,
    /// Right stick / aim vector after deadzone is applied. Blink aim and any
    /// future twin-stick aiming should consume this instead of reading raw axes.
    pub aim_x: f32,
    pub aim_y: f32,
}

impl ControlFrame {
    pub fn raw_direction_edges(self) -> RawDirectionEdges {
        RawDirectionEdges::new(
            self.left_pressed,
            self.right_pressed,
            self.up_pressed,
            self.down_pressed,
        )
    }

    /// Merge a newer device sample into `self`, the frame accumulated so far
    /// within one sim tick.
    ///
    /// Levels (axes, aim, every `*_held`) take the LATEST sample: a stick
    /// released mid-tick is released. Edges (every `*_pressed` /
    /// `*_released`) OR together, so a tap that begins and ends between two sim
    /// ticks is never swallowed.
    #[must_use]
    pub fn merge_sample(self, sample: ControlFrame) -> ControlFrame {
        ControlFrame {
            // Levels — latest wins. The frame-mode policy is a level and not an
            // edge: it is the seat's standing preference, so two device samples
            // inside one tick cannot disagree about it in practice, and if they
            // ever did the later one is the one the human just chose.
            control_frame_modes: sample.control_frame_modes,
            axis_x: sample.axis_x,
            axis_y: sample.axis_y,
            aim_x: sample.aim_x,
            aim_y: sample.aim_y,
            jump_held: sample.jump_held,
            blink_held: sample.blink_held,
            interact_held: sample.interact_held,
            projectile_held: sample.projectile_held,
            shield_held: sample.shield_held,
            modifier_held: sample.modifier_held,
            attack_held: sample.attack_held,
            special_held: sample.special_held,
            // Edges — sticky until a tick consumes them.
            jump_pressed: self.jump_pressed | sample.jump_pressed,
            jump_released: self.jump_released | sample.jump_released,
            burst_pressed: self.burst_pressed | sample.burst_pressed,
            left_pressed: self.left_pressed | sample.left_pressed,
            right_pressed: self.right_pressed | sample.right_pressed,
            up_pressed: self.up_pressed | sample.up_pressed,
            down_pressed: self.down_pressed | sample.down_pressed,
            fast_fall_pressed: self.fast_fall_pressed | sample.fast_fall_pressed,
            blink_pressed: self.blink_pressed | sample.blink_pressed,
            blink_released: self.blink_released | sample.blink_released,
            special_pressed: self.special_pressed | sample.special_pressed,
            attack_pressed: self.attack_pressed | sample.attack_pressed,
            attack_released: self.attack_released | sample.attack_released,
            attack_strength_hint: self.attack_strength_hint.merge(sample.attack_strength_hint),
            attack_from_aim_stick: self.attack_from_aim_stick | sample.attack_from_aim_stick,
            // ⭐ THE DIRECTION RIDES ITS EDGE. Captured from the sample that
            // carries the C-stick press and never overwritten by a later
            // neutral one — the newest PRESS wins, which is the same rule
            // `attack_strength_hint` follows, not the newest SAMPLE.
            attack_aim_x: if sample.attack_from_aim_stick {
                sample.attack_aim_x
            } else {
                self.attack_aim_x
            },
            attack_aim_y: if sample.attack_from_aim_stick {
                sample.attack_aim_y
            } else {
                self.attack_aim_y
            },
            pogo_pressed: self.pogo_pressed | sample.pogo_pressed,
            fly_toggle_pressed: self.fly_toggle_pressed | sample.fly_toggle_pressed,
            interact_pressed: self.interact_pressed | sample.interact_pressed,
            reset_pressed: self.reset_pressed | sample.reset_pressed,
            start_pressed: self.start_pressed | sample.start_pressed,
            projectile_pressed: self.projectile_pressed | sample.projectile_pressed,
            projectile_released: self.projectile_released | sample.projectile_released,
            modifier_pressed: self.modifier_pressed | sample.modifier_pressed,
            grab_pressed: self.grab_pressed | sample.grab_pressed,
            taunt_pressed: self.taunt_pressed | sample.taunt_pressed,
        }
    }

    /// The frame with every edge cleared and every level kept.
    ///
    /// What survives a tick's consumption of the latch: a held stick stays held
    /// into the next tick even if no new device sample arrived, while a press
    /// fires exactly once.
    #[must_use]
    pub fn levels_only(self) -> ControlFrame {
        ControlFrame {
            axis_x: self.axis_x,
            axis_y: self.axis_y,
            aim_x: self.aim_x,
            aim_y: self.aim_y,
            jump_held: self.jump_held,
            blink_held: self.blink_held,
            interact_held: self.interact_held,
            projectile_held: self.projectile_held,
            shield_held: self.shield_held,
            modifier_held: self.modifier_held,
            attack_held: self.attack_held,
            ..ControlFrame::default()
        }
    }
}

/// Latches frame-rate device samples until the next simulation tick.
///
/// Levels use the newest sample while edges accumulate. The first tick drains
/// accumulated edges; additional ticks before another device sample see levels
/// only, so sub-tick taps are neither lost nor repeated. Headless, replay, and
/// rollback paths author per-tick [`ControlFrame`] values directly and do not
/// use this latch.
///
/// This is one row of `ambition_platformer2d::characters::control::SlotControlLatches`, not a
/// standalone resource. Device publication and simulation consumption must stay
/// explicitly ordered.
#[derive(Clone, Copy, Debug, Default)]
pub struct ControlFrameLatch {
    accumulated: ControlFrame,
    /// Whether a DEVICE has ever fed this latch.
    ///
    /// STICKY, and that is the whole subtlety. The question is not "did a
    /// device speak this frame" — a tick that samples nothing must still hand
    /// over the retained levels, or a held direction sticks on forever. The
    /// question is "does this composition HAVE a device feeding this latch at
    /// all". Once one does, the latch speaks for the frame from then on.
    ///
    /// Silence is not a request (the same rule `drive_control_frame` learned the hard way about
    /// `PendingSeatInputs`).
    device_seen: bool,
}

impl ControlFrameLatch {
    /// Fold one device sample in. Levels overwrite; edges stick.
    pub fn accumulate(&mut self, sample: ControlFrame) {
        self.accumulated = self.accumulated.merge_sample(sample);
        self.device_seen = true;
    }

    /// Whether a device feeds this latch at all, so it speaks for the frame.
    ///
    /// A consumer that would OVERWRITE another writer's frame must ask this
    /// first: an untouched latch means "no device is wired to this latch", not
    /// "the device said nothing". Sticky by design — see the field.
    pub fn is_device_authority(&self) -> bool {
        self.device_seen
    }

    /// Hand the accumulated frame to a tick, retaining levels for the next one.
    pub fn take(&mut self) -> ControlFrame {
        let frame = self.accumulated;
        self.accumulated = frame.levels_only();
        frame
    }

    /// The frame a tick would currently take. Test/debug only.
    pub fn peek(&self) -> ControlFrame {
        self.accumulated
    }
}

#[cfg(test)]
mod latch_tests {
    use super::{AttackStrengthHint, ControlFrame, ControlFrameLatch};

    /// A tap that opens and closes between two ticks must still reach the sim.
    #[test]
    fn a_sub_tick_tap_survives_the_latch() {
        let mut latch = ControlFrameLatch::default();
        latch.accumulate(ControlFrame {
            jump_pressed: true,
            jump_held: true,
            ..ControlFrame::default()
        });
        // ... and the button is already back up by the next rendered frame.
        latch.accumulate(ControlFrame::default());

        let tick = latch.take();
        assert!(tick.jump_pressed, "the press edge must survive the release");
        assert!(
            !tick.jump_held,
            "but the level must reflect the LATEST sample"
        );
    }

    #[test]
    fn a_sub_tick_attack_tap_preserves_both_edges_and_latest_level() {
        let mut latch = ControlFrameLatch::default();
        latch.accumulate(ControlFrame {
            attack_pressed: true,
            attack_held: true,
            attack_strength_hint: AttackStrengthHint::Smash,
            ..ControlFrame::default()
        });
        latch.accumulate(ControlFrame {
            attack_released: true,
            ..ControlFrame::default()
        });

        let tick = latch.take();
        assert!(tick.attack_pressed);
        assert!(tick.attack_released);
        assert_eq!(tick.attack_strength_hint, AttackStrengthHint::Smash);
        assert!(!tick.attack_held);

        let next = latch.take();
        assert!(!next.attack_pressed);
        assert!(!next.attack_released);
        assert_eq!(next.attack_strength_hint, AttackStrengthHint::Auto);
        assert!(!next.attack_held);
    }

    /// ⛔⛔ A C-STICK FLICK THAT RECENTERS BEFORE THE TICK KEEPS ITS PRESS AND
    /// MUST KEEP ITS DIRECTION.
    ///
    /// The press, the strength and `attack_from_aim_stick` are EDGES and survive
    /// the recenter. The direction rode `aim_x`/`aim_y`, which are LEVELS —
    /// latest sample wins — so a stick already back at rest by the next device
    /// sample delivered an armed C-stick attack pointing NOWHERE.
    ///
    /// ⭐ THE LEFT STICK IS HELD THE OTHER WAY ON PURPOSE. With the aim zeroed,
    /// `player::attack_axis` falls back to the movement axis, so the failure is
    /// not "an attack with no direction" — it is an attack thrown in the
    /// OPPOSITE direction to the one the player flicked. A fixture with a
    /// neutral left stick would have measured a much smaller bug.
    #[test]
    fn a_sub_tick_c_stick_flick_keeps_the_direction_it_was_flicked() {
        let mut latch = ControlFrameLatch::default();
        // Running left, flick the C-stick RIGHT.
        latch.accumulate(ControlFrame {
            axis_x: -1.0,
            aim_x: 1.0,
            attack_pressed: true,
            attack_from_aim_stick: true,
            attack_aim_x: 1.0,
            attack_strength_hint: AttackStrengthHint::Tilt,
            ..ControlFrame::default()
        });
        // The stick is already back at rest by the next device sample; the left
        // stick is still held left.
        latch.accumulate(ControlFrame {
            axis_x: -1.0,
            ..ControlFrame::default()
        });

        let tick = latch.take();
        assert!(tick.attack_pressed, "the press edge survives");
        assert!(
            tick.attack_from_aim_stick,
            "and it is still a C-stick attack"
        );
        assert_eq!(tick.attack_strength_hint, AttackStrengthHint::Tilt);
        assert_eq!(
            (tick.attack_aim_x, tick.attack_aim_y),
            (1.0, 0.0),
            "the flick pointed RIGHT; a C-stick press must carry its own \
             direction, because the aim LEVEL is back at rest and the movement \
             axis points the other way"
        );
    }

    /// Levels are the latest sample, never an OR.
    #[test]
    fn levels_take_the_latest_sample() {
        let mut latch = ControlFrameLatch::default();
        latch.accumulate(ControlFrame {
            axis_x: 1.0,
            shield_held: true,
            ..ControlFrame::default()
        });
        latch.accumulate(ControlFrame {
            axis_x: -0.5,
            ..ControlFrame::default()
        });

        let tick = latch.take();
        assert_eq!(tick.axis_x, -0.5);
        assert!(!tick.shield_held);
    }

    /// When the sim runs several ticks inside one frame, one press fires once —
    /// but a held stick keeps holding.
    #[test]
    fn a_second_tick_in_the_same_frame_sees_levels_but_not_edges() {
        let mut latch = ControlFrameLatch::default();
        latch.accumulate(ControlFrame {
            axis_x: 1.0,
            attack_pressed: true,
            attack_held: true,
            jump_held: true,
            ..ControlFrame::default()
        });

        let first = latch.take();
        assert!(first.attack_pressed);
        assert!(first.attack_held);

        // No new device sample arrives before the catch-up tick.
        let second = latch.take();
        assert!(!second.attack_pressed, "one press must not fire twice");
        assert!(second.attack_held, "the attack level remains held");
        assert_eq!(second.axis_x, 1.0, "a held stick stays held");
        assert!(second.jump_held);
    }
}

#[cfg(test)]
mod latch_authority_tests {
    use super::*;

    /// An untouched latch is not a request for a neutral frame.
    ///
    /// The latch only becomes an authority over the tick's input once a device has fed it.
    #[test]
    fn a_latch_nobody_fed_is_not_an_authority() {
        let mut latch = ControlFrameLatch::default();
        assert!(
            !latch.is_device_authority(),
            "a fresh latch has heard from no device and must not claim the frame"
        );

        latch.accumulate(ControlFrame {
            jump_pressed: true,
            jump_held: true,
            ..ControlFrame::default()
        });
        assert!(
            latch.is_device_authority(),
            "a device sample makes it the authority"
        );

        let taken = latch.take();
        assert!(taken.jump_pressed, "the tick receives the accumulated edge");
        assert!(
            latch.is_device_authority(),
            "and it STAYS the authority: a tick that sampled nothing must still \
             receive the retained levels, or a held direction sticks on forever"
        );
    }

    /// A NEUTRAL sample still counts as a device speaking. "Nothing is pressed"
    /// is an answer, and a host that stops publishing it would leave the last
    /// held direction stuck on.
    #[test]
    fn a_neutral_sample_is_still_a_sample() {
        let mut latch = ControlFrameLatch::default();
        latch.accumulate(ControlFrame::default());
        assert!(
            latch.is_device_authority(),
            "a device that reports nothing pressed has still reported"
        );
    }
}

#[cfg(test)]
mod the_bytes_two_peers_exchange {
    //! ⛔⛤ **NOTHING VERSIONED THE SHAPE OF THE INPUT WIRE, AND EVERY CANDIDATE
    //! THAT LOOKED LIKE IT DID COVERS SOMETHING ELSE.**
    //!
    //! `AmbitionGgrsConfig = GgrsConfig<ControlFrame>`, so [`ControlFrame`] IS
    //! what crosses between peers. The state half of the wire has both an
    //! identity and a ratchet (`GGRS_ROLLBACK_SCHEMA_VERSION` and
    //! `scripts/tests/rollback_codec_shape.txt`). Before this module the input
    //! half had neither:
    //!
    //! - `INPUT_STREAM_VERSION` versions RECORDED REPLAY FILES and exempts added
    //!   fields **by design** — this very type's doc comment says so: *"adding a
    //!   `ControlFrame` field does not bump `INPUT_STREAM_VERSION`"*;
    //! - the rollback dump carries ONE row naming the type
    //!   (`derived.control_frame`) and not its fields;
    //! - the schema fingerprint hashes that dump, so it inherits the same blindness;
    //! - `rollback_codec_shape.txt` has zero mentions, because `ControlFrame` has
    //!   no `SnapshotState` impl at all — it is `derived`, rebuilt FROM the input
    //!   stream rather than snapshotted.
    //!
    //! ⇒ So this is a RATCHET, not a version: it cannot tell you what to bump,
    //! because there is nothing to bump yet. It makes a change to the peer input
    //! payload impossible to make silently, which is the property that was
    //! missing. See ID-PEER's fifteenth road in `docs/planning/queue.md`.
    //!
    //! ⚠ **LATENT, NOT HARMLESS.** Only `SyncTestSession` is built today
    //! (netcode `N2`), and a sync test has one peer, so no mismatch can be
    //! observed. The first real P2P session is where a silent shape change
    //! becomes two hosts that agree on every checksum and cannot decode each
    //! other's input.
    //!
    //! ⛔⛤ **AND FOR A DAY THIS MODULE PINNED PRETTY-PRINTED serde JSON AND
    //! `size_of`, CALLING THE FIRST "the exact payload serde puts on the wire".
    //! NEITHER IS THE WIRE.** Read out of the pinned `ggrs` checkout itself
    //! (`0.13.0`, git `e97e3d2`), not out of a changelog:
    //!
    //! ```text
    //! network/protocol.rs:68   bincode::serialized_size(&T::Input::default())
    //! network/protocol.rs:90   bincode::serialize_into(&mut bytes, &input.input)
    //! network/protocol.rs:117  bincode::deserialize(player_byte_slice)
    //! ```
    //!
    //! bincode is POSITIONAL: it writes fields in declaration order and carries
    //! no field names at all. So the old module's whole poison argument was
    //! false. **Measured 2026-09-16 by poison:** renaming the live
    //! `#[serde(rename = "dash_pressed")]` attribute to something else leaves
    //! this encoding **byte-identical** — same 68 bytes, same hex — while it
    //! reddened the JSON assertion the module was built around. And `size_of`
    //! was **60** against a serialized size of **68**, so the companion
    //! "neither implies the other" constant was pinning Rust's in-memory layout
    //! and calling it the transport.
    //!
    //! ⭐⛤ **WHAT THE COMPILER ALREADY CATCHES.** ADDING a field fails to
    //! compile: [`ControlFrame::merge_sample`] builds an EXHAUSTIVE literal, so a
    //! new field must also declare whether it is a LEVEL (latest wins) or an
    //! EDGE (OR-accumulated). That is about merge semantics, not about the peer,
    //! and satisfying it is one line — it says nothing of the wire.
    //!
    //! ⚠ **AND WHAT THESE BYTES CANNOT SEE, SO THAT THE NEXT READER DOES NOT
    //! TRUST THEM FOR IT.** `bool` and `u8` are both ONE byte in bincode and
    //! encode the same values identically, so a type swap between them moves
    //! nothing here. The FIELD TYPES are carried by the source-level census in
    //! `scripts/check_absence_contracts.py::input_payload_shape`, which records
    //! declaration order, each field's type, and the variants of every nested
    //! enum. The two are complements: that one is the shape, this one is the
    //! transport actually producing bytes.

    use super::{AttackStrengthHint, ControlFrame};
    use crate::{ControlFrameModes, InputFrameMode};

    /// A frame chosen so the ENCODING is legible, because the default one is
    /// not.
    ///
    /// ⛔ `bincode::serialize(&ControlFrame::default())` is sixty-eight ZERO
    /// bytes. Pinning that would catch a length change and nothing else: every
    /// falsy field is the same byte as every other, so a reordering among them,
    /// a changed enum discriminant and a swapped float would all leave it
    /// untouched. Every bool here alternates, the floats are distinct, and both
    /// enums take a NON-default variant — so field order, field width and each
    /// variant index are all visible in the hex below.
    fn a_frame_whose_encoding_is_legible() -> ControlFrame {
        ControlFrame {
            axis_x: -0.75,
            axis_y: 0.5,
            jump_pressed: true,
            jump_held: false,
            jump_released: true,
            burst_pressed: false,
            left_pressed: true,
            right_pressed: false,
            up_pressed: true,
            down_pressed: false,
            fast_fall_pressed: true,
            blink_pressed: false,
            blink_held: true,
            blink_released: false,
            special_pressed: true,
            special_held: false,
            attack_pressed: true,
            attack_held: false,
            attack_released: true,
            attack_strength_hint: AttackStrengthHint::Smash,
            attack_from_aim_stick: true,
            attack_aim_x: 0.25,
            attack_aim_y: -1.0,
            control_frame_modes: ControlFrameModes {
                movement: InputFrameMode::BodyRelativeStrict,
                aim: InputFrameMode::BodyRelativeAssist,
            },
            pogo_pressed: true,
            fly_toggle_pressed: false,
            interact_pressed: true,
            interact_held: false,
            reset_pressed: true,
            start_pressed: false,
            projectile_pressed: true,
            projectile_held: false,
            projectile_released: true,
            shield_held: false,
            grab_pressed: true,
            taunt_pressed: false,
            modifier_held: true,
            modifier_pressed: false,
            aim_x: 0.125,
            aim_y: -0.375,
        }
    }

    /// The bytes a peer decodes for [`a_frame_whose_encoding_is_legible`].
    ///
    /// Readable as the struct: `000040bf` is `-0.75` little-endian, the runs of
    /// `0100`/`0001` are the alternating bools, `02000000` is
    /// `AttackStrengthHint::Smash` (bincode writes a unit variant as a `u32`
    /// index), and `01000000 02000000` are the two frame modes.
    const RECORDED: &str = "000040bf0000003f010001000100010001000100010001000102000000010000803e000080bf010000000200000001000100010001000100010001000000003e0000c0be";

    /// What `ggrs` sizes the per-player input slot by — `serialized_size` of a
    /// DEFAULT frame, read at `network/protocol.rs:68`.
    ///
    /// ⛔ It is not `size_of::<ControlFrame>()`, which is 60. A previous version
    /// of this module pinned that number and described it as the wire.
    const RECORDED_WIRE_BYTES: u64 = 68;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn the_bytes_a_peer_decodes_are_pinned() {
        let bytes = bincode::serialize(&a_frame_whose_encoding_is_legible())
            .expect("a ControlFrame serializes");
        assert_eq!(
            hex(&bytes),
            RECORDED,
            "\nTHE PEER INPUT PAYLOAD CHANGED SHAPE.\n\n\
             `ControlFrame` is `GgrsConfig::Input`, and `ggrs` puts it on the \
             wire with bincode -- these are the bytes the other host decodes. \
             Nothing versions them: `INPUT_STREAM_VERSION` deliberately exempts \
             added fields (see this type's own doc comment), the rollback dump \
             names the type without its fields, and the schema fingerprint \
             hashes that dump.\n\n\
             bincode is POSITIONAL, so what moves this is a field added, \
             removed or REORDERED, a field's width changing, or an enum gaining \
             a variant before an existing one. A `#[serde(rename)]` does not: \
             there are no field names in these bytes.\n\n\
             If the change is deliberate: re-record `RECORDED`, and say in the \
             commit message what a peer running the previous shape would decode \
             from the new payload. bincode has no field names to miss, so \
             `#[serde(default)]` supplies nothing here -- an inserted field \
             shifts the meaning of every byte after it. Whether the decode then \
             REFUSES or yields garbage is recorded as unestablished in \
             `docs/planning/queue.md`; either way it is not a refusal that names \
             the cause."
        );
    }

    /// ⭐⭐ **THE PROPERTY `ggrs`'s TRANSPORT ACTUALLY DEPENDS ON, AND NEITHER
    /// THE OLD JSON NOR A SOURCE CENSUS COULD STATE IT.**
    ///
    /// `protocol.rs` measures ONE default frame at line 68 and then slices every
    /// player's input out of a packet at that fixed stride (line 117). That is
    /// only correct while the encoding is FIXED-WIDTH. Give `ControlFrame` a
    /// `String`, a `Vec`, an `Option` or a data-carrying enum variant and the
    /// bytes stop being a constant size — at which point player 2's slice starts
    /// mid-way through player 1's frame and every peer decodes garbage, with no
    /// checksum anywhere to notice.
    #[test]
    fn every_frame_encodes_to_the_same_width_as_the_default() {
        let default_size = bincode::serialized_size(&ControlFrame::default())
            .expect("a default ControlFrame has a serialized size");
        assert_eq!(
            default_size, RECORDED_WIRE_BYTES,
            "the per-player input slot ggrs sizes from a default frame moved"
        );
        for (what, frame) in [
            ("default", ControlFrame::default()),
            ("legible", a_frame_whose_encoding_is_legible()),
            (
                "one float",
                ControlFrame {
                    axis_x: f32::MIN,
                    ..ControlFrame::default()
                },
            ),
            (
                "last enum variant",
                ControlFrame {
                    attack_strength_hint: AttackStrengthHint::Smash,
                    control_frame_modes: ControlFrameModes {
                        movement: InputFrameMode::BodyRelativeAssist,
                        aim: InputFrameMode::BodyRelativeAssist,
                    },
                    ..ControlFrame::default()
                },
            ),
        ] {
            assert_eq!(
                bincode::serialized_size(&frame).expect("serialized size"),
                default_size,
                "the `{what}` frame does not encode to the width ggrs slices \
                 by, so a packet carrying more than one player's input would be \
                 cut in the wrong places"
            );
        }
    }

    /// ⚠ THE MODULE'S OWN PREMISE: bincode is really what the transport uses. If
    /// the pinned `ggrs` ever changes serializer, or `ControlFrame` gains a
    /// hand-written `Serialize`, these bytes stop describing the wire.
    #[test]
    fn a_frame_survives_the_transports_own_round_trip() {
        let frame = a_frame_whose_encoding_is_legible();
        let bytes = bincode::serialize(&frame).expect("serialize");
        let back: ControlFrame = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(
            back, frame,
            "a frame does not survive the round trip ggrs performs on it, so \
             the recorded bytes above are not what a peer would decode"
        );
    }

    /// The fact that kills the rename argument, asserted rather than asserted
    /// ABOUT: if a field name appeared in the encoding, a `#[serde(rename)]`
    /// would be a wire change and the old module would have been right.
    #[test]
    fn the_encoding_carries_no_field_names() {
        let bytes = bincode::serialize(&a_frame_whose_encoding_is_legible()).expect("serialize");
        for name in [
            &b"axis_x"[..],
            &b"dash_pressed"[..],
            &b"burst_pressed"[..],
            &b"attack_strength_hint"[..],
            &b"Smash"[..],
            &b"movement"[..],
        ] {
            assert!(
                !bytes.windows(name.len()).any(|window| window == name),
                "`{}` appears in the encoding, so it is name-based after all and \
                 a rename IS a wire change",
                String::from_utf8_lossy(name)
            );
        }
    }
}
