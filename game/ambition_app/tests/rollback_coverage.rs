//! Runtime census for rollback coverage.
//!
//! The test boots the real simulation and requires authoritative component and
//! resource state to be registered, explicitly derived, or narrowly waived.
//! Entity composition catches state inserted through `Commands` as well as
//! ordinary system writes. New simulation state should normally be registered;
//! derived state must be rebuilt before use after rewind.

#![cfg(feature = "rl_sim")]

use std::collections::{BTreeMap, BTreeSet};

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness, TimestepMode};
use bevy::prelude::*;

/// Type-name substrings that are NOT authoritative simulation state.
///
/// Each entry is a claim that rewinding the named state would be meaningless or
/// harmful, plus the reason. This list is the part of the test that can lie —
/// keep it short and justified.
const WAIVED: &[(&str, &str)] = &[
    // Presentation / observation: derived from sim facts, never authoritative.
    (
        "ambition_sim_view::",
        "read model, rebuilt from sim facts each frame",
    ),
    (
        "ambition_render::",
        "presentation: draws the sim, never authors it",
    ),
    ("ambition_vfx::", "presentation effects"),
    ("ambition_sfx::", "presentation audio"),
    ("ambition_audio::", "presentation audio"),
    ("ambition_portal2d_presentation::", "presentation"),
    ("ambition_load_presentation::", "presentation"),
    ("ambition_menu", "UI"),
    ("ambition_settings_menu::", "UI"),
    ("ambition_inventory_ui::", "UI"),
    ("ambition_ui_nav::", "UI"),
    ("ambition_dialog::", "narrative view state"),
    (
        "ambition_cutscene::",
        "scripted presentation sequence state. ⚠ the namespace waiver is NARROWER          than it reads: `ActiveCutscene` (`cutscene.playback`) and          `LastCutsceneRoom` (`cutscene.last_room`) are both REGISTERED, because          playback decides whether the participant can act and the room memory          decides whether a trigger fires. What this waives is the rest — the          library, the bindings table, the skip accumulator the HUD draws",
    ),
    ("ambition_game_shell::", "host shell/session chrome"),
    ("ambition_load::", "load coordination, not gameplay truth"),
    // Dev / host / infrastructure.
    ("ambition_dev_tools::", "developer tooling, not gameplay"),
    (
        "ambition_gameplay_trace::",
        "flight recorder; already replay-gated",
    ),
    ("ambition_asset_manager::", "asset plumbing"),
    (
        "ambition_input::",
        "device input; the GGRS input stream is the seam",
    ),
    ("ambition_touch_input::", "device input"),
    ("ambition_sprite_sheet::", "sprite metadata / asset binding"),
    // Authored, immutable-by-contract content bound by PreparedContentIdentity.
    (
        "ambition_entity_catalog::",
        "authored contract, immutable during a session",
    ),
    // ── The SESSION ROOT entity ──────────────────────────────────────────────
    //
    // Pulled into the population once it was derived from the rollback vocabulary:
    // the root carries `RoomSet`, which is a rollback anchor, so the root is an
    // entity the rollback participates in. What it carries besides `RoomSet` is
    // session ACTIVATION identity, decided before the first simulated frame and
    // never written again — rewinding it could only change which session the sim
    // thinks it is in.
    (
        "ambition_platformer2d_shared_tangle::lifecycle::session::SessionRoot",
        "session activation identity; assigned at activation, never mutated",
    ),
    (
        "ambition_platformer2d_runtime::content_identity::",
        "content identity; a change invalidates the session rather than moving inside it",
    ),
    (
        "ambition_platformer2d_runtime::session_world::PlatformerSessionCatalogs",
        "which providers this session composed; fixed at activation",
    ),
    //  arrived with K2b edit 2, and it belongs to the group above it.
    // The build-time root spawned four things — `SessionRoot`, the live world,
    // prepared content and its identity — and never stamped an epoch, because a
    // root that exists before tick 0 has no activation to be a generation OF.
    // A shell activation does stamp one, so deleting the build-time publisher
    // put it on the census.
    //
    //  measured, not assumed: it sits on the SESSION ROOT, in the same
    // archetype as `PreparedContent`, `PreparedContentIdentity` and
    // `PlatformerSessionCatalogs`, all waived here for this reason already. An
    // epoch changing mid-session does not mean "rewind it" — it means the
    // session is a different one, which invalidates the session rather than
    // moving inside it.
    (
        "ambition_platformer2d_core::content_epoch::ContentEpoch",
        "which committed activation this session IS; a change invalidates the \
         session rather than moving inside it",
    ),
    (
        "ambition_platformer2d_actor_monolith::avatar::starting_character::StartingCharacter",
        "session activation input, resolved once at player spawn",
    ),
    (
        "ambition_platformer2d_actor_monolith::avatar::starting_character::InitialBodyPolicy",
        "the same session activation input one level up: WHETHER this session \
         lowers a home avatar at all, and which one. Authored by the experience \
         definition, written onto the session world root when the session is \
         built, and read only by `simulation_world` at that moment. A rewind \
         cannot reach the frame that wrote it, and a session whose body policy \
         changed mid-flight would be a different session",
    ),
    // ── Authored geometry and identity on world props ────────────────────────
    //
    // Same population change surfaced these: a shrine, a moving platform's visual
    // index, and a portal's authored channel all sit on entities that carry
    // registered state, and none of the three is written after the room loads.
    (
        "ambition_platformer2d_actor_monolith::shrine::HealShrine",
        "authored shrine geometry; the heal reads it and never writes it",
    ),
    (
        "ambition_portal2d::link::PortalLink",
        "authored portal channel identity, hashed at spawn",
    ),
    // ── An EXTERNAL INPUT to the simulation ──────────────────────────────────
    //
    //  the one category on this list where rewinding would be actively
    // harmful rather than merely meaningless, and it is the same category as
    // the device input stream the `ambition_input::` waiver above already
    // covers: a rewind restores what the simulation DECIDED, never what it was
    // TOLD. Erasing an input is how the replay reaches a different decision.
    //
    //  the waiver names the LEDGER, not one payload: every narrative-input
    // family a game registers is the same category, and a waiver that had to be
    // re-typed per payload would go stale the first time content added one.
    (
        "ambition_conversation::ledger::NarrativeInputLedger",
        "an EXTERNAL INPUT, stamped with the tick it applies from — the same \
         category as the device input stream, and rewinding it would erase what \
         the simulation was told rather than what it decided",
    ),
];

fn waiver(type_name: &str) -> Option<&'static str> {
    WAIVED
        .iter()
        .find(|(needle, _)| type_name.contains(needle))
        .map(|(_, reason)| *reason)
}

/// Every type name the rollback vocabulary mentions at all — state, anchors, and
/// derived declarations alike.
fn rollback_vocabulary(sim: &mut Platformer2dSimHarness) -> BTreeSet<String> {
    sim.world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("rollback registry is installed by the engine plugins")
        .descriptors()
        .map(|d| d.type_name.clone())
        .collect()
}

/// Entities whose state a rewind must reproduce: feature-sim entities, bodies
/// integrated by the simulation, and any entity carrying a type named by the
/// rollback vocabulary. The last group includes transient rollback state such as
/// strike volumes that carry neither of the broader entity markers.
fn simulated_population(sim: &mut Platformer2dSimHarness) -> Vec<Entity> {
    let vocabulary = rollback_vocabulary(sim);
    let world = sim.world_mut();
    let mut found: BTreeSet<Entity> = BTreeSet::new();
    let mut tagged =
        world.query_filtered::<Entity, With<ambition_platformer2d::platformer::lifecycle::FeatureSimEntity>>();
    found.extend(tagged.iter(world));
    let mut bodies =
        world.query_filtered::<Entity, With<ambition_platformer2d::engine_core::BodyKinematics>>();
    let body_hits: Vec<Entity> = bodies.iter(world).collect();
    let body_count = body_hits.len();
    found.extend(body_hits);

    let all: Vec<Entity> = {
        let world = sim.world_mut();
        let mut everything = world.query_filtered::<Entity, ()>();
        everything.iter(world).collect()
    };
    let world = sim.world();
    for entity in all {
        if found.contains(&entity) {
            continue;
        }
        let Ok(mut components) = world.inspect_entity(entity) else {
            continue;
        };
        if components.any(|info| vocabulary.contains(&info.name().to_string())) {
            found.insert(entity);
        }
    }
    //  ANTI-VACUITY, and it is load-bearing for NINETEEN tests. Every sweep
    // in this file runs `unaccounted_components` over this population and passes
    // when the result is empty — which is also what an EMPTY POPULATION produces.
    // A fixture that booted no bodies, or a filter that stopped matching after a
    // bundle changed, would turn the whole file green in one commit and read
    // exactly like an all-clear.
    //
    // what is asserted is what is TRUE OF EVERY FIXTURE: a body exists, and the union is non-empty.
    // It now serves ten rooms, and `portal_lab` authors no `FeatureSimEntity` at all. A room that
    // authors no feature entities is a legitimate room, not a broken fixture, so the assert the
    // review asked for would be a false alarm on real content. The granularity that was right for
    // the smoke is wrong for the sweep it became.
    assert!(
        body_count > 0,
        "no entity in this fixture carries `BodyKinematics`, so the rollback \
         coverage sweep is about to inspect a population with no bodies in it \
         and report a confident all-clear"
    );
    assert!(
        !found.is_empty(),
        "the rollback coverage population is EMPTY — every sweep in this file \
         would pass by having nothing to look at"
    );
    found.into_iter().collect()
}

/// The component sweep for one booted room: every `ambition_`-named component
/// on a simulated entity that is neither registered, declared derived, nor
/// waived. The population differs per room — enemies, switches, and breakables
/// only exist where a room authors them — so callers sweep representative
/// rooms, not just the boot default.
///
///  it walks the entities PRESENT in a booted room, so state that only exists
/// after an EVENT is structurally invisible to it. An item spat out by a
/// struck block, a projectile in flight, a pickup mid-arc: none of them are in a
/// world nobody has played yet, and no amount of sweeping more rooms reaches
/// them. This is a property of a one-shot census, not a gap to be waived, and it
/// is worth stating because the sweep's silence about a component reads exactly
/// like a pass.
pub(crate) fn unaccounted_components(sim: &mut Platformer2dSimHarness) -> BTreeMap<String, usize> {
    // An ANCHOR is not coverage. `require_rollback` only installs the
    // `bevy_ggrs::Rollback` marker so the entity participates; it snapshots
    // nothing. Counting it as accounted is how `TransformBeat` shipped claiming
    // to be "registered snapshot state" while its `remaining` and its borrowed
    // invulnerability were never restored — this sweep said yes because a
    // descriptor existed, without asking what kind. Every other anchored type
    // also carries a canonical or clone registration, so ignoring the anchor
    // kind here costs nothing and closes that hole.
    let known: BTreeSet<String> = sim
        .world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("rollback registry is installed by the engine plugins")
        .descriptors()
        .filter(|d| d.kind != ambition_platformer2d::rollback::RollbackEntryKind::RequiredRollback)
        .map(|d| d.type_name.clone())
        .collect();

    let sim_entities = simulated_population(sim);
    assert!(
        !sim_entities.is_empty(),
        "no simulated entities found — the fixture did not actually boot a world, \
         so a green result here would be vacuous"
    );

    let mut unaccounted: BTreeMap<String, usize> = BTreeMap::new();
    let world = sim.world();
    for entity in sim_entities {
        let Ok(components) = world.inspect_entity(entity) else {
            continue;
        };
        for info in components {
            let name = info.name().to_string();
            if !name.contains("ambition_") || known.contains(&name) || waiver(&name).is_some() {
                continue;
            }
            *unaccounted.entry(name).or_default() += 1;
        }
    }
    unaccounted
}

/// Every component on a simulated entity that is accounted ONLY by a waiver.
///
/// A9 found `BossAnimFrame` — a sim-owned animation cursor that boss hurtbox
/// geometry derives from — silently swallowed by the `ambition_sprite_sheet::`
/// prefix waiver, whose stated reason is "sprite metadata / asset binding". A
/// prefix waiver assumes a crate holds exactly one kind of thing, and crates grow.
///
/// This lists what each waiver is actually covering so the claim can be re-read
/// against reality instead of against the crate name.
pub(crate) fn waived_components(
    sim: &mut Platformer2dSimHarness,
) -> BTreeMap<String, &'static str> {
    let known: BTreeSet<String> = sim
        .world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("rollback registry is installed by the engine plugins")
        .descriptors()
        .map(|d| d.type_name.clone())
        .collect();
    let sim_entities = simulated_population(sim);
    let mut waived: BTreeMap<String, &'static str> = BTreeMap::new();
    let world = sim.world();
    for entity in sim_entities {
        let Ok(components) = world.inspect_entity(entity) else {
            continue;
        };
        for info in components {
            let name = info.name().to_string();
            if !name.contains("ambition_") || known.contains(&name) {
                continue;
            }
            if let Some(reason) = waiver(&name) {
                waived.insert(name, reason);
            }
        }
    }
    waived
}

