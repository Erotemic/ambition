//! The Mockingbird's air chase, played headlessly in the real rooms: its sky
//! (`mockingbird_sky`), the shore under it (`mockingbird_arena`), the scripted
//! pattern, and the conducted module that performs it
//! (`ambition_content_modules::mockingbird`).

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_content::bosses::mockingbird::{conductor_module as module, conductor_of, MockingbirdView, Move, MOCKINGBIRD_ID};
use ambition_platformer2d::boss_encounter::{BossConfig, BossEncounter};
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::world::collision::MovingPlatformSet;
use bevy::prelude::*;

const SKY: &str = "mockingbird_sky";
const SHORE: &str = "mockingbird_arena";
const PLACEMENT: &str = "cove.mockingbird";

fn boot(room: &str) -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED: the tolerant road falls back to the start room, which has
        // no Mockingbird, and every assertion below would be vacuous.
        .with_required_start_room(room);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the room builds headlessly");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    sim
}

fn sky() -> Platformer2dSimHarness {
    let mut sim = boot(SKY);
    untouchable_player(&mut sim, true);
    sim
}

/// Where a test holds the player: across the sky from the Mockingbird, out of
/// its snap's reach, so it fights at range (the flock would otherwise carry a
/// passive player into its jaws).
fn across_the_sky(sim: &mut Platformer2dSimHarness) -> ae::Vec2 {
    let room = bird(sim).view.room.expect("its room");
    ae::Vec2::new(room.x * 0.78, room.y * 0.45)
}

/// One tick with the player held across the sky.
fn step_across(sim: &mut Platformer2dSimHarness) {
    let at = across_the_sky(sim);
    place_player(sim, at);
    sim.step(AgentAction::default());
}

fn place_player(sim: &mut Platformer2dSimHarness, at: ae::Vec2) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), PrimaryPlayerOnly>();
    let (mut clusters, mut model) = q.iter_mut(world).next().expect("the player");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

fn player(sim: &mut Platformer2dSimHarness) -> (ae::BodyKinematics, i32) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(&ae::BodyKinematics, &BodyHealth), PrimaryPlayerOnly>();
    let (kin, health) = q.iter(world).next().expect("the player");
    (kin.clone(), health.current())
}

fn untouchable_player(sim: &mut Platformer2dSimHarness, untouchable: bool) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, untouchable);
    }
}

fn active_room(sim: &mut Platformer2dSimHarness) -> String {
    sim.observation().active_room.clone()
}

#[derive(Debug)]
struct Bird {
    kin: ae::BodyKinematics,
    hp: i32,
    guarded: bool,
    view: MockingbirdView,
}

fn bird(sim: &mut Platformer2dSimHarness) -> Bird {
    let world = sim.world_mut();
    let (entity, kin, hp, guarded) = world
        .query::<(Entity, &BossConfig, &ae::BodyKinematics, &BodyHealth, &BossEncounter)>()
        .iter(world)
        .find(|(_, config, ..)| config.behavior.id == MOCKINGBIRD_ID)
        .map(|(entity, _, kin, health, status)| (entity, kin.clone(), health.current(), status.guarded))
        .expect("the Mockingbird is in its sky");
    let view = conductor_of(world, entity).expect("its conductor has measured its room");
    Bird { kin, hp, guarded, view }
}

/// Wound it to `fraction` of its health: phase 2 under 60%, enraged under 25%.
fn wound(sim: &mut Platformer2dSimHarness, fraction: f32) {
    let world = sim.world_mut();
    let mut q = world.query::<(&BossConfig, &mut BodyHealth)>();
    let (_, mut health) = q
        .iter_mut(world)
        .find(|(config, _)| config.behavior.id == MOCKINGBIRD_ID)
        .expect("the Mockingbird");
    health.health.current = (health.max() as f32 * fraction).ceil() as i32;
}

fn step_until(sim: &mut Platformer2dSimHarness, frames: usize, what: &str, mut until: impl FnMut(&mut Platformer2dSimHarness) -> bool) {
    for _ in 0..frames {
        step_across(sim);
        if until(sim) {
            return;
        }
    }
    panic!("waited {frames} frames for {what}; last: {:?}", bird(sim));
}

