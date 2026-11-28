//! Play the mine with its authored rooms, normal player input, and boss scripts.

use ambition_app::AgentAction;
use ambition_platformer2d::boss_encounter::{BossConfig, BossEncounterPhase};
use ambition_platformer2d::characters::actor::{BodyHealth, BodyWallet, CharacterForm};
use ambition_platformer2d::characters::brain::{BossAttackProfile, BossAttackState};
use ambition_platformer2d::combat::strike::{Hitbox, HitboxAnchor};
use ambition_platformer2d::engine_core::Vec2;
use ambition_platformer2d::extension::ExtensionFaults;
use ambition_platformer2d::platformer::lifecycle::sole_live_room_component;
use ambition_platformer2d::projectiles::ProjectileVisualId;
use ambition_platformer2d::vfx::vfx::AttackMarks;
use ambition_platformer2d::world::FeatureEcsWorldOverlay;
use ambition_platformer2d::world::collision::MovingPlatformSet;
use bevy::prelude::{Entity, World};

use crate::boss_lifecycle::{boss_cleared, boss_phase, boss_pos};
use crate::common::walk::{Pad, Press, Walker as _, fight, the_player, walk_through, walk_to};
use crate::common::{base, fixed_60hz_room_sim};

const YARD: &str = "science_mine_yard";
const SHAFT: &str = "science_mine_shaft";
const ASSAY: &str = "science_mine_assay";
const VENT: &str = "science_mine_vent";
const BASIN: &str = "science_mine_basin";
const POCKET: &str = "science_mine_pocket";
const LOWER: &str = "science_mine_lower";
const CAVERN: &str = "science_mine_lode";
const LODE: &str = "elemental_mine.mother_lode";

fn gate_stands(world: &mut World, gate: &str) -> bool {
    sole_live_room_component::<FeatureEcsWorldOverlay>(world)
        .expect("the live room has a collision overlay")
        .gate_solids
        .iter()
        .any(|solid| solid.name == format!("gated_lock:{gate}"))
}

fn interact_at(pad: &mut Pad<'_>, at: Vec2) {
    walk_to(pad, at).unwrap_or_else(|why| panic!("reach {at}: {why}"));
    pad.step(Press::default());
    pad.step(Press { interact: true, ..Press::default() });
    for _ in 0..20 {
        pad.step(Press::default());
    }
}

fn platform_position(world: &mut World, id: &str) -> Vec2 {
    sole_live_room_component::<MovingPlatformSet>(world)
        .expect("the live room has moving platforms")
        .0
        .iter()
        .find(|platform| platform.id == id)
        .unwrap_or_else(|| panic!("no moving platform {id}"))
        .pos
}

#[test]
fn the_relief_lever_opens_the_deep_works_and_the_service_stair_returns_to_the_yard() {
    let mut sim = fixed_60hz_room_sim(YARD);
    sim.step_n(base(), 30);
    let mut pad = Pad::new(&mut sim);
    walk_through(&mut pad, SHAFT).expect("take the pit stair");
    walk_through(&mut pad, ASSAY).expect("visit the assay gallery");
    assert!(gate_stands(pad.world_mut(), "mine_service_screen"));

    walk_through(&mut pad, BASIN).expect("take the sample stairs to the basin");
    walk_through(&mut pad, SHAFT).expect("reach the bottom of the main shaft");
    walk_through(&mut pad, LOWER).expect("inspect the lower screen");
    assert!(gate_stands(pad.world_mut(), "mine_deep_screen"));
    walk_through(&mut pad, SHAFT).expect("back up to the shaft base");
    walk_through(&mut pad, BASIN).expect("back into the basin");
    walk_through(&mut pad, VENT).expect("cross the basin to the drift");
    interact_at(&mut pad, Vec2::new(243.0, 608.0));

    walk_through(&mut pad, BASIN).expect("return to the settling basin");
    walk_through(&mut pad, SHAFT).expect("return to the lower landing");
    walk_through(&mut pad, LOWER).expect("return to the relief screen");
    assert!(!gate_stands(pad.world_mut(), "mine_deep_screen"));
    let shelf = Vec2::new(704.0, 592.0);
    walk_to(&mut pad, shelf).expect("approach the cart spur");
    fight(
        &mut pad,
        |world, body| world.get::<CharacterForm>(body).is_some_and(|form| form.id() == "uranium_elemental"),
        |world| {
            !world.iter_entities().any(|body| {
                body.get::<CharacterForm>().is_some_and(|form| form.id() == "uranium_elemental")
                    && body.get::<ambition_platformer2d::characters::actor::BodyHealth>().is_some_and(|health| health.alive())
            })
        },
        shelf,
        2200,
    ).expect("get past the uranium sentry");
    interact_at(&mut pad, Vec2::new(1338.0, 592.0));
    walk_through(&mut pad, ASSAY).expect("take the service stair back");
    assert!(!gate_stands(pad.world_mut(), "mine_service_screen"));
    walk_through(&mut pad, SHAFT).expect("take the top corridor");
    walk_through(&mut pad, YARD).expect("return to the ore yard");
    walk_through(&mut pad, "eureka_test_range").expect("cross to the Test Range");
    walk_through(&mut pad, YARD).expect("take the same road back to the yard");
}

