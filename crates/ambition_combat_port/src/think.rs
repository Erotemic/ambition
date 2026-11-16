//! The ports of a custom brain: a body whose brain is a `CharacterBrain::Custom`
//! key that a module binds decides through `actor_think`
//! (`ambition_extension_sdk::phases::ACTOR_THINK`).

use ambition_extension_sdk::wire::{WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The trigger port marker for a custom brain.
///
/// Port card:
///
/// * **Operation** — a live body whose brain is the bound key decides this
///   tick. The selector is the `CharacterBrain::Custom` key the placement
///   authors. A body a player drives, and a dead body, does not decide.
/// * **Control** — a controller's hook: it publishes the body's intent while
///   its brain controls it, and yields when a player drives it. What must
///   last across a change of controller (a coating, a shield, a carried
///   thing) is a status the hook holds on the body (`hold_status`), ticked
///   by `status_tick` whoever drives it.
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
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<ActorThink, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}

ambition_extension_sdk::port_record! {
    ActorThink {
        /// The body's centre (world units, +Y down).
        position: [f32; 2],
        /// Its velocity (world units per second).
        velocity: [f32; 2],
        /// Its box, width and height.
        size: [f32; 2],
        /// The way it faces: `1.0` is +x, `-1.0` is -x.
        facing: f32,
        /// Whether it stands on the ground.
        on_ground: bool,
        /// Its health.
        health: i32,
        /// Where its target is now, or `()` when it has none.
        target: Option<[f32; 2]>,
        /// The id of the placement that built it, or `()` for a body no
        /// placement built.
        placement: Option<String>,
    }
}

ambition_extension_sdk::script_port!(ActorThinkPort);

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
        ambition_extension_sdk::structure::put(v, out)
    }

    fn decode(r: &mut WireReader<'_>) -> Result<ActorFire, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}

ambition_extension_sdk::port_record! {
    ActorFire {
        /// Which way to fire (world frame; its length does not matter).
        direction: [f32; 2],
    }
}

ambition_extension_sdk::script_port!(ActorFirePort);

#[cfg(test)]
mod described_tests {
    use super::*;
    use ambition_extension_sdk::wire;

    /// Described structurally, the brain's trigger and its fire keep the
    /// bytes their hand-written codecs published, with and without the
    /// optional fields.
    #[test]
    fn the_described_brain_ports_keep_their_bytes() {
        for (target, placement) in [(Some([3.0, -4.0]), Some("EnemySpawn-7".to_owned())), (None, None)] {
            let think = ActorThink {
                position: [1.0, 2.0],
                velocity: [-5.0, 6.5],
                size: [24.0, 30.0],
                facing: -1.0,
                on_ground: true,
                health: 3,
                target,
                placement,
            };
            let (mut described, mut by_hand) = (Vec::new(), Vec::new());
            ActorThinkPort::encode(&think, &mut described);
            wire::put_vec2(&mut by_hand, think.position);
            wire::put_vec2(&mut by_hand, think.velocity);
            wire::put_vec2(&mut by_hand, think.size);
            wire::put_f32(&mut by_hand, think.facing);
            wire::put_bool(&mut by_hand, think.on_ground);
            wire::put_i32(&mut by_hand, think.health);
            wire::put_opt(&mut by_hand, think.target, wire::put_vec2);
            wire::put_opt(&mut by_hand, think.placement.as_deref(), wire::put_str);
            assert_eq!(described, by_hand);
            let mut r = WireReader::new(&described);
            assert_eq!(ActorThinkPort::decode(&mut r).unwrap(), think);
            r.finish().unwrap();
        }
        let fire = ActorFire { direction: [0.0, 1.0] };
        let (mut described, mut by_hand) = (Vec::new(), Vec::new());
        ActorFirePort::encode(&fire, &mut described);
        wire::put_vec2(&mut by_hand, fire.direction);
        assert_eq!(described, by_hand);
        let velocity = crate::SetVelocity { velocity: [120.0, -30.0] };
        let (mut described, mut by_hand) = (Vec::new(), Vec::new());
        crate::SetVelocityPort::encode(&velocity, &mut described);
        wire::put_vec2(&mut by_hand, velocity.velocity);
        assert_eq!(described, by_hand);
    }
}
