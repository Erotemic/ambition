//! Held-item pickup/throw tests: axe melee grant + restore, gun-sword/fireball
//! ranged swap, attack-press consume, and thrown-item gravity settling.

use super::*;
use ambition_characters::actor::attack_gesture::{
    AttackGestureState, AttackGestureTuning, ResolvedAttackGesture,
};
use ambition_combat::moveset::{ActorMoveset, MovePlayback};
use ambition_entity_catalog::{
    ClipBinding, MoveGates, MoveSpec, MoveWindow, MovesetContract, WindowTag,
};
use ambition_input::ControlFrame;
use ambition_platformer2d_core::BodyBaseSize;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use std::collections::BTreeMap;

fn spawn_player(app: &mut App, pos: Vec2) -> Entity {
    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
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
            ambition_characters::control::ActorControl::default(),
            // `fire_held_ranged_system` reads the resolved frame (ADR 0024).
            ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
        ))
        .id();
    // `fire_held_ranged_system` keys on the controlled subject; in tests the
    // spawned player IS the controlled body.
    app.insert_resource(
        ambition_platformer2d_shared_tangle::markers::ControlledSubject(Some(entity)),
    );
    entity
}

/// How many items are lying IN THE WORLD.
///
/// not `query::<&GroundItem>().count()`. A picked-up item keeps its entity now
/// — it records [`ItemCustody::Held`] instead of being despawned — so counting
/// components answers "how many item objects exist", which was never the
/// question these tests were asking.
fn items_in_world(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<(&GroundItem, &ItemCustody)>();
    q.iter(app.world())
        .filter(|(_, custody)| custody.in_world())
        .count()
}

/// Stamp the body-facing semantic control frame directly. Production reaches
/// this state through `SlotControls -> DrivingParticipant -> ActorControl`; pickup and
/// held-item mechanics consume only the body-facing end of that seam.
fn set_control(app: &mut App, player: Entity, attack: bool, shield: bool) {
    let mut control = app
        .world_mut()
        .get_mut::<ambition_characters::control::ActorControl>(player)
        .unwrap();
    control.0.melee_pressed = attack;
    control.0.shield_held = shield;
}

// ---------------------------------------------------------------------------: a held weapon
// owns the Attack press.
//
// still use my normal jab attack. Holding an item should reroute normal attack
// actions to the item action."
//
// The probe drives the REAL chain — pickup → gesture resolution → move trigger →
// the item's own fire system — and asserts BOTH halves of the press: what the
// item did, and what the wearer's own moveset did. A test that watched only the
// bolt would be green in both worlds, because the bolt was never the broken
// half.
// ---------------------------------------------------------------------------

/// One timeline named `id`, gated to `grounded` when the gate is authored.
fn timeline(id: &str, grounded: Option<bool>) -> MoveSpec {
    MoveSpec {
        display_name: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.4,
        events: vec![],
        windows: vec![MoveWindow {
            start_s: 0.1,
            end_s: 0.2,
            tag: WindowTag::Active,
            volumes: vec![],
            sustain_effect: None,
            motion_scale: 1.0,
        }],
        gates: MoveGates {
            grounded,
            ..Default::default()
        },
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
    }
}

/// A wearer whose own repertoire answers Attack on the ground AND in the air.
///
/// the aerial verb is the poison, and it is the direction the ranged
/// precedent (`revoke_host_owned_ranged`) says shipped broken once already: a
/// guard that took `attack` and left `attack_air` gives the jab straight back
/// the moment the wearer leaves the ground.
fn jab_and_air_jab() -> MovesetContract {
    MovesetContract {
        verbs: BTreeMap::from([
            ("attack".to_string(), "jab".to_string()),
            ("attack_air".to_string(), "air_jab".to_string()),
        ]),
        moves: vec![
            timeline("jab", Some(true)),
            timeline("air_jab", Some(false)),
        ],
    }
}

/// Give a spawned body everything the move runtime reads, plus a moveset.
fn with_moveset(app: &mut App, body: Entity, moveset: MovesetContract, on_ground: bool) {
    app.world_mut().entity_mut(body).insert((
        ActorMoveset(moveset),
        AttackGestureState::default(),
        AttackGestureTuning::default(),
        ResolvedAttackGesture::default(),
        ambition_platformer2d_core::BodyGroundState {
            head_contact: false,
            on_ground,
            ..Default::default()
        },
    ));
}

