//! Smash's gameplay overrides. Shared menu conventions remain unchanged.
use ambition_platformer2d::input::{bind, BindingLayout, InputAction as Action, InputProfile, OverrideControl};
use bevy::prelude::GamepadButton as Pad;

pub fn layout() -> BindingLayout {
    let gameplay = InputProfile::Gameplay;
    BindingLayout::new([
        bind(gameplay, OverrideControl::Button(Pad::South),         Action::Attack),
        bind(gameplay, OverrideControl::Button(Pad::East),          Action::Jump),
        bind(gameplay, OverrideControl::Button(Pad::West),          Action::Special),
        bind(gameplay, OverrideControl::Button(Pad::North),         Action::Grab),
        bind(gameplay, OverrideControl::Button(Pad::DPadUp),        Action::Taunt),
        bind(gameplay, OverrideControl::Button(Pad::DPadDown),      Action::Taunt),
        bind(gameplay, OverrideControl::Button(Pad::LeftTrigger),   Action::Shield),
        bind(gameplay, OverrideControl::Button(Pad::LeftTrigger2),  Action::Shield),
        bind(gameplay, OverrideControl::Button(Pad::RightTrigger),  Action::ShortHop),
    ])
}
