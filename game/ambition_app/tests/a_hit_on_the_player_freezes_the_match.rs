#![cfg(feature = "rl_sim")]
//! ⛔⛔ THE MATCH FREEZE WORKS WHEN A CPU IS HIT AND NOT WHEN THE HUMAN IS.
//!
//! `request_impact_hitstop_on_landed_hits` used to run in `CombatSet::Settle` and read
//! `BodyCombat::hitstop_timer` off the victim. Player-victim hits are staged into
//! a rollback-registered FIFO that the player resolver drains in NEXT frame's
//! `PlayerSimulation`, so on the frame the freeze is decided that timer is still
//! zero. The connect that stops the world for a CPU does not stop it for the
//! person holding the controller — a feel difference a player notices without
//! knowing why.
//!
//! ⛔ EVERY EXISTING ARM WAS BLIND TO THIS. The unit tests inject `hitstop_timer`
//! on the victim before firing the message — which answers the question before
//! asking it — and the app-level duel (`hit_shakes_the_camera`) deliberately
//! REMOVES the home avatar. Only a real schedule, with a real enemy hitting the
//! real player, can see the ordering, which is why this fixture exists at all.
//!
//! ⭐ FIXED by `ResolvedBodyHit`: the resolver publishes its OWN hitlag beside
//! the reaction, so the freeze never has to know which road resolved the hit or
//! on which frame.

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::characters::actor::BodyCombat;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::World;

const ENEMY_ID: &str = "freeze_aggressor";

fn player_pos(world: &mut World) -> ambition_platformer2d::engine_core::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

/// ⭐ THE ENEMY STARTS OUTSIDE ITS OWN REACH AND WALKS IN, and that is the
/// fixture's whole design. Placed close, the automaton's contact footprint is
/// what reaches the player first: measured over 900 frames at +26, +60 and
/// +100, every hit is `HitSource::Contact` — attrition, which correctly freezes
/// nothing, so the fixture reports 0.036s of player hitlag and no freeze and
/// looks like the bug it is guarding against.
///
/// ⛔⛔ THE OLD NUMBER WAS TUNED AGAINST A SIZE THE CHARACTER NO LONGER HAS.
/// `perfect_cellular_automaton` was `body_kind: Floating` with no authored
/// height, so its body came out of `ldtk_box × collision_scale / FRAME_H` —
/// the spawn box below and the sheet's PUBLISHING PADDING. Authoring its
/// `standing_height` (D129, 2026-08-28) took the frame out of the derivation
/// and the tuned spacing stopped working, which is the correct consequence.
///
/// ⇒ 200px is not tuned: it is OUTSIDE the automaton's authored
/// `attack_range: 150` and well inside its `aggro: 540`, so it closes the gap
/// and throws an aimed attack the way it would in a room. The connect is then
/// the brain's decision rather than an accident of where the test put a body.
#[derive(Debug, Default)]
struct Bout {
    /// The hardest hitlag the PLAYER served — proof the player was hit at all.
    player_hitstop: f32,
    /// The hardest any OTHER body served.
    other_hitstop: f32,
    /// Frames on which the match was frozen.
    frozen_frames: u32,
}

#[test]
fn a_hit_that_lands_on_the_player_freezes_the_match() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");

    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_character_at(
        ENEMY_ID,
        "Perfect Cellular Automaton",
        (p.x + 200.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
        "perfect_cellular_automaton",
    );

    let mut bout = Bout::default();
    // Stand still and be hit. The player never presses anything, so it is the
    // only victim in the room and the freeze cannot be somebody else's.
    for _ in 0..900 {
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
    // ⭐⭐ THE WHOLE CLAIM IN ONE LINE, and it is stronger than it looks: a
    // freeze can only have come from a CONNECT, because contact attrition — the
    // other thing hitting this player — arms nothing.
    assert!(
        bout.frozen_frames > 0,
        "the player was hit hard enough to serve {:.3}s of its own hitlag and \
         the MATCH never froze. The freeze used to be decided in Settle from the \
         victim's own timer, and a PLAYER victim's timer is not written until \
         the next frame — the player-victim hits are staged into a rollback FIFO \
         the resolver drains in PlayerSimulation. So the connect that stops the \
         world for a CPU never stopped it for the human: {bout:#?}",
        bout.player_hitstop
    );
}
