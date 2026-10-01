//! The Flying Spaghetti Monster's fight, in its shipped arena.
//!
//! The god swims BESIDE you with its noodles hanging to just over your head;
//! the noodles sting, the bell is the target, and the bell is reached from the
//! ledges or when it lies stranded after a dive.

use crate::common::base;
use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use ambition_content::bosses::fsm::{conductor_of, Move, FSM_ID};
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

/// The god: the boss whose behaviour is the FSM's.
fn the_god(world: &mut World) -> Option<Entity> {
    let mut q = world.query::<(Entity, &BossConfig)>();
    q.iter(world).find(|(_, config)| config.behavior.id == FSM_ID).map(|(e, _)| e)
}

fn god(sim: &mut Platformer2dSimHarness) -> God {
    let world = sim.world_mut();
    let entity = the_god(world).expect("the FSM");
    let kin = *world.get::<ae::BodyKinematics>(entity).unwrap();
    let hp = world.get::<BodyHealth>(entity).unwrap().current();
    // Its conductor is the `fsm` module; its memory is the module's record.
    let view = conductor_of(world, entity).expect("the FSM, conducted");
    God {
        entity,
        pos: kin.pos,
        performing: view.performing,
        stranded: view.stranded,
        hp,
        floor: view.hall.map_or(f32::NAN, |hall| hall.floor),
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
        the_god(world)
            .and_then(|god| world.get::<BossEncounter>(god))
            .is_some_and(|e| format!("{:?}", e.encounter_phase()) == "Phase1")
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

/// Each landing shock and where it stands: (entity, x of its centre).
/// The landing's shocks: the god's volumes placed in the world (every other
/// volume of the god rides it). Its conductor is a module, and a module's
/// held box carries no inspector name.
fn shocks(sim: &mut Platformer2dSimHarness) -> Vec<(Entity, f32)> {
    use ambition_platformer2d::combat::strike::{Hitbox, HitboxAnchor};
    let world = sim.world_mut();
    let god = the_god(world);
    let mut q = world.query::<(Entity, &Hitbox)>();
    let mut shocks: Vec<_> = q
        .iter(world)
        .filter(|(_, hitbox)| Some(hitbox.owner) == god)
        .filter_map(|(entity, hitbox)| match hitbox.anchor {
            HitboxAnchor::World { center } => Some((entity, center.x)),
            HitboxAnchor::FollowOwner { .. } => None,
        })
        .collect();
    shocks.sort_by(|a, b| a.1.total_cmp(&b.1));
    shocks
}

/// The dive lands with a shock each way along the floor. The shocks live past
/// the landing tick, roll outward, and end with the god.
#[test]
fn the_landing_shocks_roll_outward_and_end_with_the_god() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    // The dive lands where the player stands. Stand mid-hall, so that neither
    // shock starts at a wall, where it ends at once by design.
    let hall = hall_of(&mut sim);
    let mid = ae::Vec2::new(hall.center_x(), player(&mut sim).1.y);
    step_until(&mut sim, 1800, "a dive's strike", |sim| {
        place_player(sim, mid);
        god(sim).performing == Some((Move::Dive, true))
    });
    step_until(&mut sim, 120, "the landing", |sim| god(sim).stranded);
    let landed = shocks(&mut sim);
    assert_eq!(landed.len(), 2, "the landing did not leave a shock each way: {landed:?}");
    sim.step(base());
    let rolled = shocks(&mut sim);
    assert_eq!(
        rolled.iter().map(|shock| shock.0).collect::<Vec<_>>(),
        landed.iter().map(|shock| shock.0).collect::<Vec<_>>(),
        "the landing's shocks did not live to the next tick: landed at {landed:?} \
         in a hall from {} to {}",
        hall.left,
        hall.right
    );
    assert!(
        rolled[0].1 < landed[0].1 && rolled[1].1 > landed[1].1,
        "the shocks did not roll outward: {landed:?} then {rolled:?}"
    );
    let g = god(&mut sim);
    sim.world_mut().get_mut::<BodyHealth>(g.entity).expect("the god").health.current = 0;
    sim.step(base());
    assert_eq!(shocks(&mut sim), [], "the shocks outlived the god");
}

fn possessed(sim: &mut Platformer2dSimHarness) -> Option<Entity> {
    sim.world_mut()
        .resource::<ambition_platformer2d::actors::control::possession::PossessionState>()
        .possessed
}

/// Hold Down + Interact: the possession gesture, and a fresh press releases.
fn down_interact(edge: bool) -> AgentAction {
    AgentAction { move_y: 1.0, interact: edge, interact_held: true, ..base() }
}

fn hall_of(sim: &mut Platformer2dSimHarness) -> ambition_content::bosses::hall::Hall {
    let world = sim.world_mut();
    let god = the_god(world).expect("the FSM");
    conductor_of(world, god).and_then(|view| view.hall).expect("the god has measured its hall")
}

/// Put any body at `at`, through the engine's transit.
fn place_body(sim: &mut Platformer2dSimHarness, body: Entity, at: ae::Vec2) {
    let world = sim.world_mut();
    let mut q = world.query::<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel)>();
    let (mut clusters, mut model) = q.get_mut(world, body).expect("the body");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, at, ae::movement::TransitVelocity::Zero);
}

