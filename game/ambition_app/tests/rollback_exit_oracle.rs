//! **The track-0 exit oracle: cross-feature state survives forced rollback.**
//!
//! Track 0's exit criterion, verbatim: *"a sync-test run that lands a melee
//! hit, spends armor, flips a switch, and breaks a brick across a forced
//! rollback window stays checksum-identical."* The registrations for combat,
//! equipment, switch, and breakable state each landed separately; this is the
//! one run that exercises them TOGETHER inside GGRS's save/rewind/resimulate
//! loop, where an unregistered interaction between two of them would finally
//! show as a checksum divergence.
//!
//! The scenario runs in `combat_calibration_lab` — the combat-verb calibration
//! room — which authors a patrol enemy, a striker pair, a breakable brick, and
//! the classify-console switch along one floor route. A steering policy walks
//! the route: absorb one enemy hit with a worn armor row, break the brick, land a
//! melee hit, and flip the switch. Every event is asserted from world state, so a
//! green run can't be vacuous — if the policy never actually landed the hit, the
//! test fails on the observation, not the checksum.
//!
//! ⚠ That last sentence is the whole design, and it is load-bearing because this
//! route DID stop doing two of the four things and stayed green for it. The
//! walker aimed at the breakable brick's centre, walked into a block whose top
//! face stands 32 above the floor, hopped onto it, and swung horizontally over
//! the thing it was breaking — so `brick_broken` and (gated behind it in route
//! order) `switch_flipped` were false on every pass. A steering policy is
//! CONTENT-SHAPED: it can stop reaching a prop because the room changed, with no
//! compile error and no failing assertion unless the assertion exists.

#![cfg(feature = "rl_sim")]

use ambition::characters::actor::BodyHealth;
use ambition::characters::equipment::{EquipmentRow, OnHit, WornEquipment};
use ambition_app::rl_sim::{AgentAction, AmbitionSim, SandboxSim, SandboxSimOptions, TimestepMode};
use bevy::prelude::{Entity, With, Without};

const ORACLE_ARMOR_ID: &str = "oracle_armor";
const MAX_FRAMES: usize = 2400;
/// Frames the route keeps resimulating even after every observation has landed.
/// See the early-exit guard in `walk_the_combat_route`.
const MIN_FRAMES: usize = 600;

fn oracle_sim() -> SandboxSim {
    SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("combat_calibration_lab")
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds in the calibration lab")
}

/// Dress the player in one armor row so the first enemy hit is an armor spend
/// rather than an HP loss. `WornEquipment` is registered rollback state, so
/// this pre-run mutation is part of frame-0 state like any authored loadout.
fn wear_oracle_armor(sim: &mut SandboxSim) {
    let world = sim.world_mut();
    let player = {
        let mut q =
            world.query_filtered::<Entity, With<ambition::platformer::markers::PrimaryPlayer>>();
        q.single(world)
            .expect("the sim boots exactly one primary player")
    };
    let row = EquipmentRow {
        id: ORACLE_ARMOR_ID.to_string(),
        on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: None }),
        ..Default::default()
    };
    match world.get_mut::<WornEquipment>(player) {
        Some(mut worn) => worn.rows.push(row),
        None => {
            world
                .entity_mut(player)
                .insert(WornEquipment::new(vec![row]));
        }
    }
    // Deep HP so the run cannot die: a player death triggers a sim-side room
    // RESET, and room reconstruction runs through Commands that no rollback
    // can undo — a reset inside the resim window is a guaranteed divergence
    // (observed at frame ~2147 during development: enemy HP snapped back to
    // full mid-brawl, then checksums split). That boundary is a recorded
    // Phase-5 finding, not this oracle's subject; the oracle stays inside the
    // proven envelope.
    if let Some(mut health) = world.get_mut::<BodyHealth>(player) {
        health.health.max = 200;
        health.health.current = 200;
    }
    // Direct world_mut mutations must become the rollback baseline — GGRS's
    // stored history predates them, and a restore would resurrect the
    // pre-setup state (harness contract on `world_mut`; GPT 5.6 review §2).
    sim.rebase_rollback_history()
        .expect("oracle armor setup becomes the rollback baseline");
}

/// Stage the player on the open arena floor as part of the frame-0 baseline.
///
/// The authored spawn corner is capped by a head-height ledge + rebound pad
/// (the room's parkour tutorial) — crossing it is a platforming exercise, and
/// platforming is not this oracle's subject. The oracle's route (spitter,
/// brick, striker, switch) all lives on the arena floor to the right, so the
/// baseline places the player just east of the hazard cycle (x=720; the
/// hazard band spans x 592-688 and eats a body staged inside it), like the
/// armor row:
/// a setup mutation folded into rollback frame zero by the rebase that follows.
fn stage_player_on_arena_floor(sim: &mut SandboxSim) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&mut ambition::platformer::body::BodyKinematics, With<ambition::platformer::markers::PrimaryPlayer>>();
    let mut kin = q
        .single_mut(world)
        .expect("the sim boots exactly one primary player");
    kin.pos = ambition::engine_core::Vec2::new(720.0, kin.pos.y);
    kin.vel = ambition::engine_core::Vec2::ZERO;
    sim.rebase_rollback_history()
        .expect("arena-floor staging becomes the rollback baseline");
}

struct OracleEvents {
    melee_landed: bool,
    armor_spent: bool,
    brick_broken: bool,
    switch_flipped: bool,
}

impl OracleEvents {
    fn all(&self) -> bool {
        self.melee_landed && self.armor_spent && self.brick_broken && self.switch_flipped
    }
}

