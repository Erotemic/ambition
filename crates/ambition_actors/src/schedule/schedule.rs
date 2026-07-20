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
use ambition_platformer_primitives::lifecycle::simulation_authorized;
use ambition_platformer_primitives::schedule::{
    GameplaySimulationRoot, SandboxSet, SimScheduleExt,
};

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
///
/// Every set here is a SIM phase, so the whole ordering is declared in the
/// app's sim schedule ([`SimSchedule`] — `Update` when frame-stepped,
/// `FixedUpdate` when fixed-tick). `PresentationVisualSync` is the one
/// presentation-side set in the list; it is configured alongside the sim so
/// that in frame-stepped mode it keeps its `.after(FeatureViewSync)` edge. In
/// fixed-tick mode the render systems that join it live in `Update` and need no
/// such edge — the read-model they consume was published by the last tick.
///
/// [`SimSchedule`]: ambition_platformer_primitives::schedule::SimSchedule
pub fn configure_sandbox_sets(app: &mut App) {
    let sim = app.sim_schedule();

    // THE session gate. Every SandboxSet variant is nested inside
    // `GameplaySimulationRoot` below, so this ONE condition puts the whole
    // gameplay simulation (tick timeline included) to sleep at frontend routes
    // in session-gated hosts, and is inert everywhere else
    // (see `simulation_authorized`).
    app.configure_sets(sim, GameplaySimulationRoot.run_if(simulation_authorized));
    app.configure_sets(
        sim,
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
            SandboxSet::FeatureViewSync,
            SandboxSet::PresentationVisualSync,
            SandboxSet::Trace,
        )
            .in_set(GameplaySimulationRoot),
    );

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
        sim,
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
        sim,
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
    .configure_sets(sim, SandboxSet::Trace.after(SandboxSet::CoreSimulation))
    // Presentation visual chain: must observe this frame's
    // FeatureViewIndex rebuild. Owning the ordering at the set level
    // means every system added to `PresentationVisualSync` inherits
    // the `.after(FeatureViewSync)` constraint without re-typing it
    // — and a test can hang a probe in the set to verify the
    // ordering survives.
    .configure_sets(
        sim,
        SandboxSet::PresentationVisualSync.after(SandboxSet::FeatureViewSync),
    );

    // Input populate contract (ambition_input::InputSet): every system that
    // WRITES the `ControlFrame` resource lives in `InputSet::Route`, and the
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
        sim,
        ambition_input::InputSet::Route.before(crate::control::populate_slot_controls),
    );
}
