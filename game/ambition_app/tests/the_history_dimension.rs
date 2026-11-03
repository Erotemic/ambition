//! The History dimension: its gate is DYNASTIC.
//!
//! Fourteen rooms (`tools/ambition_ldtk_tools/specs/history_dimension/`): a
//! timeline of eight eras side by side, earliest in the west, and six
//! buildings by doors in them. This asks the game what the generator takes as
//! given: that the eras join in order and each building opens off its era,
//! that each room places its cast, that the sluice raises the reed bridge,
//! that Socratease's answer opens the stoa gate, that each of the brothers at
//! the Great Wall opens his own road through it and not the other's, that the
//! mill's boiler opens the factory gate, and that the two vigils keep their
//! store rooms shut until they are over.

use std::collections::BTreeSet;

use crate::common::{base, fixed_60hz_room_options};
use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::ldtk_map::LdtkProject;

use crate::the_pirate_port::{a_session_in, talk};

fn room(id: &str) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(id)).unwrap_or_else(|e| panic!("`{id}` builds: {e}"));
    for _ in 0..30 {
        sim.step(base());
    }
    sim
}

/// The gate solids that stand in the live room, by name (a gated wall is
/// `gated_lock:<its id>`).
fn standing_in(world: &mut bevy::prelude::World) -> BTreeSet<String> {
    ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<ambition_platformer2d::world::FeatureEcsWorldOverlay>(world)
        .expect("the live room has a collision overlay")
        .gate_solids
        .iter()
        .map(|solid| solid.name.clone())
        .collect()
}

fn standing(sim: &mut Platformer2dSimHarness) -> BTreeSet<String> {
    standing_in(sim.world_mut())
}

fn stands(sim: &mut Platformer2dSimHarness, wall: &str) -> bool {
    standing(sim).contains(&format!("gated_lock:{wall}"))
}

fn set_flag(sim: &mut Platformer2dSimHarness, id: &str) {
    sim.world_mut().resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>().data_mut().set_flag(id, true);
}

/// How many bodies in the room wear catalog character `id`.
fn wearing(sim: &mut Platformer2dSimHarness, id: &str) -> usize {
    let world = sim.world_mut();
    let mut q = world.query::<&WornCharacter>();
    q.iter(world).filter(|worn| worn.id() == id).count()
}

/// Stand on the lever `id` in the live room and press Interact once.
fn press(sim: &mut Platformer2dSimHarness, id: &str) {
    use ambition_platformer2d::encounter::switches::SwitchFeature;
    let at = {
        let world = sim.world_mut();
        world
            .query::<(&SwitchFeature, &ambition_platformer2d::engine_core::geometry::CenteredAabb)>()
            .iter(world)
            .find(|(feature, _)| feature.activation.id == id)
            .map(|(_, aabb)| (aabb.center.x, aabb.center.y))
            .unwrap_or_else(|| panic!("the room has no lever `{id}`"))
    };
    sim.teleport_player(at);
    sim.step(base());
    sim.step(AgentAction { interact: true, ..base() });
    for _ in 0..5 {
        sim.step(base());
    }
}

fn settle(sim: &mut Platformer2dSimHarness) {
    for _ in 0..10 {
        sim.step(base());
    }
}

