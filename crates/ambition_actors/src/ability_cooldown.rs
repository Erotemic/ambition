//! Shared cooldown for the movement abilities (Blink, Grapple) so they read as
//! deliberate verbs instead of spammable teleports.
//!
//! Stored as a per-BODY [`AbilityCooldown`] component, lazily inserted on first
//! use. A single shared timer per body is fine: a body holds one ability at a
//! time, so only the equipped ability's cooldown is ever in play.
//!
//! **Body-generic, not player-only (S5/S6 fold, refactor-chain R6a).** `blink`
//! and `grapple` already act on the `ControlledSubject` — any body the human is
//! driving — and arm the cooldown on THAT body. The tick used to filter
//! `With<PlayerEntity>, With<PrimaryPlayer>`, so a possessed actor's armed
//! cooldown never counted down and it could never blink again. The fold fixed a
//! real bug rather than merely relaxing a filter.

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

/// Tick EVERY body's ability cooldown down by scaled dt, so bullet-time / pause
/// slow it the same way they slow everything else.
///
/// No `With<PrimaryPlayer>` filter: a body only carries this component once it has
/// actually used an ability, and `blink`/`grapple` arm it on the
/// `ControlledSubject` — which may be a possessed actor. Filtering to the home
/// avatar left a possessed body's cooldown armed forever.
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
