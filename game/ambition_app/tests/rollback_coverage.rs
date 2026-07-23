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
];

fn waiver(type_name: &str) -> Option<&'static str> {
    WAIVED
        .iter()
        .find(|(needle, _)| type_name.contains(needle))
        .map(|(_, reason)| *reason)
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

    let known: BTreeSet<String> = sim
        .world()
        .get_resource::<ambition::runtime::rollback::RollbackRegistry>()
        .expect("rollback registry is installed by the engine plugins")
        .descriptors()
        .map(|d| d.type_name.clone())
        .collect();

    // The simulated population: every body/feature/projectile/encounter the sim
    // owns is tagged as a sim entity, which is precisely the set a rollback must
    // reproduce exactly.
    let sim_entities: Vec<Entity> = {
        let world = sim.world_mut();
        let mut q = world
            .query_filtered::<Entity, With<ambition::platformer::lifecycle::FeatureSimEntity>>();
        q.iter(world).collect()
    };
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

    if !unaccounted.is_empty() {
        let mut report = String::from(
            "Components live on simulated entities that GGRS will not rewind.\n\
             Each is a rollback desync waiting to happen. For each one: register it\n\
             in `register_engine_rollback_state`, declare it derived, or add a\n\
             justified waiver to WAIVED in this file.\n\n",
        );
        for (type_name, count) in &unaccounted {
            report.push_str(&format!("  {type_name}  (on {count} sim entities)\n"));
        }
        panic!("{report}");
    }
}

/// Resource type-name substrings that are NOT authoritative simulation state.
///
/// Same contract as [`WAIVED`]: each entry claims rewinding the named resource
/// would be meaningless or harmful, with the reason. Crate-prefix waivers from
/// [`WAIVED`] apply here too; this list holds the resource-specific remainder.
const RESOURCE_WAIVED: &[(&str, &str)] = &[
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
    (
        "::features::enemies::CharacterRoster",
        "authored roster (and its registry)",
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
        "::session::data::SandboxData",
        "authored data assets (spec, asset handle, and Assets store)",
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
    ("::intro::plugin::Intro", "install-once content markers"),
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
        if known.contains(name)
            || waiver(name).is_some()
            || RESOURCE_WAIVED
                .iter()
                .any(|(needle, _)| name.contains(needle))
        {
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