/// **The exact props the route is supposed to act on, by authored id.**
///
/// The objectives used to read "any breakable is broken" and "any switch is on",
/// with the initial states stated in a comment (GPT 5.6, 2026-07-26). Author one
/// decorative already-shattered crate into this room, or one switch that starts
/// active, and both objectives pass before the player reaches anything — the same
/// defect class as A10's silhouette premise, in the test that was just cleaned up
/// for it.
///
/// Pinned by [`FeatureId`] rather than by `Entity`: bevy_ggrs DESTROYS and recreates
/// rollback entities, so a handle captured at calibration names nothing after the
/// first forced rewind. The authored id is the identity that survives, which is the
/// same reason the localizer projects entity references through `SimId`.
#[derive(Clone, Debug)]
struct OracleTargets {
    brick: String,
    switch: String,
}

/// Identify the route's targets and ASSERT they start in the state the route is
/// supposed to change. A calibration that cannot find them, or finds them already
/// done, fails here rather than producing a green run that proved nothing.
fn calibrate_targets(sim: &mut SandboxSim) -> OracleTargets {
    let world = sim.world_mut();

    let bricks: Vec<(String, bool)> = {
        let mut q = world.query::<(
            &ambition::combat::components::FeatureId,
            &ambition::combat::components::BreakableFeature,
        )>();
        q.iter(world)
            .map(|(id, feature)| (id.0.clone(), feature.broken()))
            .collect()
    };
    let switches: Vec<(String, bool)> = {
        let mut q = world.query::<(
            &ambition::combat::components::FeatureId,
            &ambition::actors::encounter::SwitchOn,
        )>();
        q.iter(world).map(|(id, on)| (id.0.clone(), on.0)).collect()
    };

    let (brick, already_broken) = bricks
        .first()
        .cloned()
        .expect("the calibration lab authors a breakable brick on the arena floor");
    assert!(
        !already_broken,
        "`{brick}` is ALREADY broken at calibration, so the brick objective is          satisfied before the route starts and proves nothing"
    );
    let (switch, already_on) = switches
        .first()
        .cloned()
        .expect("the calibration lab authors the classify-console switch");
    assert!(
        !already_on,
        "`{switch}` is ALREADY on at calibration, so the switch objective is          satisfied before the route starts and proves nothing"
    );
    OracleTargets { brick, switch }
}

/// Read every oracle observation from live world state.
fn observe(
    sim: &mut SandboxSim,
    targets: &OracleTargets,
    enemy_health_baseline: i32,
    events: &mut OracleEvents,
) {
    let world = sim.world_mut();

    let enemy_health: i32 = {
        let mut q = world
            .query_filtered::<&BodyHealth, Without<ambition::platformer::markers::PrimaryPlayer>>();
        q.iter(world).map(|body| body.health.current).sum()
    };
    if enemy_health < enemy_health_baseline {
        events.melee_landed = true;
    }

    {
        let mut q = world
            .query_filtered::<&WornEquipment, With<ambition::platformer::markers::PrimaryPlayer>>();
        if let Ok(worn) = q.single(world) {
            if !worn.wears(ORACLE_ARMOR_ID) {
                events.armor_spent = true;
            }
        }
    }

    // THE brick and THE switch the route is aimed at, by authored id — not "any
    // breakable" and "any switch", which a second prop in a different initial state
    // would satisfy for free.
    {
        let mut q = world.query::<(
            &ambition::combat::components::FeatureId,
            &ambition::combat::components::BreakableFeature,
        )>();
        if q.iter(world)
            .any(|(id, feature)| id.0 == targets.brick && feature.broken())
        {
            events.brick_broken = true;
        }
    }

    {
        let mut q = world.query::<(
            &ambition::combat::components::FeatureId,
            &ambition::actors::encounter::SwitchOn,
        )>();
        if q.iter(world).any(|(id, on)| id.0 == targets.switch && on.0) {
            events.switch_flipped = true;
        }
    }
}

/// Centers of the living enemies, in sim space.
///
/// Split out of `target_positions` because the probes below run a policy that
/// only chases enemies: building and iterating the brick and switch queries for
/// values they discard costs two fresh `QueryState`s on every simulated frame,
/// and these loops run 600-2400 frames.
fn enemy_positions(sim: &mut SandboxSim) -> Vec<(f32, f32)> {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(
        &ambition::platformer::body::BodyKinematics,
        &BodyHealth,
    ), Without<ambition::platformer::markers::PrimaryPlayer>>();
    q.iter(world)
        .filter(|(_, health)| health.health.current > 0)
        .map(|(kin, _)| {
            use bevy::math::bounding::BoundingVolume;
            let center = kin.aabb().center();
            (center.x, center.y)
        })
        .collect()
}

/// A prop's box, in sim space: center plus half-width.
///
/// The half-width is not decoration. The brick is a 48x48 block whose top face
/// stands 32 above the floor, so a policy that steers at its CENTER walks into it,
/// climbs it (the route's periodic hop is enough), and then swings horizontally
/// over the thing it is trying to break. That is exactly what this route did for
/// its whole existence — see `brick_standoff`.
#[derive(Clone, Copy, Debug)]
struct PropBox {
    x: f32,
    y: f32,
    half_w: f32,
}

/// Where to STAND to hit the brick, rather than where the brick is.
///
/// Approach from whichever side the player is already on, and stop clear of the
/// block's face by its half-width plus the swing's reach. The strike volume is
/// offset forward of the body, so standing flush against the face puts the volume
/// PAST the block — and standing on top of it puts the volume above it.
fn brick_standoff(brick: PropBox, px: f32) -> f32 {
    const STANDOFF: f32 = 26.0;
    if px <= brick.x {
        brick.x - brick.half_w - STANDOFF
    } else {
        brick.x + brick.half_w + STANDOFF
    }
}

