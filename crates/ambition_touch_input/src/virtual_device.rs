//! The touch overlay as a VIRTUAL DEVICE: leafwing input kinds computed from
//! [`MobileTouchState`], so touch resolves through the participant's
//! `InputMap` bindings and the active input context exactly like a keyboard
//! or gamepad — never as a second system writing gameplay/menu resources
//! directly.
//!
//! Three input kinds over one source:
//!
//! - [`TouchVirtualButton`] — one on-screen action button, held-state
//!   semantics identical to a physical button;
//! - [`TouchVirtualStick`] — the move stick as a dual-axis (published in
//!   leafwing's +Y-up convention; the gameplay reader flips to the sim's
//!   +Y-down exactly as it does for a gamepad stick);
//! - [`TouchStickDirection`] — the stick as four threshold buttons, the
//!   `GamepadControlDirection`-with-threshold analog for the discrete
//!   `MoveLeft/Right/Up/Down` gesture edges (double-tap-down morph, etc.).
//!
//! [`bind_touch_virtual_inputs`] adds the bindings to the participant's
//! `InputMap`. The context-sensitive behaviors that used to be special-cased
//! (the Jump button acting as menu confirm, the Reset button acting as Back)
//! are DECLARED double-bindings here: the gameplay verb routes only while the
//! gameplay context owns input, the menu action always feeds the menu frame —
//! the same routing every physical device gets.

use bevy::ecs::system::lifetimeless::SRes;
use bevy::ecs::system::StaticSystemParam;
use bevy::prelude::*;
use leafwing_input_manager::buttonlike::ButtonValue;
use leafwing_input_manager::clashing_inputs::BasicInputs;
use leafwing_input_manager::prelude::updating::{CentralInputStore, UpdatableInput};
use leafwing_input_manager::prelude::{serde_typetag, Buttonlike, DualAxislike, UserInput};
use leafwing_input_manager::InputControlKind;
use serde::{Deserialize, Serialize};

use super::bevy_plugin::MobileTouchState;
use super::layout::TouchActionButton;
use super::state::TouchButton;

/// Raw stick deflection past this magnitude counts as a held direction —
/// the same threshold the gamepad's `GamepadControlDirection` bindings use
/// (`STICK_DIRECTION_THRESHOLD` in the presets), so a touch flick and a pad
/// flick produce identical `MoveLeft/Right/Up/Down` press edges.
const DIRECTION_THRESHOLD: f32 = 0.5;

/// One on-screen touch button as a bindable virtual button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub struct TouchVirtualButton(pub TouchActionButton);

impl UserInput for TouchVirtualButton {
    fn kind(&self) -> InputControlKind {
        InputControlKind::Button
    }

    fn decompose(&self) -> BasicInputs {
        BasicInputs::Simple(Box::new(*self))
    }
}

impl UpdatableInput for TouchVirtualButton {
    type SourceData = SRes<MobileTouchState>;

    fn compute(
        mut central_input_store: ResMut<CentralInputStore>,
        source_data: StaticSystemParam<Self::SourceData>,
    ) {
        for action in ALL_TOUCH_BUTTONS {
            let held = touch_button_state(&source_data, action).held;
            central_input_store
                .update_buttonlike(TouchVirtualButton(action), ButtonValue::from_pressed(held));
        }
    }
}

#[serde_typetag]
impl Buttonlike for TouchVirtualButton {
    fn get_pressed(&self, input_store: &CentralInputStore, _gamepad: Entity) -> Option<bool> {
        input_store.pressed(self)
    }

    /// Test/mocking seam: press the underlying touch state directly.
    fn press(&self, world: &mut World) {
        set_touch_button(world, self.0, true);
    }

    fn release(&self, world: &mut World) {
        set_touch_button(world, self.0, false);
    }
}

/// The touch move stick as a bindable dual-axis. Published in leafwing's
/// +Y-up convention (the touch state stores +Y-down screen space), matching
/// `GamepadStick::LEFT` so every downstream reader treats them identically.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub struct TouchVirtualStick;

impl UserInput for TouchVirtualStick {
    fn kind(&self) -> InputControlKind {
        InputControlKind::DualAxis
    }

    fn decompose(&self) -> BasicInputs {
        BasicInputs::Composite(vec![
            Box::new(TouchStickDirection::Up),
            Box::new(TouchStickDirection::Down),
            Box::new(TouchStickDirection::Left),
            Box::new(TouchStickDirection::Right),
        ])
    }
}