#[test]
fn the_cracked_stope_has_a_breakable_seam_and_a_sample_locker() {
    use ambition_platformer2d::breakables::BreakableFeature;
    let mut sim = fixed_60hz_room_sim(POCKET);
    sim.step_n(base(), 30);
    let mut pad = Pad::new(&mut sim);
    walk_to(&mut pad, Vec2::new(584.0, 480.0)).expect("reach the cracked seam");
    let seam_broken = |world: &mut World| {
        world.query::<&BreakableFeature>().iter(world).any(BreakableFeature::broken)
    };
    assert!(!seam_broken(pad.world_mut()));
    for _ in 0..8 {
        pad.step(Press { axis: -1.0, jump_pressed: true, jump_held: true, ..Press::default() });
        for _ in 0..10 {
            pad.step(Press { axis: -1.0, jump_held: true, ..Press::default() });
        }
        pad.step(Press { axis: -1.0, jump_held: true, attack: true, ..Press::default() });
        for _ in 0..45 {
            pad.step(Press::default());
        }
        if seam_broken(pad.world_mut()) {
            break;
        }
    }
    assert!(seam_broken(pad.world_mut()), "the opening blade breaks the seam");
    let player = the_player(pad.world_mut());
    let before = pad.world().get::<BodyWallet>(player).unwrap().balance;
    interact_at(&mut pad, Vec2::new(230.0, 480.0));
    assert_eq!(pad.world().get::<BodyWallet>(player).unwrap().balance - before, 24);
    walk_through(&mut pad, BASIN).expect("leave the sample stope");
    walk_through(&mut pad, POCKET).expect("return to the sample stope");
}

#[test]
fn the_ore_shuttle_carries_a_rider_over_the_pool() {
    let mut sim = fixed_60hz_room_sim(BASIN);
    sim.step_n(base(), 30);
    walk_to(&mut Pad::new(&mut sim), Vec2::new(1360.0, 512.0)).expect("reach the east dock");
    let ready = (0..720).any(|_| {
        if platform_position(sim.world_mut(), "mine_ore_cart").x > 1180.0 { return true; }
        sim.step(base());
        false
    });
    assert!(ready, "the shuttle should reach the east dock");
    for tick in 0..75 {
        let x = sim.observation().player_pos.0;
        let center = platform_position(sim.world_mut(), "mine_ore_cart").x;
        let move_x = ((center - x) / 22.0).clamp(-1.0, 0.0);
        sim.step(AgentAction { move_x, jump: tick == 0, jump_held: tick < 13, ..base() });
        if sim.observation().on_ground && sim.observation().player_pos.1 < 452.0 { break; }
    }
    assert!(sim.observation().on_ground && sim.observation().player_pos.1 < 452.0,
        "land on the moving shuttle: {:?}", sim.observation().player_pos);
    let x_at_boarding = sim.observation().player_pos.0;
    for _ in 0..370 {
        let observation = sim.step(base());
        if observation.player_pos.0 < 620.0 && observation.on_ground {
            assert!(x_at_boarding - observation.player_pos.0 > 500.0);
            assert!(observation.player_pos.1 < 452.0);
            return;
        }
    }
    panic!("the shuttle did not carry the rider over the pool: {:?}", sim.observation().player_pos);
}

