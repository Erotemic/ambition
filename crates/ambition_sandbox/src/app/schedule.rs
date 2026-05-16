//! Sandbox simulation schedule: system sets and their explicit ordering.
//!
//! Single source of truth for which simulation phases exist and how they
//! relate. Add new systems to one of these sets in `plugins.rs` via
//! `.in_set(SandboxSet::…)` rather than pinning a fragile cross-system
//! `.after(other_system)`.

use bevy::prelude::*;

/// Coarse simulation ordering for sandbox gameplay systems.
///
/// Each set groups one phase of the sim tick. The chained ordering between
/// sets is configured by [`configure_sandbox_sets`]. Tail sets that run
/// *after* the main chain (reset processing, trace recording) are also
/// configured there.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum SandboxSet {
    /// LDtk polling, feature world rebuild, player loop, room-transition apply.
    CoreSimulation,
    /// Pickup collection and player heal request consumption.
    FeatureCollection,
    /// Actor/switch/chest/breakable interaction systems.
    FeatureInteraction,
    /// LDtk runtime spine index rebuild + parity check.
    LdtkRuntimeSpine,
    /// Moving platforms + encounter state + gameplay banner.
    EncounterSimulation,
    /// Auto-triggered cutscenes, cutscene drain/tick.
    Cutscene,
    /// Flag/quest/switch/boss/NPC/sfx gameplay-effect routing.
    GameplayEffects,
    /// Boss save sync, quest events, body-mode, room metadata, map sync.
    Progression,
    /// Sandbox reset request processor. Runs after CoreSimulation.
    ResetProcessing,
    /// Trace recording + dump flush. Runs after CoreSimulation.
    Trace,
}

/// Configure the chained ordering between [`SandboxSet`] variants.
///
/// The main chain runs:
/// `CoreSimulation → FeatureCollection → FeatureInteraction →
/// LdtkRuntimeSpine → EncounterSimulation → Cutscene → GameplayEffects →
/// Progression`.
///
/// `ResetProcessing` and `Trace` are tail consumers — they only need to
/// observe state after the main sim has resolved, so they're each
/// configured `.after(CoreSimulation)` without joining the chain.
pub fn configure_sandbox_sets(app: &mut App) {
    app.configure_sets(
        Update,
        (
            SandboxSet::CoreSimulation,
            SandboxSet::FeatureCollection,
            SandboxSet::FeatureInteraction,
            SandboxSet::LdtkRuntimeSpine,
            SandboxSet::EncounterSimulation,
            SandboxSet::Cutscene,
            SandboxSet::GameplayEffects,
            SandboxSet::Progression,
        )
            .chain(),
    )
    .configure_sets(
        Update,
        (
            SandboxSet::ResetProcessing.after(SandboxSet::CoreSimulation),
            SandboxSet::Trace.after(SandboxSet::CoreSimulation),
        ),
    );
}
