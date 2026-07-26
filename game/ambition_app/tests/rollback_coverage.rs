//! **The rollback coverage forcing function.**
//!
//! GGRS (ADR 0027) can only rewind state it was told about, and nothing in the
//! type system says "this component is authoritative simulation truth". So
//! coverage rots silently: someone adds a component to a simulated body, never
//! registers it, and a rewind quietly keeps the predicted value. That is a
//! desync in netplay and a wrong outcome in resimulation — with no compile error
//! and no failing test.
//!
//! The July-18 GGRS migration deleted the previous guard (checked-in component
//! and resource debt ledgers) without a replacement; the 2026-07-19 deep review
//! then found nine unregistered mutable families by hand. This is the
//! replacement, and it is COMPUTED rather than checked in: it boots the real
//! sim, looks at what is actually ON the simulated entities, and requires every
//! component found there to be accounted for. A stale ledger cannot drift out
//! from under it, and a new component on a body cannot slip through unnoticed.
//!
//! ## Why entity composition rather than system access
//!
//! Asking "which components do sim systems write" would be the other natural
//! shape, but Bevy 0.18 does not expose per-system `FilteredAccessSet` through
//! any public API (it lives on the crate-private `SystemWithAccess`). Entity
//! composition is public, and is arguably the better question anyway: it asks
//! what state a simulated body actually CARRIES, which is exactly what a
//! rollback has to reproduce. It also catches state parked on an entity by a
//! `Commands` insert, which a system-access walk would miss entirely.
//!
//! Resources are covered by the sibling forcing function below — same
//! contract, over `World::iter_resources` instead of entity composition.
//!
//! ## When this fails
//!
//! You put new state on a simulated entity. Pick one, deliberately:
//!
//! 1. **Register it** in `register_engine_rollback_state` — the default for
//!    anything gameplay-authoritative.
//! 2. **Declare it derived** (`declare_rollback_derived`) if it is recomputed
//!    from authoritative state every frame before anyone reads it.
//! 3. **Waive it below**, with a reason, if it is genuinely not simulation truth
//!    (presentation, dev tooling, device input, host bookkeeping).
//!
//! Do not waive to get green. A wrong choice here is a desync later.

#![cfg(feature = "rl_sim")]

use std::collections::{BTreeMap, BTreeSet};

