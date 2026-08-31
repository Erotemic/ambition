//! The player -> static-chest open path as a minimal-App harness:
//! a buffered interact over an overlapping, unopened chest inserts
//! `Opened`; an unbuffered player or a non-overlapping chest does not.
use super::*;
// ⭐ the module above stopped globbing `features/ecs`, so this fixture names
// what it was borrowing through it.
use ambition_characters::actor::BodyAnimFacts;
use ambition_characters::control::SlotInteractionState;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::BodyBaseSize;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use bevy::prelude::{App, Entity, Update};

fn app() -> App {
    let mut app = App::new();
    app.insert_resource(GameplayBanner::default());
    app.init_resource::<SlotInteractionState>();
    app.add_message::<SetFlagRequested>();
    app.add_message::<crate::avatar::PlayerHealRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_systems(Update, open_ecs_chests);
    app
}

fn player(app: &mut App, pos: ae::Vec2, buffered: bool) -> Entity {
    // The buffered interact is SLOT state now, not a per-body component.
    if buffered {
        app.world_mut()
            .resource_mut::<SlotInteractionState>()
            .primary_mut()
            .interact_buffer_timer = 0.5;
    }
    let entity = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyKinematics {
                pos,
                size: ae::Vec2::new(28.0, 46.0),
                facing: 1.0,
                ..Default::default()
            },
            BodyBaseSize {
                base_size: ae::Vec2::new(28.0, 46.0),
            },
            BodyAnimFacts::default(),
            // The reward lands HERE — an opened chest pays out to the body that
            // opened it, the same way a walked-over coin does.
            ambition_characters::actor::BodyWallet::default(),
        ))
        .id();
    app.world_mut()
        .insert_resource(ControlledSubject(Some(entity)));
    entity
}

fn chest(app: &mut App, id: &str, pos: ae::Vec2) -> Entity {
    chest_holding(app, id, pos, None)
}

fn chest_holding(
    app: &mut App,
    id: &str,
    pos: ae::Vec2,
    reward: Option<ambition_interaction::PickupKind>,
) -> Entity {
    app.world_mut()
        .spawn((
            FeatureSimEntity,
            FeatureId::new(id),
            FeatureName::new("Chest"),
            CenteredAabb::from_center_size(pos, ae::Vec2::new(24.0, 24.0)),
            ChestFeature::new(ambition_interaction::Chest::new(id, reward)),
        ))
        .id()
}

#[test]
fn buffered_interact_opens_an_overlapping_chest() {
    let mut app = app();
    let center = ae::Vec2::new(64.0, 64.0);
    player(&mut app, center, true);
    let c = chest(&mut app, "c1", center);
    app.update();
    assert!(
        app.world().get::<Opened>(c).is_some(),
        "buffered interact over the chest opens it"
    );
}

#[test]
fn unbuffered_player_leaves_chest_closed() {
    let mut app = app();
    let center = ae::Vec2::new(64.0, 64.0);
    player(&mut app, center, false);
    let c = chest(&mut app, "c1", center);
    app.update();
    assert!(
        app.world().get::<Opened>(c).is_none(),
        "no buffered interact -> chest stays closed"
    );
}

#[test]
fn distant_chest_is_not_opened() {
    let mut app = app();
    player(&mut app, ae::Vec2::new(64.0, 64.0), true);
    let c = chest(&mut app, "c1", ae::Vec2::new(2000.0, 2000.0));
    app.update();
    assert!(
        app.world().get::<Opened>(c).is_none(),
        "a non-overlapping chest stays closed even with a buffered interact"
    );
}

/// AN OPENED CHEST PAYS OUT WHAT IT WAS AUTHORED WITH.
///
///  this is the guard for a payload that had ZERO READERS.
/// `ChestFeature::reward()` is an `Option<PickupKind>` filled by every chest
/// author in the game — LDtk's `spawn_static`, the mob encounter's reward chest
/// and the boss's `DropChest` profile — and `open_ecs_chests` asked for
/// `With<ChestFeature>`, never `&ChestFeature`. So every chest in the game
/// opened, sparked, played its sound, announced *"opened X"* and gave the player
/// nothing, and no test noticed because the three that existed all authored a
/// chest with `None` in it.
#[test]
fn an_opened_chest_pays_out_what_it_was_authored_with() {
    let mut app = app();
    let center = ae::Vec2::new(64.0, 64.0);
    let body = player(&mut app, center, true);
    let c = chest_holding(
        &mut app,
        "c1",
        center,
        Some(ambition_interaction::PickupKind::Currency { amount: 25 }),
    );

    assert_eq!(
        app.world()
            .get::<ambition_characters::actor::BodyWallet>(body)
            .expect("the opener carries a wallet")
            .balance,
        0,
        "the fixture must start broke, or the payout below proves nothing"
    );

    app.update();

    assert!(
        app.world().get::<Opened>(c).is_some(),
        "the chest did not open, so this measures the payout of nothing"
    );
    assert_eq!(
        app.world()
            .get::<ambition_characters::actor::BodyWallet>(body)
            .expect("the opener carries a wallet")
            .balance,
        25,
        "the chest opened and paid nothing: its authored reward reached no grant"
    );
}

