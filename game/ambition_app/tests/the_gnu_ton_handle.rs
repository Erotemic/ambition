//! The GNU-ton gravity handle: the gravity works of the Science dimension,
//! four rooms of gravity and portal puzzles, the approach, and GNU-ton at the
//! end (`tools/ambition_ldtk_tools/specs/gnu_ton_gravity/`). The spoke joins
//! the works to the orbital station on foot.
//!
//! Each test drives the robot through the way a room means to be solved,
//! with the inputs of a player who runs and jumps (no flight, blink or wall
//! climb), and reads where the body ends. The coordinates are the room's
//! own (`make_specs.py`).

use crate::common::{base, fixed_60hz_room_options, walk_through_the_door_to};
use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};

const ORCHARD: &str = "gnu_ton_orchard";
const WELL: &str = "gnu_ton_vector_well";
const HALL: &str = "gnu_ton_conservation_hall";
const GALLERY: &str = "gnu_ton_inversion_gallery";
const APPROACH: &str = "gnu_ton_works_approach";
const ARENA: &str = "gnu_ton_shoulders";
const SPOKE: &str = "orbital_station_spoke";
const PLACEMENT: &str = "gnu_ton_handle.gnu_ton";

fn room(id: &str) -> Platformer2dSimHarness {
    room_with(id, Default::default())
}

fn room_with(id: &str, save: ambition_platformer2d::session::AmbitionGameSaveData) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(id).with_save(save))
        .unwrap_or_else(|e| panic!("`{id}` builds: {e}"));
    for _ in 0..30 {
        sim.step(base());
    }
    sim
}

/// Put the robot on the floor whose top is `floor`, at `x`, and let it settle.
fn stand(sim: &mut Platformer2dSimHarness, x: f32, floor: f32) {
    let half = sim.observation().player_size.1 * 0.5;
    sim.teleport_player((x, floor - half));
    for _ in 0..20 {
        sim.step(base());
    }
}

fn axis(x: f32, y: f32) -> AgentAction {
    AgentAction { move_x: x, move_y: y, ..base() }
}

fn gravity(sim: &mut Platformer2dSimHarness) -> (f32, f32) {
    let g = sim.observation().gravity_dir;
    (g.0.round(), g.1.round())
}

fn at(sim: &mut Platformer2dSimHarness) -> (f32, f32) {
    sim.observation().player_pos
}

/// The live box of the gravity zone `id` (its centre x and half width).
fn zone_x(sim: &mut Platformer2dSimHarness, name: &str) -> (f32, f32) {
    use ambition_platformer2d::engine_core::AabbExt as _;
    use ambition_platformer2d::platformer::gravity::GravityZone;
    let world = sim.world_mut();
    let mut zones = world.query::<(&bevy::prelude::Name, &GravityZone)>();
    zones
        .iter(world)
        .find(|(n, _)| n.as_str().ends_with(name))
        .map(|(_, zone)| (zone.aabb.center().x, zone.aabb.half_size().x))
        .unwrap_or_else(|| panic!("the zone `{name}` is in the room"))
}

/// Step with `action` until `done` or `ticks` pass; returns the ticks taken.
fn until(
    sim: &mut Platformer2dSimHarness,
    ticks: usize,
    mut action: impl FnMut(&mut Platformer2dSimHarness) -> AgentAction,
    mut done: impl FnMut(&mut Platformer2dSimHarness) -> bool,
) -> Option<usize> {
    for tick in 0..ticks {
        if done(sim) {
            return Some(tick);
        }
        let a = action(sim);
        sim.step(a);
    }
    done(sim).then_some(ticks)
}

fn trace(sim: &mut Platformer2dSimHarness, what: &str) -> String {
    let o = sim.observation();
    format!("{what}: at {:?} vel {:?} gravity {:?} ground {} resets {}", o.player_pos, o.player_vel, o.gravity_dir, o.on_ground, o.resets)
}

