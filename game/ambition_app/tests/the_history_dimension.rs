//! The History dimension: its gate is DYNASTIC.
//!
//! Forty-seven rooms (`tools/ambition_ldtk_tools/specs/history_dimension/`):
//! a grid of forty outdoor rooms, where east is later and each floor is
//! a region of the world, and seven buildings by doors. The gate is in the
//! Cradle, in Africa, at the bottom left. This asks the game what the
//! generator takes as given: that the rooms of a floor join in order, that the
//! ways up and down join the floors where the regions met, that each building opens off
//! its room, that each room places its cast, that each way through opens for
//! its key (the first fire, the sluice, the solstice, the oracle bone, the
//! unicorn seal, the ballcourt ring, Socratease's answer, each brother at the Great Wall, the
//! mill's boiler), that the two vigils keep their store rooms shut until
//! they are over, and that the Door of No Return goes one way.

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

/// The ways of the grid: along each floor (later to the east), up and down
/// (a hatch in a floor, a tower of ledges under it), and into each building.
const WAYS: &[(&str, &str)] = &[
    // The Americas.
    ("history_beringia", "history_first_maize"),
    // Beringia's two ways south: inland at its top, along the coast at its foot.
    ("history_beringia", "history_andean_highlands"),
    ("history_first_maize", "history_olmec_heartland"),
    ("history_olmec_heartland", "history_maya_city"),
    ("history_maya_city", "history_taino_islands"),
    ("history_taino_islands", "history_haitian_revolution"),
    // South America.
    ("history_andean_highlands", "history_caral"),
    // East Asia and the steppe.
    ("history_mammoth_steppe", "history_yellow_river"),
    ("history_yellow_river", "history_shang_court"),
    ("history_shang_court", "history_silk_road"),
    ("history_silk_road", "history_mongol_steppe"),
    // Oceania.
    ("history_sahul", "history_new_guinea_highlands"),
    ("history_new_guinea_highlands", "history_lapita_voyagers"),
    ("history_lapita_voyagers", "history_wayfinders"),
    ("history_wayfinders", "history_rapa_nui"),
    // South Asia.
    ("history_southern_route", "history_mehrgarh"),
    ("history_mehrgarh", "history_indus_cities"),
    // Europe.
    ("history_ice_age_europe", "history_first_farmers_of_europe"),
    ("history_first_farmers_of_europe", "history_megalith_builders"),
    ("history_megalith_builders", "history_celtic_frontier"),
    ("history_celtic_frontier", "history_viking_north"),
    ("history_viking_north", "history_age_of_sail"),
    ("history_age_of_sail", "history_industrial_age"),
    // The Mediterranean and Southwest Asia.
    ("history_levant", "history_fertile_crescent"),
    ("history_fertile_crescent", "history_uruk"),
    ("history_uruk", "history_classical_shore"),
    // Africa.
    ("history_cradle", "history_southern_coast"),
    ("history_southern_coast", "history_green_sahara"),
    ("history_green_sahara", "history_nile_valley"),
    ("history_nile_valley", "history_aksum_and_kush"),
    ("history_aksum_and_kush", "history_timbuktu_coast"),
    ("history_timbuktu_coast", "history_benin_city"),
    ("history_benin_city", "history_door_of_no_return"),
    // Up and down: the great dispersal, out of Africa to every land mass.
    ("history_southern_coast", "history_levant"),
    ("history_levant", "history_ice_age_europe"),
    ("history_ice_age_europe", "history_southern_route"),
    ("history_southern_route", "history_sahul"),
    ("history_sahul", "history_mammoth_steppe"),
    ("history_mammoth_steppe", "history_beringia"),
    // Maize goes south to the Andes.
    ("history_first_maize", "history_andean_highlands"),
    // The Austronesians sail from Taiwan into the Pacific.
    ("history_shang_court", "history_lapita_voyagers"),
    // Farming spreads from the crescent; the first cities trade; Rome.
    ("history_green_sahara", "history_fertile_crescent"),
    ("history_fertile_crescent", "history_first_farmers_of_europe"),
    ("history_nile_valley", "history_uruk"),
    ("history_uruk", "history_megalith_builders"),
    ("history_aksum_and_kush", "history_classical_shore"),
    ("history_classical_shore", "history_celtic_frontier"),
    // The buildings.
    ("history_ice_age_europe", "history_painted_cave"),
    ("history_fertile_crescent", "history_temple_granary"),
    ("history_nile_valley", "history_pyramid_tomb"),
    ("history_classical_shore", "history_colosseum"),
    ("history_mongol_steppe", "history_great_ger"),
    ("history_timbuktu_coast", "history_timbuktu_library"),
    ("history_industrial_age", "history_cotton_mill"),
];

