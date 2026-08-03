//! **The lifecycle domain's rollback schema.**
//!
//! Scope identities — the deterministic allocators that decide which entities a
//! reset, a round or a session owns. What a rewind has to put back is not the
//! entities themselves but the *id they are culled by*.
//!
//! ⚠ **why this module exists rather than a line in the central function.** The
//! first registration here was written centrally and the
//! `central-rollback-does-not-enumerate-domains` contract caught it: the central
//! function is allowed to name runtime-adjacent state (engine_core, persistence,
//! sfx, sim_view, time, world) and nothing else, and a scope allocator is
//! gameplay-domain state. The contract's own remedy is the one taken here —
//! *"if the domain has no module yet, adding one is the work."*
//!
//! ⚠ the owner label stays `ambition_platformer2d_runtime` because this module
//! is in it, and must be: `ambition_platformer2d_shared_tangle` sits below the
//! runtime in the crate graph.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;

const OWNER: &str = "ambition_platformer2d_runtime";

/// Register everything the lifecycle domain needs rewound.
pub(in crate::rollback) fn register(app: &mut App) {
    // `next_raw` is a deterministic id allocator and `settle_versus_round`
    // mints the next round from inside the sim schedule, so a rewind across a
    // round boundary re-runs the mint — and without this, against a `next_raw`
    // that never rewound. The resimulated timeline would allocate a different
    // `RoundScopeId` than the one it is reproducing, and `RoundScopedEntity`
    // culls by that id: entities would be spared or despawned differently on the
    // two timelines. Found 2026-08-03 by the shipped-composition resource sweep,
    // the third real defect it has caught after `BrokenBricks` and
    // `SpentMonitors`.
    //
    // ⚠ **OPTIONAL-canonical, not canonical.** `RoundScopePlugin` is installed
    // by whatever composes a MATCH — a single-player platformer has no rounds
    // and carries no round culler — so this resource legitimately COMES AND
    // GOES. `rollback_resource_canonical` installs a checksum system taking
    // `Res<T>`, which panics on every frame the resource is absent; picking it
    // first turned eight rollback-oracle tests red in the calibration lab, which
    // composes no match. The optional form checksums `Option<T>` so "no match
    // yet" and "round 0 is live" are different values rather than one of them
    // being unrepresentable.
    app.rollback_resource_optional_canonical::<
        ambition_platformer2d_shared_tangle::lifecycle::ActiveRoundScope,
    >(OWNER, "resource.active_round_scope");
}