/// Positions of the actionable things, in sim space, queried live so the
/// policy needs no knowledge of the room's coordinate frame.
fn target_positions(
    sim: &mut SandboxSim,
    targets: &OracleTargets,
) -> (Vec<(f32, f32)>, Option<PropBox>, Option<(f32, f32)>) {
    let enemies = enemy_positions(sim);
    let world = sim.world_mut();

    // The SAME props `observe` watches. Steering at one brick while asserting on
    // another is how a route can walk past its objective and still report it done.
    let brick = {
        let mut q = world.query::<(
            &ambition::combat::components::FeatureId,
            &ambition::combat::components::BreakableFeature,
            &ambition::engine_core::geometry::CenteredAabb,
        )>();
        q.iter(world)
            .find(|(id, feature, _)| id.0 == targets.brick && !feature.broken())
            .map(|(_, _, aabb)| PropBox {
                x: aabb.center.x,
                y: aabb.center.y,
                half_w: aabb.size().x / 2.0,
            })
    };

    let switch = {
        let mut q = world.query::<(
            &ambition::combat::components::FeatureId,
            &ambition::actors::encounter::SwitchFeature,
            &ambition::engine_core::geometry::CenteredAabb,
        )>();
        q.iter(world)
            .find(|(id, _, _)| id.0 == targets.switch)
            .map(|(_, _, aabb)| (aabb.center.x, aabb.center.y))
    };

    (enemies, brick, switch)
}

/// **Every registration that carries state across a rollback owns a probe.**
///
/// The localizer's promise, as written in `probes.rs`, is that "a component cannot
/// be rollback-registered and remain invisible to localization", and the planning
/// notes turned that into "all 99 registered components and resources were probed".
/// Neither was true. `record_probe` was called from five of the ten state-bearing
/// registration arms; the plain-clone and custom-checksum arms installed GGRS
/// snapshot and checksum machinery and no probe at all. `RoomSet`,
/// `LdtkRuntimeIndex`, `EncounterParticipants`, `PendingPlayerHitEvents` and —
/// pointedly — `ProjectileOwner`, whose remap is the fix the equipment divergence
/// turned on, were all invisible to the tool built to find exactly that.
///
/// The diagnostic below asserted only `probes > 0`, which cannot tell the difference
/// between full coverage and 5%. So its green result could not support the
/// conclusion the triage report drew from it — "every registered component and
/// resource came back identical" — and that conclusion was withdrawn.
///
/// This test is the forcing function. It is NOT `#[ignore]`d and it walks no route:
/// it builds the sim, then compares the rollback registry's state-bearing
/// descriptors against the probe set. A new registration arm that forgets its probe
/// fails here, at the point the coverage is lost, instead of silently narrowing
/// every future localizer run.
#[test]
fn every_state_bearing_rollback_registration_owns_a_localization_probe() {
    let sim = oracle_sim();
    let registry = sim
        .world()
        .resource::<ambition::runtime::rollback::RollbackRegistry>();
    let probed = sim
        .world()
        .resource::<ambition::runtime::rollback::RollbackChecksumProbes>()
        .type_names();

    let mut state_bearing = 0usize;
    let mut missing: Vec<String> = Vec::new();
    for descriptor in registry.descriptors() {
        if !descriptor.kind.carries_state() {
            continue;
        }
        state_bearing += 1;
        if !probed.contains(descriptor.type_name.as_str()) {
            missing.push(format!(
                "  {} [{}] {}",
                descriptor.name,
                descriptor.kind.canonical_name(),
                descriptor.type_name
            ));
        }
    }
    // Vacuity guard: a composition that registered nothing would pass the coverage
    // check trivially, and this test would then be asserting that zero equals zero
    // for the rest of the project's life.
    assert!(
        state_bearing > 50,
        "only {state_bearing} state-bearing registrations were found; this sim is \
         supposed to compose the whole game, so the comparison below would be \
         vacuous"
    );
    missing.sort();
    assert!(
        missing.is_empty(),
        "{} of {state_bearing} state-bearing rollback registrations have NO \
         localization probe, so the localizer is blind to them and cannot support \
         any statement about what did or did not survive a restore:\n{}",
        missing.len(),
        missing.join("\n")
    );
    println!(
        "[probe coverage] {state_bearing} state-bearing registrations, {} probes",
        probed.len()
    );
}