/// A press of Interact on the lever `id` (the robot stands on it first).
fn press(sim: &mut Platformer2dSimHarness, id: &str) {
    let lever = {
        let world = sim.world_mut();
        world
            .query::<(
                &ambition_platformer2d::encounter::switches::SwitchFeature,
                &ambition_platformer2d::engine_core::geometry::CenteredAabb,
            )>()
            .iter(world)
            .find(|(feature, _)| feature.activation.id == id)
            .map(|(_, aabb)| (aabb.center.x, aabb.center.y))
            .unwrap_or_else(|| panic!("the room has no lever `{id}`"))
    };
    let (x, _) = at(sim);
    assert!((x - lever.0).abs() < 24.0, "the robot stands at the lever `{id}` ({x} against {lever:?})");
    sim.step(AgentAction { interact: true, ..base() });
    for _ in 0..5 {
        sim.step(base());
    }
}

/// The gate solids that stand in the live room, by name.
fn standing(sim: &mut Platformer2dSimHarness) -> Vec<String> {
    ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<ambition_platformer2d::world::FeatureEcsWorldOverlay>(
        sim.world_mut(),
    )
    .expect("the live room has a collision overlay")
    .gate_solids
    .iter()
    .map(|solid| solid.name.clone())
    .collect()
}

/// Hub, orchard, well, hall, gallery, approach, arena: each door leads to
/// the next room, and each room's door leads back. The arena's way out goes
/// home.
#[test]
fn the_handle_is_one_road_from_the_hub_to_gnu_ton_and_home() {
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_boss(PLACEMENT, ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared);
    let mut sim = room_with("central_hub_complex", save);
    let road = ["central_hub_complex", ORCHARD, WELL, HALL, GALLERY, APPROACH, ARENA];
    for pair in road.windows(2) {
        assert_eq!(walk_through_the_door_to(&mut sim, pair[1]), pair[1], "`{}` has a door to `{}`", pair[0], pair[1]);
        assert_eq!(walk_through_the_door_to(&mut sim, pair[0]), pair[0], "`{}` has a door back to `{}`", pair[1], pair[0]);
        assert_eq!(walk_through_the_door_to(&mut sim, pair[1]), pair[1]);
    }
    assert_eq!(walk_through_the_door_to(&mut sim, "central_hub_complex"), "central_hub_complex", "the arena's way out goes home");
}

/// THE ORCHARD. Hold right from the door: the first tree lifts the robot to
/// the ceiling and puts it down on the floor past the tree; the bridge tree
/// lifts it over the first chasm and puts it down on the middle bank.
#[test]
fn the_orchards_trees_lift_the_robot_and_put_it_down_past_them() {
    let mut sim = room(ORCHARD);
    let mut seen = Vec::new();
    for _ in 0..420 {
        sim.step(axis(1.0, 0.0));
        let o = sim.observation();
        let (x, y) = o.player_pos;
        let place = if y < 120.0 && (352.0..592.0).contains(&x) {
            "first tree's ceiling"
        } else if o.on_ground && o.gravity_dir.1 > 0.0 && (592.0..896.0).contains(&x) {
            "floor past the first tree"
        } else if y < 120.0 && (960.0..1504.0).contains(&x) {
            "ceiling over the chasm"
        } else if o.on_ground && o.gravity_dir.1 > 0.0 && (1504.0..1888.0).contains(&x) && (y - 680.0).abs() < 1.0 {
            "middle bank"
        } else {
            continue;
        };
        if seen.last() != Some(&place) {
            seen.push(place);
        }
    }
    assert_eq!(
        seen,
        ["first tree's ceiling", "floor past the first tree", "ceiling over the chasm", "middle bank"],
        "{}",
        trace(&mut sim, "end")
    );
    assert_eq!(sim.observation().resets, 0);
}

/// THE ORCHARD's twist. The swinging tree takes a robot that waits on the
/// middle bank, and carries it over the second chasm while it walks with
/// the tree on the ceiling; it walks out over the east bank.
#[test]
fn the_swinging_tree_carries_a_robot_that_walks_with_it() {
    let mut sim = room(ORCHARD);
    stand(&mut sim, 1860.0, 704.0);
    let mut riding = false;
    let done = until(
        &mut sim,
        1500,
        |sim| {
            let (tree, _) = zone_x(sim, "orchard swinging tree");
            let (x, _) = at(sim);
            riding |= gravity(sim) == (0.0, -1.0);
            if !riding {
                base()
            } else if tree > 2380.0 {
                axis(1.0, 0.0)
            } else {
                axis(((tree - x) / 40.0).clamp(-1.0, 1.0), 0.0)
            }
        },
        |sim| {
            let o = sim.observation();
            o.on_ground && o.player_pos.0 > 2464.0 && (o.player_pos.1 - 680.0).abs() < 1.0
        },
    );
    assert!(done.is_some() && riding, "{}", trace(&mut sim, "the robot is not on the east bank"));
    assert_eq!(sim.observation().resets, 0, "it never fell in the chasm");
}

