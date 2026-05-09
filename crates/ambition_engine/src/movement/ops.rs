use std::fmt;

/// A symbolic movement operation that can be shown in the debug HUD.
///
/// These are the first seeds of the "movement algebra" concept: order matters,
/// and the game can explain advanced movement as compositions of simple verbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementOp {
    Jump,
    DoubleJump,
    WallJump,
    WallCling,
    WallClimb,
    Dash,
    DoubleDash,
    FlyToggle,
    Blink,
    PrecisionBlink,
    Pogo,
    Rebound,
    Slash,
    Reset,
}

impl MovementOp {
    pub fn symbol(self) -> &'static str {
        match self {
            MovementOp::Jump => "J",
            MovementOp::DoubleJump => "DJ",
            MovementOp::WallJump => "WJ",
            MovementOp::WallCling => "WC",
            MovementOp::WallClimb => "W^",
            MovementOp::Dash => "D",
            MovementOp::DoubleDash => "DD",
            MovementOp::FlyToggle => "F",
            MovementOp::Blink => "B",
            MovementOp::PrecisionBlink => "PB",
            MovementOp::Pogo => "P",
            MovementOp::Rebound => "R",
            MovementOp::Slash => "S",
            MovementOp::Reset => "0",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            MovementOp::Jump => "jump",
            MovementOp::DoubleJump => "double jump",
            MovementOp::WallJump => "wall jump",
            MovementOp::WallCling => "wall cling",
            MovementOp::WallClimb => "wall climb",
            MovementOp::Dash => "dash",
            MovementOp::DoubleDash => "double dash",
            MovementOp::FlyToggle => "fly toggle",
            MovementOp::Blink => "blink",
            MovementOp::PrecisionBlink => "precision blink",
            MovementOp::Pogo => "pogo",
            MovementOp::Rebound => "rebound",
            MovementOp::Slash => "slash",
            MovementOp::Reset => "reset",
        }
    }
}

impl fmt::Display for MovementOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.symbol())
    }
}

/// A timestamped combo entry for debug display and future scoring/teaching.
#[derive(Clone, Debug)]
pub struct ComboMark {
    pub op: MovementOp,
    pub age: f32,
}

