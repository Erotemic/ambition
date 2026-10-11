//! The Laden Swallow: the bird that stays over the player and lets coconuts go.
//!
//! Its row is the flyer the planes are (`Aerial` with no attack range), which
//! flies to a place over the body it sees and never dives. Its action set has
//! one ranged action, the coconut: a shot with gravity and no launch speed to
//! speak of. This module is the one thing the row can not say: when to let one
//! go, and that it goes straight down.
//!
//! The drop is a function of the simulation tick, so it is no state of its
//! own. It raises the ordinary `fire` intent of the body; the shared moveset
//! and the shared projectile path do the rest, and the weapon's own recharge
//! and its limit of live shots stay the authority on how many fall.
//!
//! A swallow is squashed from above as a walker is
//! (`ai_slop::bounce_squash_ai_slop`).

use bevy::prelude::*;

use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};

/// The brain key a level placement carries beside `character_id`.
pub const LADEN_SWALLOW_BRAIN_KEY: &str = "mary_o_laden_swallow";
/// The catalog id. The sheet is the African swallow's.
pub const LADEN_SWALLOW_CHARACTER_ID: &str = "laden_swallow";
/// The display name of its row.
pub const LADEN_SWALLOW_DISPLAY_NAME: &str = "Laden Swallow";
/// The `ProjectileVisualId` its coconut authors (`provider.rs` registers it).
pub const COCONUT_VISUAL: &str = "mary_o_coconut";

/// Ticks from one coconut to the next.
const DROP_EVERY_TICKS: u64 = 150;
/// It lets go only when the player is this near under it, side to side (px).
const DROP_REACH: f32 = 150.0;

/// Is this body a Laden Swallow?
pub fn is_laden_swallow_brain(brain: &CharacterBrain) -> bool {
    matches!(brain, CharacterBrain::Custom(key) if key == LADEN_SWALLOW_BRAIN_KEY)
}

/// Whether the swallow with this placement id lets a coconut go on `tick`.
/// Each placement has its own tick of the period, so two birds do not drop as
/// one.
pub fn drops_on(tick: u64, placement: &str) -> bool {
    let offset = placement.bytes().map(u64::from).sum::<u64>() % DROP_EVERY_TICKS;
    (tick + offset) % DROP_EVERY_TICKS == 0
}

/// Ask each swallow that is over a player to let a coconut go, on its tick.
pub fn drop_coconuts(
    tick: Option<Res<ambition_platformer2d::time::SimTick>>,
    players: Query<&ae::BodyKinematics, With<PrimaryPlayer>>,
    mut birds: Query<
        (&mut ActorControl, &ae::BodyKinematics, &ActorConfig, &BodyHealth, &FeatureId),
        (Without<PrimaryPlayer>, Without<PlayerEntity>),
    >,
) {
    // A world with no simulation clock (a test of one rule) has no cycle.
    let Some(tick) = tick else {
        return;
    };
    let Ok(player) = players.single() else {
        return;
    };
    for (mut control, kin, config, health, placement) in &mut birds {
        if !is_laden_swallow_brain(&config.brain) || !health.alive() {
            continue;
        }
        let over_the_player =
            (kin.pos.x - player.pos.x).abs() <= DROP_REACH && kin.pos.y < player.pos.y;
        if !over_the_player || !drops_on(tick.get(), placement.as_str()) {
            continue;
        }
        // Straight down in the world: the shot's own gravity does the rest.
        control.0.fire = Some(
            ambition_platformer2d::characters::actor::control::ActorFireRequest::world_space(
                ae::DEFAULT_GRAVITY_DIR,
            ),
        );
    }
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
