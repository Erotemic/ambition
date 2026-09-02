//! S1: the event-created families are not just REGISTERED — their per-frame
//! simulation is REWIND-STABLE.
//!
//! `rollback_coverage.rs` proves that a sentry, a vortex well, a temporary
//! gravity zone, a falling hazard, a portal shot and a held-item bolt carry
//! accounted, anchored registrations. That is a census of a standing world. It
//! cannot see a system that STEPS one of those entities from state a rewind does
//! not restore — a `Local`, an unregistered field, a HashMap walk — because the
//! census never rewinds.
//!
//! This file does. The same production seams build the same population, the
//! populated world becomes a fresh SyncTest baseline, and then the session
//! saves, advances, rewinds and resimulates every frame while each family is
//! LIVE, comparing checksums. A desync here names the frame; the family that
//! moved on that frame is the suspect.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use bevy::prelude::{Entity, With};

fn rollback_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            // ⛔ NOT distance 0: a SyncTest at check distance 0 saves nothing
            // and compares nothing. Seven of eight so nearly every frame is
            // resimulated from a save several frames old.
            .with_sync_test_rollback_settings(7, 8),
    )
    .expect("Ambition GGRS sync-test harness builds")
}

/// A press pattern that keeps the subject moving, jumping and FIRING, so the
/// held-item bolt family is created by play rather than by the fixture.
fn busy(frame: usize) -> AgentAction {
    AgentAction {
        move_x: if frame % 40 < 20 { 1.0 } else { -1.0 },
        jump: frame % 23 == 0,
        jump_held: frame % 23 < 6,
        attack: frame % 9 == 1,
        ..AgentAction::default()
    }
}

/// How many entities carry `T` right now.
fn count<T: bevy::prelude::Component>(sim: &mut Platformer2dSimHarness) -> usize {
    let world = sim.world_mut();
    world
        .query_filtered::<Entity, With<T>>()
        .iter(world)
        .count()
}

/// Build the event-created population through the production seams (the
/// same calls `rollback_coverage.rs` makes), give the subject a bolt thrower,
/// and make the result the session's frame-zero baseline.
fn populate(sim: &mut Platformer2dSimHarness) {
    use ambition_platformer2d::actors::abilities::ranged::sentry::deploy_sentry;
    use ambition_platformer2d::actors::abilities::ranged::vortex::open_vortex_well;
    use ambition_platformer2d::actors::abilities::thrown::gravity_grenade::open_temporary_gravity_well;
    use ambition_platformer2d::boss_encounter::{drop_hazard, FallingHazard};
    use ambition_platformer2d::combat::components::ActorFaction;
    use ambition_platformer2d::platformer::lifecycle::SessionSpawnScope;
    use ambition_platformer2d::platformer::sim_id::SimId;
    use ambition_platformer2d::portal::{PortalFireIntent, PortalGunColor};

    let (subject, target) = {
        let world = sim.world_mut();
        let subject = world
            .resource::<ambition_platformer2d::platformer::markers::ControlledSubject>()
            .0
            .expect("the sandbox session has a controlled subject");
        let mut bodies = world
            .query_filtered::<Entity, With<ambition_platformer2d::engine_core::BodyKinematics>>();
        let target = bodies
            .iter(world)
            .find(|body| *body != subject)
            .unwrap_or(subject);
        (subject, target)
    };
    {
        let world = sim.world_mut();
        world
            .entity_mut(subject)
            .insert(ambition_platformer2d::combat::held_items::HeldItem::new(
            ambition_platformer2d::characters::brain::HeldItemSpec {
                id: "populated_timeline_bolt_thrower".to_string(),
                melee: None,
                ranged: Some(
                    ambition_platformer2d::characters::brain::action_set::RangedActionSpec::bolt(
                        400.0, 1,
                    ),
                ),
                use_behavior: Default::default(),
            },
        ));
        let mut commands = world.commands();
        deploy_sentry(
            &mut commands,
            SessionSpawnScope::UNSCOPED,
            bevy::math::Vec2::new(96.0, 96.0),
            ActorFaction::Player,
            None,
            None,
            Some(SimId::spawned(&SimId::player_slot(0), 0)),
        );
        open_vortex_well(
            &mut commands,
            SessionSpawnScope::UNSCOPED,
            bevy::math::Vec2::new(128.0, 96.0),
            Some(SimId::spawned(&SimId::player_slot(0), 1)),
        );
        open_temporary_gravity_well(
            &mut commands,
            SessionSpawnScope::UNSCOPED,
            bevy::math::Vec2::new(160.0, 96.0),
        );
        drop_hazard(
            &mut commands,
            SessionSpawnScope::UNSCOPED,
            bevy::math::Vec2::new(192.0, 240.0),
            FallingHazard {
                size: bevy::math::Vec2::new(24.0, 24.0),
                gravity: 900.0,
                terminal: 600.0,
                align_tolerance: 8.0,
                target,
                impact_gate: "a_gate_this_test_never_reads".to_string(),
                vel_y: 0.0,
                dropping: false,
            },
        );
        world.flush();
    }
    sim.world_mut().write_message(PortalFireIntent {
        origin: bevy::math::Vec2::new(224.0, 96.0),
        dir: bevy::math::Vec2::new(1.0, 0.0),
        channel: ambition_platformer2d::portal::PortalChannel::Gun(PortalGunColor::BLUE),
    });
    // The intent is consumed by the sim on a SETUP frame the timeline does not
    // keep, and the populated world becomes the new baseline. ⛔ Not a plain
    // `step`: under the live check the rewind lands behind the spawns,
    // `LoadWorld` despawns every anchored entity the fixture placed, and the
    // resimulation never recreates them — measured as "no sentry in the
    // baseline world" the first time this ran.
    sim.run_rollback_setup_frame()
        .expect("the populated world becomes the SyncTest baseline");
}

