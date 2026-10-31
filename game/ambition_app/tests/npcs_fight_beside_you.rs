//! Ambition's people fight (Jon, 2026-10-30: "All NPCs need to have their
//! fight kit available in ambition. NPCs in dangerous areas that are aligned
//! with you need to help you fight, or defend themselves if they are
//! attacked.").
//!
//! - A struck NPC defends itself with its own kit and its character's own
//!   fighting mind (its provoked policy), not a generic one.
//! - An NPC on the player's side, in a room authored `dangerous`, joins the
//!   player's fight against a foe that is fighting the player, and goes back
//!   to its own mind when the fight is over (`body::rally_allies_to_the_fight`).
//! - The same kind of NPC in a room that is not dangerous stays out of it.
//!
//! The rangers of the primeval trail are the subjects: Brandvold keeps the
//! trailhead (dangerous), Dlamini the ranger station beside it (not).

#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_room_sim, hold_right};
use ambition_app::Platformer2dSimHarness;
use ambition_platformer2d::actor::{ActorFaction, ActorPolicy};
use ambition_platformer2d::character::AutonomousSource;
use ambition_platformer2d::character::BrainBinding;
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::combat::components::{ActorAggression, ActorDisposition, ActorTarget};
use ambition_platformer2d::combat::moveset::{ActorMoveset, MeleeSwingQuery, MovePlayback};
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::{Entity, World};

const BRANDVOLD: &str = "npc_primeval_brandvold";
const DLAMINI: &str = "npc_primeval_dlamini";

fn the_player(world: &mut World) -> Entity {
    let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
    q.single(world).expect("one primary player")
}

fn the_npc(world: &mut World, character: &str) -> Entity {
    let mut q = world.query::<(Entity, &WornCharacter, &ActorDisposition)>();
    q.iter(world)
        .find(|(_, worn, _)| worn.id() == character)
        .map(|(entity, ..)| entity)
        .unwrap_or_else(|| panic!("`{character}` stands in this room"))
}

/// The move ids of the body's own kit, and that it has one.
fn its_kit(world: &World, npc: Entity) -> Vec<String> {
    let moves: Vec<String> = world
        .get::<ActorMoveset>(npc)
        .expect("a built NPC carries its kit")
        .0
        .moves
        .iter()
        .map(|spec| spec.id.clone())
        .collect();
    assert!(!moves.is_empty(), "the NPC's kit is on its body");
    moves
}

/// The move the body is swinging this tick, if it is swinging.
fn swing(world: &mut World, npc: Entity) -> Option<String> {
    let mut q = world.query::<(MeleeSwingQuery, Option<&MovePlayback>)>();
    let (swing, playback) = q.get(world, npc).ok()?;
    swing.swing()?;
    Some(playback.map(|pb| pb.spec.id.clone()).unwrap_or_default())
}

/// Walk the player right until it stands `gap` px short of `npc`: the ranger
/// station's door is west of the ranger, farther off than she notices.
fn walk_up_to(sim: &mut Platformer2dSimHarness, player: Entity, npc: Entity, gap: f32) {
    let her = sim.world().get::<BodyKinematics>(npc).expect("the NPC is a body").pos.x;
    for _ in 0..600 {
        let at = sim.world().get::<BodyKinematics>(player).expect("the player is a body").pos.x;
        if at >= her - gap {
            sim.step_n(base(), 20);
            return;
        }
        sim.step(hold_right());
    }
    panic!("the player could not walk up to the NPC");
}

fn place_a_raptor_at(sim: &mut Platformer2dSimHarness, at: (f32, f32), id: &str) -> Entity {
    sim.spawn_enemy_character_at(
        id,
        "Raptor Stalker",
        at,
        (24.0, 40.0),
        CharacterBrain::Custom("raptor".to_string()),
        "npc_raptor_stalker",
    );
    let world = sim.world_mut();
    let mut q = world.query::<(Entity, &ambition_platformer2d::combat::components::FeatureId)>();
    q.iter(world).find(|(_, fid)| fid.as_str() == id).map(|(e, _)| e).expect("the raptor is placed")
}

/// Dlamini is struck three times by the player and fights back: hostile, with
/// her character's own provoked policy, swinging a move of her own kit.
#[test]
fn a_struck_npc_fights_back_with_its_own_kit_and_its_own_mind() {
    let mut sim = fixed_60hz_room_sim("primeval_ranger_station");
    sim.step_n(base(), 60);
    let player = the_player(sim.world_mut());
    let dlamini = the_npc(sim.world_mut(), DLAMINI);
    let kit = its_kit(sim.world(), dlamini);
    walk_up_to(&mut sim, player, dlamini, 120.0);
    let own_policy = {
        let world = sim.world();
        world
            .resource::<ambition_platformer2d::character::PreparedCharacterRegistry>()
            .get(DLAMINI)
            .and_then(|prepared| prepared.provoked_profile)
            .expect("an Ambition character resolves a provoked policy")
    };

    for _ in 0..3 {
        sim.world_mut().write_message(ambition_platformer2d::combat::events::ActorStimulus::DamagedBy {
            actor: dlamini,
            source: Some(player),
            damage: 1,
        });
        sim.step(base());
    }
    assert_eq!(
        sim.world().get::<ActorDisposition>(dlamini).copied(),
        Some(ActorDisposition::Hostile),
        "three blows provoke her"
    );
    assert_eq!(
        sim.world().get::<ActorPolicy>(dlamini).map(|policy| policy.0),
        Some(own_policy),
        "she fights with her character's own provoked policy"
    );

    let mut swung = None;
    for _ in 0..900 {
        sim.step(base());
        if let Some(id) = swing(sim.world_mut(), dlamini) {
            swung = Some(id);
            break;
        }
    }
    let swung = swung.expect("the provoked ranger never swung at the player");
    assert!(kit.contains(&swung), "she swings `{swung}`, a move of her own kit {kit:?}");
}

