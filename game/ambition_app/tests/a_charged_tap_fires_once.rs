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
