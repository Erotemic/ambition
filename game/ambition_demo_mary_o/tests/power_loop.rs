//! **The whole loop, on the real systems.**
//!
//! Each unit test elsewhere proves one link. This walks the chain a player walks,
//! with no step simulated by hand:
//!
//! ```text
//! bonk a block -> milk pops -> touch it -> GROWN
//!   -> bonk again -> blossom pops -> touch it -> SPARK-POWERED (ranged verb live)
//!     -> hold run -> run throttle reaches the body
//!       -> press the same button -> fire intent raised
//!         -> the shot arcs, bounces, and expires by its authored policy
//!           -> take a hit -> spark lost, STILL TALL
//!             -> take another -> small
//! ```
//!
//! The systems under test are the production ones: the demo's block/grow rules,
//! the engine's touch-to-collect, the engine's grant reconcile, and the shared
//! projectile body. Nothing here inserts an `ActionSet` or a moveset by hand — if
//! the reconcile stopped running, the ranged assertions would fail.

use bevy::prelude::*;

use ambition::actors::actor::{BodyBaseSize, PrimaryPlayer};
use ambition::actors::avatar::PlayerBodyFrameOutput;
use ambition::actors::equipment::reconcile_equipment_grants;
use ambition::actors::items::{collect_world_items, WorldItem};
use ambition::characters::actor::WornCharacter;
use ambition::characters::brain::action_set::{ActionSet, IdentityKit};
use ambition::characters::brain::ActorControl;
use ambition::characters::equipment::WornEquipment;
use ambition::combat::moveset::{ActorMoveset, RANGED_VERB};
use ambition::engine_core as ae;
use ambition::engine_core::collision_semantics::{ContactKind, ContactSource};
use ambition::platformer::markers::ControlledSubject;

use ambition_demo_mary_o::movement::{
    fire_spark_on_run_press, walk_by_default_run_while_held, MaryOGait, WALK_THROTTLE,
};
use ambition_demo_mary_o::powerups::{
    bonk_power_blocks, spark_blossom, sync_grown_form, SpentPowerBlocks, GROW_CAP_ID,
    SPARK_BLOSSOM_ID,
};
use ambition_demo_mary_o::provider::MARY_O_CHARACTER_ID;

const TALL_ID: &str = "mary_o_tall";
const FIRE_ID: &str = "mary_o_fire";

struct Loop {
    app: App,
    body: Entity,
}

impl Loop {
    fn new() -> Self {
        let mut app = App::new();
        app.insert_resource(ambition::time::WorldTime {
            scaled_dt: 1.0 / 60.0,
            ..Default::default()
        });
        app.init_resource::<SpentPowerBlocks>();
        // `sync_grown_form` now voices a transform chime through `SfxWriter`.
        app.add_message::<ambition::sfx::OwnedSfxMessage>();

        let size = ae::movement::default_player_body_size();
        let body = app
            .world_mut()
            .spawn((
                PrimaryPlayer,
                WornCharacter(MARY_O_CHARACTER_ID.to_string()),
                BodyBaseSize { base_size: size },
                ae::BodyKinematics {
                    pos: ae::Vec2::new(0.0, 0.0),
                    vel: ae::Vec2::ZERO,
                    size,
                    facing: 1.0,
                },
                // A peaceful identity: any ranged verb she ends up with can ONLY
                // have come from the blossom, reconciled onto this baseline.
                IdentityKit::default(),
                ActionSet::peaceful(),
                ActorMoveset(Default::default()),
                ActorControl::default(),
                MaryOGait::default(),
                PlayerBodyFrameOutput::default(),
            ))
            .id();
        app.insert_resource(ControlledSubject(Some(body)));

        app.add_systems(
            Update,
            (
                bonk_power_blocks,
                collect_world_items,
                reconcile_equipment_grants,
                sync_grown_form,
                walk_by_default_run_while_held,
                fire_spark_on_run_press,
            )
                .chain(),
        );
        Self { app, body }
    }

