//! **[the observation boundary]** — the `SimView` read-model (E4).
//!
//! Everything here is a plain-data snapshot of sim state, rebuilt once per
//! sim tick by extraction systems that run LAST in the sim tail
//! (`SandboxSet::FeatureViewSync`). Builders are pure functions of sim state —
//! no caching across ticks, no `Entity`/`Handle` borrows in the rows — so every
//! observer (render, RL observation, netcode confirmation, the fighter brain,
//! slower-light shaders) consumes the SAME facts.
//!
//! **[`camera_snapshot`] is the one exception, deliberately.** The simulation
//! produces authoritative world facts; where the camera *looks* at them is
//! presentation state, not a sim fact — it depends on the physical viewport,
//! the active presentation profile and video settings, and it integrates on the
//! render clock. It therefore resolves once per rendered FRAME in `Update`,
//! not once per sim tick, and no sim system reads it. It lives in this crate
//! because it is still an observation of sim state that must not reach back
//! into the sim.
//!
//! Render depends on THIS crate for sim facts; it never queries the sim
//! heart's live components (the boundary test in `ambition_render` pins
//! that).

use ambition_platformer_primitives::schedule::SimScheduleExt;
mod anim_index;
pub mod camera_snapshot;
mod control_prompt;
mod dialog_view;
mod facts;
mod pose_view;
pub mod presented_pose;
mod view_index;

pub use anim_index::{
    rebuild_actor_anim_index, rebuild_boss_frame_index, ActorAnimFrame, ActorAnimIndex,
    ActorSpriteData, BossFrameIndex, BossFrameView, HazardLaneFact,
};
pub use control_prompt::{
    publish_frontend_context_prompt, rebuild_control_prompt, ControlContextKind, ControlPrompt,
    PromptEntry,
};
// Re-exported so `ControlPrompt` consumers (the touch overlay) can name the
// slot vocabulary without a direct `entity_catalog` dep.
pub use ambition_entity_catalog::action_scheme::{ControlSlot, VisualId};
pub use dialog_view::{rebuild_dialog_view, DialogView};
pub use facts::*;
pub use pose_view::{
    rebuild_body_pose_views, rebuild_shield_rings_view, BodyPoseView, ShieldRingFact,
    ShieldRingsView,
};
pub use presented_pose::{
    PresentationPhase, PresentedFeaturePoses, PresentedPose, PresentedPosePlugin, PresentedPoseSet,
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
/// [`BodyPoseView`] components, and [`ShieldRingsView`]. All let observers
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
        app.init_resource::<BossFrameIndex>();
        app.init_resource::<NameplateIndex>();
        app.init_resource::<DialogView>();
        app.init_resource::<ControlPrompt>();
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
                    ambition_actors::features::advance_actor_anim_overlays,
                    rebuild_actor_anim_index,
                )
                    .chain(),
                // Player-bodied pose components + the pooled shield-ring rows —
                // the per-body half of the pose read-model (E4 slices 1–4).
                rebuild_body_pose_views,
                rebuild_shield_rings_view,
                // The dialogue overlay's row (recon C3): presentation reads
                // THIS, never the live `DialogState`.
                rebuild_dialog_view,
                // "What does each control do right now" for the controlled
                // subject — the touch overlay reads this instead of the sim.
                rebuild_control_prompt,
            )
                .in_set(ambition_platformer_primitives::schedule::SandboxSet::FeatureViewSync),
        );
    }
}