#[test]
fn the_rooms_of_the_history_grid_join_where_history_did() {
    let manifest = ambition_content::worlds::world_manifest();
    let project = LdtkProject::load_default_for_dev(&manifest).expect("the world loads");
    let rooms = project.to_room_set(&manifest, &ambition_app::composed_ldtk_vocabulary()).expect("the world composes");
    let index = |id: &str| rooms.room_index_by_id(id).unwrap_or_else(|| panic!("the world has no room `{id}`"));
    for &(a, b) in WAYS {
        let (from, to) = (index(a), index(b));
        assert!(rooms.neighboring_room_indices_of(from).contains(&to), "`{a}` has no way to `{b}`");
        assert!(rooms.neighboring_room_indices_of(to).contains(&from), "`{b}` has no way back to `{a}`");
    }
    // Everyone starts in Africa: nothing is before the Cradle, and its one
    // way on is the southern coast.
    let cradle = index("history_cradle");
    assert_eq!(rooms.neighboring_room_indices_of(cradle).len(), 1, "the Cradle joins only the southern coast");
    // The ways name all forty-seven rooms, and the gate reaches every one.
    let named: BTreeSet<&str> = WAYS.iter().flat_map(|&(a, b)| [a, b]).collect();
    assert_eq!(named.len(), 47, "the ways name every room of the dimension: {named:?}");
    let mut reached = BTreeSet::from(["history_cradle"]);
    while let Some(&(a, b)) = WAYS.iter().find(|&&(a, b)| reached.contains(a) != reached.contains(b)) {
        reached.extend([a, b]);
    }
    assert_eq!(reached, named, "from the Cradle the ways reach every room");
}

/// The Door of No Return, in the fort on the Gold Coast, opens on the plain
/// of Saint-Domingue, and nothing in Haiti leads back through it: the way
/// home from there is the ring (ANTILLES, `the_gate_network`).
#[test]
fn the_door_of_no_return_goes_one_way() {
    let manifest = ambition_content::worlds::world_manifest();
    let project = LdtkProject::load_default_for_dev(&manifest).expect("the world loads");
    let rooms = project.to_room_set(&manifest, &ambition_app::composed_ldtk_vocabulary()).expect("the world composes");
    let index = |id: &str| rooms.room_index_by_id(id).unwrap_or_else(|| panic!("the world has no room `{id}`"));
    let (fort, haiti) = (index("history_door_of_no_return"), index("history_haitian_revolution"));
    assert!(rooms.neighboring_room_indices_of(fort).contains(&haiti), "the door leads to Saint-Domingue");
    assert!(!rooms.neighboring_room_indices_of(haiti).contains(&fort), "nothing in Haiti leads back through the door");
}

