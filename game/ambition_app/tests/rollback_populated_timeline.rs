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
    use ambition_platformer2d::abilities::ranged::sentry::deploy_sentry;
    use ambition_platformer2d::abilities::ranged::vortex::open_vortex_well;
    use ambition_platformer2d::abilities::thrown::gravity_grenade::open_temporary_gravity_well;
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
    // The fixture's spawns are minted UNDER THE SUBJECT, from the subject's own
    // counter — the way every production spawner mints — so the bolts the
    // subject fires later continue the same sequence instead of colliding with
    // a hand-picked `slot:0/0`. (They did: the first hand-minted sentry and the
    // first bolt shared an id, and the identity census below caught it.)
    let (spawner, mut seq) = {
        let world = sim.world_mut();
        let spawner = world
            .get::<SimId>(subject)
            .cloned()
            .expect("the controlled subject carries a SimId");
        let mut counter = world
            .get_mut::<ambition_platformer2d::platformer::sim_id::SimIdCounter>(subject)
            .expect("an identified entity carries a counter");
        // ⚠ FIVE, not four: the portal shot at the bottom mints one too, since
        // 2026-09-04. This array is the supply and the `expect` below is what
        // says so out loud — a spawn added without extending it panics here
        // rather than silently reusing an id.
        let seq = [
            counter.next(),
            counter.next(),
            counter.next(),
            counter.next(),
            counter.next(),
        ];
        (spawner, seq.into_iter())
    };
    let mut mint = || SimId::spawned(&spawner, seq.next().expect("five ids"));
    // ⭐ AND THE MINT'S RESIM STABILITY IS MEASURED, NOT ASSUMED (2026-09-04).
    // `SimId` is registered `rollback_component_canonical`, but this repository
    // has already recorded that REGISTERED ≠ CHECKSUMMED — a real desync once
    // read clean — so registration is not the proof.
    //
    // Poisoned `SimIdCounter::next()` with a process-global `AtomicU64` drift
    // term, so a resimulated tick mints a DIFFERENT id from identical rollback
    // state. The timeline below reds at **frame 9**, naming *"GGRS sync-test
    // checksum mismatch at frames [2, 3, 4, 5, 6, 7]"*. ⇒ The id IS in the
    // session checksum and this timeline sees an unstable one. The subject
    // fires a bolt every 9 frames (`busy`), so a MID-WINDOW mint is what is
    // being exercised, not just the fixture's own five.
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
            Some(mint()),
        );
        open_vortex_well(
            &mut commands,
            SessionSpawnScope::UNSCOPED,
            bevy::math::Vec2::new(128.0, 96.0),
            Some(mint()),
        );
        open_temporary_gravity_well(
            &mut commands,
            SessionSpawnScope::UNSCOPED,
            bevy::math::Vec2::new(160.0, 96.0),
            Some(mint()),
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
            Some(mint()),
        );
        world.flush();
    }
    // ⛔⛔ THE DEATH-DROP CLASS, WHICH THIS POPULATION DID NOT WALK.
    //
    // `features/ecs/damage_drops.rs::drop_held_weapon` — the boss signature
    // gauntlet (`damage/boss_hit.rs`) and ordinary actor deaths
    // (`actor_hit.rs`) — spawns a `GroundItem`, which
    // `rollback_registration.rs` declares with `require_rollback::<GroundItem>`.
    // So a dropped weapon IS a rollback anchor and the census below has an
    // opinion about it.
    //
    // ⚠ Before this, `populate` spawned a sentry, a vortex well, a temporary
    // gravity well, a falling hazard and a portal shot — five things, and not
    // one ground item. The census asserted "every rollback-anchored entity has a
    // unique `SimId`" over a population that excluded the only road then known
    // to break it, and passed. ⭐ MEASURED 2026-09-03: adding an anonymous one
    // took the census from 27 entities to 28 and it FAILED, naming it — so the
    // assertion was always sensitive and the corpus was blind. The road was
    // fixed the same day (`SimId::death_drop`), and this keeps the class in the
    // population so a regression in the mint reddens here rather than nowhere.
    //
    // ⇒ Spawned with the identity the FIXED road mints, `{parent}/drop/weapon`,
    // so the fixture and the road agree on the shape rather than the fixture
    // inventing one.
    {
        let spec = ambition_platformer2d::characters::brain::held_item_by_id("shockwave")
            .expect("`shockwave` is an authored held item: trex_boss's signature gauntlet");
        // ⚠ THE SUBJECT ITSELF IS THE PARENT, not another `mint()`. A death
        // drop's parent is the body that died, and `death_drop` derives from it
        // rather than taking a sequence number — so borrowing one of the four
        // pre-allocated ids would both exhaust the supply (it did, "four ids")
        // and model the road wrongly.
        let dropped = ambition_platformer2d::platformer::sim_id::SimId::death_drop(
            &spawner, "weapon",
        );
        let world = sim.world_mut();
        world.spawn((
            ambition_platformer2d::held_items::GroundItem {
                spec,
                pos: bevy::math::Vec2::new(320.0, 96.0),
                vel: bevy::math::Vec2::ZERO,
                half_extent: bevy::math::Vec2::splat(18.0),
            },
            dropped,
            bevy::prelude::Name::new("death-drop gauntlet"),
        ));
        world.flush();
    }
    sim.world_mut().write_message(PortalFireIntent {
        origin: bevy::math::Vec2::new(224.0, 96.0),
        dir: bevy::math::Vec2::new(1.0, 0.0),
        channel: ambition_platformer2d::portal::PortalChannel::Gun(PortalGunColor::BLUE),
        // ⛔ MINTED, like every other spawn in this fixture. A shot is a
        // rollback anchor, and this fixture exists to prove anchors carry
        // identity — firing anonymously here would make the census assert
        // against a population the fixture itself broke.
        id: Some(mint()),
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
    use ambition_platformer2d::abilities::ranged::sentry::Sentry;
    use ambition_platformer2d::abilities::ranged::vortex::VortexWell;
    use ambition_platformer2d::boss_encounter::FallingHazard;
    use ambition_platformer2d::held_items::GroundItem;
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
        // ⛔ THE CLASS THAT ALREADY COST A DEFECT. `populate` has spawned a
        // death-dropped weapon since 2026-09-03, and this list did not name it —
        // so the family that broke the identity census was in the world and
        // outside every anti-vacuity check that guards this timeline. A corpus
        // is only widened once someone asserts the widening.
        ("death-dropped ground item", count::<GroundItem>(&mut sim)),
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

/// S4, the census half: every entity on the rollback timeline carries ONE
/// stable identity, and no two carry the same one.
///
/// `Rollback` is what makes an entity's state rewind; `SimId` is what lets a
/// resimulation say WHICH logical object came back (the checksum probes for
/// entity references fold through it, and an entity index is not stable across
/// a rewind). An anchored entity with no id is state that rewinds anonymously;
/// two entities with one id are a selection nobody can make deterministically.
/// Measured on the populated world, after the families have stepped and spawned
/// (bolts, sentry shots), not on the empty boot room.
#[test]
fn every_rollback_anchored_entity_has_a_unique_sim_id_on_the_populated_timeline() {
    use ambition_platformer2d::abilities::ranged::sentry::Sentry;
    use ambition_platformer2d::abilities::ranged::vortex::VortexWell;
    use ambition_platformer2d::boss_encounter::FallingHazard;
    use ambition_platformer2d::held_items::GroundItem;
    use ambition_platformer2d::platformer::gravity::TemporaryZone;
    use ambition_platformer2d::platformer::sim_id::SimId;
    use ambition_platformer2d::portal::PortalShot;
    use ambition_platformer2d::rollback::Rollback;
    use std::collections::BTreeMap;

    /// Walk every rollback anchor in `world` and fail on an anonymous one or a
    /// shared id. `when` says which moment produced the finding.
    fn census(world: &mut bevy::prelude::World, when: &str) -> usize {
        let mut anchored = world.query_filtered::<(
            Entity,
            Option<&SimId>,
            Option<&bevy::prelude::Name>,
        ), With<Rollback>>();
        let rows: Vec<(Entity, Option<String>, String)> = anchored
            .iter(world)
            .map(|(entity, id, name)| {
                (
                    entity,
                    id.map(|id| id.to_string()),
                    name.map(|n| n.as_str().to_string())
                        .unwrap_or_else(|| format!("<unnamed {entity}>")),
                )
            })
            .collect();
        let total = rows.len();
        let mut anonymous: Vec<String> = Vec::new();
        let mut by_id: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (entity, id, label) in rows {
            match id {
                Some(id) => by_id.entry(id).or_default().push(label),
                None => {
                    // An unnamed anonymous entity is unfindable; list what it is
                    // made of so the failure names the archetype.
                    let label = if label.starts_with("<unnamed") {
                        let parts: Vec<String> = world
                            .inspect_entity(entity)
                            .map(|components| {
                                components
                                    .map(|info| {
                                        info.name()
                                            .to_string()
                                            .rsplit("::")
                                            .next()
                                            .unwrap_or("")
                                            .to_string()
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        format!("{label} [{}]", parts.join(", "))
                    } else {
                        label
                    };
                    anonymous.push(label)
                }
            }
        }
        anonymous.sort();
        anonymous.dedup();
        let shared: Vec<(&String, &Vec<String>)> = by_id
            .iter()
            .filter(|(_, carriers)| carriers.len() > 1)
            .collect();
        assert!(
            anonymous.is_empty() && shared.is_empty(),
            "{when}: of {total} rollback-anchored entities:\n  {} carry NO SimId (rewind anonymously): {anonymous:#?}\n  \
             {} SimIds are carried by more than one entity: {shared:#?}",
            anonymous.len(),
            shared.len()
        );
        total
    }

    /// Which anchor classes are actually in the world, so a zero is a finding
    /// rather than a silently narrower corpus.
    fn walked(world: &mut bevy::prelude::World) -> Vec<(&'static str, usize)> {
        vec![
            ("sentry", world.query::<&Sentry>().iter(world).count()),
            ("vortex well", world.query::<&VortexWell>().iter(world).count()),
            (
                "temporary gravity zone",
                world.query::<&TemporaryZone>().iter(world).count(),
            ),
            (
                "falling hazard",
                world.query::<&FallingHazard>().iter(world).count(),
            ),
            ("portal shot", world.query::<&PortalShot>().iter(world).count()),
            (
                "death-dropped ground item",
                world.query::<&GroundItem>().iter(world).count(),
            ),
        ]
    }

    fn require_all(world: &mut bevy::prelude::World, when: &str) {
        let missing: Vec<&str> = walked(world)
            .into_iter()
            .filter(|(_, n)| *n == 0)
            .map(|(what, _)| what)
            .collect();
        assert!(
            missing.is_empty(),
            "{when}: no {missing:?} in the world, so this census proves nothing \
             about that class. Either `populate` stopped creating it or a seam \
             stopped spawning it — and both narrow what is walked without making \
             any assertion fail."
        );
    }

    let mut sim = rollback_sim();
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }
    populate(&mut sim);

    // ⛔⛔ THE CENSUS RUNS TWICE, AND THE FIRST RUN IS THE ONE THIS TEST DID NOT
    // HAVE.
    //
    // ⭐ MEASURED 2026-09-04, by asserting the class floor for the first time:
    // at frame 60 the world holds **28 anchors and NEITHER a vortex well NOR a
    // portal shot** — both are transient, and both had legitimately ended their
    // lives. So a census taken only at the end walks the population *"whatever
    // survives sixty frames"*, and an anonymous `VortexWell` — exactly the
    // defect this test exists to catch — would have rewound anonymously and
    // passed, because by the time anyone looked it was gone.
    //
    // ⇒ This is S4's own rule in a second costume: *"a census with no waiver
    // list is only as strong as the population it walks."* Widening `populate`
    // put the class in the world; only widening WHEN we look puts it in the
    // census.
    //
    // ⚠ The baseline moment is where every class is required, because that is
    // the moment they all exist. At frame 60 the durable ones are required and
    // the transient ones are not — asserting them there would be asserting that
    // a vortex well never expires.
    require_all(sim.world_mut(), "the populated baseline");
    let at_baseline = census(sim.world_mut(), "the populated baseline");
    assert!(
        at_baseline > 20,
        "premise: a populated baseline anchors more than {at_baseline} entities"
    );

    for frame in 0..60 {
        sim.step(busy(frame));
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));
    }

    // The durable half, after sixty frames of play — the classes that are still
    // meant to be here, plus every anchor play itself created.
    for what in ["sentry", "temporary gravity zone", "death-dropped ground item"] {
        let n = walked(sim.world_mut())
            .into_iter()
            .find(|(name, _)| *name == what)
            .map(|(_, n)| n)
            .unwrap_or(0);
        assert!(
            n > 0,
            "{what} did not survive sixty frames, so the second census below \
             says nothing about it — if that is now correct behaviour, move it \
             to the transient list rather than deleting the check"
        );
    }
    let after_play = census(sim.world_mut(), "after sixty frames of play");
    assert!(
        after_play > 20,
        "premise: a played timeline anchors more than {after_play} entities"
    );
}
