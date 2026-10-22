//! The Spectral Wolf, played headlessly on the road outside Castle Godwin
//! (Jon, 2026-10-22: it "fades between attacks" and "bursts into wisps"):
//! faint and intangible while it circles you, solid from the windup of its
//! bite to the end of its recovery, and on its death the wisps run the floor
//! out of it (`ambition_combat::fade`, `CharacterDeathTraits::bursts_into`).

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability, WornCharacter};
use ambition_platformer2d::combat::actor_tuning::ContactThreatWithdrawn;
use ambition_platformer2d::combat::components::DamageableVolumes;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::projectiles::{ProjectileOwner, ProjectileVisualId};
use ambition_platformer2d::sprite_sheet::character::PinnedRow;
use bevy::prelude::*;

const ROAD: &str = "evil_gate_forest";
const WOLF: &str = "npc_evil_spectral_wolf";

fn road() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(ROAD);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the road builds headlessly");
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
    }
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    sim
}

fn wolf(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    world
        .query::<(Entity, &WornCharacter)>()
        .iter(world)
        .find(|(_, worn)| worn.id() == WOLF)
        .map(|(entity, _)| entity)
        .expect("a spectral wolf on the road")
}

/// The player walks toward the wolf until it is `near` px from it, then stands.
fn walk_up(sim: &mut Platformer2dSimHarness, wolf: Entity, near: f32) -> AgentAction {
    let world = sim.world_mut();
    let at = world.get::<ae::BodyKinematics>(wolf).expect("a body").pos;
    let mut q = world.query_filtered::<&ae::BodyKinematics, PrimaryPlayerOnly>();
    let me = q.iter(world).next().expect("the player").pos;
    let dx = at.x - me.x;
    AgentAction { move_x: if dx.abs() > near { dx.signum() } else { 0.0 }, ..Default::default() }
}

#[derive(Debug, Default)]
struct Seen {
    faded: usize,
    solid: usize,
    /// A faded tick whose touch was not withdrawn, or whose hurtbox was not empty.
    tangible_while_faint: usize,
    rows: Vec<String>,
    /// Where the wolf and the player were, and what its brain was doing, each second.
    trail: Vec<String>,
}

#[test]
fn it_fades_between_its_bites_and_bursts_into_wisps() {
    let mut sim = road();
    let wolf = wolf(&mut sim);
    // You walk up the road to it, and stand; it comes for you.
    let mut seen = Seen::default();
    let mut solid_at: Option<ae::Vec2> = None;
    for _ in 0..60 * 12 {
        let action = walk_up(&mut sim, wolf, 90.0);
        sim.step(action);
        let world = sim.world_mut();
        let row = world.get::<PinnedRow>(wolf).and_then(|pin| pin.rows.first().cloned()).unwrap_or_default();
        let empty = world.get::<DamageableVolumes>(wolf).is_some_and(|volumes| volumes.intangible());
        let withdrawn = world.get::<ContactThreatWithdrawn>(wolf).is_some_and(|w| w.0);
        if (seen.faded + seen.solid) % 60 == 0 {
            let wolf_at = world.get::<ae::BodyKinematics>(wolf).expect("a body").pos;
            let mut q = world.query_filtered::<&ae::BodyKinematics, PrimaryPlayerOnly>();
            let me = q.iter(world).next().expect("the player").pos;
            let brain = world.get::<ambition_platformer2d::characters::brain::Brain>(wolf).map(|b| format!("{b:?}")).unwrap_or_default();
            let mode = brain.find("mode: ").map(|i| brain[i + 6..].split(',').next().unwrap_or("").to_string()).unwrap_or_default();
            seen.trail.push(format!("wolf {:.0},{:.0} you {:.0},{:.0} {mode}", wolf_at.x, wolf_at.y, me.x, me.y));
        }
        if row == "faded" {
            seen.faded += 1;
            // The first faded tick may still be the end of the swing for a
            // system that ran before the swing ended this tick.
            if seen.rows.last().is_some_and(|last| last == "faded") && !(empty && withdrawn) {
                seen.tangible_while_faint += 1;
            }
        } else {
            seen.solid += 1;
            if !empty && solid_at.is_none() && seen.faded > 0 {
                solid_at = Some(world.get::<ae::BodyKinematics>(wolf).expect("a body").pos);
            }
        }
        if seen.rows.last() != Some(&row) {
            seen.rows.push(row);
        }
    }
    assert!(seen.faded > 60, "it was not faded between its bites: {seen:?}");
    assert_eq!(seen.tangible_while_faint, 0, "it could be hit or hurt while faint: {seen:?}");
    for row in ["faded", "fade_in", "fade_out"] {
        assert!(seen.rows.iter().any(|r| r == row), "it never drew `{row}`: {seen:?}");
    }
    assert!(solid_at.is_some(), "it never came solid to bite: {seen:?}");

    // Killed while it is solid, it bursts into four wisps of its fire.
    let mut at = None;
    for _ in 0..60 * 6 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        if world.get::<DamageableVolumes>(wolf).is_some_and(|volumes| !volumes.intangible()) {
            at = Some(world.get::<ae::BodyKinematics>(wolf).expect("a body").pos);
            break;
        }
    }
    let at = at.expect("it came solid again");
    let volume: ae::CombatVolume = ae::Aabb::new(at, ae::Vec2::new(48.0, 48.0)).into();
    {
        use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
        sim.world_mut().write_message(HitEvent {
            pieces: Vec::new(),
            strike_sfx: None,
            volume,
            damage: 9_999,
            source: HitSource::Projectile,
            attacker: None,
            room: None,
            target: HitTarget::Body(wolf),
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
            attacker_move_instance: None,
        });
    }
    let mut wisps = 0;
    for _ in 0..10 {
        sim.step(AgentAction::default());
        let world = sim.world_mut();
        wisps = wisps.max(
            world
                .query::<(&ProjectileOwner, &ProjectileVisualId)>()
                .iter(world)
                .filter(|(owner, visual)| owner.0 == wolf && visual.0 == "spectral_wolf_wisp")
                .count(),
        );
    }
    assert!(!sim.world_mut().get::<BodyHealth>(wolf).is_some_and(|h| h.alive()), "it did not die");
    assert_eq!(wisps, 4, "it burst into {wisps} wisps");
}
