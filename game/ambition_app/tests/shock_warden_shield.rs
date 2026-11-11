//! The Shock Warden's riot shield, played headlessly against the real player
//! (`ambition_combat::guard`; Jon, 2026-10-19). A strike into the shield from
//! the front is turned aside: it does him no harm, the player is thrown back
//! and locked out, and he answers with a shove of the shield. A strike down
//! on him from over the top of the shield gets through.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::combat::guard::{GuardState, GuardVolumes};
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const WARDEN: &str = "shield_test_warden";

/// The sandbox, with a hostile Shock Warden to the player's right, settled
/// and facing the player.
fn a_warden_beside_you() -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("the sandbox builds");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let (p, ..) = player(&mut sim);
    sim.spawn_enemy_character_at(
        WARDEN,
        "Shock Warden",
        (p.pos.x + 160.0, p.pos.y - 10.0),
        (16.0, 32.0),
        CharacterBrain::Passive,
        "npc_shock_warden",
    );
    for _ in 0..40 {
        sim.step(AgentAction::default());
    }
    sim
}

#[derive(Debug)]
struct Warden {
    kin: ae::BodyKinematics,
    hp: i32,
    guards: Vec<ae::Aabb>,
    state: GuardState,
    bashing: bool,
}

fn warden(sim: &mut Platformer2dSimHarness) -> Warden {
    let world = sim.world_mut();
    let mut q = world.query::<(
        Entity,
        &FeatureId,
        &ae::BodyKinematics,
        &BodyHealth,
        Option<&GuardVolumes>,
        Option<&GuardState>,
        Option<&MovePlayback>,
    )>();
    let (_, _, kin, health, guards, state, playback) =
        q.iter(world).find(|(_, id, ..)| id.as_str() == WARDEN).expect("the warden was built");
    Warden {
        kin: kin.clone(),
        hp: health.current(),
        guards: guards.map_or_else(Vec::new, |guards| guards.pieces.iter().map(|piece| piece.world_volume().unwrap().bounds()).collect()),
        state: state.copied().unwrap_or_default(),
        bashing: playback.is_some_and(|playback| playback.spec.id == "shock_warden_shield_bash"),
    }
}

fn player(sim: &mut Platformer2dSimHarness) -> (ae::BodyKinematics, i32, f32) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(&ae::BodyKinematics, &BodyHealth, &BodyCombat), PrimaryPlayerOnly>();
    let (kin, health, combat) = q.iter(world).next().expect("the player");
    (kin.clone(), health.current(), combat.recoil_lock_timer)
}

fn place_player(sim: &mut Platformer2dSimHarness, at: ae::Vec2) {
    let world = sim.world_mut();
    let mut q = world
        .query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), PrimaryPlayerOnly>();
    let (mut clusters, mut model) = q.iter_mut(world).next().expect("the player");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

/// A strike into his shield from the front does him no harm; it throws the
/// player back and locks it out, and he answers with a shove of the shield
/// that lands.
#[test]
fn a_strike_into_his_shield_is_turned_aside_and_answered() {
    let mut sim = a_warden_beside_you();
    let w = warden(&mut sim);
    let shield = *w.guards.first().unwrap_or_else(|| panic!("his shield is not placed, so nothing here is tested: {w:?}"));
    assert!(w.kin.facing < 0.0, "he faces the player, to his left: {w:?}");
    let warden_hp = w.hp;
    // Stand just in front of his shield, facing it, and swing.
    let (p, hp_before, _) = player(&mut sim);
    let at = ae::Vec2::new(shield.min.x - 22.0, p.pos.y);
    place_player(&mut sim, at);
    sim.step(AgentAction { move_x: 0.2, ..Default::default() });
    sim.step(AgentAction { attack: true, attack_held: true, ..Default::default() });
    let (mut locked, mut bashed, mut hp_after, mut thrown) = (0.0f32, false, hp_before, 0.0f32);
    for frame in 0..80 {
        sim.step(AgentAction::default());
        let (p, hp, lock) = player(&mut sim);
        let w = warden(&mut sim);
        locked = locked.max(lock);
        bashed |= w.bashing;
        if frame < 20 {
            thrown = thrown.max(at.x - p.pos.x);
        }
        if bashed {
            hp_after = hp_after.min(hp);
        }
    }
    let w = warden(&mut sim);
    assert!(w.state.deflects >= 1, "his shield turned nothing aside: {w:?}");
    assert_eq!(w.hp, warden_hp, "the strike hurt him through his shield: {w:?}");
    assert!(locked >= 0.3, "the player was not locked out after the deflect: {locked}");
    assert!(thrown > 12.0, "the player was not thrown back from the shield: {thrown} px");
    assert!(bashed, "he never answered with his shield: {w:?}");
    assert!(hp_after < hp_before, "his shove never landed on the locked-out player ({hp_before} -> {hp_after})");
}