/// THE VECTOR WELL. Right onto the east wall, up it; left across the well,
/// up the west wall; up to the vault; left into the passage of the top door.
#[test]
fn the_vector_well_falls_four_ways_to_its_top_door() {
    let mut sim = room(WELL);
    let mut ways = vec![gravity(&mut sim)];
    let done = until(
        &mut sim,
        900,
        |sim| {
            let g = gravity(sim);
            if ways.last() != Some(&g) {
                ways.push(g);
            }
            match g {
                (0.0, 1.0) if at(sim).1 > 1000.0 => axis(1.0, 0.0),
                (1.0, 0.0) | (-1.0, 0.0) => axis(0.0, -1.0),
                _ => axis(-1.0, 0.0),
            }
        },
        |sim| {
            let o = sim.observation();
            o.on_ground && o.player_pos.0 < 300.0 && (o.player_pos.1 - 120.0).abs() < 1.0
        },
    );
    assert!(done.is_some(), "{}", trace(&mut sim, "the robot is not in the top passage"));
    assert_eq!(ways, [(0.0, 1.0), (1.0, 0.0), (-1.0, 0.0), (0.0, -1.0), (0.0, 1.0)], "down, right, left, up, down");
    assert_eq!(sim.observation().resets, 0);
}

/// THE CONSERVATION HALL. Hold right from the door: the floor's portal puts
/// the robot on the shelf; off the shelf's edge, the basin's portal flings
/// it out of the pillar and over the pit.
#[test]
fn the_halls_portals_lift_the_robot_and_fling_it_over_the_pit() {
    let mut sim = room(HALL);
    let on_shelf = until(&mut sim, 200, |_| axis(1.0, 0.0), |sim| {
        let o = sim.observation();
        o.on_ground && (o.player_pos.1 - 424.0).abs() < 1.0 && o.player_pos.0 > 608.0
    });
    assert!(on_shelf.is_some(), "{}", trace(&mut sim, "the floor's portal did not put the robot on the shelf"));
    let over = until(&mut sim, 300, |_| axis(1.0, 0.0), |sim| {
        let o = sim.observation();
        o.on_ground && (o.player_pos.1 - 872.0).abs() < 1.0 && o.player_pos.0 > 1568.0
    });
    assert!(over.is_some(), "{}", trace(&mut sim, "the robot is not over the pit"));
    assert_eq!(sim.observation().resets, 0, "it never fell in the pit");
}

/// THE CONSERVATION HALL's twist. A robot that walks into the floor's last
/// portal comes up out of the gallery's floor, falls up onto its ceiling,
/// and walks out of the field down to the door.
#[test]
fn the_gallery_is_entered_by_falling_up_out_of_its_floor() {
    let mut sim = room(HALL);
    stand(&mut sim, 2400.0, 896.0);
    let mut fell_up = false;
    let done = until(
        &mut sim,
        600,
        |sim| {
            fell_up |= gravity(sim) == (0.0, -1.0);
            axis(if fell_up { 1.0 } else { -1.0 }, 0.0)
        },
        |sim| {
            let o = sim.observation();
            o.on_ground && o.gravity_dir.1 > 0.0 && (o.player_pos.1 - 408.0).abs() < 1.0 && o.player_pos.0 > 2304.0
        },
    );
    assert!(done.is_some() && fell_up, "{}", trace(&mut sim, "the robot is not at the gallery's door"));
    assert_eq!(sim.observation().resets, 0);
}