/// Press Attack while holding `spec`, and report BOTH claimants of that press:
/// the number of item bolts fired, and the move the body ended up playing
/// (`None` when nothing did).
fn attack_while_holding(spec: HeldItemSpec, on_ground: bool) -> (usize, Option<MoveSpec>) {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.insert_resource(ControlFrame::default());
    app.add_systems(
        Update,
        (
            pickup_held_item_system,
            throw_held_item_system,
            ambition_combat::moveset::resolve_attack_gestures,
            ambition_combat::moveset::trigger_moveset_moves,
            fire_held_ranged_system,
        )
            .chain(),
    );
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
    with_moveset(&mut app, player, jab_and_air_jab(), on_ground);
    app.world_mut().spawn(GroundItem {
        spec,
        pos: Vec2::new(100.0, 100.0),
        vel: Vec2::ZERO,
        half_extent: Vec2::splat(PICKUP_HALF),
    });

    // Tick 1: the press is spent picking the weapon up (and is consumed, so it
    // cannot also fire it) — the same contract `pickup_consumes_the_attack_press`
    // pins.
    set_control(&mut app, player, true, false);
    app.update();
    assert!(
        app.world().get::<HeldItem>(player).is_some(),
        "the weapon should be in hand before the press under test"
    );

    // Tick 2: a plain Attack while holding it. THIS is the press in question.
    set_control(&mut app, player, true, false);
    app.update();

    let bolts = {
        let mut q = app.world_mut().query::<&HeldProjectile>();
        q.iter(app.world()).count()
    };
    let played = app
        .world()
        .get::<MovePlayback>(player)
        .map(|pb| pb.spec.clone());
    (bolts, played)
}

/// The move id a body ended up playing, for a legible failure message.
fn played_id(played: &Option<MoveSpec>) -> Option<&str> {
    played.as_ref().map(|m| m.id.as_str())
}

/// A plain Attack with the gun-sword shoots, and does NOT also jab.
#[test]
fn the_gunsword_owns_the_attack_press_on_the_ground() {
    let (bolts, played) = attack_while_holding(gunsword_spec(), true);
    assert_eq!(bolts, 1, "the gun-sword fires exactly one bolt");
    assert_eq!(
        played_id(&played),
        None,
        "the wearer's own moveset must not ALSO answer a press the held weapon owns"
    );
}

/// The same, AIRBORNE — the direction a base-verb-only guard hands back.
#[test]
fn the_gunsword_owns_the_attack_press_in_the_air() {
    let (bolts, played) = attack_while_holding(gunsword_spec(), false);
    assert_eq!(bolts, 1, "the gun-sword fires in the air too");
    assert_eq!(
        played_id(&played),
        None,
        "an airborne wearer's `attack_air` must not answer the press either"
    );
}

/// A pure throwable: the throw is the item action, and the jab must not ride
/// along with it.
#[test]
fn a_thrown_item_owns_the_attack_press_too() {
    let (_, played) = attack_while_holding(javelin_spec(), true);
    assert_eq!(
        played_id(&played),
        None,
        "throwing the javelin is the whole of that press"
    );
}