/// A participant who drives the god steers it, both ways; released, the
/// conductor flies it again.
#[test]
fn a_driven_god_goes_where_it_is_steered_and_swims_again_when_released() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    let the_god = god(&mut sim).entity;
    for i in 0..300 {
        let at = god(&mut sim).pos;
        sim.teleport_player((at.x - 40.0, at.y));
        sim.step(down_interact(i == 0));
        if possessed(&mut sim) == Some(the_god) {
            break;
        }
    }
    assert_eq!(possessed(&mut sim), Some(the_god), "setup: the Down+Interact hold did not possess the god");
    // Let any move the pattern had started run out, and any rise after a dive.
    step_until(&mut sim, 900, "no move to hold the god", |sim| {
        let g = god(sim);
        g.performing.is_none() && !g.stranded
    });
    for _ in 0..60 {
        sim.step(base());
    }
    let mut steer = |move_x: f32| {
        let from = god(&mut sim).pos.x;
        for _ in 0..40 {
            sim.step(AgentAction { move_x, ..base() });
        }
        god(&mut sim).pos.x - from
    };
    let right = steer(1.0);
    let left = steer(-1.0);
    assert!(
        right > 20.0 && left < -20.0,
        "the driven god did not follow its participant: held right it moved {right:.0}, held left {left:.0}"
    );

    sim.step(down_interact(true));
    assert_eq!(possessed(&mut sim), None, "a fresh press releases the god");
    let from = god(&mut sim).pos;
    for _ in 0..90 {
        sim.step(base());
    }
    let g = god(&mut sim);
    assert!(
        sim.world().get::<ae::PoseOwnedExternally>(g.entity).is_some(),
        "released, the conductor did not take the god's pose back"
    );
    assert!(g.pos.distance(from) > 10.0, "released, the god did not swim: it stayed at {from:?}");
}

/// The god aims at the foe the engine chose for it (its `ActorTarget`), not at
/// the home avatar. A participant drives another body on one side of the god;
/// the home avatar stands at the far wall on the other side.
#[test]
fn the_god_aims_at_the_foe_its_target_names_not_the_home_avatar() {
    use ambition_platformer2d::combat::components::{ActorTarget, FeatureId};
    const DRIVEN: &str = "fsm_driven_foe";

    let mut sim = arena();
    untouchable_player(&mut sim, true);
    let hall = hall_of(&mut sim);
    let (home, at, _) = player(&mut sim);
    sim.spawn_enemy_character_at(
        DRIVEN,
        "Driven Foe",
        (at.x + 60.0, at.y),
        (14.0, 23.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
        "perfect_cellular_automaton",
    );
    let driven = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &FeatureId)>();
        q.iter(world).find(|(_, id)| id.as_str() == DRIVEN).map(|(entity, _)| entity).expect("the spawned body")
    };
    sim.world_mut()
        .get_mut::<BodyHealth>(driven)
        .expect("the driven body has health")
        .health
        .invulnerable
        .set(Invulnerability::SCRIPTED, true);
    for i in 0..900 {
        sim.step(down_interact(i == 0));
        if possessed(&mut sim) == Some(driven) {
            break;
        }
    }
    assert_eq!(possessed(&mut sim), Some(driven), "setup: the Down+Interact hold did not possess the body");

    // The home avatar leaves the playable plane, so it is no foe the engine
    // can select, whatever the god does; the driven body is the only one. The
    // home avatar stands at the far wall and the driven body close on the other
    // side of the god, both held in place.
    sim.world_mut().entity_mut(home).insert(ae::DepthPlane::BEHIND);
    let g = god(&mut sim);
    let side = if g.pos.x < hall.center_x() { 1.0 } else { -1.0 };
    let home_at = ae::Vec2::new(if side > 0.0 { hall.left + 40.0 } else { hall.right - 40.0 }, at.y);
    let driven_at = ae::Vec2::new((g.pos.x + side * 150.0).clamp(hall.left + 40.0, hall.right - 40.0), at.y);
    let hold = |sim: &mut Platformer2dSimHarness| {
        place_body(sim, home, home_at);
        place_body(sim, driven, driven_at);
    };
    for _ in 0..20 {
        hold(&mut sim);
        sim.step(base());
    }
    // The volley lobs its meatballs at the god's target: where they fly is
    // the conductor's own aim.
    step_until(&mut sim, 1800, "a volley's strike", |sim| {
        hold(sim);
        god(sim).performing == Some((Move::Volley, true))
    });
    hold(&mut sim);
    sim.step(base());
    let g = god(&mut sim);
    let target = sim.world().get::<ActorTarget>(g.entity).expect("the god has a target").entity;
    assert_eq!(target, Some(driven), "premise: the engine chose the driven body as the god's foe");
    assert!(
        (driven_at.x - g.pos.x).signum() != (home_at.x - g.pos.x).signum(),
        "premise: the god at x {:.0} is between the home avatar at {:.0} and its foe at {:.0}",
        g.pos.x,
        home_at.x,
        driven_at.x
    );
    let thrown: Vec<f32> = {
        let world = sim.world_mut();
        let mut q = world.query::<(&ambition_platformer2d::projectiles::ProjectileVisualId, &ae::BodyKinematics)>();
        q.iter(world).filter(|(id, _)| id.0 == "meatball").map(|(_, kin)| kin.vel.x).collect()
    };
    assert!(!thrown.is_empty(), "premise: the volley threw meatballs");
    assert!(
        thrown.iter().all(|vx| vx.signum() == (driven_at.x - g.pos.x).signum()),
        "the god at x {:.0} threw at the home avatar at {:.0}, not its foe at {:.0}: meatball x speeds {thrown:?}",
        g.pos.x,
        home_at.x,
        driven_at.x
    );
}