impl UpdatableInput for TouchVirtualStick {
    type SourceData = SRes<MobileTouchState>;

    fn compute(
        mut central_input_store: ResMut<CentralInputStore>,
        source_data: StaticSystemParam<Self::SourceData>,
    ) {
        let state = source_data.0;
        central_input_store
            .update_dualaxislike(TouchVirtualStick, Vec2::new(state.move_x, -state.move_y));
    }
}

#[serde_typetag]
impl DualAxislike for TouchVirtualStick {
    fn get_axis_pair(&self, input_store: &CentralInputStore, _gamepad: Entity) -> Option<Vec2> {
        input_store.pair(self)
    }

    /// Test/mocking seam: drive the underlying touch stick (value arrives in
    /// leafwing's +Y-up convention and is stored back as +Y-down).
    fn set_axis_pair(&self, world: &mut World, value: Vec2) {
        let mut state = world.resource_mut::<MobileTouchState>();
        state.0.move_x = value.x;
        state.0.move_y = -value.y;
    }
}

/// The touch stick's cardinal directions as threshold buttons — the source
/// for the discrete `MoveLeft/Right/Up/Down` press edges (leafwing derives
/// the edge from the held transition, exactly like a pad stick direction).
/// Directions are in leafwing's convention: `Up` = stick pushed up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub enum TouchStickDirection {
    Up,
    Down,
    Left,
    Right,
}

impl UserInput for TouchStickDirection {
    fn kind(&self) -> InputControlKind {
        InputControlKind::Button
    }

    fn decompose(&self) -> BasicInputs {
        BasicInputs::Simple(Box::new(*self))
    }
}

impl UpdatableInput for TouchStickDirection {
    type SourceData = SRes<MobileTouchState>;

    fn compute(
        mut central_input_store: ResMut<CentralInputStore>,
        source_data: StaticSystemParam<Self::SourceData>,
    ) {
        let state = source_data.0;
        // Touch state is +Y-down: pushing the stick UP is negative move_y.
        let held = [
            (
                TouchStickDirection::Up,
                -state.move_y >= DIRECTION_THRESHOLD,
            ),
            (
                TouchStickDirection::Down,
                state.move_y >= DIRECTION_THRESHOLD,
            ),
            (
                TouchStickDirection::Left,
                -state.move_x >= DIRECTION_THRESHOLD,
            ),
            (
                TouchStickDirection::Right,
                state.move_x >= DIRECTION_THRESHOLD,
            ),
        ];
        for (direction, held) in held {
            central_input_store.update_buttonlike(direction, ButtonValue::from_pressed(held));
        }
    }
}

#[serde_typetag]
impl Buttonlike for TouchStickDirection {
    fn get_pressed(&self, input_store: &CentralInputStore, _gamepad: Entity) -> Option<bool> {
        input_store.pressed(self)
    }

    /// Test/mocking seam: deflect the underlying touch stick fully in this
    /// direction (release recenters that axis).
    fn press(&self, world: &mut World) {
        let mut state = world.resource_mut::<MobileTouchState>();
        match self {
            TouchStickDirection::Up => state.0.move_y = -1.0,
            TouchStickDirection::Down => state.0.move_y = 1.0,
            TouchStickDirection::Left => state.0.move_x = -1.0,
            TouchStickDirection::Right => state.0.move_x = 1.0,
        }
    }

    fn release(&self, world: &mut World) {
        let mut state = world.resource_mut::<MobileTouchState>();
        match self {
            TouchStickDirection::Up | TouchStickDirection::Down => state.0.move_y = 0.0,
            TouchStickDirection::Left | TouchStickDirection::Right => state.0.move_x = 0.0,
        }
    }
}

const ALL_TOUCH_BUTTONS: [TouchActionButton; 12] = [
    TouchActionButton::Jump,
    TouchActionButton::Attack,
    TouchActionButton::Special,
    TouchActionButton::Dash,
    TouchActionButton::Blink,
    TouchActionButton::Interact,
    TouchActionButton::Projectile,
    TouchActionButton::FlyToggle,
    TouchActionButton::Shield,
    TouchActionButton::Modifier,
    TouchActionButton::Start,
    TouchActionButton::Reset,
];

