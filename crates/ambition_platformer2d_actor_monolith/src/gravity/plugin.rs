//! Gravity-zone mechanic plugin.
//!
//! This is a *gravity mechanic*, so it owns its own scheduling and must not depend on
//! `ambition_portal2d`.
//!
//! Note: `ambition_platformer2d_shared_tangle::gravity::BaseGravity` (the ambient-gravity resource) STAYS in
//! [`ambition_platformer2d_shared_tangle::gravity`] because it is read widely; this plugin only owns the
//! gravity-ZONE behavior (zones / switches that flip the ambient + their
//! per-frame snapshot), initializing the shared resources so the mechanic is
//! self-contained when installed.

use bevy::prelude::*;

use super::lifecycle::reset_gravity_on_room_reset;
use ambition_platformer2d_shared_tangle::frame_env::{collect_force_zones, FrameResolveSet};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// Gravity-mechanic schedule labels, local to the gravity subsystem.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GravitySet {
    /// Snapshot every gravity zone (oscillate → collect) once per frame BEFORE
    /// actor integrators read them, so each body resolves local gravity by
    /// position. Portal carve publishing pins itself after this set so the
    /// early-world snapshot cadence is identical to before the extraction.
    ZoneSnapshot,
    /// Reset-time gravity reset (room transition).
    RoomReset,
}

/// Top-level gravity-zone mechanic plugin.
pub struct GravityPlugin;

impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Shared ambient-gravity resources. `BaseGravity`/`GravityField` live in
        // shared_tangle (read widely) but the gravity mechanic owns making
        // sure they (and the per-frame `GravityZones` snapshot) exist.
        app.init_resource::<ambition_platformer2d_shared_tangle::gravity::GravityField>();
        app.init_resource::<ambition_platformer2d_shared_tangle::gravity::BaseGravity>();
        app.init_resource::<ambition_platformer2d_shared_tangle::gravity::GravityZones>();
        app.init_resource::<ambition_platformer2d_shared_tangle::frame_env::ForceZones>();

        // the gravity capability publishes its own construction schema, exactly as the portal
        // gun does — metadata only, so prepared-content fingerprinting names the domain while the
        // executable constructor stays the closed `GravityZoneConstruction` dispatch.
        app.init_resource::<
            ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog,
        >();
        let gravity_registry =
            ambition_platformer2d_shared_tangle::gravity::construction::gravity_zone_construction_registry();
        app.world_mut()
            .resource_mut::<
                ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog,
            >()
            .try_contribute(
                ambition_platformer2d_shared_tangle::gravity::construction::GRAVITY_ZONE_CONSTRUCTION_DOMAIN,
                gravity_registry.deterministic_dump(),
            )
            .expect("the gravity-zone construction schema cannot conflict with itself");

        // Oscillation advances zones by THIS tick's dt, so the snapshot follows
        // the clock.
        app.configure_sets(
            sim,
            GravitySet::ZoneSnapshot
                .after(ambition_platformer2d_shared_tangle::schedule::SimClockHead),
        );
        // Portal carve publishing pins `.after(collect_gravity_zones)` so the combined cadence
        // is byte-identical to the pre-extraction `PortalSet::GravityAndCarves` chain
        // (cite-ok: that variant is gone -- `PortalSet::Carves` is what survived the split).
        app.add_systems(
            sim,
            (
                ambition_platformer2d_shared_tangle::gravity::oscillate_gravity_zones,
                ambition_platformer2d_shared_tangle::gravity::collect_gravity_zones
                    .in_set(ambition_platformer2d_shared_tangle::gravity::GravityZonesCollected),
                collect_force_zones,
            )
                .chain()
                .in_set(GravitySet::ZoneSnapshot)
                .before(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::CoreSimulation),
        );

        // THE frame resolution phase (ADR 0024): after the zone snapshot, before
        // any CoreSimulation consumer — the player brain (`PlayerInput`), actor
        // and possessed brains (`WorldPrep`), body integration, and combat all
        // read the per-body `ResolvedMotionFrame` published here. The
        // presentation `GravityField` mirror derives from the SAME artifact,
        // chained immediately after the resolver.
        app.configure_sets(
            sim,
            FrameResolveSet
                .after(GravitySet::ZoneSnapshot)
                .before(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::CoreSimulation),
        );
        // Ambient-gravity changes arrive as REQUESTS from outside the sim and
        // are applied here, in the sim, before the resolver copies the ambient
        // into each body's frame — see `AmbientGravityRequest` for why a dev
        // control must not write `BaseGravity` itself.
        app.add_message::<ambition_platformer2d_shared_tangle::gravity::AmbientGravityRequest>();
        app.add_systems(
            sim,
            (
                ambition_platformer2d_shared_tangle::gravity::apply_ambient_gravity_requests,
                super::resolve::resolve_body_motion_frames,
                ambition_platformer2d_shared_tangle::gravity::resolve_active_gravity,
            )
                .chain()
                .in_set(FrameResolveSet),
        );

        // ⛔ THE OVERLAP PLATE IS GONE — RULED 2026-09-19 (Q137). Gravity
        // switching stays; what went is the second, unreachable ROAD to it. The
        // `GravityFlipSwitch` component and its system were never registered
        // here, nothing authored or spawned one, and they survived only for
        // their own unit test — while carrying a rollback registration, a
        // view-facts rebuild and a renderer. A later pressure plate is an INPUT
        // into `BaseGravity`, the same way the LDtk-authored `FlipGravity`
        // switch already is; it is not a reason to keep a parallel mechanism
        // alive with no author.

        // Reset gravity to default when the room resets — after the
        // content layer's room-reset work (named boss arenas), ordered
        // against the SET label so this generic plugin names no content.
        // ⛔⛔ **A `MessageReader` FOR AN UNREGISTERED MESSAGE PANICS AT PARAMETER
        // VALIDATION, NOT AT COMPILE TIME.** `reset_gravity_on_room_reset` grew a
        // second reader — `NewGameResetCommitted`, owned by
        // `session::reset`'s plugin — and a composition that installs gravity
        // without that plugin died on its first frame (measured: three gravity
        // unit apps). `add_message` is guarded against a second registration, so
        // the system and the channel it reads are declared together here, which
        // is the only arrangement a composition cannot get half of.
        app.add_message::<crate::session::reset::NewGameResetCommitted>();
        app.add_systems(
            sim,
            reset_gravity_on_room_reset
                .in_set(GravitySet::RoomReset)
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::RoomTransition)
                .after(crate::session::reset::ContentRoomResetSet),
        );
    }
}
