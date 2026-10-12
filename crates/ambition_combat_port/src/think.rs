//! The ports of a custom brain: a body whose brain is a `CharacterBrain::Custom`
//! key that a module binds decides through `actor_think`
//! (`ambition_extension_sdk::phases::ACTOR_THINK`).

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The trigger port marker for a custom brain.
///
/// Port card:
///
/// * **Operation** — a live body whose brain is the bound key decides this
///   tick. The selector is the `CharacterBrain::Custom` key the placement
///   authors. A body a player drives, and a dead body, does not decide.
/// * **Owner** — `ambition_combat::extension::think`.
/// * **Time** — `actor_think`: after the brains published their control
///   frames, and before the control gate. One invocation for each body, in
///   the order of the body's simulation identity.
/// * **Read model** — world units, +Y down; the body's own facts as they are
///   at the start of the phase, and its target's position now.
pub struct ActorThinkPort;

/// What a custom brain reads about its body.
#[derive(Clone, Debug, PartialEq)]
pub struct ActorThink {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub size: [f32; 2],
    pub facing: f32,
    pub on_ground: bool,
    pub health: i32,
    /// Where the body's target is now. `None` when it has no target.
    pub target: Option<[f32; 2]>,
    /// The id of the placement that built the body (`FeatureId`): a module
    /// that gives each placement its own beat reads it. `None` for a body no
    /// placement built.
    pub placement: Option<String>,
}

impl Port for ActorThinkPort {
    /// Version 2 (2026-10-11): `placement`.
    const KEY: PortKey = PortKey::new("ambition.actor.think", 2);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = ActorThink;

    fn encode(v: &ActorThink, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.position);
        wire::put_vec2(out, v.velocity);
        wire::put_vec2(out, v.size);
        wire::put_f32(out, v.facing);
        wire::put_bool(out, v.on_ground);
        wire::put_i32(out, v.health);
        wire::put_opt(out, v.target, wire::put_vec2);
        wire::put_opt(out, v.placement.as_deref(), wire::put_str);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<ActorThink, WireError> {
        Ok(ActorThink {
            position: r.vec2()?,
            velocity: r.vec2()?,
            size: r.vec2()?,
            facing: r.f32()?,
            on_ground: r.bool()?,
            health: r.i32()?,
            target: r.opt(WireReader::vec2)?,
            placement: r.opt(|r| r.str().map(str::to_owned))?,
        })
    }
}

/// The request port marker for a body's fire intent.
///
/// Port card:
///
/// * **Operation** — the body's control frame asks to fire toward `direction`
///   (world frame; the length does not matter). It is the `fire` of the
///   frame a brain publishes, and the body's moveset fires it as it fires a
///   brain's.
/// * **Owner** — `ambition_combat::extension::think` (`ActorControl`).
/// * **Time** — `actor_think`, before the control gate: a restriction on the
///   body (a script that holds it, a capture) removes it as it removes a
///   brain's fire.
/// * **Result** — submitted is not applied. A body with no ranged move fires
///   nothing.
pub struct ActorFirePort;

#[derive(Clone, Debug, PartialEq)]
pub struct ActorFire {
    pub direction: [f32; 2],
}

impl Port for ActorFirePort {
    const KEY: PortKey = PortKey::new("ambition.actor.fire", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = ActorFire;

    fn encode(v: &ActorFire, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.direction);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<ActorFire, WireError> {
        Ok(ActorFire { direction: r.vec2()? })
    }
}