/// In the trailhead (dangerous), a raptor goes for the player beside
/// Brandvold: he takes it as his foe, fights it with his own kit, and when it
/// is gone he goes back to his own mind.
#[test]
fn an_npc_on_your_side_joins_your_fight_in_a_dangerous_room() {
    let mut sim = fixed_60hz_room_sim("primeval_trailhead");
    sim.step_n(base(), 30);
    let player = the_player(sim.world_mut());
    let brandvold = the_npc(sim.world_mut(), BRANDVOLD);
    let kit = its_kit(sim.world(), brandvold);
    walk_up_to(&mut sim, player, brandvold, 120.0);
    let at = sim.world().get::<BodyKinematics>(brandvold).expect("the NPC is a body").pos;
    let raptor = place_a_raptor_at(&mut sim, (at.x + 90.0, at.y), "trailhead_raptor");

    let mut rallied_against = None;
    let mut swung = None;
    for _ in 0..900 {
        sim.step(base());
        let world = sim.world_mut();
        if rallied_against.is_none()
            && world.get::<ActorDisposition>(brandvold).copied() == Some(ActorDisposition::Hostile)
        {
            rallied_against = world.get::<ActorTarget>(brandvold).and_then(|target| target.entity);
        }
        if rallied_against.is_some() {
            if let Some(id) = swing(world, brandvold) {
                swung = Some(id);
                break;
            }
        }
    }
    let foe = rallied_against.expect("the ranger never joined the fight");
    let world = sim.world_mut();
    assert_ne!(foe, player, "he fights beside the player, not against it");
    assert_eq!(world.get::<ActorFaction>(foe).copied(), Some(ActorFaction::Enemy), "his foe is the player's foe");
    let swung = swung.expect("the ranger joined the fight and never swung");
    assert!(kit.contains(&swung), "he swings `{swung}`, a move of his own kit {kit:?}");
    assert!(
        world.get::<ActorAggression>(brandvold).is_some_and(|aggression| aggression.grudge.is_some()),
        "a rally is a grudge against the foe"
    );

    // The fight ends: every foe in the room is gone.
    let foes: Vec<Entity> = {
        let mut q = world.query::<(Entity, &ActorFaction)>();
        q.iter(world).filter(|(_, faction)| **faction == ActorFaction::Enemy).map(|(e, _)| e).collect()
    };
    assert!(foes.contains(&raptor));
    for foe in foes {
        world.despawn(foe);
    }
    let mut stood_down = false;
    for _ in 0..120 {
        sim.step(base());
        let world = sim.world();
        let peaceful = world.get::<ActorDisposition>(brandvold).copied() == Some(ActorDisposition::Peaceful);
        let own_mind = world
            .get::<BrainBinding>(brandvold)
            .is_some_and(|binding| !matches!(binding.source, AutonomousSource::ProvokedProfile { .. }));
        if peaceful && own_mind {
            stood_down = true;
            break;
        }
    }
    assert!(stood_down, "with the fight over, the ranger goes back to his own mind");
    assert!(
        sim.world().get::<ActorAggression>(brandvold).is_some_and(|aggression| aggression.grudge.is_none()),
        "his feud ends with the fight"
    );
}

/// In the ranger station (not dangerous), the same raptor going for the
/// player beside Dlamini, who carries the same kit, does not draw her in.
#[test]
fn the_same_kind_of_npc_in_a_safe_room_stays_out_of_the_fight() {
    let mut sim = fixed_60hz_room_sim("primeval_ranger_station");
    sim.step_n(base(), 30);
    let player = the_player(sim.world_mut());
    let dlamini = the_npc(sim.world_mut(), DLAMINI);
    its_kit(sim.world(), dlamini);
    walk_up_to(&mut sim, player, dlamini, 260.0);
    // Past the player, on the side away from her, so its strikes at the
    // player do not reach her: a blow on her would be self-defence, which the
    // first test owns.
    let her = sim.world().get::<BodyKinematics>(dlamini).expect("the NPC is a body").pos;
    let them = sim.world().get::<BodyKinematics>(player).expect("the player is a body").pos;
    let away = if them.x >= her.x { 1.0 } else { -1.0 };
    let at = (them.x + away * 120.0, them.y);
    assert!((at.0 - her.x).abs() < 400.0, "the raptor is inside her notice range");
    let raptor = place_a_raptor_at(&mut sim, at, "station_raptor");

    let mut raptor_went_for_the_player = false;
    for _ in 0..600 {
        sim.step(base());
        let world = sim.world_mut();
        raptor_went_for_the_player |=
            world.get::<ActorTarget>(raptor).and_then(|target| target.entity) == Some(player);
        if world.get::<ActorAggression>(dlamini).is_some_and(|aggression| aggression.strikes > 0) {
            panic!("the raptor struck her; the fixture no longer keeps her out of its reach");
        }
        assert_eq!(
            world.get::<ActorDisposition>(dlamini).copied(),
            Some(ActorDisposition::Peaceful),
            "a ranger outside a dangerous room does not join the fight"
        );
        assert!(swing(world, dlamini).is_none(), "she never swings");
    }
    assert!(raptor_went_for_the_player, "the raptor fought the player, so the arm above is not vacuous");
}


