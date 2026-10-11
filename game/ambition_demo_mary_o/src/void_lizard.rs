//! The Void Lizard: the boss of the castle.
//!
//! A lizard made of the void: a body with no light in it and a white edge. It
//! is an enemy body like the others, not a boss of the engine's boss encounter: it goes along its bridge
//! and turns at the post at each end (its row is the walker the AI Slop is),
//! and this module adds the two things its row can not say. It throws orbs
//! of the void along the bridge, toward the player, and it hops. A round of
//! the fight is seven seconds: a volley of three orbs (the second goes up at
//! a slant), a hop, and a pair of orbs. Each is on a tick of the simulation,
//! so the Void Lizard has no state of its own.
//!
//! No rule makes its top safe: a stomp on it is a hit. Sparks kill it (it has
//! the health of five), a shell or a quasar kills it, and it falls with its
//! bridge when the goal of the room is taken
//! (`bricks::drop_planks_when_the_goal_is_taken`).

use bevy::prelude::*;

use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::combat::actor_tuning::ActorConfig;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};

/// The brain key a level placement carries beside `character_id`.
pub const VOID_LIZARD_BRAIN_KEY: &str = "mary_o_void_lizard";
/// The catalog id, and the sheet target.
pub const VOID_LIZARD_CHARACTER_ID: &str = "void_lizard";
/// The display name of its row.
pub const VOID_LIZARD_DISPLAY_NAME: &str = "Void Lizard";
/// The `ProjectileVisualId` its orb authors (`provider.rs` registers it).
pub const VOID_ORB_VISUAL: &str = "mary_o_void_orb";

/// Ticks of one round of the fight.
const ROUND_TICKS: u64 = 420;
/// The ticks of a round on which it throws: a volley of three, and later a pair.
const THROW_TICKS: [u64; 5] = [0, 34, 68, 250, 288];
/// The throws of a round that go up at a slant: a body that jumps the low
/// orbs meets these.
const HIGH_THROW_TICKS: [u64; 2] = [34, 288];
/// The tick of a round on which it hops: after the volley, and before the pair.
const HOP_TICK: u64 = 165;
/// The speed of the hop, up (px/s). It is a low hop: the posts at the ends of
/// its bridge are three tiles high, and it must not go over them.
const HOP_SPEED: f32 = 300.0;
/// It throws only at a player this near, side to side (px).
pub const SIGHT: f32 = 620.0;

/// Is this body the Void Lizard?
pub fn is_void_lizard_brain(brain: &CharacterBrain) -> bool {
    matches!(brain, CharacterBrain::Custom(key) if key == VOID_LIZARD_BRAIN_KEY)
}

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

/// Ask the Void Lizard for its orb and its hop, on their ticks.
pub fn run_void_lizard(
    tick: Option<Res<ambition_platformer2d::time::SimTick>>,
    players: Query<&ae::BodyKinematics, With<PrimaryPlayer>>,
    mut bosses: Query<
        (&mut ActorControl, &mut ae::BodyKinematics, &ae::BodyGroundState, &ActorConfig, &BodyHealth),
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
    for (mut control, mut kin, ground, config, health) in &mut bosses {
        if !is_void_lizard_brain(&config.brain) || !health.alive() {
            continue;
        }
        let to_player = player.pos.x - kin.pos.x;
        if to_player.abs() > SIGHT {
            continue;
        }
        if throws_on(tick.get()) {
            // Along the bridge, toward the player, whichever way it goes.
            control.0.fire = Some(
                ambition_platformer2d::characters::actor::control::ActorFireRequest::world_space(
                    ae::Vec2::new(to_player.signum(), if throws_high_on(tick.get()) { -0.34 } else { 0.0 }),
                ),
            );
        }
        if hops_on(tick.get()) && ground.on_ground {
            ae::movement::set_jump_velocity(&mut kin.vel, ae::DEFAULT_GRAVITY_DIR, HOP_SPEED);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hop on the tick of a throw puts the Void Lizard in its own orb, and a
    /// player who jumps the orb meets the Void Lizard in the air.
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