/// THE INVERSION GALLERY's lesson. The lever by the door turns the room
/// over; the lever on the ceiling over it turns it back; and the first works
/// again after that.
#[test]
fn the_gallerys_levers_turn_it_over_and_back() {
    let mut sim = room(GALLERY);
    stand(&mut sim, 332.0, 896.0);
    for (lever, way) in [("gallery_up_lever", -1.0), ("gallery_down_lever", 1.0), ("gallery_up_lever", -1.0)] {
        let at_lever = if lever == "gallery_up_lever" { 332.0 } else { 412.0 };
        let there = until(&mut sim, 120, |sim| axis((at_lever - at(sim).0).clamp(-1.0, 1.0), 0.0), |sim| {
            sim.observation().on_ground && (at(sim).0 - at_lever).abs() < 4.0
        });
        assert!(there.is_some(), "{}", trace(&mut sim, &format!("the robot did not get to `{lever}`")));
        press(&mut sim, lever);
        let turned = until(&mut sim, 120, |_| base(), |sim| gravity(sim).1 == way && sim.observation().on_ground);
        assert!(turned.is_some(), "{}", trace(&mut sim, &format!("`{lever}` did not turn gravity to {way}")));
    }
    assert_eq!(sim.observation().resets, 0);
}

/// THE INVERSION GALLERY. The lever turns the room over; the robot walks
/// the ceiling over the chasm, steps off the ceiling's step into the first
/// portal, comes out of the hanging wall past the ceiling's hole, and the
/// column of down field puts it on the ledge of GNU-ton's door. It only
/// holds right.
#[test]
fn the_inversion_gallery_is_crossed_on_its_ceiling() {
    let mut sim = room(GALLERY);
    stand(&mut sim, 332.0, 896.0);
    press(&mut sim, "gallery_up_lever");
    let up = until(&mut sim, 120, |_| base(), |sim| gravity(sim) == (0.0, -1.0) && sim.observation().on_ground);
    assert!(up.is_some(), "{}", trace(&mut sim, "the lever did not turn the room over"));
    let mut past_the_hole = false;
    let done = until(
        &mut sim,
        900,
        |sim| {
            let o = sim.observation();
            // Out of the hanging wall's portal, past the hole, still upside down.
            past_the_hole |= o.gravity_dir.1 < 0.0 && o.player_pos.0 > 2112.0;
            axis(1.0, 0.0)
        },
        |sim| {
            let o = sim.observation();
            o.on_ground && o.gravity_dir.1 > 0.0 && (o.player_pos.1 - 872.0).abs() < 1.0 && o.player_pos.0 > 2304.0
        },
    );
    assert!(done.is_some() && past_the_hole, "{}", trace(&mut sim, "the robot is not at GNU-ton's door"));
    assert_eq!(sim.observation().resets, 0, "it never fell in the chasm or the hole");
}

/// THE SHOULDERS. GNU-ton rides his giant here under his own placement, and
/// the grille of the way home stands until his defeat is recorded.
#[test]
fn gnu_ton_waits_at_the_end_and_his_defeat_opens_the_way_home() {
    use ambition_platformer2d::boss_encounter::BossConfig;
    use ambition_platformer2d::characters::actor::BodyHealth;
    let gnu_tons = |sim: &mut Platformer2dSimHarness| -> Vec<(String, bool)> {
        let world = sim.world_mut();
        world
            .query::<(&BossConfig, &BodyHealth)>()
            .iter(world)
            .filter(|(config, _)| config.behavior.id == ambition_content::bosses::gnu_ton::GNU_TON_ID)
            .map(|(config, health)| (config.id.clone(), health.alive()))
            .collect()
    };
    let mut sim = room(ARENA);
    assert_eq!(gnu_tons(&mut sim), [(PLACEMENT.to_string(), true)], "GNU-ton, alive, at the handle's placement");
    assert!(standing(&mut sim).contains(&"gated_lock:shoulders_exit_grille".to_string()), "{:?}", standing(&mut sim));

    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_boss(PLACEMENT, ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared);
    let mut won = room_with(ARENA, save);
    assert!(!gnu_tons(&mut won).iter().any(|(_, alive)| *alive), "a cleared placement is not built alive");
    assert!(!standing(&mut won).iter().any(|name| name.contains("shoulders_exit_grille")), "{:?}", standing(&mut won));
}


