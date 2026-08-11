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
    LedgeGetupAttack,
    SwimStroke,
    Dash,
    DoubleDash,
    DodgeRoll,
    /// The AERIAL evade — see `apply_dodge`. Distinct from [`Self::DodgeRoll`]
    /// so a listener (animation, fx, a causal trace) can tell a body that rolled
    /// along the floor from one that spent its airtime.
    AirDodge,
    /// Launched into tumble — helpless until it decays or the floor decides.
    Tumble,
    /// A teched landing: the knockdown refused.
    Tech,
    /// Landed while tumbling without teching — prone.
    Knockdown,
    /// Stood up from a knockdown (by choice or by timeout).
    Getup,
    /// Rolled out of a knockdown.
    GetupRoll,
    /// Stood up swinging. The kernel publishes the option; combat answers it.
    GetupAttack,
    FlyToggle,
    Blink,
    PrecisionBlink,
    Pogo,
    Rebound,
    /// The crawler SEATED itself on a surface this step.
    ///
    /// ⛔ this locomotion mode published NOTHING before 2026-08-02 — not because
    /// nobody wrote the op, but because `step_crawler` was handed `&mut
    /// Vec<Contact>` rather than the whole [`super::FrameEvents`], so the
    /// operations channel was not in scope for it to write to. A whole movement
    /// policy was structurally unable to say what it did, and the causal
    /// instrument that answered the ladder question is blind to exactly the mode
    /// with an open contact bug.
    CrawlAttach,
    /// The crawler LEFT its surface this step (knocked off, walked off an end,
    /// or the surface stopped qualifying).
    CrawlDetach,
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
            MovementOp::LedgeGetupAttack => "LA",
            MovementOp::SwimStroke => "SW",
            MovementOp::Dash => "D",
            MovementOp::DoubleDash => "DD",
            MovementOp::DodgeRoll => "DR",
            MovementOp::AirDodge => "AD",
            MovementOp::Tumble => "TB",
            MovementOp::Tech => "TC",
            MovementOp::Knockdown => "KD",
            MovementOp::Getup => "GU",
            MovementOp::GetupRoll => "GR",
            MovementOp::GetupAttack => "GA",
            MovementOp::FlyToggle => "F",
            MovementOp::Blink => "B",
            MovementOp::PrecisionBlink => "PB",
            MovementOp::Pogo => "P",
            MovementOp::Rebound => "R",
            MovementOp::CrawlAttach => "CA",
            MovementOp::CrawlDetach => "CD",
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
            MovementOp::LedgeGetupAttack => "ledge getup attack",
            MovementOp::SwimStroke => "swim stroke",
            MovementOp::Dash => "dash",
            MovementOp::DoubleDash => "double dash",
            MovementOp::DodgeRoll => "dodge roll",
            MovementOp::AirDodge => "air dodge",
            MovementOp::Tumble => "tumble",
            MovementOp::Tech => "tech",
            MovementOp::Knockdown => "knockdown",
            MovementOp::Getup => "getup",
            MovementOp::GetupRoll => "getup roll",
            MovementOp::GetupAttack => "getup attack",
            MovementOp::FlyToggle => "fly toggle",
            MovementOp::Blink => "blink",
            MovementOp::PrecisionBlink => "precision blink",
            MovementOp::Pogo => "pogo",
            MovementOp::Rebound => "rebound",
            MovementOp::CrawlAttach => "crawl attach",
            MovementOp::CrawlDetach => "crawl detach",
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
