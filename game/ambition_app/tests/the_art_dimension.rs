//! The Art dimension: its gate is EASEL.
//!
//! Fifteen rooms (`tools/ambition_ldtk_tools/specs/art_dimension/`): the
//! plaza with the ring, the sketchbook cliffs over it, the sculpture garden
//! east of it, the inkfall caverns under both, the unfinished gallery by a
//! door in the garden, and four quarters for the arts that are not paint:
//! fiction (the story stacks, the library of endings), performance (the
//! amphitheatre, backstage), music (the resonant reach, the concert hall) and
//! teknis, the art of making (the workshop deeps, the loom hall), and the
//! Picture Palace off the amphitheatre (writers, directors and their machines). This asks the game what the generator takes as given: that the
//! rooms join as the flat map says, that each room places its cast, that a
//! pigment opens its own door of the colour gate and no other, that the lever
//! in the caverns opens the plaza's drain, and that the palette lift carries
//! a rider up to the cliffs, that the Curator by the ring talks, that the
//! three chimes of the reach open the concert hall's stage, and that the
//! crane's lever swings the bridge over the deeps' pit, and that an eraser
//! rubs out its wall of scribble.

use std::collections::BTreeSet;

use crate::common::{base, fixed_60hz_room_options, finish_passages};
use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::ldtk_map::LdtkProject;

use crate::the_pirate_port::{a_session_in, go_to, live_room, talk};

fn room(id: &str) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(id)).unwrap_or_else(|e| panic!("`{id}` builds: {e}"));
    for _ in 0..30 {
        sim.step(base());
    }
    sim
}

/// The gate solids that stand in the live room, by name (a gated wall is
/// `gated_lock:<its id>`).
fn standing(sim: &mut Platformer2dSimHarness) -> BTreeSet<String> {
    ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<ambition_platformer2d::world::FeatureEcsWorldOverlay>(
        sim.world_mut(),
    )
    .expect("the live room has a collision overlay")
    .gate_solids
    .iter()
    .map(|solid| solid.name.clone())
    .collect()
}

fn stands(sim: &mut Platformer2dSimHarness, wall: &str) -> bool {
    standing(sim).contains(&format!("gated_lock:{wall}"))
}

fn flag(sim: &mut Platformer2dSimHarness, id: &str) -> bool {
    sim.world_mut().resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>().data().flag(id)
}

/// How many bodies in the room wear catalog character `id`.
fn wearing(sim: &mut Platformer2dSimHarness, id: &str) -> usize {
    let world = sim.world_mut();
    let mut q = world.query::<&WornCharacter>();
    q.iter(world).filter(|worn| worn.id() == id).count()
}

/// Where the middle of a placed thing is, found by `find` over the room's
/// features of type `F`.
fn centre_of<F: bevy::prelude::Component>(sim: &mut Platformer2dSimHarness, what: &str, find: impl Fn(&F) -> bool) -> (f32, f32) {
    let world = sim.world_mut();
    world
        .query::<(&F, &ambition_platformer2d::engine_core::geometry::CenteredAabb)>()
        .iter(world)
        .find(|(feature, _)| find(feature))
        .map(|(_, aabb)| (aabb.center.x, aabb.center.y))
        .unwrap_or_else(|| panic!("the room has no {what}"))
}

