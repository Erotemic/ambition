//! The whole loop, on the real systems.
//!
//! Each unit test elsewhere proves one link. This walks the chain a player walks,
//! with no step simulated by hand:
//!
//! ```text
//! bonk a block -> wand pops -> touch it -> GROWN
//!   -> bonk again -> beacon pops -> touch it -> SPARK-POWERED (ranged verb live)
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

use ambition_platformer2d::actors::features::spawn_encounter_mob;
use ambition_platformer2d::actors::features::EncounterMobSeed;
use ambition_platformer2d::platformer::feature_overlay::FeatureEcsWorldOverlay;
use bevy::prelude::*;

use ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput;
use ambition_platformer2d::actors::features::transform_beat::{
    TransformBeatPolicy, TransformBeatRequested,
};
use ambition_platformer2d::actors::items::{collect_world_items, WorldItem};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::characters::brain::action_set::{ActionSet, IdentityKit};
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::characters::equipment::WornEquipment;
use ambition_platformer2d::combat::moveset::{ActorMoveset, RANGED_VERB};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::collision_semantics::{ContactKind, ContactSource};
use ambition_platformer2d::engine_core::BodyBaseSize;
use ambition_platformer2d::items::equipment::reconcile_equipment_grants;
use ambition_platformer2d::platformer::markers::ControlledSubject;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::sprite_sheet::character::{
    try_load_spec_for_target, CharacterAnim, SheetTuning,
};

use ambition_demo_mary_o::movement::{
    fire_spark_on_run_press, tick_spark_cooldown, walk_by_default_run_while_held, MaryOGait,
    MaryOSparkCooldown, WALK_THROTTLE,
};
use ambition_demo_mary_o::powerups::{
    bonk_power_blocks, cinder_beacon, sync_grown_form, SpentPowerBlocks, CINDER_BEACON_ID,
    STAR_WAND_ID,
};
use ambition_demo_mary_o::provider::MARY_O_CHARACTER_ID;

const TALL_ID: &str = "mary_o_tall";
const FIRE_ID: &str = "mary_o_fire";

struct Loop {
    app: App,
    body: Entity,
    /// The AUTHORED ?-block this harness bonks.
    struck: ae::GeoId,
}

impl Loop {
    fn new() -> Self {
        let mut app = App::new();
        app.insert_resource(ambition_platformer2d::time::WorldTime {
            scaled_dt: 1.0 / 60.0,
            ..Default::default()
        });
        app.init_resource::<SpentPowerBlocks>();
        // The bonk handler asks the ROOM what it hit, so the room has to be here.
        let room = ambition_demo_mary_o::level_1_1();
        let struck = room
            .world
            .blocks
            .iter()
            .find(|b| {
                ambition_demo_mary_o::ldtk_vocabulary::block_look_of(&b.name)
                    == Some(ambition_demo_mary_o::ldtk_vocabulary::MaryOBlockLook::Question)
            })
            .expect("the level authors a ?-block")
            .id
            .clone();
        ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
            app.world_mut(),
            ae::RoomGeometry(room.world.clone()),
        );
        // `sync_grown_form` now voices a transform chime through `SfxWriter`.
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        app.add_message::<ambition_platformer2d::platformer::block_nudge::BlockStruck>();
        app.add_message::<ambition_platformer2d::vfx::VfxMessage>();

