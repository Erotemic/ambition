//! Multi-player smoke tests: spawn two player entities and assert their
//! per-player components (attacks, safety anchors, input frames) and the
//! singleton queries / heal routing stay independent and correct.

use super::*;
use crate::actor::BodyMelee;
use crate::actor::PrimaryPlayerOnly;
use crate::player::PlayerSafetyState;
use ambition_engine_core as ae;

fn dummy_attack_spec() -> crate::combat::AttackSpec {
    // Construct via the live `attack_spec` builder; a minimal Player
    // is enough — only the `intent` field is meaningful for these
    // tests, and the builder gives us a well-formed spec with
    // non-zero timings so the `MeleeSwing::done()` path
    // doesn't short-circuit.
    let world = ae::World::new(
        "smoke",
        ae::Vec2::new(1000.0, 1000.0),
        ae::Vec2::new(100.0, 900.0),
        vec![],
    );
    let scratch = crate::player::primary_player_scratch(world.spawn, ae::AbilitySet::sandbox_all());
    let view = crate::combat::AttackView {
        pos: scratch.kinematics.pos,
        size: scratch.kinematics.size,
        facing: scratch.kinematics.facing,
        on_ground: scratch.ground.on_ground,
        wall_clinging: scratch.wall.wall_clinging,
        dash_timer: scratch.dash.timer,
        abilities_directional_primary: scratch.abilities.abilities.directional_primary,
    };
    crate::combat::attack_spec_from_view(&view, crate::combat::AttackIntent::Forward)
}

/// Two player entities each carry their own `BodyMelee`,
/// so a swing on one player does not silently affect the other.
/// Regression guard for the old shared-resource shape — if a
/// future patch turns `BodyMelee` back into a global
/// `Resource`, this test stops being meaningful and should fail
/// loudly when it tries to read two values.
#[test]
fn two_players_have_independent_active_attacks() {
    let mut app = App::new();
    let p1 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PlayerSlot(0),
            PrimaryPlayer,
            BodyMelee::default(),
        ))
        .id();
    let p2 = app
        .world_mut()
        .spawn((PlayerEntity, PlayerSlot(1), BodyMelee::default()))
        .id();

    // Start an attack on player 1 only.
    let attack_spec = dummy_attack_spec();
    app.world_mut()
        .entity_mut(p1)
        .get_mut::<BodyMelee>()
        .expect("p1 has the component")
        .swing = Some(crate::MeleeSwing::new(attack_spec));

    let p1_attack = app
        .world()
        .entity(p1)
        .get::<BodyMelee>()
        .expect("p1 has the component");
    let p2_attack = app
        .world()
        .entity(p2)
        .get::<BodyMelee>()
        .expect("p2 has the component");

    assert!(p1_attack.is_swinging(), "p1 should be mid-attack");
    assert!(
        !p2_attack.is_swinging(),
        "p2's attack must not pick up p1's swing — that's the whole \
             point of moving CurrentPlayerAttack onto the player entity \
             (OVERNIGHT-TODO #17.4)"
    );
}

/// Two players each carry their own `PlayerSafetyState`; updating
/// one player's safe position must not move the other player's
/// anchor (OVERNIGHT-TODO #17.9).
#[test]
fn two_players_have_independent_safety_anchors() {
    let mut app = App::new();
    let p1_initial = ae::Vec2::new(100.0, 100.0);
    let p2_initial = ae::Vec2::new(500.0, 500.0);
    let p1 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PlayerSlot(0),
            PrimaryPlayer,
            PlayerSafetyState::new(p1_initial),
        ))
        .id();
    let p2 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PlayerSlot(1),
            PlayerSafetyState::new(p2_initial),
        ))
        .id();

    app.world_mut()
        .entity_mut(p1)
        .get_mut::<PlayerSafetyState>()
        .unwrap()
        .last_safe_pos = ae::Vec2::new(999.0, 999.0);

    assert_eq!(
        app.world()
            .entity(p1)
            .get::<PlayerSafetyState>()
            .unwrap()
            .last_safe_pos,
        ae::Vec2::new(999.0, 999.0)
    );
    assert_eq!(
        app.world()
            .entity(p2)
            .get::<PlayerSafetyState>()
            .unwrap()
            .last_safe_pos,
        p2_initial,
        "p2's anchor must not pick up p1's update — that's the whole \
             point of moving last_safe_player_pos onto the player entity"
    );
}

/// With two `PlayerEntity` actors spawned, a `Query<...,
/// PrimaryPlayerOnly>` resolves to exactly one entity. Together
/// with the next test (which checks generic `With<PlayerEntity>`
/// queries see both), this pins the invariant the audit calls
/// out: only one player carries the `PrimaryPlayer` marker, so
/// camera/HUD/input systems can keep using `.single()` safely
/// while combat/hazard systems iterate.
#[test]
fn primary_player_query_resolves_with_two_players_spawned() {
    let mut app = App::new();
    app.world_mut()
        .spawn((PlayerEntity, PlayerSlot(0), PrimaryPlayer));
    app.world_mut().spawn((PlayerEntity, PlayerSlot(1)));

    let mut q = app
        .world_mut()
        .query_filtered::<Entity, PrimaryPlayerOnly>();
    let primaries: Vec<Entity> = q.iter(app.world()).collect();
    assert_eq!(
        primaries.len(),
        1,
        "exactly one entity must carry both PlayerEntity and PrimaryPlayer; \
             camera/HUD systems rely on this for `.single()` correctness"
    );
}