/// AND A CHEST AUTHORED EMPTY PAYS NOTHING — the other half, without which
/// the grant could be paying out a constant.
#[test]
fn an_empty_chest_pays_nothing() {
    let mut app = app();
    let center = ae::Vec2::new(64.0, 64.0);
    let body = player(&mut app, center, true);
    let c = chest(&mut app, "c1", center);
    app.update();

    assert!(
        app.world().get::<Opened>(c).is_some(),
        "the chest did not open, so this measures nothing"
    );
    assert_eq!(
        app.world()
            .get::<ambition_characters::actor::BodyWallet>(body)
            .expect("the opener carries a wallet")
            .balance,
        0,
        "a chest authored with no reward paid one out anyway"
    );
}

/// ⭐⭐ TWO DRIVEN BODIES EACH OPEN THEIR OWN CHEST, IN ONE TICK.
///
/// ⛔⛔ THIS SYSTEM RESOLVED ONE `ControlledSubject`, which is one entity by
/// construction — so on a couch stage the second seat could stand on a chest and
/// press interact forever. The gesture half was already per-body
/// (`ActingParticipant` keys the buffered interact off the body's own driving
/// slot); only the SUBJECT was singular, which is the same shape the item verbs
/// had.
///
/// ⛔ THE REWARD IS PART OF THE ASSERTION. "Both chests opened" would also hold
/// if one body opened both, so each wallet has to show its own payout.
#[test]
fn two_driven_bodies_each_open_their_own_chest() {
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    let mut app = app();
    app.insert_resource(ControlledSubject(None));
    // Both seats are holding an interact.
    {
        let mut gestures = app.world_mut().resource_mut::<SlotInteractionState>();
        gestures.primary_mut().interact_buffer_timer = 0.5;
        if let Some(second) = gestures.get_mut(PlayerSlot(1)) {
            second.interact_buffer_timer = 0.5;
        }
    }

    let seated = |app: &mut App, slot: u8, sim: &str, pos: ae::Vec2| -> Entity {
        app.world_mut()
            .spawn((
                BodyKinematics {
                    pos,
                    size: ae::Vec2::new(28.0, 46.0),
                    facing: 1.0,
                    ..Default::default()
                },
                BodyBaseSize {
                    base_size: ae::Vec2::new(28.0, 46.0),
                },
                BodyAnimFacts::default(),
                ambition_characters::actor::BodyWallet::default(),
                DrivingParticipant(PlayerSlot(slot)),
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement(sim),
            ))
            .id()
    };

    let a = seated(&mut app, 0, "seat_a", ae::Vec2::new(100.0, 100.0));
    let b = seated(&mut app, 1, "seat_b", ae::Vec2::new(900.0, 100.0));
    let chest_a = chest_holding(
        &mut app,
        "chest_a",
        ae::Vec2::new(100.0, 100.0),
        Some(ambition_interaction::PickupKind::Currency { amount: 7 }),
    );
    let chest_b = chest_holding(
        &mut app,
        "chest_b",
        ae::Vec2::new(900.0, 100.0),
        Some(ambition_interaction::PickupKind::Currency { amount: 7 }),
    );

    app.update();

    assert!(
        app.world().get::<Opened>(chest_a).is_some(),
        "seat a's chest stayed shut"
    );
    assert!(
        app.world().get::<Opened>(chest_b).is_some(),
        "seat b's chest stayed shut"
    );
    for (body, who) in [(a, "a"), (b, "b")] {
        let wallet = app
            .world()
            .get::<ambition_characters::actor::BodyWallet>(body)
            .expect("the body has a wallet");
        assert!(
            wallet.balance > 0,
            "seat {who} opened a chest and somebody else was paid for it"
        );
    }
}