        let size = ae::movement::default_player_body_size();
        let body = app
            .world_mut()
            .spawn((
                PrimaryPlayer,
                // A home avatar carries BOTH markers in production, and this
                // fixture carried only one. Touch-collection is body-generic
                // now — the population is "in the player population, or driven
                // through possession" — so a body that is neither collects
                // nothing, exactly as an autonomous actor standing on a
                // mushroom collects nothing.
                ambition_platformer2d::platformer::markers::PlayerEntity,
                WornCharacter::new(MARY_O_CHARACTER_ID),
                BodyBaseSize { base_size: size },
                ae::BodyKinematics {
                    pos: ae::Vec2::new(0.0, 0.0),
                    vel: ae::Vec2::ZERO,
                    size,
                    facing: 1.0,
                },
                // A peaceful identity: any ranged verb she ends up with can ONLY
                // have come from the beacon, reconciled onto this baseline.
                IdentityKit::default(),
                ActionSet::peaceful(),
                ActorMoveset(Default::default()),
                ActorControl::default(),
                MaryOGait::default(),
                MaryOSparkCooldown::default(),
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
                tick_spark_cooldown,
                fire_spark_on_run_press,
            )
                .chain(),
        );
        Self { app, body, struck }
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
                // A hand-built rest contact: nothing arrived at this surface.
                impact_speed: 0.0,
                involuntary: false,
                kind: ContactKind::Head,
                point: ae::Vec2::ZERO,
                normal: ae::Vec2::new(0.0, 1.0),
                toi: 0.0,
                surface_velocity: ae::Vec2::ZERO,
                source: ContactSource::Block {
                    kind: ae::BlockKind::Solid,
                    id: self.struck.clone(),
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
            .rearm_all();
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
        // Both power forms (grown wand = `mary_o_tall`, fire beacon = `mary_o_fire`)
        // share the tall SIZE and differ only from the small starting form, so
        // "tall" is "wearing any power sheet" rather than one specific sheet.
        self.app
            .world()
            .get::<WornCharacter>(self.body)
            .unwrap()
            .id()
            != MARY_O_CHARACTER_ID
    }
    fn worn_character(&self) -> String {
        self.app
            .world()
            .get::<WornCharacter>(self.body)
            .unwrap()
            .id()
            .to_string()
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

    /// The transformation beat the last tier change asked for: the policy it
    /// authored, and whether the request that starts it is on the body.
    fn requested_beat(&self) -> (TransformBeatPolicy, bool) {
        let policy = *self
            .app
            .world()
            .get::<TransformBeatPolicy>(self.body)
            .expect("a tier change authors its transformation beat");
        let requested = self
            .app
            .world()
            .get::<TransformBeatRequested>(self.body)
            .is_some();
        (policy, requested)
    }
}

/// How long one pass of `anim` takes on a form's sheet — the same question the
/// demo asks the art, asked here independently so the beat is checked against
/// the ART rather than against the number the demo happened to write down.
fn clip_secs(sheet_target: &str, anim: CharacterAnim) -> f32 {
    try_load_spec_for_target(sheet_target, &SheetTuning::default())
        .unwrap_or_else(|| panic!("{sheet_target} publishes a sheet manifest"))
        .clip_seconds(anim)
}

/// A transformation lasts as long as the art that shows it.
///
/// Every tier change — up AND down — authors a beat that names the clip the ARRIVING sheet drew for
/// it and holds long enough for every frame of that clip to be drawn.
///
/// The durations are compared against the sheets, not against constants: the
/// generator owns the frame tables, and a test that copied them would agree with
/// a stale demo and disagree with the art.
// It has.

#[test]
fn every_tier_change_holds_its_arriving_sheets_transition_clip() {
    let mut game = Loop::new();

    // --- small -> grown: the growth flicker, on the sheet she becomes --------
    game.bonk();
    game.collect_pending_item();
    assert_eq!(game.worn_character(), TALL_ID);
    let (grow, requested) = game.requested_beat();
    assert!(requested, "growing asks the engine for its beat");
    assert_eq!(
        grow.anim,
        CharacterAnim::Grow,
        "growing shows the grow clip, not a stand-in held pose"
    );
    assert!(
        grow.clock_scale < 1.0,
        "a step UP asks the regime to slow the world for the moment"
    );
    let grow_clip = clip_secs("mary_o_v2_tall", CharacterAnim::Grow);
    assert!(
        grow.duration * grow.clock_scale >= grow_clip - 1e-4,
        "the beat must outlast its own dilation: {:.3}s of wall time at {:.2}x \
         draws {:.3}s of a {:.3}s clip",
        grow.duration,
        grow.clock_scale,
        grow.duration * grow.clock_scale,
        grow_clip,
    );

    // --- grown -> fire: the same-size transformation -------------------------
    game.bonk();
    game.collect_pending_item();
    assert_eq!(game.worn_character(), FIRE_ID);
    let (transform, _) = game.requested_beat();
    assert_eq!(transform.anim, CharacterAnim::Transform);
    let transform_clip = clip_secs("mary_o_v2_fire", CharacterAnim::Transform);
    assert!(
        transform.duration * transform.clock_scale >= transform_clip - 1e-4,
        "the eight-frame fire transformation is the clip a flat 0.5s cut off"
    );
    assert!(
        transform_clip > grow_clip,
        "and it is the longer of the two, so a shared constant could not fit both"
    );

    // --- a hit: fire -> grown, the power-loss clip ---------------------------
    game.hit();
    assert_eq!(game.worn_character(), TALL_ID);
    let (shrink, requested) = game.requested_beat();
    assert!(
        requested,
        "losing a form is a transformation too (Jon bug #17)"
    );
    assert_eq!(
        shrink.anim,
        CharacterAnim::Shrink,
        "and it shows the shrink the arriving sheet drew"
    );
    assert_eq!(
        shrink.clock_scale, 1.0,
        "a hit does not slow the world — that would take her recovery away"
    );
    assert!(
        shrink.untouchable,
        "the beat is the hitstun window she had none of"
    );
    assert!(
        shrink.duration >= clip_secs("mary_o_v2_tall", CharacterAnim::Shrink) - 1e-4,
        "and it lasts as long as the clip"
    );

    // --- a second hit: grown -> small ---------------------------------------
    game.hit();
    assert_eq!(game.worn_character(), MARY_O_CHARACTER_ID);
    let (small, _) = game.requested_beat();
    assert_eq!(small.anim, CharacterAnim::Shrink);
    assert!(small.duration >= clip_secs("mary_o_v2", CharacterAnim::Shrink) - 1e-4);
}

/// The transition clips are on the sheets, and they are their OWN rows.
///
/// A sheet publishes the clips for the form it is ARRIVED AT, which is what lets
/// the runtime swap identity first and still show the change. And none of them
/// alias `Hit`: the ordinary hitstun read picks `Hit` for as long as hitstun
/// runs, so a shrink that WAS `Hit` would be replayed by the locomotion picker
/// after the beat had already finished playing it.
#[test]
fn each_mary_o_sheet_publishes_the_transitions_that_arrive_at_it() {
    let sheet = |target: &str| {
        try_load_spec_for_target(target, &SheetTuning::default())
            .unwrap_or_else(|| panic!("{target} publishes a sheet manifest"))
    };

    let small = sheet("mary_o_v2");
    assert!(small.maps(CharacterAnim::Shrink), "tall becomes small here");
    assert!(
        small.maps(CharacterAnim::BigShrink),
        "and fire can lose two tiers into it"
    );

    let tall = sheet("mary_o_v2_tall");
    assert!(tall.maps(CharacterAnim::Grow), "small becomes tall here");
    assert!(
        tall.maps(CharacterAnim::Shrink),
        "and fire falls back to it"
    );

    let fire = sheet("mary_o_v2_fire");
    assert!(
        fire.maps(CharacterAnim::Transform),
        "tall becomes fire here"
    );

    for (name, spec) in [
        ("mary_o_v2", &small),
        ("mary_o_v2_tall", &tall),
        ("mary_o_v2_fire", &fire),
    ] {
        assert!(
            !spec.maps(CharacterAnim::Hit),
            "{name}: a form change is not the generic hurt row — aliasing them \
             lets the hitstun read replay the transition after its beat ended"
        );
    }
}

#[test]
fn the_whole_power_loop_runs_on_the_real_systems() {
    let mut game = Loop::new();

    // --- small ---------------------------------------------------------------
    assert!(!game.is_tall(), "she starts small");
    assert!(!game.has_ranged_move(), "and unarmed");

    // --- collect wand -> grown ----------------------------------------------
    game.bonk();
    game.collect_pending_item();
    assert!(
        game.wears(STAR_WAND_ID),
        "the block gave a small Mary-O wand"
    );
    assert!(game.is_tall(), "collecting it grew her");
    assert_eq!(
        game.worn_character(),
        TALL_ID,
        "the wand shows the plain grown sheet"
    );
    assert!(
        !game.has_ranged_move(),
        "the wand is armor only — it grants no verb"
    );

    // --- collect beacon -> spark-powered ------------------------------------
    game.bonk();
    game.collect_pending_item();
    assert!(
        game.wears(CINDER_BEACON_ID),
        "the block gave a GROWN Mary-O the beacon, not another wand"
    );
    assert!(game.is_tall(), "she is still tall");
    assert_eq!(
        game.worn_character(),
        FIRE_ID,
        "the beacon swaps her to the DISTINCT fire sheet, not the plain grown one (Jon bug #10)"
    );
    assert!(
        game.has_ranged_move(),
        "and the reconcile turned the beacon's grant into a fireable move"
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
    assert!(!game.wears(CINDER_BEACON_ID), "the hit spent the beacon");
    assert!(game.wears(STAR_WAND_ID), "downgrading to the wand");
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
    assert!(!game.wears(STAR_WAND_ID), "the second hit spent the wand");
    assert!(!game.is_tall(), "and returned her to small");
}

/// The spark's flight, on the shared projectile body. Her shot is authored
/// data; this steps the ENGINE's projectile primitive with that data and watches
/// it arc, skip off a floor, and expire on its authored budget. No Mary-O code is
/// involved in the stepping — that is the point.
#[test]
fn the_authored_spark_arcs_bounces_and_expires() {
    use ambition_platformer2d::characters::equipment::apply_equipment_grants;
    use ambition_platformer2d::platformer::projectile::{
        ProjectileBody, ProjectileSpec, WorldHitPolicy,
    };

    // Take the shot exactly as the beacon grants it.
    let mut actions = ActionSet::peaceful();
    apply_equipment_grants(&mut actions, &WornEquipment::new(vec![cinder_beacon()]));
    let shot = actions.ranged.expect("the beacon grants a shot");
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
        boomerang_return_s: None,
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

/// The spark kills a snake through the canonical hit path.
///
/// The composition is the claim worth testing: the engine already proves its
/// stepper damages actors, and the loop above already proves the beacon grants an
/// ordinary ranged verb. What is left to show is that HER shot — authored flight,
/// authored visual, content-marked — is not special to any of it. So this builds
/// the projectile from the beacon's own grant, hands it to the shared stepper as
/// a player-faction shot, and watches a snake lose HP through
/// `apply_feature_hit_events`. Nothing in the damage path knows what a spark is.
#[test]
fn her_spark_damages_a_snake_through_the_shared_hit_pipeline() {
    use ambition_platformer2d::world::FeatureEcsWorldOverlay;
    use ambition_platformer2d::actors::features::{apply_feature_hit_events, spawn_encounter_mob, EncounterMobSeed};
    use ambition_platformer2d::combat::components::ActorIdentity;
    use ambition_platformer2d::combat::events::{GameplayBanner, HitEvent, SetFlagRequested};
    // ⛔ `ProjectileBody` belongs to the projectile MODEL crate; the monolith's
    // glob forward of it was deleted, and no gate builds this target.
    use ambition_platformer2d::actors::projectile::step_projectiles;
    use ambition_platformer2d::characters::actor::{
        character_catalog::CharacterCatalog, BodyHealth,
    };
    use ambition_platformer2d::characters::equipment::apply_equipment_grants;
    use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
    use ambition_platformer2d::platformer::lifecycle::SessionSpawnScope;
    use ambition_platformer2d::platformer::projectile::{ProjectileSpec, WorldHitPolicy};
    use ambition_platformer2d::projectiles::ProjectileBody;
    use ambition_platformer2d::projectiles::{
        LiveProjectile, ProjectileOwner, ProjectileSeqCounter, ProjectileVisualCatalog,
        ProjectileVisualId,
    };

    const SNAKE_POS: ae::Vec2 = ae::Vec2::new(400.0, 300.0);

    let mut app = App::new();
    app.insert_resource(ambition_platformer2d::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new(
            "spark_range",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        )),
    );
    app.insert_resource(CharacterCatalog::empty());
    // The damage path sizes split offspring from their sheets (U1 stage B), so
    // the authored registry is required authority here too. This fixture
    // authors none.
    app.init_resource::<ambition_platformer2d::character::AuthoredSheets>();
    app.insert_resource(GameplayBanner::default());
    app.init_resource::<ambition_platformer2d::boss_encounter::BossCatalog>();
    app.init_resource::<ProjectileSeqCounter>();
    app.init_resource::<ProjectileVisualCatalog>();
    app.init_resource::<FeatureEcsWorldOverlay>();
    app.init_resource::<ambition_platformer2d::gameplay_trace::GameplayTraceBuffer>();
    app.add_message::<HitEvent>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<ambition_platformer2d::combat::events::ActorStimulus>();
    app.add_message::<ambition_platformer2d::damage::WalletShieldSpent>();
    // A body reaching zero says so through `BodyKnockedOut` whether or not a
    // stocks ruleset is listening. This fixture hand-picks its systems, so it
    // hand-registers their messages; `CombatSchedulePlugin` covers the apps that
    // install the whole schedule.
    app.add_message::<ambition_platformer2d::combat::stocks::BodyKnockedOut>();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::vfx::vfx::DebrisBurstMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::platformer::block_nudge::BlockStruck>();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::actors::avatar::PlayerHealRequested>();

    // every Mary-O enemy is a CHARACTER now (the plane swarms joined
    ambition_demo_mary_o::snake::register_solid_snake_character(&mut app);
    ambition_platformer2d::platformer::app_finalization::finalize(&mut app);
    app.add_systems(Update, (step_projectiles, apply_feature_hit_events).chain());

    // A player-faction firer to own the shot.
    let firer = app
        .world_mut()
        .spawn((
            PrimaryPlayer,
            ambition_platformer2d::characters::actor::ActorFaction::Player,
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
        // The prepared cast this demo registers. Its two enemies are CHARACTERS
        // now, so a mob that names one is built from it rather than from a
        // roster row — the row is gone.
        let world_prepared = world
            .get_resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>()
            .cloned()
            .unwrap_or_default();
        let mut commands = world.commands();
        spawn_encounter_mob(
            &mut commands,
            &catalog,
            &Default::default(),
            &world_prepared,
            SessionSpawnScope::UNSCOPED,
            "mary_o_spark_range",
            EncounterMobSeed {
                id: "snake_under_fire".into(),
                // Production names the character on the placement; a fixture
                // that named none would exercise a spawn shape the game no
                // longer has.
                character: Some(ambition_demo_mary_o::snake::SNAKE_SHEET_TARGET),
                brain: CharacterBrain::Custom("mary_o_snake".into()),
                pos: SNAKE_POS,
                size: ae::Vec2::new(28.0, 32.0),
            },
        );
    }
    app.update();

    // Her shot, straight from the beacon's grant.
    let mut actions = ActionSet::peaceful();
    apply_equipment_grants(&mut actions, &WornEquipment::new(vec![cinder_beacon()]));
    let shot = actions.ranged.expect("the beacon grants a shot");
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
        boomerang_return_s: None,
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
        LiveProjectile,
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

/// A stomp SHELLS a snake — it never kills it.
#[test]
fn a_stomp_shells_a_snake_alive_it_never_dies() {
    use ambition_demo_mary_o::snake::{run_snake_shells, SnakeShell};
    use ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog;
    use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
    use ambition_platformer2d::combat::actor_tuning::ActorConfig;
    use ambition_platformer2d::combat::components::ActorIdentity;
    use ambition_platformer2d::combat::events::{GameplayBanner, HitEvent};
    use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
    use ambition_platformer2d::platformer::lifecycle::SessionSpawnScope;

    // Snake head sits at y = 300 - 16 = 284 (size.y = 32). Player feet land in the
    // stomp band just onto that head, falling (+y is down), overlapping in x.
    const SNAKE_POS: ae::Vec2 = ae::Vec2::new(400.0, 300.0);

    let mut app = App::new();
    app.insert_resource(ambition_platformer2d::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new(
            "stomp_range",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        )),
    );
    app.insert_resource(CharacterCatalog::empty());
    // The damage path sizes split offspring from their sheets (U1 stage B), so
    // the authored registry is required authority here too. This fixture
    // authors none.
    app.init_resource::<ambition_platformer2d::character::AuthoredSheets>();
    app.insert_resource(GameplayBanner::default());
    app.init_resource::<ambition_platformer2d::boss_encounter::BossCatalog>();
    app.init_resource::<FeatureEcsWorldOverlay>();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::platformer::block_nudge::BlockStruck>();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<HitEvent>();

    // every Mary-O enemy is a CHARACTER now (the plane swarms joined
    ambition_demo_mary_o::snake::register_solid_snake_character(&mut app);
    ambition_platformer2d::platformer::app_finalization::finalize(&mut app);
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
        // The prepared cast this demo registers. Its two enemies are CHARACTERS
        // now, so a mob that names one is built from it rather than from a
        // roster row — the row is gone.
        let world_prepared = world
            .get_resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>()
            .cloned()
            .unwrap_or_default();
        let mut commands = world.commands();
        spawn_encounter_mob(
            &mut commands,
            &catalog,
            &Default::default(),
            &world_prepared,
            SessionSpawnScope::UNSCOPED,
            "mary_o_stomp_range",
            EncounterMobSeed {
                id: "stomped_snake".into(),
                // Production names the character on the placement; a fixture
                // that named none would exercise a spawn shape the game no
                // longer has.
                character: Some(ambition_demo_mary_o::snake::SNAKE_SHEET_TARGET),
                brain: CharacterBrain::Custom("mary_o_snake".into()),
                pos: SNAKE_POS,
                size: ae::Vec2::new(28.0, 32.0),
            },
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

/// A moving shell is a kinetic hazard through the SHARED hit pipeline.
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
    use ambition_demo_mary_o::snake::{run_snake_shells, SnakeShell};
    use ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog;
    use ambition_platformer2d::combat::components::ActorIdentity;
    use ambition_platformer2d::combat::events::{GameplayBanner, HitEvent, HitSource, HitTarget};
    use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
    use ambition_platformer2d::platformer::lifecycle::SessionSpawnScope;

    const SNAKE_POS: ae::Vec2 = ae::Vec2::new(400.0, 300.0);

    let mut app = App::new();
    app.insert_resource(ambition_platformer2d::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new(
            "shell_range",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        )),
    );
    app.insert_resource(CharacterCatalog::empty());
    // The damage path sizes split offspring from their sheets (U1 stage B), so
    // the authored registry is required authority here too. This fixture
    // authors none.
    app.init_resource::<ambition_platformer2d::character::AuthoredSheets>();
    app.insert_resource(GameplayBanner::default());
    app.init_resource::<ambition_platformer2d::boss_encounter::BossCatalog>();
    app.init_resource::<FeatureEcsWorldOverlay>();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::platformer::block_nudge::BlockStruck>();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<HitEvent>();

    // every Mary-O enemy is a CHARACTER now (the plane swarms joined
    ambition_demo_mary_o::snake::register_solid_snake_character(&mut app);
    ambition_platformer2d::platformer::app_finalization::finalize(&mut app);
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
        // The prepared cast this demo registers. Its two enemies are CHARACTERS
        // now, so a mob that names one is built from it rather than from a
        // roster row — the row is gone.
        let world_prepared = world
            .get_resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>()
            .cloned()
            .unwrap_or_default();
        let mut commands = world.commands();
        spawn_encounter_mob(
            &mut commands,
            &catalog,
            &Default::default(),
            &world_prepared,
            SessionSpawnScope::UNSCOPED,
            "mary_o_shell_range",
            EncounterMobSeed {
                id: "sliding_snake".into(),
                // Production names the character on the placement; a fixture
                // that named none would exercise a spawn shape the game no
                // longer has.
                character: Some(ambition_demo_mary_o::snake::SNAKE_SHEET_TARGET),
                brain: CharacterBrain::Custom("mary_o_snake".into()),
                pos: SNAKE_POS,
                size: ae::Vec2::new(28.0, 32.0),
            },
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

    let enemy_kill = hits
        .iter()
        .find(|h| matches!(h.target, HitTarget::Volume) && matches!(h.source, HitSource::Contact));
    assert!(
        enemy_kill.is_some_and(|h| h.damage >= 2 && h.attacker == Some(snake)),
        "a sliding shell broadcasts a lethal Volume hit attributed to the shell entity"
    );

    let player_hit = hits.iter().find(|h| {
        matches!(h.target, HitTarget::Body(e) if e == player)
            && matches!(h.source, HitSource::Contact)
    });
    assert!(
        player_hit.is_some_and(|h| h.attacker == Some(snake)),
        "and a SIDE hit against the player retains the same shell attribution (a stomp \
         from above would instead stop the shell and bounce)"
    );
}

/// A DEAD snake leaves the shell machine — no invisible hits.
#[test]
fn a_dead_snake_leaves_the_shell_machine_and_emits_no_hits() {
    use ambition_demo_mary_o::snake::{run_snake_shells, SnakeShell};
    use ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog;
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::combat::components::ActorIdentity;
    use ambition_platformer2d::combat::events::{GameplayBanner, HitEvent};
    use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
    use ambition_platformer2d::platformer::lifecycle::SessionSpawnScope;

    const SNAKE_POS: ae::Vec2 = ae::Vec2::new(400.0, 300.0);

    let mut app = App::new();
    app.insert_resource(ambition_platformer2d::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new(
            "corpse_range",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 200.0),
            Vec::new(),
        )),
    );
    app.insert_resource(CharacterCatalog::empty());
    // The damage path sizes split offspring from their sheets (U1 stage B), so
    // the authored registry is required authority here too. This fixture
    // authors none.
    app.init_resource::<ambition_platformer2d::character::AuthoredSheets>();
    app.insert_resource(GameplayBanner::default());
    app.init_resource::<ambition_platformer2d::boss_encounter::BossCatalog>();
    app.init_resource::<FeatureEcsWorldOverlay>();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::platformer::block_nudge::BlockStruck>();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_message::<HitEvent>();

    // every Mary-O enemy is a CHARACTER now (the plane swarms joined
    ambition_demo_mary_o::snake::register_solid_snake_character(&mut app);
    ambition_platformer2d::platformer::app_finalization::finalize(&mut app);
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
        // The prepared cast this demo registers. Its two enemies are CHARACTERS
        // now, so a mob that names one is built from it rather than from a
        // roster row — the row is gone.
        let world_prepared = world
            .get_resource::<ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry>()
            .cloned()
            .unwrap_or_default();
        let mut commands = world.commands();
        spawn_encounter_mob(
            &mut commands,
            &catalog,
            &Default::default(),
            &world_prepared,
            SessionSpawnScope::UNSCOPED,
            "mary_o_corpse_range",
            EncounterMobSeed {
                id: "dead_snake".into(),
                // Production names the character on the placement; a fixture
                // that named none would exercise a spawn shape the game no
                // longer has.
                character: Some(ambition_demo_mary_o::snake::SNAKE_SHEET_TARGET),
                brain: CharacterBrain::Custom("mary_o_snake".into()),
                pos: SNAKE_POS,
                size: ae::Vec2::new(28.0, 32.0),
            },
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

// ── Warp tubes ─────────────────────────────────────────────────────────────

/// A rules-only shell STANDING IN one authored room.
///
/// Tubes are keyed by the room that authors them now and the reader asks `RoomSet` where the body
/// is standing, so the room is a parameter — which is what lets the three tests below be the same
/// test in three rooms.
fn pipe_shell(room_id: &str) -> App {
    let mut app = App::new();
    app.insert_resource(ambition_platformer2d::time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
    app.add_message::<ambition_platformer2d::platformer::block_nudge::BlockStruck>();
    app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
    app.add_plugins(ambition_demo_mary_o::MaryORulesPlugin::global());
    ambition_platformer2d::platformer::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_demo_mary_o::provider::mary_o_session_world_entering(room_id).room_set,
    );
    app
}

/// A full-size, controlled body standing at `pos` — the player, as far as every
/// rule under test can tell.
fn body_standing_at(app: &mut App, pos: ae::Vec2) -> Entity {
    let body = app
        .world_mut()
        .spawn((
            PrimaryPlayer,
            ambition_platformer2d::platformer::markers::PlayerEntity,
            ae::BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size: ae::movement::default_player_body_size(),
                facing: 1.0,
            },
            ambition_platformer2d::actors::actor::AncillaryMovementBundle::from_scratch(
                ae::BodyClusterScratch::new_with_abilities(pos, ae::AbilitySet::sandbox_all()),
            ),
            ambition_platformer2d::actor::MotionModel::default(),
            ambition_platformer2d::characters::actor::BodyCombat::default(),
            ActorControl::default(),
        ))
        .id();
    app.insert_resource(ControlledSubject(Some(body)));
    body
}

/// Press a direction on the body's locomotion stick: `+1` is toward the feet
/// (screen-down under Mary-O's gravity), `-1` away from them.
fn press_locomotion(app: &mut App, body: Entity, y: f32) {
    app.world_mut()
        .get_mut::<ActorControl>(body)
        .unwrap()
        .0
        .locomotion
        .y = y;
}

/// A warp is a MOVE, not a teleport.
///
/// That reads as a teleport rather than as a pipe. This drives the REAL systems on a REAL body and
/// asserts the trip: the press starts a transit instead of finishing one, the body is still near
/// the mouth a frame later (sinking, not gone), and it arrives — exactly on the authored arrival —
/// only after the authored slide has run. Nothing here sets a position by hand.
#[test]
fn pressing_down_on_the_pipe_slides_the_body_through_it_over_time() {
    use ambition_demo_mary_o::pipe::{PipeTransit, EMERGE_S, SWALLOW_S};
    use ambition_demo_mary_o::{pipe_mouth, vault_arrival, LEVEL_1_1_ROOM_ID};
    use ambition_platformer2d::characters::actor::BodyCombat;
    use ambition_platformer2d::engine_core::AabbExt;

    let mut app = pipe_shell(LEVEL_1_1_ROOM_ID);

    // A full body standing ON the entry pipe's mouth.
    let mouth = pipe_mouth();
    let body = body_standing_at(&mut app, mouth.center());
    // Deliberately NOT adding `run_pipe_transits` by hand: the plugin must wire
    // the slide itself, or a warp starts and never finishes in the real game.
    app.update();

    let pos_of = |app: &App| app.world().get::<ae::BodyKinematics>(body).unwrap().pos;
    let started = pos_of(&app);

    // Press DOWN — the pipe's own verb.
    press_locomotion(&mut app, body, 1.0);
    app.update();

    assert!(
        app.world().get::<PipeTransit>(body).is_some(),
        "the press must START a transit, not finish one"
    );
    // And the trip is AUDIBLE: the warp cue goes out once, through the shared sfx
    // seam, on the demo's own authored id.
    let warps = app
        .world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>()
        .drain()
        .filter(|m| {
            matches!(
                m.request,
                ambition_platformer2d::sfx::SfxMessage::Play { id, .. }
                    if id == ambition_platformer2d::sfx::SfxId::new(ambition_demo_mary_o::pipe::PIPE_WARP_SFX)
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

/// And the way BACK — the leg nothing else runs.
///
/// the two tests that walked 1-1's whole secret route (`scripted_level_run`
/// and `level_1_acceptance`) are both `#[ignore]`d, tuned to an older
/// arrangement — so the ASCENT was the half of the vault trip with no running
/// coverage at all. It is also the half the authored `role` decides: both of
/// the vault's pipes hang from the same ceiling with the same down-facing
/// mouth, and only the field says which one you may press UP into.
///
/// Same real plugin, same real systems, opposite verb.
#[test]
fn pressing_up_under_the_vault_pipe_surfaces_her_on_the_exit_pipe() {
    use ambition_demo_mary_o::pipe::{PipeTransit, EMERGE_S, SWALLOW_S};
    use ambition_demo_mary_o::{pipe_arrival, vault_exit, LEVEL_1_1_ROOM_ID};
    use ambition_platformer2d::engine_core::AabbExt;

    let mut app = pipe_shell(LEVEL_1_1_ROOM_ID);

    // A body with its head in the return pipe's mouth, which is where a player
    // standing on the vault floor under it ends up.
    let mouth = vault_exit();
    let body = body_standing_at(&mut app, mouth.center());
    app.update();

    // Press UP — the verb a down-facing mouth answers, and the one the descent
    // tube's entrance beside it does NOT.
    press_locomotion(&mut app, body, -1.0);
    app.update();
    assert!(
        app.world().get::<PipeTransit>(body).is_some(),
        "pressing UP at the return pipe's lip must start a transit"
    );

    let ticks = ((SWALLOW_S + EMERGE_S) / (1.0 / 60.0)).ceil() as usize + 4;
    for _ in 0..ticks {
        app.update();
    }
    assert!(
        app.world().get::<PipeTransit>(body).is_none(),
        "the transit ends and gives the body back"
    );
    assert_eq!(
        app.world().get::<ae::BodyKinematics>(body).unwrap().pos,
        pipe_arrival(),
        "and she surfaces exactly on the ascent tube's own exit pipe"
    );
}

/// A tube authored OUTSIDE 1-1 warps her too — where it was drawn.
///
/// A warp pipe drawn in any other level converted, paired, passed the load-time Entrance/Exit
/// check, drew its prop art — and did nothing at all. The system's own comment promised the
/// opposite, which was true INSIDE 1-1 and false one room along, and nothing in the vocabulary, the
/// validator or the entity docs mentioned the restriction. `mary_o_1_3` shipped with two correct,
/// completely inert pairs.
///
/// the room is DERIVED, not named. This asks the world for the first
/// authored area other than 1-1 that draws a tube, so it covers whichever level
/// learn — which is the same failure, one layer up, that the test would then be.
#[test]
fn a_warp_tube_authored_outside_1_1_still_warps_her() {
    use ambition_demo_mary_o::ldtk_vocabulary::MaryOPipeMouth;
    use ambition_demo_mary_o::pipe::{PipeTransit, EMERGE_S, SWALLOW_S};
    use ambition_demo_mary_o::{authored_area_ids, tubes_for_room, LEVEL_1_1_ROOM_ID};
    use ambition_platformer2d::engine_core::AabbExt;

    let (room, tube) = authored_area_ids()
        .into_iter()
        .filter(|id| id != LEVEL_1_1_ROOM_ID)
        .find_map(|id| tubes_for_room(&id).first().map(|tube| (id, tube)))
        .expect(
            "some authored area other than 1-1 draws a warp tube — with none, \
             this test is a check that cannot fail, so the level that had one \
             losing it must fail HERE rather than pass quietly",
        );

    let mut app = pipe_shell(&room);
    let body = body_standing_at(&mut app, tube.entrance.mouth_band().center());
    app.update();

    // Into the mouth: DOWN into a lip you stand on, UP into one overhead. The
    // authored `mouth` decides, exactly as it does in 1-1.
    press_locomotion(
        &mut app,
        body,
        match tube.entrance.mouth {
            MaryOPipeMouth::Up => 1.0,
            MaryOPipeMouth::Down => -1.0,
        },
    );
    app.update();
    assert!(
        app.world().get::<PipeTransit>(body).is_some(),
        "pressing into room `{room}`'s `{}` tube started nothing. The pipe is \
         authored, paired and validated — and inert, which is what a tube table \
         keyed to one room does to every other room's pipes.",
        tube.link,
    );

    // The whole authored slide, then the arrival — THIS room's paired exit, not
    // a coordinate this test carries.
    let ticks = ((SWALLOW_S + EMERGE_S) / (1.0 / 60.0)).ceil() as usize + 4;
    for _ in 0..ticks {
        app.update();
    }
    assert!(
        app.world().get::<PipeTransit>(body).is_none(),
        "the transit ends and gives the body back"
    );
    assert_eq!(
        app.world().get::<ae::BodyKinematics>(body).unwrap().pos,
        tube.exit.arrival(),
        "and she comes out of the exit `{}` is paired with in room `{room}`",
        tube.link,
    );
}