/// Print the shipped composition's remaining unaccounted resources.
///
/// The sibling of the waiver listing below, for the other sweep. Not an
/// assertion — the ceiling test is the assertion; this is how you READ what the
/// ceiling is holding, which is the first thing anyone lowering it further needs.
///
/// these 25 are deliberately NOT covered by a category waiver. What remains is what no family could
/// honestly swallow: a mix of provider-lifecycle catalogs, session-scope markers, authored art
/// manifests and a few genuinely ambiguous ones.
///
///  read them one at a time and register or waive INDIVIDUALLY. Both bugs
/// this sweep has caught — `BrokenBricks` and `SpentMonitors` — were in a demo
/// provider's namespace, which is precisely where a broad new family waiver would
/// hide the next one.
#[test]
#[ignore = "audit listing: prints what the ceiling is holding; read it, do not assert on it"]
fn probe_what_the_shipped_ceiling_is_still_holding() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..8 {
        app.update();
    }
    let unaccounted: Vec<String> = unaccounted_resources(app.world())
        .into_iter()
        .filter(|name| !name.contains("::Messages<"))
        .filter(|name| name.starts_with("ambition_"))
        .collect();
    eprintln!(
        "[shipped-sweep] {} unaccounted resources still on the ceiling:",
        unaccounted.len()
    );
    for name in &unaccounted {
        eprintln!("[shipped-sweep]   {name}");
    }
}

/// Print the concrete types covered by each rollback waiver.
///
/// Prefix waivers can grow to cover types their reason no longer describes, so this
/// audit listing is for comparing each current type against its waiver rationale.
#[test]
#[ignore = "audit listing: prints what each waiver covers; read it, do not assert on it"]
fn probe_what_every_waiver_actually_covers() {
    for room in ["combat_calibration_lab", "mockingbird_arena"] {
        let mut sim = Platformer2dSimHarness::new_with_options(
            ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
                .with_timestep(TimestepMode::fixed_60hz())
                .with_required_start_room(room),
        )
        .expect("sandbox sim builds");
        for _ in 0..8 {
            sim.step(AgentAction::default());
        }
        println!("\n=== {room} ===");
        for (name, reason) in waived_components(&mut sim) {
            println!("  {name}\n      waived as: {reason}");
        }
    }
}

fn assert_components_accounted(sim: &mut Platformer2dSimHarness, room: &str) {
    let unaccounted = unaccounted_components(sim);
    if !unaccounted.is_empty() {
        let mut report = format!(
            "Components live on simulated entities in `{room}` that GGRS will not\n\
             rewind. Each is a rollback desync waiting to happen. For each one:\n\
             register it in `register_engine_rollback_state`, declare it derived,\n\
             or add a justified waiver to WAIVED in this file.\n\n",
        );
        for (type_name, count) in &unaccounted {
            report.push_str(&format!("  {type_name}  (on {count} sim entities)\n"));
        }
        panic!("{report}");
    }
}

#[test]
fn every_component_on_a_simulated_entity_is_registered_derived_or_waived() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    // Step a few frames so lazily-inserted runtime state (timers, resolved
    // frames, published hurtboxes) is actually present on the bodies.
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }
    assert_components_accounted(&mut sim, "default boot room");
}

/// The same sweep over the combat-calibration population: enemies, a switch,
/// and a breakable, none of which exist in the default boot room. The exit
/// oracle (`rollback_exit_oracle.rs`) runs its sync-test here, so this room's
/// composition being fully accounted is what makes that checksum meaningful.
#[test]
fn every_component_in_the_combat_calibration_lab_is_registered_derived_or_waived() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("combat_calibration_lab"),
    )
    .expect("sandbox sim builds in the calibration lab");
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }
    assert_components_accounted(&mut sim, "combat_calibration_lab");
}

/// The same sweep over a BOSS population, which nothing swept before.
///
/// The two rooms above contain no boss, so every boss-only component — the
/// animation cursor, the pattern timer, the death animation, the encounter
/// authority — had never appeared in this sweep's population at all. That is the
/// same shape as the two holes this instrument has already had (it did not
/// inspect the player; it did not inspect transients): a confident empty result
/// produced by never looking.
///
/// It is not hypothetical. `BossAnimFrame` is a SIM-owned animation cursor —
/// `drive_boss_animators` advances it on `world_time.entity_dt`, and
/// `BossAnimationFrameSample` turns it into the boss's active hurtbox parts — and
/// it was not rollback state. A rewind left the cursor wherever an abandoned
/// future put it, so the boss's damageable geometry after a rollback was derived
/// from the wrong frame. Exactly the class that produced the equipment-oracle
/// divergence: registered-looking state feeding combat geometry, invisible to the
/// instrument because of population, not because of accounting.
#[test]
fn every_component_in_a_boss_arena_is_registered_derived_or_waived() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("mockingbird_arena"),
    )
    .expect("sandbox sim builds in a boss arena");
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }
    assert_components_accounted(&mut sim, "mockingbird_arena");
}

/// Populations no sweep had ever visited. (A19)
///
/// This instrument is POPULATION-driven: it can only report on components that
/// exist in the rooms it boots. It has now been confidently empty three times
/// about something it never looked at — the player (no `FeatureSimEntity`),
/// transients (sampled one instant), and every boss-only component (no swept room
/// had a boss). Adding rooms is therefore not padding; it is the only way this
/// test's silence means anything.
///
/// Each room here is chosen for what it AUTHORS that the others do not: portals,
/// NPCs and dialogue state, hazards and chests.
#[test]
fn every_component_in_unswept_populations_is_registered_derived_or_waived() {
    for room in [
        "portal_lab",
        "basement_npcs",
        "basement_hazards",
        // Encounter authority: waves, gates, and the mob bookkeeping that decides
        // whether a room is cleared.
        "goblin_encounter",
        // Kinematic movers and the bodies riding them — moving platforms carry
        // path state the sim advances every tick.
        "vertical_shaft",
        "combat_calibration_lab",
    ] {
        let mut sim = Platformer2dSimHarness::new_with_options(
            ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
                .with_timestep(TimestepMode::fixed_60hz())
                .with_required_start_room(room),
        )
        .unwrap_or_else(|error| panic!("sandbox sim builds in `{room}`: {error}"));
        for _ in 0..8 {
            sim.step(AgentAction::default());
        }
        assert_components_accounted(&mut sim, room);
    }
}

/// Every sweep above inspects a room at rest, and a moveset strike volume exists
/// only inside its authored active window — a handful of ticks. Those entities carry
/// `Hitbox`, `HitboxHits`, `StrikeVolume` and optionally `HitboxOnHit`, and NONE of
/// `FeatureSimEntity` or `BodyKinematics`. So for as long as the population was
/// those two tags plus more rooms, new state parked on a live strike volume was
/// outside this instrument no matter how thorough it looked: adding rooms cannot
/// reach a family that only exists for six frames.
///
/// Two changes make this test mean something. The population now includes
/// `bevy_ggrs::Rollback` — the mechanism's own answer to "what does the rollback
/// carry" — and this test holds the attack button down, sweeping on the tick the
/// volume is live and ASSERTING that it was. A transient sweep that silently missed
/// its transient would be the same false negative in a new costume.
#[test]
fn every_component_on_a_live_strike_volume_is_registered_derived_or_waived() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("combat_calibration_lab"),
    )
    .expect("sandbox sim builds in the calibration lab");
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }

    // Swing repeatedly and sweep on every tick a volume is live.
    let mut swept_with_a_live_volume = 0usize;
    let mut live_volume_peak = 0usize;
    for frame in 0..240 {
        let action = AgentAction {
            attack: frame % 24 == 0,
            attack_held: frame % 24 < 4,
            attack_released: frame % 24 == 4,
            ..AgentAction::default()
        };
        sim.step(action);
        let live: Vec<Entity> = {
            let world = sim.world_mut();
            let mut volumes =
                world.query_filtered::<Entity, With<ambition_platformer2d::combat::moveset::StrikeVolume>>();
            volumes.iter(world).collect()
        };
        if live.is_empty() {
            continue;
        }
        live_volume_peak = live_volume_peak.max(live.len());
        // The assertion that this test is FOR. A live transient existing is not the
        // same fact as the sweep looking at it, and conflating the two is how an
        // instrument reports coverage it does not have: the previous population
        // would have left every one of these entities uninspected while this loop
        // happily counted them.
        let population: BTreeSet<Entity> = simulated_population(&mut sim).into_iter().collect();
        for volume in &live {
            assert!(
                population.contains(volume),
                "a live strike volume ({volume}) is not in the swept population, so \
                 no amount of sweeping reaches the transient families — this is the \
                 hole `Rollback` was added to the population to close"
            );
        }
        let unidentified: Vec<Entity> = {
            let world = sim.world_mut();
            live.iter()
                .copied()
                .filter(|volume| {
                    world
                        .get::<ambition_platformer2d::platformer::sim_id::SimId>(*volume)
                        .is_none()
                })
                .collect()
        };
        assert!(
            unidentified.is_empty(),
            "{} live strike volume(s) carry no `SimId`, so the entity-reference \
             probes cannot tell them apart: {unidentified:?}",
            unidentified.len(),
        );
        swept_with_a_live_volume += 1;
        assert_components_accounted(&mut sim, "combat_calibration_lab (live strike volume)");
    }

    // The vacuity guard, and the whole reason this test is not just another room.
    assert!(
        swept_with_a_live_volume > 0,
        "no strike volume was ever alive during this sweep, so it inspected the \
         same at-rest population as every other test here and proves nothing about \
         transients. Either the attack never triggered or the window is shorter \
         than one sampled tick."
    );
    println!(
        "[transient sweep] {swept_with_a_live_volume} tick(s) with a live strike \
         volume, peak {live_volume_peak} concurrent"
    );
}

