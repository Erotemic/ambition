//! Held-item pickup/throw tests: axe melee grant + restore, gun-sword/fireball
//! ranged swap, attack-press consume, and thrown-item gravity settling.

use super::*;
use crate::actor::BodyBaseSize;
use crate::actor::{PlayerEntity, PrimaryPlayer};

fn spawn_player(app: &mut App, pos: Vec2) -> Entity {
    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            PlayerInputFrame::default(),
            BodyKinematics {
                pos,
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            ActionSet::default(),
            ambition_characters::brain::ActorControl::default(),
        ))
        .id();
    // `fire_held_ranged_system` keys on the controlled subject; in tests the
    // spawned player IS the controlled body.
    app.insert_resource(ambition_platformer_primitives::markers::ControlledSubject(
        Some(entity),
    ));
    entity
}

/// Stamp the input onto BOTH the actor-local `PlayerInputFrame` (read by
/// pickup/throw) and the `ActorControl` brain frame (read by the now
/// subject-generic `fire_held_ranged_system`). In production
/// `sync_local_player_input_frame` + `tick_player_brains` keep these coherent
/// from the one `ControlFrame`; here we set both directly.
fn set_control(app: &mut App, player: Entity, attack: bool, shield: bool) {
    {
        let mut input = app.world_mut().get_mut::<PlayerInputFrame>(player).unwrap();
        input.frame.attack_pressed = attack;
        input.frame.shield_held = shield;
    }
    let mut control = app
        .world_mut()
        .get_mut::<ambition_characters::brain::ActorControl>(player)
        .unwrap();
    control.0.melee_pressed = attack;
    control.0.shield_held = shield;
}

#[test]
fn attack_picks_up_axe_and_grants_its_swing_then_throw_restores() {
    let mut app = App::new();
    app.insert_resource(ControlFrame::default());
    app.add_systems(Update, (pickup_held_item_system, throw_held_item_system));
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
    // An axe on the ground, overlapping the player.
    app.world_mut().spawn(GroundItem {
        spec: axe_spec(),
        pos: Vec2::new(100.0, 100.0),
        vel: Vec2::ZERO,
        half_extent: Vec2::splat(PICKUP_HALF),
    });
    // Player starts with no melee.
    assert!(app
        .world()
        .get::<ActionSet>(player)
        .unwrap()
        .melee
        .is_none());

    // Attack (no shield) → pick up the axe.
    set_control(&mut app, player, true, false);
    app.update();
    assert!(
        app.world().get::<HeldItem>(player).is_some(),
        "player should be holding the axe"
    );
    assert!(
        app.world()
            .get::<ActionSet>(player)
            .unwrap()
            .melee
            .is_some(),
        "the axe should grant its melee swing"
    );
    let remaining_ground = {
        let mut q = app.world_mut().query::<&GroundItem>();
        q.iter(app.world()).count()
    };
    assert_eq!(
        remaining_ground, 0,
        "the picked-up axe should leave the ground"
    );

    // Shield + Attack → throw it back onto the ground.
    set_control(&mut app, player, true, true);
    app.update();
    assert!(
        app.world().get::<HeldItem>(player).is_none(),
        "throwing should empty the player's hands"
    );
    assert!(
        app.world()
            .get::<ActionSet>(player)
            .unwrap()
            .melee
            .is_none(),
        "throwing should restore the original (empty) action set"
    );
    let thrown = {
        let mut q = app.world_mut().query::<&GroundItem>();
        q.iter(app.world()).count()
    };
    assert_eq!(thrown, 1, "the thrown axe should be back on the ground");
}

#[test]
fn gunsword_pickup_swaps_to_ranged_and_attack_fires_a_bolt() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.insert_resource(ControlFrame::default());
    app.add_systems(Update, (pickup_held_item_system, fire_held_ranged_system));
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
    // Give the player a default melee swing so we can see it get cleared.
    app.world_mut().get_mut::<ActionSet>(player).unwrap().melee =
        Some(MeleeActionSpec::Swipe(SwipeSpec {
            windup_s: 0.1,
            active_s: 0.1,
            recover_s: 0.1,
            damage: 1,
            reach_px: 32.0,
        }));
    app.world_mut().spawn(GroundItem {
        spec: gunsword_spec(),
        pos: Vec2::new(100.0, 100.0),
        vel: Vec2::ZERO,
        half_extent: Vec2::splat(PICKUP_HALF),
    });

    // Attack picks up the gun-sword (commands flush after the tick, so the
    // fire system can't also fire on this same press).
    set_control(&mut app, player, true, false);
    app.update();
    let actions = app.world().get::<ActionSet>(player).unwrap();
    assert!(
        actions.melee.is_none(),
        "the gun-sword should REPLACE (clear) the player's melee swing"
    );
    assert!(
        actions.ranged.is_some(),
        "the gun-sword should grant its ranged bolt"
    );

    // A second Attack while holding it fires exactly one laser bolt.
    set_control(&mut app, player, true, false);
    app.update();
    let bolts = {
        let mut q = app.world_mut().query::<&HeldProjectile>();
        q.iter(app.world()).count()
    };
    assert_eq!(
        bolts, 1,
        "Attack while holding the gun-sword fires one laser bolt"
    );
}

