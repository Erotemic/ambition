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
//! It does not see resources — those still need review by hand.
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