fn touch_button_state(state: &MobileTouchState, action: TouchActionButton) -> TouchButton {
    match action {
        TouchActionButton::Jump => state.0.jump,
        TouchActionButton::Attack => state.0.attack,
        TouchActionButton::Special => state.0.special,
        TouchActionButton::Dash => state.0.dash,
        TouchActionButton::Blink => state.0.blink,
        TouchActionButton::Interact => state.0.interact,
        TouchActionButton::Projectile => state.0.projectile,
        TouchActionButton::FlyToggle => state.0.fly_toggle,
        TouchActionButton::Shield => state.0.shield,
        TouchActionButton::Modifier => state.0.modifier,
        TouchActionButton::Start => state.0.start,
        TouchActionButton::Reset => state.0.reset,
    }
}

fn set_touch_button(world: &mut World, action: TouchActionButton, held: bool) {
    let mut state = world.resource_mut::<MobileTouchState>();
    let button = match action {
        TouchActionButton::Jump => &mut state.0.jump,
        TouchActionButton::Attack => &mut state.0.attack,
        TouchActionButton::Special => &mut state.0.special,
        TouchActionButton::Dash => &mut state.0.dash,
        TouchActionButton::Blink => &mut state.0.blink,
        TouchActionButton::Interact => &mut state.0.interact,
        TouchActionButton::Projectile => &mut state.0.projectile,
        TouchActionButton::FlyToggle => &mut state.0.fly_toggle,
        TouchActionButton::Shield => &mut state.0.shield,
        TouchActionButton::Modifier => &mut state.0.modifier,
        TouchActionButton::Start => &mut state.0.start,
        TouchActionButton::Reset => &mut state.0.reset,
    };
    let was_held = button.held;
    *button = TouchButton {
        held,
        pressed_this_frame: held && !was_held,
        released_this_frame: !held && was_held,
    };
}

/// The declared touch binding table: which `SandboxAction`s each virtual
/// control feeds. Multi-action rows are the honest form of what used to be
/// hidden branches (Jump/Interact confirm menus; Reset backs out).
pub fn touch_bindings() -> Vec<(ambition_input::SandboxAction, TouchVirtualButton)> {
    use ambition_input::SandboxAction as A;
    use TouchActionButton as B;
    vec![
        (A::Jump, TouchVirtualButton(B::Jump)),
        (A::MenuSelect, TouchVirtualButton(B::Jump)),
        (A::Attack, TouchVirtualButton(B::Attack)),
        (A::Special, TouchVirtualButton(B::Special)),
        (A::Dash, TouchVirtualButton(B::Dash)),
        (A::Blink, TouchVirtualButton(B::Blink)),
        (A::Interact, TouchVirtualButton(B::Interact)),
        (A::MenuSelect, TouchVirtualButton(B::Interact)),
        (A::Projectile, TouchVirtualButton(B::Projectile)),
        // The overlay's Fly button is the Utility slot (fly toggle), and
        // Shield is the QuickAction slot (shield/roll) — the same actions
        // the keyboard/gamepad bindings feed.
        (A::Utility, TouchVirtualButton(B::FlyToggle)),
        (A::QuickAction, TouchVirtualButton(B::Shield)),
        (A::Start, TouchVirtualButton(B::Start)),
        (A::Reset, TouchVirtualButton(B::Reset)),
        (A::MenuBack, TouchVirtualButton(B::Reset)),
    ]
}

/// Add the touch virtual-device bindings to the participant's `InputMap`.
///
/// Runs on `Added`/`Changed` so a preset swap (which REPLACES the map
/// wholesale) re-binds touch; our own insertion bypasses change detection so
/// the write does not re-trigger this system into duplicate bindings.
pub fn bind_touch_virtual_inputs(
    mut maps: Query<
        &mut leafwing_input_manager::prelude::InputMap<ambition_input::SandboxAction>,
        (
            With<ambition_input::InputParticipant>,
            Changed<leafwing_input_manager::prelude::InputMap<ambition_input::SandboxAction>>,
        ),
    >,
) {
    use ambition_input::SandboxAction as A;
    for mut map in &mut maps {
        let map = map.bypass_change_detection();
        for (action, button) in touch_bindings() {
            map.insert(action, button);
        }
        map.insert_dual_axis(A::Move, TouchVirtualStick);
        map.insert_dual_axis(A::MenuStick, TouchVirtualStick);
        map.insert(A::MoveUp, TouchStickDirection::Up);
        map.insert(A::MoveDown, TouchStickDirection::Down);
        map.insert(A::MoveLeft, TouchStickDirection::Left);
        map.insert(A::MoveRight, TouchStickDirection::Right);
    }
}