/// An item that authors its OWN melee answers with THAT swing, not with the wearer's jab and
/// not with silence.
///
/// asserted on the swing's DAMAGE, not on a move id: the wearer's fixture jab
/// carries no hit volume at all, so a swing that lands the axe's authored 3 is
/// the axe's and could not be the wearer's under any renaming.
#[test]
fn a_melee_item_answers_the_attack_press_with_its_own_swing() {
    let (_, played) = attack_while_holding(axe_spec(), true);
    let swing = played.expect("holding a weapon that swings must still answer Attack");
    let damage: Vec<i32> = swing
        .windows
        .iter()
        .flat_map(|w| w.volumes.iter().map(|v| v.damage))
        .collect();
    assert_eq!(
        damage,
        vec![3],
        "the swing that answered is the AXE's (damage 3), not the wearer's own \
         volume-less jab — it played {:?}",
        swing.id
    );
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
    assert_eq!(
        items_in_world(&mut app),
        0,
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
    assert_eq!(
        items_in_world(&mut app),
        1,
        "the thrown axe should be back on the ground"
    );
}

#[test]
fn gunsword_pickup_swaps_to_ranged_and_attack_fires_a_bolt() {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
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
    // Picking an item up must EAT the body-semantic Attack edge, so the same
    // edge does NOT also fire the just-equipped item this frame.
    let mut app = App::new();
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
            .get::<ambition_characters::control::ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed,
        "pickup must spend the body-semantic attack edge so the same press \
         cannot also fire the just-equipped item (throw/fire/portal gun)"
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
    // A driven body with NO PlayerEntity / PrimaryPlayer — just
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
            ambition_characters::control::ActorControl::default(),
        ))
        .id();
    app.insert_resource(
        ambition_platformer2d_shared_tangle::markers::ControlledSubject(Some(body)),
    );
    app.world_mut().spawn(GroundItem {
        spec: axe_spec(),
        pos: Vec2::new(100.0, 100.0),
        vel: Vec2::ZERO,
        half_extent: Vec2::splat(PICKUP_HALF),
    });
    // Drive an Attack on the body's own semantic control frame.
    app.world_mut()
        .get_mut::<ambition_characters::control::ActorControl>(body)
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
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
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
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
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
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "phys",
            Vec2::new(400.0, 400.0),
            Vec2::new(200.0, 360.0),
            blocks,
        )),
    );
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
    assert_eq!(
        items_in_world(&mut app),
        1,
        "the thrown javelin should be on the ground"
    );
}

/// THE ITEM YOU THROW IS THE ITEM YOU PICKED UP.
///
/// The invariant: a custody change moves an item between world and hand without
/// destroying it, so its identity — the `SimId` an authored ground item is
/// stamped with by the construction executor — survives world → held → world.
///
/// asserted on the ENTITY and the `SimId` together, because either alone is
/// weak: an entity id can be recycled, and a `SimId` can be re-minted with the
/// same spelling. Both surviving means nothing was recreated.
///
/// and it asserts the item was genuinely OUT of the world in between.
///
/// Falsified by restoring the old pair (despawn at pickup, `spawn_room_scoped`
/// at throw): the entity lookup fails outright.
#[test]
fn a_thrown_item_is_the_same_object_that_was_picked_up() {
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    let mut app = App::new();
    app.insert_resource(ControlFrame::default());
    app.add_systems(Update, (pickup_held_item_system, throw_held_item_system));
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
    // An AUTHORED item: the identity the construction executor gives an LDtk
    // ground item, which the old despawn-on-pickup destroyed.
    let authored = SimId::placement("Sandbox:GroundItem-0042");
    let item = app
        .world_mut()
        .spawn((
            GroundItem {
                spec: axe_spec(),
                pos: Vec2::new(100.0, 100.0),
                vel: Vec2::ZERO,
                half_extent: Vec2::splat(PICKUP_HALF),
            },
            authored.clone(),
        ))
        .id();

    set_control(&mut app, player, true, false);
    app.update();
    assert!(
        app.world().get::<HeldItem>(player).is_some(),
        "the axe is in hand"
    );
    assert_eq!(
        items_in_world(&mut app),
        0,
        "a carried axe is not also lying on the floor"
    );
    assert!(
        app.world()
            .get::<ItemCustody>(item)
            .is_some_and(|c| c.held_by(player)),
        "the SAME entity records who is carrying it"
    );

    set_control(&mut app, player, true, true);
    app.update();

    assert!(
        app.world().get::<HeldItem>(player).is_none(),
        "the throw empties the hand"
    );
    let custody = app
        .world()
        .get::<ItemCustody>(item)
        .expect("the thrown axe is the SAME entity, not a replacement");
    assert!(custody.in_world(), "and it is back in the world");
    assert_eq!(
        app.world().get::<SimId>(item),
        Some(&authored),
        "its authored identity survived the round trip"
    );
    assert_eq!(
        items_in_world(&mut app),
        1,
        "exactly one axe exists — the throw did not mint a second",
    );
}

