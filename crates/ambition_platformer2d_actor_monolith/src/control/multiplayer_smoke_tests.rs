//! Multi-player smoke tests: spawn two player entities and assert their
//! per-player components (safety anchors) plus slot-owned input, and the
//! singleton queries / heal routing stay independent and correct.

// ⚠ NAMED HERE NOW. This file is `#[path]`-included from `components.rs`, which
// used to carry the prelude for it; that import became unused there the moment
// `LocalPlayer` moved to the floor crate and left the module with nothing else.
use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly;
use ambition_platformer2d_shared_tangle::safe_position::PlayerSafetyState;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_characters::control::PlayerSlot;
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;

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
            PrimaryPlayer,
            PlayerSafetyState::new(p1_initial),
        ))
        .id();
    let p2 = app
        .world_mut()
        .spawn((
            PlayerEntity,
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
        .spawn((PlayerEntity, PrimaryPlayer));
    app.world_mut().spawn((PlayerEntity,));

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
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    app.world_mut()
        .spawn((PlayerEntity, SimId::player_slot(0), PrimaryPlayer));
    app.world_mut().spawn((PlayerEntity, SimId::player_slot(1)));
    app.world_mut().spawn((PlayerEntity, SimId::player_slot(2)));

    let mut q = app
        .world_mut()
        .query_filtered::<&SimId, With<PlayerEntity>>();
    let mut ids: Vec<String> = q.iter(app.world()).map(|id| id.as_str().to_string()).collect();
    ids.sort_unstable();
    let mut expected: Vec<String> = (0..3).map(|n| SimId::player_slot(n).as_str().to_string()).collect();
    expected.sort_unstable();
    assert_eq!(ids, expected);
}

/// A `PlayerHealRequested` carrying `target: Some(p2)` heals p2,
/// not the primary p1. Pins the OVERNIGHT-TODO #17.6 bridge —
/// pickups now route heals to the player who actually overlapped
/// the heart instead of always to primary.
#[test]
fn targeted_heal_routes_to_named_entity_not_primary() {
    use crate::avatar::{apply_player_heal_requests, PlayerHealRequested};
    use ambition_characters::actor::BodyHealth;

    let mut app = App::new();
    app.add_message::<PlayerHealRequested>();
    app.add_systems(Update, apply_player_heal_requests);

    let p1 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
            }),
        ))
        .id();
    let p2 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
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
    use crate::avatar::{apply_player_heal_requests, PlayerHealRequested};
    use ambition_characters::actor::BodyHealth;

    let mut app = App::new();
    app.add_message::<PlayerHealRequested>();
    app.add_systems(Update, apply_player_heal_requests);

    let p1 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
            }),
        ))
        .id();
    let p2 = app
        .world_mut()
        .spawn((
            PlayerEntity,
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
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

/// Two participant slots carry independent control frames. Mutating one slot
/// must not propagate into another; this is the multiplayer-readiness invariant
/// now that input is slot-owned rather than copied onto bodies.
#[test]
fn two_slots_have_independent_control_frames() {
    use ambition_characters::control::SlotControls;
    use ambition_platformer2d_core::ControlFrame;

    let mut slots = SlotControls::default();
    let mut first = ControlFrame::default();
    first.interact_pressed = true;
    let mut second = ControlFrame::default();
    second.axis_x = -1.0;
    slots.set(PlayerSlot(0), first);
    slots.set(PlayerSlot(1), second);

    assert!(slots.get(PlayerSlot(0)).interact_pressed);
    assert!(!slots.get(PlayerSlot(1)).interact_pressed);
    assert_eq!(slots.get(PlayerSlot(0)).axis_x, 0.0);
    assert_eq!(slots.get(PlayerSlot(1)).axis_x, -1.0);
}