use ambition_app::{AgentAction, AmbitionSim, SandboxSim, TimestepMode};
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
    ("ambition_portal_presentation::", "presentation"),
    ("ambition_load_presentation::", "presentation"),
    ("ambition_menu", "UI"),
    ("ambition_settings_menu::", "UI"),
    ("ambition_inventory_ui::", "UI"),
    ("ambition_ui_nav::", "UI"),
    ("ambition_dialog::", "narrative view state"),
    (
        "ambition_cutscene::",
        "scripted presentation sequence state",
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
        "ambition_platformer_primitives::lifecycle::session::SessionRoot",
        "session activation identity; assigned at activation, never mutated",
    ),
    (
        "ambition_runtime::content_identity::",
        "content identity; a change invalidates the session rather than moving inside it",
    ),
    (
        "ambition_runtime::session_world::PlatformerSessionCatalogs",
        "which providers this session composed; fixed at activation",
    ),
    (
        "ambition_actors::avatar::starting_character::StartingCharacter",
        "session activation input, resolved once at player spawn",
    ),
    // ── Authored geometry and identity on world props ────────────────────────
    //
    // Same population change surfaced these: a shrine, a moving platform's visual
    // index, and a portal's authored channel all sit on entities that carry
    // registered state, and none of the three is written after the room loads.
    (
        "ambition_actors::shrine::HealShrine",
        "authored shrine geometry; the heal reads it and never writes it",
    ),
    (
        "ambition_actors::world::platforms::MovingPlatformVisual",
        "presentation index for the platform's sprite",
    ),
    (
        "ambition_portal::link::PortalLink",
        "authored portal channel identity, hashed at spawn",
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
///
/// Used to DERIVE the swept population rather than to judge coverage: an entity
/// carrying even one type the rollback knows about is an entity the rollback
/// participates in, and therefore one whose every component has to be accounted for.
fn rollback_vocabulary(sim: &mut SandboxSim) -> BTreeSet<String> {
    sim.world()
        .get_resource::<ambition::runtime::rollback::RollbackRegistry>()
        .expect("rollback registry is installed by the engine plugins")
        .descriptors()
        .map(|d| d.type_name.clone())
        .collect()
}

/// **The population a rewind has to reproduce.**
///
/// Three sources, unioned, and each was added because the previous set produced a
/// confident empty result about something it never looked at:
///
/// * `FeatureSimEntity` — the original set. The PLAYER does not carry it
///   (`PlayerBundle` never inserts it), so the single most heavily-mutated body in
///   the game went uninspected while the rollback oracle was diverging on it.
/// * `BodyKinematics` — anything the sim integrates every tick. That covers the
///   player and every other body regardless of how it was spawned.
/// * **anything carrying a type the rollback vocabulary names.** This is the
///   mechanism's own answer, and it is not a list anyone maintains: if the rollback
///   registers, anchors, or declares-derived even one component on an entity, that
///   entity is in the rollback's world and all of its state is in scope.
///
/// The third source is what reaches the TRANSIENT families. A moveset strike volume
/// carries `Hitbox`, `HitboxHits` and `StrikeVolume` and neither of the two tags
/// above, so for as long as the population was those tags, new state on a live
/// strike volume was outside this instrument no matter how many rooms were added —
/// adding rooms cannot reach a family that exists for six frames.
///
/// `bevy_ggrs::Rollback` would be the obvious spelling of that third source and is
/// the WRONG one here: these fixtures boot a fixed-tick host, where
/// `require_rollback` is recorded for schema identity and never installed, so the
/// marker is absent from every entity in the world and a population derived from it
/// would silently be the old two-tag population again.
fn simulated_population(sim: &mut SandboxSim) -> Vec<Entity> {
    let vocabulary = rollback_vocabulary(sim);
    let world = sim.world_mut();
    let mut found: BTreeSet<Entity> = BTreeSet::new();
    let mut tagged =
        world.query_filtered::<Entity, With<ambition::platformer::lifecycle::FeatureSimEntity>>();
    found.extend(tagged.iter(world));
    let mut bodies =
        world.query_filtered::<Entity, With<ambition::actors::actor::BodyKinematics>>();
    found.extend(bodies.iter(world));

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
    found.into_iter().collect()
}

/// The component sweep for one booted room: every `ambition_`-named component
/// on a simulated entity that is neither registered, declared derived, nor
/// waived. The population differs per room — enemies, switches, and breakables
/// only exist where a room authors them — so callers sweep representative
/// rooms, not just the boot default.
pub(crate) fn unaccounted_components(sim: &mut SandboxSim) -> BTreeMap<String, usize> {
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
        .get_resource::<ambition::runtime::rollback::RollbackRegistry>()
        .expect("rollback registry is installed by the engine plugins")
        .descriptors()
        .filter(|d| d.kind != ambition::runtime::rollback::RollbackEntryKind::RequiredRollback)
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
pub(crate) fn waived_components(sim: &mut SandboxSim) -> BTreeMap<String, &'static str> {
    let known: BTreeSet<String> = sim
        .world()
        .get_resource::<ambition::runtime::rollback::RollbackRegistry>()
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

/// **Print what every waiver is actually covering.** (A18)
///
/// Not an assertion — a listing, so the claims can be re-read against reality.
/// `BossAnimFrame` was swallowed by `ambition_sprite_sheet::` ("sprite metadata /
/// asset binding") while being a sim-owned cursor that boss hurtbox geometry
/// derives from. A prefix waiver assumes a crate holds exactly one kind of thing,
/// and crates grow.
#[test]
#[ignore = "audit listing: prints what each waiver covers; read it, do not assert on it"]
fn list_what_every_waiver_actually_covers() {
    for room in ["combat_calibration_lab", "mockingbird_arena"] {
        let mut sim = SandboxSim::new_with_options(
            ambition_app::rl_sim::SandboxSimOptions::default()
                .with_timestep(TimestepMode::fixed_60hz())
                .with_start_room(room),
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

fn assert_components_accounted(sim: &mut SandboxSim, room: &str) {
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
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
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
    let mut sim = SandboxSim::new_with_options(
        ambition_app::rl_sim::SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("combat_calibration_lab"),
    )
    .expect("sandbox sim builds in the calibration lab");
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }
    assert_components_accounted(&mut sim, "combat_calibration_lab");
}

/// **The same sweep over a BOSS population, which nothing swept before.**
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
    let mut sim = SandboxSim::new_with_options(
        ambition_app::rl_sim::SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("mockingbird_arena"),
    )
    .expect("sandbox sim builds in a boss arena");
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }
    assert_components_accounted(&mut sim, "mockingbird_arena");
}

/// **Populations no sweep had ever visited.** (A19)
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
    ] {
        let mut sim = SandboxSim::new_with_options(
            ambition_app::rl_sim::SandboxSimOptions::default()
                .with_timestep(TimestepMode::fixed_60hz())
                .with_start_room(room),
        )
        .unwrap_or_else(|error| panic!("sandbox sim builds in `{room}`: {error}"));
        for _ in 0..8 {
            sim.step(AgentAction::default());
        }
        assert_components_accounted(&mut sim, room);
    }
}

