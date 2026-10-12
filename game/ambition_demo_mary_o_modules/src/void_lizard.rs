//! The Void Lizard's brain: the boss of the castle.
//!
//! Its row walks it along its bridge and turns it at the post at each end
//! (the walker the AI Slop is). This module adds the two things the row can
//! not say. It throws orbs of the void along the bridge, toward its target,
//! and it hops. A round of the fight is seven seconds: a volley of three orbs
//! (the second goes up at a slant), a hop, and a pair of orbs. Each is on a
//! tick of the simulation, so the module has no state of its own.
//!
//! It decides in `actor_think`, before the control gate: a restriction on the
//! body (a script that holds it) removes its throw as it removes any fire.

use ambition_combat_port::{ActorFire, ActorFirePort, ActorThink, ActorThinkPort, SetVelocity, SetVelocityPort};
use ambition_extension_sdk::{
    phases::ACTOR_THINK, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The brain key a level placement carries beside its `character_id`.
pub const BRAIN_KEY: &str = "mary_o_void_lizard";

/// Ticks of one round of the fight.
pub const ROUND_TICKS: u64 = 420;
/// The ticks of a round on which it throws: a volley of three, and later a pair.
const THROW_TICKS: [u64; 5] = [0, 34, 68, 250, 288];
/// The throws of a round that go up at a slant: a body that jumps the low
/// orbs meets these.
const HIGH_THROW_TICKS: [u64; 2] = [34, 288];
/// How far up a high throw goes, for each unit along the bridge.
pub const HIGH_THROW_RISE: f32 = 0.34;
/// The tick of a round on which it hops: after the volley, and before the pair.
const HOP_TICK: u64 = 165;
/// The speed of the hop, up (px/s). It is a low hop: the posts at the ends of
/// its bridge are three tiles high, and it must not go over them.
pub const HOP_SPEED: f32 = 300.0;
/// It acts only when its target is this near, side to side (px).
pub const SIGHT: f32 = 620.0;

/// Whether the Void Lizard throws an orb on `tick`.
pub fn throws_on(tick: u64) -> bool {
    THROW_TICKS.contains(&(tick % ROUND_TICKS))
}

/// Whether the throw of `tick` goes up at a slant.
pub fn throws_high_on(tick: u64) -> bool {
    HIGH_THROW_TICKS.contains(&(tick % ROUND_TICKS))
}

/// Whether the Void Lizard hops on `tick`.
pub fn hops_on(tick: u64) -> bool {
    tick % ROUND_TICKS == HOP_TICK
}

/// The way the throw of `tick` goes, for a target `to_target` along the
/// bridge (+X right, +Y down).
pub fn throw_direction(tick: u64, to_target: f32) -> [f32; 2] {
    [to_target.signum(), if throws_high_on(tick) { -HIGH_THROW_RISE } else { 0.0 }]
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "void_lizard"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            key: "think".into(),
            phase: ACTOR_THINK,
            trigger: TriggerBinding {
                port: ActorThinkPort::KEY,
                selector: BRAIN_KEY.into(),
            },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![ActorFirePort::KEY, SetVelocityPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(think),
        }],
    }
}

fn think(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let body: ActorThink = inv.trigger::<ActorThinkPort>()?.clone();
    let Some(target) = body.target else {
        return Ok(());
    };
    let to_target = target[0] - body.position[0];
    if to_target.abs() > SIGHT {
        return Ok(());
    }
    let tick = inv.tick();
    if throws_on(tick) {
        // Along the bridge, toward the target, whichever way it goes.
        inv.submit::<ActorFirePort>(ActorFire { direction: throw_direction(tick, to_target) })?;
    }
    if hops_on(tick) && body.on_ground {
        // Up against gravity (+Y down); its walk keeps the side velocity.
        inv.submit::<SetVelocityPort>(SetVelocity { velocity: [body.velocity[0], -HOP_SPEED] })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hop on the tick of a throw puts the Void Lizard in its own orb, and a
    /// body that jumps the orb meets the Void Lizard in the air. It also keeps
    /// one request for each call (`Limits::max_requests`).
    #[test]
    fn the_void_lizard_does_not_hop_on_the_tick_it_throws() {
        let period = ROUND_TICKS * 2;
        let (mut throws, mut hops) = (0, 0);
        for tick in 0..period {
            assert!(!(throws_on(tick) && hops_on(tick)), "it hops into its orb on tick {tick}");
            throws += usize::from(throws_on(tick));
            hops += usize::from(hops_on(tick));
        }
        assert!(throws > 0 && hops > 0);
    }
}