#[test]
fn pickup_consumes_the_attack_press() {
    // Picking an item up must EAT the Attack press, so the same press does
    // NOT also fire the just-equipped item this frame (the gauntlet/weapon
    // attack systems all gate on `ControlFrame::attack_pressed`).
    let mut app = App::new();
    app.insert_resource(ControlFrame::default());
    app.add_systems(Update, pickup_held_item_system);
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
    app.world_mut().spawn(GroundItem {
        spec: gunsword_spec(),
        pos: Vec2::new(100.0, 100.0),
        vel: Vec2::ZERO,
        half_extent: Vec2::splat(PICKUP_HALF),
    });
    set_control(&mut app, player, true, false);
    app.update();
    assert!(
        app.world().get::<HeldItem>(player).is_some(),
        "the item should be picked up"
    );
    assert!(
        !app.world()
            .get::<PlayerInputFrame>(player)
            .unwrap()
            .frame
            .attack_pressed,
        "pickup must clear the actor-local attack_pressed so the same press \
         doesn't also fire the just-equipped item (throw/fire/portal gun)"
    );
}

/// Fork E — pickup/throw are SUBJECT-generic: they act on the `ControlledSubject`
/// (the body you drive), not a `PrimaryPlayer` marker. A controlled body carrying
/// NEITHER `PlayerEntity` NOR `PrimaryPlayer` (the shape a possessed actor takes)
/// still picks the item up and OWNS it. Pins "inventory ownership is explicit
/// (the controlled body), not accidental primary-player".
#[test]
fn pickup_targets_the_controlled_subject_not_a_primary_player_marker() {
    let mut app = App::new();
    app.insert_resource(ControlFrame::default());
    app.add_systems(Update, (pickup_held_item_system, throw_held_item_system));
    // A driven body with NO PlayerEntity / PrimaryPlayer / PlayerInputFrame — just
    // the body-generic control + kinematics + action set.
    let body = app
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: Vec2::new(100.0, 100.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            ActionSet::default(),
            ambition_characters::brain::ActorControl::default(),
        ))
        .id();
    app.insert_resource(ambition_platformer_primitives::markers::ControlledSubject(
        Some(body),
    ));
    app.world_mut().spawn(GroundItem {
        spec: axe_spec(),
        pos: Vec2::new(100.0, 100.0),
        vel: Vec2::ZERO,
        half_extent: Vec2::splat(PICKUP_HALF),
    });
    // Drive an Attack on the body's OWN control frame (not a PlayerInputFrame).
    app.world_mut()
        .get_mut::<ambition_characters::brain::ActorControl>(body)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    assert!(
        app.world().get::<HeldItem>(body).is_some(),
        "the controlled body (no PrimaryPlayer marker) picks the item up and owns it"
    );
    assert!(
        app.world().get::<ActionSet>(body).unwrap().melee.is_some(),
        "the axe grants its swing to the controlled body"
    );
}

#[test]
fn fireball_shot_is_tagged_to_explode_unlike_a_plain_bolt() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.insert_resource(ControlFrame::default());
    app.add_systems(Update, fire_held_ranged_system);
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
    let spec = ambition_characters::brain::held_item_by_id(FIREBALL_ID).unwrap();
    app.world_mut()
        .entity_mut(player)
        .insert(HeldItem::new(spec));
    set_control(&mut app, player, true, false);
    app.update();
    let halves: Vec<f32> = {
        let mut q = app.world_mut().query::<&HeldProjectile>();
        q.iter(app.world()).map(|p| p.explode_half).collect()
    };
    assert_eq!(halves.len(), 1, "Attack fires one fireball");
    assert_eq!(
        halves[0], FIREBALL_EXPLODE_HALF,
        "the fireball shot is tagged to explode on contact"
    );
}

#[test]
fn shot_collision_geometry_is_a_single_source_of_truth() {
    // The contact box (what hits) and splash box (Fireball AOE) are the
    // exact geometry the debug overlay draws, so the drawn box can't drift
    // from the box that registers a hit — the original "fireball hits
    // gnuton before it touches the visible box" report.
    let pos = Vec2::new(50.0, 20.0);
    let bolt = HeldProjectile {
        damage: 3,
        traveled: 0.0,
        explode_half: 0.0,
    };
    assert_eq!(
        HeldProjectile::contact_aabb(pos),
        ae::Aabb::new(pos, HELD_SHOT_HALF)
    );
    assert!(
        bolt.splash_aabb(pos).is_none(),
        "a plain bolt has no splash AOE to draw"
    );

    let fireball = HeldProjectile {
        explode_half: FIREBALL_EXPLODE_HALF,
        ..bolt
    };
    assert_eq!(
        fireball.splash_aabb(pos),
        Some(ae::Aabb::new(pos, Vec2::splat(FIREBALL_EXPLODE_HALF))),
        "a fireball's splash box is centered on the shot at its explode half-extent"
    );
}

