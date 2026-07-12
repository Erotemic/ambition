use crate::reference_frame::{LocalAxes, WorldVec2};
use crate::Vec2;

/// Game-action input for one simulation frame — the resolved motion intent.
///
/// Keyboard/gamepad remapping belongs in the presentation layer, and
/// screen-vs-body input-frame accommodation belongs at the controller seam.
/// Every directional field here carries its frame in its TYPE: by the time an
/// `InputState` reaches the movement kernel, all frame resolution has already
/// happened against the same [`crate::MotionFrame`] the kernel will step with.
///
/// - [`LocalAxes`] — controlled-body-local (`+x` side/right, `+y` toward-feet);
/// - [`WorldVec2`] — world-space, resolved through a controller frame policy at
///   the seam.
///
/// Raw [`crate::ScreenAxes`] never appear below the seam.
#[derive(Clone, Copy, Debug, Default)]
pub struct InputState {
    /// Locomotion stick in the controlled body's local frame.
    pub axes: LocalAxes,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub jump_released: bool,
    pub dash_pressed: bool,
    /// Toggle free-flight mode when the ability is enabled.
    pub fly_toggle_pressed: bool,
    /// Blink/special button pressed this frame.
    pub blink_pressed: bool,
    /// Blink/special button held this frame.
    pub blink_held: bool,
    /// Blink/special button released this frame.
    pub blink_released: bool,
    /// WORLD-space quick-blink direction, already resolved through the movement
    /// frame mode at the input seam. The engine consumes this directly (it does
    /// NOT re-derive blink direction from the local `axes`), so quick blink is
    /// locomotion-framed and gravity-correct without the engine knowing the
    /// gravity frame. Zero → fall back to facing.
    pub blink_quick_dir: WorldVec2,
    /// WORLD-space precision-blink steer vector for the current frame, resolved
    /// through the *aim* frame mode at the seam (screen-directed by default).
    /// Magnitude carries the stick deflection; the engine integrates it into the
    /// precision aim offset. Decoupled from `blink_quick_dir` so quick blink and
    /// precision blink can use different frame policies on the same stick.
    pub blink_aim_step: WorldVec2,
    /// Double-tap-down gesture recognized by the input layer. This is separate
    /// from the local descend axis so down+attack can mean pogo without forcing
    /// fast-fall.
    pub fast_fall_pressed: bool,
    pub attack_pressed: bool,
    /// Dedicated downward/pogo slash action. This is separate from
    /// `attack_pressed` so layouts can expose four main face-button verbs.
    pub pogo_pressed: bool,
    /// Generic context/confirm input. The engine only uses this for mechanics
    /// that are already movement-owned (currently ledge pull-up confirm); room
    /// interactions remain sandbox-owned.
    pub interact_pressed: bool,
    pub reset_pressed: bool,
    /// Shield button is currently held. When the `shield` ability is active,
    /// holding this deploys the bubble; releasing drops it. The first
    /// `parry_window_time` seconds after activation are the parry window (full
    /// invulnerability).
    pub shield_held: bool,
    /// Real, unscaled frame duration supplied by the presentation layer.
    ///
    /// Most simulation uses the scaled `raw_dt`, but precision-blink aiming is
    /// a control/UI gesture: the cursor should stay responsive even when game
    /// time is nearly frozen. If zero, the engine falls back to scaled dt.
    pub control_dt: f32,
}

impl InputState {
    /// The locomotion stick in the controlled body's local acceleration frame,
    /// as a bare vector for kernel-internal math.
    pub const fn local_axis(self) -> Vec2 {
        self.axes.vec()
    }

    /// Convenience constructor for a locomotion-only intent.
    pub const fn with_axes(x: f32, y: f32) -> Self {
        let mut input = Self::const_default();
        input.axes = LocalAxes::new(x, y);
        input
    }

    const fn const_default() -> Self {
        Self {
            axes: LocalAxes::ZERO,
            jump_pressed: false,
            jump_held: false,
            jump_released: false,
            dash_pressed: false,
            fly_toggle_pressed: false,
            blink_pressed: false,
            blink_held: false,
            blink_released: false,
            blink_quick_dir: WorldVec2::ZERO,
            blink_aim_step: WorldVec2::ZERO,
            fast_fall_pressed: false,
            attack_pressed: false,
            pogo_pressed: false,
            interact_pressed: false,
            reset_pressed: false,
            shield_held: false,
            control_dt: 0.0,
        }
    }
}