/// The cast each room places: the kinds, and how many of each.
const CAST: &[(&str, &[(&str, usize)])] = &[
    // Africa.
    ("history_cradle", &[("npc_history_clio", 1), ("npc_history_lucy", 1), ("npc_history_mary_leaky", 1), ("hyena", 2)]),
    ("history_southern_coast", &[("npc_history_ochre_artist", 1), ("hyena", 2), ("cave_lion", 1)]),
    ("history_green_sahara", &[("npc_history_cattle_herder", 1), ("hippo", 2), ("locust_swarm", 1)]),
    (
        "history_nile_valley",
        &[("npc_history_hatshepsut_hats", 1), ("npc_history_ramesses_pretty_good", 1), ("scarab_beetle", 2), ("asp", 2), ("mummy", 1)],
    ),
    ("history_aksum_and_kush", &[("npc_history_kandake_can_do", 1), ("asp", 2), ("mummy", 1), ("scarab_beetle", 1)]),
    (
        "history_timbuktu_coast",
        &[("npc_history_mansa_moolah", 1), ("npc_history_zheng_hey", 1), ("inflated_coin", 2), ("coral_crab", 2), ("monsoon_squall", 2)],
    ),
    ("history_timbuktu_library", &[("npc_history_ibn_batootah", 1)]),
    (
        "history_benin_city",
        &[
            ("npc_history_brass_caster", 1),
            ("npc_history_iyoba_idia", 1),
            ("npc_history_lord_finders_keepers", 1),
            ("driver_ants", 2),
            ("asp", 1),
        ],
    ),
    ("history_door_of_no_return", &[("npc_history_ottobah_cugoano", 1), ("npc_history_castle_guide", 1)]),
    // The Mediterranean and Southwest Asia.
    ("history_levant", &[("npc_history_neville_neanderthal", 1), ("aurochs", 2), ("cave_lion", 1)]),
    (
        "history_fertile_crescent",
        &[("npc_history_hammurabbi", 1), ("npc_history_gilgamash", 1), ("locust_swarm", 2), ("aurochs", 1), ("scorpion_man", 1)],
    ),
    ("history_temple_granary", &[("npc_history_scribe_of_debts", 1)]),
    ("history_uruk", &[("npc_history_enheduanna", 1), ("scorpion_man", 1), ("sea_raider", 3)]),
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
    // Europe.
    ("history_ice_age_europe", &[("cave_bear", 1), ("woolly_mammoth", 1), ("cave_lion", 1)]),
    ("history_painted_cave", &[("npc_history_cave_painter", 1)]),
    ("history_first_farmers_of_europe", &[("npc_history_otzi_nice_man", 1), ("aurochs", 1), ("steppe_wolf", 2)]),
    ("history_megalith_builders", &[("npc_history_not_a_druid", 1), ("aurochs", 2), ("steppe_wolf", 1)]),
    ("history_celtic_frontier", &[("npc_history_boudic_car", 1), ("vandal", 2), ("steppe_wolf", 1)]),
    ("history_viking_north", &[("npc_history_leif_errandson", 1), ("viking_raider", 3)]),
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
    // South Asia.
    ("history_southern_route", &[("npc_history_coast_walker", 1), ("coral_crab", 2), ("tiger", 1)]),
    ("history_mehrgarh", &[("npc_history_dentist_of_mehrgarh", 1), ("locust_swarm", 1), ("tiger", 2)]),
    ("history_indus_cities", &[("npc_history_priest_king_probably", 1), ("indus_unicorn", 2), ("tiger", 1)]),
    // Oceania.
    ("history_sahul", &[("npc_history_raft_builder", 1), ("diprotodon", 1), ("megalania", 1)]),
    ("history_new_guinea_highlands", &[("npc_history_kuk_gardener", 1), ("cassowary_history", 3)]),
    ("history_lapita_voyagers", &[("npc_history_lapita_potter", 1), ("coconut_crab", 2), ("coral_crab", 1)]),
    ("history_wayfinders", &[("npc_history_wayfinder", 1), ("frigatebird", 3), ("coconut_crab", 1)]),
    ("history_rapa_nui", &[("npc_history_moai_carver", 1), ("npc_history_thor_hey_im_right_dahl", 1), ("pacific_rat", 3)]),
    // East Asia and the steppe.
    ("history_mammoth_steppe", &[("npc_history_denny_denisovan", 1), ("woolly_mammoth", 1), ("cave_bear", 1), ("steppe_wolf", 1)]),
    ("history_yellow_river", &[("npc_history_leizu", 1), ("silk_moth", 2), ("coral_crab", 2)]),
    ("history_shang_court", &[("npc_history_fu_hao", 1), ("taotie_spirit", 2), ("steppe_wolf", 1)]),
    (
        "history_silk_road",
        &[("npc_history_marco_polo_neck", 1), ("npc_history_confusius", 1), ("desert_bandit", 2), ("silk_moth", 2), ("terracotta_warrior", 2)],
    ),
    (
        "history_mongol_steppe",
        &[("npc_genghis_can", 1), ("npc_genghis_cant", 1), ("horse_archer", 2), ("steppe_wolf", 2), ("golden_eagle", 2)],
    ),
    ("history_great_ger", &[("npc_history_kublai_khant_wait", 1)]),
    // The Americas.
    ("history_beringia", &[("npc_history_dr_clovis_first", 1), ("woolly_mammoth", 1), ("smilodon", 2)]),
    ("history_first_maize", &[("npc_history_teosinte_breeder", 1), ("green_iguana", 1), ("howler_monkey", 1), ("jaguar", 2)]),
    // South America.
    (
        "history_andean_highlands",
        &[("npc_history_potato_farmer", 1), ("npc_history_antoine_parmen_tater", 1), ("llama", 2), ("andean_condor", 1)],
    ),
    ("history_caral", &[("npc_history_caral_flautist", 1), ("npc_history_net_maker", 1)]),
    ("history_olmec_heartland", &[("npc_history_head_sculptor", 1), ("jaguar", 2), ("howler_monkey", 1)]),
    (
        "history_maya_city",
        &[("npc_history_zero_hero", 1), ("npc_history_lady_six_sky_high", 1), ("howler_monkey", 1), ("jaguar", 1), ("green_iguana", 1)],
    ),
    (
        "history_taino_islands",
        &[("npc_history_cacica_of_boriken", 1), ("npc_history_canoe_maker", 1), ("coral_crab", 3)],
    ),
    (
        "history_haitian_revolution",
        &[
            ("npc_history_toussaint_louverture", 1),
            ("npc_history_catherine_flon", 1),
            ("npc_history_soup_joumou_cook", 1),
            ("napoleonic_grenadier", 3),
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

/// Each lever gate stands until its lever is pulled: the thorn boma in the
/// Cradle (the first fire), the trilithon (the solstice), the bronze gate
/// of the Shang court (the oracle bone) and the ballcourt gate of the Maya
/// city (the stone ring).
#[test]
fn each_lever_opens_its_gate() {
    for (id, lever, gate) in [
        ("history_cradle", "history_first_fire", "history_thorn_boma"),
        ("history_megalith_builders", "history_solstice", "history_trilithon_gate"),
        ("history_shang_court", "history_oracle_bone", "history_bronze_gate"),
        ("history_maya_city", "history_ballcourt_ring", "history_ballcourt_gate"),
    ] {
        let mut sim = room(id);
        assert!(stands(&mut sim, gate), "`{gate}` stands at first: {:?}", standing(&mut sim));
        press(&mut sim, lever);
        settle(&mut sim);
        assert!(!stands(&mut sim, gate), "after `{lever}`, `{gate}` is open: {:?}", standing(&mut sim));
    }
}

/// The citadel gate of the Indus cities stands until the traveller has the
/// unicorn seal (the flag of the pickup on the granary roof).
#[test]
fn the_unicorn_seal_opens_the_citadel() {
    let mut sim = room("history_indus_cities");
    assert!(stands(&mut sim, "history_seal_gate"), "the citadel gate stands at first: {:?}", standing(&mut sim));
    set_flag(&mut sim, "history_unicorn_seal");
    settle(&mut sim);
    assert!(!stands(&mut sim, "history_seal_gate"), "with the seal, the citadel gate is open: {:?}", standing(&mut sim));
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