/// A populated timeline rewinds and resimulates to the same checksums, frame
/// after frame, while every event-created family is live and stepping.
#[test]
fn the_event_created_families_are_rewind_stable_while_they_step() {
    use ambition_platformer2d::actors::abilities::ranged::sentry::Sentry;
    use ambition_platformer2d::actors::abilities::ranged::vortex::VortexWell;
    use ambition_platformer2d::boss_encounter::FallingHazard;
    use ambition_platformer2d::platformer::gravity::TemporaryZone;
    use ambition_platformer2d::platformer::projectile::ProjectileGameplay;
    use ambition_platformer2d::portal::PortalShot;

    let mut sim = rollback_sim();
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }
    populate(&mut sim);
    // ⛔⛔ THE SESSION CHECKSUM IS NOT ENOUGH HERE, and that is the second half
    // of this test. Forty-seven registrations — the sentry, the vortex well,
    // the gravity zones, the falling chest, item motion among them — are
    // "value-probed for localization, not in the session checksum". A sentry
    // whose stepper reads a process-global counter drifts on replay and
    // `rollback_health` stays GREEN for all 150 frames (measured 2026-09-02:
    // `remaining_s -= dt * (1 + (n % 5) / 100)` with a static counter). The
    // restore audit censuses EVERY registered type at each save and compares a
    // frame's repeat save against its first, so it is the oracle that sees the
    // probed families — the same poison fails the assertion below at frame 2,
    // naming `Sentry`. Enabled AFTER the rebase: its baselines are keyed by
    // frame number, which the rebase restarts.
    //
    // ⚠ A poison PERIODIC IN THE CHECK WINDOW cancels: `n % 7` under check
    // distance 7 summed to the same drift on every replay and stayed green.
    sim.world_mut()
        .insert_resource(ambition_platformer2d::rollback::RollbackRestoreAudit::enabled());

    // ⛔ ANTI-VACUITY: every family exists at the baseline, and the ones that
    // must be CREATED BY PLAY (the bolt) are counted below at the frame they
    // first appear. A seam that stops spawning turns this red rather than
    // quietly shrinking what the timeline proves.
    let baseline = [
        ("sentry", count::<Sentry>(&mut sim)),
        ("vortex well", count::<VortexWell>(&mut sim)),
        ("temporary gravity zone", count::<TemporaryZone>(&mut sim)),
        ("falling hazard", count::<FallingHazard>(&mut sim)),
        ("portal shot", count::<PortalShot>(&mut sim)),
    ];
    for (what, n) in baseline {
        assert!(
            n > 0,
            "no {what} in the baseline world; the timeline below would prove nothing about it"
        );
    }

    let mut first_bolt_frame = None;
    let mut live_frames = std::collections::BTreeMap::<&str, usize>::new();
    for frame in 0..150 {
        sim.step(busy(frame));
        sim.rollback_health().unwrap_or_else(|error| {
            panic!(
                "frame {frame}: the populated timeline desynced under SyncTest — a \
                 family stepping on this frame reads state a rewind does not \
                 restore: {error}"
            )
        });
        if count::<ProjectileGameplay>(&mut sim) > 0 {
            first_bolt_frame.get_or_insert(frame);
            *live_frames.entry("bolt").or_default() += 1;
        }
        if count::<Sentry>(&mut sim) > 0 {
            *live_frames.entry("sentry").or_default() += 1;
        }
        if count::<FallingHazard>(&mut sim) > 0 {
            *live_frames.entry("falling hazard").or_default() += 1;
        }
        if count::<PortalShot>(&mut sim) > 0 {
            *live_frames.entry("portal shot").or_default() += 1;
        }
        if count::<VortexWell>(&mut sim) > 0 {
            *live_frames.entry("vortex well").or_default() += 1;
        }
    }

    let stats = sim
        .rollback_execution_stats()
        .expect("GGRS instrumentation is installed");
    assert!(
        stats.load_runs > 0 && stats.advance_runs > 150,
        "the session never rewound ({stats:?}), so the checksums above compared nothing"
    );
    assert!(
        first_bolt_frame.is_some(),
        "the held bolt thrower never fired in 150 frames of pressing attack, so \
         the one family created BY PLAY was never on the timeline"
    );
    // Each family must have been live for at least one full check window
    // (eight frames), or no rewind ever resimulated it stepping.
    for what in [
        "sentry",
        "vortex well",
        "falling hazard",
        "portal shot",
        "bolt",
    ] {
        let frames = live_frames.get(what).copied().unwrap_or(0);
        assert!(
            frames >= 8,
            "{what} was live for only {frames} frame(s) of the timeline — fewer than \
             one check window — so no rewind resimulated it stepping"
        );
    }
    let audit = sim
        .world()
        .resource::<ambition_platformer2d::rollback::RollbackRestoreAudit>();
    assert!(
        audit.comparisons > 0 && audit.resimulations > 0,
        "the restore audit compared nothing ({}), so its silence below is not evidence",
        audit.coverage()
    );
    assert!(
        audit.divergences.is_empty(),
        "a registered component was recomputed differently on replay, or did not \
         survive its own snapshot, while the event-created families were live — \
         the session checksum cannot see a probed-only type, this can:\n{}\n{}",
        audit.report(),
        audit.coverage()
    );
    eprintln!(
        "[populated timeline] 150 frames, {} loads, {} advances; live frames {live_frames:?}; audit: {}",
        stats.load_runs,
        stats.advance_runs,
        audit.coverage()
    );
}