/// The live room's platforms: (id, position, visual).
fn platforms(sim: &mut Platformer2dSimHarness) -> Vec<(String, ae::Vec2, Option<String>, bool)> {
    let set = ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<MovingPlatformSet>(sim.world())
        .expect("the room has platforms");
    set.0.iter().map(|p| (p.id.clone(), p.pos, p.visual.clone(), p.is_ferry())).collect()
}

/// Shots in the air: (visual, position, velocity).
fn shots(sim: &mut Platformer2dSimHarness) -> Vec<(String, ae::Vec2, ae::Vec2)> {
    let world = sim.world_mut();
    let mut q = world.query::<(&ambition_platformer2d::projectiles::ProjectileVisualId, &ae::BodyKinematics)>();
    q.iter(world)
        .filter(|(id, _)| id.0.starts_with("mockingbird_"))
        .map(|(id, kin)| (id.0.clone(), kin.pos, kin.vel))
        .collect()
}

/// A melee blow at the Mockingbird, the road a sword takes: what it did to
/// its health.
fn strike(sim: &mut Platformer2dSimHarness, damage: i32) -> i32 {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    let before = bird(sim);
    sim.world_mut().write_message(HitEvent {
        volume: ae::Aabb::new(before.kin.pos, ae::Vec2::splat(64.0)).into(),
        damage,
        source: HitSource::Melee,
        attacker: None,
        room: None,
        target: HitTarget::UnresolvedFeatures,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
        strike_sfx: None,
        attacker_move_instance: None,
    });
    sim.step(AgentAction::default());
    before.hp - bird(sim).hp
}

/// What it has played since `cursor` last listened, by name.
fn heard(sim: &mut Platformer2dSimHarness, cursor: &mut bevy::ecs::message::MessageCursor<ambition_platformer2d::sfx::OwnedSfxMessage>) -> Vec<&'static str> {
    use ambition_platformer2d::sfx::SfxMessage;
    let names: Vec<(ambition_platformer2d::sfx::SfxId, &'static str)> =
        module::SOUNDS.iter().map(|name| (ambition_platformer2d::sfx::SfxId::new(name), *name)).collect();
    let messages = sim.world().resource::<bevy::ecs::message::Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>();
    cursor
        .read(messages)
        .filter_map(|m| match m.request {
            SfxMessage::Play { id, .. } => names.iter().find(|(known, _)| *known == id).map(|(_, name)| *name),
            _ => None,
        })
        .collect()
}

/// It is drawn at the scale its volumes are measured at: the sky's spawn box
/// at the sheet's `collision_scale`, over its frame. A module PX that drifted
/// from either would put its bite and its missiles off its art.
#[test]
fn the_mockingbird_is_drawn_at_the_scale_its_volumes_are_measured() {
    let world_manifest = ambition_content::worlds::world_manifest();
    let project = ambition_platformer2d::ldtk_map::LdtkProject::load_default_for_dev(&world_manifest).expect("the world loads");
    let rooms = project.to_room_set(&world_manifest, &ambition_app::composed_ldtk_vocabulary()).expect("it lowers");
    let sheet = ambition_content::bosses::shipped_boss_catalog().sheet_for_key("mockingbird");
    let mut placed = 0;
    for room in &rooms.rooms {
        for boss in room.boss_spawns.iter().filter(|boss| boss.name == "Mockingbird") {
            placed += 1;
            let size = boss.aabb.max - boss.aabb.min;
            let px = size.x.max(size.y) * sheet.collision_scale / sheet.frame_height as f32;
            assert!((px - module::PX).abs() < 1e-3, "{} draws it at {px} world units a pixel; its module measures at {}", room.id, module::PX);
        }
    }
    assert!(placed >= 2, "precondition: the story's and the hall's skies place it ({placed})");
}