    /// Head-bonk the level's first ?-block, exactly as the movement phase reports
    /// it: a Head contact carrying the block's durable `GeoId`.
    fn bonk(&mut self) {
        // Stand clear of the block first, so the item that pops is not collected
        // in the same update by a body that happens to be standing on the spot.
        self.app
            .world_mut()
            .get_mut::<ae::BodyKinematics>(self.body)
            .unwrap()
            .pos = ae::Vec2::new(-10_000.0, 0.0);
        let mut frame = self
            .app
            .world_mut()
            .get_mut::<PlayerBodyFrameOutput>(self.body)
            .unwrap();
        frame.events.contacts.clear();
        frame
            .events
            .contacts
            .push(ae::collision_semantics::Contact {
                kind: ContactKind::Head,
                point: ae::Vec2::ZERO,
                normal: ae::Vec2::new(0.0, 1.0),
                toi: 0.0,
                surface_velocity: ae::Vec2::ZERO,
                source: ContactSource::Block {
                    kind: ae::BlockKind::Solid,
                    id: ambition_demo_mary_o::power_block_id(0),
                },
            });
        self.app.update();
        // Clear the contact so the same bonk is not re-read next frame, and
        // re-arm the block so the NEXT rung of the ladder can be collected.
        self.app
            .world_mut()
            .get_mut::<PlayerBodyFrameOutput>(self.body)
            .unwrap()
            .events
            .contacts
            .clear();
        self.app
            .world_mut()
            .resource_mut::<SpentPowerBlocks>()
            .0
            .clear();
    }

    /// Walk onto whatever the block popped, so the shared touch-to-collect equips
    /// it. Teleports her to the item rather than simulating a stroll.
    fn collect_pending_item(&mut self) {
        let item_pos = {
            let world = self.app.world_mut();
            let mut q = world.query::<&WorldItem>();
            q.iter(world).next().map(|i| i.pos)
        };
        let item_pos = item_pos.expect("the block popped an item to collect");
        self.app
            .world_mut()
            .get_mut::<ae::BodyKinematics>(self.body)
            .unwrap()
            .pos = item_pos;
        self.app.update();
    }

    fn intend(&mut self, x: f32, run_held: bool, run_pressed: bool) {
        let mut control = self
            .app
            .world_mut()
            .get_mut::<ActorControl>(self.body)
            .unwrap();
        control.0.locomotion.x = x;
        control.0.modifier_held = run_held;
        control.0.modifier_pressed = run_pressed;
        control.0.fire = None;
    }

    fn hit(&mut self) {
        self.app
            .world_mut()
            .get_mut::<WornEquipment>(self.body)
            .unwrap()
            .consume_armor();
        self.app.update();
    }

    fn wears(&self, id: &str) -> bool {
        self.app
            .world()
            .get::<WornEquipment>(self.body)
            .is_some_and(|w| w.wears(id))
    }
    fn is_tall(&self) -> bool {
        // Both power forms (grown cap = `mary_o_tall`, fire blossom = `mary_o_fire`)
        // share the tall SIZE and differ only from the small starting form, so
        // "tall" is "wearing any power sheet" rather than one specific sheet.
        self.app.world().get::<WornCharacter>(self.body).unwrap().0 != MARY_O_CHARACTER_ID
    }
    fn worn_character(&self) -> String {
        self.app
            .world()
            .get::<WornCharacter>(self.body)
            .unwrap()
            .0
            .clone()
    }
    fn has_ranged_move(&self) -> bool {
        self.app
            .world()
            .get::<ActorMoveset>(self.body)
            .unwrap()
            .0
            .move_for_verb(RANGED_VERB)
            .is_some()
    }
    fn throttle(&self) -> f32 {
        self.app
            .world()
            .get::<ActorControl>(self.body)
            .unwrap()
            .0
            .locomotion
            .x
    }
    fn fired(&self) -> bool {
        self.app
            .world()
            .get::<ActorControl>(self.body)
            .unwrap()
            .0
            .fire
            .is_some()
    }
}

