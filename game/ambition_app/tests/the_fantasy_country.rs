//! Travel through the Fantasy Dimension with player inputs.

use ambition_app::AgentAction;
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::CharacterForm;
use ambition_platformer2d::engine_core::{BodyKinematics, Vec2};
use ambition_platformer2d::items::{ItemCatalog, OwnedItems};
use ambition_platformer2d::persistence::save::AmbitionGameSave;
use ambition_platformer2d::persistence::save_data::PersistedEncounterState;
use ambition_platformer2d::platformer::lifecycle::sole_live_room_component;
use ambition_platformer2d::world::FeatureEcsWorldOverlay;
use bevy::prelude::World;

use crate::common::walk::{Pad, Press, Walker as _, fight, the_player, walk_through, walk_to};
use crate::common::{base, fixed_60hz_room_sim};

#[test]
fn the_gate_stack_service_ladder_reaches_pirate_sky_and_back() {
    const STACK: &str = "gate_stack_lower";
    const ARENA: &str = "pirate_sky_arena";
    const SHAFT_CENTER: f32 = 208.0;
    let mut sim = fixed_60hz_room_sim(STACK);
    sim.step_n(base(), 30);
    let climbed = (0..420).any(|_| {
        let move_x = ((SHAFT_CENTER - sim.observation().player_pos.0) / 24.0).clamp(-1.0, 1.0);
        sim.step(AgentAction { move_x, move_y: -1.0, ..base() }).active_room == ARENA
    });
    assert!(climbed, "climb from the stack floor to the arena: {:?}", sim.observation().player_pos);
    let landed = (0..90).any(|_| sim.step(base()).on_ground);
    assert!(landed, "land on the arena pad: {:?}", sim.observation().player_pos);
    assert_eq!(sim.observation().active_room, ARENA);

    let returned = (0..180).any(|tick| {
        let jump = sim.observation().on_ground && tick % 12 == 0;
        sim.step(AgentAction { move_y: 1.0, jump, ..base() }).active_room == STACK
    });
    assert!(returned, "drop through the arena pad: {:?}", sim.observation().player_pos);

    let reached_floor = (0..420).any(|tick| {
        let observation = sim.observation();
        if observation.on_ground && observation.player_pos.1 > 660.0 {
            return true;
        }
        let jump = observation.on_ground && tick % 12 == 0;
        sim.step(AgentAction { move_y: 1.0, jump, ..base() });
        false
    });
    assert!(reached_floor, "return to the stack floor: {:?}", sim.observation().player_pos);
    assert_eq!(sim.observation().active_room, STACK);
}

#[test]
fn both_pirate_sky_lookout_platforms_can_be_reached_from_the_floor() {
    let mut sim = fixed_60hz_room_sim("pirate_sky_arena");
    sim.step_n(base(), 30);
    let half_height = sim.observation().player_size.1 * 0.5;
    for x in [576.0, 1824.0] {
        sim.teleport_player((x, 992.0 - half_height));
        sim.step_n(base(), 10);
        let mut highest = sim.observation().player_pos.1;
        let reached = (0..420).any(|tick| {
            let observation = sim.observation();
            highest = highest.min(observation.player_pos.1);
            if observation.on_ground && observation.player_pos.1 + half_height <= 641.0 {
                return true;
            }
            let jump = observation.on_ground && tick % 12 == 0;
            sim.step(AgentAction { jump, jump_held: true, ..base() });
            false
        });
        assert!(reached, "reach the lookout above x={x}: highest y={highest}, last position {:?}", sim.observation().player_pos);
    }
}

fn gate_stands(world: &mut World, gate: &str) -> bool {
    sole_live_room_component::<FeatureEcsWorldOverlay>(world)
        .expect("the live room has a collision overlay")
        .gate_solids
        .iter()
        .any(|solid| solid.name == format!("gated_lock:{gate}"))
}

#[test]
fn the_high_road_goes_to_brackenford_and_back_after_the_stair_opens() {
    let mut sim = fixed_60hz_room_sim("intro_hobford_lane");
    sim.step_n(base(), 30);
    assert!(gate_stands(sim.world_mut(), "fantasy_high_trail_gate"));

    sim.world_mut()
        .resource_mut::<AmbitionGameSave>()
        .data_mut()
        .set_flag("fantasy_high_trail_open", true);
    sim.step_n(base(), 60);
    assert!(!gate_stands(sim.world_mut(), "fantasy_high_trail_gate"));

    let mut pad = Pad::new(&mut sim);
    for room in [
        "fantasy_mountain_ascent",
        "fantasy_mountain_pass",
        "fantasy_mountain_shelter",
        "fantasy_mountain_pass",
        "fantasy_mountain_descent",
        "fantasy_brackenford",
        "fantasy_old_watch",
        "fantasy_brackenford",
        "fantasy_mountain_descent",
        "fantasy_mountain_pass",
        "fantasy_mountain_ascent",
        "intro_hobford_lane",
    ] {
        walk_through(&mut pad, room).unwrap_or_else(|why| panic!("walk to {room}: {why}"));
    }
}

