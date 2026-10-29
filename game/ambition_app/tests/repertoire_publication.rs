//! Equipment changes in the input phase reach body execution in the same tick.

#![cfg(feature = "rl_sim")]

use ambition_app::{Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::characters::brain::action_set::{
    ActionSet, IdentityKit, RangedActionSpec,
};
use ambition_platformer2d::characters::equipment::{EquipmentGrant, EquipmentRow, WornEquipment};
use ambition_platformer2d::combat::moveset::{ActorMoveset, RANGED_VERB};
use ambition_platformer2d::engine_core::ControlFrame;
use ambition_platformer2d::platformer::schedule::PlayerInputSet;
use ambition_platformer2d::sim::{Platformer2dSimulationPhase, SimScheduleExt};
use bevy::prelude::*;

#[derive(Resource)]
struct Subject(Entity);

#[derive(Resource, Default)]
struct GrantWanted(bool);

#[derive(Resource, Default)]
struct Seen(Vec<(bool, bool)>);

fn change_equipment(
    subject: Res<Subject>,
    wanted: Res<GrantWanted>,
    mut bodies: Query<&mut WornEquipment>,
) {
    let mut worn = bodies.get_mut(subject.0).unwrap();
    if wanted.0 {
        worn.equip(EquipmentRow {
            id: "ranged_grant".into(),
            grants: vec![EquipmentGrant::Ranged(RangedActionSpec::bolt(400.0, 5))],
            ..Default::default()
        });
    } else {
        worn.unequip("ranged_grant");
    }
}

fn read_execution_repertoire(
    subject: Res<Subject>,
    bodies: Query<(&ActionSet, &ActorMoveset)>,
    mut seen: ResMut<Seen>,
) {
    let (actions, moves) = bodies.get(subject.0).unwrap();
    seen.0.push((
        actions.ranged.is_some(),
        moves.0.move_for_verb(RANGED_VERB).is_some(),
    ));
}

#[test]
fn a_grant_and_its_retraction_reach_execution_in_the_same_tick() {
    let mut sim = Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_fixed_tick(true),
        |app, options| {
            ambition_app::rl_sim::ambition_sim_composition(app, options)?;
            let subject = app
                .world_mut()
                .spawn((
                    IdentityKit::default(),
                    ActionSet::peaceful(),
                    ActorMoveset::default(),
                    WornEquipment::default(),
                ))
                .id();
            app.insert_resource(Subject(subject));
            app.init_resource::<GrantWanted>();
            app.init_resource::<Seen>();
            let schedule = app.sim_schedule();
            app.add_systems(
                schedule,
                (
                    change_equipment.in_set(PlayerInputSet::Persona),
                    read_execution_repertoire.in_set(Platformer2dSimulationPhase::PlayerSimulation),
                ),
            );
            Ok(())
        },
    )
    .expect("the composed simulation builds");

    for granted in [true, false, true] {
        sim.world_mut().resource_mut::<GrantWanted>().0 = granted;
        let before = sim.world().resource::<Seen>().0.len();
        sim.step_frame(ControlFrame::default());
        let observed = &sim.world().resource::<Seen>().0[before..];
        assert!(
            !observed.is_empty(),
            "execution must observe at least one simulation tick"
        );
        assert!(
            observed.iter().all(|row| *row == (granted, granted)),
            "the live action and move must both follow the equipment change in this tick: {observed:?}"
        );
    }
}
