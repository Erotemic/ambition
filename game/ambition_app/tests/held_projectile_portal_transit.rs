// Portal integration test: only built with the portal mechanic + RL stepping
// API. Compiled out (empty test binary) when `portal` is disabled.
#![cfg(all(feature = "portal", feature = "rl_sim"))]
//! A HAND-FIRED shot transits an AUTHORED portal in the REAL APP.
//!
//! ⛔ THIS IS NOT COVERED BY `projectile_transit_tests.rs`, and the difference is
//! the fixture rather than the components. Those three cases spawn a bare
//! `(BodyKinematics, straight_projectile())` into a hand-built `app_with_transit`
//! with `PlacedPortal::fixed` walls, and prove the transit MATHS — rotation of
//! velocity and acceleration through a pair. This one asks a different question:
//! does a projectile the GAME ITSELF created, from a weapon a player picked up,
//! cross a portal the LEVEL authored, in the shipped composition.
//!
//! Before the K2 fold this file spawned a `HeldProjectile` by hand and tagged it
//! `PortalBody` + `PortalPolicy` at the spawn site, mirroring what the deleted
//! held-shot road did. Held shots are ordinary projectiles now, so the hand-tag
//! would assert the test's own setup: the interesting claim is that
//! `ensure_projectile_portal_bodies` OPTS THE SHOT IN on its own.

use crate::common::{base, first_floor_authored_portal_pair, fixed_60hz_room_sim};

use ambition_platformer2d::engine_core::{BodyKinematics, ControlFrame, Vec2};
use ambition_platformer2d::item::{GroundItem, ItemCustody};
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::portal::{PortalBody, PortalPolicy};
use bevy::prelude::*;

/// Speed the shot is given for its run at the aperture. Well above the exit-speed
/// floor so the assertion reads the transit and not the floor.
const RUN_UP_SPEED: f32 = 440.0;

/// How far outside the aperture the shot starts its run, along the portal's
/// outward normal. Clear of the surface the portal is cut into, and close enough
/// that `RUN_UP_SPEED` crosses it within the measured window.
const APPROACH_BACKOFF: f32 = 24.0;

