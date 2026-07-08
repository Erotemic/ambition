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

use crate::{InputState, RawDirectionEdges, Vec2};

/// Per-frame snapshot of semantic controller input feeding actor brains.
///
/// This is intentionally device-free: there are no keyboards, gamepads,
/// touch events, or Leafwing actions here. Visible builds populate it from
/// `ambition_input`; headless tests, replay, and future netcode can populate it
/// directly.
///
/// **Multiplayer caveat (primary-player compatibility):** the legacy app still
/// keeps one global `ControlFrame` resource for the local primary input before
/// mirroring it into per-slot brain state. The type itself is slot-neutral and
/// is also used by `ambition_characters::brain::SlotControls`.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ControlFrame {
    pub axis_x: f32,
    pub axis_y: f32,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub jump_released: bool,
    pub dash_pressed: bool,
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
    pub attack_pressed: bool,
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

    pub fn engine_input(self, control_dt: f32) -> InputState {
        // The drop-through gesture is formed gravity-relatively in the engine
        // (`movement::wants_drop_through`) from axis_y + jump, not precomputed here.
        InputState {
            axis_x: self.axis_x,
            axis_y: self.axis_y,
            jump_pressed: self.jump_pressed,
            jump_held: self.jump_held,
            jump_released: self.jump_released,
            dash_pressed: self.dash_pressed,
            fly_toggle_pressed: self.fly_toggle_pressed,
            blink_pressed: self.blink_pressed,
            blink_held: self.blink_held,
            blink_released: self.blink_released,
            // This raw passthrough has no gravity frame, so screen == world: the
            // blink aim vectors are the raw stick. The canonical player path
            // resolves these per-frame at the brain seam instead.
            blink_quick_dir: Vec2::new(self.axis_x, self.axis_y),
            blink_aim_step: Vec2::new(self.axis_x, self.axis_y),
            fast_fall_pressed: self.fast_fall_pressed,
            attack_pressed: self.attack_pressed,
            pogo_pressed: self.pogo_pressed,
            interact_pressed: self.interact_pressed,
            reset_pressed: false,
            shield_held: self.shield_held,
            control_dt,
        }
    }
}