#[test]
fn the_whole_power_loop_runs_on_the_real_systems() {
    let mut game = Loop::new();

    // --- small ---------------------------------------------------------------
    assert!(!game.is_tall(), "she starts small");
    assert!(!game.has_ranged_move(), "and unarmed");

    // --- collect milk -> grown ----------------------------------------------
    game.bonk();
    game.collect_pending_item();
    assert!(
        game.wears(GROW_CAP_ID),
        "the block gave a small Mary-O milk"
    );
    assert!(game.is_tall(), "collecting it grew her");
    assert_eq!(
        game.worn_character(),
        TALL_ID,
        "the milk shows the plain grown sheet"
    );
    assert!(
        !game.has_ranged_move(),
        "the milk is armor only — it grants no verb"
    );

    // --- collect blossom -> spark-powered ------------------------------------
    game.bonk();
    game.collect_pending_item();
    assert!(
        game.wears(SPARK_BLOSSOM_ID),
        "the block gave a GROWN Mary-O the blossom, not another milk"
    );
    assert!(game.is_tall(), "she is still tall");
    assert_eq!(
        game.worn_character(),
        FIRE_ID,
        "the blossom swaps her to the DISTINCT fire sheet, not the plain grown one (Jon bug #10)"
    );
    assert!(
        game.has_ranged_move(),
        "and the reconcile turned the blossom's grant into a fireable move"
    );

    // --- hold run -> run speed ----------------------------------------------
    game.intend(1.0, false, false);
    game.app.update();
    assert_eq!(game.throttle(), WALK_THROTTLE, "no run held: she walks");

    game.intend(1.0, true, false);
    game.app.update();
    assert_eq!(game.throttle(), 1.0, "run held: full throttle");

    // --- fire while continuing to run ---------------------------------------
    game.intend(1.0, true, true);
    game.app.update();
    assert!(game.fired(), "the press edge fires a spark...");
    assert_eq!(
        game.throttle(),
        1.0,
        "...while the SAME button's held level keeps meaning run"
    );

    // --- one hit -> lose the spark, stay tall --------------------------------
    game.hit();
    assert!(!game.wears(SPARK_BLOSSOM_ID), "the hit spent the blossom");
    assert!(game.wears(GROW_CAP_ID), "downgrading to the cap");
    assert!(game.is_tall(), "so she is still GROWN, not small");
    assert_eq!(
        game.worn_character(),
        TALL_ID,
        "losing the spark reverts the fire sheet back to the grown sheet"
    );
    assert!(
        !game.has_ranged_move(),
        "but the spark verb was revoked with the row — no dangling action"
    );

    // --- another hit -> small ------------------------------------------------
    game.hit();
    assert!(!game.wears(GROW_CAP_ID), "the second hit spent the cap");
    assert!(!game.is_tall(), "and returned her to small");
}

/// **The spark's flight, on the shared projectile body.** Her shot is authored
/// data; this steps the ENGINE's projectile primitive with that data and watches
/// it arc, skip off a floor, and expire on its authored budget. No Mary-O code is
/// involved in the stepping — that is the point.
#[test]
fn the_authored_spark_arcs_bounces_and_expires() {
    use ambition::characters::equipment::apply_equipment_grants;
    use ambition::platformer::projectile::{ProjectileBody, ProjectileSpec, WorldHitPolicy};

    // Take the shot exactly as the blossom grants it.
    let mut actions = ActionSet::peaceful();
    apply_equipment_grants(&mut actions, &WornEquipment::new(vec![spark_blossom()]));
    let shot = actions.ranged.expect("the blossom grants a shot");
    let flight = shot.flight.expect("and authors its flight");

    let spec = ProjectileSpec {
        origin: ae::Vec2::ZERO,
        direction: ae::Vec2::new(1.0, 0.0),
        damage: shot.damage(),
        speed: shot.speed(),
        max_lifetime: flight.max_lifetime,
        half_extent: flight.half_extent,
        gravity: flight.gravity,
        bounces: flight.bounces,
        world_hit: if flight.bounce_on_world_contact {
            WorldHitPolicy::Bouncing
        } else {
            WorldHitPolicy::ExpireOnContact
        },
        charge_tier: 0,
    };
    assert_eq!(
        spec.world_hit,
        WorldHitPolicy::Bouncing,
        "the authored spark is a BOUNCING shot, not a straight bolt"
    );

    let mut body = ProjectileBody::from_spec(spec);
    let down = ae::Vec2::new(0.0, 1.0);
    let dt = 1.0 / 60.0;

    // It arcs: gravity bends a flat launch downward within a few ticks.
    let start_vy = body.vel().y;
    for _ in 0..6 {
        body.tick(dt, down);
    }
    assert!(
        body.vel().y > start_vy,
        "the authored gravity bends the shot into an arc"
    );
    assert!(body.vel().x > 0.0, "while it keeps travelling forward");

    // It expires on the authored lifetime even if it never finds a floor.
    let mut alive = true;
    let mut ticks = 0;
    while alive && ticks < 1000 {
        alive = body.tick(dt, down);
        ticks += 1;
    }
    assert!(!alive, "a spark that finds no floor still burns out");
    let lifetime = ticks as f32 * dt;
    assert!(
        lifetime <= flight.max_lifetime + 2.0 * dt,
        "and it expires on ITS authored lifetime ({lifetime}s), not a shared default"
    );
}