/// ⭐ THE Z-DROP: `Grab` while holding lets the item go WHERE THE BODY STANDS,
/// with nothing added.
///
/// ⭐ THE ASSERTION IS A CONTRAST, because "the item is in the world with some
/// position" is true of a throw as well. The same fixture released by
/// `Shield + Attack` puts the axe AHEAD of the body and moving; released by
/// `Grab` it is at the body's own position and at rest. A test that only
/// checked custody would be green against a drop that secretly threw.
///
/// and the Grab press is SPENT here, for the reason the throw spends Attack: the
/// hand empties in `PlayerSimulation` and a later reader would find an empty
/// hand and hand the same press to a grab attempt.
#[test]
fn a_grab_press_while_holding_drops_the_item_where_the_body_stands() {
    const STANDING: Vec2 = Vec2::new(100.0, 100.0);

    let release = |grab: bool| {
        let mut app = App::new();
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, (pickup_held_item_system, throw_held_item_system));
        let player = spawn_player(&mut app, STANDING);
        let item = app
            .world_mut()
            .spawn(GroundItem {
                spec: axe_spec(),
                pos: STANDING,
                vel: Vec2::ZERO,
                half_extent: Vec2::splat(PICKUP_HALF),
            })
            .id();

        set_control(&mut app, player, true, false);
        app.update();
        assert!(
            app.world().get::<HeldItem>(player).is_some(),
            "the axe is in hand before anything is released"
        );

        if grab {
            set_control(&mut app, player, false, false);
            app.world_mut()
                .get_mut::<ambition_characters::control::ActorControl>(player)
                .unwrap()
                .0
                .grab_pressed = true;
        } else {
            set_control(&mut app, player, true, true);
        }
        app.update();

        assert!(
            app.world().get::<HeldItem>(player).is_none(),
            "both roads empty the hand — the comparison below is about WHERE the \
             item went, not whether it was let go"
        );
        let ground = app
            .world()
            .get::<GroundItem>(item)
            .expect("the released axe is the same entity")
            .clone();
        let spent = !app
            .world()
            .get::<ambition_characters::control::ActorControl>(player)
            .unwrap()
            .0
            .grab_pressed;
        (ground.pos, ground.vel, spent)
    };

    let (thrown_pos, thrown_vel, _) = release(false);
    assert!(
        thrown_pos.x > STANDING.x && thrown_vel.length() > 0.0,
        "the throw road stopped putting the axe ahead of the body and moving, so \
         the contrast below proves nothing: {thrown_pos:?} {thrown_vel:?}"
    );

    let (dropped_pos, dropped_vel, spent) = release(true);
    assert_eq!(
        dropped_pos, STANDING,
        "a Z-drop leaves the axe where the body was standing, not ahead of it"
    );
    assert_eq!(
        dropped_vel,
        Vec2::ZERO,
        "and at rest — a drop is not a weak throw"
    );
    assert!(
        spent,
        "the Grab press is spent where the drop commits, or the same press also \
         starts a grab one phase later"
    );
}

/// The one case that legitimately MINTS an object: a body holding an item with
/// no world instance behind it (the inventory menu equips straight out of the
/// `OwnedItems` count table) throws a fresh instance, and that instance takes a
/// `SimId::spawned` under its thrower rather than joining the world anonymously.
///
/// this pins the boundary of the unclosed inventory leg, not a feature: the
/// mint exists because a quantity has no identity to hand back.
#[test]
fn throwing_a_menu_equipped_item_mints_an_identity_under_the_thrower() {

    let mut app = App::new();
    app.insert_resource(ControlFrame::default());
    app.add_systems(Update, throw_held_item_system);
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
    // Equipped with no `GroundItem` anywhere — the menu's shape.
    app.world_mut()
        .entity_mut(player)
        .insert((HeldItem::new(axe_spec()), SimId::player_slot(0)));

    set_control(&mut app, player, true, true);
    app.update();

    let minted: Vec<SimId> = {
        let mut q = app.world_mut().query::<(&GroundItem, &SimId)>();
        q.iter(app.world()).map(|(_, id)| id.clone()).collect()
    };
    assert_eq!(
        minted,
        vec![SimId::spawned(&SimId::player_slot(0), 0)],
        "a materialized instance is named under the body that materialized it",
    );
}