/// **Every probe that can see only PRESENCE is written down, with a reason.**
///
/// The test above compares type NAMES, which a presence-only probe satisfies while
/// reporting nothing about the value (GPT 5.6, 2026-07-26). So "254 of 254 probed"
/// was true and weaker than it read: a restore that put back the right NUMBER of
/// `ProjectileOwner`s and pointed one bolt at the wrong body changed no census.
///
/// The response is not to demand a value projection everywhere — some registrations
/// genuinely have none, and a checksum the GGRS aggregate must not see cannot be
/// invented here. It is to make the weakness ENUMERATED. Every presence-only probe
/// appears below with the reason it is weak; a new one fails this test until somebody
/// writes down why, and an entry that stops being presence-only fails it too, so the
/// list cannot rot into a description of an older world.
///
/// This is the same discipline as the coverage sweep's waiver list, applied to the
/// other axis: the sweep says which types are unlooked-at, and this says which of the
/// looked-at ones are only counted.
#[test]
fn every_presence_only_probe_is_named_with_its_reason() {
    // (type name suffix, why this registration cannot see its own value)
    //
    // Zero-sized markers are NOT here: `ProbeStrength::Complete` distinguishes them
    // mechanically, because presence is not a partial view of a marker's state, it
    // is all of it. What remains is state with a value that no projection measures.
    const PRESENCE_ONLY: &[(&str, &str)] = &[
        // ── Authored at spawn, never written again ───────────────────────────
        //
        // The registration exists because bevy_ggrs DESTROYS and recreates rollback
        // entities, so an unregistered authored component is simply absent
        // afterwards. Its value cannot drift, so a carrier count answers the only
        // question a restore can get wrong about it. The premise is "nothing mutates
        // this after spawn" — if that stops being true for one of these, it needs a
        // value probe, and this line is where to notice.
        ("::ActorConfig", "authored actor definition"),
        ("::BossConfig", "authored boss definition"),
        ("::BossOverrides", "authored spawn overrides"),
        ("::BossCapability", "authored capability set"),
        ("::EncounterDef", "authored encounter definition"),
        ("::EncounterRegistry", "authored registry"),
        ("::EncounterObjective", "authored objective"),
        ("::EncounterTrack", "authored track"),
        ("::EncounterLockWall", "authored staging geometry"),
        ("::EncounterCameraZoom", "authored staging camera"),
        ("::EncounterMusicRequest", "authored music request"),
        ("::Encounter", "authored encounter handle"),
        ("::AuthoredHurtboxes", "authored hurtbox document"),
        ("::SwitchFeature", "authored switch payload"),
        ("::BreakableFeature", "authored breakable payload"),
        ("::ChestFeature", "authored chest payload"),
        ("::PickupFeature", "authored pickup payload"),
        ("::HazardFeature", "authored hazard payload"),
        ("::FeatureId", "authored stable id"),
        ("::FeatureName", "authored name"),
        ("::ActorIdentity", "authored identity"),
        ("::ActorInteraction", "authored interaction payload"),
        ("::ActorRenderSize", "authored size"),
        ("::ActorSpriteOffset", "authored offset"),
        ("::PickupArt", "authored art id"),
        ("::PogoPolicy", "authored policy"),
        ("::FriendlyFire", "authored policy"),
        ("::FactionRelations", "authored relation matrix"),
        ("::ActorFaction", "authored faction"),
        ("::PlayerSlot", "authored slot index"),
        ("::CombatTuning", "authored tuning"),
        ("::CombatCapabilities", "authored capability set"),
        ("::CombatKit", "authored kit"),
        ("::ActorMoveset", "authored moveset"),
        ("::IdentityKit", "authored kit"),
        ("::ActionSet", "authored action set"),
        ("::StashedActionSet", "authored action set, stashed"),
        ("::HeldItem", "authored item spec"),
        ("::GroundItem", "authored item spec"),
        ("::MountSlot", "authored mount geometry"),
        ("::MountedSize", "authored size"),
        ("::Mountable", "authored capability"),
        ("::CanPilot", "authored capability"),
        ("::Mass", "authored mass"),
        ("::RoomGeometry", "authored room geometry"),
        ("::ActiveRoomMetadata", "authored room metadata"),
        ("::RoomMusicRequest", "authored music request"),
        ("::PortalPolicy", "authored policy"),
        ("::BossDeathAnimation", "authored animation spec"),
        ("::SpritePosedBody", "authored per-pose body table"),
        ("::LimbRig", "authored rig"),
        ("::Limb", "authored limb"),
        ("::QuestRegistry", "authored quest registry"),
        ("bevy_ecs::name::Name", "authored debug name"),
        // ── Holds ENTITY handles: needs the stable-identity projection ───────
        //
        // The same treatment `ProjectileOwner` now has
        // (`rollback_component_clone_entity_ref`). A raw handle differs after a load
        // by design, so these cannot be probed by value until each names which field
        // is the reference. That is the next piece of work on this axis, and it is
        // the one with real failure modes behind it — a remap that lands on the wrong
        // body is invisible today.
        (
            "::RidingOn",
            "entity handle: wants the stable-identity projection",
        ),
        (
            "::MountedBrainCache",
            "entity handle: wants the stable-identity projection",
        ),
        (
            "::PossessionState",
            "entity handle: wants the stable-identity projection",
        ),
        (
            "::HitboxHits",
            "entity SET: wants the stable-identity projection",
        ),
        ("ambition_vfx::Hitbox", "carries its owner handle"),
        ("::StrikeVolume", "carries its owner handle"),
        ("::HitboxOnHit", "carries per-victim fired handles"),
        (
            "::SwitchActivationQueue",
            "queued activations carry target handles",
        ),
        ("::PortalFrameHistory", "per-frame body handles"),
        ("::PortalEmission", "carries the emitting portal handle"),
        ("::PortalTransit", "carries the transiting pair"),
        ("::PlacedPortal", "carries its partner handle"),
        ("::PortalShot", "carries its firer handle"),
        // ── Large or derived-shaped state: a projection would cost more than it
        //    buys, or the value is republished every tick anyway ──────────────
        ("::SandboxSave", "the whole save document"),
        (
            "::OwnedItems",
            "inventory set; wants a canonical projection",
        ),
        (
            "::WornEquipment",
            "equipment rows; wants a canonical projection",
        ),
        (
            "::DamageableVolumes",
            "republished every tick from the hurtbox resolver",
        ),
        ("::PogoTargetVolumes", "republished every tick"),
        ("::BodyAnimFacts", "republished every tick from motion"),
        ("::ActorAnimOverride", "republished from the move clock"),
        ("::LimbIntents", "republished every tick by the limb router"),
        (
            "::LimbRouteState",
            "republished every tick by the limb router",
        ),
        ("::AbilityBase", "refreshed every tick from the ability set"),
        ("::PlayerBlinkCameraState", "presentation camera state"),
        ("::GravityFlipSwitch", "authored switch payload"),
        ("::CutRopeHeavyObjectCycle", "authored boss cycle"),
        (
            "::PortalGun",
            "held-gun state; wants a canonical projection",
        ),
        (
            "::PortalGunPickup",
            "arm timer; wants a canonical projection",
        ),
        ("::InputStreamRecorder", "the recorded stream itself"),
        (
            "bevy_transform::components::transform::Transform",
            "presentation transform, republished from BodyKinematics",
        ),
        // ── Derived declarations ─────────────────────────────────────────────
        // A derived component is legitimately ABSENT right after a load, so its
        // contract is tested across resimulation, not restore. Presence catches the
        // failure that actually shipped (`ProjectileOwner`'s unkept derived promise:
        // nothing rebuilt it at all). It cannot catch a value rebuilt wrongly, and
        // `declare_rollback_derived_component_state` is the arm for the ones that can
        // do better.
        ("derived:", "derived state; see the note above"),
    ];

    let sim = oracle_sim();
    let probes = sim
        .world()
        .resource::<ambition::runtime::rollback::RollbackChecksumProbes>();
    let presence_only = probes.presence_only_type_names();
    let derived: std::collections::BTreeSet<&str> = probes
        .probes()
        .filter(|probe| probe.is_derived())
        .map(|probe| probe.type_name)
        .collect();

    let mut unlisted: Vec<&str> = Vec::new();
    for type_name in &presence_only {
        if derived.contains(type_name) {
            continue;
        }
        if PRESENCE_ONLY
            .iter()
            .any(|(needle, _)| *needle != "derived:" && type_name.contains(needle))
        {
            continue;
        }
        unlisted.push(type_name);
    }
    unlisted.sort();
    assert!(
        unlisted.is_empty(),
        "{} snapshot registration(s) carry a PRESENCE-ONLY localization probe and are          not named in this test's list. A presence probe satisfies the coverage test          above while seeing nothing of the value, so each one needs either a value          projection (`rollback_component_clone_entity_ref` for a handle,          `rollback_component_clone_checksum` for anything else) or an entry here          saying why it cannot have one:
  {}",
        unlisted.len(),
        unlisted.join("
  ")
    );

    // And the reverse: an entry that is no longer presence-only must be removed, or
    // the list becomes a description of a world that has moved on.
    let mut stale: Vec<&str> = Vec::new();
    for (needle, _) in PRESENCE_ONLY {
        if *needle == "derived:" {
            continue;
        }
        let probed_at_all = probes
            .probes()
            .any(|probe| probe.type_name.contains(needle));
        let still_weak = presence_only.iter().any(|name| name.contains(needle));
        if probed_at_all && !still_weak {
            stale.push(needle);
        }
    }
    assert!(
        stale.is_empty(),
        "these types now have VALUE probes and must be dropped from the          presence-only list: {stale:?}"
    );

    let (complete, value, presence) = probes.strength_tally();
    println!(
        "[probe strength] {} probes: {value} value, {complete} complete (zero-sized, \
         presence IS the value), {presence} presence-only ({} of those derived)",
        probes.len(),
        presence_only
            .iter()
            .filter(|name| derived.contains(*name))
            .count()
    );
}

/// Sharpest probe: no armor, no attacks — stand in the striker's path and take
/// repeated hits. Isolates the victim-side damage path under rollback: every
/// hit crosses the staging FIFO, the striker's swing runs its strike volume
/// through GGRS despawn/respawn, and the post-hit clock ramp rewinds. This
/// caught (in order) the unregistered `Collected` latch, the in-flight
/// victim-hit loss (`PendingPlayerHitEvents`), and the strike-volume family
/// living outside the rollback envelope.
#[test]
fn a_player_taking_hp_damage_survives_rollback() {
    let mut sim = oracle_sim();
    let mut last_hp = i32::MAX;
    for frame in 0..600 {
        let enemies = enemy_positions(&mut sim);
        let obs = sim.observation();
        let (px, _) = obs.player_pos;
        if obs.hp != last_hp {
            eprintln!("[hit] frame {frame}: player_hp={} px={px:.1}", obs.hp);
            last_hp = obs.hp;
        }
        let nearest = enemies
            .iter()
            .copied()
            .map(|(x, y)| (x, y, (x - px).abs()))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let action = match nearest {
            Some((x, _, d)) if d > 10.0 => AgentAction::move_x((x - px).signum()),
            _ => AgentAction::default(),
        };
        sim.step(action);
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));
    }
}