/// From over the top of his shield, a strike down on him gets through.
#[test]
fn a_strike_down_from_over_his_shield_gets_through() {
    let mut sim = a_warden_beside_you();
    let w = warden(&mut sim);
    let shield = *w.guards.first().unwrap_or_else(|| panic!("his shield is not placed: {w:?}"));
    let hp_before = w.hp;
    // Above his head, higher than his shield, falling onto him: a down-air.
    let (p, ..) = player(&mut sim);
    let above = ae::Vec2::new(w.kin.pos.x, shield.min.y - p.size().y * 0.5 - 18.0);
    place_player(&mut sim, above);
    for frame in 0..40 {
        let attack = frame % 8 == 0;
        sim.step(AgentAction { move_y: 1.0, attack, attack_held: attack, ..Default::default() });
    }
    let w = warden(&mut sim);
    assert!(w.hp < hp_before, "a strike down over his shield did not reach him: {w:?}");
    assert_eq!(w.state.deflects, 0, "his shield turned a strike from above: {w:?}");
}

fn wear(sim: &mut Platformer2dSimHarness, character: &str) {
    let world = sim.world_mut();
    let mut q = world.query::<(&FeatureId, &mut ambition_platformer2d::characters::actor::WornCharacter)>();
    let (_, mut worn) = q.iter_mut(world).find(|(id, _)| id.as_str() == WARDEN).expect("the warden was built");
    *worn = ambition_platformer2d::characters::actor::WornCharacter::new(character);
}

/// A BROKEN GUARD IS THE BODY'S, ACROSS A CHANGE OF FORM (review, integration
/// check, 2026-10-19; Q159: a change of form is the same body). His guard is
/// broken; he becomes a form with no guards and back while the stagger runs:
/// his shield stays down until the stagger is over, and then comes up. The
/// guard state is the same component throughout, and a form with no guards
/// places none. Control: before the break his shield is placed.
#[test]
fn a_broken_guard_stays_broken_across_a_change_of_form() {
    let mut sim = a_warden_beside_you();
    let w = warden(&mut sim);
    assert!(!w.guards.is_empty(), "premise: his shield is placed: {w:?}");
    // Premise, unbroken: the form with no guards places none, and his own
    // form places his shield again.
    wear(&mut sim, "npc_kernel_guide");
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    assert!(warden(&mut sim).guards.is_empty(), "premise: the guide's form places no guard");
    wear(&mut sim, "npc_shock_warden");
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    assert!(!warden(&mut sim).guards.is_empty(), "premise: back in his form, his shield is placed");
    {
        let world = sim.world_mut();
        let mut q = world.query::<(&FeatureId, &mut GuardState)>();
        let (_, mut state) = q.iter_mut(world).find(|(id, _)| id.as_str() == WARDEN).expect("his guard state");
        state.broken_s = 1.0;
        state.deflects = 7;
    }
    sim.step(AgentAction::default());
    assert!(warden(&mut sim).guards.is_empty(), "premise: a broken guard is not placed");

    wear(&mut sim, "npc_kernel_guide");
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let w = warden(&mut sim);
    assert!(w.guards.is_empty(), "a form with no guards placed a guard: {w:?}");
    assert!(w.state.broken() && w.state.deflects == 7, "the change of form reset the guard state: {w:?}");

    wear(&mut sim, "npc_shock_warden");
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let w = warden(&mut sim);
    assert!(w.state.broken() && w.guards.is_empty(), "back in his form, the stagger is not over and his shield came up: {w:?}");

    for _ in 0..60 {
        sim.step(AgentAction::default());
    }
    let w = warden(&mut sim);
    assert!(!w.state.broken() && !w.guards.is_empty(), "the stagger is over and his shield did not come up: {w:?}");
    assert_eq!(w.state.deflects, 7, "the count of deflects is the body's: {w:?}");
}

