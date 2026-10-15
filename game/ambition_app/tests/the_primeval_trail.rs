//! The primeval trail: the level of the dinosaurs.
//!
//! Six rooms in a row (`tools/ambition_ldtk_tools/specs/primeval/`): the
//! T-rex's kin are its enemies and the T-rex is at its end. This asks four
//! things of it: that its rooms are a row with a door each way, that an enemy
//! it places hunts (a placed catalog character with no profile stands where
//! it was put and notices nobody), that an African swallow picks up a coconut
//! and lets it go over the player, and that its last room holds a T-rex that
//! has measured his hall.

use crate::common::{base, fixed_60hz_room_options};
use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_content::bosses::trex::{conductor_of, TREX_ID};
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::ldtk_map::LdtkProject;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::Entity;

/// The rooms of the trail, in the order a player walks them.
const TRAIL: [&str; 7] = [
    "central_hub_complex",
    "primeval_trailhead",
    "primeval_run",
    "primeval_rookery",
    "primeval_ridge",
    "primeval_canopy",
    "primeval_tyrant_hall",
];

fn room(id: &str) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(id)).unwrap_or_else(|e| panic!("`{id}` builds: {e}"));
    for _ in 0..30 {
        sim.step(base());
    }
    sim
}

/// Where each living body that wears catalog character `id` is.
fn wearing(sim: &mut Platformer2dSimHarness, id: &str) -> Vec<ae::Vec2> {
    let world = sim.world_mut();
    let mut q = world.query::<(&WornCharacter, &BodyHealth, &ae::BodyKinematics)>();
    q.iter(world).filter(|(worn, health, _)| worn.id() == id && health.alive()).map(|(_, _, kin)| kin.pos).collect()
}

fn player(sim: &mut Platformer2dSimHarness) -> ae::Vec2 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ae::BodyKinematics, PrimaryPlayerOnly>();
    q.iter(world).next().expect("the room has a player").pos
}

#[test]
fn the_rooms_of_the_trail_are_a_row_with_a_door_each_way() {
    let manifest = ambition_content::worlds::world_manifest();
    let project = LdtkProject::load_default_for_dev(&manifest).expect("the world loads");
    let rooms = project.to_room_set(&manifest, &ambition_app::composed_ldtk_vocabulary()).expect("the world composes");
    let index = |id: &str| rooms.room_index_by_id(id).unwrap_or_else(|| panic!("the world has no room `{id}`"));
    for pair in TRAIL.windows(2) {
        let (from, to) = (index(pair[0]), index(pair[1]));
        assert!(rooms.neighboring_room_indices_of(from).contains(&to), "`{}` has no door on to `{}`", pair[0], pair[1]);
        assert!(rooms.neighboring_room_indices_of(to).contains(&from), "`{}` has no door back to `{}`", pair[1], pair[0]);
    }
}

/// The cast each room places: the kinds, and how many of each.
const CAST: [(&str, &[(&str, usize)]); 5] = [
    ("primeval_trailhead", &[("kiwi", 2), ("chicken", 1), ("chicken_hen", 1), ("compsognathus", 3)]),
    (
        "primeval_run",
        &[
            ("emu", 2),
            ("european_swallow", 2),
            ("ostrich", 1),
            ("ostrich_hen", 1),
            ("pachycephalosaurus", 1),
            ("dilophosaurus", 1),
            ("willson", 1),
        ],
    ),
    ("primeval_rookery", &[("african_swallow", 3), ("penguin", 5), ("shoebill", 1)]),
    (
        "primeval_ridge",
        &[
            ("compsognathus", 3),
            ("willson", 2),
            ("african_swallow", 1),
            ("emu", 1),
            ("npc_raptor_stalker", 1),
            ("pachycephalosaurus", 1),
        ],
    ),
    (
        "primeval_canopy",
        &[
            ("owl", 1),
            ("npc_raptor_stalker", 2),
            ("shoebill", 1),
            ("african_swallow", 2),
            ("cassowary", 1),
            ("vulture", 1),
            ("dilophosaurus", 1),
            ("triceratops", 1),
        ],
    ),
];

