//! Sandbox simulation schedule: system sets and their explicit ordering.
//!
//! Single source of truth for the concrete sandbox app schedule.
//!
//! `crate::platformer_runtime::schedule::PlatformerRuntimeSet` names the
//! reusable runtime vocabulary that future crates should depend on. `SandboxSet`
//! is the app-level realization of that vocabulary, plus Ambition-specific tail
//! phases. Add new systems through module-owned plugins and stable sets rather
//! than pinning a fragile cross-system `.after(other_system)` in this file or in
//! `plugins.rs`.

use bevy::prelude::*;

// Canonical schedule labels live in the lower platformer-primitives crate so
// runtime, host, content, sim-view, and render can order systems without
// depending on `ambition_actors`. This module keeps only the concrete ordering
// function because it still refers to actor-system anchors.
use ambition_platformer_primitives::schedule::SandboxSet;

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
    //
    // CONTROL-SEAM ORDERING: `PlayerInput` runs BEFORE `WorldPrep`. This is the
    // slot-input invariant — `PlayerInput` finalizes this frame's device input,
    // publishes it into `SlotControls`, and resolves `ControlledSubject`; only
    // THEN does `WorldPrep` tick the actor/boss brains (`update_ecs_actors` /
    // `tick_boss_brains_system`). So a possessed body carrying `Brain::Player`
    // reads THIS frame's input, not last frame's. The `ActorActionMessage`
    // emitters were moved out of `PlayerInput` to run after `WorldPrep` (see
    // `register_player_input_systems`) so they observe both the player's and the
    // actors' freshly-ticked `ActorControl`.
    app.configure_sets(
        Update,
        (
            SandboxSet::PlayerInput,
            SandboxSet::WorldPrep,
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

    // Input populate contract (ambition_input::InputSet): every system that
    // WRITES the `ControlFrame` resource lives in `InputSet::Populate`, and the
    // whole set is pinned BEFORE the gameplay consume boundary. That boundary is
    // now `populate_slot_controls` — the FIRST reader of the finalized
    // `ControlFrame`, publishing it into the slot-based controller model
    // (`SlotControls[PRIMARY]`). `sync_local_player_input_frame` (SlotControls →
    // the controlled body's `PlayerInputFrame`) is chained after it, so it
    // inherits the ordering. This is ADDITIVE: every tagged writer already ran
    // before that consumer (device populate + touch fold run
    // `.before(CoreSimulation)`; the portal write-back and edge-derived flags run
    // earlier in the `PlayerInput` chain `.before` the consumer). Naming the
    // window makes it structurally impossible for a `ControlFrame` writer to
    // float past the consume and stamp stale input over the fresh frame — the
    // Move-axis regression this contract exists to prevent.
    app.configure_sets(
        Update,
        ambition_input::InputSet::Populate.before(crate::player::populate_slot_controls),
    );
}