/// The production pickup plugin must register custody release for stow/equip
/// so a held authored item can round-trip through inventory without losing its
/// identity. The schedule is initialized explicitly before enumerating systems.
#[test]
fn the_production_plugin_registers_the_custody_release() {
    let mut app = App::new();
    app.add_plugins(super::ItemPickupSimulationPlugin);
    let label = {
        use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
        app.sim_schedule()
    };
    // Initialize the schedule against the app's world before enumeration.
    let names: Vec<String> = app
        .world_mut()
        .resource_scope::<bevy::ecs::schedule::Schedules, Vec<String>>(|world, mut schedules| {
            let schedule = schedules
                .get_mut(label)
                .expect("the plugin added systems to the sim schedule, so it exists");
            schedule.initialize(world).expect("the sim schedule builds");
            schedule
                .systems()
                .map(|systems| systems.map(|(_, s)| format!("{}", s.name())).collect())
                .unwrap_or_default()
        });
    assert!(
        !names.is_empty(),
        "the sim schedule enumerated NO systems, so the assertion below could \
         only ever fail — this measures the enumeration, not the registration"
    );
    assert!(
        names
            .iter()
            .any(|name| name.contains("return_released_items")),
        "`return_released_items` is not in the production sim schedule. The \
         behaviour test below lists it in a chain of its own, so it stays GREEN \
         when the registration is deleted — which is exactly how an authored axe \
         goes back to ceasing to exist through the menu. Registered systems: {}",
        names.len()
    );
}

/// Falsified by dropping `return_released_items` from the schedule below: the
/// stowed axe stays `Held` by an empty hand, `items_in_world` reports 0, and the
/// second Attack finds nothing to grab.
#[test]
fn an_item_stowed_from_the_menu_returns_to_the_world_and_can_be_taken_again() {

    /// The menu's Stow, reduced to the one production call it makes. Driven off a
    /// flag so the test can place it on a specific tick.
    #[derive(Resource, Default)]
    struct StowRequested(bool);

    fn stow_from_menu(
        mut commands: Commands,
        mut requested: ResMut<StowRequested>,
        mut bodies: Query<(Entity, &mut ActionSet, Option<&StashedActionSet>)>,
    ) {
        if !requested.0 {
            return;
        }
        requested.0 = false;
        for (body, mut action_set, stashed) in &mut bodies {
            // The menu passes no catalog in this fixture; `None` is its "no
            // inventory behind this body" case, not "skip the bookkeeping".
            unequip_held(&mut commands, body, &mut action_set, stashed, None);
        }
    }

    let mut app = App::new();
    app.insert_resource(ControlFrame::default());
    app.init_resource::<StowRequested>();
    app.add_systems(
        Update,
        (
            return_released_items,
            pickup_held_item_system,
            throw_held_item_system,
            // Last, so a stow requested on tick N is observed by the release on
            // tick N+1 — the schedule relationship production has, where the menu
            // runs in `Update` and this chain runs in the sim.
            stow_from_menu,
        )
            .chain(),
    );
    let player = spawn_player(&mut app, Vec2::new(100.0, 100.0));
    let authored = SimId::placement("Sandbox:GroundItem-0042");
    let item = app
        .world_mut()
        .spawn((
            GroundItem {
                spec: axe_spec(),
                pos: Vec2::new(100.0, 100.0),
                vel: Vec2::ZERO,
                half_extent: Vec2::splat(PICKUP_HALF),
            },
            authored.clone(),
        ))
        .id();

    // Take it off the floor.
    set_control(&mut app, player, true, false);
    app.update();
    assert!(
        app.world().get::<HeldItem>(player).is_some(),
        "the axe is in hand"
    );

    // Open the menu and Stow. The press is released — a stow is not an attack.
    set_control(&mut app, player, false, false);
    app.world_mut().resource_mut::<StowRequested>().0 = true;
    app.update();
    // …and the tick after, with the hand settled empty.
    app.update();

    let custody = app
        .world()
        .get::<ItemCustody>(item)
        .expect("the stowed axe is the SAME entity, not a replacement");
    assert!(
        custody.in_world(),
        "a body that let go left the axe IN THE WORLD — it used to keep recording \
         a hand that was already empty, which is neither state this enum has",
    );
    assert_eq!(
        items_in_world(&mut app),
        1,
        "and the world can see it: exactly one axe is lying there",
    );
    assert_eq!(
        app.world().get::<GroundItem>(item).map(|g| g.pos),
        Some(Vec2::new(100.0, 100.0)),
        "dropped where the body that released it was standing",
    );

    // THE HALF THAT MATTERS: take it again, and it is the same object.
    set_control(&mut app, player, true, false);
    app.update();
    assert!(
        app.world().get::<HeldItem>(player).is_some(),
        "the stowed axe can be picked back up — it could not while it was orphaned",
    );
    assert!(
        app.world()
            .get::<ItemCustody>(item)
            .is_some_and(|c| c.held_by(player)),
        "and it is the SAME entity that is back in the hand",
    );
    assert_eq!(
        app.world().get::<SimId>(item),
        Some(&authored),
        "with the identity it was authored with — a stow does not mint a replacement",
    );
}

