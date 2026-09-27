//! The Flying Spaghetti Monster's fight, in its shipped arena.
//!
//! The god swims BESIDE you with its noodles hanging to just over your head;
//! the noodles sting, the bell is the target, and the bell is reached from the
//! ledges or when it lies stranded after a dive.

use crate::common::base;
use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_content::bosses::fsm::{FsmConductor, Move};
use ambition_platformer2d::boss_encounter::{BossConfig, BossEncounter};
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::*;

const ARENA: &str = "flying_spaghetti_monster_arena";
/// The bell's half-extent (the profile's `combat_size`).
const BELL_HALF: ae::Vec2 = ae::Vec2::new(95.0, 44.0);

fn arena() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED: the tolerant road falls back to the start room, which has
        // no FSM, and every assertion below would be vacuous.
        .with_required_start_room(ARENA);
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("the FSM arena builds headlessly");
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    sim
}

struct God {
    entity: Entity,
    pos: ae::Vec2,
    performing: Option<(Move, bool)>,
    stranded: bool,
    hp: i32,
    floor: f32,
}

fn god(sim: &mut Platformer2dSimHarness) -> God {
    let world = sim.world_mut();
    let mut q = world.query::<(Entity, &ae::BodyKinematics, &FsmConductor, &BodyHealth)>();
    let (entity, kin, conductor, health) = q.iter(world).next().expect("the FSM, conducted");
    God {
        entity,
        pos: kin.pos,
        performing: conductor.performing(),
        stranded: conductor.stranded(),
        hp: health.current(),
        floor: conductor.hall().map_or(f32::NAN, |hall| hall.floor),
    }
}

fn player(sim: &mut Platformer2dSimHarness) -> (Entity, ae::Vec2, i32) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(Entity, &ae::BodyKinematics, &BodyHealth), PrimaryPlayerOnly>();
    let (entity, kin, health) = q.iter(world).next().expect("the player");
    (entity, kin.pos, health.current())
}

fn place_player(sim: &mut Platformer2dSimHarness, at: ae::Vec2) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel), PrimaryPlayerOnly>();
    let (mut clusters, mut model) = q.iter_mut(world).next().expect("the player");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

fn untouchable_player(sim: &mut Platformer2dSimHarness, untouchable: bool) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut BodyHealth, PrimaryPlayerOnly>();
    for mut health in q.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, untouchable);
    }
}

fn step_until(sim: &mut Platformer2dSimHarness, max: usize, what: &str, mut done: impl FnMut(&mut Platformer2dSimHarness) -> bool) {
    for _ in 0..max {
        if done(sim) {
            return;
        }
        sim.step(base());
    }
    panic!("never saw {what} within {max} frames");
}

/// Past the intro, when the god can be hurt and can hurt.
fn fight_started(sim: &mut Platformer2dSimHarness) {
    step_until(sim, 900, "the fight to start", |sim| {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&BossEncounter, With<FsmConductor>>();
        q.iter(world).next().is_some_and(|e| format!("{:?}", e.encounter_phase()) == "Phase1")
    });
}

/// A player's blow of `half` at `center`: a one-tick player-side volume.
fn strike_at(sim: &mut Platformer2dSimHarness, center: ae::Vec2, half: ae::Vec2) {
    let (owner, ..) = player(sim);
    let world = sim.world_mut();
    let mut commands = world.commands();
    ambition_platformer2d::combat::strike::spawn_damage_box(
        &mut commands,
        owner,
        ambition_platformer2d::combat::hitbox::HitSide::Player,
        center,
        ambition_platformer2d::combat::strike::DamageBox {
            half_extent: half,
            shape: None,
            damage: 1,
            knockback: 0.0,
            lifetime_s: 0.05,
            name: Some("test_blow"),
        },
    );
    world.flush();
}

