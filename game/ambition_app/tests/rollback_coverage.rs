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
    // ⭐ **arrived with K2b edit 2, and it belongs to the group above it.**
    // The build-time root spawned four things — `SessionRoot`, the live world,
    // prepared content and its identity — and never stamped an epoch, because a
    // root that exists before tick 0 has no activation to be a generation OF.
    // A shell activation does stamp one, so deleting the build-time publisher
    // put it on the census.
    //
    // ⚠ **measured, not assumed**: it sits on the SESSION ROOT, in the same
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
    // ⛔ **the one category on this list where rewinding would be actively
    // harmful rather than merely meaningless**, and it is the same category as
    // the device input stream the `ambition_input::` waiver above already
    // covers: a rewind restores what the simulation DECIDED, never what it was
    // TOLD. Erasing an input is how the replay reaches a different decision.
    //
    // The narrative end used to be a `Message` cleared on load, which is exactly
    // that erasure — and the system that would re-deliver it (presentation,
    // watching the live Yarn runner) does not execute between resimulated ticks,
    // so a rewind past the end simply lost it. The ledger records WHICH
    // conversation said each thing and the first `SimTick` the simulation may act
    // on it, so a resimulated tick reaches the same answer at the same tick.
    //
    // ⚠ the waiver names the LEDGER, not one payload: every narrative-input
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
///
/// Used to DERIVE the swept population rather than to judge coverage: an entity
/// carrying even one type the rollback knows about is an entity the rollback
/// participates in, and therefore one whose every component has to be accounted for.
fn rollback_vocabulary(sim: &mut Platformer2dSimHarness) -> BTreeSet<String> {
    sim.world()
        .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
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
fn simulated_population(sim: &mut Platformer2dSimHarness) -> Vec<Entity> {
    let vocabulary = rollback_vocabulary(sim);
    let world = sim.world_mut();
    let mut found: BTreeSet<Entity> = BTreeSet::new();
    let mut tagged =
        world.query_filtered::<Entity, With<ambition_platformer2d::platformer::lifecycle::FeatureSimEntity>>();
    found.extend(tagged.iter(world));
    let mut bodies = world
        .query_filtered::<Entity, With<ambition_platformer2d::actors::actor::BodyKinematics>>();
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
    // ⛔ **ANTI-VACUITY, and it is load-bearing for NINETEEN tests.** Every sweep
    // in this file runs `unaccounted_components` over this population and passes
    // when the result is empty — which is also what an EMPTY POPULATION produces.
    // A fixture that booted no bodies, or a filter that stopped matching after a
    // bundle changed, would turn the whole file green in one commit and read
    // exactly like an all-clear.
    //
    // ⚠ what is asserted is what is TRUE OF EVERY FIXTURE: a body exists, and the
    // union is non-empty. The vocabulary-derived third source is not asserted —
    // a room with no transient volumes legitimately contributes none.
    // ⛔ **NOT asserted per-filter on `FeatureSimEntity`, and the reason is a
    // measurement.** The review that asked for per-filter anti-vacuity
    // (`fable-reply-2026-07-19-b.md` §3) was written when this helper served ONE
    // fixture — a single boot room — and "a filter that matched nothing" could
    // only mean a broken filter. It now serves ten rooms, and `portal_lab`
    // authors no `FeatureSimEntity` at all (measured 2026-08-07, by asserting it
    // and watching that room alone fail). A room that authors no feature entities
    // is a legitimate room, not a broken fixture, so the assert the review asked
    // for would be a false alarm on real content. The granularity that was right
    // for the smoke is wrong for the sweep it became.
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
/// ⚠ **it walks the entities PRESENT in a booted room, so state that only exists
/// after an EVENT is structurally invisible to it.** An item spat out by a
/// struck block, a projectile in flight, a pickup mid-arc: none of them are in a
/// world nobody has played yet, and no amount of sweeping more rooms reaches
/// them. This is a property of a one-shot census, not a gap to be waived, and it
/// is worth stating because the sweep's silence about a component reads exactly
/// like a pass.
///
/// **What covers the other half:** `rollback_exit_oracle`'s PER-FRAME census,
/// which watches a session as it runs and therefore sees state that comes into
/// existence and goes away again. It caught `PickupMagnet` and
/// `SpawnedThisAttempt` on the day this note was written — both transient, both
/// invisible here. The two instruments are complementary, and the failure mode
/// this comment exists to prevent is reading one of them as if it were both.
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

/// **Print the shipped composition's remaining unaccounted resources.**
///
/// The sibling of the waiver listing below, for the other sweep. Not an
/// assertion — the ceiling test is the assertion; this is how you READ what the
/// ceiling is holding, which is the first thing anyone lowering it further needs.
///
/// ⭐ these 25 are deliberately NOT covered by a category waiver. Eight families
/// earned one on 2026-08-03 (menus, dev instruments, transition presentation,
/// asset staging, content presentation, the presentation-profile stack,
/// persistence, the rollback driver) and took the count 64 → 25. What remains is
/// what no family could honestly swallow: a mix of provider-lifecycle catalogs,
/// session-scope markers, authored art manifests and a few genuinely ambiguous
/// ones.
///
/// ⚠ **read them one at a time and register or waive INDIVIDUALLY.** Both bugs
/// this sweep has caught — `BrokenBricks` and `SpentMonitors` — were in a demo
/// provider's namespace, which is precisely where a broad new family waiver would
/// hide the next one.
#[test]
#[ignore = "audit listing: prints what the ceiling is holding; read it, do not assert on it"]
fn list_what_the_shipped_ceiling_is_still_holding() {
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
fn list_what_every_waiver_actually_covers() {
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
        // ⭐ **the room `rollback_exit_oracle` itself simulates**, added
        // 2026-08-11 while chasing D78 — a character-first enemy body there
        // desyncs the oracle, and this instrument was the first thing to ask.
        // It passes WITH that body present, which is the finding: the divergence
        // is not an unaccounted COMPONENT. The room stays swept regardless; a
        // population the oracle trusts should be one this instrument has seen.
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
    let mut sim = Platformer2dSimHarness::new_with_options(
        ambition_app::rl_sim::Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("combat_calibration_lab"),
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
        // I4 (GPT 5.6 review 5): being in the population is not enough. The
        // entity-reference probes over `Hitbox`/`StrikeVolume`/`HitboxHits` fold
        // each carrier through its `SimId`, and these spawned WITHOUT one — so
        // every anonymous carrier contributed the same constant and two live
        // hitboxes with swapped owners hashed identically. That is the exact
        // permutation the pair projection was added to catch, defeated on the one
        // family it was added for. Checked here rather than in a unit fixture
        // because the claim is about the box the real move path opens.
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

/// **The MOUNT population, which authors no LDtk room.** (A20)
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
    use ambition_platformer2d::actors::features::{MountSlot, Mounted, RidingOn};
    use ambition_platformer2d::characters::brain::Brain;

    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let home = {
        let world = sim.world_mut();
        let mut q = world
            .query_filtered::<Entity, ambition_platformer2d::actors::actor::PrimaryPlayerOnly>();
        q.single(world).expect("one primary player")
    };
    let anchor = sim
        .world_mut()
        .get::<ambition_platformer2d::actors::actor::BodyKinematics>(home)
        .expect("the player has a body")
        .pos;

    // ⭐ **the pair NAMES its characters** (D102). Both said only a brain key,
    // and both of those archetype rows were DELETED when the shark and the
    // raider became characters — so this rollback sweep had been walking a pair
    // of generic `combatant` bodies: not a mount, not a pilot, and none of the
    // components it is here to register.
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
        let mut q = world.query::<(Entity, &ambition_platformer2d::actors::features::FeatureId)>();
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

/// Seat a two-CPU match through the PRODUCTION seating system and step until it
/// activates. Returns the tick the last seat landed on.
///
/// Two CPU seats, so neither depends on the harness having a primary player
/// wearing the right character. A seat that silently fails to adopt is how a
/// match sweep ends up inspecting an empty roster and reporting success.
///
/// ⚠ the robot lineage, not the arena duelists. A plain `Platformer2dSimHarness` prepares
/// exactly `["player_robot_v2", "player_robot_v3", "robot"]` — the duelists are
/// versus-ROUTE content — and `seat_character` returns `None` for an unprepared
/// id, silently. The vacuity guard is what said so; the first version of this
/// named the duelists and swept nothing.
fn seat_a_two_cpu_match(sim: &mut Platformer2dSimHarness) -> usize {
    use ambition_platformer2d::actors::character_runtime::{
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
        opens_suspended: true,
        // No ceremony in a rollback fixture: the stage that owns the opening
        // is not part of what these tests exercise.
        opening_countdown_ticks: 0,
        time_limit_ticks: 0,
        seating: ambition_platformer2d::actor::RosterSeating::activated_at(7),
        fighter_abilities: None,
        fighter_body: None,
        fighter_stocks: None,
        fighter_health_pool: None,
        // A fixture's roster has no publisher: nothing else in this App claims
        // one, which is the case `None` is for.
        published_by: None,
    });
    // A direct world mutation is setup, not gameplay: it becomes frame zero.
    sim.rebase_rollback_history()
        .expect("the roster insert becomes the rollback baseline");

    for tick in 0..90 {
        sim.step(AgentAction::default());
        if sim
            .world()
            .get_resource::<ambition_platformer2d::actors::character_runtime::ActiveMatch>()
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

/// **A LIVE MATCH, which nothing swept.** (AA2 / AC2)
///
/// Two independent GPT 5.6 reviews named `ActiveMatch` and `MatchSeat` as
/// simulation-critical state outside rollback, and both were right. What is
/// worth recording is why neither this instrument nor any other caught it
/// first: **no swept population contained a match.** That is the exact shape
/// A19 already hit — `PogoTargetContributor`, `ChestFeature` and `PortalHostScanned` were
/// not unregistered-and-missed, they were never in the population — and the
/// lesson evidently did not generalise on its own. A sweep answers only the
/// question its population asks.
///
/// So the guard comes before the fix. This seats a real two-CPU roster through
/// the production `seat_match_participants` and sweeps every tick of the match's
/// life, including the activation tick, which is the one the reviews say a
/// rewind crosses badly.
#[test]
fn every_component_in_a_live_match_is_registered_derived_or_waived() {
    use ambition_platformer2d::actors::character_runtime::MatchSeat;

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
        let mut switches = world.query::<&ambition_platformer2d::actors::features::SwitchFeature>();
        let activation = switches
            .iter(world)
            .map(|feature| feature.activation.clone())
            .find(|activation| activation.id == SAND_SWITCH)
            .unwrap_or_else(|| panic!("authored switch `{SAND_SWITCH}` exists in {ROOM_ID}"));
        world.write_message(ambition_platformer2d::actors::features::SwitchActivated {
            activation,
            pos: ambition_platformer2d::engine_core::Vec2::ZERO,
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
    // **The condition catalog**: which questions the installed domains can
    // answer, and the function that answers each.
    //
    // ⭐ IMMUTABLE ONCE THE SIMULATION STARTS, AND STRUCTURALLY SO. `publish` is
    // private to its module; the only way in is `PublishCondition` on `App`, and
    // a tick holds a `World`, never an `App`. So a rewind restoring this would
    // restore a byte-identical value — there is no timeline in which the set of
    // questions the engine can answer differs.
    //
    // ⚠ this waiver is about the CATALOG, not about answers. An evaluator reads
    // live state, and that state is registered by whichever domain owns it; the
    // day rule EXECUTION gains runtime state (a cursor, a latch, a timer), that
    // state is a different value with a different answer, and D127's M4 says so
    // explicitly.
    (
        "ambition_platformer2d_shared_tangle::authored_logic::ConditionCatalog",
        "published during plugin build only; `publish` is private and a tick has no `App`",
    ),
    // **The command catalog**: which verbs the installed domains can perform,
    // and the function that performs each.
    //
    // ⭐ THE SAME STRUCTURAL ARGUMENT, and this is the half where it had to be
    // made before anything was built. `publish` is private, the only way in is
    // `PublishCommand` on `App`, and a tick holds a `World`. ⛔ a command
    // registry a system could write to IS rollback state, and then every
    // authored verb in the game joins the snapshot.
    //
    // ⚠ and `run` is private too, which is a different claim from this waiver
    // but the reason the waiver is not merely true: nothing can perform a
    // command out of `AuthoredCommandSet`, so there is no timeline in which the
    // catalog and the world disagree about what happened.
    (
        "ambition_platformer2d_shared_tangle::authored_logic::commands::CommandCatalog",
        "published during plugin build only; `publish` is private and a tick has no `App`",
    ),
    // **Every game's death rules** (ADR 0033): how long a death holds, and the
    // roster question that decides a level reset — one declaration per game,
    // keyed by the rooms that game governs.
    //
    // AUTHORED CONSTANTS, stated once when each game's plugin is built and never
    // written by any system. A rewind cannot change what a game's rules are —
    // rewinding them would be rewinding the ruleset itself, not the simulation
    // it governs. The state the rules PRODUCE (`DeathInterlude`, `OutOfPlay`)
    // is per-body and IS registered, in the combat domain.
    //
    // ⚠ **the argument survived the collection becoming plural** (2026-08-16):
    // `declare` is only reachable through `App`, and a tick holds a `World`. It
    // would NOT survive a resolved-rules resource written each tick from the
    // active room — which is why the resolution is a `SystemParam` that stores
    // nothing rather than a derived global.
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
    // **Whether the twintrack spacetime MINIMAP is showing.** A viewer's toggle
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
    // ⭐ **The two relativity READ MODELS, and the reason is not "presentation"
    // but REPUBLICATION.** Both are recomputed every frame in `Update` from
    // `SpacetimeCoordinateTime2d` and canonical `BodyKinematics` — no
    // accumulator, no entity, nothing carried between frames. A restored value
    // is overwritten before anything reads it, so rewinding them is not harmful,
    // it is a no-op with a cost.
    //
    // ⚠ **waived rather than DECLARED DERIVED, deliberately.** A derived
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
    //
    // ⚠ **NARROWED from the module family `ambition_platformer2d_actor_monolith::character_runtime::`
    // on 2026-07-29, and the widening was the whole bug.** That waiver was
    // written when the module held art-load bookkeeping and nothing else. The
    // module then grew SEATING — `ActiveMatch`, the latch that decides whether a
    // match is live and whether seating may run — and the waiver silently
    // covered it, so the resource sweep reported green over simulation-critical
    // state for as long as both existed. Two GPT 5.6 reviews found it by
    // reading; this instrument could not, because it had already excused it.
    //
    // Third instance of this exact class: `BossAnimFrame` was swallowed by a
    // crate-prefix waiver reading "sprite metadata / asset binding" (A9/A18),
    // and `::enemies::CharacterRoster` shadowed `CharacterRosterRegistry` before
    // anchored matching landed. **A module-family waiver is a standing bet that
    // nobody will ever put simulation state in that module.** Prefer one entry
    // per type; if a family entry is genuinely right, it has to be re-earned
    // every time the module grows.
    (
        "::character_runtime::CharacterLoadStates",
        "character art load bookkeeping; decoded-ness has no simulation consequence",
    ),
    (
        "::character_runtime::CharacterLoadDemand",
        "which sheets have been ASKED for; idempotent, and a decode has no simulation consequence",
    ),
    (
        "::character_runtime::CharacterMaterializationService",
        "the art materializer seam itself; holds no per-frame simulation state",
    ),
    (
        // ⚠ **the path moved crates, and the guard CAUGHT it** — which is what a
        // waiver keyed on a full type path is for. `PreparedCharacterRegistry`
        // was `ambition_platformer2d_actor_monolith::character_runtime::definition`
        // until the P1.7 move put the model and preparation into
        // `ambition_characters::prepared`. The resource and its reason are
        // unchanged; only its address is.
        "::prepared::PreparedCharacterRegistry",
        "prepared authored definitions; immutable within a session and bound by PreparedContentIdentity",
    ),
    (
        // ⚠ ALSO REPATHED, and by the same crate move as the entry above — the
        // staging lifecycle followed the fold down to `ambition_characters` so
        // the fold could stop being public (GPT 5.6 review, priority 2). The
        // resource and its reason are unchanged; only its address is.
        "::prepared::StagedCharacterOverrides",
        "preparation-private staging input, resolved before the session's first simulated frame",
    ),
    // The ROSTER, not the activation. It is authored by whoever entered the
    // route, before the match exists, and seating only reads it — so a rewind
    // inside a match cannot move it. `ActiveMatch`, which IS written from inside
    // the sim schedule, is registered rather than waived.
    (
        "::character_runtime::staging::MatchParticipantRoster",
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
        "::character_runtime::prepared_match::PreparedMatch",
        "the resolved match DECISION, made once before the fighters exist and \
         never written from inside the sim. Rewinding it would remove what \
         activation replays FROM",
    ),
    (
        "::character_runtime::prepared_match::MatchPreparationProblems",
        "the refusal that answers an unpreparable roster; published beside the \
         plan, on the same pre-session decision",
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
    ("ambition_boss_encounter::catalog::", "authored boss catalog"),
    (
        "ambition_boss_encounter::registry::BossEncounterRegistry",
        "authored encounter registry",
    ),
    (
        "::features::banter::CombatBanterRegistry",
        "authored banter registry",
    ),
    // ⛔ **`CharacterRoster` and `CharacterRosterRegistry` WERE WAIVED HERE and
    // the types are DELETED** (AC6.1). A waiver answers a checker's question
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
    // ⛔ **this waiver was WRONG until 2026-08-15 and said only *"authored gate
    // portals"*.** The resource carried each portal's live `GatePortalPhase`
    // alongside the authored switch id and sprite names, and
    // `tick_portal_phases_system` advanced that phase by `WorldTime::scaled_dt`
    // in the sim schedule — `GgrsSchedule` on the shipped host. So a sentence
    // that was true about three `String`s was answering for an f32 timer nobody
    // rewound, and the string-keyed `HashMap` it lived in is invisible to the
    // entity-scoped sweeps, which is why no instrument said so.
    //
    // The phase now lives in `GatePortalPhases`, registered as
    // `resource.gate_portal_phases`. What is left here really is authored: it is
    // written once by the content plugin that authors a portal and never again.
    // ⚠ and it must NOT be registered — that plugin runs in `Update` behind a
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
        // ⚠ path moved 2026-08-21 (D33): the type left
        // `actor_monolith::time::feel` for `ambition_combat::feel`, the crate
        // that owns every rule it modifies. The WAIVER's reason is unchanged
        // because the decision is unchanged — feel tuning is still a
        // forward-only knob, not per-frame state. Only its address moved.
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
    // ⛔ `::cut_rope::PendingCutRopeRoomReplay` USED TO BE WAIVED HERE as a
    // "dialog-flow latch … presentation-gated", and the waiver was answering the
    // wrong question. It is a latch that BRIDGES TICKS: the choice is made while
    // the last line is on screen and the reset fires whenever the player
    // dismisses it. It is written and cleared by the simulation now and gated on
    // the conversation authority rather than on `DialogState`, so it is
    // registered rather than excused.
    // ── The SHIPPED composition's categories (2026-08-03) ────────────────────
    //
    // The sandbox sweep never saw these: it boots `Platformer2dSimHarness`, and
    // these live in the app and in provider compositions. The shipped-composition
    // sweep beside it found 64 unaccounted, and the first pass through them found
    // two REAL bugs (`BrokenBricks`, `SpentMonitors`, both registered now). What
    // is left sorts into a small number of categories, and these are them.
    //
    // ⚠ **each is a MODULE family, which is the widest form this file allows and
    // the one that most easily rots.** The test is whether a sim-authoritative
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
    (
        "ambition_app::app::world_flow::room_transition_presentation::",
        "the transition's own curtain/telemetry: what the player is shown WHILE a          room swaps, not what the room becomes",
    ),
    (
        "ambition_app::app::world_flow::room_transition_assets::",
        "which assets are staged and how far the prefetch got — a readiness          question about loading, never about the simulated world",
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
        "ambition_platformer2d_runtime::RollbackConfirmationState",
        "current rollback-driver confirmation health. It is host authority ABOUT whether speculative work may be promoted, updated by session lifecycle/mismatch handling outside the rewound world; rewinding the authority doing the rewind would be backwards",
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
        "ambition_characters::brain::SeatRawFrames",
        "the DEVICE side of the input boundary, one row per seat, holding what the          local device PROPOSED this frame before any shaping stage has run. It is          rewritten from scratch every frame by the producer and consumed by the          commit, so a rewind has nothing to put back — and restoring it would feed          a resimulation a stale proposal in place of the confirmed input it is          replaying. Same argument as `SlotControlLatches` below; the two are one          model, and D175 added this half so every seat has somewhere for a gesture,          a portal warp or a scripted substitution to happen",
    ),
    (
        "ambition_characters::brain::SlotControlLatches",
        "the DEVICE side of the input boundary, for EVERY seat including zero: it          folds device samples between ticks and drains on the tick clock. A rollback resimulates from STORED          INPUTS and never by re-reading a latch, so this is input TO the rollback          rather than state inside it — restoring it would feed the resimulation a          second copy of what it is already replaying",
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
    // ⚠ the matching entry in `rollback_exit_oracle`'s `NOT_REWOUND` argues the
    // stale-CURSOR half (what a reader resumes from). This one argues the
    // stale-BUFFER half (what the resource holds). Same subject, two instruments,
    // and each states its own question rather than pointing at the other.
    (
        "ambition_platformer2d_shared_tangle::block_nudge::",
        "a struck block's flinch is a drawn offset: one render-plugin reader on          the wall clock, writing only presentation components, over geometry          that is authoritative and static by design",
    ),
    // ⭐ **AUTHORED CONTENT, written once and never by a system.** The game's
    // fighter difficulty rungs, lowered from the compiled content pack and
    // inserted at plugin build. No system mutates it; there is no tick at which
    // its value differs from the tick before, so there is nothing for a rewind to
    // restore. What a rewind DOES restore is the brains built from it, and those
    // are ordinary rollback state.
    //
    // ⚠ the question this answers is not "is it important" — it is very
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

/// **The same sweep, over the composition that actually ships.**
///
/// ⛔ **the sweep above boots `Platformer2dSimHarness` — the Ambition sandbox —
/// so a resource that only exists in a DEMO PROVIDER's composition is invisible
/// to it.** `SpentPowerBlocks` (Mary-O's spent ?-blocks) lived unregistered for
/// as long as both existed, and no sweep could have said so: the world it walks
/// never had Mary-O's plugin in it.
///
/// `build_visible_app` composes every provider — Ambition, Sanic, Mary-O,
/// Pocket, Smash — so booting it needs no new fixture and sees what a player's
/// process sees. `NoWindow` keeps it headless (and, since 2026-08-03, writes its
/// own state directory rather than the user's).
///
/// ⚠ this is the SECOND of the two blind spots B3b names; the first — transient
/// entities spawned and despawned inside a route — is covered by
/// `rollback_exit_oracle`'s per-frame census, which caught two regressions the
/// day it was pointed at them.
///
/// ⛔ **AND IT HAS A BLIND SPOT OF ITS OWN, MEASURED 2026-08-03: this fixture
/// never runs the simulation.** `build_visible_app` seats no session, and in
/// rollback mode there deliberately is none until one is, so `GgrsSchedule` never
/// advances a frame — `the_shipped_fixture_does_not_advance_the_simulation`
/// prints the witness (0 of 255 snapshot stores written across 30 updates).
///
/// So this sweep enumerates the world **as COMPOSED, not as PLAYED**, and any
/// resource the running simulation creates is invisible to it. That is the same
/// shape as the `ActiveMatch` bug, which was invisible until the sandbox sweep
/// seated a match. A green result here means *composed clean* — it is not, and
/// must not be cited as, a statement about a session in progress.
#[test]
fn every_mutable_ambition_resource_in_the_shipped_composition_is_accounted() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // A few frames so lazily-inserted runtime resources exist, exactly as the
    // sandbox sweep steps its harness first.
    for _ in 0..8 {
        app.update();
    }

    // ⚠ **message channels are somebody else's job.**
    // `Messages<T>` is a resource, so a naive sweep counts ~240 of them here —
    // and `rollback_exit_oracle::every_gameplay_message_channel_is_rewound_on_rollback_or_named`
    // already owns that question, with its own named list and its own reasoning
    // about stale reader cursors. Two instruments claiming one population is how
    // a waiver in one gets read as coverage by the other.
    // ...and so is the rollback ENGINE's own storage. `bevy_ggrs`'s snapshot
    // stores are generic over our types, so `ComponentSnapshots<ambition_…>`
    // matches a substring search for `ambition_` while being the machinery doing
    // the rewinding rather than state to be rewound. Same for the two bevy render
    // resources that mention our types. Keeping only names that START with
    // `ambition_` is the population this sweep is actually about: 308 → 66.
    let unaccounted: Vec<String> = unaccounted_resources(app.world())
        .into_iter()
        .filter(|name| !name.contains("::Messages<"))
        .filter(|name| name.starts_with("ambition_"))
        .collect();

    // ⚠ **A RATCHET, not a pass/fail — and the number is a debt, not a target.**
    //
    // The shipped composition has 66 resources this sweep cannot account for.
    // Classifying them is real work and most are presentation, dev tooling or
    // host bookkeeping that the sandbox sweep's own WAIVED list already justifies
    // by category — but "most" is not "all", and this file's header is explicit:
    // *do not waive to get green, a wrong choice here is a desync later.* Waiving
    // 66 in one pass at the speed they were discovered is exactly that mistake.
    //
    // So the sweep lands as a ceiling. It cannot go UP — a new unaccounted
    // resource in any provider fails here immediately, which is the property the
    // blind spot never had — and every classification lowers it. When it reaches
    // the point where the remainder is a short justified list, this becomes the
    // plain assertion its sibling above already is.
    // 66 → 64 the same day it was set, by registering `BrokenBricks` and
    // `SpentMonitors`. Both halves of the ratchet were exercised doing it: the
    // sweep FOUND them, and the staleness assert REFUSED to let the ceiling stay
    // at 66 once they were gone.
    //
    // 64 → 25 by classifying eight CATEGORIES into `RESOURCE_WAIVED` — menus,
    // developer instruments, transition presentation, asset staging, content
    // presentation, the presentation-profile stack, persistence paths, and the
    // rollback driver's own state. Each is a module family whose NAME is the
    // argument. The staleness assert fired again on the way (`only 25 unaccounted
    // now`), which is the second time in one day it has stopped a ratchet from
    // quietly going slack.
    //
    // ⚠ **the 25 that remain are the ones a category could not honestly cover**,
    // and they are where the next real bug will be. Do not reach for a wider
    // waiver to finish the job — the two bugs this sweep has already caught were
    // both in a demo provider's own namespace, exactly the kind of place a broad
    // family waiver would have swallowed.
    //
    // 25 → 24 by REGISTERING `ActiveRoundScope`, the third real defect this sweep
    // has caught: a round-id allocator mutated inside the sim schedule and never
    // rewound. The staleness assert caught the stale ceiling a THIRD time in one
    // day.
    //
    // 24 → 23 by DECLARING `FallingSandProjectionReport` derived — the same
    // shape as `ActiveRoundScope` (mutated by a system in the sim schedule) but
    // wholly overwritten each tick rather than accumulated, which is the whole
    // difference between derived state and a memo.
    //
    // ⭐ **1 → 0 on 2026-08-04, and the last one was a real bug, not a
    // reclassification.** `SaveRestored` (then `InventoryRestored`) was the item the ceiling held. The
    // queue row said it should be READ once more rather than swept, and reading
    // it found the asymmetry: `OwnedItems`, `BodyWallet` and `AmbitionGameSave`
    // are all in the rollback schema and the latch that says "the save has been
    // applied to them" was not. A rewind therefore undid the restore and kept
    // the record of it, after which the write-back — gated on that latch being
    // TRUE — put the STARTER inventory over the loaded save.
    //
    // ⚠ **zero is the number this can now hold, and it is the number that will
    // hurt.** Every future resource that is mutated in the sim schedule and not
    // registered lands here with nothing to hide behind. That is the point; the
    // staleness assert below is what made lowering it safe, and it fired on this
    // change exactly as designed.
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
    // ⚠ and it must be able to go DOWN without anybody noticing by accident:
    // a classification pass that lowers the real number and forgets the ceiling
    // leaves a check that has stopped constraining anything.
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
    // ...and then SEAT A MATCH, because a resource that only exists while a
    // match is live is invisible to a sweep of a world with no match in it.
    // `ActiveMatch` is exactly that resource, and this sweep reported green
    // over it for as long as both existed (AA2 / AC2).
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
///
/// `fixed_tick`, not merely a fixed FRAME dt: it is what puts the sim in
/// `FixedUpdate`. Without it the sim hosts on the render frame and there is no
/// such thing as a render-only frame to probe — the first draft of this made
/// exactly that mistake and reported twenty resources, every one of them a sim
/// write.
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

/// **No render frame writes state the simulation owns.** (queue N4)
///
/// The two sweeps above ask *"is this state registered?"*. This asks the
/// question they cannot: *"is registered state being written from the wrong
/// schedule?"* Both answers can be yes at once, and that combination is what
/// shipped — `VersusMatch` was properly rollback-registered AND advanced by a
/// system in `Update` counting on the render clock (GPT 5.6, 2026-07-27).
///
/// That is a subtle desync rather than a loud one. Resimulation replays sim
/// steps; it does not replay render frames with their original durations. So
/// the restored value depends on presentation history the rewind does not have,
/// and the two peers disagree about a scoreboard neither of them wrote wrongly.
/// Nothing in the registry, the type system, or the other two sweeps notices —
/// registering the resource is exactly what makes it look correct.
///
/// The instrument is a frame in which the SIM CANNOT RUN. Under this host the
/// sim schedule is `FixedUpdate`, so a zero-length frame leaves the `Time<Fixed>`
/// accumulator empty and the whole simulation is skipped while `Update`,
/// `PostUpdate` and the rest run normally. Anything rollback-registered that
/// changes during such a frame was written by something that is not the
/// simulation.
///
/// ## What this covers, and what its sibling covers
///
/// It watches the 29 restored resources the RL-sim composition installs — the
/// engine and its content. It does not see resources that only the shell app
/// registers, and `VersusMatch` is one of them.
///
/// ⚠ This paragraph used to end "the shipped host runs its sim on the render
/// frame, so there is no such thing as a render-only frame there to probe", and
/// that was WRONG — reasoned rather than checked, hours after a whole session
/// spent on exactly that mistake. `build_visible_app` sets
/// `SimulationHost::Rollback` under `dev_tools`, so the shell app's sim lives in
/// `GgrsSchedule` and stopping the fixed-step clock leaves `Update` running over
/// a still simulation, same as here. `versus_stage::
/// no_render_only_frame_of_the_shipped_host_writes_rollback_state` is that
/// sweep, RED-verified against the shape the bug actually shipped in.
///
/// Only `Derived` declarations are excluded, and deliberately: a derived
/// resource is one republished every frame before anyone reads it, so writing it
/// off the sim schedule is exactly its job. `ControlFrame` — the input the
/// harness hands in per frame — is the honest example, and it is the one entry
/// the first draft flagged.
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

// ── An inert registration is worse than a missing one ────────────────────────
//
// API finding (a), made measurable. `rollback_component_canonical::<T>` installs
// a `ComponentSnapshotPlugin`, and bevy_ggrs' snapshot plugins act ONLY on
// entities carrying `bevy_ggrs::Rollback`. That marker is installed by
// `require_rollback::<A>` for some ANCHOR component `A`. So a component that is
// registered canonical, but only ever lives on entities that carry no anchor,
// is registered and inert: the registry lists it, `encoded_types()` counts it,
// the sweep above says the component is "accounted", and nothing rewinds it.
//
// That is strictly worse than forgetting to register it, because every
// instrument reports success. It is the same shape as the module-family waiver
// that swallowed `ActiveMatch` and the single-file collector that read one of
// two codec files: green, about less than it claimed.
//
// The check cannot be static — which entities carry which components is a
// runtime fact — so it runs over the SAME real populations the sweeps above
// build, and asks the co-occurrence question directly. It deliberately does not
// look for `bevy_ggrs::Rollback` itself: that marker is absent under the
// fixed-tick fixtures these sweeps use, so a check written against it would be
// vacuous exactly where it runs.

/// **Archetypes deliberately OUTSIDE the rollback envelope**, keyed by a
/// component whose presence identifies them, with the reason.
///
/// A waiver here is a CLAIM: *this entity's registered components never change
/// during simulation, so nothing is lost by not restoring them.* That is a much
/// narrower statement than "this entity does not matter", and it is the only one
/// that makes an unanchored registration harmless rather than silently broken.
///
/// ⚠ each of these earns its place by being IMMUTABLE-after-construction, not by
/// being unimportant. `SpawnOrigin`, `TransactionId` and `SimId` are construction
/// provenance (ADR 0030) — written once when the entity is built and never
/// again — so a rewind that does not restore them restores the same values they
/// already hold. A prop that started MOVING would fall out of this justification
/// immediately, which is why the waiver names a marker rather than a module.
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

/// **Construction provenance is written ONCE and never again** (ADR 0030), so an
/// entity whose only snapshot-registered components are provenance is outside
/// the envelope by construction rather than by exception: a rewind that does not
/// restore them restores exactly the values they already hold.
///
/// A RULE rather than a per-archetype waiver, deliberately. Every room prop, every
/// authored fixture, every future one lands here without anybody adding a line —
/// and the moment a prop gains a component that CHANGES, its stranded set stops
/// being a subset of this and the sweep speaks up. A waiver list would have grown
/// one entry per prop and gone unread by the third.
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
        // ⛔ **the value used to be `names.intersection(&anchors)`, which is
        // PROVABLY EMPTY here** — the loop `continue`s a few lines up whenever
        // that intersection is non-empty, so every archetype reported an empty
        // set beside it. The failure named a shape and could never name a thing.
        //
        // ⭐ the entity's NAME is what an investigation actually needs, and
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

/// **The boot world's snapshot registrations all actually apply.**
#[test]
fn no_snapshot_registration_is_inert_in_the_boot_world() {
    let mut sim = Platformer2dSimHarness::new().expect("sandbox sim boots");
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }
    assert_no_inert_registrations(&mut sim, "the boot world");
}

/// **A live match's are too** — the population that produced this defect class
/// twice (`MatchSeat` and friends were registered canonical while nothing
/// proved the bodies were anchored).
#[test]
fn no_snapshot_registration_is_inert_in_a_live_match() {
    // Fixed-tick, like its sibling sweep: `seat_a_two_cpu_match` drives the
    // seating retry to completion and the default timestep does not reach it.
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    // ⚠ the helper returns the TICK the match activated, not a seat count — and
    // with the S2 transaction that tick is 0, because every seat now resolves
    // and commits together. Count the bodies, like the sibling sweep does.
    seat_a_two_cpu_match(&mut sim);
    let seated = {
        let world = sim.world_mut();
        let mut q = world.query::<&ambition_platformer2d::actors::character_runtime::MatchSeat>();
        q.iter(world).count()
    };
    assert_eq!(
        seated, 2,
        "the match fixture seated {seated} fighters, so this check swept a world \
         with no match in it"
    );
    assert_no_inert_registrations(&mut sim, "a live match");
}

/// **The instrument itself goes red**, which is the only reason to trust the two
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
        .spawn(ambition_platformer2d::actors::character_runtime::MatchSeat(
            0,
        ));

    let inert = inert_registrations(&mut sim);
    assert!(
        inert.keys().any(|key| key.contains("MatchSeat")),
        "the sweep did not notice a snapshot-registered component on an \
         unanchored entity, so its green result above proves nothing: {inert:#?}"
    );
}

/// **The shipped sweep, AS PLAYED** — B9's blind spot, closed.
///
/// `every_mutable_ambition_resource_in_the_shipped_composition_is_accounted`
/// boots a world that never simulates: `build_visible_app` seats no session, so
/// `GgrsSchedule` never advances and **any resource the running simulation
/// creates is structurally invisible to it.** That is the same shape as the
/// `ActiveMatch` bug, which hid until the sandbox sweep seated a match.
///
/// This drives the real path — `ShellCommand::GoTo("ambition_gameplay")`, which
/// activates the route headlessly and starts the session — and then asks what
/// EXISTS that did not before. Measured: the route activates, a `PrimaryPlayer`
/// spawns, and `GgrsSchedule` runs once per update.
///
/// ⚠ **a DIFFERENCE ratchet, not a count.** The absolute unaccounted number is
/// the other sweep's job and its ceiling; what only this fixture can see is the
/// set that appears *because the world played*. Two are known and read clean:
///
/// | `ConfirmedFrameBoundary` | *"published once per simulated frame by the rollback bridge, from the GGRS session's own frame counters"* — re-derived every frame, so a rewind has nothing to put back. The same category as `PendingSeatInputs` and `RollbackExecutionStats`, which the shipped ceiling already waives as the machinery doing the rewinding. |
/// | `bevy_ggrs::Session<…>` | the session itself. |
///
/// ⛔ **anything else appearing here is the interesting case** and is why this is
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
    for _ in 0..240 {
        app.update();
    }

    // ⛔ **the fixture has to prove it PLAYED**, or this test passes by measuring
    // the same halted world twice and says nothing at all — which is exactly the
    // failure the composed sweep has and this one exists to fix.
    let players = app
        .world_mut()
        .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::actors::actor::PrimaryPlayer>>()
        .iter(app.world())
        .count();
    assert_eq!(
        players, 1,
        "the gameplay route did not produce a player, so nothing below was measured \
         against a world that played"
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