#[test]
fn each_room_places_its_cast_and_its_coconuts() {
    for (id, cast) in CAST {
        let mut sim = room(id);
        for (kind, count) in cast {
            let here = wearing(&mut sim, kind);
            assert_eq!(here.len(), *count, "`{id}` places {count} `{kind}`; it has {here:?}");
        }
        let world = sim.world_mut();
        let mut items = world.query::<&ambition_platformer2d::held_items::GroundItem>();
        let coconuts = items.iter(world).filter(|item| item.spec.id == "coconut").count();
        assert_eq!(coconuts, 2, "`{id}` has two coconuts: to throw, and for a swallow to carry");
    }
}

/// An African swallow by the door of the rookery goes to the coconut under it,
/// takes it, carries it over a player who stands still, and lets it go from
/// high enough to hurt.
#[test]
fn an_african_swallow_takes_a_coconut_and_lets_it_go_over_the_player() {
    use ambition_content::primeval::{COCONUT, COCONUT_CARRIER};
    use ambition_platformer2d::held_items::{GroundItem, ItemCustody};

    let mut sim = room("primeval_rookery");
    // The tuning this App plays: the bird lets go no lower than this.
    let least_over = sim.world().resource::<ambition_content::tuning::AmbitionTuning>().0.african_swallow.let_go_over;
    // Each coconut a swallow holds now, and how far over the player the first
    // one to be let go was when it left the bird.
    let mut in_its_feet: Vec<Entity> = Vec::new();
    let mut carried = false;
    let mut let_go_over: Option<f32> = None;
    for _ in 0..60 * 30 {
        sim.step(AgentAction::default());
        let at = player(&mut sim);
        let world = sim.world_mut();
        let mut items = world.query::<(Entity, &GroundItem, &ItemCustody)>();
        let coconuts: Vec<(Entity, f32, ItemCustody)> =
            items.iter(world).filter(|(_, item, _)| item.spec.id == COCONUT).map(|(e, item, custody)| (e, item.pos.y, *custody)).collect();
        let mut worn = world.query::<&WornCharacter>();
        let mut held_now = Vec::new();
        for (coconut, y, custody) in coconuts {
            match custody {
                ItemCustody::Held { holder } if worn.get(world, holder).is_ok_and(|worn| worn.id() == COCONUT_CARRIER) => {
                    carried = true;
                    held_now.push(coconut);
                }
                ItemCustody::InWorld if in_its_feet.contains(&coconut) && let_go_over.is_none() => let_go_over = Some(at.y - y),
                _ => {}
            }
        }
        in_its_feet = held_now;
        if let_go_over.is_some() {
            break;
        }
    }
    assert!(carried, "no African swallow of the rookery took a coconut in 30 s");
    let over = let_go_over.expect("a swallow took a coconut and did not let it go in 30 s");
    assert!(over >= least_over - 12.0, "the coconut was let go {over} over the player: under {least_over}, it does not hurt");
}

/// A placed bird hunts: the kiwis and the chickens of the trailhead come for a
/// player who stands still at its door.
#[test]
fn the_birds_of_the_trailhead_come_for_the_player() {
    let mut sim = room("primeval_trailhead");
    let mut closest = f32::MAX;
    for _ in 0..60 * 12 {
        sim.step(AgentAction::default());
        let at = player(&mut sim);
        for kind in ["kiwi", "chicken", "chicken_hen"] {
            for bird in wearing(&mut sim, kind) {
                closest = closest.min(bird.distance(at));
            }
        }
    }
    assert!(closest < 70.0, "no bird of the trailhead came nearer than {closest} to a player who stood still");
}

#[test]
fn the_tyrant_is_in_his_hall_at_the_end_of_the_trail() {
    let mut sim = room("primeval_tyrant_hall");
    for _ in 0..60 {
        sim.step(base());
    }
    let world = sim.world_mut();
    let rex: Entity = world
        .query::<(Entity, &BossConfig, &BodyHealth)>()
        .iter(world)
        .find(|(_, config, health)| config.behavior.id == TREX_ID && health.alive())
        .map(|(entity, ..)| entity)
        .expect("the T-rex is in the hall, alive");
    let view = conductor_of(world, rex).expect("his conductor has a record");
    assert!(view.hall.is_some(), "his conductor has measured the hall: {view:?}");
}