/// The sky never stops: its backdrop scrolls, and its sharks fly left in
/// their lanes and come back in at the right.
#[test]
fn the_sky_scrolls_and_its_sharks_fly_and_wrap() {
    let mut sim = sky();
    let spec_scroll = {
        let world = sim.world_mut();
        let definition = ambition_platformer2d::world::rooms::sole_live_room_definition(world).expect("a live room");
        let mut q = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let rooms = q.iter(world).next().expect("rooms");
        let spec = rooms.spec(definition);
        (spec.metadata.visual_profile.sky_scroll_px_s, spec.metadata.fall_rescue.clone())
    };
    assert_eq!(spec_scroll, (Some(-800), Some("burning_flying_shark".to_string())));
    let sharks: Vec<_> = platforms(&mut sim).into_iter().filter(|(.., ferry)| !ferry).collect();
    assert_eq!(sharks.len(), 9, "three lanes of three: {sharks:?}");
    assert!(sharks.iter().all(|(_, _, visual, _)| visual.as_deref() == Some("burning_flying_shark")));
    let mut wrapped = false;
    let mut last: Vec<f32> = sharks.iter().map(|(_, pos, ..)| pos.x).collect();
    for _ in 0..60 * 20 {
        sim.step(AgentAction::default());
        let now: Vec<f32> = platforms(&mut sim).into_iter().filter(|(.., ferry)| !ferry).map(|(_, pos, ..)| pos.x).collect();
        for (a, b) in last.iter().zip(&now) {
            if b - a > 1000.0 {
                wrapped = true;
            } else {
                assert!(b <= a, "a shark flew right: {a} -> {b}");
            }
        }
        last = now;
    }
    assert!(wrapped, "in twenty seconds no shark left the sky at the left and came back at the right");
}

/// A player who falls is caught by a shark and carried back up into the
/// fight: no death, no restart.
#[test]
fn a_falling_player_is_caught_by_a_shark_and_carried_back_up() {
    let mut sim = sky();
    let room = bird(&mut sim).view.room.expect("its room");
    // Somewhere no lane's shark is under: drop from just above the bottom.
    let (kin, hp) = player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(room.x * 0.5, room.y - 120.0));
    let mut lowest = 0.0f32;
    let mut highest_after = f32::MAX;
    for frame in 0..60 * 3 {
        sim.step(AgentAction::default());
        let (now, _) = player(&mut sim);
        lowest = lowest.max(now.pos.y);
        if frame > 60 {
            highest_after = highest_after.min(now.pos.y);
        }
    }
    let (now, hp_after) = player(&mut sim);
    assert_eq!(active_room(&mut sim), SKY, "the fall took the player out of the sky");
    assert!(lowest < room.y, "the player fell out of the sky ({lowest})");
    assert!(highest_after < room.y * 0.65, "the player was not carried back up (highest {highest_after}, now {:?})", now.pos);
    assert_eq!(hp_after, hp, "a rescued fall costs nothing here ({kin:?})");
    let ferries = platforms(&mut sim).iter().filter(|(.., ferry)| *ferry).count();
    // One carrier for this fall (and one for the fall the player booted
    // into): a rider who sank through its carrier would be caught again and
    // again, and the carriers would pile up.
    assert!((1..=2).contains(&ferries), "{ferries} carriers came up for two falls");
    let standing = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&ae::BodyGroundState, PrimaryPlayerOnly>();
        q.iter(world).next().expect("the player").on_ground
    };
    assert!(standing, "the player is not standing on the shark that caught them: {:?}", now.pos);
}

/// Out at its side, its hull turns every blow; winded after its dive's bite,
/// the same blow hurts it. That is the fight's punish window.
#[test]
fn out_at_its_side_its_hull_turns_blows_and_after_its_dive_it_is_open() {
    let mut sim = sky();
    step_until(&mut sim, 60 * 6, "it to hold its side, guarded", |sim| {
        let b = bird(sim);
        b.guarded && b.view.performing.is_some() && !b.view.stunned
    });
    assert_eq!(strike(&mut sim, 3), 0, "a blow at its guarded hull hurt it: {:?}", bird(&mut sim));
    step_until(&mut sim, 60 * 20, "it to dive, bite and hang winded", |sim| bird(sim).view.stunned);
    let b = bird(&mut sim);
    assert!(!b.guarded && b.view.open, "winded, its guard is down: {b:?}");
    assert_eq!(strike(&mut sim, 3), 3, "a blow while it hangs winded did not hurt it");
}