// ---------------------------------------------------------------------------
// RESIDENCY FOLLOWS CUSTODY — see `project_custody_onto_residency`.
//
// The behavioural proof (an authored item carried through a REAL room
// transition) is `game/ambition_app/tests/carried_item_crosses_rooms.rs`. These
// pin the projection's three answers, including the two the app test cannot
// reach: a room-fixture holder, and a holder that no longer exists.

/// A ground item, room-scoped exactly as construction spawns one.
fn room_scoped_item(app: &mut App, pos: Vec2) -> Entity {
    app.world_mut()
        .spawn((
            GroundItem {
                spec: axe_spec(),
                pos,
                vel: Vec2::ZERO,
                half_extent: Vec2::splat(18.0),
            },
            ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity,
        ))
        .id()
}

fn residency_app() -> App {
    let mut app = App::new();
    app.add_systems(Update, project_custody_onto_residency);
    app
}

fn is_resident(app: &App, item: Entity) -> bool {
    app.world()
        .get::<ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf>(item)
        .is_none()
}

/// The invariant, both terms. An object in a travelling body's custody is
/// NOT a resident of the room, and the moment custody returns it to the world it
/// IS one again — while never losing the room SCOPE that a reset sweeps on.
#[test]
fn custody_suspends_and_restores_room_residency() {
    let mut app = residency_app();
    // A travelling body: the home avatar carries no room scope (possession
    // promotes a body it takes over into the same state).
    let carrier = app.world_mut().spawn_empty().id();
    let item = room_scoped_item(&mut app, Vec2::ZERO);

    app.update();
    assert!(
        is_resident(&app, item),
        "an item lying in the room is a resident of it"
    );

    *app.world_mut().get_mut::<ItemCustody>(item).unwrap() = ItemCustody::Held { holder: carrier };
    app.update();
    assert!(
        !is_resident(&app, item),
        "an object in a travelling body's custody is not resident in the room it \
         was picked up in — this is what stops the room it leaves from retiring it",
    );
    assert!(
        app.world()
            .get::<ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity>(item)
            .is_some(),
        "and it KEEPS the room scope: residency is suspended, the lifetime is not \
         retracted, so a sandbox reset still destroys it",
    );

    *app.world_mut().get_mut::<ItemCustody>(item).unwrap() = ItemCustody::InWorld;
    app.update();
    assert!(
        is_resident(&app, item),
        "dropped back into the world it is a resident again — of whatever room is \
         active now, because room residency carries no room id",
    );
}

/// A holder that is room-scoped AND ITSELF IN CUSTODY is travelling.
///
/// this is the case possession creates: a possessed body keeps
/// `RoomScopedEntity` and suspends its own residency with `InCustodyOf`, so the
/// thing in ITS hand must travel too. The projection asks the `RoomResident`
/// roster rather than `Has<RoomScopedEntity>` precisely so custody is
/// TRANSITIVE — a chain of any length resolves without this system counting its
/// links.
#[test]
fn an_item_held_by_a_possessed_body_travels_with_it() {
    let mut app = residency_app();
    let participant = app.world_mut().spawn_empty().id();
    // A possessed body: room-scoped, and NOT resident because a participant has
    // custody of it — exactly what `possess_target` produces.
    let possessed = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity,
            ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf(participant),
        ))
        .id();
    let item = room_scoped_item(&mut app, Vec2::ZERO);

    app.update();
    assert!(
        is_resident(&app, item),
        "setup: the item starts resident, so the assertion below is a CHANGE"
    );

    *app.world_mut().get_mut::<ItemCustody>(item).unwrap() =
        ItemCustody::Held { holder: possessed };
    app.update();
    assert!(
        !is_resident(&app, item),
        "an object held by a POSSESSED body must travel with it. The holder is \
         room-scoped, so a `Has<RoomScopedEntity>` question calls it a room \
         fixture; the `RoomResident` roster knows better, because the holder's own \
         residency is suspended by the custody a participant has of it"
    );
}