/// The MOUNT population, which authors no LDtk room. (A20)
///
/// ADR 0020's mount model is two linked actors with two HP pools, welded by
/// `RidingOn` / `Mounted` / `MountSlot`. No swept room authors a mounted pair, so
/// every component that only exists while a body is ridden — the brain cache the
/// weld parks, the mount's borrowed size, the rider's saddle link — had never been
/// in this sweep's population. Population, not accounting: exactly the hole that
/// hid `PogoTargetContributor` and `BossAnimFrame`.
///
/// Built in Rust rather than as a room, because a room id is not available and
/// waiting for one is how a population stays unswept.
#[test]
fn every_component_on_a_mounted_pair_is_registered_derived_or_waived() {
    use ambition_platformer2d::characters::brain::Brain;
    use ambition_platformer2d::mount::{MountSlot, Mounted, RidingOn};

    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let home = {
        let world = sim.world_mut();
        let mut q = world
            .query_filtered::<Entity, ambition_platformer2d::platformer::markers::PrimaryPlayerOnly>();
        q.single(world).expect("one primary player")
    };
    let anchor = sim
        .world_mut()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(home)
        .expect("the player has a body")
        .pos;

    // Both said only a brain key, and both of those archetype rows were DELETED when the shark
    // and the raider became characters — so this rollback sweep had been walking a pair of
    // generic `combatant` bodies: not a mount, not a pilot, and none of the components it is
    // here to register.
    sim.spawn_enemy_character_at(
        "sweep_mount",
        "Burning Flying Shark",
        (anchor.x + 120.0, anchor.y),
        (63.0, 26.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Custom(
            "burning_flying_shark".to_string(),
        ),
        "npc_burning_flying_shark",
    );
    sim.spawn_enemy_character_at(
        "sweep_rider",
        "Pirate Raider",
        (anchor.x + 120.0, anchor.y - 66.0),
        (22.0, 39.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Custom(
            "pirate_raider".to_string(),
        ),
        "npc_pirate_raider",
    );
    let by_id = |sim: &mut Platformer2dSimHarness, id: &str| {
        let world = sim.world_mut();
        let mut q = world.query::<(
            Entity,
            &ambition_platformer2d::combat::components::FeatureId,
        )>();
        q.iter(world)
            .find(|(_, feature)| feature.as_str() == id)
            .map(|(entity, _)| entity)
            .unwrap_or_else(|| panic!("`{id}` spawned"))
    };
    let mount = by_id(&mut sim, "sweep_mount");
    let rider = by_id(&mut sim, "sweep_rider");
    // Neutral brains: this sweeps a WELD, not an approach.
    sim.world_mut()
        .entity_mut(mount)
        .insert(Brain::stand_still());
    sim.world_mut()
        .entity_mut(rider)
        .insert(Brain::stand_still());
    sim.world_mut().entity_mut(rider).insert((
        RidingOn { mount },
        Mounted,
        // ⭐⭐ THE COMPONENTS A SUMMONED, LEASED RIDE ADDS, and this population is
        // the reason to put them here rather than trust that somebody will. This
        // sweep's own note says it is about POPULATION, not accounting — the
        // hole that hid `PogoTargetContributor` was a body nobody swept, not a
        // rule nobody wrote. D207 added three components that only exist while a
        // body is ridden or waiting to be, and none of them had ever been in
        // this pair.
        //
        // `PoseOwnedExternally` is stamped by `mount::board`; `RideLease` is the
        // ride's clock; `MountReservedFor` is a mount held for a rider who has
        // not boarded yet, which is a state this pair never reaches naturally
        // and so is asserted onto the mount below.
        ambition_platformer2d::engine_core::PoseOwnedExternally,
        ambition_platformer2d::mount::RideLease { remaining: 5.0 },
    ));
    sim.world_mut().entity_mut(mount).insert((
        MountSlot { rider: Some(rider) },
        ambition_platformer2d::mount::MountReservedFor {
            rider,
            lease_seconds: 5.0,
            board_within: 96.0,
            expires_in: 1.0,
        },
        // The departure a dismissed mount carries. ⚠ Smash-owned rather than
        // engine-owned, which is why it is spelled out here: a component this
        // sweep cannot see is a component whose rollback registration nothing
        // checks.
        ambition_demo_smash::shark_ride::Departing {
            remaining: 2.0,
            velocity: ambition_platformer2d::engine_core::Vec2::new(-1400.0, 0.0),
        },
    ));

    // Sweep on every tick of the weld's life. The mount-only components are
    // inserted by systems that run after the weld, and some are transient.
    for _ in 0..30 {
        sim.step(AgentAction::default());
        // Vacuity guard: the pair must be IN the population, every tick. A mount
        // sweep that inspects everything except the mount is the shape this whole
        // instrument keeps failing in.
        let population: BTreeSet<Entity> = simulated_population(&mut sim).into_iter().collect();
        for (label, entity) in [("mount", mount), ("rider", rider)] {
            assert!(
                population.contains(&entity),
                "the {label} is not in the swept population, so this test inspects \
                 everything except the thing it is named for"
            );
        }
        assert_components_accounted(&mut sim, "a welded mount pair");
    }
    assert!(
        sim.world_mut().get::<Mounted>(rider).is_some(),
        "the weld must survive the sweep, or the mount-only components were never \
         present while it ran"
    );
}

/// Seat a two-CPU match through the PRODUCTION seating system and step until it activates.
///
/// Two CPU seats, so neither depends on the harness having a primary player
/// wearing the right character. A seat that silently fails to adopt is how a
/// match sweep ends up inspecting an empty roster and reporting success.
///
/// Use characters prepared by the plain simulation harness. `seat_character`
/// returns `None` for unprepared IDs, so the vacuity guard must prove seats were
/// actually created.
fn seat_a_two_cpu_match(sim: &mut Platformer2dSimHarness) -> usize {
    use ambition_platformer2d::versus_match::{
        ControllerBinding, MatchParticipant, MatchParticipantRoster,
    };

    let cpu = |character: &str, team: &str| {
        MatchParticipant::new(character)
            .driven_by(ControllerBinding::Cpu {
                brain_profile: Some("medium_striker".to_string()),
            })
            .on_team(team)
    };
    sim.world_mut().insert_resource(MatchParticipantRoster {
        participants: vec![
            cpu("player_robot_v3", "blue"),
            cpu("player_robot_v2", "red"),
        ],
        seating: ambition_platformer2d::actor::RosterSeating::activated_at(7),
        // A fixture's roster has no publisher: nothing else in this App claims
        // one, which is the case `None` is for.
        published_by: None,
        rules: ambition_platformer2d::versus_match::MatchRules {
            item_spawns: None,
            opens_suspended: true,
            // No ceremony in a rollback fixture: the stage that owns the opening
            // is not part of what these tests exercise.
            opening_countdown_ticks: 0,
            time_limit_ticks: 0,
            abilities: None,
            body: None,
            stocks: None,
            health_pool: None,
            ..Default::default()
        },
    });
    // A direct world mutation is setup, not gameplay: it becomes frame zero.
    sim.rebase_rollback_history()
        .expect("the roster insert becomes the rollback baseline");

    for tick in 0..90 {
        sim.step(AgentAction::default());
        if sim
            .world()
            .get_resource::<ambition_platformer2d::versus_match::ActiveMatch>()
            .is_some()
        {
            return tick;
        }
    }
    panic!(
        "no roster seat ever produced a live match in 90 ticks, so every sweep \
         built on this helper would inspect a world with no match in it and pass \
         for the wrong reason"
    );
}

/// A LIVE MATCH, which nothing swept. (AA2 / AC2)
///
/// What is worth recording is why neither this instrument nor any other caught it first: no swept
/// population contained a match. That is the exact shape A19 already hit —
/// `PogoTargetContributor`, `ChestFeature` and `PortalHostScanned` were not
/// unregistered-and-missed, they were never in the population — and the lesson evidently did not
/// generalise on its own. A sweep answers only the question its population asks.
///
/// This seats a real two-CPU roster through the production `seat_match_participants` and sweeps
/// every tick of the match's life, including the activation tick, which is the one the reviews
/// say a rewind crosses badly.
#[test]
fn every_component_in_a_live_match_is_registered_derived_or_waived() {
    use ambition_platformer2d::versus_match::MatchSeat;

    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    seat_a_two_cpu_match(&mut sim);
    // The activation tick itself is already behind us, and it is swept below on
    // the way through: seating publishes on the tick the last seat lands, and
    // the match then lives for the rest of this loop.
    let seat_count = |sim: &mut Platformer2dSimHarness| -> usize {
        let world = sim.world_mut();
        let mut q = world.query::<&MatchSeat>();
        q.iter(world).count()
    };
    assert_eq!(
        seat_count(&mut sim),
        2,
        "the match activated without two seated bodies, so this sweeps something \
         other than what it is named for"
    );
    for _ in 0..60 {
        sim.step(AgentAction::default());
        assert_components_accounted(&mut sim, "a live versus match");
    }
}

/// The falling-sand room, which nothing swept. (A20)
///
/// The sand grid itself is deliberately outside rollback (see the
/// `::falling_sand_sim::` waiver, which carries its own in-code guard), but the
/// ROOM authors a spout, a sand switch, and hazard geometry that no other swept
/// room has. The switch is activated so the room is swept in its ACTIVE state
/// rather than at rest — a spout that never opened is a population of one idle
/// entity.
#[test]
#[cfg(feature = "falling_sand")]
fn every_component_in_the_falling_sand_room_is_registered_derived_or_waived() {
    use ambition_content::falling_sand_sim::{FallingSandWorld, ROOM_ID, SAND_SWITCH};

    let mut sim = crate::common::fixed_60hz_room_sim(ROOM_ID);
    for _ in 0..10 {
        sim.step(crate::common::base());
    }
    {
        let world = sim.world_mut();
        let mut switches =
            world.query::<&ambition_platformer2d::encounter::switches::SwitchFeature>();
        let activation = switches
            .iter(world)
            .map(|feature| feature.activation.clone())
            .find(|activation| activation.id == SAND_SWITCH)
            .unwrap_or_else(|| panic!("authored switch `{SAND_SWITCH}` exists in {ROOM_ID}"));
        world.write_message(
            ambition_platformer2d::encounter::switches::SwitchActivated {
                activation,
                pos: ambition_platformer2d::engine_core::Vec2::ZERO,
            },
        );
    }
    for _ in 0..60 {
        sim.step(crate::common::base());
        assert_components_accounted(&mut sim, ROOM_ID);
    }
    // Vacuity guard: the spout has to have poured, or this swept an idle room.
    let emitted = sim
        .world_mut()
        .get_resource::<FallingSandWorld>()
        .and_then(|sand| sand.grid.as_ref().map(|grid| grid.emitted()))
        .unwrap_or(0);
    assert!(
        emitted > 0,
        "no matter was emitted, so the room was swept in its idle state and this \
         test says nothing about the sand slice's live population"
    );
}

/// Resource type-name substrings that are NOT authoritative simulation state.
///
/// Same contract as [`WAIVED`]: each entry claims rewinding the named resource
/// would be meaningless or harmful, with the reason. Crate-prefix waivers from
/// [`WAIVED`] apply here too; this list holds the resource-specific remainder.
const RESOURCE_WAIVED: &[(&str, &str)] = &[
    // The tier floor of the room the HOST is loading behind a cover.
    //
    // The authored respawn beat, in SECONDS.
    //
    // ⭐ CONFIG, NOT STATE, and structurally so: the ruleset inserts it once in
    // its plugin `build` and NOTHING in the simulation writes it. A rewind
    // restoring it would restore the same number it already holds.
    //
    // ⛔ WHAT IS ROLLBACK STATE IS `DeathInterlude::remaining`, the countdown
    // this SEEDS — the window a body waits out is a position in time, so it
    // rewinds. `PendingRespawn` beside it is a MARKER: it names the consequence
    // the window owes when it closes and carries no countdown of its own.
    //
    // ⛔⛔ A WAIVER IS KEYED BY TYPE NAME, SO ITS REASON CAN GO STALE WITHOUT
    // ANYTHING FAILING — and a stale reason hands the next rollback reviewer an
    // architecture that is gone. Re-read the reason when the type moves.
    (
        "ambition_combat::stocks::RespawnInterval",
        "authored config in seconds: inserted once at plugin build, never written by a system; the countdown it seeds (DeathInterlude::remaining) is registered",
    ),
    // SOMEBODY ASKED TO STOP A MATCH, and this one is waived because rewinding
    // it would LOSE the request rather than preserve it.
    //
    // ⛔⛔ THE ASK IS MADE OUTSIDE THE SIMULATION — a shell menu — so a
    // resimulation cannot re-make it. ⛔ A `MatchAbandoned` message registered
    // with `clear_message_on_rollback` cannot carry it either: the backend
    // `.clear()`s the buffer, so an Exit Match consumed on a speculative frame is
    // GONE after a rewind. Snapshotting it fails the same way from the other
    // side.
    //
    // ⭐ WHAT SURVIVES BOTH is a latch that does not rewind and NAMES ITS MATCH:
    // a rewind leaves the ask standing so the resim reaches the same verdict,
    // and the next match ignores it because the instance differs. The scoping is
    // pinned by `stocks::a_stop_request_ends_the_match_it_names_and_no_other`.
    (
        "ambition_platformer2d_actor_monolith::features::stocks_match::MatchAbandonRequest",
        "an ask made outside the sim cannot be re-made by a resim; it is scoped by MatchInstance instead of rewound",
    ),
    // The condition catalog: which questions the installed domains can
    // answer, and the function that answers each.
    //
    //  IMMUTABLE ONCE THE SIMULATION STARTS, AND STRUCTURALLY SO. `publish` is
    // private to its module; the only way in is `PublishCondition` on `App`, and
    // a tick holds a `World`, never an `App`. So a rewind restoring this would
    // restore a byte-identical value — there is no timeline in which the set of
    // questions the engine can answer differs.
    //
    //  this waiver is about the CATALOG, not about answers.
    (
        "ambition_platformer2d_shared_tangle::authored_logic::ConditionCatalog",
        "published during plugin build only; `publish` is private and a tick has no `App`",
    ),
    // The command catalog: which verbs the installed domains can perform,
    // and the function that performs each.
    //
    //  THE SAME STRUCTURAL ARGUMENT, and this is the half where it had to be
    // made before anything was built. `publish` is private, the only way in is
    // `PublishCommand` on `App`, and a tick holds a `World`.  a command
    // registry a system could write to IS rollback state, and then every
    // authored verb in the game joins the snapshot.
    //
    //  and `run` is private too, which is a different claim from this waiver
    // but the reason the waiver is not merely true: nothing can perform a
    // command out of `AuthoredCommandSet`, so there is no timeline in which the
    // catalog and the world disagree about what happened.
    (
        "ambition_platformer2d_shared_tangle::authored_logic::commands::CommandCatalog",
        "published during plugin build only; `publish` is private and a tick has no `App`",
    ),
    // Every game's death rules (ADR 0033): how long a death holds, and the
    // roster question that decides a level reset — one declaration per game,
    // keyed by the rooms that game governs.
    //
    // AUTHORED CONSTANTS, stated once when each game's plugin is built and never
    // written by any system. A rewind cannot change what a game's rules are —
    // rewinding them would be rewinding the ruleset itself, not the simulation
    // it governs. The state the rules PRODUCE (`DeathInterlude`, `OutOfPlay`)
    // is per-body and IS registered, in the combat domain.
    //
    // It would NOT survive a resolved-rules resource written each tick from the active room —
    // which is why the resolution is a `SystemParam` that stores nothing rather than a derived
    // global.
    (
        "ambition_combat::death_rules::DeclaredDeathRules",
        "authored rules stated at plugin build; the state they produce is registered per body",
    ),
    // The rollback localizer's own state: the probe table and its audit ledger.
    //
    // Diagnostic instrumentation ABOUT the rollback, not state the rollback
    // reproduces. The probe table is built once at plugin registration and never
    // mutated by gameplay; the audit is inert unless a diagnostic test enables it,
    // and rewinding a measurement of the rewind is meaningless — it would erase
    // the very record being compared.
    //
    // Waived rather than exempted: this sweep caught the localizer the moment it
    // was added, which is the sweep working correctly on its own author.
    (
        "ambition_platformer2d_rollback_ggrs::probes::",
        "rollback diagnostics: measures the rewind, is not reproduced by it",
    ),
    // Whether the twintrack spacetime MINIMAP is showing. A viewer's toggle
    // over a 3D diagram of worldlines the simulation already computed — the
    // diagram READS the experiment, and nothing in the experiment reads the
    // diagram.
    //
    // Rewinding it would rewind a UI preference: press M during a prediction
    // window and the minimap would blink back closed when the frame resimulated,
    // which is the panel fighting the person using it. The same argument the
    // sheet-decode waiver below makes — presentation state a checksum cannot see,
    // because no body's collision, health or moves depend on it.
    (
        "ambition_demo_twintrack::spacetime_3d::SpacetimeMinimapState",
        "presentation toggle for a read-only diagram; rewinding it would fight the viewer",
    ),
    //  The two relativity READ MODELS, and the reason is not "presentation"
    // but REPUBLICATION. Both are recomputed every frame in `Update` from
    // `SpacetimeCoordinateTime2d` and canonical `BodyKinematics` — no
    // accumulator, no entity, nothing carried between frames. A restored value
    // is overwritten before anything reads it, so rewinding them is not harmful,
    // it is a no-op with a cost.
    //
    //  waived rather than DECLARED DERIVED, deliberately. A derived
    // declaration's reason string is hashed into `schema_fingerprint`, so it
    // would put a demo's exhibit into the engine's wire format and owe a version
    // bump every time somebody reworded it. `RelativisticOpticalView2d` is
    // declared derived because it lives in an ENGINE crate and the simulation
    // reads it; these two live in a demo and only the two split-screen panes do.
    (
        "ambition_demo_twintrack::dual_observer::TwinTrackDualObserverView",
        "republished each frame from coordinate time and kinematics; a rewind cannot \
         change what the next frame recomputes",
    ),
    (
        "ambition_demo_twintrack::light_pulse::TwinTrackLightPulseView",
        "republished each frame from the emission event and the invariant speed; \
         holds no accumulated state a rewind could corrupt",
    ),
    // The engine character-art load pipeline (§7.1). Which SHEETS have been
    // decoded is presentation, not simulation: a body's collision, health, and
    // moves are identical whether its art arrived or it is drawing the marked
    // placeholder, so rewinding a decode would change nothing a checksum can see.
    //
    // Re-triggering is safe by construction rather than by luck. A rollback that
    // re-inserts `WornCharacter` marks it `Changed`, so the demand system asks
    // again; `request` is idempotent, decoding an already-decoded sheet is a
    // no-op, and demand order is a `BTreeSet`. If materialization ever gains a
    // gameplay consequence — art-derived hitboxes would be exactly that, and
    // §4.11 forbids it for this reason — this waiver becomes wrong.
    (
        "::character_runtime::CharacterLoadStates",
        "character art load bookkeeping; decoded-ness has no simulation consequence",
    ),
    (
        "::load_demand::CharacterLoadDemand",
        "which sheets have been ASKED for; idempotent, and a decode has no simulation consequence",
    ),
    (
        "::character_runtime::CharacterMaterializationService",
        "the art materializer seam itself; holds no per-frame simulation state",
    ),
    (
        // The resource and its reason are unchanged; only its address is.
        "::prepared::PreparedCharacterRegistry",
        "prepared authored definitions; immutable within a session and bound by PreparedContentIdentity",
    ),
    (
        //  ALSO REPATHED, and by the same crate move as the entry above — the
        // staging lifecycle followed the fold down to `ambition_characters` so
        // the fold could stop being public. The
        // resource and its reason are unchanged; only its address is.
        "::prepared::StagedCharacterOverrides",
        "preparation-private staging input, resolved before the session's first simulated frame",
    ),
    // The ROSTER, not the activation. It is authored by whoever entered the
    // route, before the match exists, and seating only reads it — so a rewind
    // inside a match cannot move it. `ActiveMatch`, which IS written from inside
    // the sim schedule, is registered rather than waived.
    (
        "ambition_match::staging::MatchParticipantRoster",
        "the match's authored request; written at route entry, read-only for the match's life",
    ),
    // The PLAN, and it is deliberately not rollback state — `prepared_match.rs`
    // argues this at length in its own header. Registering it would DELETE it on
    // a rewind to before it was decided and leave activation with nothing to
    // replay, which is the opposite of the invariant it exists to serve: the
    // receipt (`ActiveMatch`) and the bodies rewind, and the same immutable plan
    // rebuilds the same cast. A plan that changed would be a different match.
    (
        "ambition_platformer2d_shared_tangle::markers::FramedCast",
        "presentation projection: WHAT THE CAMERA LOOKS AT when nothing local is \
         driving a body. Rebuilt in `Update` from the live seats every frame it \
         changes, never read by simulation, and a rewind that restored an older \
         cast would aim the camera at bodies the restored frame does not have",
    ),
    (
        "ambition_match::prepared::PreparedMatch",
        "the resolved match DECISION, made once before the fighters exist and \
         never written from inside the sim. Rewinding it would remove what \
         activation replays FROM",
    ),
    (
        "ambition_match::prepared::MatchPreparationProblems",
        "the refusal that answers an unpreparable roster; published beside the \
         plan, on the same pre-session decision",
    ),
    // Under a rollback host `detect_room_transition_system` DEFERS the crossing to the
    // confirmed-frame boundary (`PendingLifecycleCommit`) precisely so this multi-tick load machine
    // never engages on a speculative frame — the policy predates the move and is why it was never
    // rollback state. If a transition ever starts on a predicted frame, this waiver is wrong and
    // the resource has to be registered, not re-justified.
    (
        "::room_transition::loading::RoomTransitionLoadState",
        "transition transactions are deferred to confirmed frames",
    ),
    // Monotonic identity for the CONTENT inputs a room plan assumes. Content is
    // immutable within a session and a change invalidates the session, so
    // resimulation cannot move this.
    (
        "::room_transition::loading::RoomTransitionContentEpoch",
        "content identity, not simulation state",
    ),
    // A cache of prepared, immutable artifacts keyed by that same identity. A
    // stale entry is refused by `promote`, so the only cost of getting it wrong
    // on a rewind is a miss and a rebuild.
    (
        "::room_transition::prefetch::RoomConstructionPlanPrefetch",
        "speculative cache of immutable plans; a miss is safe",
    ),
    // Authored, immutable-by-contract content bound by PreparedContentIdentity;
    // a changed generation invalidates the GGRS session before the next frame.
    ("ambition_boss_encounter::catalog::", "authored boss catalog"),
    (
        "ambition_boss_encounter::registry::BossEncounterRegistry",
        "authored encounter registry",
    ),
    (
        // ⛔ THE PATH MOVED 2026-08-28 and this is a STRING, so nothing would have
        // told us: `CombatBanterRegistry` left the actor monolith for
        // `ambition_conversation`, and a waiver keyed on the old path answers a
        // question about a type that no longer has that name.
        "ambition_conversation::banter::CombatBanterRegistry",
        "authored banter registry",
    ),
    //  `CharacterRoster` and `CharacterRosterRegistry` WERE WAIVED HERE and
    // the types are DELETED (AC6.1). A waiver answers a checker's question
    // about something that exists; two entries naming nothing answered nothing,
    // and this list is exactly where that goes unnoticed.
    (
        "::actor::character_catalog::",
        "authored character catalog family",
    ),
    (
        "::authored_volumes::AuthoredAttackVolumeResolver",
        "authored attack volumes",
    ),
    (
        "ConstructionRegistry<",
        "recipe identity registry, frozen at content preparation",
    ),
    ("PlacementLoweringRegistry<", "authored lowering registry"),
    (
        "::content_staging::RoomContentStagingRegistry",
        "authored staging seam",
    ),
    (
        "::visual::ProjectileVisualCatalog",
        "authored projectile visuals",
    ),
    // So a sentence that was true about three `String`s was answering for an f32 timer nobody
    // rewound, and the string-keyed `HashMap` it lived in is invisible to the entity-scoped sweeps,
    // which is why no instrument said so.
    //
    // The phase now lives in `GatePortalPhases`, registered as
    // `resource.gate_portal_phases`. What is left here really is authored: it is
    // written once by the content plugin that authors a portal and never again.
    //  and it must NOT be registered — that plugin runs in `Update` behind a
    // one-shot `installed` flag that is itself waived, so a rewind past the
    // populate would restore an empty registry nothing ever refills.
    (
        "::gate_portal::GatePortalRegistry",
        "authored portal configuration — switch id and sprite names, written once \
         by the authoring content plugin. The live phase it used to carry is \
         rollback state and moved to `GatePortalPhases`",
    ),
    ("::world_manifest::WorldManifest", "authored world manifest"),
    (
        "::project::ActiveLdtkProject",
        "authored LDtk project; hot reload restarts the session",
    ),
    (
        "::session::data::Platformer2dGameplayDefaults",
        "authored data-spec value",
    ),
    (
        "::session::data::Platformer2dGameplayDefaultsHandle",
        "authored data asset handle",
    ),
    (
        "::provider::AmbitionPreparedWorld",
        "prepared-content value handed to the provider lifecycle",
    ),
    (
        "::bevy_runtime::indices::",
        "derived index of authored geometry, immutable per content epoch",
    ),
    (
        "::bevy_runtime::parity::",
        "LDtk parity diagnostics, not gameplay state",
    ),
    (
        "::hot_reload::WorldSourceHotReload",
        "dev hot-reload machinery; a commit restarts the GGRS session",
    ),
    // Settings and tuning: forward-only knobs, not per-frame simulation state.
    ("::settings::UserSettings", "user settings, forward-only"),
    (
        "::movement::tuning::ActiveMovementTuning",
        "movement tuning, forward-only",
    ),
    (
        // The WAIVER's reason is unchanged because the decision is unchanged — feel tuning is still
        // a forward-only knob, not per-frame state. Only its address moved.
        "::feel::Platformer2dFeelTuningMonolith",
        "feel tuning, forward-only",
    ),
    (
        "::physics::PhysicsSandboxSettings",
        "physics settings, forward-only",
    ),
    ("::tuning::PortalTuning", "portal tuning, forward-only"),
    // Presentation state living in otherwise-simulation crates.
    (
        "::camera_ease::",
        "camera presentation: ease/shake state and tuning follow the presented pose",
    ),
    (
        "::shrine::ShrineActivationPulse",
        "shrine presentation pulse",
    ),
    (
        "::events::GameplayBanner",
        "HUD banner read model (its request message is cleared on rollback)",
    ),
    (
        "::avatar::trail::PlayerTrailEnabled",
        "trail visuals toggle",
    ),
    // Host, lifecycle, and bookkeeping: never advanced inside a GGRS frame.
    (
        "::rollback::registry::RollbackRegistry",
        "the registration contract itself",
    ),
    (
        "ambition_platformer2d_runtime::SimulationHost",
        "host composition mode, fixed for the session",
    ),
    (
        "::content_identity::ContentEpochSequence",
        "epoch allocator; mutated only by hot reload, which restarts the session",
    ),
    ("::schedule::SimSchedule", "schedule handle"),
    (
        "::rooms::stage::LastRoomConstructionCommit",
        "construction receipt: lifecycle evidence, not frame state",
    ),
    (
        "::rooms::transaction::",
        "construction transaction bookkeeping (verification record, live binding)",
    ),
    (
        "::world_flow::room_transition_loading::",
        "room-load coordination, outside the sim frame",
    ),
    (
        "::app::player_clone::",
        "dev clone-spawn bookkeeping; the spawned clone's body is registered component state",
    ),
    // Install-once content latches: set exactly once when the intro content
    // plugin installs its fragments, never advanced inside a GGRS frame.
    (
        "::intro::plugin::IntroPropSpritesInstalled",
        "install-once latch",
    ),
    (
        "::intro::plugin::IntroCutscenesInstalled",
        "install-once latch",
    ),
    (
        "::intro::plugin::IntroBanterInstalled",
        "install-once latch",
    ),
    (
        "::intro::plugin::IntroGatedZonesInstalled",
        "install-once latch",
    ),
    (
        "ambition_cutscene::CutsceneTriggerQueue",
        "narrative trigger seam. ⛔ **this reason was WRONG until 2026-08-06** and          said only *\"seen-flags in the rollback-registered AmbitionGameSave dedup          re-fires\"* — which assumes the trigger re-fires. It could not: the room          memory driving it was a `Local<Option<String>>` on a SIM system, and Bevy          locals are not rewound, so a rewind past a room entry left the local          claiming that room and resimulation emitted NOTHING. A seen flag cannot          deduplicate a re-fire that never happens (GPT 5.6 through `32eb27a`).          The memory is `ambition_cutscene::LastCutsceneRoom` now, registered as          `cutscene.last_room`, so the queue's contents ARE regenerated from          rollback state on the restored timeline — which is the condition under          which a transient queue is legitimately transient, and it is now met          rather than assumed",
    ),
    (
        "::brain::BrainActionCounter",
        "diagnostic counter surfaced by HUD/debug tooling",
    ),
    (
        "::developer_hotkeys::DeveloperAction",
        "developer hotkey message",
    ),
    // Deliberate rollback exclusions, each with an in-code guard.
    (
        "::falling_sand_sim::",
        "deliberately outside rollback: grid/ledger advance only on authoritative passes \
         (simulation_pass_is_authoritative guard; module-level warning; falling-sand.md)",
    ),
    (
        "::cut_rope::arena::CutRopeBossArenaState",
        "per-frame mirror of the FallingHazard entity, rebuilt each frame",
    ),
    // It is a latch that BRIDGES TICKS: the choice is made while the last line is on screen and the
    // reset fires whenever the player dismisses it. It is written and cleared by the simulation now
    // and gated on the conversation authority rather than on `DialogState`, so it is registered
    // rather than excused. ── The SHIPPED composition's categories ────────────────────
    //
    // The sandbox sweep never saw these: it boots `Platformer2dSimHarness`, and
    // these live in the app and in provider compositions. The shipped-composition
    // sweep beside it found 64 unaccounted, and the first pass through them found
    // two REAL bugs (`BrokenBricks`, `SpentMonitors`, both registered now). What
    // is left sorts into a small number of categories, and these are them.
    //
    //  each is a MODULE family, which is the widest form this file allows and
    // the one that most easily rots. The test is whether a sim-authoritative
    // resource could plausibly be added to that module later. For a menu, a
    // developer overlay or a file path, no — the module name IS the argument. Any
    // family where the answer is "maybe" is deliberately absent below and stays
    // on the ceiling.
    (
        "ambition_app::menu::",
        "frontend menu state: cursor, tab, scroll, cached pages. Outside the          session; a rewind cannot reach it and would have nothing to say",
    ),
    (
        "ambition_app::dev::",
        "developer instruments (fps overlay, rollback observatory). Measurements          ABOUT the run; rewinding a measurement of the rewind is meaningless",
    ),
    // THE OPENING BELL OUTRAN THE ART, and this is the instrument that says so.
    //
    // ⭐ NOTHING IN THE SIMULATION READS IT — grep-checkable, and the property the
    // waiver rests on: the only non-test reference outside `audit.rs` is the
    // `init_resource` that installs it. It is a monotone report (a name set, two
    // frame counters) written by `report_late_match_critical_art`, and its own
    // doc states the rule that keeps it out of the sim: it REPORTS, it never
    // GATES, because the bell is deterministic and asset loading is not.
    //
    // ⚠ AND SAY THE COST RATHER THAN IMPLY IT IS FREE: it is written in the sim
    // schedule, so a resimulated frame counts again and `unready_frames` /
    // `live_frames` over-count under a live rollback session. That makes them a
    // RATE-shaped instrument, not a frame ledger — acceptable only because no
    // gameplay decision reads them. ⛔ If anything ever does, this waiver is void
    // and the counters become rollback state.
    (
        "ambition_platformer2d_actor_monolith::character_runtime::audit::LateMatchCriticalArt",
        "a loading-punctuality instrument: monotone counters plus a name set, read          by no simulation system, never fed back into gameplay",
    ),
    (
        "ambition_app::app::world_flow::room_transition_presentation::",
        "the transition's own curtain/telemetry: what the player is shown WHILE a          room swaps, not what the room becomes",
    ),
    (
        "ambition_app::app::world_flow::room_transition_assets::",
        "which assets are staged and how far the prefetch got — a readiness          question about loading, never about the simulated world",
    ),
    (
        "ambition_app::app::world_flow::first_room_art::FirstRoomArtJobs",
        "the same readiness question asked of a shell route's FIRST room, BEFORE          it activates: per published session, the manifest being waited on and          how many updates it took. Loading work that finishes before any session          simulates; nothing in a session reads it",
    ),
    (
        "ambition_platformer2d_provider::lifecycle::FirstRoomArtContributor",
        "a marker inserted by `init_resource` at plugin build: THIS host answers          `prepare-first-room-art` itself. Composition identity, written once,          never taken mutably",
    ),
    (
        "ambition_content::presentation::",
        "content-side presentation (dialog layout/portrait playback, the deep          dream settings): draws the sim, never authors it",
    ),
    (
        "ambition_platformer2d_shared_tangle::gameplay_presentation::",
        "the presentation PROFILE stack: HUD declarations and readouts, safe-area          insets, control footprints, the resolved profile. Every one is a          statement about the display, and the display is not rewound",
    ),
    (
        "ambition_platformer2d_shared_tangle::construction::schema_catalog::ConstructionSchemaCatalog",
        "descriptor-only construction metadata assembled during composition and bound into prepared-content identity. It records which typed construction schemas this App was built with; simulation ticks do not mutate it, and rewinding it would change composition identity rather than restore world state",
    ),
    (
        "ambition_persistence::",
        "where this App keeps its files and what it last wrote. Disk, not world",
    ),
    (
        "ambition_platformer2d_rollback_ggrs::session::",
        "the ROLLBACK DRIVER's own state — pending inputs, session status,          execution stats. This is the machinery doing the rewinding, and it is          the one thing a rewind must not rewind",
    ),
    (
        "ambition_platformer2d_rollback_ggrs::registration::GgrsInstalledRegistrations",
        "backend-install idempotence bookkeeping: populated while rollback declarations are installed into the App, never by the simulation. Rewinding it would mutate which snapshot plugins the process believes are installed rather than restore world state",
    ),
    (
        "ambition_platformer2d_runtime::RollbackHostReady",
        "composition marker installed once by the selected rollback backend before the simulation runs. It says which host machinery this App was assembled with; no simulation tick can change that fact",
    ),
    (
        "ambition_platformer2d_runtime::EngineRollbackStateDeclared",
        "the SECOND composition marker, and the reason it is a second one is the whole point: `RollbackHostReady` says a backend is installed, this says that backend declared the ENGINE'S rollback state. `GgrsBackendPlugin` publishes the first and not the second, which is correct for a capability host that composes no engine domains and a silent desync for one that composes the engine group -- so the engine foundation asserts on this rather than on readiness. Composition state like its neighbour: written once at build, and no simulation tick can change which plugins this App was assembled from",
    ),
    (
        "ambition_platformer2d_runtime::rollback::authority::ActiveRollbackAuthority",
        "the live rollback timeline's contract, generation, health, and the SessionScopeId that owns it. Host authority ABOUT whether speculative work may be promoted, written by session lifecycle and mismatch handling outside the rewound world; rewinding the authority doing the rewind would be backwards. It replaced a bare RollbackConfirmationState resource whose ownerless health leaked from one gameplay session into the next",
    ),
    (
        "ambition_platformer2d_runtime::rollback::authority::RollbackDiagnosticHistory",
        "what went wrong on timelines this PROCESS has run, kept after the gameplay sessions that owned them ended. Deliberately outside every session lifetime and deliberately powerless: it gates nothing, so remembering a failure cannot become a way to inherit one. A rewind restoring it would delete a record of the divergence being diagnosed",
    ),
    (
        "ambition_platformer2d_actor_monolith::audio::environment::AudioEnvironment",
        "⭐ the strongest argument on this list, and it is in the type's own doc:          `wetness` is smoothed \"using wall-clock dt, so the transition keeps          progressing while the world is paused or in bullet-time — audio buses          always run on the WALL CLOCK\". A rewind does not move wall time          backwards, so wall-clock state is not rollback state by construction          rather than by category",
    ),
    (
        "ambition_conversation::music::NarrativeMusicRequest",
        "⛔ REGISTERING IT WOULD BE THE DEFECT. Its only writer is the `<<music>>` Yarn command, which runs in `Update` — outside the rollback schedule, never re-executed on resimulation — so a rewind that restored this would DELETE a claim the runner cannot make again. Its only reader is the music intent adapter. Nothing in the simulation branches on which track is playing",
    ),
    (
        "ambition_platformer2d_host::gameplay_presentation::ScreenOccupancy",
        "what the framing was composed against, \"resolved to logical display          pixels\" — a statement about the DISPLAY, kept as its own resource so a          debug overlay can show it. The display is not rewound",
    ),
    (
        "ambition_app::app::shell_host::AmbitionShellHosted",
        "a marker saying THIS APP is composed as the shell-routed multi-game host          — \"absent in direct-entry and headless harnesses\". Composition identity,          inserted before the first frame; a rewind cannot change which app this is",
    ),
    (
        "ambition_platformer2d_provider::authoring::PlatformerAuthoredCatalogRegistry",
        "its own doc: \"app-local map from experience id to its authored catalog          fragments — the authority the shared preparation systems validate          against\". Authored content indexed at composition time; a rewind does not          re-author a catalog",
    ),
    (
        "ambition_platformer2d_provider::lifecycle::PlatformerStreamingReadiness",
        "which packed-SFX loads a provider is still waiting on. ASSET streaming          bookkeeping keyed by `LoadId` — it describes work the loader is doing,          and a rewind neither un-loads a file nor re-requests one",
    ),
    (
        "ambition_platformer2d_provider::lifecycle::PreparedPlatformerSessions",
        "the shared PREPARED-CONTENT store: immutable content plus the report from          the validation transaction that produced it. Preparation happens BEFORE a          session simulates, and the content is immutable by construction — the          rollback contract already fingerprints this content as part of session          identity, which is the stronger statement that it cannot drift",
    ),
    (
        "ambition_characters::control::SeatRawFrames",
        "the DEVICE side of the input boundary, one row per seat, holding what the          local device PROPOSED this frame before any shaping stage has run. It is          rewritten from scratch every frame by the producer and consumed by the          commit, so a rewind has nothing to put back — and restoring it would feed          a resimulation a stale proposal in place of the confirmed input it is          replaying. Same argument as `SlotControlLatches` below; the two are one          model, and D175 added this half so every seat has somewhere for a gesture,          a portal warp or a scripted substitution to happen",
    ),
    (
        "ambition_characters::control::SlotControlLatches",
        "the DEVICE side of the input boundary, for EVERY seat including zero: it          folds device samples between ticks and drains on the tick clock. A rollback resimulates from STORED          INPUTS and never by re-reading a latch, so this is input TO the rollback          rather than state inside it — restoring it would feed the resimulation a          second copy of what it is already replaying",
    ),
    (
        "ambition_characters::brain::profile::AuthoredBrainOverride",
        "what a DEVELOPER forced every authored actor's brain to: published once at          plugin build from the environment, read by room lowering as a snapshot          value (D33's inversion of the dev-crate read), never written by a system.          A measurement knob that changes which cast is built, not state the cast          produces — a resimulated frame lowers no room",
    ),
    (
        "ambition_characters::actor::population_cap::AuthoredPopulationCap",
        "the developer's actor population cap, the same shape and the same          reason: env-parsed once at plugin build, read into the construction          plan's admission quota, never written by a system. ⛔ THE QUOTA IS SPENT          WHILE THE PLAN IS BUILT, not carried on it: `ActorAdmission` filters          `room.placements` as the record list is assembled (`spawn/mod.rs`), so a          refused placement never becomes a record and never gets an identity. The          older wording here said the quota `lives on the plan`, which described          the design before the cap moved to plan time and left a refused NPC          holding an authoritative root",
    ),
    (
        "ambition_characters::perception::PerceptionExtentOverride",
        "the developer's perception viewport override, the THIRD of the same          shape: env-parsed once at plugin build by `ambition_dev_tools`, read by          `ensure_perception` when it attaches a policy to a new body, never written          by a system. A measurement knob that changes how far a brain can SEE, so          it changes what a resimulated frame perceives only through           `Perception::Sighted`, which IS rollback state and is registered — the          knob itself is the input that seeded it, not a second copy of it",
    ),
    (
        "ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperHotkeyBindings",
        "which key toggles which developer overlay. No production code takes it          mutably; it is configuration, and it is not present in a player's build's          decisions at all",
    ),
    (
        "ambition_platformer2d_shared_tangle::lifecycle::session::SessionGatedSimulation",
        "a marker inserted by `init_resource` at plugin build, saying THIS APP          routes gameplay through a launcher. Composition identity, written once          before any frame; `simulation_authorized` reads it and nothing writes it",
    ),
    (
        "ambition_platformer2d_runtime::room_transition::loading::RoomTransitionAssetContributor",
        "which assets a transition wants STAGED. Nothing takes it mutably in          production, and what it describes is loading work rather than anything          the world did",
    ),
    (
        "ambition_platformer2d_runtime::room_transition::loading::RoomTransitionPresentationAvailable",
        "whether the transition's presentation half is installed in THIS          composition — a fact about the build, not about the run",
    ),
    (
        "ambition_demo_smash::SmashStageChoice",
        "WHICH STAGE the next match is played on. A pre-match decision, not \
         simulation state: the select screen writes it before a match exists, \
         `smash_prepared_session_world` reads it ONCE when preparation is \
         requested, and nothing inside a live match reads or writes it. GGRS \
         rewinds ticks WITHIN a match, and the stage cannot differ across that \
         window — restoring it would restore a value that never changed. \
         ⛔ Not `derived`: it is authored by a player pressing a button, not \
         computed from anything. If a rule ever lets a match change stage \
         mid-session, this waiver is wrong and the resource becomes real \
         rollback state.",
    ),
    (
        "ambition_demo_smash::SmashStockChoice",
        "HOW MANY STOCKS the next match is played with — the stage choice's \
         twin, waived on the same reading and re-derived for it rather than \
         waived by resemblance. Written only by `drive_select_screen` \
         (`select_screen.rs:943`), read once on the preparation road behind \
         its `on_select` guard (`lib.rs:3012`, whose own comment says the \
         guard exists because the system would otherwise re-fire DURING the \
         match), and read by the button's label. Nothing inside a live match \
         touches it, and GGRS rewinds ticks within a match. \
         ⛔ Not `derived`: a player presses a button. ⭐ And the count does not \
         stay here — `apply_smash_match_rules` takes it as an ARGUMENT, so the \
         value the match runs on is passed, not re-read. If a rule ever lets a \
         match change its stock count mid-session, this waiver is wrong.",
    ),
    (
        "ambition_demo_smash::select::SmashSelect",
        "the character-select screen's per-seat choices. All three readers are the          screen itself — present it, drive it, and hand off — and what the MATCH          reads is the `MatchParticipantRoster` it publishes, which is a different          resource with its own owner. Frontend state, decided before a session          exists. ⚠ read to its readers rather than waived by category: this repo          has been bitten repeatedly around rosters and seats, so \"it is only the          menu\" is a claim that has to be checked",
    ),
    (
        "ambition_demo_smash::select::SmashRoster",
        "WHICH fighters the character-select grid can offer in this composition.          Assembled once in `Startup` from the catalog — the demo's own four plus          every row tagged `smash` — and never written again; the only mutable          reference in the workspace is that one assembler. Composition identity,          decided before any session exists, and the MATCH reads the          `MatchParticipantRoster` this eventually produces rather than this",
    ),
    (
        "ambition_demo_smash::select_screen::StartRequested",
        "whether somebody clicked START on the select screen. Written by          `drive_the_cursor` and cleared on arrival at the screen, both in `Update`          on a route that has no session; read once by the hand-off. ⚠ read to its          readers rather than waived as \"menu state\": a latch that says \"go\" is          exactly the shape that would matter if it ever survived into a match, and          the reason it cannot is that arriving at the screen resets it",
    ),
    (
        "ambition_demo_smash::select_screen::LeaveRequested",
        "whether somebody pressed BACK on the select screen. The same shape as          `StartRequested` one line up and waived on the same reading, not by          category: written by `drive_the_cursor` and SPENT by          `leave_the_select_screen_when_asked` on the very frame it is set — both          in `Update`, on a route that has no session — and reset again by this          experience's scope on the way out, so it cannot be true on any tick a          session simulates",
    ),
    (
        "ambition_demo_smash::select_screen::cursor::SelectCursors",
        "where each seat's select-screen pointer is and what is in its hand —          FOUR of them since 2026-08-21, one per seat, which changes the count and          not the reading. Written only by `drive_the_cursor`, which is gated on          the select ROUTE being active, so it cannot run while a session          simulates; a rewind of a pointer position would also be meaningless,          since the position is re-derived from the device on the next frame          either way",
    ),
    (
        "ambition_demo_smash::select_screen::SelectPage",
        "which page of the character grid is showing. A phone cannot fit the          roster at a hittable size, so the grid pages; this is which page. Same          reading as the cursor two lines up rather than the same category: it is          written only by `drive_the_cursor` on the select route, it is reset by          this experience's scope on the way out, and what it selects is which          RECTANGLES are drawn — nothing downstream of the hand-off can see it, so          a rewound page could not change a simulated tick",
    ),
    (
        "ambition_demo_smash::select_screen::SelectInteractionPolicy",
        "character-select interaction policy, currently whether one human hand          may manipulate another human's token. Waived as frontend configuration,          not as simulation state: the policy is initialized before play, read only          by `drive_the_cursor` on the select route, and the match consumes the          resulting `MatchParticipantRoster` rather than this resource",
    ),
    (
        "ambition_encounter::spec::EncounterWaveBook",
        "the authored encounter wave timelines, keyed by trigger id. Inserted ONCE at          plugin build from the prepared content pack and never taken mutably          anywhere in the workspace, so it cannot differ between two timelines of one          session. ⚠ it appeared in this sweep the day it stopped being a          process-global `OnceLock` — which is the sweep working: a value nobody          owned was invisible to it, and an App resource is not",
    ),
    (
        "ambition_demo_sanic::ball_dash::BallDashTuning",
        "authored dash numbers. Every production reference is `Res<>`; nothing          takes it mutably anywhere in the workspace, so it cannot differ between          two timelines of one session",
    ),
    (
        "ambition_demo_mary_o::quasar_shader::QuasarShaderInstalled",
        "a marker inserted during PLUGIN BUILD to make the shader install          idempotent. Composition state, written once before any frame runs",
    ),
    (
        "ambition_demo_mary_o::quasar_shader::MaryOQuasarShaderSettings",
        "its own doc: \"runtime tuning for development and capture tooling\", and          the field that disables the overlay says it does so \"without changing          the authoritative invincible fact\". The invincibility IS rollback state          and is registered as such; how brightly it is DRAWN is not. ⚠ read          rather than assumed because both real defects this sweep has caught          were in a demo namespace",
    ),
    (
        "ambition_platformer2d_shared_tangle::held_item_art::HeldItemArtManifest",
        "which SPRITE a held item id binds to. Written by app-builder          REGISTRATION (`&mut Self` on `App`, at composition time), never by a          system — so there is no tick on which it can differ between two          timelines. Authored art binding, not world state",
    ),
    (
        "ambition_platformer2d_shared_tangle::world_item_art::WorldItemArtManifest",
        "the same, for world items: an id → sprite table filled at composition          time by the provider. A pickup's POSITION and payload are rollback state          and are registered; what it LOOKS like is not",
    ),
    (
        "ambition_platformer2d_shared_tangle::camera_layers::MainCameraEntity",
        "which entity is the camera. Written where the camera is SPAWNED by the          render composition; a rewind does not respawn the camera, and a          simulation that depended on which entity draws it would already be          wrong. Presentation identity",
    ),
    (
        "ambition_platformer2d_shared_tangle::schedule::SimulationReplayState",
        "the marker saying THIS PASS IS A REPLAY — its own doc calls it a          \"host-owned marker for a historical replay pass\", raised after loading          historical state and cleared when the host finishes the request batch.          It is the machinery doing the rewinding, so a rewind that restored it          would be restoring the thing doing the restoring. Same argument as          `PendingSeatInputs` and `RollbackExecutionStats`; it sits in a different          module only because the SCHEDULE vocabulary owns the marker while the          driver owns the writers",
    ),
    (
        "ambition_platformer2d_shared_tangle::lifecycle::session::ActiveSessionScope",
        "the SESSION scope and its deterministic allocator — and unlike its          sibling `ActiveRoundScope`, which WAS a real desync, its sole writer          (`translate_shell_session_lifecycle`) is registered in literal `Update`.          A rewind cannot re-run it, so the allocator cannot mint differently on a          resimulated timeline. ⚠ this waiver goes stale the moment that system          moves into `app.sim_schedule()` — and the round scope is the proof that          such a move happens: somebody moved the score system into the sim and          the scope it wrote did not follow",
    ),
    (
        "ambition_platformer2d_shared_tangle::lifecycle::session::SessionScopeRetired",
        "the announcement that a session scope ENDED — the same authority and the          same writer as `ActiveSessionScope` above (`translate_shell_session_lifecycle`,          registered in literal `Update`, verified 2026-08-06), so a rewind cannot          re-run it. It arrived on the census with K2b edit 2: a build-time root          never retired a scope because it never had an activation to retire.          ⚠ this waiver goes stale with its sibling's, and for the same reason — if          that system ever moves into `app.sim_schedule()`, BOTH arguments fail          together",
    ),
    (
        "ambition_platformer2d_shared_tangle::lifecycle::session::SessionScopeActivated",
        "the announcement that a session scope BEGAN, and the third member of the          family above: the same authority, the same sole writer          (`translate_shell_session_lifecycle`), the same literal `Update`          registration, so a rewind cannot re-run it. It exists because retirement          alone could not make the session-scoped process globals safe — a          teardown that is delayed, misordered or skipped leaves them for the next          game, while a value re-established when a session BEGINS is one that          session wrote. ⚠ it goes stale with both siblings, and for one reason:          if that system moves into `app.sim_schedule()`, all three arguments fail          together",
    ),
    (
        "ambition_platformer2d_rollback_ggrs::local_session::",
        "WHO OWNS the local session and how deeply it verifies — the policy and          the ownership record. Same argument as the driver state above and the          same module family in spirit: this decides whether a session EXISTS,          so a rewind that restored it would be restoring the thing doing the          restoring. ⚠ it is also not per-tick state: the policy changes when a          developer asks for a proof pulse, and the ownership record when          gameplay starts or ends",
    ),
    // The struck-block flinch, which is presentation and is keyed to nothing the
    // sim reads.
    //
    // This sweep unwraps `Messages<T>` to `T` (see `unwrap_message_buffer`), so a
    // message channel arrives here as ordinary state and needs the same argument
    // its sibling instrument already carries. `BlockStruck` has exactly ONE
    // reader — `flinch_struck_blocks`, registered in `Update` in the RENDER
    // plugin — which writes only a `Transform` on a `BlockVisual` and a
    // `BlockFlinch` that nothing outside presentation reads, advancing on the
    // WALL clock. The block's geometry is authoritative and static BY DESIGN:
    // `block_nudge`'s module doc says moving the box would lift a body standing
    // on it and give a rollback an animation to rewind, which is exactly why the
    // nudge is a drawn offset. A rewound-or-not flinch changes no simulation
    // state a checksum can see.
    //
    //  the matching entry in `rollback_exit_oracle`'s `NOT_REWOUND` argues the
    // stale-CURSOR half (what a reader resumes from). This one argues the
    // stale-BUFFER half (what the resource holds). Same subject, two instruments,
    // and each states its own question rather than pointing at the other.
    (
        "ambition_platformer2d_shared_tangle::block_nudge::",
        "a struck block's flinch is a drawn offset: one render-plugin reader on          the wall clock, writing only presentation components, over geometry          that is authoritative and static by design",
    ),
    //  AUTHORED CONTENT, written once and never by a system. The game's
    // fighter difficulty rungs, lowered from the compiled content pack and
    // inserted at plugin build. No system mutates it; there is no tick at which
    // its value differs from the tick before, so there is nothing for a rewind to
    // restore. What a rewind DOES restore is the brains built from it, and those
    // are ordinary rollback state.
    //
    //  the question this answers is not "is it important" — it is very
    // important, and a fighter reads it on the frame it spawns. It is whether a
    // REWIND can observe it changing, and it cannot: the only writer is
    // `AmbitionContentPlugin`, before any frame runs.
    (
        "ambition_characters::brain::fighter::profile::AuthoredFighterLadder",
        "authored difficulty rungs, lowered from the content pack at plugin build          and never written by a system: no tick changes it, so a rewind has          nothing to restore",
    ),
    // Bevy wrapper resources around non-simulation machinery.
    ("bevy_asset::", "asset plumbing"),
    (
        "bevy_state::",
        "host session gating (GameMode); GGRS frames only advance in gameplay mode",
    ),
];

/// A bare substring `contains` let every type entry shadow its neighbors —
/// `::enemies::CharacterRoster` silently waived `CharacterRosterRegistry`,
/// and any NEW mutable resource whose path happened to contain a waived
/// fragment vanished from the sweep. Entry spelling now selects scope:
/// - ends with `::` — a deliberate MODULE-family waiver (`contains`);
/// - ends with `<`  — a generic type's prefix (`contains`);
/// - otherwise      — a full type-path SUFFIX (`ends_with`): one entry, one
///   type, and a new sibling type must earn its own waiver row.
fn resource_waived(name: &str) -> bool {
    RESOURCE_WAIVED.iter().any(|(needle, _)| {
        if needle.ends_with("::") || needle.ends_with('<') {
            name.contains(needle)
        } else {
            name.ends_with(needle)
        }
    })
}

/// Strip Bevy's message-buffer wrapper so a buffer is judged by its message
/// type: `clear_message_on_rollback` registrations record the MESSAGE type
/// name, while `iter_resources` reports the `Messages<T>` wrapper.
fn unwrap_message_buffer(name: &str) -> &str {
    name.strip_prefix("bevy_ecs::message::messages::Messages<")
        .and_then(|inner| inner.strip_suffix('>'))
        .unwrap_or(name)
}

/// The resource sweep shared by the forcing function and its poison test:
/// every `ambition_`-named resource in `world` that is neither registered,
/// declared derived, nor waived.
fn unaccounted_resources(world: &World) -> Vec<String> {
    let known: BTreeSet<String> = world
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("rollback registry is installed by the engine plugins")
        .descriptors()
        .map(|d| d.type_name.clone())
        .collect();

    let mut unaccounted: Vec<String> = Vec::new();
    let mut seen_any = false;
    for (info, _) in world.iter_resources() {
        let full_name = info.name().to_string();
        if !full_name.contains("ambition_") {
            continue;
        }
        seen_any = true;
        let name = unwrap_message_buffer(&full_name);
        if known.contains(name) || waiver(name).is_some() || resource_waived(name) {
            continue;
        }
        unaccounted.push(full_name);
    }
    assert!(
        seen_any,
        "no ambition resources found — the fixture did not actually boot a world, \
         so a green result here would be vacuous"
    );
    unaccounted.sort();
    unaccounted
}

/// Poison: an unregistered, unwaived resource whose type path contains
/// `ambition_` (via this module's name), so the sweep must flag it.
mod ambition_poison {
    #[derive(bevy::prelude::Resource, Default)]
    pub struct DeliberatelyUnregistered;
}

#[test]
fn the_resource_sweep_actually_catches_an_unregistered_resource() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    sim.world_mut()
        .insert_resource(ambition_poison::DeliberatelyUnregistered);
    let flagged = unaccounted_resources(sim.world());
    assert!(
        flagged
            .iter()
            .any(|name| name.contains("DeliberatelyUnregistered")),
        "the sweep failed to flag a deliberately unregistered resource — \
         every green result it has ever produced is suspect: {flagged:?}"
    );
}

/// Sweep the shipped multi-provider composition for mutable Ambition resources.
/// This fixture inspects composed state only; it does not advance a live rollback session,
/// so runtime-created resources require the simulation sweep below.
#[test]
fn every_mutable_ambition_resource_in_the_shipped_composition_is_accounted() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // A few frames so lazily-inserted runtime resources exist, exactly as the
    // sandbox sweep steps its harness first.
    for _ in 0..8 {
        app.update();
    }

    // Message channels have their own rollback oracle. Restrict this sweep to resources whose
    // type names start with `ambition_` so GGRS/render storage mentioning Ambition types is excluded.
    let unaccounted: Vec<String> = unaccounted_resources(app.world())
        .into_iter()
        .filter(|name| !name.contains("::Messages<"))
        .filter(|name| name.starts_with("ambition_"))
        .collect();

    // No simulation-mutated resource may remain unclassified. Register it,
    // declare it derived, or add a narrowly justified waiver.
    const UNACCOUNTED_CEILING: usize = 0;
    if unaccounted.len() > UNACCOUNTED_CEILING {
        let mut report = format!(
            "The SHIPPED composition gained an unaccounted resource: {} now, \
             ceiling {UNACCOUNTED_CEILING}.\n\
             Register it in its owning content plugin's rollback seam, declare it \
             derived, or waive it with a reason — then LOWER the ceiling.\n\n",
            unaccounted.len(),
        );
        for type_name in &unaccounted {
            report.push_str(&format!("  {type_name}\n"));
        }
        panic!("{report}");
    }
    // Keep the ceiling exact so classifications cannot make this guard slack.
    assert!(
        unaccounted.len() >= UNACCOUNTED_CEILING,
        "the ceiling is stale: only {} unaccounted now, so lower \
         UNACCOUNTED_CEILING to {} and keep the ratchet tight",
        unaccounted.len(),
        unaccounted.len(),
    );
}