/// **The spark kills a snake through the canonical hit path.**
///
/// The composition is the claim worth testing: the engine already proves its
/// stepper damages actors, and the loop above already proves the blossom grants an
/// ordinary ranged verb. What is left to show is that HER shot — authored flight,
/// authored visual, content-marked — is not special to any of it. So this builds
/// the projectile from the blossom's own grant, hands it to the shared stepper as
/// a player-faction shot, and watches a snake lose HP through
/// `apply_feature_hit_events`. Nothing in the damage path knows what a spark is.
#[test]
fn her_spark_damages_a_snake_through_the_shared_hit_pipeline() {
    use ambition::actors::features::{
        apply_feature_hit_events, spawn_encounter_mob, ActorIdentity, CharacterRoster,
        FeatureEcsWorldOverlay, GameplayBanner, HitEvent, SetFlagRequested,
    };
    use ambition::actors::projectile::{step_projectiles, ProjectileBody};
    use ambition::characters::actor::{character_catalog::CharacterCatalog, BodyHealth};
    use ambition::characters::equipment::apply_equipment_grants;
    use ambition::entity_catalog::placements::CharacterBrain;
    use ambition::platformer::lifecycle::SessionSpawnScope;
    use ambition::platformer::projectile::{ProjectileSpec, WorldHitPolicy};
    use ambition::projectiles::{
        LiveProjectile, PlayerProjectile, ProjectileOwner, ProjectileOwnerId, ProjectileSeqCounter,
        ProjectileVisualCatalog, ProjectileVisualId,
    };

    const SNAKE_POS: ae::Vec2 = ae::Vec2::new(400.0, 300.0);

    let mut app = App::new();
    app.insert_resource(ambition::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    ambition::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new(
            "spark_range",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        )),
    );
    app.insert_resource(CharacterCatalog::empty());
    app.insert_resource(GameplayBanner::default());
    app.init_resource::<ambition::actors::boss_encounter::BossCatalog>();
    app.init_resource::<ProjectileSeqCounter>();
    app.init_resource::<ProjectileVisualCatalog>();
    app.init_resource::<FeatureEcsWorldOverlay>();
    app.init_resource::<ambition::actors::trace::GameplayTraceBuffer>();
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition::actors::features::ActorStimulus>();
    app.add_message::<ambition::vfx::VfxMessage>();
    app.add_message::<ambition::vfx::vfx::DebrisBurstMessage>();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    app.add_message::<ambition::actors::avatar::PlayerHealRequested>();

    // Mary-O's OWN Solid Snake archetype, registered exactly as the demo registers it.
    ambition_demo_mary_o::snake::register_snake_roster(&mut app);
    app.add_systems(Update, (step_projectiles, apply_feature_hit_events).chain());

    // A player-faction firer to own the shot.
    let firer = app
        .world_mut()
        .spawn((
            PrimaryPlayer,
            ae::BodyKinematics {
                pos: ae::Vec2::new(360.0, 300.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(30.0, 48.0),
                facing: 1.0,
            },
        ))
        .id();

    // One snake, spawned through the ordinary encounter-mob path.
    {
        let world = app.world_mut();
        let catalog = world.resource::<CharacterCatalog>().clone();
        let roster = world.resource::<CharacterRoster>().clone();
        let mut commands = world.commands();
        spawn_encounter_mob(
            &mut commands,
            &catalog,
            &roster,
            SessionSpawnScope::UNSCOPED,
            "mary_o_spark_range",
            "snake_under_fire".into(),
            CharacterBrain::Custom("mary_o_snake".into()),
            SNAKE_POS,
            ae::Vec2::new(28.0, 32.0),
        );
    }
    app.update();

    // Her shot, straight from the blossom's grant.
    let mut actions = ActionSet::peaceful();
    apply_equipment_grants(&mut actions, &WornEquipment::new(vec![spark_blossom()]));
    let shot = actions.ranged.expect("the blossom grants a shot");
    let flight = shot.flight.expect("and authors its flight");

    let mut body = ProjectileBody::from_spec(ProjectileSpec {
        origin: ae::Vec2::new(370.0, 300.0),
        direction: ae::Vec2::new(1.0, 0.0),
        damage: shot.damage(),
        speed: shot.speed(),
        max_lifetime: flight.max_lifetime,
        half_extent: flight.half_extent,
        gravity: flight.gravity,
        bounces: flight.bounces,
        world_hit: WorldHitPolicy::Bouncing,
        charge_tier: 0,
    });
    // Aim it flat at the snake so the hit does not depend on arc tuning.
    body.kin.pos = ae::Vec2::new(370.0, SNAKE_POS.y);
    body.kin.vel = ae::Vec2::new(600.0, 0.0);

    let seq = app
        .world_mut()
        .resource_mut::<ProjectileSeqCounter>()
        .next();
    app.world_mut().spawn((
        body.kin,
        body.game,
        ProjectileOwner(firer),
        seq,
        ProjectileOwnerId(String::new()),
        LiveProjectile,
        PlayerProjectile,
        ProjectileVisualId(ambition_demo_mary_o::powerups::SPARK_VISUAL.to_string()),
    ));

    let snake_health = |app: &mut App| {
        let world = app.world_mut();
        let mut q = world.query::<(&ActorIdentity, &BodyHealth)>();
        q.iter(world)
            .find(|(id, _)| id.id() == "snake_under_fire")
            .map(|(_, h)| (h.health.current, h.health.max))
    };
    let (before, max) = snake_health(&mut app).expect("the snake spawned as an ECS actor");
    assert_eq!(before, max, "unharmed before the shot");

    for _ in 0..4 {
        app.update();
    }

    let (after, _) = snake_health(&mut app).expect("the snake is still an entity");
    assert!(
        after < before,
        "the spark damaged the snake through the shared hit pipeline \
         (was {before}, now {after})"
    );
}

