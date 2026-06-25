use crate::Vec2;

/// Game-action input for one simulation frame.
///
/// Keyboard/gamepad remapping belongs in the presentation layer. Once those
/// devices are interpreted, the engine only needs a small set of actions.
///
/// Directional axes are controlled-body-local by the time they reach the
/// movement engine: `axis_x` is local side/right and `axis_y` is local
/// down/toward-feet unless a field explicitly says it is screen/raw input.
#[derive(Clone, Copy, Debug, Default)]
pub struct InputState {
    /// -1 local-left, +1 local-right.
    pub axis_x: f32,
    /// -1 local-up / away-from-feet, +1 local-down / toward-feet.
    pub axis_y: f32,
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
    /// NOT re-derive blink direction from the local `axis_*`), so quick blink is
    /// locomotion-framed and gravity-correct without the engine knowing the
    /// gravity frame. Zero → fall back to facing.
    pub blink_quick_dir: Vec2,
    /// WORLD-space precision-blink steer vector for the current frame, resolved
    /// through the *aim* frame mode at the seam (screen-directed by default).
    /// Magnitude carries the stick deflection; the engine integrates it into the
    /// precision aim offset. Decoupled from `blink_quick_dir` so quick blink and
    /// precision blink can use different frame policies on the same stick.
    pub blink_aim_step: Vec2,
    /// Double-tap-down gesture recognized by the input layer. This is separate
    /// from `axis_y` so down+attack can mean pogo without forcing fast-fall.
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
    /// `MovementTuning::parry_window_time` seconds after activation are the
    /// parry window (full invulnerability).
    pub shield_held: bool,
    /// Real, unscaled frame duration supplied by the presentation layer.
    ///
    /// Most simulation uses the scaled `raw_dt`, but precision-blink aiming is
    /// a control/UI gesture: the cursor should stay responsive even when game
    /// time is nearly frozen. If zero, the engine falls back to scaled dt.
    pub control_dt: f32,
}