#[test]
fn every_mutable_ambition_resource_is_registered_derived_or_waived() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    // Step a few frames so lazily-inserted runtime resources exist.
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }
    // Seat a match so resources that exist only during active matches are included.
    seat_a_two_cpu_match(&mut sim);

    let unaccounted = unaccounted_resources(sim.world());

    if !unaccounted.is_empty() {
        let mut report = String::from(
            "Resources live in the simulated world that GGRS will not rewind.\n\
             For each one: register it in `register_engine_rollback_state` (or the\n\
             owning content plugin's rollback seam), declare it derived, or add a\n\
             justified waiver to RESOURCE_WAIVED / WAIVED in this file.\n\n",
        );
        for type_name in &unaccounted {
            report.push_str(&format!("  {type_name}\n"));
        }
        panic!("{report}");
    }
}

/// Build a sim whose simulation lives in `FixedUpdate`, boot it, and then stop
/// the fixed clock so `Update` keeps running while the sim cannot.
fn sim_with_a_stopped_clock() -> Platformer2dSimHarness {
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    let mut sim = Platformer2dSimHarness::new_with_options(
        ambition_app::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_fixed_tick(true),
    )
    .expect("sandbox sim builds");
    for _ in 0..40 {
        sim.step(AgentAction::default());
    }
    sim.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    // One frame to drain whatever the switch itself disturbed.
    sim.step(AgentAction::default());
    sim
}