/// It dives across the sky at you, bites at the end, hangs winded where it
/// bit, and then flies home and raises its guard.
#[test]
fn it_dives_at_you_bites_hangs_winded_and_flies_home() {
    let mut sim = sky();
    step_until(&mut sim, 60 * 20, "it to dive", |sim| matches!(bird(sim).view.performing, Some((Move::Dive, true))));
    let home = bird(&mut sim).kin.pos;
    let mut cursor = Default::default();
    heard(&mut sim, &mut cursor);
    let mut said = Vec::new();
    step_until(&mut sim, 60 * 3, "the bite", |sim| {
        said.extend(heard(sim, &mut cursor));
        bird(sim).view.stunned
    });
    let bit_at = bird(&mut sim).kin.pos;
    assert!((bit_at - home).length() > 250.0, "it dove only {} from its side", (bit_at - home).length());
    assert!(said.contains(&"boss.mockingbird.chomp"), "it bit silently: {said:?}");
    step_until(&mut sim, 60 * 5, "it to fly home and raise its guard", |sim| {
        let b = bird(sim);
        !b.view.stunned && b.guarded
    });
    let back = bird(&mut sim).kin.pos;
    let room = bird(&mut sim).view.room.expect("room");
    assert!(back.x < room.x * 0.25, "home is its side of the sky, not {back:?}");
}

/// From its side it fires: a missile off its wingtip at you, and a fan of
/// fireballs from its throat.
#[test]
fn it_fires_missiles_and_fireballs_at_you() {
    let mut sim = sky();
    let mut seen = std::collections::BTreeMap::<String, usize>::new();
    let mut toward_you = false;
    for _ in 0..60 * 12 {
        step_across(&mut sim);
        let (you, _) = player(&mut sim);
        let now = shots(&mut sim);
        for (visual, pos, vel) in &now {
            let count = now.iter().filter(|(v, ..)| v == visual).count();
            let best = seen.entry(visual.clone()).or_default();
            *best = (*best).max(count);
            if visual == module::MISSILE_VISUAL && vel.dot(you.pos - *pos) > 0.0 {
                toward_you = true;
            }
        }
    }
    assert!(seen.get(module::MISSILE_VISUAL).is_some_and(|n| *n >= 1), "no missile: {seen:?}");
    assert!(toward_you, "its missiles flew away from the player");
    assert!(seen.get(module::FIRE_VISUAL).is_some_and(|n| *n >= 3), "no fan of fireballs: {seen:?}");
}

/// Wounded, it screeches into phase 2: its missiles come in salvoes and its
/// dive turns and comes again.
#[test]
fn wounded_it_screeches_and_then_fires_salvoes_and_dives_twice() {
    let mut sim = sky();
    let mut cursor = Default::default();
    heard(&mut sim, &mut cursor);
    wound(&mut sim, 0.5);
    let mut said = Vec::new();
    step_until(&mut sim, 60 * 8, "it to screech", |sim| {
        said.extend(heard(sim, &mut cursor));
        said.contains(&"boss.mockingbird.screech")
    });
    assert!(bird(&mut sim).view.screeching, "it screeched but is not rearing");
    let mut missiles = 0;
    step_until(&mut sim, 60 * 20, "a salvo", |sim| {
        missiles = missiles.max(shots(sim).iter().filter(|(v, ..)| v == module::MISSILE_VISUAL).count());
        missiles >= 3
    });
    step_until(&mut sim, 60 * 20, "the double dive", |sim| matches!(bird(sim).view.performing, Some((Move::DoubleDive, true))));
}

/// Enraged, it leaves its side: a strafing run along the top of the sky,
/// raining fire on the flock, and back low under it.
#[test]
fn enraged_it_strafes_the_top_of_the_sky_raining_fire() {
    let mut sim = sky();
    wound(&mut sim, 0.2);
    step_until(&mut sim, 60 * 20, "the strafing run", |sim| matches!(bird(sim).view.performing, Some((Move::Strafe, true))));
    let room = bird(&mut sim).view.room.expect("room");
    let (mut highest, mut furthest, mut bombs) = (f32::MAX, 0.0f32, 0);
    for _ in 0..60 * 3 {
        step_across(&mut sim);
        let b = bird(&mut sim);
        highest = highest.min(b.kin.pos.y);
        furthest = furthest.max(b.kin.pos.x);
        bombs = bombs.max(shots(&mut sim).iter().filter(|(v, _, vel)| v == module::FIRE_VISUAL && vel.y > 0.0).count());
    }
    assert!(highest < room.y * 0.2, "it stayed low ({highest})");
    assert!(furthest > room.x * 0.7, "it stayed on its side ({furthest})");
    assert!(bombs >= 4, "it rained only {bombs} fireballs");
}