/// Minimal repro probe: kill the patrol enemy, then stand still through its
/// in-place revive and re-aggro. Isolates the death → respawn-timer → revive →
/// re-engage cycle that the full oracle exposed.
#[test]
fn enemy_death_and_inplace_revive_survive_rollback() {
    let mut sim = oracle_sim();
    wear_oracle_armor(&mut sim);
    let mut phase = "approach";
    let mut last_hp = i32::MAX;
    for frame in 0..900 {
        let enemies = enemy_positions(&mut sim);
        let obs = sim.observation();
        let (px, _) = obs.player_pos;
        let nearest = enemies
            .iter()
            .copied()
            .map(|(x, y)| (x, y, (x - px).abs()))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let (hp, count) = {
            let world = sim.world_mut();
            let mut q = world.query_filtered::<&BodyHealth, Without<ambition::platformer::markers::PrimaryPlayer>>();
            // One pass: this runs every frame for 900 frames, and the two
            // values only feed the change-triggered log line below.
            q.iter(world)
                .fold((0, 0), |(hp, count), b| (hp + b.health.current, count + 1))
        };
        if hp != last_hp {
            eprintln!(
                "[repro] frame {frame}: phase={phase} enemy_hp={hp} enemies={count} px={px:.1}"
            );
            last_hp = hp;
        }
        let action = match (phase, nearest) {
            ("approach", Some((x, _, d))) => {
                if d < 60.0 {
                    phase = "kill";
                }
                AgentAction::move_x((x - px).signum())
            }
            ("kill", Some((x, _, d))) => AgentAction {
                move_x: if d < 30.0 { 0.0 } else { (x - px).signum() },
                attack: frame % 6 == 2,
                ..AgentAction::default()
            },
            _ => AgentAction::default(),
        };
        sim.step(action);
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame} (phase {phase}): {error}"));
    }
}