/// THE SHOULDERS. The fight is the hall's: his intro ends, and his first
/// punch opens his pair of portals in this room.
#[test]
fn his_fight_here_opens_his_portals() {
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::portal::extension::HeldPortalOf;
    use ambition_platformer2d::portal::PlacedPortal;
    let mut sim = room(ARENA);
    {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&mut BodyHealth, ambition_platformer2d::platformer::markers::PrimaryPlayerOnly>();
        for mut health in q.iter_mut(world) {
            health.health.invulnerable.set(ambition_platformer2d::characters::actor::Invulnerability::SCRIPTED, true);
        }
    }
    stand(&mut sim, 400.0, 1248.0);
    let held = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world.query_filtered::<&PlacedPortal, bevy::prelude::With<HeldPortalOf>>().iter(world).count()
    };
    let opened = until(&mut sim, 900, |_| base(), |sim| held(sim) == 2);
    assert!(opened.is_some(), "his punch opens a pair of two lines here ({} held)", held(&mut sim));
}


/// THE APPROACH. From the gallery's door, a robot that holds right walks
/// into the column against the tower, falls up along its face, and out of
/// the column over its top drops onto the tower, by GNU-ton's door.
#[test]
fn the_approach_lifts_the_robot_to_gnu_tons_door() {
    let mut sim = room(APPROACH);
    stand(&mut sim, 420.0, 1344.0);
    let landed = until(
        &mut sim,
        900,
        |_| axis(1.0, 0.0),
        |sim| {
            let o = sim.observation();
            o.on_ground && o.gravity_dir.1 > 0.0 && o.player_pos.0 > 1000.0 && (o.player_pos.1 - 328.0).abs() < 1.0
        },
    );
    assert!(landed.is_some(), "{}", trace(&mut sim, "end"));
    assert_eq!(sim.observation().resets, 0);
    let door = crate::common::door_to(&mut sim, ARENA).aabb;
    use ambition_platformer2d::engine_core::AabbExt as _;
    assert!((door.center().y - 304.0).abs() < 1.0, "GNU-ton's door stands on the landing: {door:?}");
}

/// THE SHORTCUT. The approach's lift goes down to the end of the spoke, by
/// the orchard. Its gate in the spoke stands until it is let go from the
/// lift's side; then the spoke, the orchard and the approach are one loop.
#[test]
fn the_lift_from_the_approach_is_let_go_from_inside() {
    let mut sim = room(APPROACH);
    assert_eq!(walk_through_the_door_to(&mut sim, SPOKE), SPOKE, "the approach's lift goes down to the spoke");
    assert!(standing(&mut sim).iter().any(|name| name.contains("spoke_lift_gate")), "{:?}", standing(&mut sim));
    // Shut in: walking west from the lift stops at the gate.
    for _ in 0..90 {
        sim.step(axis(-1.0, 0.0));
    }
    assert!(at(&mut sim).0 > 2880.0, "the gate holds the robot in the alcove: {}", trace(&mut sim, "west"));
    stand(&mut sim, 2932.0, 576.0);
    press(&mut sim, "spoke_lift_release");
    assert!(!standing(&mut sim).iter().any(|name| name.contains("spoke_lift_gate")), "{:?}", standing(&mut sim));
    let walked = crate::common::walk::walk_through(&mut crate::common::walk::Pad::new(&mut sim), ORCHARD);
    assert!(walked.is_ok(), "out of the alcove to the orchard: {walked:?}");
    assert_eq!(sim.observation().active_room, ORCHARD);
    // The release is a fact of the save: the gate stays open.
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("switch_spoke_lift_release_used".to_string(), true);
    let mut later = room_with(SPOKE, save);
    assert!(!standing(&mut later).iter().any(|name| name.contains("spoke_lift_gate")), "{:?}", standing(&mut later));
}

/// THE WORKS AND THE STATION ARE ONE PLACE. The orchard's spoke door leads
/// to the spoke, and the spoke's far door to the station's service deck.
#[test]
fn the_spoke_joins_the_works_to_the_station() {
    let mut sim = room(ORCHARD);
    for (from, to) in [(ORCHARD, SPOKE), (SPOKE, "orbital_station_service_deck")] {
        assert_eq!(walk_through_the_door_to(&mut sim, to), to, "`{from}` has a door to `{to}`");
    }
    for (from, to) in [("orbital_station_service_deck", SPOKE), (SPOKE, ORCHARD)] {
        assert_eq!(walk_through_the_door_to(&mut sim, to), to, "`{from}` has a door back to `{to}`");
    }
}

