/// Game-action input for one simulation frame.
///
/// Keyboard/gamepad remapping belongs in the presentation layer. Once those
/// devices are interpreted, the engine only needs a small set of actions.
#[derive(Clone, Copy, Debug, Default)]
pub struct InputState {
    /// -1 left, +1 right.
    pub axis_x: f32,
    /// -1 up, +1 down.
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
    /// Double-tap-down gesture recognized by the input layer. This is separate
    /// from `axis_y` so down+attack can mean pogo without forcing fast-fall.
    pub fast_fall_pressed: bool,
    /// Down-held + jump-pressed gesture: drop through one-way platforms.
    /// The presentation layer composes this from raw inputs so the engine
    /// does not have to reason about jump-vs-drop disambiguation itself.
    pub drop_through_pressed: bool,
    pub attack_pressed: bool,
    /// Dedicated downward/pogo slash action. This is separate from
    /// `attack_pressed` so layouts can expose four main face-button verbs.
    pub pogo_pressed: bool,
    pub reset_pressed: bool,
    /// Real, unscaled frame duration supplied by the presentation layer.
    ///
    /// Most simulation uses the scaled `raw_dt`, but precision-blink aiming is
    /// a control/UI gesture: the cursor should stay responsive even when game
    /// time is nearly frozen. If zero, the engine falls back to scaled dt.
    pub control_dt: f32,
}
