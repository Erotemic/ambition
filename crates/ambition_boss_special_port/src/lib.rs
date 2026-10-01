//! The boss domain's extension trigger port.
//!
//! Port card (`docs/planning/engine/extension-domain-contracts.md`):
//!
//! * **Operation** — whether a boss's pattern pressed `Special(<key>)` this
//!   tick. The entry's selector is the key.
//! * **Owner** — `ambition_boss_encounter::extension::queue_boss_special_casts`
//!   reads `ActorActionMessage::Special` and the boss's components.
//! * **Scope** — one invocation for EACH boss, EACH tick the phase runs, for
//!   each bound key, with [`BossCaster::pressed`]. A boss pattern presses a
//!   special on every tick of its beat, so a technique that acts once per
//!   beat must see the ticks with no press to know that a beat ended. (The
//!   simulation tick cannot tell it: it also advances while gameplay is
//!   suspended, and a beat continues across a pause.) Several presses of one
//!   key by one boss in one tick are one press; the last gives the move use.
//! * **Time** — `technique_execution`. Kinematics and the tracked target are
//!   this tick's settled values.
//! * **Read model** — world units, y up; `facing` is the sign of the body's
//!   facing (`1.0` or `-1.0`).
//! * **Absence** — `target` is `None` when the boss tracks nothing.
//! * **Replay** — the value is derived each tick from rollback state; the
//!   port keeps nothing between ticks.

use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The trigger port marker.
pub struct BossSpecialCast;

impl Port for BossSpecialCast {
    const KEY: PortKey = PortKey::new("ambition.boss.special_cast", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = BossCaster;
}

/// The boss that pressed the special, at the read cut.
#[derive(Clone, Debug, PartialEq)]
pub struct BossCaster {
    /// True when the boss pressed the selector key this tick.
    pub pressed: bool,
    /// False when the boss has no health left.
    pub alive: bool,
    /// The body's position.
    pub position: [f32; 2],
    /// `1.0` faces +x, `-1.0` faces -x.
    pub facing: f32,
    /// Where the boss's authored shots leave its body: its position plus its
    /// authored projectile-origin offset (not mirrored by facing).
    pub launch_origin: [f32; 2],
    /// The centre of the body the boss tracks, or the tracked point when the
    /// target is not a body.
    pub target: Option<[f32; 2]>,
}