#[test]
fn the_small_mounds_link_the_moor_and_the_burial_vaults() {
    let mut sim = fixed_60hz_room_sim("intro_moor");
    sim.step_n(base(), 30);
    let mut pad = Pad::new(&mut sim);
    for room in [
        "intro_barrow_cairnfield",
        "intro_barrow_votive",
        "intro_barrow_cairnfield",
        "intro_barrow_watch_crypt",
        "intro_barrow_tally_vault",
        "intro_barrow",
        "intro_moor",
    ] {
        walk_through(&mut pad, room).unwrap_or_else(|why| panic!("walk to {room}: {why}"));
    }
}

fn has_key(world: &World, id: &str) -> bool {
    let key = world.resource::<ItemCatalog>().item_by_dialog_id(id).expect("authored key");
    world.resource::<OwnedItems>().count(key) > 0
}

fn use_at(pad: &mut Pad<'_>, x: f32, floor: f32) {
    walk_to(pad, Vec2::new(x, floor)).unwrap_or_else(|why| panic!("reach ({x}, {floor}): {why}"));
    pad.step(Press::default());
    pad.step(Press { interact: true, ..Press::default() });
    for _ in 0..20 {
        pad.step(Press::default());
    }
}

#[test]
fn the_barrow_keys_and_wheels_lead_to_the_king_and_back() {
    const VAULT: &str = "intro_barrow_tally_vault";
    const WORKS: &str = "intro_barrow_wheelworks";
    const RELIQUARY: &str = "intro_barrow_reliquary";
    const WATCH: &str = "intro_barrow_reliquary_watch";
    const TOMB: &str = "intro_barrow_royal_tomb";

    let mut sim = fixed_60hz_room_sim("intro_barrow");
    sim.step_n(base(), 30);
    let mut pad = Pad::new(&mut sim);

    walk_through(&mut pad, VAULT).expect("descend to the tally vault");
    assert!(gate_stands(pad.world_mut(), "barrow_bronze_seal"));
    assert!(gate_stands(pad.world_mut(), "barrow_crown_seal"));
    walk_through(&mut pad, "intro_barrow").expect("return to the family graves");
    walk_through(&mut pad, "intro_barrow_votive").expect("take the votive passage");
    walk_to(&mut pad, Vec2::new(840.0, 352.0)).expect("reach the warden's shelf");
    assert!(has_key(pad.world(), "barrowwardkey"));
    walk_through(&mut pad, "intro_barrow").expect("return by the votive passage");
    walk_through(&mut pad, VAULT).expect("return to the seals");
    assert!(!gate_stands(pad.world_mut(), "barrow_bronze_seal"));
    assert!(gate_stands(pad.world_mut(), "barrow_crown_seal"));

    walk_through(&mut pad, WORKS).expect("enter the wheelworks");
    assert!(gate_stands(pad.world_mut(), "wheel_sluice_recess"));
    assert!(gate_stands(pad.world_mut(), "wheel_sluice_bolt"));
    use_at(&mut pad, 276.0, 512.0);
    assert!(gate_stands(pad.world_mut(), "wheel_upper_gate"));
    assert!(!gate_stands(pad.world_mut(), "wheel_lower_gate"));
    use_at(&mut pad, 922.0, 512.0);
    assert!(!gate_stands(pad.world_mut(), "wheel_sluice_bolt"));
    assert!(gate_stands(pad.world_mut(), "wheel_weight_bolt"));
    use_at(&mut pad, 276.0, 512.0);
    assert!(!gate_stands(pad.world_mut(), "wheel_weight_bolt"));
    assert!(gate_stands(pad.world_mut(), "wheel_lower_gate"));
    walk_through(&mut pad, RELIQUARY).expect("cross the high passage to the reliquary");

    let guard_approach = Vec2::new(302.0, 464.0);
    assert!(gate_stands(pad.world_mut(), "reliquary_screen"));
    walk_to(&mut pad, guard_approach).expect("approach the wardens");
    fight(
        &mut pad,
        |world, body| world.get::<CharacterForm>(body).is_some_and(|form| {
            matches!(form.id(), "barrow_wight" | "barrow_shambler" | "barrow_crawler")
        }),
        |world| world.resource::<AmbitionGameSave>().data().encounter(WATCH) == PersistedEncounterState::Cleared,
        guard_approach,
        3600,
    ).expect("clear the reliquary with the opening kit");
    for _ in 0..20 {
        pad.step(Press::default());
    }
    assert!(!gate_stands(pad.world_mut(), "reliquary_screen"));
    use_at(&mut pad, 1488.0, 464.0);
    assert!(has_key(pad.world(), "barrowcrownkey"));
    walk_through(&mut pad, WORKS).expect("leave the reliquary");
    walk_through(&mut pad, VAULT).expect("carry both keys back to the seals");
    assert!(!gate_stands(pad.world_mut(), "barrow_crown_seal"));
    walk_through(&mut pad, TOMB).expect("enter the royal tomb");
    defeat_barrow_king(&mut pad);
    walk_through(&mut pad, VAULT).expect("leave the royal tomb");
    walk_through(&mut pad, "intro_barrow_watch_crypt").expect("take the watch mound loop");
    walk_through(&mut pad, "intro_barrow_cairnfield").expect("leave the Barrows");
}