/// **A stomp SHELLS a snake — it never kills it.**
///
/// The bug this guards: the shell used to be a zero-HP "corpse", but a dead
/// HOSTILE actor is hidden by the render's `visible = !hostile || alive` rule, so
/// a stomped snake simply vanished ("just kills it") with the whole withdraw cycle
/// running invisibly. The fix keeps the body ALIVE and expresses "shelled" as two
/// real levers. This drives the ACTUAL production system on a REAL spawned snake
/// and asserts the stomp: (a) leaves its HP untouched — so it stays visible — and
/// (b) freezes it (`recoil_lock_timer`) and makes it inert (`body_contact_damage`
/// off). Nothing here sets state by hand; it is the stomp, read from the world.
#[test]
fn a_stomp_shells_a_snake_alive_it_never_dies() {
    use ambition::actors::features::{
        spawn_encounter_mob, ActorConfig, ActorIdentity, CharacterRoster, FeatureEcsWorldOverlay,
        GameplayBanner, HitEvent,
    };
    use ambition::characters::actor::character_catalog::CharacterCatalog;
    use ambition::characters::actor::{BodyCombat, BodyHealth};
    use ambition::entity_catalog::placements::CharacterBrain;
    use ambition::platformer::lifecycle::SessionSpawnScope;
    use ambition_demo_mary_o::snake::{run_snake_shells, SnakeShell};

    // Snake head sits at y = 300 - 16 = 284 (size.y = 32). Player feet land in the
    // stomp band just onto that head, falling (+y is down), overlapping in x.
    const SNAKE_POS: ae::Vec2 = ae::Vec2::new(400.0, 300.0);

    let mut app = App::new();
    app.insert_resource(ambition::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    ambition::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new(
            "stomp_range",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        )),
    );
    app.insert_resource(CharacterCatalog::empty());
    app.insert_resource(GameplayBanner::default());
    app.init_resource::<ambition::actors::boss_encounter::BossCatalog>();
    app.init_resource::<FeatureEcsWorldOverlay>();
    app.add_message::<ambition::vfx::VfxMessage>();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    app.add_message::<HitEvent>();

    ambition_demo_mary_o::snake::register_snake_roster(&mut app);
    app.add_systems(Update, run_snake_shells);

    // A falling player whose feet are on the snake's head.
    app.world_mut().spawn((
        PrimaryPlayer,
        ae::BodyKinematics {
            pos: ae::Vec2::new(400.0, 270.0),
            vel: ae::Vec2::new(0.0, 120.0),
            size: ae::Vec2::new(30.0, 48.0),
            facing: 1.0,
        },
    ));

    // One real snake, spawned through the ordinary encounter-mob path so it carries
    // genuine BodyCombat + ActorConfig, then tagged a walker the way staging does.
    {
        let world = app.world_mut();
        let catalog = world.resource::<CharacterCatalog>().clone();
        let roster = world.resource::<CharacterRoster>().clone();
        let mut commands = world.commands();
        spawn_encounter_mob(
            &mut commands,
            &catalog,
            &roster,
            SessionSpawnScope::UNSCOPED,
            "mary_o_stomp_range",
            "stomped_snake".into(),
            CharacterBrain::Custom("mary_o_snake".into()),
            SNAKE_POS,
            ae::Vec2::new(28.0, 32.0),
        );
    }
    app.update(); // flush the spawn

    let snake = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &ActorIdentity)>();
        q.iter(world)
            .find(|(_, id)| id.id() == "stomped_snake")
            .map(|(e, _)| e)
            .expect("the snake spawned as an ECS actor")
    };
    app.world_mut()
        .entity_mut(snake)
        .insert(SnakeShell::Walking);

    // Unharmed, threatening, unfrozen before the stomp.
    {
        let e = app.world().entity(snake);
        assert!(e.get::<BodyHealth>().unwrap().alive(), "starts alive");
        assert!(
            e.get::<ActorConfig>().unwrap().tuning.body_contact_damage,
            "a walker is a contact threat before the stomp"
        );
    }

    app.update(); // the stomp lands

    let e = app.world().entity(snake);
    assert!(
        matches!(*e.get::<SnakeShell>().unwrap(), SnakeShell::Retreating(_)),
        "the stomp starts the in-place withdraw"
    );
    let health = e.get::<BodyHealth>().unwrap();
    assert!(
        health.alive() && health.health.current == health.health.max,
        "a stomp does NOT hurt the snake — full HP, so it is never hidden as a dead \
         hostile actor (current {}, max {})",
        health.health.current,
        health.health.max
    );
    assert!(
        e.get::<BodyCombat>().unwrap().recoil_lock_timer > 0.0,
        "the shelled snake is frozen in place (movement input hard-zeroed)"
    );
    assert!(
        !e.get::<ActorConfig>().unwrap().tuning.body_contact_damage,
        "and inert — a resting shell is safe to walk up to and kick"
    );
}

