//! Ordinary room transitions, end to end — the ENGINE's copy.
//!
//! Detection (`detect_room_transition_system`) and the per-room feature reset
//! have always been engine-side. Everything between them — turning the request
//! into an exact `ambition_load` transaction, preflighting the target while the
//! source room stays authoritative, waiting for readiness, taking one-shot
//! authorization on a later tick, and performing the commit — used to live in
//! `ambition_app`. That made room transitions a GAME capability: no demo host
//! could change rooms, which is why Super Mary-O's secret vault had to be dug
//! into the same `RoomSpec` as the surface instead of being a second room.
//!
//! The blocker was never a dependency. It was one call: the commit drew the new
//! room itself (`spawn_room_visuals`), which named `ambition_render`. Now it
//! writes `RespawnRoomVisualsRequested` like every other room-changing path, and
//! the whole chain names nothing an engine crate may not.
//!
//! ## What a host still owns
//!
//! Two OPTIONAL contributors, both marker-gated so absence is honest rather than
//! silent:
//!
//! - [`RoomTransitionAssetContributor`] — "has the destination room's art
//!   arrived". Needs a sprite catalog, an asset server, and resolved visual
//!   quality; a headless host has none and its work item is `Skipped`.
//! - [`RoomTransitionPresentationAvailable`] — "a cover has survived a
//!   presentation frame", so a visible host never exposes a partially built
//!   room. A headless host commits as soon as readiness is authorized.
//!
//! Plus [`RoomConstructionPlanPrefetch`], which the host FILLS and the
//! transition READS: deciding when to prepare a neighbor is host policy, but a
//! prepared plan is an engine artifact and promoting one is engine identity.

mod commit;
mod loading;
mod prefetch;

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

pub use commit::{commit_ready_room_transition_system, RoomClock, RoomTransitionEffects};
pub use loading::{
    advance_room_transition_content_epoch_system, authorize_ready_room_transition_system,
    begin_room_transition_load_system, finalize_unpresented_room_transition_failure_system,
    set_room_transition_work_state, ActiveRoomTransitionLoad, RoomTransitionAssetContributor,
    RoomTransitionContentEpoch, RoomTransitionLoadPhase, RoomTransitionLoadState,
    RoomTransitionPresentationAvailable,
};
pub use prefetch::RoomConstructionPlanPrefetch;

/// The readiness transaction + authorized commit, registered into the gap
/// `RoomTransitionSchedulePlugin` documents: after detection, before the
/// per-room feature reset.
///
/// Part of [`crate::PlatformerEnginePlugins`], so every host — Ambition, a demo
/// app, an external provider — gets the same transition without registering
/// anything. Registering a second copy is a hard schedule error in a host that
/// carries an ordering edge against these systems, and a silent double-execution
/// in one that does not; do not.
/// **The host-side readiness phase** — begin the transaction, preflight the
/// destination, gather asset readiness, authorize the commit. Never rewound, and
/// never a room mutation.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RoomTransitionReadinessSet;

pub struct RoomTransitionComposerPlugin;

impl Plugin for RoomTransitionComposerPlugin {
    fn build(&self, app: &mut App) {
        // The transition IS an `ambition_load` plan — barrier, work items,
        // one-shot commit authorization — so the engine group owes the
        // coordinator rather than requiring every host to remember it.
        //
        // Unconditional: the plugin is idempotent by construction now, so a host
        // or shell group that also adds it composes in any order. The guard that
        // used to be here worked for THIS caller and could not help the shell
        // group, which adds the plugin as a `PluginGroupBuilder` member and
        // cannot make that conditional.
        app.add_plugins(ambition_load::AmbitionLoadPlugin);
        app.init_resource::<RoomTransitionContentEpoch>()
            .init_resource::<RoomTransitionLoadState>()
            .init_resource::<RoomConstructionPlanPrefetch>();
        let sim = app.sim_schedule();
        // ⭐⭐ **READINESS IS HOST-SIDE; THE ROOM CHANGE IS SIMULATION** (D71).
        //
        // These four ask a question — is the destination prepared, are its assets
        // accounted for, is the cover up, may this commit — and answering it
        // mutates no sim state. None of them is rollback-registered, and running
        // them inside a rewound schedule is what kept the whole transaction off
        // the shipped host: a rollback host cannot run the load machine on a
        // speculative frame, so it ran nothing at all and every room change in
        // the game went uncovered.
        //
        // In `Update` they are never rewound, so BOTH hosts get the same
        // readiness. The room change itself stays in the sim schedule, where the
        // eager host performs it — the rollback host reaches an identical change
        // through `lifecycle_commit::commit_confirmed_lifecycle`, which rebases.
        app.add_systems(
            Update,
            (
                begin_room_transition_load_system,
                authorize_ready_room_transition_system,
                finalize_unpresented_room_transition_failure_system,
            )
                .chain()
                .in_set(RoomTransitionReadinessSet),
        );
        app.add_systems(
            sim,
            (
                advance_room_transition_content_epoch_system,
                commit_ready_room_transition_system,
            )
                .chain()
                // THE transaction phase — detection has run, the reset has not.
                .in_set(ambition_platformer2d_shared_tangle::schedule::RoomTransitionSet::Apply),
        );
        // ⛔ **STATED, not inferred.** On a host whose sim IS `Update` all three
        // phases share one graph, and leaving Bevy's parameter-conflict
        // resolution to order them would be luck wearing a deterministic face. On
        // a `FixedUpdate` or GGRS host the sim runs in its own schedule and
        // precedes `Update` within the frame anyway, so there is no edge to
        // draw — and drawing one against a set with no members here would be a
        // silently vacuous `.after`.
        if app.sim_is(Update) {
            app.configure_sets(
                Update,
                RoomTransitionReadinessSet
                    .after(ambition_platformer2d_shared_tangle::schedule::RoomTransitionSet::Detect)
                    .before(
                        ambition_platformer2d_shared_tangle::schedule::RoomTransitionSet::Apply,
                    ),
            );
        }
    }
}
