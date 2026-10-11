//! The Void Lizard: the boss of the castle.
//!
//! A lizard made of the void: a body with no light in it and a white edge. It
//! is an enemy body like the others, not a boss of the engine's boss encounter: it goes along its bridge
//! and turns at the post at each end (its row is the walker the AI Slop is),
//! and this module adds the two things its row can not say. It throws an orb
//! of the void along the bridge, toward the player, and it hops. Each is on a
//! tick of the simulation, so the Void Lizard has no state of its own.
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

/// Ticks from one orb to the next.
const ORB_EVERY_TICKS: u64 = 170;
/// Ticks from one hop to the next: two orbs. A period that is not a whole
/// number of orbs puts a hop on the tick of a throw sooner or later.
const HOP_EVERY_TICKS: u64 = ORB_EVERY_TICKS * 2;
/// The hop is this many ticks after an orb, so it does not hop into its own orb.
const HOP_AFTER_ORB_TICKS: u64 = 90;
/// The speed of the hop, up (px/s): high, so a body can run under it.
const HOP_SPEED: f32 = 700.0;
/// It throws only at a player this near, side to side (px).
const SIGHT: f32 = 560.0;

/// Is this body the Void Lizard?
pub fn is_void_lizard_brain(brain: &CharacterBrain) -> bool {
    matches!(brain, CharacterBrain::Custom(key) if key == VOID_LIZARD_BRAIN_KEY)
}

/// Whether the Void Lizard throws an orb on `tick`.
pub fn throws_on(tick: u64) -> bool {
    tick % ORB_EVERY_TICKS == 0
}

/// Whether the Void Lizard hops on `tick`.
pub fn hops_on(tick: u64) -> bool {
    tick % HOP_EVERY_TICKS == HOP_AFTER_ORB_TICKS
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
                    ae::Vec2::new(to_player.signum(), 0.0),
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
        let period = HOP_EVERY_TICKS * 3;
        let (mut throws, mut hops) = (0, 0);
        for tick in 0..period {
            assert!(!(throws_on(tick) && hops_on(tick)), "it hops into its orb on tick {tick}");
            throws += usize::from(throws_on(tick));
            hops += usize::from(hops_on(tick));
        }
        assert!(throws > 0 && hops > 0);
    }
}
