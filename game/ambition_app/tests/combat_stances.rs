//! A combat stance is an item that rebinds the robot's whole home hit kit
//! (`items.ron`, `docs/planning/game/combat-stances.md`). Equipped through the
//! menu's road, each stance plays its own move for each attack verb, every
//! stance's pogo bounces off what it strikes, a Lunge thrust carries the body
//! where the Edge's swing roots no one, and the Crescent's string is three
//! steps.

#![cfg(feature = "rl_sim")]

use crate::common::base;
use ambition_app::rl_sim::TimestepMode;
use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core::{Block, Vec2};
use ambition_platformer2d::items::{Item, ItemEquipRequested};
use ambition_platformer2d::sim::AttackStrengthHint;
use bevy::prelude::{Entity, With};

fn open_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        crate::common::sandbox_options()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("central_hub_complex"),
    )
    .expect("sim")
}

fn primary(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    let mut query = world.query_filtered::<Entity, With<ambition_platformer2d::platformer::body::PrimaryBody>>();
    query.single(world).expect("one primary body")
}

fn equip(sim: &mut Platformer2dSimHarness, item: Item) {
    ambition_platformer2d::platformer::host_intents::write_host_intent(
        sim.world_mut(),
        ItemEquipRequested { participant: ambition_platformer2d::characters::control::PlayerSlot(0), item },
    );
    sim.step_n(base(), 3);
}

fn playing(sim: &Platformer2dSimHarness, body: Entity) -> Option<String> {
    sim.world().get::<MovePlayback>(body).map(|playback| playback.spec.id.clone())
}

fn x_of(sim: &mut Platformer2dSimHarness) -> f32 {
    sim.observation().player_pos.0
}

/// Press once from rest and return the first move that plays and how far the
/// body moved along x while it played.
fn press(sim: &mut Platformer2dSimHarness, body: Entity, action: AgentAction) -> (Option<String>, f32) {
    sim.step_n(base(), 60);
    let from = x_of(sim);
    sim.step(action);
    let mut first = playing(sim, body);
    for _ in 0..40 {
        sim.step(AgentAction { move_x: action.move_x, move_y: action.move_y, ..base() });
        first = first.or_else(|| playing(sim, body));
    }
    (first, x_of(sim) - from)
}

fn attack(move_y: f32) -> AgentAction {
    AgentAction { attack: true, attack_held: true, move_y, attack_strength: AttackStrengthHint::Tilt, ..base() }
}

fn smash_forward(facing: f32) -> AgentAction {
    AgentAction { attack: true, attack_held: true, move_x: facing, attack_strength: AttackStrengthHint::Smash, ..base() }
}

/// Drop onto a pogo orb with the pogo press: the move that plays and the
/// fastest rise after it.
fn pogo(sim: &mut Platformer2dSimHarness, body: Entity, at: (f32, f32)) -> (Option<String>, f32) {
    sim.step_n(base(), 60);
    sim.teleport_player(at);
    let mut first = None;
    let mut rise = f32::MIN;
    for _ in 0..40 {
        let o = sim.step(AgentAction { pogo: true, ..base() });
        first = first.or_else(|| playing(sim, body));
        rise = rise.max(-o.player_vel.1);
    }
    sim.step_n(base(), 90);
    (first, rise)
}

struct Stance {
    item: Item,
    attack: &'static str,
    up: &'static str,
    down: &'static str,
    smash: &'static str,
    pogo: &'static str,
}

const STANCES: [Stance; 3] = [
    Stance { item: Item::SwordStance, attack: "blade_rise", up: "blade_overhead", down: "blade_low", smash: "smash_forward", pogo: "air_down" },
    Stance { item: Item::LungeStance, attack: "lunge_thrust", up: "lunge_corkscrew", down: "lunge_slide", smash: "lunge_smash_forward", pogo: "lunge_plunge" },
    Stance {
        item: Item::CrescentStance,
        attack: "crescent_arc",
        up: "crescent_overhead",
        down: "crescent_low",
        smash: "crescent_smash_forward",
        pogo: "crescent_pogo",
    },
];

#[test]
fn each_stance_plays_its_own_kit_and_every_pogo_bounces() {
    let mut sim = open_sim();
    sim.step_n(base(), 120);
    let body = primary(&mut sim);
    let facing = sim.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(body).expect("a live body").facing.signum();
    let spawn = sim.observation().world_spawn;
    let above = (spawn.0, spawn.1 - 120.0);
    sim.grant_pogo_ability();
    sim.add_block(Block::pogo_orb("orb", Vec2::new(above.0, above.1 + 44.0), 30.0));

    let mut carried = Vec::new();
    for stance in &STANCES {
        equip(&mut sim, stance.item);
        sim.teleport_player(spawn);
        let (attacked, moved) = press(&mut sim, body, attack(0.0));
        assert_eq!(attacked.as_deref(), Some(stance.attack), "{:?}: the attack", stance.item);
        carried.push(moved * facing);
        assert_eq!(press(&mut sim, body, attack(-1.0)).0.as_deref(), Some(stance.up), "{:?}: up", stance.item);
        assert_eq!(press(&mut sim, body, attack(1.0)).0.as_deref(), Some(stance.down), "{:?}: down", stance.item);
        sim.teleport_player(spawn);
        assert_eq!(press(&mut sim, body, smash_forward(facing)).0.as_deref(), Some(stance.smash), "{:?}: the smash", stance.item);
        let (pogoed, rise) = pogo(&mut sim, body, above);
        assert_eq!(pogoed.as_deref(), Some(stance.pogo), "{:?}: the pogo", stance.item);
        assert!(rise > 400.0, "{:?}: the pogo did not bounce off the orb (rise {rise:.0})", stance.item);
    }
    // The Edge's 1-2 roots no one and carries no one; the Lunge's thrust steps.
    assert!(carried[0].abs() < 8.0, "the Edge's swing moved the body {:.1}", carried[0]);
    assert!(carried[1] > 20.0, "the Lunge's thrust did not carry the body: {:.1}", carried[1]);
}

/// The Crescent's string: a press in each step's tail is the next step.
#[test]
fn the_crescent_string_is_three_steps() {
    let mut sim = open_sim();
    sim.step_n(base(), 120);
    let body = primary(&mut sim);
    equip(&mut sim, Item::CrescentStance);
    sim.step_n(base(), 30);
    let mut seen: Vec<String> = Vec::new();
    for tick in 0..90 {
        let press = tick == 0 || tick == 20 || tick == 40;
        sim.step(AgentAction { attack: press, attack_held: press, ..base() });
        if let Some(id) = playing(&sim, body) {
            if seen.last() != Some(&id) {
                seen.push(id);
            }
        }
    }
    assert_eq!(seen, ["crescent_arc", "crescent_return", "crescent_sweep"], "the string");
}