/// The god takes over its own pose, and swims BESIDE you, not over you: its
/// bell well clear of your head and off to one side.
#[test]
fn the_god_swims_beside_you_with_its_noodles_over_your_head() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    let owned = {
        let entity = god(&mut sim).entity;
        sim.world().get::<ae::PoseOwnedExternally>(entity).is_some()
    };
    assert!(owned, "the conductor never took the god's pose, so the engine's movement profile flies it");
    for _ in 0..150 {
        sim.step(base());
    }
    let g = god(&mut sim);
    let (_, p, _) = player(&mut sim);
    let beside = (g.pos.x - p.x).abs();
    assert!(
        beside > BELL_HALF.x,
        "the bell is over the player (|dx| = {beside:.0}), not beside them"
    );
    let head_room = (p.y - 24.0) - (g.pos.y + BELL_HALF.y);
    assert!(
        head_room > 110.0,
        "the bell's underside is only {head_room:.0} over the player's head; its noodles (~140) would sweep the floor"
    );
}

/// The noodles sting: jump up into the curtain under the bell and it hurts;
/// the same jump beside it does not.
#[test]
fn jumping_into_the_noodles_under_the_bell_stings() {
    let mut sim = arena();
    fight_started(&mut sim);
    let jump_under = |sim: &mut Platformer2dSimHarness, dx: f32| -> i32 {
        let g = god(sim);
        let floor = g.floor;
        place_player(sim, ae::Vec2::new(g.pos.x + dx, floor - 24.0));
        let (_, _, before) = player(sim);
        for f in 0..40 {
            sim.step(AgentAction { jump: f == 0, jump_held: f < 25, ..base() });
        }
        before - player(sim).2
    };
    // Under it: the god glides away to stand off, so jump the tick you land.
    let under = jump_under(&mut sim, 0.0);
    assert!(under > 0, "a jump straight up into the noodles under the bell did not sting");
    // Beside it, well clear of the skirt, the same jump is safe. A fresh arena
    // so the sting's one-hit-per-pass memory is not what spares it.
    let mut sim = arena();
    fight_started(&mut sim);
    let beside = jump_under(&mut sim, 300.0);
    assert_eq!(beside, 0, "a jump well clear of the noodles was stung");
}

/// Its body is the BELL: a blow into the noodles hanging under it does not
/// reach the god; the same blow into the bell does.
#[test]
fn a_blow_to_the_noodles_misses_and_a_blow_to_the_bell_lands() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    fight_started(&mut sim);
    let g = god(&mut sim);
    // Well down the noodles: under the bell's rim, above their tips.
    strike_at(&mut sim, g.pos + ae::Vec2::new(0.0, BELL_HALF.y + 70.0), ae::Vec2::splat(20.0));
    for _ in 0..3 {
        sim.step(base());
    }
    let after_noodles = god(&mut sim).hp;
    assert_eq!(after_noodles, g.hp, "a blow to the noodles hurt the god: its body is the whole drawn frame, not the bell");
    let g = god(&mut sim);
    strike_at(&mut sim, g.pos, ae::Vec2::splat(20.0));
    for _ in 0..3 {
        sim.step(base());
    }
    assert!(god(&mut sim).hp < g.hp, "a blow to the middle of the bell did not land");
}