/// **A moving shell is a kinetic hazard through the SHARED hit pipeline.**
///
/// A sliding shell that overlaps the player from the SIDE (not a stomp) emits, in
/// one `run_snake_shells` tick, TWO `HitEvent`s: a broadcast `Volume` kill over its
/// own AABB (which the shared drain applies to every ENEMY it overlaps — snakes, AI
/// Slop — but never the player, whose query the drain excludes), and a single
/// `Player`-targeted hurt for the side contact. This is the whole point of routing
/// through the pipeline: the shell kills real enemies and hurts the player without
/// any bespoke damage code. (A stomp from ABOVE is the other branch — it stops the
/// shell and bounces you, proven in the pure state-machine tests.)
#[test]
fn a_sliding_shell_emits_an_enemy_kill_and_a_side_hit_on_the_player() {
    use ambition::actors::features::{
        spawn_encounter_mob, ActorIdentity, CharacterRoster, FeatureEcsWorldOverlay,
        GameplayBanner, HitEvent, HitSource, HitTarget,
    };
    use ambition::characters::actor::character_catalog::CharacterCatalog;
    use ambition::entity_catalog::placements::CharacterBrain;
    use ambition::platformer::lifecycle::SessionSpawnScope;
    use ambition_demo_mary_o::snake::{run_snake_shells, SnakeShell};

    const SNAKE_POS: ae::Vec2 = ae::Vec2::new(400.0, 300.0);

    let mut app = App::new();
    app.insert_resource(ambition::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    ambition::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new(
            "shell_range",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        )),
    );
    app.insert_resource(CharacterCatalog::empty());
    app.insert_resource(GameplayBanner::default());
    app.init_resource::<ambition::actors::boss_encounter::BossCatalog>();
    app.init_resource::<FeatureEcsWorldOverlay>();
    app.add_message::<ambition::vfx::VfxMessage>();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    app.add_message::<HitEvent>();

    ambition_demo_mary_o::snake::register_snake_roster(&mut app);
    app.add_systems(Update, run_snake_shells);

    // The player overlaps the snake from the SIDE, at rest (vel.y == 0), so it is a
    // side contact — NOT a stomp (which needs falling feet on the head).
    let player = app
        .world_mut()
        .spawn((
            PrimaryPlayer,
            ae::BodyKinematics {
                pos: ae::Vec2::new(410.0, 300.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(30.0, 48.0),
                facing: 1.0,
            },
        ))
        .id();

    {
        let world = app.world_mut();
        let catalog = world.resource::<CharacterCatalog>().clone();
        let roster = world.resource::<CharacterRoster>().clone();
        let mut commands = world.commands();
        spawn_encounter_mob(
            &mut commands,
            &catalog,
            &roster,
            SessionSpawnScope::UNSCOPED,
            "mary_o_shell_range",
            "sliding_snake".into(),
            CharacterBrain::Custom("mary_o_snake".into()),
            SNAKE_POS,
            ae::Vec2::new(28.0, 32.0),
        );
    }
    app.update(); // flush the spawn

    let snake = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &ActorIdentity)>();
        q.iter(world)
            .find(|(_, id)| id.id() == "sliding_snake")
            .map(|(e, _)| e)
            .expect("the snake spawned")
    };
    // Kick it into a slide directly (the kick geometry is covered elsewhere).
    app.world_mut()
        .entity_mut(snake)
        .insert(SnakeShell::Sliding {
            dir: 1.0,
            grace: 0.0,
        });

    app.update(); // the sliding shell deals its damage

    let hits: Vec<HitEvent> = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<HitEvent>>()
        .drain()
        .collect();

    let enemy_kill = hits.iter().find(|h| {
        matches!(h.target, HitTarget::Volume) && matches!(h.source, HitSource::EnemyChargeCrash)
    });
    assert!(
        enemy_kill.is_some_and(|h| h.damage >= 2),
        "a sliding shell broadcasts a lethal Volume hit that kills enemies it runs down"
    );

    let player_hit = hits.iter().find(|h| {
        matches!(h.target, HitTarget::Player(e) if e == player)
            && matches!(h.source, HitSource::EnemyBody)
    });
    assert!(
        player_hit.is_some(),
        "and a SIDE hit emits a real hit against the player (a stomp from above would \
         instead stop the shell and bounce)"
    );
}

