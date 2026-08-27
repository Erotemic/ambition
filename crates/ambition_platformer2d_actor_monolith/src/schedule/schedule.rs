//! Sandbox simulation schedule: system sets and their explicit ordering.
//!
//! Single source of truth for the concrete sandbox app schedule.
//!
//! `ambition_platformer2d_shared_tangle::schedule::PlatformerRuntimeSet` names the
//! reusable runtime vocabulary that future crates should depend on. `Platformer2dSimulationPhaseMonolith`
//! is the app-level realization of that vocabulary, plus Ambition-specific tail
//! phases. Add new systems through module-owned plugins and stable sets rather
//! than pinning a fragile cross-system `.after(other_system)` in this file or in
//! `plugins.rs`.

use bevy::prelude::*;

// Canonical schedule labels live in the lower platformer-primitives crate so
// runtime, host, content, sim-view, and render can order systems without
// depending on `ambition_platformer2d_actor_monolith`. This module keeps only the concrete ordering
// function because it still refers to actor-system anchors.
use ambition_platformer2d_shared_tangle::lifecycle::simulation_authorized;
use ambition_platformer2d_shared_tangle::schedule::{
    CombatSet, GameplaySimulationRoot, Platformer2dSimulationPhaseMonolith, PlayerInputSet,
    PlayerSimulationSet, RoomTransitionSet, SimScheduleExt, WorldPrepSet,
};

