//! The primeval trail: the level of the dinosaurs.
//!
//! Six rooms in a row (`dinosaur.ldtk`, LDtk-owned): the
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
use ambition_platformer2d::characters::actor::{BodyHealth, CharacterForm};
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
    let mut q = world.query::<(&CharacterForm, &BodyHealth, &ae::BodyKinematics)>();
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
    use ambition_content::primeval::COCONUT;
    use ambition_platformer2d::held_items::{GroundItem, ItemCustody};

    let mut sim = room("primeval_rookery");
    // The rule this App plays: the bird lets go no lower than this.
    let least_over: f32 = ambition_content::primeval::SWALLOWS_SCRIPT
        .lines()
        .find_map(|line| line.strip_prefix("const LET_GO_OVER = "))
        .and_then(|value| value.trim_end_matches(';').parse().ok())
        .expect("the swallows' script states LET_GO_OVER");
    // Each coconut a swallow holds now, and how far over the player the first
    // one to be let go was when it left the bird.
    let mut in_its_feet: Vec<Entity> = Vec::new();
    let mut carried = false;
    let mut let_go_over: Option<f32> = None;
    for _ in 0..60 * 30 {
        sim.step(AgentAction::default());
        let at = player(&mut sim);
        let world = sim.world_mut();
        let mut items = world.query::<(Entity, &GroundItem, &ae::BodyKinematics, &ItemCustody)>();
        let coconuts: Vec<(Entity, f32, ItemCustody)> =
            items.iter(world).filter(|(_, item, _, _)| item.spec.id == COCONUT).map(|(e, _, body, custody)| (e, body.pos.y, *custody)).collect();
        let mut worn = world.query::<&CharacterForm>();
        let mut held_now = Vec::new();
        for (coconut, y, custody) in coconuts {
            match custody {
                ItemCustody::Held { holder } if worn.get(world, holder).is_ok_and(|worn| worn.id() == "african_swallow") => {
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

/// The authored pair uses the real movement, custody, and falling-item systems.
#[test]
fn european_swallows_carry_one_coconut_together_and_drop_it_over_the_player() {
    use ambition_platformer2d::held_items::{GroundItem, ItemCustody, SupportedCarry};
    use ambition_platformer2d::combat::held_items::HeldItem;
    let mut sim = room("primeval_run");
    bring_player_into_swallow_range(&mut sim);
    let mut load = None;
    let mut airborne = false;
    let mut dropped = false;
    for _ in 0..60 * 35 {
        sim.step(AgentAction::default());
        let prey = player(&mut sim);
        let world = sim.world_mut();
        if let Some((item, leader, supporter, pickup_y)) = load {
            let custody = *world.get::<ItemCustody>(item).expect("the same coconut instance stays alive");
            if custody.in_world() {
                let ground = world.get::<ae::BodyKinematics>(item).unwrap();
                assert!(prey.y - ground.pos.y >= 75.0, "release is above the target: {ground:?}, prey={prey:?}");
                assert!(world.get::<SupportedCarry>(leader).is_none());
                assert!(world.get::<HeldItem>(supporter).is_none());
                assert!(ground.vel.y >= 0.0);
                dropped = true;
                break;
            }
            let a = world.get::<ae::BodyKinematics>(leader).unwrap();
            let b = world.get::<ae::BodyKinematics>(supporter).unwrap();
            let anchor = SupportedCarry::anchor(a, b, ae::Vec2::Y);
            airborne |= pickup_y - anchor.y > 40.0;
        } else {
            let mut pairs = world.query::<(Entity, &CharacterForm, &SupportedCarry)>();
            let pair = pairs.iter(world).find(|(_, worn, _)| worn.id() == "european_swallow").map(|(e, _, link)| (e, link.supporter));
            if let Some((leader, supporter)) = pair {
                assert_eq!(world.get::<CharacterForm>(supporter).unwrap().id(), "european_swallow");
                let mut items = world.query::<(Entity, &GroundItem, &ae::BodyKinematics, &ItemCustody)>();
                let (item, _, ground, _) = items.iter(world).find(|(_, ground, _, custody)| ground.spec.id == "coconut" && custody.held_by(leader)).unwrap();
                load = Some((item, leader, supporter, ground.pos.y));
            }
        }
    }
    assert!(load.is_some(), "the authored European pair did not join a load");
    assert!(airborne, "the pair did not lift the load");
    assert!(dropped, "the pair did not release the load over the player");
}

fn bring_player_into_swallow_range(sim: &mut Platformer2dSimHarness) {
    let world = sim.world_mut();
    let mut player = world.query_filtered::<(&mut ae::BodyKinematics, &mut BodyHealth), PrimaryPlayerOnly>();
    let (mut kin, mut health) = player.single_mut(world).unwrap();
    kin.pos.x = 800.0;
    *health = BodyHealth::new(ambition_platformer2d::characters::actor::Health::new(10000));
}

/// The same pair and coconut survive GGRS save, load, and replay.
#[test]
fn rollback_european_sling_replays_pickup_and_release() {
    use ambition_platformer2d::held_items::{GroundItem, ItemCustody, SupportedCarry};
    use ambition_platformer2d::platformer::sim_id::SimId;
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options("primeval_run").with_sync_test_rollback_settings(4, 10),
    ).expect("the trail builds under the sync-test host");
    for _ in 0..30 { sim.step(AgentAction::default()); }
    bring_player_into_swallow_range(&mut sim);
    sim.rebase_rollback_history().expect("the room fixture becomes the baseline");
    let before = sim.rollback_execution_stats().unwrap().lifetime_load_runs;
    let mut carried = None;
    let mut released = false;
    for _ in 0..360 {
        sim.step(AgentAction::default());
        sim.rollback_health().expect("the swallow sequence replays without a checksum mismatch");
        let world = sim.world_mut();
        let mut query = world.query::<(&SimId, &GroundItem, &ItemCustody)>();
        let nuts: Vec<_> = query.iter(world).filter(|(_, item, _)| item.spec.id == "coconut")
            .map(|(id, _, custody)| (id.as_str().to_owned(), *custody)).collect();
        for (id, custody) in nuts {
            match custody {
                ItemCustody::Held { holder } => {
                    if let Some(link) = world.get::<SupportedCarry>(holder) {
                        assert_eq!(world.get::<CharacterForm>(holder).unwrap().id(), "european_swallow");
                        assert_eq!(world.get::<CharacterForm>(link.supporter).unwrap().id(), "european_swallow");
                        assert_ne!(holder, link.supporter);
                        carried = Some(id);
                    }
                }
                ItemCustody::InWorld if carried.as_ref() == Some(&id) => released = true,
                _ => {}
            }
        }
        if released { break; }
    }
    assert!(carried.is_some(), "the pair must take custody during the replay window");
    assert!(released, "the same coconut must return to world custody");
    assert!(sim.rollback_execution_stats().unwrap().lifetime_load_runs > before, "LoadWorld must run");
}

/// A declared module that steers a room's birds away from their targets, as
/// one group (`group_think`, runtime authoring audit P4): one invocation for
/// the room lists its tagged members and its items, and each member is named
/// by its identity in its `steer`. It steers only when it sees the room's two
/// coconuts, so the items' list is part of what it reads.
const KEEP_AWAY: &str = r#"
fn declare() {
    #{
        provider: "drill",
        key: "keep_away",
        schemas: [],
        entries: [
            #{ key: "flock", phase: "actor_think",
               trigger: #{ port: "group_think", selector: "bird" },
               requests: ["steer"], limit: 16 },
        ],
    }
}

fn flock() {
    let room = this.trigger;
    let coconuts = room.items.filter(|item| item.kind == "coconut");
    if coconuts.len() != 2 { return; }
    for member in room.members {
        if !member.alive || member.driven || type_of(member.target) == "()" { continue; }
        let away = if member.position[0] < member.target[0] { -1.0 } else { 1.0 };
        this.requests.push(request("steer", #{ body: member.id, velocity: [away * 200.0, 0.0] }));
    }
}
"#;

/// ⭐ A GROUP DECIDES IN ONE INVOCATION: with the module, the birds of the
/// trailhead keep away from a player who stands still, through every rewind
/// of a sync-test session. The control is
/// `the_birds_of_the_trailhead_come_for_the_player`: with no module they come
/// within 70.
#[test]
fn a_script_steers_the_birds_of_the_trailhead_as_one_group() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../target/extension-modules/script_test_{}_keep_away", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("keep_away.rhai"), KEEP_AWAY).unwrap();
    let options = fixed_60hz_room_options("primeval_trailhead")
        .with_sync_test_rollback_settings(4, 10)
        .with_extension_module_files(vec![dir.join("keep_away.rhai")]);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the trailhead builds");
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
    assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    let faults = sim.world().resource::<ambition_platformer2d::extension::ExtensionFaults>().total;
    assert_eq!(faults, 0, "the module ran without a fault");
    assert!(closest > 200.0, "steered away, the nearest bird came to {closest} of the player");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A declared module that sends the trailhead's birds after one coconut:
/// every bird with a free hand asks to take the first coconut lying in the
/// room, each tick, near or not (the item owner refuses a take out of reach,
/// and a second take of one item in a tick); a bird holding one throws it
/// up every second and a half.
const FETCH: &str = r#"
fn declare() {
    #{
        provider: "drill",
        key: "fetch",
        schemas: [],
        entries: [
            #{ key: "flock", phase: "actor_think",
               trigger: #{ port: "group_think", selector: "bird" },
               requests: ["steer", "take", "release"], limit: 16 },
        ],
    }
}

fn flock() {
    let room = this.trigger;
    let coconuts = room.items.filter(|item| item.kind == "coconut");
    for member in room.members {
        if !member.alive || member.driven { continue; }
        if member.held == "coconut" {
            if this.tick % 90 == 0 {
                this.requests.push(request("release", #{ body: member.id, thrown: true, velocity: [0.0, -300.0] }));
            }
            continue;
        }
        if coconuts.is_empty() { continue; }
        let nut = coconuts[0];
        let way = if member.position[0] < nut.position[0] { 1.0 } else { -1.0 };
        this.requests.push(request("steer", #{ body: member.id, velocity: [way * 200.0, 0.0] }));
        this.requests.push(request("take", #{ body: member.id, item: nut.id }));
    }
}
"#;

/// ⭐ CUSTODY THROUGH THE ITEM OWNER: birds a script sends after a coconut
/// take it and throw it, by the item owner's own take and release, and no two
/// take one coconut: each bird that holds a coconut is that coconut's holder,
/// through every rewind of a sync-test session. The premise: on some tick two
/// birds were within reach of one coconut lying in the room.
#[test]
fn a_script_takes_and_throws_a_coconut_through_the_item_owner() {
    use ambition_platformer2d::combat::held_items::HeldItem;
    use ambition_platformer2d::held_items::{GroundItem, ItemCustody, ReleasedAs, Release};

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../target/extension-modules/script_test_{}_fetch", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("fetch.rhai"), FETCH).unwrap();
    let options = fixed_60hz_room_options("primeval_trailhead")
        .with_sync_test_rollback_settings(4, 10)
        .with_extension_module_files(vec![dir.join("fetch.rhai")]);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the trailhead builds");
    // The item owner's reach for a requested take (`ambition_combat_port::TAKE_REACH`).
    let reach = 32.0;
    let (mut takes, mut throws, mut contested) = (0, 0, false);
    let mut was_held: Vec<Entity> = Vec::new();
    for _ in 0..60 * 15 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        let mut items = world.query::<(Entity, &GroundItem, &ae::BodyKinematics, &ItemCustody, Option<&ReleasedAs>)>();
        let coconuts: Vec<_> = items
            .iter(world)
            .filter(|(_, item, ..)| item.spec.id == "coconut")
            .map(|(entity, _, kin, custody, released)| (entity, *kin, *custody, released.copied()))
            .collect();
        let mut hands = world.query::<(Entity, &CharacterForm, &ae::BodyKinematics, Option<&HeldItem>)>();
        let birds: Vec<_> = hands
            .iter(world)
            .filter(|(_, form, ..)| ["kiwi", "chicken", "chicken_hen"].contains(&form.id()))
            .map(|(entity, _, kin, held)| (entity, *kin, held.is_some_and(|held| held.id() == "coconut")))
            .collect();
        let held_now: Vec<Entity> = coconuts
            .iter()
            .filter_map(|(entity, _, custody, _)| matches!(custody, ItemCustody::Held { .. }).then_some(*entity))
            .collect();
        for (entity, _, custody, released) in &coconuts {
            if let ItemCustody::Held { holder } = custody {
                assert!(birds.iter().any(|(bird, _, holds)| bird == holder && *holds), "a held coconut's holder holds it");
                takes += usize::from(!was_held.contains(entity));
            } else if was_held.contains(entity) && *released == Some(ReleasedAs(Release::Throw)) {
                throws += 1;
            }
        }
        let holders = birds.iter().filter(|(_, _, holds)| *holds).count();
        assert_eq!(holders, held_now.len(), "each bird with a coconut in its hand is that coconut's holder");
        for (_, nut, custody, _) in &coconuts {
            let near = birds
                .iter()
                .filter(|(_, kin, holds)| {
                    let gap = (nut.pos - kin.pos).abs() - (nut.size() + kin.size()) * 0.5;
                    !holds && gap.max_element() <= reach
                })
                .count();
            contested |= custody.in_world() && near >= 2;
        }
        was_held = held_now;
    }
    assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    let faults = sim.world().resource::<ambition_platformer2d::extension::ExtensionFaults>().total;
    assert_eq!(faults, 0, "the module ran without a fault");
    assert!(takes >= 1 && throws >= 1, "a bird took a coconut ({takes}) and threw it ({throws})");
    assert!(contested, "premise: two birds were within reach of one coconut lying in the room");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A declared module that keeps the trailhead's flock in a record of the
/// room: its members and the coconut it would go for, by reference (each a
/// simulation identity), and how many ticks it has kept them.
const ROLL: &str = r#"
fn declare() {
    #{
        provider: "drill",
        key: "roll",
        schemas: [
            #{ key: "roll.flock", attachment: "body",
               fields: [
                   #{ tag: 1, name: "members", kind: "seq 16 body" },
                   #{ tag: 2, name: "goal", kind: "opt item" },
                   #{ tag: 3, name: "ticks", kind: "u32" },
               ] },
        ],
        entries: [
            #{ key: "flock", phase: "actor_think",
               trigger: #{ port: "group_think", selector: "bird" },
               writes: ["roll.flock"], requests: [], limit: 1 },
        ],
    }
}

fn flock() {
    let roll = this.state.flock;
    roll.members = this.trigger.members.map(|member| member.id);
    let coconuts = this.trigger.items.filter(|item| item.kind == "coconut");
    roll.goal = if coconuts.is_empty() { () } else { coconuts[0].id };
    roll.ticks += 1;
    this.state.flock = roll;
}
"#;

/// ⭐ A ROOM'S RECORD HOLDS REFERENCES: the record a group keeps lives on the
/// room's root, and its member and item references are the bodies' and the
/// coconut's simulation identities, through every rewind of a sync-test
/// session.
#[test]
fn a_room_record_keeps_its_flock_by_reference() {
    use ambition_platformer2d::extension::sdk::{FieldRef, SchemaKey, Value};
    use ambition_platformer2d::extension::BodyRecords;
    use ambition_platformer2d::held_items::GroundItem;
    use ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot;
    use ambition_platformer2d::platformer::sim_id::SimId;

    const FLOCK: SchemaKey = SchemaKey::new("drill", "roll.flock", 1);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../target/extension-modules/script_test_{}_roll", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("roll.rhai"), ROLL).unwrap();
    let options = fixed_60hz_room_options("primeval_trailhead")
        .with_sync_test_rollback_settings(4, 10)
        .with_extension_module_files(vec![dir.join("roll.rhai")]);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the trailhead builds");
    for _ in 0..120 {
        sim.step(AgentAction::default());
    }
    assert_eq!(ambition_platformer2d::rollback::session_health(sim.world()), Ok(()));
    let world = sim.world_mut();
    let mut birds: Vec<Value> = world
        .query::<(&CharacterForm, &SimId)>()
        .iter(world)
        .filter(|(form, _)| ["kiwi", "chicken", "chicken_hen"].contains(&form.id()))
        .map(|(_, id)| Value::Ref(id.as_str().to_owned()))
        .collect();
    birds.sort_by(|a, b| a.as_ref_id().cmp(&b.as_ref_id()));
    let first_coconut = world
        .query::<(&GroundItem, &SimId)>()
        .iter(world)
        .filter(|(item, _)| item.spec.id == "coconut")
        .map(|(_, id)| id.as_str().to_owned())
        .min()
        .expect("the trailhead has coconuts");
    let records = world
        .query_filtered::<&BodyRecords, bevy::prelude::With<RoomInstanceRoot>>()
        .iter(world)
        .next()
        .expect("the room's root keeps the group's record")
        .clone();
    let flock = records.get(&FLOCK).expect("the record of the flock");
    assert_eq!(flock.get(FieldRef(0)), Ok(&Value::Seq(birds)), "the members, by identity");
    assert_eq!(flock.get(FieldRef(1)), Ok(&Value::some(Value::Ref(first_coconut))), "the coconut, by identity");
    assert!(matches!(flock.get(FieldRef(2)), Ok(Value::U32(ticks)) if *ticks >= 100), "kept each tick: {:?}", flock.get(FieldRef(2)));
    let _ = std::fs::remove_dir_all(&dir);
}