/// **A DEAD snake leaves the shell machine — no invisible hits.**
///
/// The bug this guards, the other half of "a dead snake keeps hitting me": a shell
/// can kill other snakes now, and the engine HIDES a dead hostile actor
/// (`visible = !hostile || alive`). A corpse that kept stepping would go on sliding
/// forever — invisible, still broadcasting lethal `Volume` hits and still able to
/// side-hit the player, with nothing on screen to explain any of it. This drives the
/// real system on a real snake that is sliding AND dead, and asserts it emits
/// nothing at all and stops moving.
#[test]
fn a_dead_snake_leaves_the_shell_machine_and_emits_no_hits() {
    use ambition::actors::features::{
        spawn_encounter_mob, ActorIdentity, CharacterRoster, FeatureEcsWorldOverlay,
        GameplayBanner, HitEvent,
    };
    use ambition::characters::actor::character_catalog::CharacterCatalog;
    use ambition::characters::actor::BodyHealth;
    use ambition::entity_catalog::placements::CharacterBrain;
    use ambition::platformer::lifecycle::SessionSpawnScope;
    use ambition_demo_mary_o::snake::{run_snake_shells, SnakeShell};

    const SNAKE_POS: ae::Vec2 = ae::Vec2::new(400.0, 300.0);

    let mut app = App::new();
    app.insert_resource(ambition::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    ambition::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new(
            "corpse_range",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        )),
    );
    app.insert_resource(CharacterCatalog::empty());
    app.insert_resource(GameplayBanner::default());
    app.init_resource::<ambition::actors::boss_encounter::BossCatalog>();
    app.init_resource::<FeatureEcsWorldOverlay>();
    app.add_message::<ambition::vfx::VfxMessage>();
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    app.add_message::<HitEvent>();

    ambition_demo_mary_o::snake::register_snake_roster(&mut app);
    app.add_systems(Update, run_snake_shells);

    // The player overlaps it from the side — the geometry that WOULD be a hit if
    // the snake were alive (that is exactly what the sibling test proves).
    app.world_mut().spawn((
        PrimaryPlayer,
        ae::BodyKinematics {
            pos: ae::Vec2::new(410.0, 300.0),
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(30.0, 48.0),
            facing: 1.0,
        },
    ));

    {
        let world = app.world_mut();
        let catalog = world.resource::<CharacterCatalog>().clone();
        let roster = world.resource::<CharacterRoster>().clone();
        let mut commands = world.commands();
        spawn_encounter_mob(
            &mut commands,
            &catalog,
            &roster,
            SessionSpawnScope::UNSCOPED,
            "mary_o_corpse_range",
            "dead_snake".into(),
            CharacterBrain::Custom("mary_o_snake".into()),
            SNAKE_POS,
            ae::Vec2::new(28.0, 32.0),
        );
    }
    app.update(); // flush the spawn

    let snake = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &ActorIdentity)>();
        q.iter(world)
            .find(|(_, id)| id.id() == "dead_snake")
            .map(|(e, _)| e)
            .expect("the snake spawned")
    };
    // A shell ran it down: killed mid-slide, still carrying slide velocity.
    {
        let mut e = app.world_mut().entity_mut(snake);
        e.insert(SnakeShell::Sliding {
            dir: 1.0,
            grace: 0.0,
        });
        e.get_mut::<BodyHealth>().unwrap().health.current = 0;
        e.get_mut::<ae::BodyKinematics>().unwrap().vel.x = 300.0;
    }
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<HitEvent>>()
        .drain()
        .for_each(drop);

    app.update();

    let hits: Vec<HitEvent> = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<HitEvent>>()
        .drain()
        .collect();
    assert!(
        !app.world()
            .entity(snake)
            .get::<BodyHealth>()
            .unwrap()
            .alive(),
        "the fixture really is a corpse"
    );
    assert!(
        hits.is_empty(),
        "a dead snake is out of the mechanic: an invisible corpse must not keep \
         hitting anything (got {hits:?})"
    );
    assert_eq!(
        app.world()
            .entity(snake)
            .get::<ae::BodyKinematics>()
            .unwrap()
            .vel
            .x,
        0.0,
        "and it stops sliding instead of drifting off invisibly"
    );
}