/// The dive comes down where you stood, and leaves the god lying on the
/// floor — its bell in reach from the ground — until its next move lifts it.
#[test]
fn the_dive_lands_on_you_and_leaves_it_stranded_in_reach() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 1800, "a dive's strike", |sim| god(sim).performing == Some((Move::Dive, true)));
    let (_, stood, _) = player(&mut sim);
    step_until(&mut sim, 120, "the landing", |sim| god(sim).stranded);
    let g = god(&mut sim);
    assert!(
        (g.pos.y + BELL_HALF.y) > g.floor - 30.0,
        "stranded, the bell's underside is at {:.0} with the floor at {:.0}: out of reach from the ground",
        g.pos.y + BELL_HALF.y,
        g.floor
    );
    assert!(
        (g.pos.x - stood.x).abs() < BELL_HALF.x + 30.0,
        "the dive landed at x {:.0}, not on the player at {:.0}",
        g.pos.x,
        stood.x
    );
    // It stays down long enough to punish: well over a second.
    for _ in 0..80 {
        sim.step(base());
        assert!(god(&mut sim).stranded, "the god rose within 80 ticks of landing: no punish window");
    }
    // A blow from the ground lands.
    let g = god(&mut sim);
    strike_at(&mut sim, g.pos + ae::Vec2::new(0.0, BELL_HALF.y * 0.5), ae::Vec2::splat(18.0));
    for _ in 0..3 {
        sim.step(base());
    }
    assert!(god(&mut sim).hp < g.hp, "a blow to the stranded bell from the ground did not land");
    // And its next move lifts it.
    step_until(&mut sim, 600, "the god to rise", |sim| !god(sim).stranded);
    for _ in 0..60 {
        sim.step(base());
    }
    let g = god(&mut sim);
    assert!(g.pos.y < g.floor - 150.0, "the god never rose off the floor after its dive");
}

/// The volley throws MEATBALLS: its shots wear the meatball.
#[test]
fn the_volley_throws_meatballs() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 900, "a volley's strike", |sim| god(sim).performing == Some((Move::Volley, true)));
    sim.step(base());
    let world = sim.world_mut();
    let mut q = world.query::<&ambition_platformer2d::projectiles::ProjectileVisualId>();
    let meatballs = q.iter(world).filter(|id| id.0 == "meatball").count();
    assert_eq!(meatballs, 3, "Phase 1's volley throws three meatballs");
}

/// Caught by the grasp, you are dragged TOWARD the god and up into its noodles.
#[test]
fn the_grasp_drags_you_toward_the_god() {
    let mut sim = arena();
    step_until(&mut sim, 1200, "a grasp's strike", |sim| god(sim).performing == Some((Move::Grasp, true)));
    let (_, before, _) = player(&mut sim);
    let g = god(&mut sim);
    let mut caught = None;
    for _ in 0..40 {
        sim.step(base());
        let (_, now, _) = player(&mut sim);
        if now.y < before.y - 20.0 {
            caught = Some(now);
            break;
        }
    }
    let now = caught.expect("the grasp never lifted the player: it missed, or it throws rather than drags");
    let toward = (now.x - before.x) * (g.pos.x - before.x).signum();
    assert!(toward > 0.0, "the grasp moved the player AWAY from the god (from {:.0} to {:.0}, god at {:.0})", before.x, now.x, g.pos.x);
}

/// Wounded, it sends noodlings.
#[test]
fn wounded_it_sends_noodlings() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    fight_started(&mut sim);
    {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<&mut BodyHealth, (With<FsmConductor>, With<BossConfig>)>();
        let mut health = q.iter_mut(world).next().expect("the god");
        health.health.current = (health.max() as f32 * 0.5).ceil() as i32;
    }
    step_until(&mut sim, 1200, "the lesser appendages", |sim| god(sim).performing == Some((Move::Appendages, true)));
    for _ in 0..3 {
        sim.step(base());
    }
    let noodlings = |sim: &mut Platformer2dSimHarness| -> Vec<i32> {
        let world = sim.world_mut();
        let mut q = world.query::<(&ambition_platformer2d::characters::actor::WornCharacter, &BodyHealth)>();
        q.iter(world).filter(|(worn, _)| worn.id() == "npc_fsm_noodling").map(|(_, h)| h.damage_taken()).collect()
    };
    assert_eq!(noodlings(&mut sim).len(), 2, "the lesser appendages did not send a pair of noodlings");
    // They are born inside the god's sting; its volumes must pass through its
    // own appendages.
    for _ in 0..120 {
        sim.step(base());
    }
    let taken = noodlings(&mut sim);
    assert!(
        taken.iter().all(|t| *t == 0),
        "the god's own volumes hurt its noodlings (damage taken {taken:?})"
    );
}



