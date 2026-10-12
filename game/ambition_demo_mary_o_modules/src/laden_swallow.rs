//! The Laden Swallow's brain: the bird that stays over its target and lets
//! coconuts go.
//!
//! Its row is the flyer the planes are (`Aerial` with no attack range), which
//! flies to a place over the body it sees and never dives. Its action set has
//! one ranged action, the coconut: a shot with gravity and no launch speed to
//! speak of. This module is the one thing the row can not say: when to let one
//! go, and that it goes straight down.
//!
//! The drop is a function of the simulation tick and the placement, so the
//! module has no state of its own. It is the ordinary fire of the body: the
//! shared moveset and the shared projectile path do the rest, and the
//! weapon's own recharge and its limit of live shots stay the authority on how
//! many fall. It decides in `actor_think`, before the control gate.

use ambition_combat_port::{ActorFire, ActorFirePort, ActorThink, ActorThinkPort};
use ambition_extension_sdk::{
    phases::ACTOR_THINK, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The brain key a level placement carries beside its `character_id`.
pub const BRAIN_KEY: &str = "mary_o_laden_swallow";

/// Ticks from one coconut to the next.
pub const DROP_EVERY_TICKS: u64 = 150;
/// It lets go only when its target is this near under it, side to side (px).
pub const DROP_REACH: f32 = 150.0;
/// Straight down in the world (+Y down): the shot's own gravity does the rest.
pub const DROP_DIRECTION: [f32; 2] = [0.0, 1.0];

/// Whether the swallow with this placement id lets a coconut go on `tick`.
/// Each placement has its own tick of the period, so two birds do not drop as
/// one.
pub fn drops_on(tick: u64, placement: &str) -> bool {
    let offset = placement.bytes().map(u64::from).sum::<u64>() % DROP_EVERY_TICKS;
    (tick + offset) % DROP_EVERY_TICKS == 0
}

/// Whether a swallow at `bird` is over a target at `target`.
pub fn over(bird: [f32; 2], target: [f32; 2]) -> bool {
    (bird[0] - target[0]).abs() <= DROP_REACH && bird[1] < target[1]
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "laden_swallow"),
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
            requests: vec![ActorFirePort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(think),
        }],
    }
}

fn think(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let body: ActorThink = inv.trigger::<ActorThinkPort>()?.clone();
    let (Some(target), Some(placement)) = (body.target, body.placement.as_deref()) else {
        return Ok(());
    };
    if over(body.position, target) && drops_on(inv.tick(), placement) {
        inv.submit::<ActorFirePort>(ActorFire { direction: DROP_DIRECTION })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One drop for each period, for each bird, and two birds on two ticks.
    #[test]
    fn each_bird_drops_one_time_in_a_period_on_its_own_tick() {
        let ticks = |placement: &str| -> Vec<u64> {
            (0..DROP_EVERY_TICKS).filter(|tick| drops_on(*tick, placement)).collect()
        };
        let (a, b) = (ticks("EnemySpawn-1"), ticks("EnemySpawn-2"));
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 1);
        assert_ne!(a, b);
    }
}