/// The road from the earliest era to the latest, and each building off it.
#[test]
fn the_eras_of_the_history_dimension_join_in_order() {
    const WAYS: [(&str, &str); 13] = [
        ("history_fertile_crescent", "history_nile_valley"),
        ("history_nile_valley", "history_classical_shore"),
        ("history_classical_shore", "history_silk_road"),
        ("history_silk_road", "history_mongol_steppe"),
        ("history_mongol_steppe", "history_timbuktu_coast"),
        ("history_timbuktu_coast", "history_age_of_sail"),
        ("history_age_of_sail", "history_industrial_age"),
        ("history_fertile_crescent", "history_temple_granary"),
        ("history_nile_valley", "history_pyramid_tomb"),
        ("history_classical_shore", "history_colosseum"),
        ("history_mongol_steppe", "history_great_ger"),
        ("history_timbuktu_coast", "history_timbuktu_library"),
        ("history_industrial_age", "history_cotton_mill"),
    ];
    let manifest = ambition_content::worlds::world_manifest();
    let project = LdtkProject::load_default_for_dev(&manifest).expect("the world loads");
    let rooms = project.to_room_set(&manifest, &ambition_app::composed_ldtk_vocabulary()).expect("the world composes");
    let index = |id: &str| rooms.room_index_by_id(id).unwrap_or_else(|| panic!("the world has no room `{id}`"));
    for (a, b) in WAYS {
        let (from, to) = (index(a), index(b));
        assert!(rooms.neighboring_room_indices_of(from).contains(&to), "`{a}` has no way to `{b}`");
        assert!(rooms.neighboring_room_indices_of(to).contains(&from), "`{b}` has no way back to `{a}`");
    }
    // The start of the road: nothing is before the crescent but its granary.
    let crescent = index("history_fertile_crescent");
    assert_eq!(rooms.neighboring_room_indices_of(crescent).len(), 2, "the crescent joins only the nile and its granary");
}

/// The cast each room places: the kinds, and how many of each.
const CAST: &[(&str, &[(&str, usize)])] = &[
    (
        "history_fertile_crescent",
        &[
            ("npc_history_clio", 1),
            ("npc_history_hammurabbi", 1),
            ("npc_history_gilgamash", 1),
            ("locust_swarm", 2),
            ("aurochs", 1),
            ("scorpion_man", 1),
        ],
    ),
    ("history_temple_granary", &[("npc_history_scribe_of_debts", 1)]),
    (
        "history_nile_valley",
        &[("npc_history_hatshepsut_hats", 1), ("npc_history_ramesses_pretty_good", 1), ("scarab_beetle", 2), ("asp", 2), ("mummy", 1)],
    ),
    (
        "history_classical_shore",
        &[
            ("npc_history_socratease", 1),
            ("npc_history_herodotoss", 1),
            ("npc_history_julius_sneezer", 1),
            ("hoplite", 2),
            ("immortal", 2),
            ("vandal", 1),
        ],
    ),
    (
        "history_silk_road",
        &[("npc_history_marco_polo_neck", 1), ("npc_history_confusius", 1), ("desert_bandit", 2), ("silk_moth", 2), ("terracotta_warrior", 2)],
    ),
    (
        "history_mongol_steppe",
        &[("npc_genghis_can", 1), ("npc_genghis_cant", 1), ("horse_archer", 2), ("steppe_wolf", 2), ("golden_eagle", 2)],
    ),
    ("history_great_ger", &[("npc_history_kublai_khant_wait", 1)]),
    (
        "history_timbuktu_coast",
        &[("npc_history_mansa_moolah", 1), ("npc_history_zheng_hey", 1), ("inflated_coin", 2), ("coral_crab", 2), ("monsoon_squall", 2)],
    ),
    ("history_timbuktu_library", &[("npc_history_ibn_batootah", 1)]),
    (
        "history_age_of_sail",
        &[
            ("npc_history_vasco_da_gamer", 1),
            ("npc_history_christopher_columbust", 1),
            ("npc_history_captain_cookbook", 1),
            ("privateer", 2),
            ("scurvy_ghost", 2),
            ("kraken_arm", 1),
        ],
    ),
    (
        "history_industrial_age",
        &[
            ("npc_history_adam_smithy", 1),
            ("npc_history_karl_marks", 1),
            ("npc_history_general_ludd", 1),
            ("steam_automaton", 1),
            ("soot_sprite", 2),
            ("robber_baron", 2),
        ],
    ),
];

#[test]
fn each_room_of_the_history_dimension_places_its_cast() {
    for &(id, cast) in CAST {
        let mut sim = room(id);
        for (kind, count) in cast {
            assert_eq!(wearing(&mut sim, kind), *count, "`{id}` places {count} `{kind}`");
        }
    }
}