/// A move keeps the side it chose when its tell began: the god is drawn facing
/// where its move goes, even when you cross under it.
#[test]
fn a_move_keeps_the_side_its_tell_chose_when_you_cross_under() {
    let mut sim = arena();
    untouchable_player(&mut sim, true);
    step_until(&mut sim, 1800, "a lash's tell", |sim| god(sim).performing == Some((Move::Lash, false)));
    let g = god(&mut sim);
    let chosen = sim.world().get::<ae::BodyKinematics>(g.entity).expect("the god").facing.signum();
    let (_, stood, _) = player(&mut sim);
    let behind = ae::Vec2::new(g.pos.x - chosen * 220.0, stood.y);
    for _ in 0..8 {
        place_player(&mut sim, behind);
        sim.step(base());
        if god(&mut sim).performing.map(|(mv, _)| mv) != Some(Move::Lash) {
            break;
        }
        let facing = sim.world().get::<ae::BodyKinematics>(g.entity).expect("the god").facing.signum();
        assert_eq!(facing, chosen, "the god turned round mid-lash when the player crossed behind it");
    }
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
        let god = the_god(world).expect("the god");
        let mut health = world.get_mut::<BodyHealth>(god).expect("the god");
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

/// The god is built with what its conductor needs on every road that builds
/// it: the arena's placement, and a boss staged by code — the side it faces
/// (`ConductedFacing`, which its conductor module chooses from then on) and
/// its drawn row. No tick sees it without them, because the side decides
/// where its first move goes.
#[test]
fn the_god_is_built_with_its_conductor_on_every_road() {
    fn conducted(sim: &mut Platformer2dSimHarness) -> Option<(bool, bool)> {
        let world = sim.world_mut();
        world
            .query::<(
                &BossConfig,
                Has<ambition_platformer2d::boss_encounter::conduct::ConductedFacing>,
                Has<ambition_platformer2d::sprite_sheet::character::PinnedRow>,
            )>()
            .iter(world)
            .find(|(config, ..)| config.behavior.id == FSM_ID)
            .map(|(_, facing, row)| (facing, row))
    }
    let mut placed = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ARENA),
    )
    .expect("the FSM arena builds headlessly");
    let first = (0..300)
        .find_map(|_| conducted(&mut placed).or_else(|| {
            placed.step(AgentAction::default());
            None
        }))
        .expect("the arena builds its god");
    assert_eq!(first, (true, true), "the arena's god, on the first tick it exists");

    let mut staged = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("the sandbox builds");
    assert_eq!(conducted(&mut staged), None, "the premise: no god before it is staged");
    staged.spawn_boss_at(
        "staged_fsm",
        "Flying Spaghetti Monster",
        (400.0, 200.0),
        (60.0, 60.0),
        ambition_platformer2d::entity_catalog::placements::BossBrain::PhaseScript {
            script_id: FSM_ID.to_string(),
        },
    );
    assert_eq!(conducted(&mut staged), Some((true, true)), "the staged god, on the frame that builds it");
}