/// The ways between the rooms, each way both ways. The plaza, the cliffs and
/// the garden are a loop, and so are the plaza, the caverns and the garden.
#[test]
fn the_rooms_of_the_art_dimension_join_as_the_flat_map_says() {
    const WAYS: [(&str, &str); 17] = [
        ("art_painters_plaza", "art_sketchbook_cliffs"),
        ("art_painters_plaza", "art_sculpture_garden"),
        ("art_sketchbook_cliffs", "art_sculpture_garden"),
        ("art_painters_plaza", "art_inkfall_caverns"),
        ("art_sculpture_garden", "art_inkfall_caverns"),
        ("art_sculpture_garden", "art_gallery_unfinished"),
        // The quarters: the stacks, the amphitheatre and the cliffs close a
        // loop over the plaza; the reach and the deeps are branches.
        ("art_painters_plaza", "art_story_stacks"),
        ("art_amphitheatre", "art_story_stacks"),
        ("art_sketchbook_cliffs", "art_amphitheatre"),
        ("art_sketchbook_cliffs", "art_resonant_reach"),
        ("art_inkfall_caverns", "art_workshop_deeps"),
        ("art_story_stacks", "art_library_of_endings"),
        ("art_amphitheatre", "art_backstage"),
        ("art_resonant_reach", "art_concert_hall"),
        ("art_workshop_deeps", "art_loom_hall"),
        ("art_painters_plaza", "art_curators_studio"),
        ("art_amphitheatre", "art_picture_palace"),
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
}

/// The cast each room places: the kinds, and how many of each.
const CAST: &[(&str, &[(&str, usize)])] = &[
    ("art_painters_plaza", &[("npc_art_curator", 1), ("npc_art_pablo_peckasso", 1), ("npc_art_rob_boss", 1)]),
    ("art_sketchbook_cliffs", &[("doodle_imp", 7), ("origami_bird", 3)]),
    ("art_sculpture_garden", &[("npc_art_frida_collage", 1), ("living_statue", 2), ("brush_knight", 1)]),
    ("art_inkfall_caverns", &[("ink_wisp", 4), ("doodle_imp", 1), ("brush_knight", 1)]),
    (
        "art_story_stacks",
        &[("bookworm", 3), ("red_herring", 2), ("paper_tiger", 1), ("npc_art_edgar_allan_crow", 1), ("npc_art_agatha_christly", 1)],
    ),
    ("art_library_of_endings", &[("npc_art_the_author", 1)]),
    ("art_amphitheatre", &[("mime", 1), ("critic", 2), ("stagehand", 1), ("npc_art_will_shakesbeard", 1)]),
    ("art_backstage", &[("npc_art_marcel_mimeau", 1)]),
    (
        "art_resonant_reach",
        &[("discord_note", 3), ("feedback_wail", 2), ("war_drum", 1), ("npc_art_ludwig_van_beathoven", 1)],
    ),
    ("art_concert_hall", &[("npc_art_johann_sebastian_batch", 1)]),
    (
        "art_picture_palace",
        &[
            ("npc_art_james_camera_ron", 1),
            ("npc_art_gale_anne_herd", 1),
            ("npc_art_t71", 1),
            ("npc_art_isaac_azimuth", 1),
            ("npc_art_ndr115", 1),
            ("npc_art_arthur_c_clerk", 1),
            ("npc_art_stanley_cubrick", 1),
            ("npc_art_mal9000", 1),
        ],
    ),
    (
        "art_workshop_deeps",
        &[("loom_spider", 2), ("kiln_sprite", 2), ("clay_golem", 1), ("npc_art_leonardo_da_vinchy", 1), ("npc_art_antoni_gaudy", 1)],
    ),
];

#[test]
fn each_room_of_the_art_dimension_places_its_cast() {
    for &(id, cast) in CAST {
        let mut sim = room(id);
        for (kind, count) in cast {
            assert_eq!(wearing(&mut sim, kind), *count, "`{id}` places {count} `{kind}`");
        }
    }
}

/// The yellow pigment is on the scaffold in the garden. Taken, it opens the
/// yellow door of the colour gate; the cyan and the magenta doors stand.
#[test]
fn a_pigment_opens_its_own_door_of_the_colour_gate_and_no_other() {
    use ambition_platformer2d::interaction::PickupKind;
    use ambition_platformer2d::item_runtime::pickups::PickupFeature;

    let mut sim = room("art_sculpture_garden");
    // The control: before a pigment is taken, each door stands.
    for colour in ["cyan", "magenta", "yellow"] {
        assert!(stands(&mut sim, &format!("art_colour_gate_{colour}")), "the {colour} door stands at first: {:?}", standing(&mut sim));
    }
    assert!(!flag(&mut sim, "art_pigment_yellow"), "the yellow pigment is not taken yet");
    let yellow = centre_of::<PickupFeature>(&mut sim, "yellow pigment", |feature| {
        matches!(&feature.pickup.kind, PickupKind::StoryFlag { flag } if flag == "art_pigment_yellow")
    });
    sim.teleport_player(yellow);
    for _ in 0..20 {
        sim.step(base());
    }
    assert!(flag(&mut sim, "art_pigment_yellow"), "a body on the yellow pigment takes it");
    for _ in 0..10 {
        sim.step(base());
    }
    for (colour, opened) in [("cyan", false), ("magenta", false), ("yellow", true)] {
        assert_eq!(stands(&mut sim, &format!("art_colour_gate_{colour}")), !opened, "with yellow, the {colour} door: {:?}", standing(&mut sim));
    }
}

/// The grate over the plaza's drain stands until the lever under it, in the
/// caverns, is pulled: a way up that opens from the far side.
#[test]
fn the_lever_in_the_caverns_opens_the_plaza_drain() {
    use ambition_platformer2d::encounter::switches::SwitchFeature;
    use ambition_platformer2d::engine_core::AabbExt as _;

    // The control: a plaza with the lever never pulled has its grate.
    let mut plaza = room("art_painters_plaza");
    assert!(stands(&mut plaza, "art_plaza_drain_grate"), "the drain's grate stands at first: {:?}", standing(&mut plaza));

    let mut sim = room("art_inkfall_caverns");
    let lever = centre_of::<SwitchFeature>(&mut sim, "drain lever", |feature| feature.activation.id == "art_drain_lever");
    sim.teleport_player(lever);
    sim.step(base());
    sim.step(AgentAction { interact: true, ..base() });
    for _ in 0..5 {
        sim.step(base());
    }
    // Up the shaft to the plaza: the edge across the caverns' roof.
    let shaft = crate::common::zone_to(&mut sim, "art_painters_plaza");
    let centre = shaft.aabb.center();
    sim.teleport_player((centre.x, centre.y));
    let mut arrived = String::new();
    for _ in 0..120 {
        arrived = sim.step(base()).active_room;
        if arrived == "art_painters_plaza" {
            break;
        }
    }
    assert_eq!(arrived, "art_painters_plaza", "up the drain's shaft from the caverns is the plaza");
    finish_passages(&mut sim);
    for _ in 0..10 {
        sim.step(base());
    }
    assert!(!stands(&mut sim, "art_plaza_drain_grate"), "with the lever pulled, the grate is open: {:?}", standing(&mut sim));
}

/// The plaza's palette lift: room px of the floor under it, and of its top
/// where it waits a hop over the floor (`make_specs.py`: `LIFT_SHAFT`,
/// `LIFT_HOP`).
const UNDER_THE_LIFT: ae::Vec2 = ae::Vec2::new(1728.0, 960.0);
const LIFT_TOP: ae::Vec2 = ae::Vec2::new(1728.0, 920.0);

/// A rider who stands on the palette lift in the plaza is carried up through
/// the roof and comes out in the sketchbook cliffs, where the hatch over the
/// shaft holds it.
#[test]
fn the_palette_lift_carries_a_rider_up_to_the_cliffs() {
    use ambition_platformer2d::platformer::markers::PrimaryPlayer;
    use bevy::prelude::With;

    let mut app = a_session_in("art_painters_plaza");
    // The walker's map is the room's still geometry: it walks to the floor
    // under the lift, and the hop onto it is a placement.
    go_to(&mut app, UNDER_THE_LIFT, "the floor under the palette lift");
    let half = {
        let mut query = app.world_mut().query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
        query.iter(app.world()).next().expect("gameplay has a primary player").size.y * 0.5
    };
    {
        let mut query = app
            .world_mut()
            .query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), With<PrimaryPlayer>>();
        let world = app.world_mut();
        let (mut cluster_item, mut motion_model) = query.iter_mut(world).next().expect("gameplay has a primary player");
        let mut clusters = cluster_item.as_clusters_mut();
        let onto = ae::Vec2::new(LIFT_TOP.x, LIFT_TOP.y - half - 1.0);
        ae::movement::transit_body(&mut motion_model, &mut clusters, onto, ae::movement::TransitVelocity::Zero);
    }
    let mut rooms = vec![live_room(&mut app)];
    for _ in 0..60 * 15 {
        app.update();
        let room = live_room(&mut app);
        if rooms.last() != Some(&room) {
            rooms.push(room);
        }
    }
    assert_eq!(rooms, ["art_painters_plaza", "art_sketchbook_cliffs"], "the rider's rooms on the way up, and it stays up");
}

