//! A technique's ask for the observer's slow (`ambition.body.observer_slow`
//! in `technique_execution`): Contextual Blink's choice of a foe.
//!
//! The ask is held on the body with the tick it was made ([`ObserverSlowAsked`]),
//! and the runtime's time control slows the world for the primary observer's
//! body while the ask is fresh. A technique that wants the slow asks every
//! tick; one that stops asking lets the world ease back.

use ambition_combat_port::ObserverSlowPort;
use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use bevy::prelude::*;

/// Install the ask's lowering in `technique_execution`.
pub fn install_for_technique_execution(app: &mut App) {
    app.install_extension_submitted_request::<ObserverSlowPort, _>(TECHNIQUE_EXECUTION, "ambition_abilities", hold_slows);
}

/// The last tick a technique asked the observer's slow for this body.
///
/// Rollback state: it decides how fast the world runs next tick.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObserverSlowAsked {
    pub tick: u64,
}

impl ObserverSlowAsked {
    /// Whether the ask holds on tick `now`: it was made on this tick or the
    /// one before. An ask stamped after `now` (a rewind restored an earlier
    /// tick) does not hold.
    pub fn holds_on(&self, now: u64) -> bool {
        self.tick <= now && now - self.tick <= 1
    }
}

/// The rollback value projection: the tick of the ask.
pub fn observer_slow_probe(asked: &ObserverSlowAsked) -> u64 {
    asked.tick
}

fn hold_slows(
    mut outbox: ResMut<ExtensionOutbox>,
    tick: Option<Res<ambition_time::SimTick>>,
    bodies: Query<(), With<ambition_platformer2d_core::BodyKinematics>>,
    mut commands: Commands,
) {
    let now = tick.map_or(0, |tick| tick.0);
    for submitted in outbox.drain::<ObserverSlowPort>(&TECHNIQUE_EXECUTION) {
        if bodies.get(submitted.scope).is_err() {
            warn!("extension entry {} asked the observer's slow for a body with no kinematics; refused", submitted.entry);
            continue;
        }
        commands.entity(submitted.scope).try_insert(ObserverSlowAsked { tick: now });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ask_holds_on_its_tick_and_the_next_only() {
        let asked = ObserverSlowAsked { tick: 10 };
        assert!(asked.holds_on(10));
        assert!(asked.holds_on(11));
        assert!(!asked.holds_on(12));
        // A rewind to before the ask: the ask is from a future it has not made.
        assert!(!asked.holds_on(9));
    }
}
