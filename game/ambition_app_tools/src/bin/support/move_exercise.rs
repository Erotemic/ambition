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
use bevy::prelude::App;

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


/// The move a verb is BOUND to on this fighter, as the composed host prepared it.
///
/// ⭐⭐ THE HOST'S ANSWER, NOT A SECOND MAPPING. A press is a REQUEST; the engine
/// decides. Without this a driver can only ask "did ANY move play", which calls
/// the known back-air-resolves-as-forward-air case a success and files the
/// forward air under `attack_air_back`.
pub fn intended_move(app: &mut App, character: &str, verb: &str) -> Option<String> {
    app.world()
        .get_resource::<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>()
        .and_then(|registry| registry.get(character))
        .and_then(|prepared| prepared.kit.projectable_moveset())
        .and_then(|set| set.verbs.get(verb).cloned())
}

/// How long the exercise HOLDS the button, in simulation ticks.
///
/// ⛔⛔ A CONSTANT, BECAUSE CAPTURE PARAMETERS MUST NOT CHANGE THE MOVE. The
/// renderer held while `shot < frames / 4`, so the hold depended on `--frames`
/// AND `--stride`: 24 frames at stride 2 held ~12 ticks, a 12-frame run held ~6,
/// and the recorder's own exercise holds ~37. Asking for more pictures charged
/// the smash differently, which means the two tools were photographing
/// different moves and neither said so.
pub const HOLD_TICKS: usize = 37;

/// The whole exercise, tick by tick: what to drive on simulation tick `n`.
///
/// ⭐ ONE SCHEDULE, TWO CONSUMERS. `moveset_takes` records every tick of it;
/// `moveset_render` photographs a subset. Which ticks are OBSERVED is the
/// caller's business; what the player does is not.
pub fn action_frame(verb: &Verb, action_tick: usize, facing: f32) -> ControlFrame {
    if action_tick == 0 {
        verb.frame(true, facing)
    } else if action_tick < HOLD_TICKS {
        verb.frame(false, facing)
    } else {
        // A charge move only pays out when the button comes UP.
        ControlFrame::default()
    }
}

/// Seat 0's facing, the axis a directional press is resolved against.
pub fn facing_of(app: &mut App) -> f32 {
    let world = app.world_mut();
    let mut q = world.query::<(
        &ambition_platformer2d::actor::MatchSeat,
        &ambition_platformer2d::engine_core::BodyKinematics,
    )>();
    q.iter(world)
        .find(|(seat, _)| seat.0 == 0)
        .map(|(_, kin)| kin.facing)
        .unwrap_or(1.0)
}

/// Is seat 0 on the ground? `None` when it has no ground state at all.
pub fn grounded(app: &mut App) -> Option<bool> {
    let world = app.world_mut();
    let mut q = world.query::<(
        &ambition_platformer2d::actor::MatchSeat,
        Option<&ambition_platformer2d::engine_core::BodyGroundState>,
    )>();
    q.iter(world)
        .find(|(seat, _)| seat.0 == 0)
        .map(|(_, g)| g.is_some_and(|g| g.on_ground))
}

/// Which move seat 0 is playing right now, if any.
pub fn playing_move(app: &mut App) -> Option<String> {
    let world = app.world_mut();
    let mut q = world.query::<(
        &ambition_platformer2d::actor::MatchSeat,
        &ambition_platformer2d::combat::moveset::MovePlayback,
    )>();
    q.iter(world)
        .find(|(seat, _)| seat.0 == 0)
        .map(|(_, play)| play.spec.id.clone())
}

/// Drive one control frame and advance one simulation tick.
pub fn step(app: &mut App, frame: ControlFrame) {
    ambition_platformer2d::sim::drive_control_frame(app.world_mut(), frame);
    app.update();
}

/// Bring the fighter to the posture this verb needs, and say whether it worked.
///
/// ⭐⭐ SHARED, BECAUSE THE TWO DRIVERS HAD DIFFERENT ALGORITHMS. The recorder
/// and the renderer each grew their own airborne loop, so "perform a back air"
/// meant two things and only one of them was ever tested.
///
/// ⛔ AIRBORNE IS CONFIRMED, NOT ASSUMED. An aerial pressed from the ground
/// reaches the GROUNDED chain and performs a different move — a take reported
/// `attack_air_down` as `smash_down` until this asked `BodyGroundState` instead
/// of counting frames after a jump.
///
/// ⛔ AND A HORIZONTAL AIM SETTLES FIRST. A back-air driven on the tick the stick
/// reverses resolves FORWARD, because the gesture resolver reads `-facing` while
/// a turnaround runs. Holding DOWN for the same settle would fast-fall back to
/// the floor, so only the horizontal axis is pre-aimed.
pub fn prepare(app: &mut App, verb: &Verb) -> bool {
    if !verb.airborne {
        return true;
    }
    for _ in 0..6 {
        step(app, ControlFrame { jump_pressed: true, jump_held: true, ..Default::default() });
        for _ in 0..10 {
            step(app, ControlFrame { jump_held: true, ..Default::default() });
        }
        if verb.axis_x != 0.0 {
            for _ in 0..8 {
                let aim = facing_of(app);
                step(app, ControlFrame {
                    axis_x: verb.axis_x * TILT_AXIS * aim.signum(),
                    ..Default::default()
                });
            }
        }
        if grounded(app) == Some(false) {
            return true;
        }
    }
    false
}