/// The Curator stands by the ring and talks (`dialogue/sandbox/art.yarn`):
/// a player walks over, presses Interact, and is offered the first question.
#[test]
fn the_curator_by_the_ring_talks() {
    let mut app = a_session_in("art_painters_plaza");
    assert_eq!(talk(&mut app, "npc_art_curator", "What is this place?"), "What is this place?");
}

/// Stand on the lever `id` in the live room and press Interact once.
fn press(sim: &mut Platformer2dSimHarness, id: &str) {
    use ambition_platformer2d::encounter::switches::SwitchFeature;
    let at = centre_of::<SwitchFeature>(sim, id, |feature| feature.activation.id == id);
    sim.teleport_player(at);
    sim.step(base());
    sim.step(AgentAction { interact: true, ..base() });
    for _ in 0..5 {
        sim.step(base());
    }
}

/// The concert hall's stage is shut by three bars, one for each chime of the
/// reach. Two chimes leave the third bar up; the third opens the stage.
#[test]
fn the_three_chimes_of_the_reach_open_the_concert_halls_stage() {
    const BARS: [&str; 3] = ["art_chord_bar_do", "art_chord_bar_mi", "art_chord_bar_sol"];
    // The control: a hall whose chimes were never struck has all its bars.
    let mut hall = room("art_concert_hall");
    for bar in BARS {
        assert!(stands(&mut hall, bar), "`{bar}` stands at first: {:?}", standing(&mut hall));
    }

    let mut sim = room("art_resonant_reach");
    press(&mut sim, "art_chime_do");
    press(&mut sim, "art_chime_mi");
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "art_concert_hall"), "art_concert_hall");
    for _ in 0..10 {
        sim.step(base());
    }
    for (bar, open) in [("art_chord_bar_do", true), ("art_chord_bar_mi", true), ("art_chord_bar_sol", false)] {
        assert_eq!(stands(&mut sim, bar), !open, "after do and mi, `{bar}`: {:?}", standing(&mut sim));
    }

    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "art_resonant_reach"), "art_resonant_reach");
    press(&mut sim, "art_chime_sol");
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "art_concert_hall"), "art_concert_hall");
    for _ in 0..10 {
        sim.step(base());
    }
    for bar in BARS {
        assert!(!stands(&mut sim, bar), "with the chord struck, `{bar}` is open: {:?}", standing(&mut sim));
    }
}