/// Every RESTORED resource — not the derived ones — the registry knows about.
pub(crate) fn restored_resource_type_names(world: &World) -> BTreeSet<String> {
    use ambition_platformer2d::rollback::RollbackEntryKind;
    world
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("rollback registry is installed by the engine plugins")
        .descriptors()
        .filter(|descriptor| {
            matches!(
                descriptor.kind,
                RollbackEntryKind::ResourceCanonical
                    | RollbackEntryKind::ResourceClone
                    | RollbackEntryKind::ResourceCloneCursor
                    | RollbackEntryKind::ResourceCloneCustomChecksum
            )
        })
        .map(|descriptor| descriptor.type_name.clone())
        .collect()
}

/// Which of `watched` changed while the clock was stopped.
///
/// `between_frames` runs once per frame OUTSIDE any schedule, which is what the
/// poison test uses to stand in for a render-frame writer — and is a faithful
/// stand-in, because "not the sim schedule" is the whole of the claim.
fn changed_while_the_sim_could_not_run(
    sim: &mut Platformer2dSimHarness,
    watched: &BTreeSet<String>,
    mut between_frames: impl FnMut(&mut World),
) -> Vec<String> {
    let baseline = sim.world().read_change_tick();
    for _ in 0..4 {
        between_frames(sim.world_mut());
        sim.step(AgentAction::default());
    }
    let now = sim.world().read_change_tick();

    let world = sim.world();
    let mut written: Vec<String> = Vec::new();
    for (info, _) in world.iter_resources() {
        let name = unwrap_message_buffer(info.name().as_ref()).to_string();
        if !watched.contains(&name) {
            continue;
        }
        let Some(ticks) = world.get_resource_change_ticks_by_id(info.id()) else {
            continue;
        };
        if ticks.is_changed(baseline, now) {
            written.push(name);
        }
    }
    written.sort();
    written.dedup();
    written
}

