//! Room techniques use each participant's driven body through the real gate.

#![cfg(feature = "rl_sim")]

use ambition_app::{AmbitionSim, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::characters::action_scheme::{DrivenTechniques, ResolvedTechniqueEdges};
use ambition_platformer2d::characters::control::{ActorControl, DrivingParticipant, PlayerSlot};
use ambition_platformer2d::combat::scoped_rules::{DeclaredRules, RulesScope};
use ambition_platformer2d::engine_core::ControlFrame;
use ambition_platformer2d::entity_catalog::action_scheme::{
    ActionGate, ActionId, ActionSpec, ControlSlot,
};
use ambition_platformer2d::platformer::markers::{ControlledSubject, PrimaryPlayerOnly};
use bevy::prelude::Entity;

#[test]
fn two_participants_route_a_room_granted_attack_through_the_same_gate() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_fixed_tick(true)
            .with_required_start_room("switch_lab"),
    )
    .expect("the two-seat simulation builds");
    let first = {
        let world = sim.world_mut();
        world
            .query_filtered::<Entity, PrimaryPlayerOnly>()
            .single(world)
            .unwrap()
    };
    let second =
        crate::a_home_body_is_built_for_a_seat::build_a_home_body(&ambition_platformer2d::sprite_sheet::test_support::shared_sprites(), &mut sim, PlayerSlot(1), ());
    sim.world_mut()
        .get_resource_or_insert_with(DeclaredRules::<DrivenTechniques>::default)
        .restate(
            RulesScope::UntaggedRooms,
            Some(DrivenTechniques(vec![ActionSpec {
                id: ActionId::new("spin_dash"),
                slot: ControlSlot::Attack,
                display_name: None,
                visual: None,
                gate: ActionGate::Technique("spin_dash".into()),
            }])),
        );
    let press = ControlFrame {
        attack_pressed: true,
        attack_held: true,
        ..Default::default()
    };
    sim.drive_seat(1, press);
    sim.step_frame(press);
    assert_eq!(sim.world().resource::<ControlledSubject>().0, Some(first));
    for (body, seat) in [(first, PlayerSlot::PRIMARY), (second, PlayerSlot(1))] {
        assert_eq!(sim.world().get::<DrivingParticipant>(body).unwrap().0, seat);
        assert!(
            sim.world()
                .get::<ResolvedTechniqueEdges>(body)
                .unwrap()
                .pressed("spin_dash"),
            "seat {seat:?} did not route its room's attack technique"
        );
        assert!(
            !sim.world()
                .get::<ActorControl>(body)
                .unwrap()
                .0
                .melee_pressed,
            "the technique also leaked into the raw melee path"
        );
    }
}