/// Narrowing probe: the lab must be checksum-stable with NO player input at
/// all — only the enemy brains, patrol paths, and feature timers running. A
/// divergence here isolates the fault to the room's autonomous population
/// before the full oracle's combat even starts.
///
/// During development this test carried a five-variant despawn matrix
/// (no_enemies / no_brick / no_switch / no_pickups) plus a print-only pickup
/// census — the bisection tools that cornered the `Collected` latch. Those
/// cost five extra sim boots per suite run and their findings are fixed and
/// pinned elsewhere, so the standing probe keeps only the intact room
/// (2026-07-23 rollback review: trim the diagnostic matrix). Resurrect the
/// matrix from git history if this ever goes red again.
#[test]
fn the_calibration_lab_is_checksum_stable_at_rest() {
    let mut sim = oracle_sim();
    for frame in 0..48 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));
    }
}

/// Walk the calibration lab's combat route under a forced rollback window,
/// returning the divergence report instead of panicking so a caller can sweep
/// several worlds and compare which ones diverge.
///
/// `frames_run` and the observed events come back on both paths: a divergence
/// still wants to say what the route had achieved when it hit.
///
/// The fourth return is a **per-frame** union of unaccounted components. A
/// one-shot sweep at failure time cannot see TRANSIENT sim entities — an attack's
/// hit volume, a projectile, a debris chunk — because they live for a handful of
/// frames and are gone by the time anyone samples. Those are exactly the entities
/// a rewind has to reproduce, so the census walks every frame and unions.
fn walk_the_combat_route(
    sim: &mut SandboxSim,
) -> (
    Result<(), String>,
    OracleEvents,
    usize,
    std::collections::BTreeMap<String, usize>,
) {
    let mut census: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let enemy_health_baseline: i32 = {
        let world = sim.world_mut();
        let mut q = world
            .query_filtered::<&BodyHealth, Without<ambition::platformer::markers::PrimaryPlayer>>();
        let total = q.iter(world).map(|body| body.health.current).sum();
        assert!(
            total > 0,
            "the calibration lab booted with no live enemies — the melee-hit \
             observation would be vacuous"
        );
        total
    };
    // The props the route must change, with their initial states CHECKED. Anything
    // this cannot find, or finds already done, fails here — before a run that would
    // otherwise report those objectives satisfied by the room's authoring.
    let targets = calibrate_targets(&mut *sim);

    let mut events = OracleEvents {
        melee_landed: false,
        armor_spent: false,
        brick_broken: false,
        switch_flipped: false,
    };

    let mut frames_run = 0usize;
    for frame in 0..MAX_FRAMES {
        let (enemies, brick, switch) = target_positions(&mut *sim, &targets);
        let player = sim.observation();
        let (px, _py) = player.player_pos;

        // The next objective, in route order: take the armor hit from the
        // nearest enemy first, then the brick, then any remaining melee proof,
        // then the switch. The brick outranks enemies once armor is spent
        // because the lab's enemies revive in place — "nearest melee target"
        // forever re-selects the respawned neighbor and the walk never leaves
        // the spawn corner.
        let nearest_enemy = enemies
            .iter()
            .copied()
            .map(|(x, y)| (x, y, (x - px).abs()))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        // Is the brick the current objective? Kept as its own flag because two
        // parts of the action below depend on it, and re-deriving the condition in
        // both is how they drift apart.
        let breaking_brick = events.armor_spent && !events.brick_broken && brick.is_some();
        let target_x = if events.switch_flipped {
            px
        } else if !events.armor_spent {
            nearest_enemy.map(|(x, _, _)| x).unwrap_or(px)
        } else if breaking_brick {
            brick.map(|b| brick_standoff(b, px)).unwrap_or(px)
        } else if !events.melee_landed {
            nearest_enemy.map(|(x, _, _)| x).unwrap_or(px)
        } else if let Some((x, _)) = switch {
            x
        } else {
            px
        };

        let dx = target_x - px;
        let near = dx.abs() < 70.0;
        // Until the armor row is spent, walk INTO the target without swinging —
        // the point is to TAKE a hit, and a policy that kills everything first
        // never exercises the equipment path.
        let brawling = events.armor_spent;
        let action = AgentAction {
            move_x: if dx.abs() < 8.0 { 0.0 } else { dx.signum() },
            // Melee in reach; the moveset faces along move_x.
            attack: brawling && near && frame % 6 == 2,
            // Interact pulses flip the switch once the player stands in its
            // region; harmless elsewhere (single-press Up never triggers).
            interact: near && frame % 10 == 5,
            // An occasional hop un-sticks the walk against bodies and debris —
            // but NEVER while the brick is the objective. The brick's top face
            // stands 32 above the floor, well within one hop, and a walker that
            // lands on top of it is a walker whose forward strike sweeps the air
            // above the block forever. This route spent its entire existence up
            // there (A16): standing at x=900 on a brick centred at (904, 728),
            // swinging, and reporting no break.
            jump: !breaking_brick && frame % 90 == 40,
            jump_held: !breaking_brick && frame % 90 >= 40 && frame % 90 < 48,
            ..AgentAction::default()
        };

        sim.step(action);
        for (name, count) in crate::rollback_coverage::unaccounted_components(sim) {
            let seen = census.entry(name).or_default();
            *seen = (*seen).max(count);
        }
        if let Err(error) = sim.rollback_health() {
            let late = crate::rollback_coverage::unaccounted_components(sim);
            let report = format!(
                "frame {frame}: resimulation diverged: {error} \
                 (events at failure: melee={} armor={} brick={} switch={}, px={px:.1}, target_x={target_x:.1})\n\
                 unaccounted components at failure (candidates inserted mid-run): {late:?}",
                events.melee_landed, events.armor_spent, events.brick_broken, events.switch_flipped
            );
            return (Err(report), events, frame + 1, census);
        }
        let before = (
            events.melee_landed,
            events.armor_spent,
            events.brick_broken,
            events.switch_flipped,
        );
        observe(sim, &targets, enemy_health_baseline, &mut events);
        let after = (
            events.melee_landed,
            events.armor_spent,
            events.brick_broken,
            events.switch_flipped,
        );
        if before != after {
            eprintln!(
                "[oracle] frame {frame}: events now melee={} armor={} brick={} switch={}",
                after.0, after.1, after.2, after.3
            );
        }
        frames_run = frame + 1;
        // Do not stop the moment the route is complete. Every event now lands
        // inside the first ~180 frames, and the divergence this oracle was built
        // for lived at frames 149-151 — a run that exits at the last event would
        // have a rollback window barely wider than the bug it is guarding. Keep
        // resimulating to the floor, holding position, so the enemies' revive and
        // re-aggro cycles keep churning combat state inside the window.
        if events.all() && frames_run >= MIN_FRAMES {
            break;
        }
    }
    (Ok(()), events, frames_run, census)
}

