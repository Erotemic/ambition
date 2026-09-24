//! The `Platformer2dInputActionMonolith` leafwing action enum. The binding
//! layer maps physical keys and sticks onto it, then folds it into the
//! device-agnostic `ControlFrame`/`MenuInputFrame`. Gated behind `input`.

#[cfg(feature = "input")]
use super::*;

/// Logical player/sandbox inputs understood by the Bevy adapter layer.
///
/// `Move` is dual-axis so analog sticks and virtual D-pads can feed a single
/// movement vector. The cardinal `Move*` button actions intentionally duplicate
/// the directional bindings so systems can still detect edge-triggered gestures
/// such as double-tap-down fast fall and double-tap-up door activation.
///
/// Menu navigation has its own `MenuNavigate*` / `MenuSelect` / `MenuBack`
/// actions, so menu confirm does not need "Jump" and all menu keys flow
/// through one seam. The renderer reads `MenuAxisFrame`, not this enum.
///
/// Gated behind `input` (leafwing `Actionlike`). Sim-only builds use the
/// engine-core `ControlFrame` instead.
#[cfg(feature = "input")]
#[derive(Actionlike, Clone, Copy, Debug, Hash, PartialEq, Eq, Reflect)]
pub enum Platformer2dInputActionMonolith {
    #[actionlike(DualAxis)]
    Move,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Jump,
    Attack,
    /// Device-side strong-attack hint. It is intentionally separate from the
    /// character action-slot vocabulary: the sim combines it with Attack and
    /// authoritative flick history to classify tilt versus smash.
    StrongAttack,
    /// The shared dodge/dash press (the burst channel). The controlled body's
    /// `BurstManeuver` decides what one press does.
    Burst,
    Blink,
    /// Player signature special. A dedicated slot, distinct from [`Self::Blink`].
    /// Default binding: a per-preset keyboard key. The gamepad has no binding
    /// yet because the face and shoulder buttons are all assigned.
    Special,
    /// The shield action. Old settings that name it `QuickAction` are
    /// migrated on load by `ControlSettings::clamp_all` (see `RENAMED_ACTIONS`).
    Shield,
    /// Grab: press to attempt a capture. It starts an authored grab move. If
    /// that move's active window acquires a body, the two enter a capture
    /// relationship that outlives the move.
    ///
    /// It is not a variant of [`Self::Attack`]: a grab beats a guard, and a
    /// shared button would make "may grab" and "may swing" one permission.
    Grab,
    /// Taunt: press to express, at the cost of standing still. It is not a
    /// modifier on Attack, because a taunt is not a swing.
    Taunt,
    Interact,
    /// Walk: hold to cap movement into the walk band.
    ///
    /// This is its own action, not [`Self::Modifier`]. Mary-O reads
    /// `modifier_held` as her run (`ambition_demo_mary_o::movement`), so a cap
    /// on the modifier would make her run key slow her down.
    ///
    /// The simulation reads stick magnitude as the gait, and a digital source
    /// always gives 1.0. Without this action a keyboard or D-pad body cannot
    /// walk. The adapter caps (not scales), so analog sticks are unaffected.
    Walk,
    Modifier,
    Utility,
    Map,
    Inventory,
    Pogo,
    Reset,
    Start,
    /// Player projectile / spell action. Default binding: `F` (keyboard) and
    /// the gamepad West face button (shared with Attack when no projectile is
    /// unlocked).
    Projectile,
    /// Toggle player trail emission.
    ///
    /// The physical binding lives in the keyboard preset. It is separate from
    /// projectile input: the trail is a persistent drawing mode, so a press
    /// edge starts or stops it.
    TrailToggle,
    /// Menu navigation. Only the pause / settings menu reads these; gameplay
    /// never does. Bindings: arrows, WASD, D-pad, left stick; Enter / Space /
    /// South select; Escape / Backspace / East back.
    MenuNavigateUp,
    MenuNavigateDown,
    MenuNavigateLeft,
    MenuNavigateRight,
    MenuSelect,
    MenuBack,
    /// Paged-menu page turn left. Bound to the left bumper (L1 / LB =
    /// `GamepadButton::LeftTrigger`) and `Q`. Only paged menus read it.
    MenuPageLeft,
    /// Paged-menu page turn right. Bound to the right bumper (R1 / RB =
    /// `GamepadButton::RightTrigger`) and `E`.
    MenuPageRight,
    #[actionlike(DualAxis)]
    MenuStick,
    /// Analog right-trigger value (0..=1). Hysteresis thresholds derive the
    /// burst press edge, so a worn trigger held above the threshold does not
    /// retrigger the burst.
    #[actionlike(Axis)]
    BurstAnalog,
    /// Analog right-stick aim. The aim deadzone applies here, before blink
    /// aim reads it, so a drifting stick does not move the blink target.
    #[actionlike(DualAxis)]
    AimStick,
}

#[cfg(feature = "input")]
impl Platformer2dInputActionMonolith {
    /// Returns true if only a menu surface reads this action.
    ///
    /// The pad shares buttons between gameplay and menus (`MenuSelect` on
    /// South with Jump, `MenuBack` on East with Blink, page turns on the
    /// bumpers). This is safe because a menu reads them only while open.
    /// A [`crate::BindingLayout`] rearranges gameplay along this line, so it
    /// can move Jump off South and keep confirm there.
    ///
    /// The match is exhaustive so each new action must choose a side.
    pub fn is_menu_only(self) -> bool {
        match self {
            Self::MenuNavigateUp
            | Self::MenuNavigateDown
            | Self::MenuNavigateLeft
            | Self::MenuNavigateRight
            | Self::MenuSelect
            | Self::MenuBack
            | Self::MenuPageLeft
            | Self::MenuPageRight
            | Self::MenuStick => true,
            Self::Move
            | Self::MoveLeft
            | Self::MoveRight
            | Self::MoveUp
            | Self::MoveDown
            | Self::Jump
            | Self::Attack
            | Self::StrongAttack
            | Self::Burst
            | Self::Blink
            | Self::Special
            | Self::Shield
            | Self::Grab
            | Self::Taunt
            | Self::Interact
            | Self::Walk
            | Self::Modifier
            | Self::Utility
            | Self::Map
            | Self::Inventory
            | Self::Pogo
            | Self::Reset
            | Self::Start
            | Self::Projectile
            | Self::TrailToggle
            | Self::BurstAnalog
            | Self::AimStick => false,
        }
    }
}