#[test]
fn a_plain_ranged_bolt_does_not_explode() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::SfxMessage>();
    app.insert_resource(ControlFrame::default());
    app.add_systems(Update, fire_held_ranged_system);
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
    app.world_mut()
        .entity_mut(player)
        .insert(HeldItem::new(gunsword_spec()));
    set_control(&mut app, player, true, false);
    app.update();
    let half = {
        let mut q = app.world_mut().query::<&HeldProjectile>();
        q.iter(app.world()).next().map(|p| p.explode_half)
    };
    assert_eq!(half, Some(0.0), "the gun-sword bolt does not explode");
}

#[test]
fn thrown_item_arcs_and_settles_on_the_floor() {
    let mut app = App::new();
    let blocks = vec![ae::Block::solid(
        "floor",
        Vec2::new(0.0, 380.0),
        Vec2::new(400.0, 20.0),
    )];
    app.insert_resource(ambition_engine_core::RoomGeometry(ae::World::new(
        "phys",
        Vec2::new(400.0, 400.0),
        Vec2::new(200.0, 360.0),
        blocks,
    )));
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    app.add_systems(Update, ground_item_physics);
    let item = app
        .world_mut()
        .spawn(GroundItem {
            spec: axe_spec(),
            pos: Vec2::new(200.0, 200.0),
            vel: Vec2::new(120.0, -200.0), // forward + up
            half_extent: Vec2::splat(PICKUP_HALF),
        })
        .id();
    for _ in 0..120 {
        app.update();
    }
    let g = app.world().get::<GroundItem>(item).unwrap();
    assert_eq!(
        g.vel,
        Vec2::ZERO,
        "thrown item should settle, vel={:?}",
        g.vel
    );
    assert!(
        g.pos.y < 380.0 && g.pos.y > 300.0 && g.pos.x > 200.0,
        "settled near the floor and moved forward, pos={:?}",
        g.pos
    );
}

#[test]
fn javelin_is_thrown_on_plain_attack_use() {
    let mut app = App::new();
    app.insert_resource(ControlFrame::default());
    app.add_systems(Update, (pickup_held_item_system, throw_held_item_system));
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
    app.world_mut().spawn(GroundItem {
        spec: javelin_spec(),
        pos: Vec2::new(100.0, 100.0),
        vel: Vec2::ZERO,
        half_extent: Vec2::splat(PICKUP_HALF),
    });

    // First Attack picks up the javelin (commands flush after the tick, so
    // the throw system can't also fire this frame).
    set_control(&mut app, player, true, false);
    app.update();
    assert!(
        app.world().get::<HeldItem>(player).is_some(),
        "javelin should be picked up first"
    );

    // A second plain Attack (no shield) *uses* the javelin — which throws
    // it, since it has no melee/ranged verb of its own.
    set_control(&mut app, player, true, false);
    app.update();
    assert!(
        app.world().get::<HeldItem>(player).is_none(),
        "using the javelin should throw it and empty the hands"
    );
    let on_ground = {
        let mut q = app.world_mut().query::<&GroundItem>();
        q.iter(app.world()).count()
    };
    assert_eq!(on_ground, 1, "the thrown javelin should be on the ground");
}

#[test]
fn held_shot_aim_resolves_screen_input_through_the_controlled_body_frame() {
    let mut control = ControlFrame::default();
    control.aim_x = 0.0;
    control.aim_y = -1.0; // screen/world up on the right stick
    let down = ae::Vec2::new(0.0, 1.0);
    let left = ae::Vec2::new(-1.0, 0.0);

    let down_frame = ae::AccelerationFrame::new(down);
    let left_frame = ae::AccelerationFrame::new(left);
    let screen = ae::ControlFrameModes {
        movement: ae::InputFrameMode::ScreenRelative,
        aim: ae::InputFrameMode::ScreenRelative,
    };
    let down_local = held_shot_aim_local(&control, 1.0, down_frame, screen);
    let left_local = held_shot_aim_local(&control, 1.0, left_frame, screen);

    assert_eq!(down_local, ae::Vec2::new(0.0, -1.0));
    assert_eq!(left_local, ae::Vec2::new(-1.0, 0.0));
    assert_eq!(down_frame.to_world(down_local), ae::Vec2::new(0.0, -1.0));
    assert_eq!(left_frame.to_world(left_local), ae::Vec2::new(0.0, -1.0));
}