/// **Track 0's exit criterion, in one run.** All four events, checksum-identical.
///
/// This was `#[ignore]`d and red for a long time, and the history is worth keeping
/// because two different failures were tangled together in it:
///
/// * a genuine **value divergence** at frames ~[149, 150, 151] —
///   `docs/planning/triage/rollback-equipment-oracle-divergence.md` records the
///   bisection. `IdentityKit` and `PlayerVisual` were found and fixed on the way;
///   the actual cause was `ProjectileOwner` declared rollback-DERIVED on the
///   promise of a system whose query could not see enemy projectiles. Fixed, and
///   the quarantine lifted;
/// * a **route** that never touched either prop. Hidden by the first failure — the
///   checksum blew up at frame ~153, so nobody saw how far the walker got. It
///   steered at the brick's centre, climbed the block, and swung over it. Fixed by
///   standing off the face and not hopping while the brick is the objective.
///
/// The order matters for anyone reading the git history: the determinism fix made
/// the route's silence visible, and only then could the two prop assertions be
/// restored. A green checksum over a route that does nothing is the failure mode
/// this file is most exposed to, which is why every event is observed from world
/// state and every observation is asserted.
#[test]
fn combat_equipment_switch_and_breakable_survive_forced_rollback_identically() {
    let mut sim = oracle_sim();
    wear_oracle_armor(&mut sim);
    stage_player_on_arena_floor(&mut sim);

    let (health, events, frames_run, census) = walk_the_combat_route(&mut sim);
    assert!(
        census.is_empty(),
        "state lived on a simulated entity at some point during the route that \
         GGRS will not rewind. These were invisible to the one-shot sweep in \
         `rollback_coverage` because they are TRANSIENT — spawned and despawned \
         inside the route — which is why this census samples every frame:\n{census:#?}"
    );
    health.unwrap_or_else(|report| panic!("{report}"));

    assert!(
        events.melee_landed,
        "no melee hit landed in {frames_run} frames — the oracle never \
         exercised combat state, so its checksum agreement proves nothing"
    );
    assert!(
        events.armor_spent,
        "the armor row was never consumed in {frames_run} frames — the oracle \
         never exercised equipment state"
    );
    // A16: these two are asserted again, and the route reaches them.
    //
    // They were replaced by an inverted guard for one run, because both were false
    // on every pass of this route — before and after the determinism fix — and
    // asserting something the walker had never done would have made the oracle red
    // for a reason unrelated to the determinism it exists to guard. The inverted
    // guard is what reported that the route had started reaching them.
    //
    // What was actually wrong was the STEERING, not the props: the policy aimed at
    // the brick's centre, walked into a 48x48 block whose top face stands 32 above
    // the floor, and the route's periodic hop put the player ON it — swinging
    // horizontally over the thing it was trying to break, for 2400 frames. It now
    // stops clear of the face and does not hop while the brick is the objective.
    // The switch was never unreachable; it was simply gated behind the brick in
    // route order.
    assert!(
        events.brick_broken,
        "the brick was never broken in {frames_run} frames — Track 0's exit \
         criterion names it explicitly, and breakable state is registered rollback \
         state that nothing else in this suite exercises inside a rewind window"
    );
    assert!(
        events.switch_flipped,
        "the switch was never flipped in {frames_run} frames — the walker either \
         never reached x≈1132 or its interact pulses did not land"
    );

    let stats = sim
        .rollback_execution_stats()
        .expect("GGRS instrumentation is installed");
    assert!(
        stats.load_runs > 0,
        "no LoadWorld request was ever issued, so nothing was rewound and the \
         checksum agreement above is agreement with itself: {stats:?}"
    );
    assert!(
        stats.advance_runs > frames_run as u64,
        "resimulation must execute more GGRS frames than the {frames_run} \
         harness steps, or the same frames were never replayed: {stats:?}"
    );
}