/// Configure the chained ordering between [`Platformer2dSimulationPhaseMonolith`] variants.
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
/// `PresentationVisualSync` is the one presentation-side set in the list; it is configured
/// alongside the sim so that in frame-stepped mode it keeps its `.after(FeatureViewSync)` edge.
///
/// [`SimSchedule`]: ambition_platformer2d_shared_tangle::schedule::SimSchedule
pub fn configure_platformer2d_simulation_phases(app: &mut App) {
    let sim = app.sim_schedule();

    // THE session gate. Every Platformer2dSimulationPhaseMonolith variant is nested inside
    // `GameplaySimulationRoot` below, so this ONE condition puts the whole
    // gameplay simulation (tick timeline included) to sleep at frontend routes
    // in session-gated hosts, and is inert everywhere else
    // (see `simulation_authorized`).
    app.configure_sets(sim, GameplaySimulationRoot.run_if(simulation_authorized));
    app.configure_sets(
        sim,
        (
            Platformer2dSimulationPhaseMonolith::CoreSimulation,
            Platformer2dSimulationPhaseMonolith::FeatureCollection,
            Platformer2dSimulationPhaseMonolith::FeatureInteraction,
            Platformer2dSimulationPhaseMonolith::LdtkRuntimeSpine,
            Platformer2dSimulationPhaseMonolith::EncounterSimulation,
            Platformer2dSimulationPhaseMonolith::Cutscene,
            Platformer2dSimulationPhaseMonolith::GameplayEffects,
            Platformer2dSimulationPhaseMonolith::Progression,
            Platformer2dSimulationPhaseMonolith::ResetProcessing,
            Platformer2dSimulationPhaseMonolith::FeatureViewSync,
            Platformer2dSimulationPhaseMonolith::PresentationVisualSync,
            Platformer2dSimulationPhaseMonolith::Trace,
        )
            .in_set(GameplaySimulationRoot),
    );

    // Sub-sets inside CoreSimulation, ordered.
    //
    // CONTROL-SEAM ORDERING: `PlayerInput` runs BEFORE `WorldPrep`. This is the slot-input
    // invariant — `PlayerInput` finalizes this frame's device input, publishes it into
    // `SlotControls`, and resolves `ControlledSubject`; only THEN does `WorldPrep` tick the
    // actor/boss brains (`update_ecs_actors` / `tick_boss_brains_system`). So a possessed body
    // holding a seat reads THIS frame's input, not last frame's.
    app.configure_sets(
        sim,
        (
            Platformer2dSimulationPhaseMonolith::PlayerInput,
            Platformer2dSimulationPhaseMonolith::WorldPrep,
            Platformer2dSimulationPhaseMonolith::PlayerSimulation,
            Platformer2dSimulationPhaseMonolith::RoomTransition,
            Platformer2dSimulationPhaseMonolith::Combat,
            Platformer2dSimulationPhaseMonolith::PresentationSync,
        )
            .chain()
            .in_set(Platformer2dSimulationPhaseMonolith::CoreSimulation),
    );

    // The phases INSIDE PlayerInput. Naming them changed no order — this is the
    // chain `register_player_input_systems` already had — but it turns "run after
    // the persona is built" from a leaf-system reference into a statement about a
    // phase. See `PlayerInputSet` for why that distinction cost a schedule cycle.
    app.configure_sets(
        sim,
        (
            PlayerInputSet::Device,
            PlayerInputSet::CharacterProjection,
            PlayerInputSet::Persona,
            PlayerInputSet::Brain,
        )
            .chain()
            .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput),
    );
    // ⛔⛔ `ControlGate` AND `BodyMode` ARE NOT IN THIS PHASE, and they cannot
    // be. Both act on FINISHED control, and control is finished for a possessed
    // body here (`Brain`) but not for an autonomous one until
    // `ActorDecisionSet::Publish`, a whole phase later. A gate that ran here ran
    // BEFORE half the frames it exists to gate — which is exactly why every
    // restriction over control was registered TWICE (D202).
    //
    // ⇒ they are placed by `configure_actor_decision_phases`, in `WorldPrep`,
    // between the second publication and integration. The enum's own docs say so.

    app.configure_sets(
        sim,
        (
            RoomTransitionSet::Detect,
            RoomTransitionSet::Apply,
            RoomTransitionSet::Reset,
        )
            .chain()
            .in_set(Platformer2dSimulationPhaseMonolith::RoomTransition),
    );

    // The movement anchor inside WorldPrep. Three placement sets around the one
    // movement system, NOT a decomposition of the chain — see `WorldPrepSet` for
    // why that restraint is deliberate.
    app.configure_sets(
        sim,
        (
            WorldPrepSet::BeforeIntegrate,
            WorldPrepSet::Integrate,
            WorldPrepSet::AfterIntegrate,
        )
            .chain()
            .in_set(Platformer2dSimulationPhaseMonolith::WorldPrep),
    );
    // A LABEL, not a chain position: see `WorldPrepSet::ContactDamage` for why
    // chaining it would add edges nobody chose.
    //
    // ⭐ ONE chosen edge, though, and it is the one contact damage actually
    // needs: it reads poses, so it runs after everything that writes them —
    // integration and the external constraints that follow it, the captive hold
    // among them. Stated at the SET.
    //
    // ⛔ NOT `.after(<one pose writer>)`. The dependency is on POSES BEING
    // SETTLED — a property of the whole phase — and naming one contributor to it
    // goes stale the moment another joins.
    app.configure_sets(
        sim,
        WorldPrepSet::ContactDamage
            .after(WorldPrepSet::AfterIntegrate)
            .in_set(Platformer2dSimulationPhaseMonolith::WorldPrep),
    );

    // The phases INSIDE PlayerSimulation. `PostPossession` is a HOST SLOT: the
    // engine registers nothing into it, and a host that registers nothing gets
    // the chain with the slot collapsed.
    app.configure_sets(
        sim,
        (
            PlayerSimulationSet::Possession,
            PlayerSimulationSet::PostPossession,
            PlayerSimulationSet::Outcome,
        )
            .chain()
            .in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation),
    );

    // The phases INSIDE Combat, plus the two content slots between them. Same
    // reasoning as `PlayerInputSet` above: this is the chain the combat schedule
    // already had, and naming it turns "run once damage has resolved" from a leaf
    // reference into a phase.
    app.configure_sets(
        sim,
        (
            CombatSet::Trigger,
            CombatSet::Playback,
            CombatSet::Materialize,
            CombatSet::Resolve,
            CombatSet::ContentFlavor,
            CombatSet::Settle,
        )
            .chain()
            .in_set(Platformer2dSimulationPhaseMonolith::Combat),
    );
    // `ContentSpecials` sits INSIDE `Materialize` rather than between phases: a
    // boss special dispatched this frame must reach its content technique this
    // frame, and the effect executors that drain its output are in the same
    // phase. Nesting says that; an edge between phases would not.
    app.configure_sets(
        sim,
        CombatSet::ContentSpecials.in_set(CombatSet::Materialize),
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
            Platformer2dSimulationPhaseMonolith::CoreSimulation,
            Platformer2dSimulationPhaseMonolith::FeatureCollection,
            Platformer2dSimulationPhaseMonolith::FeatureInteraction,
            Platformer2dSimulationPhaseMonolith::LdtkRuntimeSpine,
            Platformer2dSimulationPhaseMonolith::EncounterSimulation,
            Platformer2dSimulationPhaseMonolith::Cutscene,
            Platformer2dSimulationPhaseMonolith::GameplayEffects,
            Platformer2dSimulationPhaseMonolith::Progression,
            Platformer2dSimulationPhaseMonolith::ResetProcessing,
            // FeatureViewSync is the final sim-side tail; everything
            // that mutates ECS feature state — including
            // ResetProcessing — has already run.
            Platformer2dSimulationPhaseMonolith::FeatureViewSync,
        )
            .chain(),
    )
    .configure_sets(
        sim,
        Platformer2dSimulationPhaseMonolith::Trace
            .after(Platformer2dSimulationPhaseMonolith::CoreSimulation),
    )
    // Presentation visual chain: must observe this frame's
    // FeatureViewIndex rebuild. Owning the ordering at the set level
    // means every system added to `PresentationVisualSync` inherits
    // the `.after(FeatureViewSync)` constraint without re-typing it
    // — and a test can hang a probe in the set to verify the
    // ordering survives.
    .configure_sets(
        sim,
        Platformer2dSimulationPhaseMonolith::PresentationVisualSync
            .after(Platformer2dSimulationPhaseMonolith::FeatureViewSync),
    );

    app.configure_sets(
        sim,
        ambition_input::InputSet::Route.before(crate::control::PrimarySlotInputCommit),
    );
}