#[test]
fn a_hand_fired_shot_transits_an_authored_portal_in_the_real_app() {
    let mut sim = fixed_60hz_room_sim("portal_lab");
    for _ in 0..30 {
        sim.step(base());
    }

    let shooter = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
        q.iter(world)
            .next()
            .expect("portal_lab spawns a primary player")
    };
    let at = sim
        .world()
        .get::<BodyKinematics>(shooter)
        .expect("the player has kinematics")
        .pos;

    // The weapon under the player's feet; one press picks it up.
    let spec = ambition_platformer2d::character::held_item_by_id("gun_sword")
        .expect("gun_sword is a registered held item");
    let weapon = sim
        .world_mut()
        .spawn((
            GroundItem {
                spec,
                pos: at,
                vel: Vec2::ZERO,
                half_extent: Vec2::splat(12.0),
            },
            ItemCustody::InWorld,
        ))
        .id();

    // ⛔ A PRESS IS A TICK OF PRESSED FOLLOWED BY RELEASED. One tick of
    // `attack_pressed` with nothing after it is not an edge the item road can
    // read across an update boundary.
    let press = ControlFrame {
        attack_pressed: true,
        ..Default::default()
    };
    sim.step_frame(press);
    for _ in 0..6 {
        sim.step_frame(ControlFrame::default());
    }
    assert!(
        matches!(
            sim.world().get::<ItemCustody>(weapon),
            Some(ItemCustody::Held { holder }) if *holder == shooter
        ),
        "premise: the player picked the gun_sword up",
    );

    // A second press fires it. The shot is the game's own -- owner, visual and
    // gameplay all minted by the fire road.
    sim.step_frame(press);
    let mut shot = None;
    for _ in 0..30 {
        // The press was driven for one tick above; every tick after it is
        // released, which is what makes it an EDGE rather than a held button.
        sim.step_frame(ControlFrame::default());
        let found = {
            let world = sim.world_mut();
            let mut q = world.query::<(
                Entity,
                &ambition_platformer2d::projectiles::ProjectileOwner,
                &ambition_platformer2d::projectiles::ProjectileVisualId,
            )>();
            q.iter(world)
                .find(|(_, owner, _)| owner.0 == shooter)
                .map(|(entity, _, _)| entity)
        };
        if let Some(found) = found {
            shot = Some(found);
            break;
        }
    }
    let shot = shot.expect("premise: pressing attack with a gun_sword in hand fires a shot");

    // ⛔ THE TAG LANDS A FRAME LATE. `ensure_projectile_portal_bodies` inserts
    // through `Commands`, so the components are queued when the shot is already
    // findable; asserting on the tick it appears would be a race the test would
    // lose intermittently. Give the insert its flush.
    sim.step_frame(ControlFrame::default());
    sim.step_frame(ControlFrame::default());

    // ⭐ THE CLAIM. The adapter opted this shot into portal transit by itself.
    // The old version of this file tagged the entity at its own spawn site, which
    // asserted the setup rather than the engine.
    assert!(
        sim.world().get::<PortalBody>(shot).is_some(),
        "ensure_projectile_portal_bodies must tag a hand-fired shot PortalBody \
         without the test doing it",
    );
    assert!(
        sim.world().get::<PortalPolicy>(shot).is_some(),
        "and must give it a transit policy",
    );

    // Put the real shot on a run at the authored aperture. Its POSITION and
    // VELOCITY are set, and nothing else: a hand-fired shot travels horizontally
    // while `portal_lab`'s linked pair is floor-to-floor, so a naturally aimed
    // shot never enters it. Everything the transit road reads -- the tagging, the
    // policy, the owner, the body -- is still the game's.
    let (entry_pos, entry_normal, exit_pos) = {
        let (entry, exit) = first_floor_authored_portal_pair(&mut sim);
        (entry.pos, entry.normal, exit.pos)
    };
    let entry_to_exit = entry_pos.distance(exit_pos);
    // ⛔ START OUTSIDE THE SURFACE, NOT ON IT. `entry_pos` is the portal's own
    // centre, which for a FLOOR portal is inside the floor; a projectile carrying
    // `WorldHitPolicy::ExpireOnContact` put there dies on its first tick and the
    // test measures a despawned entity. Back it off along the outward normal so
    // it approaches the aperture through open space, the way a real shot would.
    let approach_from = entry_pos + entry_normal * APPROACH_BACKOFF;
    {
        let mut kin = sim
            .world_mut()
            .get_mut::<BodyKinematics>(shot)
            .expect("the shot has a body");
        kin.pos = approach_from;
        kin.vel = -entry_normal * RUN_UP_SPEED;
    }

    // ⛔ THE DISCRIMINATOR IS "NEAR THE EXIT", NOT "FAR FROM THE ENTRY". A shot
    // that ignores the portal entirely still travels RUN_UP_SPEED / 60 per tick
    // and would leave the entry behind, so "it moved a long way" passes whether
    // or not it transited. Ask where it ARRIVED.
    assert!(
        entry_to_exit > RUN_UP_SPEED * 12.0 / 60.0,
        "premise — the pair must be further apart than the shot can fly in the \
         window, or arriving near the exit proves nothing (entry->exit \
         {entry_to_exit})",
    );

    // Two frames is the aperture cadence the actor transit test relies on: one
    // tags and begins, the next transfers.
    let mut closest_to_exit = f32::MAX;
    let mut despawned_on = None;
    for tick in 0..24 {
        sim.step_frame(ControlFrame::default());
        let Some(kin) = sim.world().get::<BodyKinematics>(shot) else {
            despawned_on = Some(tick);
            break;
        };
        closest_to_exit = closest_to_exit.min(exit_pos.distance(kin.pos));
    }

    // ⛔ SAY WHICH FAILURE THIS IS. A despawned shot leaves `closest_to_exit` at
    // f32::MAX, and reporting that as a distance is a nonsense number that hides
    // the actual event — the shot expired rather than missing the portal.
    assert!(
        despawned_on.is_none(),
        "the shot expired on tick {:?} instead of transiting: it was placed at \
         {approach_from:?} heading into the aperture at {entry_pos:?}",
        despawned_on,
    );
    assert!(
        closest_to_exit < entry_to_exit * 0.25,
        "the shot should have come out of the PARTNER portal: closest approach to \
         the exit was {closest_to_exit}, against an entry->exit distance of \
         {entry_to_exit}",
    );
}
