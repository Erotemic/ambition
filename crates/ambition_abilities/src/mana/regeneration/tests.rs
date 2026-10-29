use super::*;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

fn mana_of(app: &App, body: Entity) -> f32 {
    crate::mana::level(app.world().get(body))
        .expect("the fixture body holds Mana")
        .current
}

fn drain_mana(app: &mut App, body: Entity, amount: f32) {
    assert!(crate::mana::spend(
        app.world_mut()
            .get_mut::<ambition_platformer2d_core::resources::ActorResources>(body)
            .as_deref_mut(),
        amount
    ));
}

fn mana_regen_app(dt: f32, policy: Option<f32>) -> App {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime::new(dt, dt));
    if let Some(per_second) = policy {
        app.insert_resource(ManaRegen(per_second));
    }
    app.add_systems(Update, regenerate_driven_mana);
    app
}

#[test]
fn mana_regenerates_over_time_but_clamps_to_max() {
    let mut app = mana_regen_app(1.0, Some(crate::mana::REGEN_PER_SEC));
    let player = app
        .world_mut()
        .spawn((PlayerEntity, PrimaryPlayer, crate::mana::bank()))
        .id();
    // Drain it, then let it tick back up.
    drain_mana(&mut app, player, 60.0);
    let before = mana_of(&app, player);
    app.update();
    let after = mana_of(&app, player);
    assert!(
        after > before,
        "mana should regenerate ({before} -> {after})"
    );

    // Many ticks can't exceed max.
    for _ in 0..20 {
        app.update();
    }
    let m = crate::mana::level(app.world().get(player)).expect("held");
    assert!(m.current <= m.max + 1e-3, "mana clamps to max");
}

/// ⛔ THE RATE IS THE COMPOSITION'S, AND ITS ABSENCE IS AN ANSWER. A game that
/// states no regen refills nothing, and a body that holds no Mana is not given
/// any by the refill. The control is the same drained body refilling under a
/// stated rate.
#[test]
fn no_stated_rate_refills_nothing_and_no_pool_gains_mana() {
    let mut app = mana_regen_app(1.0, None);
    let player = app
        .world_mut()
        .spawn((PlayerEntity, PrimaryPlayer, crate::mana::bank()))
        .id();
    drain_mana(&mut app, player, 60.0);
    app.update();
    assert_eq!(mana_of(&app, player), 40.0, "a game with no stated rate refilled Mana");

    let mut app = mana_regen_app(1.0, Some(crate::mana::REGEN_PER_SEC));
    let bare = app.world_mut().spawn((PlayerEntity, PrimaryPlayer)).id();
    app.update();
    assert!(
        app.world()
            .get::<ambition_platformer2d_core::resources::ActorResources>(bare)
            .is_none(),
        "the refill handed a body a pool it was never declared"
    );
    app.world_mut()
        .entity_mut(bare)
        .insert(crate::mana::bank());
    drain_mana(&mut app, bare, 60.0);
    app.update();
    assert!(mana_of(&app, bare) > 40.0, "the control: a held pool does refill");
}

#[test]
fn two_driven_bodies_each_regenerate_their_own_mana() {
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};

    let mut app = mana_regen_app(0.5, Some(crate::mana::REGEN_PER_SEC));
    app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));

    let drained = |app: &mut App, slot: u8, sim: &str| -> Entity {
        let body = app
            .world_mut()
            .spawn((
                crate::mana::bank(),
                DrivingParticipant(PlayerSlot(slot)),
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement(sim),
            ))
            .id();
        drain_mana(app, body, 40.0);
        body
    };
    let a = drained(&mut app, 0, "seat_a");
    let b = drained(&mut app, 1, "seat_b");
    let before = mana_of;
    let (a_before, b_before) = (before(&app, a), before(&app, b));
    // ⛔ THE PREMISE: a full meter cannot be seen to refill.
    assert!(
        a_before < 100.0 && b_before < 100.0,
        "the fixture spent no mana"
    );

    app.update();

    for (body, was, who) in [(a, a_before, "a"), (b, b_before, "b")] {
        assert!(
            before(&app, body) > was,
            "seat {who}'s mana did not regenerate: {was} -> {}",
            before(&app, body)
        );
    }
}