/// Generic `With<PlayerEntity>` queries see every spawned player,
/// even the non-primary one. This is the half of the architectural
/// promise that lets hazards/projectiles/pickups iterate over all
/// players in B-bucket systems (audit doc §B).
#[test]
fn player_entity_query_iterates_all_spawned_players() {
    let mut app = App::new();
    app.world_mut()
        .spawn((PlayerEntity, PlayerSlot(0), PrimaryPlayer));
    app.world_mut().spawn((PlayerEntity, PlayerSlot(1)));
    app.world_mut().spawn((PlayerEntity, PlayerSlot(2)));

    let mut q = app
        .world_mut()
        .query_filtered::<&PlayerSlot, With<PlayerEntity>>();
    let mut slots: Vec<u8> = q.iter(app.world()).map(|s| s.0).collect();
    slots.sort_unstable();
    assert_eq!(slots, vec![0, 1, 2]);
}

/// `BodyMelee::clear` zeroes the attack on its own
/// entity without touching sibling players.
#[test]
fn clear_is_per_entity() {
    let mut app = App::new();
    let attack_spec = dummy_attack_spec();
    let p1 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PlayerSlot(0),
            BodyMelee {
                swing: Some(crate::MeleeSwing::new(attack_spec.clone())),
                ..Default::default()
            },
        ))
        .id();
    let p2 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PlayerSlot(1),
            BodyMelee {
                swing: Some(crate::MeleeSwing::new(attack_spec)),
                ..Default::default()
            },
        ))
        .id();

    app.world_mut()
        .entity_mut(p1)
        .get_mut::<BodyMelee>()
        .unwrap()
        .clear();

    assert!(!app
        .world()
        .entity(p1)
        .get::<BodyMelee>()
        .unwrap()
        .is_swinging());
    assert!(
        app.world()
            .entity(p2)
            .get::<BodyMelee>()
            .unwrap()
            .is_swinging(),
        "clearing p1's attack must not touch p2's component"
    );
}

/// A `PlayerHealRequested` carrying `target: Some(p2)` heals p2,
/// not the primary p1. Pins the OVERNIGHT-TODO #17.6 bridge —
/// pickups now route heals to the player who actually overlapped
/// the heart instead of always to primary.
#[test]
fn targeted_heal_routes_to_named_entity_not_primary() {
    use crate::player::{apply_player_heal_requests, PlayerHealRequested};
    use ambition_characters::actor::BodyHealth;

    let mut app = App::new();
    app.add_message::<PlayerHealRequested>();
    app.add_systems(Update, apply_player_heal_requests);

    let p1 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PlayerSlot(0),
            PrimaryPlayer,
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: false,
            }),
        ))
        .id();
    let p2 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PlayerSlot(1),
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: false,
            }),
        ))
        .id();

    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<PlayerHealRequested>>()
        .write(PlayerHealRequested::for_target(2, p2));
    app.update();

    let p1_health = app.world().entity(p1).get::<BodyHealth>().unwrap();
    let p2_health = app.world().entity(p2).get::<BodyHealth>().unwrap();
    assert_eq!(p1_health.current(), 1, "primary must not pick up p2's heal");
    assert_eq!(p2_health.current(), 3, "p2 must be healed by 2");
}

/// `PlayerHealRequested::new` (target = None) keeps legacy
/// behavior: heal lands on the primary player. Pins the
/// backwards-compatible path so cutscene/quest heals don't
/// silently break when other code starts using `for_target`.
#[test]
fn untargeted_heal_routes_to_primary() {
    use crate::player::{apply_player_heal_requests, PlayerHealRequested};
    use ambition_characters::actor::BodyHealth;

    let mut app = App::new();
    app.add_message::<PlayerHealRequested>();
    app.add_systems(Update, apply_player_heal_requests);

    let p1 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PlayerSlot(0),
            PrimaryPlayer,
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: false,
            }),
        ))
        .id();
    let p2 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PlayerSlot(1),
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: false,
            }),
        ))
        .id();

    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<PlayerHealRequested>>()
        .write(PlayerHealRequested::new(3));
    app.update();

    let p1_health = app.world().entity(p1).get::<BodyHealth>().unwrap();
    let p2_health = app.world().entity(p2).get::<BodyHealth>().unwrap();
    assert_eq!(p1_health.current(), 4, "primary picks up untargeted heal");
    assert_eq!(p2_health.current(), 1, "p2 not touched by untargeted heal");
}

/// Two players each carry their own `PlayerInputFrame`; mutating
/// one player's input frame must not propagate to the other.
/// Pins the multiplayer-readiness invariant for the per-player
/// input migration (OVERNIGHT-TODO #17.5) — if a future patch
/// turns the input frame back into a `Resource`, this test will
/// stop being meaningful and should fail loudly.
#[test]
fn two_players_have_independent_input_frames() {
    let mut app = App::new();
    let p1 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PlayerSlot(0),
            PrimaryPlayer,
            PlayerInputFrame::default(),
        ))
        .id();
    let p2 = app
        .world_mut()
        .spawn((PlayerEntity, PlayerSlot(1), PlayerInputFrame::default()))
        .id();

    // Mutate p1's input frame only.
    app.world_mut()
        .entity_mut(p1)
        .get_mut::<PlayerInputFrame>()
        .unwrap()
        .frame
        .interact_pressed = true;

    let p1_input = app.world().entity(p1).get::<PlayerInputFrame>().unwrap();
    let p2_input = app.world().entity(p2).get::<PlayerInputFrame>().unwrap();
    assert!(
        p1_input.frame.interact_pressed,
        "p1's input frame should reflect the just-written press"
    );
    assert!(
        !p2_input.frame.interact_pressed,
        "p2's input frame must not pick up p1's press — per-player input \
             only makes sense if the components are independent"
    );
}