#[test]
fn his_guard_uses_physical_material_for_shape_and_front_coverage() {
    use ambition_platformer2d::combat::guard::DeflectedBodyHit;
    use ambition_platformer2d::combat::strike::{Hitbox, HitboxAnchor, HitboxHits, HitboxKnockback, HitboxLifetime, HitSide};
    let mut sim = a_warden_beside_you();
    let (defender, attacker) = {
        let world = sim.world_mut();
        let defender = world.query::<(Entity, &FeatureId)>().iter(world)
            .find(|(_, id)| id.as_str() == WARDEN).unwrap().0;
        let attacker = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
        (defender, attacker)
    };
    // Keep the authored guard pose fixed during contact.
    sim.world_mut().entity_mut(defender)
        .remove::<ambition_platformer2d::characters::brain::Brain>()
        .insert(ambition_platformer2d::characters::control::ActorControl::default());
    sim.world_mut().entity_mut(defender).insert(ae::PoseOwnedExternally::only(ae::PoseHold::Path));
    sim.world_mut().entity_mut(attacker).insert(ae::PoseOwnedExternally::only(ae::PoseHold::Path));
    for (scale, reflected) in [(0.5, true), (2.0, false)] {
        let axes = Mat2::from_angle(0.6) * if reflected { Mat2::from_cols(-Vec2::X, Vec2::Y) } else { Mat2::IDENTITY };
        {
            let mut body = sim.world_mut().get_mut::<ae::BodyKinematics>(defender).unwrap();
            body.pos = Vec2::new(1200.0, 150.0);
            body.vel = Vec2::ZERO;
            body.material = ae::frame::BodyMaterial::new(scale, axes).unwrap();
        }
        sim.world_mut().entity_mut(defender).insert(GuardState::default());
        sim.rebase_rollback_history().unwrap();
        sim.step(AgentAction::default());
        let body = *sim.world().get::<ae::BodyKinematics>(defender).unwrap();
        let guard = sim.world().get::<GuardVolumes>(defender).unwrap().pieces[0];
        assert_eq!(guard.material.unwrap(), body.material_map().unwrap());
        let shape = guard.world_volume().unwrap();
        let centre = guard.material.unwrap().point((guard.domain.min + guard.domain.max) * 0.5).unwrap();
        let bounds = shape.bounds();
        let corner = ae::CombatVolume::Circle { center: bounds.min + Vec2::splat(0.001), radius: 0.0001 };
        assert!(!shape.intersects(&corner), "the turned shield does not fill its broad-phase box");
        let front = guard.material.unwrap().point((guard.domain.min + guard.domain.max) * 0.5
            + Vec2::X * guard.facing * 100.0).unwrap();
        assert!(guard.covers(front).unwrap());
        let behind = guard.material.unwrap().point((guard.domain.min + guard.domain.max) * 0.5
            - Vec2::X * guard.facing * 100.0).unwrap();
        assert!(!guard.covers(behind).unwrap());
        place_player(&mut sim, front);
        let hp = sim.world().get::<BodyHealth>(defender).unwrap().current();
        let local_offset = sim.world().get::<ae::BodyKinematics>(attacker).unwrap().material_map().unwrap()
            .inverse().unwrap().point(centre).unwrap();
        let hit = sim.world_mut().spawn((Hitbox { owner: attacker, source: HitSide::Player,
            anchor: HitboxAnchor::FollowOwner { local_offset }, half_extent: Vec2::splat(0.01),
            shape: None, facing: 1.0, damage: 1,
            knockback: HitboxKnockback::LaunchSpeed { base: 0.0, growth: None }, launch_dir: None,
            reaction: None, guard: Default::default(), frame_down: Vec2::Y, strike_sfx: None },
            HitboxLifetime { remaining_s: 0.1 }, HitboxHits::default())).id();
        let mut deflects = bevy::ecs::message::MessageCursor::<DeflectedBodyHit>::default();
        sim.rebase_rollback_history().unwrap();
        sim.step(AgentAction::default());
        assert!(deflects.read(sim.world().resource::<Messages<DeflectedBodyHit>>())
            .any(|event| event.defender == defender && event.hitbox == hit), "the real contact path must turn the strike: scale={scale} reflected={reflected} guard={guard:?}, after={:?}",
            sim.world().get::<GuardVolumes>(defender));
        assert_eq!(sim.world().get::<BodyHealth>(defender).unwrap().current(), hp);
        sim.world_mut().despawn(hit);
    }
}