/// The crescent's canal has no bridge until the sluice is opened; then the
/// two reed bundles stand over it.
#[test]
fn the_sluice_raises_the_reed_bridge_over_the_canal() {
    const BUNDLES: [&str; 2] = ["history_reed_bridge_0", "history_reed_bridge_1"];
    let mut sim = room("history_fertile_crescent");
    for bundle in BUNDLES {
        assert!(!stands(&mut sim, bundle), "`{bundle}` is not there before the sluice: {:?}", standing(&mut sim));
    }
    press(&mut sim, "history_sluice");
    settle(&mut sim);
    for bundle in BUNDLES {
        assert!(stands(&mut sim, bundle), "with the sluice open, `{bundle}` spans the canal: {:?}", standing(&mut sim));
    }
}

/// The stoa gate stands until the traveller knows that they know nothing
/// (the flag Socratease's answer sets).
#[test]
fn knowing_nothing_opens_the_stoa_gate() {
    let mut sim = room("history_classical_shore");
    assert!(stands(&mut sim, "history_stoa_gate"), "the stoa gate stands at first: {:?}", standing(&mut sim));
    set_flag(&mut sim, "history_knows_nothing");
    settle(&mut sim);
    assert!(!stands(&mut sim, "history_stoa_gate"), "knowing nothing, the stoa gate is open: {:?}", standing(&mut sim));
}

/// The two roads through the Great Wall. At first the gate stands and there
/// is no scaffold. Genghis Can's yes opens the gate and puts up no scaffold;
/// Genghis Can't's maybe puts up the scaffold.
#[test]
fn each_brother_at_the_great_wall_opens_his_own_road() {
    const SCAFFOLD: [&str; 3] = ["history_wall_scaffold_0", "history_wall_scaffold_1", "history_wall_scaffold_2"];
    let gate = "gated_lock:history_wall_gate";
    let scaffold = |standing: &BTreeSet<String>| SCAFFOLD.iter().filter(|plank| standing.contains(&format!("gated_lock:{plank}"))).count();

    let mut app = a_session_in("history_mongol_steppe");
    let first = standing_in(app.world_mut());
    assert!(first.contains(gate), "the Wall's gate stands at first: {first:?}");
    assert_eq!(scaffold(&first), 0, "no scaffold at first: {first:?}");

    assert_eq!(talk(&mut app, "npc_genghis_can", "Open the gate."), "Open the gate.");
    for _ in 0..10 {
        app.update();
    }
    let after_can = standing_in(app.world_mut());
    assert!(!after_can.contains(gate), "after Genghis Can's yes the gate is open: {after_can:?}");
    assert_eq!(scaffold(&after_can), 0, "his yes puts up no scaffold: {after_can:?}");

    assert_eq!(talk(&mut app, "npc_genghis_cant", "Is there another way?"), "Is there another way?");
    for _ in 0..10 {
        app.update();
    }
    let after_cant = standing_in(app.world_mut());
    assert_eq!(scaffold(&after_cant), SCAFFOLD.len(), "after Genghis Can't's maybe the scaffold stands: {after_cant:?}");
}

/// The factory gate stands until the boiler in the cotton mill is stoked.
#[test]
fn the_mills_boiler_opens_the_factory_gate() {
    let mut street = room("history_industrial_age");
    assert!(stands(&mut street, "history_factory_gate"), "the factory gate stands at first: {:?}", standing(&mut street));

    let mut sim = room("history_cotton_mill");
    press(&mut sim, "history_boiler");
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "history_industrial_age"), "history_industrial_age");
    settle(&mut sim);
    assert!(!stands(&mut sim, "history_factory_gate"), "with the engine running, the factory gate is open: {:?}", standing(&mut sim));
}

/// The two vigils keep their store rooms shut until they are over.
#[test]
fn the_vigils_keep_their_store_rooms_shut() {
    for (id, grille) in [("history_pyramid_tomb", "history_pyramid_tomb_store_grille"), ("history_colosseum", "history_colosseum_store_grille")] {
        let mut sim = room(id);
        assert!(stands(&mut sim, grille), "`{grille}` stands before the vigil: {:?}", standing(&mut sim));
    }
}