/// A ROOM FIXTURE's hand is still the room. An unpossessed NPC carries
/// `RoomScopedEntity`; the object it holds dies with the room exactly as the NPC
/// does. Nothing here asks whether a holder is the player — it asks where the
/// holder lives.
#[test]
fn an_item_held_by_a_room_fixture_stays_resident() {
    let mut app = residency_app();
    let npc = app
        .world_mut()
        .spawn(ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity)
        .id();
    let item = room_scoped_item(&mut app, Vec2::ZERO);
    *app.world_mut().get_mut::<ItemCustody>(item).unwrap() = ItemCustody::Held { holder: npc };

    app.update();

    assert!(
        is_resident(&app, item),
        "an object in the custody of a body that is itself a fixture of this room \
         is still the room's to retire",
    );
}

/// A holder that no longer EXISTS confers no residency. `ItemCustody` keeps
/// naming a dead body on purpose (the death drop owns that question), and an
/// orphan that also escaped every room sweep would leak for the rest of the
/// process — the exact hazard of expressing "not here" by removing a scope.
#[test]
fn an_item_whose_holder_is_gone_is_the_rooms_again() {
    let mut app = residency_app();
    let carrier = app.world_mut().spawn_empty().id();
    let item = room_scoped_item(&mut app, Vec2::ZERO);
    *app.world_mut().get_mut::<ItemCustody>(item).unwrap() = ItemCustody::Held { holder: carrier };
    app.update();
    assert!(!is_resident(&app, item), "carried while the holder lives");

    app.world_mut().entity_mut(carrier).despawn();
    app.update();

    assert!(
        is_resident(&app, item),
        "the holder is gone, so the object is a thing in the room again and the \
         room can retire it — it does not outlive every sweep in the engine",
    );
}

/// AN ITEM WHOSE SUPPORT LEAVES FALLS.
///
/// ⛔⛔ SETTLING WAS PERMANENT. `ground_item_physics` skips `Without<SettledItem>`
/// and the marker was only lifted by CUSTODY transitions, so an item that landed
/// on a MOVING PLATFORM — which the composited collision world deliberately lets
/// it do — stayed fixed in WORLD SPACE once the platform moved on. A platform
/// that disappears leaves the same hovering item.
///
/// ⚠ THIS PINS THE SAFE MINIMUM, and says so: an unsupported item FALLS. It does
/// not RIDE a support that is still there — that needs support identity, which
/// the ledger records as the endpoint.
#[test]
fn a_settled_item_wakes_when_its_support_goes_away() {
    let mut app = App::new();
    let world_with = |blocks: Vec<ae::Block>| {
        ambition_platformer2d_core::RoomGeometry(ae::World::new(
            "phys",
            Vec2::new(400.0, 400.0),
            Vec2::new(200.0, 360.0),
            blocks,
        ))
    };
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        world_with(vec![ae::Block::solid(
            "ledge",
            Vec2::new(0.0, 380.0),
            Vec2::new(400.0, 20.0),
        )]),
    );
    app.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    app.add_systems(
        Update,
        (carry_or_wake_settled_items, ground_item_physics).chain(),
    );

    let item = app
        .world_mut()
        .spawn(GroundItem {
            spec: axe_spec(),
            pos: Vec2::new(200.0, 300.0),
            vel: Vec2::new(0.0, 0.0),
            half_extent: Vec2::splat(PICKUP_HALF),
        })
        .id();
    for _ in 0..120 {
        app.update();
    }
    assert!(
        app.world().get::<SettledItem>(item).is_some(),
        "the item never settled on the floor, so nothing below is about waking"
    );
    let resting = app.world().get::<GroundItem>(item).unwrap().pos.y;

    // ⚠ NO ARM HERE FOR "A SUPPORTED ITEM STAYS SETTLED", AND THAT IS MEASURED
    // RATHER THAN LAZY. Poisoning the support check to wake EVERY settled item
    // each tick leaves this fixture green: `ground_item_physics` re-settles in
    // the same tick and, because it detects the block before moving anything,
    // the item does not drift by so much as a thousandth of a pixel. The two
    // implementations are the SAME PROGRAM along every path this world can take.
    //
    // ⇒ The support check earns its place on semantics and per-tick churn, not
    // on observable behaviour, and an assertion claiming otherwise would be a
    // check that cannot fail.

    // THE SUPPORT LEAVES. A moving platform that travels on, or one that is
    // removed outright — the composited world simply stops holding a block here.
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        world_with(Vec::new()),
    );
    for _ in 0..60 {
        app.update();
    }

    let fell = app.world().get::<GroundItem>(item).unwrap().pos.y - resting;
    assert!(
        fell > 10.0,
        "the item stayed put ({fell:.1}px) after its support went away — settling \
         is permanent, so an item caught by a moving platform hangs in world \
         space once that platform leaves"
    );
}

