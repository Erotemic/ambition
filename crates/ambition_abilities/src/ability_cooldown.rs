//! Shared per-body cooldown for movement abilities such as Blink and Grapple.
//! The component is inserted lazily on the acting body.

use bevy::prelude::*;

/// A body's movement-ability cooldown (seconds remaining until the next use).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AbilityCooldown {
    pub remaining: f32,
}

impl AbilityCooldown {
    pub fn ready(&self) -> bool {
        self.remaining <= 0.0
    }

    pub fn trigger(&mut self, seconds: f32) {
        self.remaining = seconds;
    }
}

/// Returns `true` and arms the cooldown if the ability may fire now; returns
/// `false` (blocking the fire) while it's still running. Pass the acting BODY's
/// optional cooldown component (from the ability's query) and a `Commands` so the
/// component is lazily inserted the first time that body uses an ability.
pub fn try_use_ability(
    cooldown: &mut Option<Mut<AbilityCooldown>>,
    commands: &mut Commands,
    body: Entity,
    seconds: f32,
) -> bool {
    match cooldown {
        Some(cd) => {
            if !cd.ready() {
                return false;
            }
            cd.trigger(seconds);
            true
        }
        None => {
            commands
                .entity(body)
                .insert(AbilityCooldown { remaining: seconds });
            true
        }
    }
}

/// Tick all body ability cooldowns using scaled simulation time.
pub fn tick_ability_cooldown(
    time: Res<ambition_time::WorldTime>,
    mut bodies: Query<&mut AbilityCooldown>,
) {
    let dt = time.scaled_dt;
    for mut cd in &mut bodies {
        if cd.remaining > 0.0 {
            cd.remaining = (cd.remaining - dt).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests;
