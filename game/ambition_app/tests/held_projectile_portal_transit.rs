// Portal integration test: only built with the portal mechanic + RL stepping
// API. Compiled out (empty test binary) when `portal` is disabled.
#![cfg(all(feature = "portal", feature = "rl_sim"))]
//! Held ranged shots (gun-sword bolt / Fireball) transit portals through the
//! SAME generic `portal_transit` algorithm the player uses (2026-06-09).
//!
//! Before this, a `HeldProjectile` was a separate motion representation with its
//! own `pos`/`vel` and collided against the RAW world, so it could never transit
//! and detonated on the wall at a portal. Now its kinematics live in the shared
//! `BodyKinematics`, it carries the `ProjectileGameplay` projectile marker (so
//! `ensure_projectile_portal_bodies` opts it into transit), and
//! `held_projectile_step` collides against the PORTAL-CARVED world — so a bolt
//! fired into a portal flies through the opening and emerges from the partner.
//!
//! Setup: spawn a held bolt straddling `portal_lab`'s first authored
//! floor-to-floor portal pair, moving INTO its entry. The authored entry sits on
//! a SOLID floor, so this also exercises carved-world collision (without it, the
//! bolt's own solid-raycast would detonate on the floor before it could transit).
//! Step the real app and assert the bolt jumps to the partner portal.

mod common;
use common::{base, first_floor_authored_portal_pair, fixed_60hz_room_sim};

use ambition::actors::items::pickup::HeldProjectile;
use ambition::actors::platformer_runtime::body::BodyKinematics;
use ambition::platformer::projectile::{ProjectileGameplay, WorldHitPolicy};
use ambition::portal::{PortalBody, PortalPolicy};
use bevy::prelude::*;

#[test]
fn a_held_bolt_transits_an_authored_portal_in_the_real_app() {
    let mut sim = fixed_60hz_room_sim("portal_lab");
    sim.step(base());

    // Read a live authored floor pair after link resolution. `portal_lab` uses
    // explicit link ids, so live portal channels may be generated `Indexed`
    // values rather than the legacy Purple/Yellow authoring colors.
    let (entry_pos, entry_normal, exit_pos) = {
        let (entry, exit) = first_floor_authored_portal_pair(&mut sim);
        (entry.pos, entry.normal, exit.pos)
    };
    let entry_to_exit = entry_pos.distance(exit_pos);

    // A held bolt straddling the floor portal, moving INTO its face
    // (against the outward normal) at 440 px/s (the Fireball speed). It carries
    // the shared body + the projectile marker + the held-shot gameplay — exactly
    // what `fire_held_ranged_system` now spawns.
    let bolt = sim
        .world_mut()
        .spawn((
            BodyKinematics {
                pos: entry_pos,
                vel: -entry_normal * 440.0,
                size: Vec2::new(24.0, 18.0),
                facing: 1.0,
            },
            ProjectileGameplay {
                age: 0.0,
                max_lifetime: f32::MAX,
                gravity: 0.0,
                damage: 3,
                bounces_remaining: 0,
                world_hit: WorldHitPolicy::ExpireOnContact,
            },
            HeldProjectile {
                damage: 3,
                traveled: 0.0,
                explode_half: 0.0,
            },
            // Tagged at spawn exactly as `fire_held_ranged_system` now does.
            PortalBody,
            PortalPolicy {
                reorient: false,
                carry_velocity: true,
            },
            Name::new("test held bolt"),
        ))
        .id();

    // Step the real app: `ensure_projectile_portal_bodies` tags the bolt, the
    // live `portal_transit` carries it across. It must end up near the EXIT
    // and far from the ENTRY — a transit, not a straight flight
    // or a detonation on the floor.
    let mut emerged = false;
    for _ in 0..12 {
        sim.step(base());
        let Some(kin) = sim.world().get::<BodyKinematics>(bolt) else {
            // Despawned = it detonated on the floor / expired without transiting.
            break;
        };
        if kin.pos.distance(exit_pos) < entry_to_exit * 0.5
            && kin.pos.distance(entry_pos) > entry_to_exit * 0.5
        {
            emerged = true;
            break;
        }
    }

    assert!(
        emerged,
        "the held bolt should transit the authored portal pair and emerge near the \
         partner portal (it is opted into the generic transit + collides against \
         the carved world)",
    );
}
