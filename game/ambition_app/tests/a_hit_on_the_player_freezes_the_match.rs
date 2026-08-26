#![cfg(feature = "rl_sim")]
//! ⛔⛔ THE MATCH FREEZE WORKS WHEN A CPU IS HIT AND NOT WHEN THE HUMAN IS.
//!
//! `request_impact_hitstop_on_landed_hits` runs in `CombatSet::Settle` and reads
//! `BodyCombat::hitstop_timer` off the victim. Player-victim hits are staged into
//! a rollback-registered FIFO that the player resolver drains in NEXT frame's
//! `PlayerSimulation`, so on the frame the freeze is decided that timer is still
//! zero. The connect that stops the world for a CPU does not stop it for the
//! person holding the controller — a feel difference a player notices without
//! knowing why.
//!
//! ⛔ EVERY EXISTING ARM IS BLIND TO THIS. The unit tests inject `hitstop_timer`
//! on the victim before firing the message, and the app-level duel
//! (`hit_shakes_the_camera`) deliberately REMOVES the home avatar. Only a real
//! schedule, with a real enemy hitting the real player, can see the ordering.

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::actors::actor::{BodyKinematics, PrimaryPlayerOnly};
use ambition_platformer2d::characters::actor::BodyCombat;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use bevy::prelude::World;

const ENEMY_ID: &str = "freeze_aggressor";

fn player_pos(world: &mut World) -> ambition_platformer2d::engine_core::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

#[derive(Debug, Default)]
struct Bout {
    /// The hardest hitlag the PLAYER served — proof the player was hit at all.
    player_hitstop: f32,
    /// The hardest any OTHER body served.
    other_hitstop: f32,
    /// Frames on which the match was frozen.
    frozen_frames: u32,
}

/// ⛔⛔ IGNORED, AND THE REASON IS THE HANDOFF. The fix was BUILT and it works —
/// a `ResolvedBodyHit` message carrying the resolver's own hitlag, published by
/// both damage roads and read by the freeze in place of the victim's timer. This
/// test went green with it. It was reverted because landing it exposed a
/// SECOND, PRE-EXISTING defect that turns the repaired freeze into a 50% duty
/// cycle: measured on `smash_in_the_host`, one strike RESOLVES SEVENTEEN TIMES
/// across twenty-three ticks on the actor road, so a freeze re-armed by every
/// resolution alternates frozen/moving forever and a fixture that budgets frames
/// never reaches the run gait. Refusing to arm while already frozen halves it
/// and no more.
///
/// ⇒ the repeat is the item to fix first; the freeze channel is correct and
/// waiting. See the D237 (4)/(5) rows.
#[ignore = "the fix is built and reverted: a pre-existing 17x repeat resolution makes the repaired freeze a 50% duty cycle — see the note above and D237 (4)/(5)"]
#[test]
fn a_hit_that_lands_on_the_player_freezes_the_match() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");

    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_character_at(
        ENEMY_ID,
        "Perfect Cellular Automaton",
        (p.x + 60.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
        "perfect_cellular_automaton",
    );

    let mut bout = Bout::default();
    // Stand still and be hit. The player never presses anything, so it is the
    // only victim in the room and the freeze cannot be somebody else's.
    for _ in 0..300 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let player_stop = {
            let mut q = world.query_filtered::<&BodyCombat, PrimaryPlayerOnly>();
            q.iter(world).map(|c| c.hitstop_timer).fold(0.0, f32::max)
        };
        let all_stop = {
            let mut q = world.query::<&BodyCombat>();
            q.iter(world).map(|c| c.hitstop_timer).fold(0.0, f32::max)
        };
        bout.player_hitstop = bout.player_hitstop.max(player_stop);
        bout.other_hitstop = bout.other_hitstop.max(all_stop - player_stop).max(0.0);
        let tick = *world.resource::<ambition_platformer2d::time::SimTick>();
        if world
            .resource::<ambition_platformer2d::combat::impact_hitstop::ImpactHitstop>()
            .is_freezing(tick)
        {
            bout.frozen_frames += 1;
        }
    }

    println!("freeze bout: {bout:#?}");
    assert!(
        bout.player_hitstop > 0.0,
        "the enemy never landed a hit on the player, so this fixture measures \
         nothing: {bout:#?}"
    );
    assert!(
        bout.frozen_frames > 0,
        "the player was hit hard enough to serve {:.3}s of its own hitlag and \
         the MATCH never froze — the freeze is decided in Settle from the \
         victim's timer, and a player victim's timer is not written until the \
         next frame: {bout:#?}",
        bout.player_hitstop
    );
}
