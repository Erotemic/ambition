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
//! Setup: spawn a held bolt straddling `portal_lab`'s authored purple floor
//! portal, moving INTO it. The authored portal sits on a SOLID floor, so this
//! also exercises the carved-world collision (without it, the bolt's own
//! solid-raycast would detonate it on the floor before it could transit). Step
//! the real app and assert the bolt jumps to the partner (yellow) portal.

use ambition_platformer_runtime::projectile::{
    ProjectileFaction, ProjectileGameplay, ProjectileKind,
};
use ambition_sandbox::items::pickup::HeldProjectile;
use ambition_sandbox::platformer_runtime::body::BodyKinematics;
use ambition_sandbox::portal::{
    PlacedPortal, PortalBody, PortalChannel, PortalChannelColor, PortalPolicy,
};
use ambition_sandbox::rl_sim::TimestepMode;
use ambition_sandbox::{AgentAction, SandboxSim, SandboxSimOptions};
use bevy::prelude::*;

const PURPLE: PortalChannel = PortalChannel::Authored(PortalChannelColor::Purple);
const YELLOW: PortalChannel = PortalChannel::Authored(PortalChannelColor::Yellow);

fn base() -> AgentAction {
    AgentAction {
        move_x: 0.0,
        move_y: 0.0,
        up_pressed: false,
        down_pressed: false,
        jump: false,
        jump_held: false,
        jump_released: false,
        dash: false,
        attack: false,
        blink: false,
        blink_held: false,
        blink_released: false,
        pogo: false,
        interact: false,
        projectile: false,
        projectile_held: false,
        projectile_released: false,
        fly_toggle: false,
        reset: false,
        start: false,
        aim_x: 0.0,
        aim_y: 0.0,
    }
}

#[test]
fn a_held_bolt_transits_an_authored_portal_in_the_real_app() {
    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_start_room("portal_lab");
    let mut sim = SandboxSim::new_with_options(opts).expect("SandboxSim::new in portal_lab");
    sim.step(base());

    // Read the authored floor pair.
    let (entry_pos, entry_normal, exit_pos) = {
        let mut q = sim.world_mut().query::<&PlacedPortal>();
        let world = sim.world();
        let entry = q
            .iter(world)
            .find(|p| p.channel == PURPLE)
            .copied()
            .expect("portal_lab has an authored Purple portal");
        let exit = q
            .iter(world)
            .find(|p| p.channel == YELLOW)
            .copied()
            .expect("portal_lab has an authored Yellow portal");
        (entry.pos, entry.normal, exit.pos)
    };
    let entry_to_exit = entry_pos.distance(exit_pos);

    // A held bolt straddling the purple floor portal, moving INTO its face
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
                kind: ProjectileKind::Fireball,
                faction: ProjectileFaction::Player,
                age: 0.0,
                max_lifetime: f32::MAX,
                gravity: 0.0,
                damage: 3,
                bounces_remaining: 0,
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
    // (yellow) and far from the ENTRY (purple) — a transit, not a straight flight
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
