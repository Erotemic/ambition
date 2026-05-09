use super::*;

/// Logical player/sandbox inputs understood by the Bevy adapter layer.
///
/// `Move` is dual-axis so analog sticks and virtual D-pads can feed a single
/// movement vector. The cardinal `Move*` button actions intentionally duplicate
/// the directional bindings so systems can still detect edge-triggered gestures
/// such as double-tap-down fast fall and double-tap-up door activation.
///
/// Menu navigation lives on its own `MenuNavigate*` / `MenuSelect` /
/// `MenuBack` axis so confirming in a menu does not require pressing
/// "Jump", and so D-pad / arrow keys / Enter all flow through one
/// semantic seam. The renderer reads `MenuAxisFrame` (drained from
/// these actions) instead of touching `SandboxAction` directly.
///
/// Gated behind `input`: this type pulls in leafwing's `Actionlike` trait.
/// Sim-only builds use `ControlFrame` (always-available) on the seam instead.
#[cfg(feature = "input")]
#[derive(Actionlike, Clone, Copy, Debug, Hash, PartialEq, Eq, Reflect)]
pub enum SandboxAction {
    #[actionlike(DualAxis)]
    Move,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Jump,
    Attack,
    Dash,
    Blink,
    QuickAction,
    Interact,
    Modifier,
    Utility,
    Map,
    Inventory,
    Pogo,
    Reset,
    Start,
    /// Player projectile / spell action. Default binding: `F` (keyboard)
    /// and the gamepad West face button (with Attack on the same button
    /// when no projectile is unlocked yet — sandbox always-on for now).
    Projectile,
    /// Menu navigation seam. These are the only actions the pause /
    /// settings menu reads; gameplay never consumes them. Bindings:
    /// arrow keys, WASD, D-pad, left stick (with deadzone applied
    /// later), Enter / Space / South for select, Escape / Backspace /
    /// East for back.
    MenuNavigateUp,
    MenuNavigateDown,
    MenuNavigateLeft,
    MenuNavigateRight,
    MenuSelect,
    MenuBack,
    /// Analog left-stick read used to drive menu navigation with
    /// configurable deadzone + repeat. Renders into `MenuAxisFrame`.
    #[actionlike(DualAxis)]
    MenuStick,
    /// Analog right-trigger value (0..=1). Used together with
    /// configurable hysteresis thresholds to derive the dash-pressed
    /// edge so a worn trigger held above the threshold cannot retrigger
    /// dash repeatedly.
    #[actionlike(Axis)]
    DashAnalog,
    /// Analog right-stick / aim read. The aim deadzone is applied here
    /// before the value reaches blink aim, so a drifting Xbox 360
    /// controller does not gradually push the blink target upward.
    #[actionlike(DualAxis)]
    AimStick,
}