/// The deeps' pit has no bridge until the crane's lever is pulled; then its
/// three planks stand over the fire.
#[test]
fn the_cranes_lever_swings_the_bridge_over_the_pit() {
    const PLANKS: [&str; 3] = ["art_deeps_bridge_0", "art_deeps_bridge_1", "art_deeps_bridge_2"];
    let mut sim = room("art_workshop_deeps");
    // The control: before the lever, nothing spans the pit.
    for plank in PLANKS {
        assert!(!stands(&mut sim, plank), "`{plank}` is not there before the lever: {:?}", standing(&mut sim));
    }
    press(&mut sim, "art_bridge_crane");
    for _ in 0..10 {
        sim.step(base());
    }
    for plank in PLANKS {
        assert!(stands(&mut sim, plank), "with the lever pulled, `{plank}` spans the pit: {:?}", standing(&mut sim));
    }
}

/// The endings of the library: both doors stand until the Author writes one,
/// and the curtain of the amphitheatre stands until the audition is over.
#[test]
fn the_endings_and_the_curtain_wait_for_their_story() {
    let mut library = room("art_library_of_endings");
    for door in ["art_library_door_happy", "art_library_door_sad"] {
        assert!(stands(&mut library, door), "`{door}` stands before an ending is written: {:?}", standing(&mut library));
    }
    let mut stage = room("art_amphitheatre");
    assert!(stands(&mut stage, "art_amphi_curtain"), "the curtain is down before the audition: {:?}", standing(&mut stage));
}

/// The eraser switches of the concept sheets: in the cliffs and in the
/// studio, a wall of scribble stands until its eraser is pulled.
#[test]
fn an_eraser_rubs_out_its_wall_of_scribble() {
    for (room_id, eraser, wall) in [
        ("art_sketchbook_cliffs", "art_eraser_cliffs", "art_cliffs_scribble"),
        ("art_curators_studio", "art_eraser_studio", "art_studio_scribble"),
    ] {
        let mut sim = room(room_id);
        assert!(stands(&mut sim, wall), "`{wall}` stands before the eraser: {:?}", standing(&mut sim));
        press(&mut sim, eraser);
        for _ in 0..10 {
            sim.step(base());
        }
        assert!(!stands(&mut sim, wall), "with `{eraser}` pulled, `{wall}` is rubbed out: {:?}", standing(&mut sim));
    }
}

/// The space set of the Picture Palace: MAL-9000 will not open the pod bay
/// door, and the door stands until MAL's memory module is pulled.
#[test]
fn pulling_mals_memory_opens_the_pod_bay_door() {
    let mut sim = room("art_picture_palace");
    assert!(stands(&mut sim, "art_pod_bay_door"), "the pod bay door stands at first: {:?}", standing(&mut sim));
    press(&mut sim, "art_mal_memory");
    for _ in 0..10 {
        sim.step(base());
    }
    assert!(!stands(&mut sim, "art_pod_bay_door"), "with MAL's memory pulled, the pod bay door is open: {:?}", standing(&mut sim));
}
