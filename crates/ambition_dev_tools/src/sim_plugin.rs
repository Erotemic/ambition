//! `DevToolsSimPlugin` — the dev-tools DOMAIN plugin for the simulation App.
//!
//! Owns the dev-editable simulation resources and registers their live-edit
//! systems into public sets. The runtime positions those sets in its phase
//! chains without naming the leaf systems.

use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::{App, IntoScheduleConfigs, Plugin, SystemSet};

/// Host-frame seam: mirror the player's live stats back into the
/// inspector-editable resource so the F3 panel shows truth.
///
/// ⚠ It moved OUT of the simulation schedule on 2026-09-14 — see the
/// registration. A set that other crates order against keeps its name; what
/// changed is the schedule it lives in, so a composition ordering against it
/// must do so in `Update`.
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
        // ⭐ THE WORLD-SOURCE WATCHER IS THIS CRATE'S, and it now says so. Its
        // resource and its `Update` system were registered by the ACTOR KERNEL's
        // feature plugin, which is a simulation package registering a developer
        // facility — the "anti-god rule" one line up, applied to the one row that
        // had escaped it. Default = watcher disabled; the visible app pre-inserts
        // its resolved value before the engine group, and `init_resource` never
        // clobbers.
        app.init_resource::<crate::WorldSourceHotReload>();
        // ⭐⭐ THE DEV TOOL WRITES; THE SIM READS. What every authored actor's
        // brain is forced to used to be two process-global `OnceLock`s that the
        // ACTOR KERNEL called while building a live brain. The value is a
        // session resource now, published here from the environment exactly
        // once, and `ambition_platformer2d_actor_monolith` names this crate for
        // it nowhere.
        //
        // ⛔ `insert_resource`, NOT `init`: `Default` is "nobody is steering",
        // and a knob that resolved to the default whenever something else
        // initialized the resource first would be a knob that silently stopped
        // working.
        app.insert_resource(crate::brain_override::from_env());
        // The actor population cap, same shape and same reason — see
        // `population_cap`. Inert (uncapped) unless the environment says.
        app.insert_resource(crate::population_cap::from_env());
        // The other axis of the same experiment; see `perception_extent`.
        app.insert_resource(crate::perception_extent::from_env());
        // ⛔⛔ `Update`, NOT THE SIMULATION. It does a BLOCKING `fs::metadata` —
        // measured at up to 3.9ms on virtiofs — so on the sim schedule a
        // dev-tooling stat sat inside the deterministic tick. It also reads
        // wall-clock `Res<Time>` and keeps its debounce in a `Local`, neither of
        // which rewinds. Every reader is a MENU system in `Update` already.
        app.add_systems(bevy::app::Update, crate::poll_world_source_changes);
        // ⛔⛤ **THE MOVEMENT-TUNING EDIT LEFT THE SIM SCHEDULE ON 2026-09-13.**
        // It used to sit at the head of the chain below, copying the inspector
        // mirror into `ActiveMovementTuning` — the value every movement policy
        // reads. Under the rollback host the sim schedule IS `GgrsSchedule`, so
        // that write happened inside the rollback window and a resimulation of
        // confirmed frames read it: `Q120` measured the sync-test canary
        // desyncing on exactly this edit.
        //
        // ⭐ It is now a PROPOSAL decided before the advance. The three sets come
        // from `ambition_platformer2d_core`; the rollback host, when one is
        // installed, orders them `.before(RunGgrsSystems)` and supplies the
        // decision. Without a host they run in `PreUpdate` and publish by
        // default — live editing is unchanged for every non-rollback build.
        //
        // ⚠ BOTH crates configure this chain, because either can be installed
        // without the other. `configure_sets` is additive, so the constraints
        // compose rather than compete.
        app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
        app.init_resource::<ambition_platformer2d_core::MechanicalEditAdmission>();
        // ⛔ A RESOURCE, NOT A `Local`: three systems share it now (proposer,
        // publisher, mirror) — and as a `Local` inside the sim schedule it
        // advanced once per ADVANCE, resimulations included.
        app.init_resource::<crate::dev_tools::PlayerStatsSyncSnapshot>();
        // ⛔ THE ADMITTED BODY PROFILE, which outlives every body that wears it.
        // See `ActivePlayerBodyProfile`: admitting a value and projecting it onto
        // a target are different jobs, and collapsing them lost edits made while
        // no player existed.
        app.init_resource::<crate::dev_tools::ActivePlayerBodyProfile>();
        // ⭐ AND THE SAME THIRD STAGE FOR THE ABILITY DOMAIN, 2026-09-14. It was
        // the last domain treating its EDITOR resource as the admitted authority,
        // which tied admission to a primary player existing. See
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
                    // ⭐ THE FIFTH DOMAIN, `EditableFeelTuning`, registers in
                    // `ambition_platformer2d_runtime` INSTEAD OF HERE — this crate
                    // does not depend on `ambition_combat` and adding that edge to
                    // reach one resource would be convenience wearing ownership's
                    // clothes. The sets are published by
                    // `ambition_platformer2d_core`, so ordering composes without
                    // the dependency.
                )
                    .in_set(ambition_platformer2d_core::MechanicalEditSet::Propose),
                // ⛔⛤ **THESE TWO LEFT THE SIM SCHEDULE ON 2026-09-13, AND THE
                // COMMENT THAT USED TO SIT ON ONE OF THEM WAS THE DEFECT:** *"this
                // mutates rollback state, so it must run in the simulation schedule
                // with the other developer edits."* Under the rollback host the sim
                // schedule IS `GgrsSchedule`, so "with the other developer edits"
                // put the write INSIDE the rollback window, where a resimulation of
                // confirmed frames reads it. `sync_live_player_dev_edits_system`
                // wrote `BodyAbilities`/`BodyFlightState`/`MotionModel`/
                // `BodyDashState`/`BodyJumpState` there from a live inspector
                // resource; `sync_developer_body_profile` wrote `BodyKinematics`
                // and `BodyBaseSize`, arbitrated by a `Local` that runs once per
                // ADVANCE and therefore remembered across a rewind.
                //
                // ⭐ The CHAIN is preserved and now means something stronger: the
                // movement publisher runs before the ability publisher, so an
                // admitted tuning edit is visible to it — and all of it lands
                // before `RunGgrsSystems`.
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
        // ⛔⛤ **A PRESENTATION MIRROR IN `Update`, AND IT USED TO BE IN THE SIM
        // SCHEDULE — i.e. inside `GgrsSchedule` under the rollback host.** It
        // never changed authoritative mechanics there, so this was not the
        // determinism defect `Q120` was about; it was the wrong LIFETIME. A
        // body→panel copy that runs once per resimulated frame makes the stats
        // domain's proposal discrimination depend on how many times history was
        // replayed, when the question it answers — *"did the developer type a
        // number, or did the game change one?"* — is a question about RENDERED
        // frames. GPT architecture review, 2026-09-14.
        //
        // ⭐ `Update` RUNS AFTER THE SIM ADVANCED THIS FRAME (`RunGgrsSystems`
        // sits in `PreUpdate`), so the panel still shows the post-advance body,
        // which is the whole job. The editor→body twin stays in the `PreUpdate`
        // mechanical-edit chain where admission owns it.
        app.add_systems(
            bevy::prelude::Update,
            crate::dev_tools::mirror_player_stats_into_the_inspector.in_set(DevInspectorMirrorSet),
        );
        let sim = app.sim_schedule();
        // The HUD flash this crate owns, decayed by this crate. It was one line
        // in the actor kernel's `cleanup_timers_system`, which is a simulation
        // package winding down a developer timer — and the only thing that kept
        // a `ResMut<DeveloperRuntimeState>` in the kernel's control module.
        // ⛔⛤ **THE PROJECTION STAYS IN THE SIM SCHEDULE AND THAT IS CORRECT.** It
        // writes no mechanical DECISION — it copies an already-admitted value onto
        // a body — so it is reconciliation, the same class as
        // `sync_live_player_dev_edits_system`'s ability refresh. What had to leave
        // `GgrsSchedule` was the read of a live EDITOR resource, and that is now
        // upstream in `MechanicalEditSet::Publish`.
        //
        // ⚠ And it has to be here rather than in `PreUpdate`: a body rebuilt by a
        // reset or a room load appears DURING the simulation, and a projection
        // that only ran before the advance would leave it wearing engine defaults
        // for a frame.
        app.add_systems(sim, crate::dev_tools::project_developer_body_profile);
        // ⭐ AND THE ABILITY PROJECTION BESIDE IT, 2026-09-14, for the reason the
        // body-profile one is here: a body REBUILT by mechanical lifecycle code
        // during the simulation must wear its admitted abilities on the same tick,
        // not a render frame later. Admission stays in `PreUpdate`; only the
        // projection follows body existence.
        app.add_systems(sim, crate::project_editable_abilities);
        app.add_systems(sim, crate::decay_developer_presentation_flash);
        // ⭐ AND THE SLOW-MOTION REQUEST, for the same reason: the toggle is this
        // crate's, so the ASK is this crate's. It was rung 4 of the actor
        // kernel's time-scale ladder, which made a simulation package read
        // developer state; `apply_clock_scale_requests` reduces by `min`, so
        // this needs no ordering against the kernel's own request.
        app.add_systems(sim, crate::request_developer_slow_motion);
    }
}