/// **A TRANSIENT family, swept while it is actually alive.** (GPT 5.6 review 5)
///
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
    let mut sim = SandboxSim::new_with_options(
        ambition_app::rl_sim::SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("combat_calibration_lab"),
    )
    .expect("sandbox sim builds in the calibration lab");
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }

    // Swing repeatedly and sweep on every tick a volume is live. One swing is a
    // handful of frames and the strike's own timeline decides which, so sampling a
    // fixed frame number would be a guess.
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
                world.query_filtered::<Entity, With<ambition::combat::moveset::StrikeVolume>>();
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

/// **The MOUNT population, which authors no LDtk room.** (A20)
///
/// ADR 0020's mount model is two linked actors with two HP pools, welded by
/// `RidingOn` / `Mounted` / `MountSlot`. No swept room authors a mounted pair, so
/// every component that only exists while a body is ridden — the brain cache the
/// weld parks, the mount's borrowed size, the rider's saddle link — had never been
/// in this sweep's population. Population, not accounting: exactly the hole that
/// hid `PogoTarget` and `BossAnimFrame`.
///
/// Built in Rust rather than as a room, because a room id is not available and
/// waiting for one is how a population stays unswept.
#[test]
fn every_component_on_a_mounted_pair_is_registered_derived_or_waived() {
    use ambition::actors::features::{MountSlot, Mounted, RidingOn};
    use ambition::characters::brain::Brain;

    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let home = {
        let world = sim.world_mut();
        let mut q = world.query_filtered::<Entity, ambition::actors::actor::PrimaryPlayerOnly>();
        q.single(world).expect("one primary player")
    };
    let anchor = sim
        .world_mut()
        .get::<ambition::actors::actor::BodyKinematics>(home)
        .expect("the player has a body")
        .pos;

    sim.spawn_enemy_at(
        "sweep_mount",
        "Burning Flying Shark",
        (anchor.x + 120.0, anchor.y),
        (63.0, 26.0),
        ambition::entity_catalog::placements::CharacterBrain::Custom(
            "burning_flying_shark".to_string(),
        ),
    );
    sim.spawn_enemy_at(
        "sweep_rider",
        "Pirate Raider",
        (anchor.x + 120.0, anchor.y - 66.0),
        (22.0, 39.0),
        ambition::entity_catalog::placements::CharacterBrain::Custom("pirate_raider".to_string()),
    );
    let by_id = |sim: &mut SandboxSim, id: &str| {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &ambition::actors::features::FeatureId)>();
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
    sim.world_mut()
        .entity_mut(rider)
        .insert((RidingOn { mount }, Mounted));
    sim.world_mut()
        .entity_mut(mount)
        .insert(MountSlot { rider: Some(rider) });

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

/// **The falling-sand room, which nothing swept.** (A20)
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
        let mut switches = world.query::<&ambition::actors::features::SwitchFeature>();
        let activation = switches
            .iter(world)
            .map(|feature| feature.activation.clone())
            .find(|activation| activation.id == SAND_SWITCH)
            .unwrap_or_else(|| panic!("authored switch `{SAND_SWITCH}` exists in {ROOM_ID}"));
        world.write_message(ambition::actors::features::SwitchActivated {
            activation,
            pos: ambition::engine_core::Vec2::ZERO,
        });
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
        "ambition_runtime::rollback::probes::",
        "rollback diagnostics: measures the rewind, is not reproduced by it",
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
        "ambition_actors::character_runtime::",
        "character art load bookkeeping; decoded-ness has no simulation consequence",
    ),
    // The room-transition transaction, engine-side since 2026-07-25. Under a
    // rollback host `detect_room_transition_system` DEFERS the crossing to the
    // confirmed-frame boundary (`PendingLifecycleCommit`) precisely so this
    // multi-tick load machine never engages on a speculative frame — the policy
    // predates the move and is why it was never rollback state. If a transition
    // ever starts on a predicted frame, this waiver is wrong and the resource
    // has to be registered, not re-justified.
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
    ("::boss_encounter::catalog::", "authored boss catalog"),
    (
        "::boss_encounter::registry::BossEncounterRegistry",
        "authored encounter registry",
    ),
    (
        "::features::banter::CombatBanterRegistry",
        "authored banter registry",
    ),
    ("::features::enemies::CharacterRoster", "authored roster"),
    (
        "::features::enemies::CharacterRosterRegistry",
        "authored roster fragment registry",
    ),
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
    ("::gate_portal::GatePortalRegistry", "authored gate portals"),
    ("::manifest::WorldManifest", "authored world manifest"),
    (
        "::project::SandboxLdtkProject",
        "authored LDtk project; hot reload restarts the session",
    ),
    (
        "::session::data::SandboxDataSpec",
        "authored data-spec value",
    ),
    (
        "::session::data::SandboxDataAsset",
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
        "::hot_reload::LdtkHotReloadState",
        "dev hot-reload machinery; a commit restarts the GGRS session",
    ),
    // Settings and tuning: forward-only knobs, not per-frame simulation state.
    ("::settings::UserSettings", "user settings, forward-only"),
    (
        "::movement::tuning::ActiveMovementTuning",
        "movement tuning, forward-only",
    ),
    (
        "::time::feel::SandboxFeelTuning",
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
        "ambition_runtime::SimulationHost",
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
        "::intro::plugin::IntroSpritesInstalled",
        "install-once latch",
    ),
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
        "::cutscene_trigger::CutsceneTriggerQueue",
        "narrative trigger seam; seen-flags in the rollback-registered SandboxSave dedup re-fires",
    ),
    (
        "::brain::BrainActionCounter",
        "diagnostic counter surfaced by HUD/debug tooling",
    ),
    (
        "::developer_hotkeys::DeveloperAction",
        "developer hotkey message",
    ),
    (
        "::affordances::devices::ActiveInputMethod",
        "last-used input device; drives prompt glyphs, not simulation",
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
    (
        "::cut_rope::PendingCutRopeRoomReplay",
        "dialog-flow latch consumed by the room-reset flow, presentation-gated",
    ),
    // Bevy wrapper resources around non-simulation machinery.
    ("bevy_asset::", "asset plumbing"),
    (
        "bevy_state::",
        "host session gating (GameMode); GGRS frames only advance in gameplay mode",
    ),
];

/// Anchored waiver matching (2026-07-23 rollback review: "narrow waivers").
///
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
        .get_resource::<ambition::runtime::rollback::RollbackRegistry>()
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
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
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

#[test]
fn every_mutable_ambition_resource_is_registered_derived_or_waived() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    // Step a few frames so lazily-inserted runtime resources exist.
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }

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