#[test]
fn the_barrow_king_approaches_and_opens_the_treasury_when_defeated() {
    let mut sim = fixed_60hz_room_sim("intro_barrow_royal_tomb");
    sim.step_n(base(), 30);
    let mut pad = Pad::new(&mut sim);
    defeat_barrow_king(&mut pad);
    walk_through(&mut pad, "intro_barrow_tally_vault").expect("leave the royal tomb");
}

fn defeat_barrow_king(pad: &mut Pad<'_>) {
    const GRILLE: &str = "royal_treasury_grille";
    const VIGIL: &str = "intro_barrow_king_vigil";

    assert!(gate_stands(pad.world_mut(), GRILLE));
    let player = the_player(pad.world_mut());
    let world = pad.world_mut();
    let player_height = world.get::<BodyKinematics>(player).unwrap().size().y;
    let (_, king) = world
        .query::<(&BossConfig, &BodyKinematics)>()
        .iter(world)
        .find(|(boss, _)| boss.id == VIGIL)
        .expect("the royal tomb has a boss placement");
    assert!(
        king.size().y >= 170.0 && king.size().y > player_height * 1.7,
        "the king stands at boss size (king {}, player {player_height})",
        king.size().y,
    );
    let spawn_x = king.pos.x;

    let approach = Vec2::new(404.0, 432.0);
    walk_to(pad, approach).expect("wake the king");
    let approached = (0..210).any(|_| {
        pad.step(Press::default());
        let world = pad.world_mut();
        world
            .query::<(&BossConfig, &BodyKinematics)>()
            .iter(world)
            .any(|(boss, king)| boss.id == VIGIL && king.pos.x < spawn_x - 32.0)
    });
    assert!(approached, "the awakened king closes the distance");
    let cleared = (0..1600).any(|frame| {
        if pad.world().resource::<AmbitionGameSave>().data().boss(VIGIL) == PersistedEncounterState::Cleared {
            return true;
        }
        let player = the_player(pad.world_mut());
        let world = pad.world_mut();
        let scout = *world.get::<BodyKinematics>(player).expect("Scout has a body");
        let mut bosses = world.query::<(&BossConfig, &BodyKinematics)>();
        let axis = bosses
            .iter(world)
            .find(|(boss, _)| boss.id == VIGIL)
            .map_or(0.0, |(_, king)| {
                let dx = king.pos.x - scout.pos.x;
                let reach = (king.size().x + scout.size().x) * 0.5 + 3.0;
                if dx.abs() > reach || scout.facing * dx < 0.0 { dx.signum() } else { 0.0 }
            });
        pad.step(Press { axis, attack: frame % 18 == 0, ..Press::default() });
        false
    });
    assert!(cleared, "defeat the king with the opening kit");
    for _ in 0..60 {
        pad.step(Default::default());
    }
    assert!(!gate_stands(pad.world_mut(), GRILLE));
    let player = the_player(pad.world_mut());
    let balance = |world: &World| world.get::<ambition_platformer2d::characters::actor::BodyWallet>(player).unwrap().balance;
    let before = balance(pad.world());
    use_at(pad, 1388.0, 432.0);
    assert_eq!(balance(pad.world()) - before, 30);
}
