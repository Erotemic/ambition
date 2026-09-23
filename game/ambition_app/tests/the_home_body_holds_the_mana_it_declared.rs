//! The Ambition home body is BUILT holding the Mana pool its experience
//! declared, a reset returns it to that same declared start, and no other body
//! holds Mana merely by being a body.
//!
//! ⛔ `BodyMana` sat on every body — the movement bundle built it full — so a
//! staged enemy could spend the player's economy the moment it was possessed.
//! The pool is a declaration of the Ambition provider now
//! (`HomeBodyResources`), and this asks the shipped composition, not a fixture
//! that installs the declaration itself.

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::abilities::mana;
use ambition_platformer2d::engine_core::resources::ActorResources;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::{PrimaryPlayer, PrimaryPlayerOnly};
use bevy::prelude::{With, Without, World};

fn home_bank(world: &mut World) -> ActorResources {
    let mut q = world.query_filtered::<Option<&ActorResources>, PrimaryPlayerOnly>();
    q.single(world)
        .expect("one primary player")
        .expect("the home body holds no bank at all")
        .clone()
}

#[test]
fn the_home_body_holds_mana_resets_to_it_and_nothing_else_does() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");

    // Built holding exactly the declared pool, at its declared (full) start.
    let bank = home_bank(sim.world_mut());
    let held: Vec<&str> = bank
        .layout()
        .declarations()
        .iter()
        .map(|declared| declared.resource.name())
        .collect();
    assert_eq!(held, [mana::MANA.name()], "the home body's layout");
    let level = mana::level(Some(&bank)).expect("held");
    assert_eq!((level.current, level.max), (mana::POOL.capacity, mana::POOL.capacity));

    // Another body — a staged enemy — holds none.
    let p = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
        q.single(world).expect("primary player").pos
    };
    sim.spawn_enemy_character_at(
        "mana_census_enemy",
        "Perfect Cellular Automaton",
        (p.x + 120.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
        "perfect_cellular_automaton",
    );
    sim.step(AgentAction::default());
    let world = sim.world_mut();
    let mut others = world.query_filtered::<Option<&ActorResources>, (
        With<BodyKinematics>,
        Without<PrimaryPlayer>,
    )>();
    let banks: Vec<Option<ActorResources>> = others.iter(world).map(|b| b.cloned()).collect();
    // ⛔ ANTI-VACUITY: a world holding only the player satisfies "nobody else
    // holds Mana" by having nobody else.
    assert!(!banks.is_empty(), "no other body exists to ask");
    let holders = banks
        .iter()
        .filter(|bank| mana::level(bank.as_ref()).is_some())
        .count();
    assert_eq!(
        holders,
        0,
        "{holders} of {} other bodies hold Mana they were never declared",
        banks.len()
    );

    // Spent, then reset: back to the declared start, by the same declaration.
    {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&mut ActorResources, PrimaryPlayerOnly>();
        let mut bank = q.single_mut(world).expect("the home body's bank");
        assert!(mana::spend(Some(&mut bank), 70.0), "the pool pays");
    }
    sim.rebase_rollback_history()
        .expect("a harness mutation is rebased before stepping");
    let spent = mana::level(Some(&home_bank(sim.world_mut()))).expect("held");
    assert!(spent.current < mana::POOL.capacity, "premise: the pool was spent");
    sim.reset_episode();
    let reset = mana::level(Some(&home_bank(sim.world_mut()))).expect("held");
    assert_eq!(reset.current, mana::POOL.capacity, "a reset did not return the declared start");
}
