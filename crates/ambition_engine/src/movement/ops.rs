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
    LedgeGrab,
    LedgeJump,
    LedgeClimbStart,
    LedgeClimbFinish,
    LedgeDrop,
    LedgeRoll,
    SwimStroke,
    Dash,
    DoubleDash,
    DodgeRoll,
    FlyToggle,
    Blink,
    PrecisionBlink,
    Pogo,
    Rebound,
    Slash,
    Reset,
    ShieldUp,
}

impl MovementOp {
    pub fn symbol(self) -> &'static str {
        match self {
            MovementOp::Jump => "J",
            MovementOp::DoubleJump => "DJ",
            MovementOp::WallJump => "WJ",
            MovementOp::WallCling => "WC",
            MovementOp::WallClimb => "W^",
            MovementOp::LedgeGrab => "LG",
            MovementOp::LedgeJump => "LJ",
            MovementOp::LedgeClimbStart => "LC",
            MovementOp::LedgeClimbFinish => "L^",
            MovementOp::LedgeDrop => "LD",
            MovementOp::LedgeRoll => "LR",
            MovementOp::SwimStroke => "SW",
            MovementOp::Dash => "D",
            MovementOp::DoubleDash => "DD",
            MovementOp::DodgeRoll => "DR",
            MovementOp::FlyToggle => "F",
            MovementOp::Blink => "B",
            MovementOp::PrecisionBlink => "PB",
            MovementOp::Pogo => "P",
            MovementOp::Rebound => "R",
            MovementOp::Slash => "S",
            MovementOp::Reset => "0",
            MovementOp::ShieldUp => "SH",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            MovementOp::Jump => "jump",
            MovementOp::DoubleJump => "double jump",
            MovementOp::WallJump => "wall jump",
            MovementOp::WallCling => "wall cling",
            MovementOp::WallClimb => "wall climb",
            MovementOp::LedgeGrab => "ledge grab",
            MovementOp::LedgeJump => "ledge jump",
            MovementOp::LedgeClimbStart => "ledge climb start",
            MovementOp::LedgeClimbFinish => "ledge climb finish",
            MovementOp::LedgeDrop => "ledge drop",
            MovementOp::LedgeRoll => "ledge roll",
            MovementOp::SwimStroke => "swim stroke",
            MovementOp::Dash => "dash",
            MovementOp::DoubleDash => "double dash",
            MovementOp::DodgeRoll => "dodge roll",
            MovementOp::FlyToggle => "fly toggle",
            MovementOp::Blink => "blink",
            MovementOp::PrecisionBlink => "precision blink",
            MovementOp::Pogo => "pogo",
            MovementOp::Rebound => "rebound",
            MovementOp::Slash => "slash",
            MovementOp::Reset => "reset",
            MovementOp::ShieldUp => "shield up",
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
