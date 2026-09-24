#![cfg(feature = "rl_sim")]
//! One tap of the ranged button is one shot.
//!
//! ⛔ MEASURED 2026-09-23: the shipped protagonist (`player_robot_v3`, which
//! fires through the charge path) fired TWO projectiles per tap: the charge
//! path's Fireball, and a bolt from the `fire` intent the player brain raises on
//! the same release. Preparation derived a `ranged` verb from the authored
//! ranged preset whatever `ranged_execution` said, so the moveset answered it;
//! with that verb gone, the flat `fire → Ranged` emission answered it instead.
//! A charger's ranged intent now has ONE owner, the charge path.

use crate::common::{base, fixed_60hz_sim};
use bevy::prelude::*;

#[test]
fn a_tap_of_the_charging_protagonists_ranged_button_fires_one_shot() {
    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);

    // ⚠ ANTI-VACUITY: the subject is a body that charges.
    let charges = {
        let mut q = sim.world_mut().query_filtered::<
            Has<ambition_platformer2d::characters::brain::ChargesProjectiles>,
            With<ambition_platformer2d::platformer::body::PrimaryBody>,
        >();
        q.single(sim.world()).expect("one primary body")
    };
    assert!(charges, "the shipped protagonist no longer charges, so this measures nothing");

    let live = |sim: &mut ambition_app::Platformer2dSimHarness| {
        let mut q = sim
            .world_mut()
            .query::<&ambition_platformer2d::projectiles::ProjectileGameplay>();
        q.iter(sim.world()).count()
    };
    assert_eq!(live(&mut sim), 0);
    sim.step(ambition_app::AgentAction { projectile: true, projectile_held: true, ..base() });
    sim.step(ambition_app::AgentAction { projectile_released: true, ..base() });
    let mut most = 0;
    for _ in 0..30 {
        sim.step(base());
        most = most.max(live(&mut sim));
    }
    assert_eq!(most, 1, "one tap put {most} projectiles in the air");
}

/// The hand's equip, as a one-shot system: the ONE take-custody operation the
/// inventory menu and the world pickup both call.
fn hold_the_gun_sword(
    mut commands: Commands,
    mut bodies: Query<
        (Entity, ambition_platformer2d::combat::hand::RepertoireQuery),
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
) {
    let (player, mut repertoire) = bodies.single_mut().expect("one primary body");
    let spec = ambition_platformer2d::held_items::held_spec_by_id("gun_sword")
        .expect("gun_sword is a registered held item");
    ambition_platformer2d::held_items::equip_held_spec(&mut commands, player, &mut repertoire, spec);
}

/// ⛔ A HELD ITEM IS THE WHOLE RANGED VOCABULARY — so it owns the press.
///
/// Charge ownership is a CHARACTER fact (`ChargesProjectiles`), and the hand
/// replaces the ranged slot after it: a charger holding a gun-sword carries the
/// item's `ranged` move AND the charge marker, and one press
/// reached both the move and the charge path.
#[test]
fn a_charger_holding_a_ranged_item_fires_the_item_once_per_tap() {
    use bevy::ecs::system::RunSystemOnce;

    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);
    sim.world_mut()
        .run_system_once(hold_the_gun_sword)
        .expect("the equip ran");
    sim.step_n(base(), 5);

    // ⚠ ANTI-VACUITY: both owners must be installed, or there is nothing to
    // arbitrate between.
    let (charges, moveset_ranged) = {
        let mut q = sim.world_mut().query_filtered::<(
            Has<ambition_platformer2d::characters::brain::ChargesProjectiles>,
            Option<&ambition_platformer2d::combat::moveset::ActorMoveset>,
        ), With<ambition_platformer2d::platformer::body::PrimaryBody>>();
        let (charges, moveset) = q.single(sim.world()).expect("one primary body");
        (
            charges,
            moveset.is_some_and(ambition_platformer2d::combat::moveset::routes_ranged),
        )
    };
    assert!(charges, "the protagonist no longer charges, so this measures nothing");
    assert!(
        moveset_ranged,
        "the held gun-sword put no ranged move in the effective moveset, so this \
         measures nothing"
    );

    let live = |sim: &mut ambition_app::Platformer2dSimHarness| {
        let mut q = sim
            .world_mut()
            .query::<&ambition_platformer2d::projectiles::ProjectileGameplay>();
        q.iter(sim.world()).count()
    };
    assert_eq!(live(&mut sim), 0);
    sim.step(ambition_app::AgentAction { projectile: true, projectile_held: true, ..base() });
    sim.step(ambition_app::AgentAction { projectile_released: true, ..base() });
    let mut most = 0;
    let mut damages = std::collections::BTreeSet::new();
    for _ in 0..40 {
        sim.step(base());
        most = most.max(live(&mut sim));
        let mut q = sim
            .world_mut()
            .query::<&ambition_platformer2d::projectiles::ProjectileGameplay>();
        damages.extend(q.iter(sim.world()).map(|projectile| projectile.damage));
    }
    assert_eq!(
        most, 1,
        "one tap with a gun-sword in hand put {most} projectiles in the air"
    );
    // ...and it is the ITEM's shot, not the charge path's Fireball: a count of
    // one says only that one owner answered, not which.
    let item_damage = ambition_platformer2d::held_items::held_spec_by_id("gun_sword")
        .and_then(|spec| spec.ranged)
        .map(|ranged| ranged.damage)
        .expect("the gun-sword authors a ranged shot");
    assert_eq!(
        damages.into_iter().collect::<Vec<_>>(),
        vec![item_damage],
        "the shot in the air is not the gun-sword's"
    );
}