/// Poison: a resource registered as RESTORED rollback state that something
/// outside the sim schedule writes. The sweep must flag it, or every green
/// result it produces is worthless.
mod render_frame_poison {
    #[derive(bevy::prelude::Resource, Default)]
    pub struct WrittenOffTheSimSchedule(pub u32);
}

#[test]
fn the_render_frame_sweep_actually_catches_a_write_from_outside_the_sim() {
    let mut sim = sim_with_a_stopped_clock();
    let type_name =
        std::any::type_name::<render_frame_poison::WrittenOffTheSimSchedule>().to_string();
    sim.world_mut()
        .insert_resource(render_frame_poison::WrittenOffTheSimSchedule::default());

    let watched: BTreeSet<String> = std::iter::once(type_name.clone()).collect();
    let flagged = changed_while_the_sim_could_not_run(&mut sim, &watched, |world| {
        world
            .resource_mut::<render_frame_poison::WrittenOffTheSimSchedule>()
            .0 += 1;
    });
    assert!(
        flagged.contains(&type_name),
        "the sweep did not notice a resource written from outside the sim \
         schedule, so it cannot have proved anything about the ones it passed: \
         {flagged:?}"
    );
}

/// Verify that rollback-restored resources are not written from render-only
/// frames. A frame where the simulation cannot run isolates writes outside the
/// sim schedule; any restored resource that changes is invalid. Derived
/// resources are excluded because republishing them outside rollback is their
/// contract.
#[test]
fn no_render_only_frame_writes_a_rollback_registered_resource() {
    let mut sim = sim_with_a_stopped_clock();
    let watched = restored_resource_type_names(sim.world());
    assert!(
        watched.len() > 10,
        "only {} restored resources found, so this would sweep an almost empty \
         set and pass for the wrong reason",
        watched.len()
    );

    eprintln!(
        "[render-frame sweep] {} restored resources watched over 4 render-only frames",
        watched.len()
    );
    let written = changed_while_the_sim_could_not_run(&mut sim, &watched, |_| {});
    assert!(
        written.is_empty(),
        "these rollback-registered resources changed during frames in which the \
         simulation did not run, so something outside the sim schedule is \
         writing simulation state:\n  {}\n\n\
         A resimulation replays sim steps, not render frames — so whatever the \
         render frame contributed is simply lost, and the restored value \
         disagrees with the one that was live. Move the writer into the sim \
         schedule, or (if the value genuinely is presentation) take it out of \
         the rollback registry.",
        written.join("\n  ")
    );
}

