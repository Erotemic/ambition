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
    /// Device-independent request to classify this attack press as strong.
    /// A dedicated smash key, C-stick adapter, replay, remote peer, or RL
    /// policy may set it; the multi-tick flick history stays sim-side.
    pub attack_strong_hint: bool,
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
            // Levels — latest wins.
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
            attack_strong_hint: self.attack_strong_hint | sample.attack_strong_hint,
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
    use super::{ControlFrame, ControlFrameLatch};

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
            attack_strong_hint: true,
            ..ControlFrame::default()
        });
        latch.accumulate(ControlFrame {
            attack_released: true,
            ..ControlFrame::default()
        });

        let tick = latch.take();
        assert!(tick.attack_pressed);
        assert!(tick.attack_released);
        assert!(tick.attack_strong_hint);
        assert!(!tick.attack_held);

        let next = latch.take();
        assert!(!next.attack_pressed);
        assert!(!next.attack_released);
        assert!(!next.attack_strong_hint);
        assert!(!next.attack_held);
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