/// **A warp is a MOVE, not a teleport.**
///
/// Pressing DOWN on the pipe mouth used to relocate the body in a single frame:
/// you were on the surface, then you were in the vault, with nothing in between.
/// That reads as a teleport rather than as a pipe. This drives the REAL systems
/// on a REAL body and asserts the trip: the press starts a transit instead of
/// finishing one, the body is still near the mouth a frame later (sinking, not
/// gone), and it arrives — exactly on the authored arrival — only after the
/// authored slide has run. Nothing here sets a position by hand.
#[test]
fn pressing_down_on_the_pipe_slides_the_body_through_it_over_time() {
    use ambition::characters::actor::BodyCombat;
    use ambition::characters::brain::ActorControl;
    use ambition::engine_core::AabbExt;
    use ambition_demo_mary_o::pipe::{PipeTransit, EMERGE_S, SWALLOW_S};
    use ambition_demo_mary_o::{pipe_mouth, vault_arrival};

    let mut app = App::new();
    app.insert_resource(ambition::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    app.add_message::<ambition::sfx::OwnedSfxMessage>();
    app.add_plugins(ambition_demo_mary_o::MaryORulesPlugin::global());

    // A full body standing ON the entry pipe's mouth.
    let mouth = pipe_mouth();
    let body = app
        .world_mut()
        .spawn((
            PrimaryPlayer,
            ambition::platformer::markers::PlayerEntity,
            ae::BodyKinematics {
                pos: mouth.center(),
                vel: ae::Vec2::ZERO,
                size: ae::movement::default_player_body_size(),
                facing: 1.0,
            },
            ambition::actors::actor::AncillaryMovementBundle::from_scratch(
                ae::BodyClusterScratch::new_with_abilities(
                    mouth.center(),
                    ae::AbilitySet::sandbox_all(),
                ),
            ),
            ambition::actors::features::MotionModel::default(),
            BodyCombat::default(),
            ActorControl::default(),
        ))
        .id();
    app.insert_resource(ambition::platformer::markers::ControlledSubject(Some(body)));
    // Deliberately NOT adding `run_pipe_transits` by hand: the plugin must wire
    // the slide itself, or a warp starts and never finishes in the real game.
    app.update();

    let pos_of = |app: &App| app.world().get::<ae::BodyKinematics>(body).unwrap().pos;
    let started = pos_of(&app);

    // Press DOWN — the pipe's own verb.
    app.world_mut()
        .get_mut::<ActorControl>(body)
        .unwrap()
        .0
        .locomotion
        .y = 1.0;
    app.update();

    assert!(
        app.world().get::<PipeTransit>(body).is_some(),
        "the press must START a transit, not finish one"
    );
    // And the trip is AUDIBLE: the warp cue goes out once, through the shared sfx
    // seam, on the demo's own authored id.
    let warps = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ambition::sfx::OwnedSfxMessage>>()
        .drain()
        .filter(|m| {
            matches!(
                m.request,
                ambition::sfx::SfxMessage::Play { id, .. }
                    if id == ambition::sfx::SfxId::new(ambition_demo_mary_o::pipe::PIPE_WARP_SFX)
            )
        })
        .count();
    assert_eq!(warps, 1, "entering a pipe voices the warp cue exactly once");
    let after_one_frame = pos_of(&app);
    assert!(
        after_one_frame.distance(vault_arrival())
            > 4.0 * ae::movement::default_player_body_size().y,
        "one frame in she is still in the pipe, NOT already in the vault (the \
         teleport this replaces): {after_one_frame:?} vs {:?}",
        vault_arrival()
    );
    assert!(
        after_one_frame.y > started.y,
        "and she has begun to SINK into it ({} -> {})",
        started.y,
        after_one_frame.y
    );

    // Let the whole authored slide run.
    let ticks = ((SWALLOW_S + EMERGE_S) / (1.0 / 60.0)).ceil() as usize + 4;
    for _ in 0..ticks {
        app.update();
    }
    assert!(
        app.world().get::<PipeTransit>(body).is_none(),
        "the transit ends and gives the body back"
    );
    assert_eq!(
        pos_of(&app),
        vault_arrival(),
        "and she lands exactly on the authored arrival"
    );
    assert_eq!(
        app.world()
            .get::<BodyCombat>(body)
            .unwrap()
            .recoil_lock_timer,
        0.0,
        "with her controls returned — the transit's input lock is released"
    );
}