/// Shot down, it falls out of the sky, and its treasure is not left falling
/// through it: the shore below hosts it.
#[test]
fn shot_down_it_falls_out_of_the_sky_leaving_no_chest_there() {
    let mut sim = sky();
    crate::boss_lifecycle::kill_boss_with_a_real_hit(&mut sim, PLACEMENT, 60 * 10);
    let room = bird(&mut sim).view.room.expect("room");
    // Held up in the sky: with it dead a fall is no longer caught, and would
    // take the player (and this test's view) down to the shore.
    step_until(&mut sim, 60 * 6, "it to fall out of the sky", |sim| {
        place_player(sim, ae::Vec2::new(room.x * 0.6, room.y * 0.3));
        bird(sim).kin.pos.y > room.y
    });
    let world = sim.world_mut();
    let chests = world.query::<&ambition_platformer2d::combat::BossRewardChest>().iter(world).count();
    assert_eq!(chests, 0, "a chest was left in the sky");
}

/// Its treasure falls onto the shore: a cleared Mockingbird's chest drops in
/// from the top of the room below its sky and lands on the floor.
#[test]
fn its_treasure_falls_onto_the_shore() {
    let mut sim = boot(SHORE);
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_boss(PLACEMENT, ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared);
    let chest = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world
            .query::<(&ambition_platformer2d::combat::BossRewardChest, &ambition_platformer2d::combat::CenteredAabb)>()
            .iter(world)
            .find(|(chest, _)| chest.encounter_id == PLACEMENT)
            .map(|(_, aabb)| aabb.center)
    };
    let mut first = None;
    for _ in 0..60 * 4 {
        sim.step(AgentAction::default());
        if first.is_none() {
            first = chest(&mut sim);
        }
    }
    let first = first.expect("no chest for the Mockingbird reached its shore");
    let landed = chest(&mut sim).expect("the chest");
    assert!(first.y < 120.0, "it did not fall in from the top: first at {first:?}");
    assert!(landed.y > 650.0, "it did not land on the floor: {landed:?}");
}

/// Once it is dead, a fall is a fall: the sky's bottom drops you to the shore.
#[test]
fn after_it_is_dead_a_fall_drops_you_to_the_shore() {
    let mut sim = sky();
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_boss(PLACEMENT, ambition_platformer2d::persistence::save_data::PersistedEncounterState::Cleared);
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
    assert!(bird(&mut sim).hp <= 0, "precondition: a cleared Mockingbird is dead");
    let room = bird(&mut sim).view.room.expect("room");
    place_player(&mut sim, ae::Vec2::new(room.x * 0.5, room.y - 120.0));
    let mut landed = false;
    for _ in 0..60 * 4 {
        sim.step(AgentAction::default());
        if active_room(&mut sim) == SHORE {
            landed = true;
            break;
        }
    }
    assert!(landed, "the fall did not drop the player to the shore (in {})", active_room(&mut sim));
}

/// The way up: on the shore a shark waits by a gap in the ceiling; stand on it
/// and it carries you up through the gap into the sky, where another catches
/// you.
#[test]
fn the_shore_shark_carries_you_up_into_the_sky() {
    let mut sim = boot(SHORE);
    untouchable_player(&mut sim, true);
    let (lift, pos) = platforms(&mut sim)
        .into_iter()
        .find(|(_, _, visual, ferry)| visual.is_some() && !ferry)
        .map(|(id, pos, ..)| (id, pos))
        .expect("the shore has its shark");
    let (kin, _) = player(&mut sim);
    place_player(&mut sim, ae::Vec2::new(pos.x, pos.y - 11.0 - kin.size.y * 0.5 - 1.0));
    let mut reached = false;
    for _ in 0..60 * 6 {
        sim.step(AgentAction::default());
        if active_room(&mut sim) == SKY {
            reached = true;
            break;
        }
    }
    assert!(reached, "the shark ({lift}) did not carry the player up into the sky");
    let hp = player(&mut sim).1;
    for _ in 0..60 * 3 {
        sim.step(AgentAction::default());
    }
    assert_eq!(active_room(&mut sim), SKY, "arriving, the player fell back out of the sky");
    let (now, hp_after) = player(&mut sim);
    assert!(now.pos.y < 700.0, "arriving from below, the player was not carried up: {:?}", now.pos);
    assert_eq!(hp_after, hp);
}
