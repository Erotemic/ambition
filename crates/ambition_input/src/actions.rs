//! The `Platformer2dInputActionMonolith` leafwing action enum — the logical-input vocabulary the
//! device-binding layer maps physical keys/sticks onto, before it is folded into
//! the device-agnostic `ControlFrame`/`MenuInputFrame`. Gated behind the `input`
//! feature (pulls in leafwing's `Actionlike`).

#[cfg(feature = "input")]
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
/// these actions) instead of touching `Platformer2dInputActionMonolith` directly.
///
/// Gated behind `input`: this type pulls in leafwing's `Actionlike` trait.
/// Sim-only builds use engine-core `ControlFrame` (re-exported here) on the seam instead.
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
    /// The shared dodge/dash press — the BURST channel. What one press buys
    /// is the controlled body's answer (`BurstManeuver`), never this name's.
    Burst,
    Blink,
    /// Player signature SPECIAL — a dedicated slot, distinct from [`Self::Blink`].
    /// Historically the player brain aliased `special_pressed = blink_pressed`
    /// because there was no special input; this is that input. Default binding:
    /// a per-preset keyboard key (gamepad Special awaits the remap pass, since
    /// the face + shoulder buttons are already fully assigned).
    Special,
    /// One cannot accidentally masquerade as the other."*).
    ///
    /// this was `QuickAction` — a generic name for an action every
    /// producer and consumer already treated as the shield (the touch overlay's
    /// Shield button maps here, every keyboard preset's `shield` key binds it,
    /// and `read_gameplay_control_frame` reads it straight into `shield_held`).
    ///
    /// It is migrated on load by `ControlSettings::clamp_all`; see `RENAMED_ACTIONS` there.
    Shield,
    /// GRAB — press to attempt a capture. Starts an authored grab move; if
    /// that move's active window acquires a body, the two enter a capture
    /// relationship the grab move itself does not own and does not end.
    ///
    /// an independent action, not a variant of [`Self::Attack`], for the
    /// same reason Shield is not a variant of Special: a grab beats a guard that
    /// stops an attack, and it establishes something that outlives the press.
    /// Binding it onto the attack button would make "may this body grab" and
    /// "may this body swing" the same permission.
    Grab,
    /// TAUNT — press to express, at the cost of standing still. Its own
    /// action rather than a modifier on Attack, because a taunt is not a swing
    /// and a body that has one is not thereby more dangerous.
    Taunt,
    Interact,
    /// WALK — hold to cap movement into the walk band.
    ///
    /// ⛔⛔ ITS OWN ACTION, AND NOT [`Self::Modifier`], which is what the obvious
    /// reading suggests. The modifier slot is already CLAIMED: Mary-O reads
    /// `modifier_held` as her RUN (`ambition_demo_mary_o::movement`), so capping
    /// the axis on it at the shared adapter would make her run key slow her
    /// down. Two games would be spelling opposite intents with one key.
    ///
    /// ⭐ WHY IT EXISTS AT ALL: the simulation reads the stick's MAGNITUDE as
    /// the gait, and a DIGITAL source can only ever say 1.0 — so without this a
    /// keyboard or D-pad fighter cannot walk, which costs the whole genre
    /// neutral (no walk approach, no walk-to-tilt spacing, and every grounded
    /// Attack answered with the dash attack). An analog stick has never needed
    /// it and is unaffected: the adapter CAPS rather than scales.
    Walk,
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
    /// Toggle player trail emission.
    ///
    /// The physical binding lives in the keyboard preset, not in gameplay code.
    /// This is intentionally separate from projectile/spell input: the trail is
    /// a persistent topological drawing mode, so it starts/stops on a press edge
    /// instead of firing an instantaneous ability.
    TrailToggle,
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
    /// Paged-menu page turn LEFT. Bound to the LEFT shoulder bumper (L1 / LB =
    /// `GamepadButton::LeftTrigger`) and the `Q` key. Read only by paged menus
    /// (the 3D inventory cube); gameplay never consumes it.
    MenuPageLeft,
    /// Paged-menu page turn RIGHT. Bound to the RIGHT shoulder bumper (R1 / RB =
    /// `GamepadButton::RightTrigger`) and the `E` key.
    MenuPageRight,
    #[actionlike(DualAxis)]
    MenuStick,
    /// Analog right-trigger value (0..=1). Used together with
    /// configurable hysteresis thresholds to derive the burst-pressed
    /// edge so a worn trigger held above the threshold cannot retrigger
    /// the burst repeatedly.
    #[actionlike(Axis)]
    BurstAnalog,
    /// Analog right-stick / aim read. The aim deadzone is applied here
    /// before the value reaches blink aim, so a drifting Xbox 360
    /// controller does not gradually push the blink target upward.
    #[actionlike(DualAxis)]
    AimStick,
}

#[cfg(feature = "input")]
impl Platformer2dInputActionMonolith {
    /// Is this action read ONLY while a menu surface is up?
    ///
    /// The pad deliberately SHARES physical buttons between gameplay and menus
    /// — `MenuSelect` sits on South beside Jump, `MenuBack` on East beside
    /// Blink, the page turns on the bumpers — and that is safe precisely
    /// because a paged menu only consumes them while it is open.
    ///
    /// A game's [`crate::BindingLayout`] re-arranges GAMEPLAY: it has to be
    /// able to take Jump off South without taking confirm off South with it.
    /// This is the line it cuts on.
    ///
    /// exhaustive on purpose. A new action must decide which side it is
    /// on, at the compiler's insistence, rather than falling through a `_` arm
    /// into whichever answer the author of this function happened to prefer.
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
