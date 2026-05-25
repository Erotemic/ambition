//! Sandbox simulation schedule: system sets and their explicit ordering.
//!
//! Single source of truth for which simulation phases exist and how they
//! relate. Add new systems to one of these sets in `plugins.rs` via
//! `.in_set(SandboxSet::…)` rather than pinning a fragile cross-system
//! `.after(other_system)`.

use bevy::prelude::*;

/// Coarse simulation ordering for sandbox gameplay systems.
///
/// The 6 sub-sets `WorldPrep` → `PlayerInput` → `PlayerSimulation` →
/// `RoomTransition` → `Combat` → `PresentationSync` are nested inside
/// `CoreSimulation`, ordered by [`configure_sandbox_sets`]. External
/// systems can still pin against `SandboxSet::CoreSimulation` (e.g.
/// `.after(CoreSimulation)`) and that constraint covers all six
/// sub-phases.
///
/// The remaining variants (`FeatureCollection`, `FeatureInteraction`,
/// …) are top-level sets that run after `CoreSimulation` in their own
/// chain. `ResetProcessing` and `Trace` are tail consumers configured
/// `.after(CoreSimulation)` without joining the main chain.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum SandboxSet {
    /// Top-level set that contains the six sub-sets below. Kept as a
    /// distinct label so existing `.before/.after(CoreSimulation)`
    /// constraints from presentation/audio/HUD systems continue to
    /// cover the full main chain after this finer-grained split.
    CoreSimulation,

    /// Pre-player-tick world prep: LDtk hot-reload polling, feature
    /// ECS world overlay rebuild, feature ticks (hazards / actors /
    /// bosses). Feeds the collision world that the player simulation
    /// consults.
    WorldPrep,
    /// Pre-player-tick input pipeline: dev-edit sync, input-driven
    /// reset, gameplay timer decay, interaction buffer update, and
    /// the suspended-time fallback.
    PlayerInput,
    /// Main player tick: `player_control_system` + `player_simulation_system`
    /// (control + simulation) plus the post-sim damage / safe-respawn
    /// resolver.
    PlayerSimulation,
    /// Room transition detection + apply + per-room feature reset.
    RoomTransition,
    /// Attack lifecycle, projectile updates, and feature damage apply.
    Combat,
    /// Player ECS write-back + presentation timer decays.
    PresentationSync,

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
    /// Sandbox reset request processor. Joined into the main post-core
    /// chain (between `Progression` and `FeatureViewSync`) because the
    /// reset path despawns every `RoomScopedEntity` (including every
    /// `RoomVisual`) and every feature sim entity, flips the active
    /// room, and re-spawns the start room's feature set via
    /// `spawn_room_feature_entities` — all mutations the cache must
    /// observe before presentation reads it.
    ResetProcessing,
    /// Rebuild the [`crate::features::FeatureViewIndex`] cache after
    /// every same-frame mutation to feature state.
    ///
    /// Runs at the tail of the post-core chain — after
    /// `FeatureCollection`, `FeatureInteraction`, `EncounterSimulation`,
    /// `GameplayEffects`, `Progression`, AND `ResetProcessing` — so
    /// the cache reflects this frame's pickup collections, chest
    /// opens, switch toggles, encounter mob spawns, reward-chest
    /// drops, save-driven actor/boss sync, and any post-reset
    /// re-spawn. Presentation systems that read the cache run in
    /// [`SandboxSet::PresentationVisualSync`], which is configured
    /// `.after(SandboxSet::FeatureViewSync)` below.
    FeatureViewSync,
    /// Presentation-side container set for visual systems that read
    /// [`crate::features::FeatureViewIndex`] (`sync_visuals`,
    /// `upgrade_enemy_sprites`, `upgrade_npc_sprites`, and the
    /// animation chain that follows them).
    ///
    /// Declared as a SandboxSet rather than left as an `.after(...)`
    /// pin on the chain itself so the ordering contract
    /// (`PresentationVisualSync.after(FeatureViewSync)`) lives in
    /// exactly one place — [`configure_sandbox_sets`] — and so tests
    /// can hang a probe on the set without re-typing the constraint.
    /// Removing the `.after` here would break the regression test
    /// `presentation_visual_sync_runs_after_feature_view_sync`.
    PresentationVisualSync,
    /// Trace recording + dump flush. Runs after CoreSimulation.
    Trace,
}

/// Configure the chained ordering between [`SandboxSet`] variants.
///
/// Within `CoreSimulation`:
/// `WorldPrep → PlayerInput → PlayerSimulation → RoomTransition →
/// Combat → PresentationSync`. The six sub-sets are nested in
/// `CoreSimulation` so `.before/.after(CoreSimulation)` covers them
/// transitively.
///
/// Top-level chain after `CoreSimulation`:
/// `FeatureCollection → FeatureInteraction → LdtkRuntimeSpine →
/// EncounterSimulation → Cutscene → GameplayEffects → Progression`.
///
/// `ResetProcessing` and `Trace` are tail consumers — they observe
/// state after the main sim has resolved, so they're each configured
/// `.after(CoreSimulation)` without joining the chain.
pub fn configure_sandbox_sets(app: &mut App) {
    // Sub-sets inside CoreSimulation, ordered.
    app.configure_sets(
        Update,
        (
            SandboxSet::WorldPrep,
            SandboxSet::PlayerInput,
            SandboxSet::PlayerSimulation,
            SandboxSet::RoomTransition,
            SandboxSet::Combat,
            SandboxSet::PresentationSync,
        )
            .chain()
            .in_set(SandboxSet::CoreSimulation),
    );

    // Top-level chain. ResetProcessing joins the main chain (rather
    // than floating off as a `.after(CoreSimulation)` tail) because
    // its work — despawn every RoomScopedEntity (every RoomVisual +
    // any future sim-only entities) plus feature sim entities, flip
    // the active room, re-spawn the start room — is exactly the kind
    // of feature-state mutation FeatureViewSync exists to observe.
    // Placing it BEFORE FeatureViewSync guarantees the cache reflects
    // the post-reset feature set on the reset frame, not one frame
    // later.
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
            SandboxSet::ResetProcessing,
            // FeatureViewSync is the final sim-side tail; everything
            // that mutates ECS feature state — including
            // ResetProcessing — has already run.
            SandboxSet::FeatureViewSync,
        )
            .chain(),
    )
    .configure_sets(Update, SandboxSet::Trace.after(SandboxSet::CoreSimulation))
    // Presentation visual chain: must observe this frame's
    // FeatureViewIndex rebuild. Owning the ordering at the set level
    // means every system added to `PresentationVisualSync` inherits
    // the `.after(FeatureViewSync)` constraint without re-typing it
    // — and a test can hang a probe in the set to verify the
    // ordering survives.
    .configure_sets(
        Update,
        SandboxSet::PresentationVisualSync.after(SandboxSet::FeatureViewSync),
    );
}
