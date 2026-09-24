//! `DevToolsSimPlugin` — the dev-tools domain plugin for the simulation App.
//!
//! Owns the dev-editable simulation resources and registers their live-edit
//! systems into public sets. The runtime positions those sets in its phase
//! chains without naming the leaf systems.

use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, SystemSet};

/// Host-frame seam: mirror the player's live stats back into the
/// inspector-editable resource so the F3 panel shows truth.
///
/// The set is in `Update`, not the simulation schedule, so a composition that
/// orders against it must do so in `Update`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DevInspectorMirrorSet;

pub struct DevToolsSimPlugin;

impl Plugin for DevToolsSimPlugin {
    fn build(&self, app: &mut App) {
        // The dev-editable sim resources this crate owns (anti-god rule: the
        // plugin that owns the systems initializes their resources).
        app.init_resource::<crate::profiling::StartupProfiler>();
        app.init_resource::<crate::profiling::FrameCensus>();
        app.init_resource::<crate::DeveloperRuntimeState>();
        app.init_resource::<crate::dev_tools::DeveloperTools>();
        app.init_resource::<crate::dev_tools::EditablePlayerStats>();
        app.init_resource::<crate::dev_tools::EditableMovementTuning>();
        app.init_resource::<crate::dev_tools::EditableAbilitySet>();
        // The world-source watcher. Default: disabled. The visible app inserts its
        // resolved value before the engine group, and `init_resource` does not
        // overwrite it.
        app.init_resource::<crate::WorldSourceHotReload>();
        // The dev tool writes the brain override; the sim reads it as a session
        // resource, published here once from the environment.
        //
        // `insert_resource`, not `init_resource`: `Default` means "nobody is
        // steering", and an earlier `init` elsewhere must not disable the knob.
        app.insert_resource(crate::brain_override::from_env());
        // The actor population cap, same shape and same reason — see
        // `population_cap`. Inert (uncapped) unless the environment says.
        app.insert_resource(crate::population_cap::from_env());
        // The other axis of the same experiment; see `perception_extent`.
        app.insert_resource(crate::perception_extent::from_env());
        // `Update`, not the simulation. It does a blocking `fs::metadata` (up to
        // 3.9 ms on virtiofs), reads wall-clock `Res<Time>`, and keeps its debounce
        // in a `Local`, none of which rewinds. Its readers are menu systems in
        // `Update`.
        app.add_systems(bevy::app::Update, crate::poll_world_source_changes);
        // Mechanical edits are proposals decided before the advance, not writes in
        // the sim schedule. Under the rollback host the sim schedule is
        // `GgrsSchedule`, and a write there is read by resimulations of confirmed
        // frames, which desyncs.
        //
        // The three sets come from `ambition_platformer2d_core`. The rollback host,
        // when installed, orders them `.before(RunGgrsSystems)` and supplies the
        // decision. Without a host they run in `PreUpdate` and publish by default.
        // Both crates configure this chain, because either can be installed alone;
        // `configure_sets` is additive.
        app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
        app.init_resource::<ambition_platformer2d_core::MechanicalEditAdmission>();
        // A resource, not a `Local`: the proposer, publisher and mirror share it.
        app.init_resource::<crate::dev_tools::PlayerStatsSyncSnapshot>();
        // The admitted body profile outlives every body that wears it. See
        // `ActivePlayerBodyProfile`.
        app.init_resource::<crate::dev_tools::ActivePlayerBodyProfile>();
        // The same admitted stage for the ability domain. See
        // `ActiveEditableAbilityMask`.
        app.init_resource::<crate::dev_tools::ActiveEditableAbilityMask>();
        ambition_platformer2d_shared_tangle::schedule::configure_mechanical_edit_sets(app);
        app.add_systems(
            bevy::app::PreUpdate,
            (
                (
                    crate::dev_tools::propose_editable_movement_tuning,
                    crate::propose_editable_abilities,
                    crate::dev_tools::propose_developer_body_profile,
                    crate::dev_tools::propose_player_stats_edits,
                    // `EditableFeelTuning` registers in `ambition_platformer2d_runtime`, not
                    // here: this crate does not depend on `ambition_combat`. The sets come
                    // from `ambition_platformer2d_core`, so ordering works without that
                    // dependency.
                )
                    .in_set(ambition_platformer2d_core::MechanicalEditSet::Propose),
                // These write rollback state, so they run here, before `RunGgrsSystems`,
                // not in the sim schedule. The chain runs the movement publisher before the
                // ability publisher, so an admitted tuning edit is visible to it.
                (
                    crate::dev_tools::publish_editable_movement_tuning,
                    crate::admit_editable_abilities,
                    crate::dev_tools::sync_developer_body_profile,
                    crate::dev_tools::publish_player_stats_edits,
                )
                    .chain()
                    .in_set(ambition_platformer2d_core::MechanicalEditSet::Publish),
            ),
        );
        // A presentation mirror in `Update`, not the sim schedule. In the sim
        // schedule it would run once per resimulated frame, but its job (telling a
        // developer edit from a game change) is about rendered frames. `Update` runs
        // after the sim advance (`RunGgrsSystems` is in `PreUpdate`), so the panel
        // shows the post-advance body. The editor-to-body direction stays in the
        // `PreUpdate` chain above.
        app.add_systems(
            bevy::prelude::Update,
            crate::dev_tools::mirror_player_stats_into_the_inspector.in_set(DevInspectorMirrorSet),
        );
        let sim = app.sim_schedule();
        // The projection stays in the sim schedule. It copies an already-admitted
        // value onto a body, so it is reconciliation, like
        // `contribute_editable_ability_mask`; only the read of the live editor
        // resource had to leave. It must run here, not in `PreUpdate`: a body rebuilt
        // by a reset or room load appears during the simulation and would otherwise
        // keep engine defaults for a frame.
        app.add_systems(sim, crate::dev_tools::project_developer_body_profile);
        // The ability mask's contribution, for the same reason. It is written before
        // integration, where `project_body_abilities` folds every source into the
        // effective set.
        app.add_systems(
            sim,
            crate::contribute_editable_ability_mask
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate),
        );
        app.add_systems(bevy::app::Update, crate::decay_developer_presentation_flash);
        // The slow-motion toggle belongs to this crate, so the request does too.
        // `apply_clock_scale_requests` reduces by `min`, so no ordering is needed.
        app.add_systems(sim, crate::request_developer_slow_motion);
    }
}
