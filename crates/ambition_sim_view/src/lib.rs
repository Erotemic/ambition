//! Plain-data observation boundary over simulation state.
//!
//! Simulation read-models are rebuilt from authoritative state in the sim tail;
//! observers consume these snapshots instead of querying live simulation ECS.
//! [`camera_snapshot`] is presentation-clock state and is rebuilt once per
//! rendered frame, but follows the same one-way observation boundary.

use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
pub mod affordances;
mod anim_index;
mod attack_vfx_view;
pub mod camera_snapshot;
mod combat_geometry_view;
mod control_prompt;
mod defense_view;
mod dialog_view;
mod facts;
pub mod local_view;
mod pose_view;
pub mod presented_pose;
mod rollback_registration;
mod view_index;

pub use anim_index::{
    rebuild_actor_anim_index, rebuild_boss_frame_index, ActorAnimFrame, ActorAnimIndex,
    ActorSpriteData, BossFrameIndex, BossFrameView, ClipRequest, HazardLaneFact, SMASH_CHARGE_CLIP,
};
pub use combat_geometry_view::{
    rebuild_combat_geometry_view, CombatBodyGeometryView, CombatGeometryView,
    CombatStrikeGeometryView, HurtboxSource,
};
pub use control_prompt::{
    project_prompt_readiness, publish_frontend_context_prompt, rebuild_control_prompt,
    ControlContextKind, ControlPrompt, ControlPromptRebuilt, PromptEntry, PromptNaming,
};
// Re-exported so `ControlPrompt` consumers (the touch overlay) can name the
// slot vocabulary without a direct `entity_catalog` dep.
pub use ambition_entity_catalog::action_scheme::{ControlSlot, VisualId};
pub use attack_vfx_view::{rebuild_attack_vfx_views, AttackVfxView};
pub use camera_snapshot::{local_view_facts, CameraViewState, PresentedViewState};
pub use defense_view::{defense_cue_causes, DefenseCueCauses};
pub use dialog_view::{rebuild_dialog_view, DialogView};
pub use facts::*;
pub use local_view::{
    compose_local_views, resolve_view_subjects, spawn_local_view, the_only_view, BoundLocalView,
    LocalView, LocalViewId, PresentedForView, PresentsView, ResolvedViewSubject, ViewParticipant,
    ViewPlacement, ViewSubject, ViewsOnHand,
};
pub use pose_view::{
    rebuild_body_pose_views, rebuild_guard_breaks_view, rebuild_launched_bodies_view,
    rebuild_shield_rings_view, BodyPoseView, GuardBreakFact, GuardBreaksView, LaunchedBodiesView,
    LaunchedBodyFact, ShieldRingFact, ShieldRingsView,
};
pub use presented_pose::{
    PresentationPhase, PresentedFeaturePoses, PresentedPose, PresentedPosePlugin, PresentedPoseSet,
    PresentedPoseStage,
};
pub use view_index::{
    rebuild_actor_render_index, rebuild_boss_render_index, rebuild_feature_view_index,
    rebuild_nameplate_index, ActorRenderIndex, ActorRenderView, BossRenderIndex, BossRenderView,
    FeatureView, FeatureViewIndex, NameplateFact, NameplateIndex,
};

/// Rebuilds the observation read-models once per frame, sim-side:
/// [`FeatureViewIndex`] (geometry/state for every feature),
/// [`ActorRenderIndex`] / [`BossRenderIndex`] (materialized identity facts),
/// [`NameplateIndex`], [`BossFrameIndex`], the per-actor POSE snapshot
/// ([`ActorAnimIndex`]: overlay advance + anim pick), the player-bodied
/// [`BodyPoseView`] components, [`ShieldRingsView`] and
/// [`LaunchedBodiesView`]. All let observers
/// read a snapshot instead of live-querying the sim's ECS.
pub struct FeatureViewSyncSchedulePlugin;

impl bevy::prelude::Plugin for FeatureViewSyncSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let sim = app.sim_schedule();
        use bevy::prelude::IntoScheduleConfigs;
        // Owned here (anti-god rule 5): the plugin that rebuilds the index
        // initializes it; consumers only read.
        app.init_resource::<ActorAnimIndex>();
        app.init_resource::<ShieldRingsView>();
        app.init_resource::<LaunchedBodiesView>();
        app.init_resource::<GuardBreaksView>();
        app.init_resource::<BossFrameIndex>();
        app.init_resource::<NameplateIndex>();
        app.init_resource::<DialogView>();
        app.init_resource::<ControlPrompt>();
        app.init_resource::<CombatGeometryView>();
        // The frontend half of the prompt: while a startup/launcher context
        // owns the participant's actions, the owning surface's cue labels the
        // confirm control (the sim-side rebuild yields on those frames).
        // Frame clock, between cue publication and the input consumers.
        app.add_systems(
            bevy::prelude::Update,
            publish_frontend_context_prompt
                .after(ambition_input::InputSet::PublishCues)
                .before(ambition_input::InputSet::Consume),
        );
        app.add_systems(
            sim,
            (
                // The nameplate rows prefer the feature view's geometry, so
                // they rebuild strictly after it (same-frame read).
                (rebuild_feature_view_index, rebuild_nameplate_index).chain(),
                rebuild_actor_render_index,
                rebuild_boss_render_index,
                rebuild_boss_frame_index,
                // Overlay clocks advance right before their one reader
                // rebuilds the pose snapshot (§A9 ordering, preserved). The
                // overlay ADVANCE mutates sim components, so it stays defined
                // in the sim heart; this plugin only schedules it.
                (
                    ambition_platformer2d_actor_monolith::features::advance_actor_anim_overlays,
                    rebuild_actor_anim_index,
                )
                    .chain(),
                // Player-bodied pose components + the pooled shield-ring rows —
                // the per-body half of the pose read-model (E4 slices 1–4).
                rebuild_body_pose_views,
                rebuild_shield_rings_view,
                // Which bodies are in an INVOLUNTARY flight, so a
                // flight-readability cue never has to infer that from speed.
                rebuild_launched_bodies_view,
                // And which have had their guard shattered — a raised guard and
                // a broken one are two different rows, not one with a flag.
                rebuild_guard_breaks_view,
                // Which bodies' characters author their own attack art, so no
                // render system has to ask the catalog — see the module docs
                // for why an absent catalog must stay ABSENT here.
                rebuild_attack_vfx_views,
                // Exact combat truth for debug/RL/tool observers: effective body
                // hurtboxes plus live world-space strike volumes, independent of
                // which controller (if any) drives each body.
                rebuild_combat_geometry_view,
                // The dialogue overlay's row (recon C3): presentation reads
                // THIS, never the live `DialogState`.
                rebuild_dialog_view,
                // "What does each control do right now" for the controlled
                // subject — the touch overlay reads this instead of the sim.
                rebuild_control_prompt.in_set(ControlPromptRebuilt),
                // ⛔ AFTER the rebuild, and OUTSIDE its cache. The rebuild skips
                // quiet frames on purpose; a fire-rate floor decays every tick,
                // so reading it in there would re-derive the scheme all the way
                // through every recharge.
                project_prompt_readiness.after(ControlPromptRebuilt),
            )
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::FeatureViewSync),
        );
    }
}

// Domain-owned rollback declaration; the host supplies the backend registrar.
pub use rollback_registration::register_rollback_state;