// A registered component is actually rewound only on entities carrying a GGRS rollback
// anchor. This runtime sweep checks that registered components co-occur with such an anchor.

/// ⛔⛔ AN AUTHORED COLUMN THAT MOVES IS DYNAMIC STATE, and the rule that let it
/// off said the opposite. `TemporaryZone`'s registration note reasoned that "an
/// authored column is room geometry a room load rebuilds", which is true of a
/// STATIC column and false of an oscillating one: `oscillate_gravity_zones` runs
/// every simulation tick, advances `phase`, and rewrites the zone's region from
/// it. Re-running the room constructor rebuilds phase ZERO, not the phase at
/// historical tick N, so a rewind through a moving column had nothing to restore
/// its position from — and every body riding it rode it to the wrong place.
///
/// ⭐ THE PREMISE IS MEASURED FIRST. The arm asserts the phase actually ADVANCES
/// before it asserts the anchor, because "this state is mutable" is the whole
/// reason the anchor is owed — a column that never moved would need no snapshot
/// and this test would be agreeing with nothing.
///
/// ⚠ The sandbox authors exactly one, in `gravity_lab`, amplitude 150.
#[test]
fn the_authored_oscillating_column_is_in_the_rollback_envelope() {
    use ambition_platformer2d::platformer::gravity::OscillatingZone;

    let mut sim = Platformer2dSimHarness::new_with_options(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("gravity_lab"),
    )
    .expect("the sandbox boots into the gravity lab");

    let column = {
        let world = sim.world_mut();
        let mut columns = world.query_filtered::<Entity, With<OscillatingZone>>();
        let found: Vec<Entity> = columns.iter(world).collect();
        assert_eq!(
            found.len(),
            1,
            "gravity_lab authors exactly one sliding gravity column (amplitude \
             150); if that changed, this arm is measuring the wrong room"
        );
        found[0]
    };

    let before = sim
        .world()
        .get::<OscillatingZone>(column)
        .expect("the column carries its oscillator")
        .phase;
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let after = sim
        .world()
        .get::<OscillatingZone>(column)
        .expect("the column still carries its oscillator")
        .phase;
    assert_ne!(
        before, after,
        "the authored column's phase did not advance over 30 ticks, so this arm \
         is not measuring mutable simulation state at all"
    );

    let anchors = rollback_anchors(&mut sim);
    let carried: BTreeSet<String> = sim
        .world()
        .inspect_entity(column)
        .expect("the column exists")
        .map(|info| info.name().to_string())
        .collect();
    assert!(
        carried.intersection(&anchors).next().is_some(),
        "the sliding gravity column carries NO rollback anchor, so its advancing \
         phase is never snapshotted and a rewind restores the column to wherever \
         the live timeline last left it. It carries: {carried:#?}"
    );
}

/// Unanchored archetypes whose registered components are immutable after construction.
/// A waiver stops applying as soon as the archetype gains simulation-mutated registered state.
const INERT_WAIVED: &[(&str, &str)] = &[
    (
        "ambition_platformer2d_shared_tangle::lifecycle::markers::RoomVisual",
        "presentation-only: a room visual carries a Transform the renderer reads \
         and the simulation never writes. It is rebuilt with its room, not \
         restored with the frame.",
    ),
    (
        "ambition_portal2d::gun_pickup::PortalGunPickup",
        "an authored world FIXTURE, placed by the room and never moved. Taking it \
         is a portal-gun grant, and THAT is rollback state on the taker \
         (`PlacedPortal` is anchored); the pedestal itself holds only its \
         construction provenance.",
    ),
];

/// Construction-provenance components are write-once, so an entity stranded only on this set
/// needs no snapshot restore; adding any mutable registered component makes it fail the sweep.
const PROVENANCE_ONLY: &[&str] = &[
    "ambition_platformer2d_shared_tangle::construction::SpawnOrigin",
    "ambition_platformer2d_shared_tangle::construction::TransactionId",
    "ambition_platformer2d_shared_tangle::sim_id::SimId",
    "ambition_platformer2d_shared_tangle::lifecycle::markers::RoomScopedEntity",
    "bevy_ecs::name::Name",
];

fn is_provenance_only(stranded: &BTreeSet<String>) -> bool {
    !stranded.is_empty()
        && stranded
            .iter()
            .all(|name| PROVENANCE_ONLY.contains(&name.as_str()))
}

fn inert_waiver(components: &BTreeSet<String>) -> Option<&'static str> {
    INERT_WAIVED
        .iter()
        .find(|(marker, _)| components.contains(*marker))
        .map(|(_, reason)| *reason)
}

/// Type names registered as SNAPSHOT state for a component (not resources, not
/// anchors, not derived declarations).
fn component_state_registrations(sim: &mut Platformer2dSimHarness) -> BTreeSet<String> {
    use ambition_platformer2d::rollback::RollbackEntryKind as K;
    sim.world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("rollback registry is installed by the engine plugins")
        .descriptors()
        .filter(|d| {
            matches!(
                d.kind,
                K::ComponentCanonical
                    | K::ComponentCloneCursor
                    | K::ComponentCloneResolved
                    | K::ComponentClone
                    | K::ComponentCloneCanonicalChecksum
                    | K::ComponentCloneCustomChecksum
            )
        })
        .map(|d| d.type_name.clone())
        .collect()
}

/// Type names whose PRESENCE puts an entity in the rollback envelope.
fn rollback_anchors(sim: &mut Platformer2dSimHarness) -> BTreeSet<String> {
    use ambition_platformer2d::rollback::RollbackEntryKind as K;
    sim.world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
        .expect("rollback registry is installed by the engine plugins")
        .descriptors()
        .filter(|d| matches!(d.kind, K::RequiredRollback | K::DynamicAnchor))
        .map(|d| d.type_name.clone())
        .collect()
}

/// Every entity in `sim`'s swept population that carries snapshot-registered
/// components but NO anchor, with the registrations that are therefore inert on
/// it.
fn inert_registrations(sim: &mut Platformer2dSimHarness) -> BTreeMap<String, BTreeSet<String>> {
    let state = component_state_registrations(sim);
    let anchors = rollback_anchors(sim);
    let population = simulated_population(sim);
    let world = sim.world();
    let mut inert: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for entity in population {
        let Ok(components) = world.inspect_entity(entity) else {
            continue;
        };
        let names: BTreeSet<String> = components.map(|info| info.name().to_string()).collect();
        if names.intersection(&anchors).next().is_some() {
            continue;
        }
        if inert_waiver(&names).is_some() {
            continue;
        }
        let stranded: BTreeSet<String> = names.intersection(&state).cloned().collect();
        if stranded.is_empty() || is_provenance_only(&stranded) {
            continue;
        }
        // Keyed by the stranded SET rather than by entity: the same archetype
        // strands the same way however many copies of it the room holds, and a
        // failure listing 40 entities is a failure nobody reads.
        let key = stranded.iter().cloned().collect::<Vec<_>>().join(" + ");
        // The failure named a shape and could never name a thing.
        //
        //  the entity's NAME is what an investigation actually needs, and
        // `tracks.md` says so in its own words: *"the next investigation should
        // probe inside `Platformer2dSimHarness` … and print the entity's `Name`,
        // rather than re-deriving that the shell is involved."* Putting it in the
        // instrument means that probe never has to be written.
        //
        // Still deduped, so 40 copies of one prop stay one line.
        let label = world
            .get::<bevy::prelude::Name>(entity)
            .map(|name| name.as_str().to_string())
            .unwrap_or_else(|| format!("<unnamed {entity}>"));
        inert.entry(key).or_default().insert(label);
    }
    inert
}