#[test]
fn mother_lode_warns_before_hazards_and_opens_the_sample_chest_after_the_fight() {
    let mut sim = fixed_60hz_room_sim(CAVERN);
    sim.step_n(base(), 25);
    assert!(gate_stands(sim.world_mut(), "mine_listening_screen"));
    walk_to(&mut Pad::new(&mut sim), Vec2::new(820.0, 636.0)).expect("reach the west crystal shelf");
    let player = the_player(sim.world_mut());
    let boss = {
        let world = sim.world_mut();
        world.query::<(Entity, &BossConfig)>().iter(world)
            .find(|(_, cfg)| cfg.id == LODE).unwrap().0
    };
    let hp = |world: &World| world.get::<BodyHealth>(player).unwrap().current();
    let mut saw_fault = false;
    let mut crystal_lanes = Vec::new();
    let mut center_fell_first = false;
    let mut saw_flanking_crystals = false;
    let mut split = None;
    let mut crossed_split = false;
    let mut hp_at_split = None;
    let mut punished = false;
    for frame in 0..2300 {
        let observation = sim.observation();
        let world = sim.world_mut();
        let marks: Vec<_> = world.resource::<AttackMarks>().0.iter()
            .filter(|mark| mark.key.owner == boss)
            .map(|mark| (mark.key.slot, mark.center, mark.half, mark.active))
            .collect();

        let mut visuals = world.query::<&ProjectileVisualId>();
        let crystals = visuals.iter(world).filter(|visual| visual.0 == "lode_falling_crystal").count();
        saw_fault |= visuals.iter(world).any(|visual| visual.0 == "lode_fault_wave");
        let attack = world.get::<BossAttackState>(boss).unwrap();
        let is_split = [&attack.telegraph_profile, &attack.active_profile].into_iter()
            .any(|profile| matches!(profile, Some(BossAttackProfile::Special(key)) if key == "lode_split"));
        let phase = boss_phase(sim.world_mut(), LODE);
        let mut input = base();
        if marks.len() == 3 && marks.iter().all(|mark| mark.2.y > mark.2.x) {
            if crystal_lanes.is_empty() && marks.iter().all(|mark| !mark.3) {
                crystal_lanes = marks.iter().map(|mark| (mark.0, mark.1, mark.2)).collect();
            }
            if phase == Some(BossEncounterPhase::Phase2) {
                for &(slot, center, half, _) in &marks {
                    assert!(crystal_lanes.contains(&(slot, center, half)), "the lanes must stay fixed");
                }
                if crystals == 1 && marks.iter().filter(|mark| mark.3).map(|mark| mark.0).eq([1]) {
                    center_fell_first = true;
                }
                if crystals >= 2 && marks.iter().all(|mark| mark.3) {
                    saw_flanking_crystals = true;
                }
            }
        }

        if !crossed_split && (!is_split || marks.is_empty()) {
            split = None;
        }
        if let (true, [left, right]) = (is_split && !crossed_split, marks.as_slice()) {
            let gap = (left.1.x + left.2.x + right.1.x - right.2.x) / 2.0;
            if !left.3 && !right.3 && split.is_none() {
                split = Some((marks.clone(), hp(sim.world())));
            }
            input.move_x = ((gap - observation.player_pos.0) / 22.0).clamp(-1.0, 1.0);
            if left.3 && right.3 {
                let (warning, health) = split.as_ref().expect("both bands must have a warning");
                let world = sim.world_mut();
                let mut hitboxes = world.query::<&Hitbox>();
                let active: Vec<_> = hitboxes.iter(world).filter_map(|hitbox| {
                    let HitboxAnchor::World { center } = hitbox.anchor else { return None };
                    (hitbox.owner == boss).then_some((center, hitbox.half_extent))
                }).collect();
                for &(_, center, half, _) in warning.iter() {
                    assert!(active.contains(&(center, half)),
                        "expected damage at {center:?} with half size {half:?}, got {active:?}");
                }
                assert!((observation.player_pos.0 - gap).abs() < 32.0 && observation.on_ground,
                    "reach the gap at {gap}, got {:?}", observation.player_pos);
                assert_eq!(hp(world), *health, "walk into the seam without a jump or a hit");
                crossed_split = true;
                hp_at_split.get_or_insert(world.get::<BodyHealth>(boss).unwrap().current());
            }
        }
        if let Some(before) = hp_at_split {
            punished |= sim.world().get::<BodyHealth>(boss).unwrap().current() < before;
        }
        if crossed_split && !punished && marks.is_empty() {
            let toward = boss_pos(sim.world_mut(), LODE).unwrap().x - observation.player_pos.0;
            if toward.abs() > 67.0 || toward * observation.facing < 0.0 {
                input.move_x = toward.signum();
            } else {
                input.attack = frame % 15 == 0;
            }
        }
        let may_hit = match phase {
            Some(BossEncounterPhase::Phase1) => saw_fault,
            Some(BossEncounterPhase::Phase2) => saw_flanking_crystals,
            Some(BossEncounterPhase::Enrage) => punished,
            _ => false,
        };
        if may_hit && frame % 91 == 0 {
            use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
            let at = boss_pos(sim.world_mut(), LODE).unwrap();
            sim.world_mut().write_message(HitEvent {
                status: None, origin: None, properties: Default::default(), contact: None, pieces: Vec::new(),
                volume: ambition_platformer2d::engine_core::Aabb::new(at, Vec2::splat(64.0)).into(),
                damage: 5, source: HitSource::Melee, attacker: None, room: None, target: HitTarget::UnresolvedFeatures,
                mode: HitMode::Knockback, knockback: None, ignored_targets: Vec::new(), strike_sfx: None,
                attacker_move_instance: None,
            });
        }
        sim.step(input);
        if boss_cleared(&sim, LODE) { break; }
    }
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent);
    assert!(saw_fault && center_fell_first && saw_flanking_crystals);
    assert!(crossed_split, "Mother Lode must leave a safe path through the split");
    assert!(punished, "the exit from the seam gives time to land a blade hit");
    assert!(boss_cleared(&sim, LODE), "the boss finish is saved");
    sim.step_n(base(), 95);
    let mut pad = Pad::new(&mut sim);
    assert!(!gate_stands(pad.world_mut(), "mine_listening_screen"));
    let reward = Vec2::new(2182.0, 720.0);
    walk_to(&mut pad, reward).expect("reach the sample chest");
    let player = the_player(pad.world_mut());
    let balance = |world: &World| world.get::<BodyWallet>(player).unwrap().balance;
    let before = balance(pad.world());
    interact_at(&mut pad, reward);
    assert_eq!(balance(pad.world()) - before, 65);
    walk_through(&mut pad, LOWER).expect("return to the lower workings");
}
