//! The Noodly Temple: a level of keys and doors, with the Flying Spaghetti
//! Monster in its last room.
//!
//! Seven rooms round one hall (`tools/ambition_ldtk_tools/specs/noodly_temple/`).
//! `check_temple.py` there walks a model of the level and proves that it can
//! be finished. This asks the game itself what the model takes as given: that
//! a key that is taken opens its doors and no other, that one wheel opens one
//! gate and shuts another, that the end of a fight opens the grille its room
//! names, and that the monster is in the sanctum.

use std::collections::BTreeSet;

use crate::common::{base, fixed_60hz_room_options, walk_through_the_door_to};
use ambition_app::{AgentAction, AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::ldtk_map::LdtkProject;

fn room(id: &str) -> Platformer2dSimHarness {
    let mut sim =
        Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(id))
            .unwrap_or_else(|e| panic!("`{id}` builds: {e}"));
    for _ in 0..30 {
        sim.step(base());
    }
    sim
}

/// The gate solids that stand in the live room, by name: a gated wall is
/// `gated_lock:<its id>` and the wall of a fight is `lockwall:<its encounter>`.
fn standing(sim: &mut Platformer2dSimHarness) -> BTreeSet<String> {
    ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<ambition_platformer2d::world::FeatureEcsWorldOverlay,
    >(
        sim.world_mut())
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

/// Stand on the lever `id` and press Interact once.
fn press(sim: &mut Platformer2dSimHarness, id: &str) {
    let at = {
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
    sim.teleport_player(at);
    sim.step(base());
    sim.step(AgentAction { interact: true, ..base() });
    for _ in 0..5 {
        sim.step(base());
    }
}

/// Each room of the temple has a door to the nave and back, the nave has a
/// door to the narthex and back, and the narthex to the central hub.
#[test]
fn the_temple_is_a_hall_with_a_door_to_each_room_and_back() {
    let manifest = ambition_content::worlds::world_manifest();
    let project = LdtkProject::load_default_for_dev(&manifest).expect("the world loads");
    let rooms = project.to_room_set(&manifest, &ambition_app::composed_ldtk_vocabulary()).expect("the world composes");
    let index = |id: &str| {
        rooms.room_index_by_id(id).unwrap_or_else(|| panic!("the world has no room `{id}`"))
    };
    let mut pairs = vec![("central_hub_complex", "noodly_narthex"), ("noodly_narthex", "noodly_nave"),
    ];
    for wing in ["noodly_scullery", "noodly_cistern", "noodly_belfry", "noodly_reliquary", "noodly_sanctum",
    ] {
        pairs.push(("noodly_nave", wing));
    }
    for (a, b) in pairs {
        let (from, to) = (index(a), index(b));
        assert!(rooms.neighboring_room_indices_of(from).contains(&to), "`{a}` has no door to `{b}`");
        assert!(rooms.neighboring_room_indices_of(to).contains(&from), "`{b}` has no door back to `{a}`");
    }
}

/// The Marinara Key is taken in the pantry of the scullery, and it is kept:
/// in the nave its two doors are open, and each other door stands.
#[test]
fn a_key_that_is_taken_opens_its_doors_and_no_other() {
    const DOORS: [(&str, bool); 5] = [
        ("noodly_nave_cistern_door_0", true),
        ("noodly_nave_belfry_door_0", true),
        ("noodly_nave_reliquary_door_0", false),
        ("noodly_nave_reliquary_door_1", false),
        ("noodly_nave_sanctum_door_0", false),
    ];
    // The control: with no key, each door of the nave stands.
    let mut nave = room("noodly_nave");
    for (door, _) in DOORS {
        assert!(stands(&mut nave, door), "`{door}` stands before a key is taken: {:?}", standing(&mut nave));
    }

    let mut sim = room("noodly_scullery");
    assert!(!flag(&mut sim, "noodly_key_marinara"), "the key is not taken yet");
    let key = {
        let world = sim.world_mut();
        world
            .query::<(&ambition_platformer2d::item_runtime::pickups::PickupFeature, &ambition_platformer2d::engine_core::geometry::CenteredAabb)>()
            .iter(world)
            .find(|(feature, _)| {
                matches!(&feature.pickup.kind, ambition_platformer2d::interaction::PickupKind::StoryFlag { flag } if flag == "noodly_key_marinara")
            })
            .map(|(_, aabb)| (aabb.center.x, aabb.center.y))
            .expect("the scullery has the Marinara Key")
    };
    sim.teleport_player(key);
    for _ in 0..20 {
        sim.step(base());
    }
    assert!(flag(&mut sim, "noodly_key_marinara"), "a body on the key takes it");

    assert_eq!(walk_through_the_door_to(&mut sim, "noodly_nave"), "noodly_nave");
    for _ in 0..10 {
        sim.step(base());
    }
    for (door, opened) in DOORS {
        assert_eq!(stands(&mut sim, door), !opened, "with the Marinara Key, `{door}`: {:?}", standing(&mut sim));
    }
}

/// Wheel I of the cistern opens the low gate and shuts the gate of the
/// well and the first gate of the lock; a second turn puts each back.
#[test]
fn a_wheel_opens_one_gate_and_shuts_another() {
    let mut sim = room("noodly_cistern");
    let read = |sim: &mut Platformer2dSimHarness| {
        ["noodly_cistern_gate_low", "noodly_cistern_gate_well", "noodly_cistern_gate_lock_a", "noodly_cistern_gate_high",
        ]
            .map(|wall| stands(sim, wall))
    };
    assert_eq!(read(&mut sim), [true, false, false, true], "with each wheel off: {:?}", standing(&mut sim));
    press(&mut sim, "noodly_wheel_a");
    assert_eq!(read(&mut sim), [false, true, true, true], "with wheel I on: {:?}", standing(&mut sim));
    press(&mut sim, "noodly_wheel_a");
    assert_eq!(read(&mut sim), [true, false, false, true], "with wheel I off again: {:?}", standing(&mut sim));
    press(&mut sim, "noodly_wheel_b");
    assert_eq!(read(&mut sim), [true, false, false, false], "wheel II opens the high gate only: {:?}", standing(&mut sim));
}

/// The fight of the scullery shuts the room while it is on, each of its three
/// waves comes, and its end opens the grille of the pantry.
#[test]
fn the_end_of_the_fight_opens_the_pantry() {
    use ambition_platformer2d::combat::components::EncounterMob;
    const FIGHT: &str = "noodly_scullery_brawl";
    const PANTRY: &str = "noodly_scullery_pantry_door";
    let mut sim = room("noodly_scullery");
    assert!(stands(&mut sim, PANTRY), "the pantry is shut before the fight");
    assert!(!standing(&mut sim).contains(&format!("lockwall:{FIGHT}")), "the room is open before the fight");

    // Into the strip that starts the fight.
    sim.teleport_player((352.0, 560.0));
    let mut shut = false;
    let mut mobs_seen = 0usize;
    let mut cleared = false;
    for _ in 0..60 * 40 {
        sim.step(base());
        shut |= standing(&mut sim).contains(&format!("lockwall:{FIGHT}"));
        // Each mob that is there is struck down, as a player who wins does.
        let world = sim.world_mut();
        let mut mobs = world.query_filtered::<&mut BodyHealth, bevy::prelude::With<EncounterMob>>();
        for mut health in mobs.iter_mut(world) {
            if health.alive() {
                mobs_seen += 1;
                health.health.current = 0;
            }
        }
        let state = world.resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>().data().encounter(FIGHT);
        if state == ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared {
            cleared = true;
            break;
        }
    }
    assert!(shut, "the fight shut the room");
    assert_eq!(mobs_seen, 8, "the three waves of the scullery are eight cooks");
    assert!(cleared, "the fight ended when each cook was down");
    for _ in 0..10 {
        sim.step(base());
    }
    assert!(!stands(&mut sim, PANTRY), "the end of the fight opens the pantry: {:?}", standing(&mut sim));
    assert!(!standing(&mut sim).contains(&format!("lockwall:{FIGHT}")), "the room is open after the fight");
}

#[test]
fn the_monster_is_in_the_sanctum() {
    let mut sim = room("noodly_sanctum");
    for _ in 0..60 {
        sim.step(base());
    }
    let world = sim.world_mut();
    let there = world
        .query::<(&BossConfig, &BodyHealth)>()
        .iter(world)
        .any(|(config, health)| {
            config.behavior.id == "flying_spaghetti_monster_boss" && health.alive()
        });
    assert!(there, "the Flying Spaghetti Monster is in the sanctum, alive");
}

/// A pool noodle lies by the pools of the cistern. An Attack press over it
/// picks it up, and the next Attack swings it: it stays in the hand.
#[test]
fn a_pool_noodle_by_the_pools_is_taken_and_swung() {
    use ambition_platformer2d::held_items::{GroundItem, ItemCustody};
    use bevy::prelude::Entity;

    let mut sim = room("noodly_cistern");
    let (noodle, at) = {
        let world = sim.world_mut();
        let mut items = world.query::<(Entity, &GroundItem)>();
        let mut noodles: Vec<(Entity, f32, f32)> =
            items.iter(world).filter(|(_, item)| item.spec.id == "pool_noodle").map(|(e, item)| (e, item.pos.x, item.pos.y)).collect();
        assert_eq!(noodles.len(), 2, "the cistern has two pool noodles");
        noodles.sort_by(|a, b| a.1.total_cmp(&b.1));
        (noodles[0].0, (noodles[0].1, noodles[0].2 - 16.0))
    };
    sim.teleport_player(at);
    for _ in 0..30 {
        sim.step(base());
    }
    let held = |sim: &Platformer2dSimHarness| {
        matches!(sim.world().get::<ItemCustody>(noodle), Some(ItemCustody::Held { .. }))
    };
    assert!(!held(&sim), "the noodle is on the floor until it is taken");
    sim.step(AgentAction { attack: true, ..base() });
    sim.step(base());
    assert!(held(&sim), "an Attack press over the pool noodle picks it up");
    // It has a swing, so a use keeps it: a coconut would be thrown here. The
    // swing is the swing of the item, not of the robot's blade: its clip is
    // the item's, it draws no blade arc, it reaches farther than the blade,
    // and the picture of the noodle turns in the hand while it lasts.
    for _ in 0..40 {
        sim.step(base());
    }
    let rest = swing_of_the_noodle(&mut sim);
    assert_eq!(rest, None, "a noodle that is held and not swung does not turn");
    sim.step(AgentAction { attack: true, ..base() });
    let mut turned = false;
    let mut blade_arc = false;
    let mut reach = 0.0_f32;
    let mut clip = String::new();
    for _ in 0..30 {
        sim.step(base());
        turned |= swing_of_the_noodle(&mut sim).is_some();
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&ambition_platformer2d::combat::moveset::MovePlayback, ambition_platformer2d::platformer::markers::PrimaryPlayerOnly>();
        if let Some(playback) = q.iter(world).next() {
            clip = playback.spec.clip.clip.clone();
            for volume in playback.spec.windows.iter().flat_map(|window| &window.volumes) {
                blade_arc |= volume.vfx.is_some();
                if let ambition_platformer2d::entity_catalog::VolumeShape::Rect { offset, half_extents,
                } = volume.shape {
                    reach = reach.max(offset.0 + half_extents.0);
                }
            }
        }
    }
    assert!(held(&sim), "an Attack press with the pool noodle in hand swings it and keeps it");
    assert_eq!(clip, "item_swing", "the swing is the swing of the item");
    assert!(!blade_arc, "the swing of the noodle draws no blade arc");
    assert!(reach > 80.0, "the noodle reaches farther than the blade: {reach}");
    assert!(turned, "the noodle turns in the hand while it is swung");
}

/// The swing the view shows for the pool noodle in the player's hand.
fn swing_of_the_noodle(sim: &mut Platformer2dSimHarness,
) -> Option<ambition_platformer2d::sim_view::ItemSwingFact> {
    sim.world_mut()
        .resource::<ambition_platformer2d::sim_view::HeldItemView>()
        .0
        .iter()
        .find(|fact| fact.item_id == "pool_noodle")
        .expect("the view has the noodle in a hand")
        .swing
}

/// ⭐ A PLAYER WALKS IN, TAKES THE FOUR KEYS AND BEATS THE MONSTER, WITH ITS
/// OWN INPUTS. From the central hub through the narthex and the nave to the
/// scullery by the room graph (`common::walk`), into the strip that starts
/// the fight, the fight won by Attack presses at the cooks (no health is
/// written), into the pantry the fight's end opens for the Marinara Key, and
/// back to the nave, whose two doors the key opens. Then the cistern's wheels
/// turned by Interact for the Pesto Key, the belfry's bells rung for the
/// Parmesan Key, the reliquary's vigil fought for the Colander Key, and the
/// Flying Spaghetti Monster fought in the sanctum until the save has it
/// beaten. No step puts the body anywhere (review 2026-10-12: each temple
/// test teleported or wrote health). The mechanism tests above are the
/// controls for each part.
#[test]
fn a_player_walks_in_takes_the_four_keys_and_beats_the_monster() {
    use crate::common::walk::{Pad, Walker as _, fight, walk_through, walk_to};
    use ambition_platformer2d::combat::components::EncounterMob;
    use ambition_platformer2d::persistence::save_data::PersistedEncounterState;
    const FIGHT: &str = "noodly_scullery_brawl";

    let mut sim = room("central_hub_complex");
    let key = {
        let mut pad = Pad::new(&mut sim);
        for (to, why) in [("noodly_narthex", "the hub"), ("noodly_nave", "the narthex"), ("noodly_scullery", "the nave"),
        ] {
            walk_through(&mut pad, to).unwrap_or_else(|e| panic!("from {why} to `{to}`: {e}"));
        }
        // Into the strip that starts the fight.
        walk_to(&mut pad, ambition_platformer2d::engine_core::Vec2::new(352.0, 560.0),
        )
            .unwrap_or_else(|e| panic!("to the strip of the fight: {e}"));
        let cleared = |world: &bevy::prelude::World| {
            world.resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>().data().encounter(FIGHT)
                == PersistedEncounterState::Cleared
        };
        // In the strip of the fight, where the walk above put the body.
        const STRIP: ambition_platformer2d::engine_core::Vec2 = ambition_platformer2d::engine_core::Vec2::new(352.0, 560.0);
        let frames = fight(&mut pad, |world, body| world.get::<EncounterMob>(body).is_some(), cleared, STRIP, 60 * 120,
        )
            .unwrap_or_else(|e| panic!("the fight of the scullery: {e}"));
        eprintln!("[temple] the fight of the scullery was won in {frames} frames");
        // The grille of the pantry and the lock walls fall a few frames after
        // the fight is cleared: wait, as a player does.
        for _ in 0..30 {
            pad.step(Default::default());
        }
        let key = {
            let world = pad.world_mut();
            world
                .query::<(&ambition_platformer2d::item_runtime::pickups::PickupFeature, &ambition_platformer2d::engine_core::geometry::CenteredAabb)>()
                .iter(world)
                .find(|(feature, _)| {
                    matches!(&feature.pickup.kind, ambition_platformer2d::interaction::PickupKind::StoryFlag { flag } if flag == "noodly_key_marinara")
                })
                .map(|(_, aabb)| aabb.center)
                .expect("the scullery has the Marinara Key")
        };
        walk_to(&mut pad, key).unwrap_or_else(|e| panic!("into the pantry to the key: {e}"));
        for _ in 0..20 {
            pad.step(Default::default());
        }
        key
    };
    assert!(flag(&mut sim, "noodly_key_marinara"), "the player took the key at {key:?}");
    {
        let mut pad = Pad::new(&mut sim);
        walk_through(&mut pad, "noodly_nave").unwrap_or_else(|e| panic!("back to the nave: {e}"));
        for _ in 0..10 {
            pad.step(Default::default());
        }
    }
    assert!(!stands(&mut sim, "noodly_nave_cistern_door_0"), "the key opens the door of the cistern: {:?}", standing(&mut sim));
    assert!(!stands(&mut sim, "noodly_nave_belfry_door_0"), "the key opens the door of the belfry: {:?}", standing(&mut sim));
    assert!(stands(&mut sim, "noodly_nave_sanctum_door_0"), "and not the door of the sanctum: {:?}", standing(&mut sim));

    // The cistern: the wheels in the order of the least turns to its key
    // (`check_temple.py` prints it), each walked to and turned with
    // Interact, then the Pesto Key.
    assert!(!flag(&mut sim, "noodly_key_pesto"), "the control: no Pesto Key before the cistern");
    {
        let mut pad = Pad::new(&mut sim);
        walk_through(&mut pad, "noodly_cistern").unwrap_or_else(|e| panic!("from the nave to the cistern: {e}"));
        for wheel in ["noodly_wheel_a", "noodly_wheel_b", "noodly_wheel_a", "noodly_wheel_c",
        ] {
            turn(&mut pad, wheel);
        }
        let key = key_at(pad.world_mut(), "noodly_key_pesto");
        walk_to(&mut pad, key).unwrap_or_else(|e| panic!("to the Pesto Key: {e}"));
        for _ in 0..20 {
            pad.step(Default::default());
        }
    }
    assert!(flag(&mut sim, "noodly_key_pesto"), "the player took the Pesto Key");

    // The belfry: its three bells rung, each by Interact, open its three
    // seals before the Parmesan Key.
    assert!(!flag(&mut sim, "noodly_key_parmesan"), "the control: no Parmesan Key before the belfry");
    {
        let mut pad = Pad::new(&mut sim);
        walk_through(&mut pad, "noodly_nave").unwrap_or_else(|e| panic!("from the cistern to the nave: {e}"));
        walk_through(&mut pad, "noodly_belfry").unwrap_or_else(|e| panic!("from the nave to the belfry: {e}"));
        for bell in ["noodly_bell_1", "noodly_bell_2", "noodly_bell_3"] {
            turn(&mut pad, bell);
        }
        let key = key_at(pad.world_mut(), "noodly_key_parmesan");
        walk_to(&mut pad, key).unwrap_or_else(|e| panic!("to the Parmesan Key: {e}"));
        for _ in 0..20 {
            pad.step(Default::default());
        }
    }
    assert!(flag(&mut sim, "noodly_key_parmesan"), "the player took the Parmesan Key");

    // The reliquary: its vigil fought and won, then the Colander Key behind
    // the grille the vigil opens.
    assert!(!flag(&mut sim, "noodly_key_colander"), "the control: no Colander Key before the reliquary");
    {
        const VIGIL: &str = "noodly_reliquary_vigil";
        // In the strip that starts the vigil, west of the spikes.
        const STRIP: ambition_platformer2d::engine_core::Vec2 = ambition_platformer2d::engine_core::Vec2::new(352.0, 560.0);
        let mut pad = Pad::new(&mut sim);
        walk_through(&mut pad, "noodly_nave").unwrap_or_else(|e| panic!("from the belfry to the nave: {e}"));
        walk_through(&mut pad, "noodly_reliquary").unwrap_or_else(|e| panic!("from the nave to the reliquary: {e}"));
        walk_to(&mut pad, STRIP).unwrap_or_else(|e| panic!("into the strip of the vigil: {e}"));
        let cleared = |world: &bevy::prelude::World| {
            world.resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>().data().encounter(VIGIL)
                == PersistedEncounterState::Cleared
        };
        let frames = fight(&mut pad, |world, body| world.get::<EncounterMob>(body).is_some(), cleared, STRIP, 60 * 120,
        )
            .unwrap_or_else(|e| {
                // The phase of the vigil: a fight that stalls in its intro
                // and one that waits for its last foe read the same above.
                let world = pad.world_mut();
                let mut vigils = world.query::<(&ambition_platformer2d::encounter::Encounter, &ambition_platformer2d::encounter::EncounterLifecycle,
            )>();
                let phases: Vec<_> = vigils.iter(world).filter(|(encounter, _)| encounter.id == VIGIL).map(|(_, life)| life.phase()).collect();
                panic!("the vigil of the reliquary: {e}; its phase {phases:?}")
            });
        eprintln!("[temple] the vigil of the reliquary was won in {frames} frames");
        for _ in 0..30 {
            pad.step(Default::default());
        }
        let key = key_at(pad.world_mut(), "noodly_key_colander");
        walk_to(&mut pad, key).unwrap_or_else(|e| panic!("to the Colander Key: {e}"));
        for _ in 0..20 {
            pad.step(Default::default());
        }
    }
    assert!(flag(&mut sim, "noodly_key_colander"), "the player took the Colander Key");

    // The sanctum: the four keys open its door, and the monster is fought
    // by Attack presses until the save has it beaten.
    let beaten = |world: &bevy::prelude::World| {
        let save = world.resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>();
        world
            .try_query::<&BossConfig>()
            .is_some_and(|mut configs| {
                configs.iter(world).any(|config| {
                    config.behavior.id == "flying_spaghetti_monster_boss"
                        && ambition_platformer2d::boss_encounter::boss_is_cleared(save, config)
                })
            })
    };
    {
        let mut pad = Pad::new(&mut sim);
        walk_through(&mut pad, "noodly_nave").unwrap_or_else(|e| panic!("from the reliquary to the nave: {e}"));
    }
    assert!(!stands(&mut sim, "noodly_nave_sanctum_door_0"), "the four keys open the door of the sanctum: {:?}", standing(&mut sim));
    assert!(!beaten(sim.world()), "the control: the monster is not beaten before the sanctum");
    {
        let mut pad = Pad::new(&mut sim);
        walk_through(&mut pad, "noodly_sanctum").unwrap_or_else(|e| panic!("from the nave to the sanctum: {e}"));
        const FLOOR: ambition_platformer2d::engine_core::Vec2 = ambition_platformer2d::engine_core::Vec2::new(480.0, 700.0);
        let frames = fight(&mut pad, |world, body| world.get::<BossConfig>(body).is_some(), beaten, FLOOR, 60 * 240,
        )
            .unwrap_or_else(|e| panic!("the fight of the sanctum: {e}"));
        eprintln!("[temple] the monster was beaten in {frames} frames");
    }
    assert!(beaten(sim.world()), "the save has the monster beaten");
}

/// Where the key that sets `flag` lies in the live room.
fn key_at(world: &mut bevy::prelude::World, flag: &str,
) -> ambition_platformer2d::engine_core::Vec2 {
    world
        .query::<(&ambition_platformer2d::item_runtime::pickups::PickupFeature, &ambition_platformer2d::engine_core::geometry::CenteredAabb)>()
        .iter(world)
        .find(|(feature, _)| {
            matches!(&feature.pickup.kind, ambition_platformer2d::interaction::PickupKind::StoryFlag { flag: f } if f == flag)
        })
        .map(|(_, aabb)| aabb.center)
        .unwrap_or_else(|| panic!("the room has the key `{flag}`"))
}

/// Walk to the lever `id` with the player's own inputs and turn it with one
/// press of Interact.
fn turn(pad: &mut crate::common::walk::Pad<'_>, id: &str) {
    use crate::common::walk::{Press, Walker as _, walk_to};
    let at = {
        let world = pad.world_mut();
        world
            .query::<(
                &ambition_platformer2d::encounter::switches::SwitchFeature,
                &ambition_platformer2d::engine_core::geometry::CenteredAabb,
            )>()
            .iter(world)
            .find(|(feature, _)| feature.activation.id == id)
            .map(|(_, aabb)| aabb.center)
            .unwrap_or_else(|| panic!("the room has no lever `{id}`"))
    };
    walk_to(pad, at).unwrap_or_else(|e| panic!("to the lever `{id}`: {e}"));
    pad.step(Press::default());
    pad.step(Press { interact: true, ..Press::default() });
    for _ in 0..20 {
        pad.step(Press::default());
    }
}
