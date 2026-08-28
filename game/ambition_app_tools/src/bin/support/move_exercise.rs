//! ⭐⭐ HOW YOU MAKE A FIGHTER PERFORM A MOVE, in one place.
//!
//! This knowledge was proved inside `moveset_takes` over many wrong answers, and
//! every one of them is a rule here rather than a comment:
//!
//! - a full stick is a SMASH, so a tilt drives `0.65` (a directional take
//!   recorded the smash for every verb until this was measured);
//! - an aerial pressed from the ground reaches the GROUNDED chain, so an aerial
//!   verb must get airborne first and be confirmed airborne, not merely jumped;
//! - a back-air driven on the tick the stick reverses resolves as FORWARD,
//!   because the gesture resolver reads `-facing` during a turnaround — so a
//!   horizontal aim settles before the press;
//! - `special_pressed` is a rising EDGE: holding it true is a press every tick;
//! - and a charge move only pays out when the button comes UP.
//!
//! ⛔ A SOURCE MODULE, NOT A LIBRARY. `ambition_app_tools` has no `lib.rs` ON
//! PURPOSE — it is a collection of binaries rather than a layer — so the two
//! consumers `#[path]`-include this. If it ever becomes real reusable Smash
//! domain API it belongs in a domain crate, and moving it then is a rename.

use ambition_platformer2d::engine_core::ControlFrame;

/// A full stick is a SMASH. A tilt has to ask for less.
pub const TILT_AXIS: f32 = 0.65;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Button {
    Attack,
    Smash,
    Special,
    Grab,
    Taunt,
}

/// One press, as the genre spells it.
#[derive(Clone, Copy, Debug)]
pub struct Verb {
    /// The repertoire verb this drives, which is the key a viewer files it under.
    pub verb: &'static str,
    pub label: &'static str,
    pub axis_x: f32,
    pub axis_y: f32,
    pub button: Button,
    /// Jump first, and CONFIRM the apex. An aerial pressed from the ground
    /// reaches the grounded chain instead and performs the wrong move.
    pub airborne: bool,
}

impl Verb {
    /// The control frame for this verb, mirrored by the body's facing.
    ///
    /// ⛔ `edge` IS THE PRESS TICK. Every button here is a rising edge; holding
    /// the flag true is a press every tick, which re-triggers instead of holding.
    pub fn frame(&self, edge: bool, facing: f32) -> ControlFrame {
        // ⛔ A SMASH REACHES FULL STICK; a tilt must not. This is the measurement
        // that cost a whole recording: every directional take reported the SMASH
        // until the tilt stopped driving `1.0`.
        let reach = if self.button == Button::Smash {
            1.0
        } else {
            TILT_AXIS
        };
        let mut frame = ControlFrame {
            axis_x: self.axis_x * reach * facing.signum(),
            axis_y: self.axis_y * reach,
            ..Default::default()
        };
        match self.button {
            Button::Attack => {
                frame.attack_pressed = edge;
                frame.attack_held = true;
            }
            Button::Smash => {
                frame.attack_pressed = edge;
                frame.attack_held = true;
                // ⛔⛔ THE GESTURE THAT TELLS A TILT FROM A SMASH. Without it
                // every "smash" records the TILT, which looks like working data.
                frame.attack_strong_hint = true;
            }
            Button::Special => {
                frame.special_pressed = edge;
                frame.special_held = true;
            }
            Button::Grab => frame.grab_pressed = edge,
            Button::Taunt => frame.taunt_pressed = edge,
        }
        frame
    }
}

/// Look a verb up by its repertoire name.
pub fn verb_named(name: &str) -> Option<&'static Verb> {
    VERBS.iter().find(|v| v.verb == name)
}

/// Every verb this exercise knows how to perform.
///
/// ⚠ CAPTURE-STATE MOVES ARE ABSENT ON PURPOSE. A pummel or a throw needs a
/// GRABBED OPPONENT, which is a state this exercise cannot establish — and
/// listing them here would promise a capture the driver would then record as a
/// mismatch. They belong here when a scenario can set that state up.
pub const VERBS: &[Verb] = &[
    Verb {
        verb: "attack",
        label: "Jab",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: false,
    },
    Verb {
        verb: "attack_forward",
        label: "F-tilt",
        axis_x: 1.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: false,
    },
    Verb {
        verb: "attack_up",
        label: "U-tilt",
        axis_x: 0.0,
        axis_y: -1.0,
        button: Button::Attack,
        airborne: false,
    },
    Verb {
        verb: "attack_down",
        label: "D-tilt",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Attack,
        airborne: false,
    },
    Verb {
        verb: "smash_forward",
        label: "F-smash",
        axis_x: 1.0,
        axis_y: 0.0,
        button: Button::Smash,
        airborne: false,
    },
    Verb {
        verb: "smash_up",
        label: "U-smash",
        axis_x: 0.0,
        axis_y: -1.0,
        button: Button::Smash,
        airborne: false,
    },
    Verb {
        verb: "smash_down",
        label: "D-smash",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Smash,
        airborne: false,
    },
    Verb {
        verb: "attack_air",
        label: "N-air",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "attack_air_forward",
        label: "F-air",
        axis_x: 1.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "attack_air_back",
        label: "B-air",
        axis_x: -1.0,
        axis_y: 0.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "attack_air_up",
        label: "U-air",
        axis_x: 0.0,
        axis_y: -1.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "attack_air_down",
        label: "D-air",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Attack,
        airborne: true,
    },
    Verb {
        verb: "special",
        label: "Neutral B",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Special,
        airborne: false,
    },
    Verb {
        verb: "special_forward",
        label: "Side B",
        axis_x: 1.0,
        axis_y: 0.0,
        button: Button::Special,
        airborne: false,
    },
    // ⭐ THE UP-B IS RECORDED FROM THE AIR, which is the only place it is the
    // move Jon is asking about. A grounded up-B answers the same press and shows
    // none of the recovery.
    Verb {
        verb: "special_up",
        label: "Up B (airborne)",
        axis_x: 0.0,
        axis_y: -1.0,
        button: Button::Special,
        airborne: true,
    },
    Verb {
        verb: "special_down",
        label: "Down B",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Special,
        airborne: false,
    },
    Verb {
        verb: "special_air_down",
        label: "Down B (air)",
        axis_x: 0.0,
        axis_y: 1.0,
        button: Button::Special,
        airborne: true,
    },
    Verb {
        verb: "grab",
        label: "Grab",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Grab,
        airborne: false,
    },
    Verb {
        verb: "taunt",
        label: "Taunt",
        axis_x: 0.0,
        axis_y: 0.0,
        button: Button::Taunt,
        airborne: false,
    },
];