/// **Which population does the divergence need?** — the localizer, opt-in.
///
/// When the oracle above goes red it names a frame and nothing else: a GGRS
/// sync-test reports one aggregate checksum, so "frames [149, 150, 151] differ"
/// is the whole story it can tell. This walks the SAME route through worlds with
/// one entity class removed at a time. A variant that goes green names the class
/// the divergence needs, which is the question the aggregate checksum cannot
/// answer and the one a fix has to start from.
///
/// `#[ignore]` because it boots five sims and re-walks a ~150-frame route in
/// each. It is a bisection tool, not a standing guard — the oracle is the guard.
/// Run it with `./run_tests.sh --heavy -k which_population`.
#[test]
#[ignore = "diagnostic bisection: five sim boots; run when the oracle above is red"]
fn which_population_does_the_rollback_divergence_need() {
    // No `no_enemies` variant: the route SPENDS ARMOR by taking an enemy hit, so
    // a world without enemies cannot walk it at all (the helper's own vacuity
    // guard says so). The removable classes are the ones the route passes but
    // does not depend on.
    let mut findings: Vec<String> = Vec::new();
    for variant in ["intact", "no_brick", "no_switch", "no_pickups"] {
        let mut sim = oracle_sim();
        wear_oracle_armor(&mut sim);
        stage_player_on_arena_floor(&mut sim);
        {
            let world = sim.world_mut();
            let doomed: Vec<Entity> = match variant {
                // NOT the player: the route needs a body to drive.
                "no_enemies" => {
                    let mut q = world.query_filtered::<Entity, (
                        With<BodyHealth>,
                        Without<ambition::platformer::markers::PrimaryPlayer>,
                    )>();
                    q.iter(world).collect()
                }
                "no_brick" => {
                    let mut q = world
                        .query_filtered::<Entity, With<ambition::combat::components::BreakableFeature>>();
                    q.iter(world).collect()
                }
                "no_switch" => {
                    let mut q = world
                        .query_filtered::<Entity, With<ambition::actors::encounter::SwitchFeature>>(
                        );
                    q.iter(world).collect()
                }
                "no_pickups" => {
                    let mut q = world
                        .query_filtered::<Entity, With<ambition::combat::components::PickupFeature>>();
                    q.iter(world).collect()
                }
                _ => Vec::new(),
            };
            for entity in doomed {
                world.despawn(entity);
            }
        }
        // The despawns are setup, not gameplay: they become frame-0 state, or
        // GGRS would rewind INTO a world that still had them.
        sim.rebase_rollback_history()
            .expect("variant despawn setup becomes the rollback baseline");

        let (health, events, frames_run, census) = walk_the_combat_route(&mut sim);
        if !census.is_empty() {
            findings.push(format!("  {variant:<12} TRANSIENT UNACCOUNTED: {census:?}"));
        }
        match health {
            Ok(()) => findings.push(format!(
                "  {variant:<12} CLEAN over {frames_run} frames \
                 (melee={} armor={} brick={} switch={})",
                events.melee_landed, events.armor_spent, events.brick_broken, events.switch_flipped
            )),
            Err(report) => findings.push(format!("  {variant:<12} DIVERGED — {report}")),
        }
    }
    panic!(
        "rollback divergence population sweep (this test always reports; read \
         the variants):\n{}",
        findings.join("\n")
    );
}

/// **Which COMPONENT does the divergence live in?** — per-component localization.
///
/// The sibling localizer above answers "which entity class", by bisection over
/// five sim boots. This answers the sharper question directly, in one run: for
/// every registered rollback component, census its checksum projection when GGRS
/// saves a frame, and census it again when GGRS loads that same frame. A component
/// whose census changed did not survive its own snapshot, and it is named.
///
/// This is the tool the triage doc ends by asking for. Two things it deliberately
/// does NOT do:
///
/// * it does not compare two independent runs — that reproduces the aggregate
///   checksum's blindness with more machinery;
/// * it does not fold per-entity checksums in iteration order. bevy_ggrs destroys
///   and recreates rollback entities, so ids and archetype order both change across
///   a load; an order-dependent fold would report every component as diverging.
///   XOR plus a count is invariant under reordering and still catches a changed
///   value, a lost carrier, or a gained one.
///
/// `#[ignore]` for cost, like its sibling: it censuses every registered type on
/// every save and every load. Run it with
/// `./run_tests.sh --heavy -k which_component`.
#[test]
#[ignore = "diagnostic: per-component restore census on every save/load; run when the oracle is red"]
fn which_component_does_the_rollback_divergence_live_in() {
    let mut sim = oracle_sim();
    sim.world_mut()
        .insert_resource(ambition::runtime::rollback::RollbackRestoreAudit::enabled());
    wear_oracle_armor(&mut sim);
    stage_player_on_arena_floor(&mut sim);

    let probes = sim
        .world()
        .resource::<ambition::runtime::rollback::RollbackChecksumProbes>()
        .len();
    assert!(
        probes > 0,
        "no localization probes were registered, so this test can only ever \
         report success — the probe registration is coupled to the checksum \
         registration precisely so that cannot happen silently"
    );

    let _ = walk_the_combat_route(&mut sim);

    let audit = sim
        .world()
        .resource::<ambition::runtime::rollback::RollbackRestoreAudit>();
    // Vacuity guard FIRST. A localizer that reports "nothing diverged" while never
    // comparing anything launders an absence of evidence into evidence of absence,
    // which is the single most useless thing a diagnostic can do.
    assert!(
        audit.comparisons > 0 && audit.resimulations > 0,
        "the audit compared nothing, so its verdict is meaningless: {}",
        audit.coverage()
    );
    assert!(
        audit.divergences.is_empty(),
        "{} registered component(s) did not survive their own snapshot across \
         {probes} probed types. THIS IS THE ANSWER the aggregate checksum could \
         not give:\n{}",
        audit.diverging_types().len(),
        audit.report()
    );
    // Report coverage on success too: the useful negative result is "N frames were
    // compared and every registered component came back identical", not "no
    // assertion fired".
    println!("[localizer] {}", audit.coverage());
}