fn assert_no_inert_registrations(sim: &mut Platformer2dSimHarness, room: &str) {
    let inert = inert_registrations(sim);
    assert!(
        inert.is_empty(),
        "in {room}: these components are registered as rollback STATE but live on \
         entities carrying no rollback anchor, so their registration is INERT — \
         the registry lists them, the coverage sweep counts them as accounted, \
         and nothing restores them. Either give the archetype an anchor \
         (`require_rollback::<A>` for a component it already carries), or the \
         registration is a claim the engine does not honour:\n{inert:#?}"
    );
}

/// The boot world's snapshot registrations all actually apply.
#[test]
fn no_snapshot_registration_is_inert_in_the_boot_world() {
    let mut sim = Platformer2dSimHarness::new().expect("sandbox sim boots");
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }
    assert_no_inert_registrations(&mut sim, "the boot world");
}

#[test]
fn no_snapshot_registration_is_inert_in_a_live_match() {
    // Fixed-tick, like its sibling sweep: `seat_a_two_cpu_match` drives the
    // seating retry to completion and the default timestep does not reach it.
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    //  the helper returns the TICK the match activated, not a seat count — and
    // with the S2 transaction that tick is 0, because every seat now resolves
    // and commits together. Count the bodies, like the sibling sweep does.
    seat_a_two_cpu_match(&mut sim);
    let seated = {
        let world = sim.world_mut();
        let mut q = world.query::<&ambition_platformer2d::versus_match::MatchSeat>();
        q.iter(world).count()
    };
    assert_eq!(
        seated, 2,
        "the match fixture seated {seated} fighters, so this check swept a world \
         with no match in it"
    );
    assert_no_inert_registrations(&mut sim, "a live match");
}

/// The instrument itself goes red, which is the only reason to trust the two
/// tests above. Spawn a body-shaped entity carrying a snapshot-registered
/// component and NO anchor — exactly the mistake — and confirm it is named.
#[test]
fn the_inert_sweep_actually_catches_an_unanchored_registration() {
    let mut sim = Platformer2dSimHarness::new().expect("sandbox sim boots");
    for _ in 0..4 {
        sim.step(AgentAction::default());
    }
    // `MatchSeat` is registered canonical and is normally worn by a body, which
    // carries the `BodyKinematics` anchor. On a bare entity it is stranded.
    sim.world_mut()
        .spawn(ambition_platformer2d::versus_match::MatchSeat(0));

    let inert = inert_registrations(&mut sim);
    assert!(
        inert.keys().any(|key| key.contains("MatchSeat")),
        "the sweep did not notice a snapshot-registered component on an \
         unanchored entity, so its green result above proves nothing: {inert:#?}"
    );
}

/// The shipped sweep, AS PLAYED — B9's blind spot, closed.
///
///  a DIFFERENCE ratchet, not a count. The absolute unaccounted number is
/// the other sweep's job and its ceiling; what only this fixture can see is the
/// set that appears *because the world played*. Two are known and read clean:
///
/// | `ConfirmedFrameBoundary` | *"published once per simulated frame by the rollback bridge, from the GGRS session's own frame counters"* — re-derived every frame, so a rewind has nothing to put back. The same category as `PendingSeatInputs` and `RollbackExecutionStats`, which the shipped ceiling already waives as the machinery doing the rewinding. |
/// | `bevy_ggrs::Session<…>` | the session itself. |
///
///  anything else appearing here is the interesting case and is why this is
/// an assertion rather than a print: it is a resource the simulation brought into
/// existence, which no other sweep in this file can reach.
#[test]
fn playing_the_shipped_composition_introduces_no_unaccounted_resource() {
    use ambition_platformer2d::game_shell::ShellCommand;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..8 {
        app.update();
    }
    let composed: BTreeSet<String> = unaccounted_resources(app.world()).into_iter().collect();

    app.world_mut()
        .write_message(ShellCommand::GoTo("ambition_gameplay".into()));
    // One sim tick per update, so the session advances on this loop rather than
    // on how fast the machine happens to run it.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_nanos(1_000_000_000u64 / 60),
    ));
    // Wait for the FACT (a player exists), not a frame count: the route
    // activates only after the first room's art is decoded (`prepare-first-
    // room-art`), and decode is wall time shared with every other test in this
    // process — 240 frames was enough alone and not beside the hall fixtures
    // once they decoded Full sheets (2026-09-02). The cap says the harness
    // gave up, not that the world is slow.
    let mut players = 0;
    let mut updates = 0;
    while players != 1 && updates < 6_000 {
        app.update();
        updates += 1;
        players = app
            .world_mut()
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .iter(app.world())
            .count();
    }
    // Then let it PLAY: the resources this fixture is about are the ones the
    // running simulation brings into existence.
    for _ in 0..240 {
        app.update();
    }
    assert_eq!(
        players, 1,
        "the gameplay route did not produce a player within {updates} updates, so nothing \
         below was measured against a world that played"
    );
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::rollback::AmbitionGgrsSession>()
            .is_some(),
        "no GGRS session after activating the gameplay route — the simulation did \
         not run, so this sweep is measuring the composed world twice"
    );

    let played: BTreeSet<String> = unaccounted_resources(app.world()).into_iter().collect();
    let fresh: Vec<&str> = played
        .difference(&composed)
        .map(String::as_str)
        .filter(|name| {
            !name.contains("ConfirmedFrameBoundary") && !name.starts_with("bevy_ggrs::Session")
        })
        .collect();

    assert!(
        fresh.is_empty(),
        "playing the shipped composition brought {} resource(s) into existence that \
         no rollback registration, derivation or waiver accounts for — and no other \
         sweep in this file can see them, because the others never run the \
         simulation: {fresh:#?}",
        fresh.len()
    );
}

/// ⛔⛔ THE POPULATION A ONE-SHOT CENSUS CANNOT REACH.
///
/// Every sweep above walks the entities a booted room HAS. `unaccounted_components`
/// says so in its own doc comment, and it has said so for a while: state that
/// only exists after an EVENT is structurally invisible to it, and no amount of
/// sweeping more rooms reaches it, because the entity is not there to sweep.
///
/// That silence read exactly like a pass, and five archetypes shipped inside it.
/// A GPT re-review found the first two by reading the spawn sites:
///
/// - `PortalShot` and `FallingHazard` carried a rollback CODEC and no rollback
///   ANCHOR. `rollback_component_clone::<T>` says what to save IF the entity is
///   in the envelope; `require_rollback::<T>` is what PUTS it there. Both looked
///   completely registered in a `rollback_registration.rs`, and both were inert
///   on the live entity — the registry listed them, `unaccounted_components`
///   counted them accounted, and nothing restored them.
/// - `Sentry`, `VortexWell` and the gravity grenade's `TemporaryZone`/`GravityZone`
///   were worse and quieter: not inert, ABSENT. No codec, no anchor, no waiver,
///   no line in the schema at all.
///
/// This test closes the hole by BUILDING the population instead of hoping to
/// find it. Each archetype comes into the world through the same named seam
/// production uses — `deploy_sentry`, `open_vortex_well`,
/// `open_temporary_gravity_well`, `drop_hazard`, and a real `PortalFireIntent`
/// through `portal_fire_system` — so a fixture cannot assemble a shape
/// production never builds. Then both existing sweeps run over the result.
///
/// ⭐ THE SEAMS ARE THE OTHER HALF OF THE FIX. Three of these five had no
/// callable spawn function at all: they were spawned inline inside a system that
/// first needs a held gauntlet, spent mana, an aim vector or a burnt fuse. An
/// archetype with no seam is an archetype no sweep can reach, so its state is
/// registered on trust forever.
#[test]
fn every_event_created_entity_is_registered_derived_or_waived_and_anchored() {
    use ambition_platformer2d::abilities::ranged::sentry::deploy_sentry;
    use ambition_platformer2d::abilities::ranged::vortex::{open_vortex_well, VortexWell};
    use ambition_platformer2d::abilities::thrown::gravity_grenade::open_temporary_gravity_well;
    use ambition_platformer2d::boss_encounter::{drop_hazard, FallingHazard};
    use ambition_platformer2d::combat::components::ActorFaction;
    use ambition_platformer2d::platformer::lifecycle::SessionSpawnScope;
    use ambition_platformer2d::platformer::sim_id::SimId;
    use ambition_platformer2d::portal::{PortalFireIntent, PortalGunColor, PortalShot};

    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }

    // A real body to aim the hazard at: its `target` is an ENTITY, and a hazard
    // pointed at nothing retires itself on the next tick.
    let target = {
        let world = sim.world_mut();
        let mut bodies = world
            .query_filtered::<Entity, With<ambition_platformer2d::engine_core::BodyKinematics>>();
        bodies
            .iter(world)
            .next()
            .expect("the booted room has a body to drop a hazard on")
    };

    {
        let world = sim.world_mut();
        let mut commands = world.commands();
        deploy_sentry(
            &mut commands,
            SessionSpawnScope::UNSCOPED,
            bevy::math::Vec2::new(96.0, 96.0),
            ActorFaction::Player,
            None,
            None,
            // The identity production mints: these are dynamically-spawned sim
            // entities, and a turret's bolts mint under IT.
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
            Some(SimId::spawned(&SimId::player_slot(0), 2)),
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
            Some(SimId::spawned(&SimId::player_slot(0), 3)),
        );
        world.flush();
    }

    // The portal shot goes through the REAL system, not a hand-built bundle: the
    // defect was in what `portal_fire_system` spawns, so a fixture that spawned
    // its own `PortalShot` would be testing the fixture.
    sim.world_mut().write_message(PortalFireIntent {
        origin: bevy::math::Vec2::new(224.0, 96.0),
        dir: bevy::math::Vec2::new(1.0, 0.0),
        channel: ambition_platformer2d::portal::PortalChannel::Gun(PortalGunColor::BLUE),
        // This file asks whether the shot is COVERED by rollback, not whether it
        // is identified; the identity census in `rollback_populated_timeline`
        // owns that and mints there.
        id: None,
    });
    sim.step(AgentAction::default());

    //  ANTI-VACUITY, and it is the whole test. Every assertion below passes
    // on an EMPTY population, which is exactly the failure mode this file exists
    // to close: a sweep is silent about what is not there, and that silence
    // reads like an all-clear. Each archetype is counted before anything is
    // swept, so a seam that stops spawning turns this red rather than green.
    let counts = {
        let world = sim.world_mut();
        let sentries = world
            .query_filtered::<Entity, With<ambition_platformer2d::abilities::ranged::sentry::Sentry>>()
            .iter(world)
            .count();
        let wells = world
            .query_filtered::<Entity, With<VortexWell>>()
            .iter(world)
            .count();
        let zones = world
            .query_filtered::<Entity, With<ambition_platformer2d::platformer::gravity::TemporaryZone>>()
            .iter(world)
            .count();
        let hazards = world
            .query_filtered::<Entity, With<FallingHazard>>()
            .iter(world)
            .count();
        let shots = world
            .query_filtered::<Entity, With<PortalShot>>()
            .iter(world)
            .count();
        [
            ("sentry", sentries),
            ("vortex well", wells),
            ("temporary gravity zone", zones),
            ("falling hazard", hazards),
            ("portal shot", shots),
        ]
    };
    for (what, count) in counts {
        assert!(
            count > 0,
            "no {what} exists after its production seam was driven, so the sweep \
             below would inspect a world without one and report a confident \
             all-clear — the exact false negative this test was written for"
        );
    }

    // And every one of them must be IN the swept population: existing is not the
    // same fact as being looked at.
    let population: BTreeSet<Entity> = simulated_population(&mut sim).into_iter().collect();
    let dynamic: Vec<(Entity, String)> = {
        let world = sim.world_mut();
        let mut found = Vec::new();
        for entity in world
            .query_filtered::<Entity, With<ambition_platformer2d::abilities::ranged::sentry::Sentry>>()
            .iter(world)
            .collect::<Vec<_>>()
        {
            found.push((entity, "sentry".to_string()));
        }
        for entity in world
            .query_filtered::<Entity, With<PortalShot>>()
            .iter(world)
            .collect::<Vec<_>>()
        {
            found.push((entity, "portal shot".to_string()));
        }
        for entity in world
            .query_filtered::<Entity, With<FallingHazard>>()
            .iter(world)
            .collect::<Vec<_>>()
        {
            found.push((entity, "falling hazard".to_string()));
        }
        found
    };
    for (entity, what) in &dynamic {
        assert!(
            population.contains(entity),
            "a live {what} ({entity}) is not in the swept population, so no \
             amount of sweeping reaches the event-created families"
        );
    }

    assert_components_accounted(&mut sim, "an event-created population");
    assert_no_inert_registrations(&mut sim, "an event-created population");
}