/// ⭐⭐ AN ITEM ON A MOVING PLATFORM GOES WITH IT.
///
/// Settling used to be world-fixed: an item that landed on a platform stayed
/// where it was while the platform slid out from under it, and only woke once
/// there was nothing left beneath the OLD position.
///
/// ⛔ NO SUPPORT IDENTITY AND NO LOCAL OFFSET WAS NEEDED. `Block::velocity` is
/// the block's own per-frame displacement, and its doc already says the sweep
/// carries *"any body resting on the block"* by it; the support probe already
/// finds the block. ⇒ the fact was at the site.
///
/// ⭐ THE STATIC ARM IS NOT DECORATION. Carrying by a block's `velocity`
/// unconditionally would drift every settled item in the game by whatever the
/// floor happens to hold, so the identity case is the one that says this reads
/// the platform rather than the clock.
#[test]
fn a_settled_item_rides_the_platform_it_landed_on() {
    let carried_by = |platform_velocity: Vec2| -> f32 {
        let mut app = App::new();
        let mut ledge = ae::Block::solid("ledge", Vec2::new(0.0, 380.0), Vec2::new(400.0, 20.0));
        ledge.velocity = platform_velocity;
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            ambition_platformer2d_core::RoomGeometry(ae::World::new(
                "phys",
                Vec2::new(400.0, 400.0),
                Vec2::new(200.0, 360.0),
                vec![ledge],
            )),
        );
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.add_systems(
            Update,
            (carry_or_wake_settled_items, ground_item_physics).chain(),
        );
        let item = app
            .world_mut()
            .spawn(GroundItem {
                spec: axe_spec(),
                pos: Vec2::new(200.0, 300.0),
                vel: Vec2::ZERO,
                half_extent: Vec2::splat(PICKUP_HALF),
            })
            .id();
        // Let it fall and settle first: an item still in flight is carried by
        // nothing, and this is about what settling means.
        for _ in 0..120 {
            app.update();
        }
        assert!(
            app.world().get::<SettledItem>(item).is_some(),
            "the item never settled, so this arm is not about riding at all"
        );
        let settled_at = app.world().get::<GroundItem>(item).unwrap().pos.x;
        for _ in 0..10 {
            app.update();
        }
        app.world().get::<GroundItem>(item).unwrap().pos.x - settled_at
    };

    assert_eq!(
        carried_by(Vec2::ZERO),
        0.0,
        "a settled item drifted on STATIC ground, so the arm below is measuring \
         the tick rather than the platform"
    );
    let ridden = carried_by(Vec2::new(2.0, 0.0));
    assert!(
        (ridden - 20.0).abs() < 0.01,
        "ten ticks of a platform moving 2px each left the item {ridden:.2}px \
         along — settling is still world-fixed, so the platform slides out from \
         under whatever lands on it"
    );
}
