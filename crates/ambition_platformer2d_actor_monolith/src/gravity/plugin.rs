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

        // Portal carve publishing pins `.after(collect_gravity_zones)` so the combined cadence
        // is byte-identical to the pre-extraction `PortalSet::GravityAndCarves` chain.
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
        app.add_systems(
            sim,
            (
                super::resolve::resolve_body_motion_frames,
                ambition_platformer2d_shared_tangle::gravity::resolve_active_gravity,
            )
                .chain()
                .in_set(FrameResolveSet),
        );

        // NOTE: `gravity_flip_switch_system` is intentionally NOT registered.
        // Nothing spawns a `GravityFlipSwitch` in-game (the hub flip is an
        // LDtk-authored Switch handled by the encounter system); the component +
        // system exist only for the unit test + any future overlap-style plate.
        // It was never registered in the app schedule before the extraction, so
        // leaving it unregistered preserves behavior exactly.

        // Reset gravity to default when the room resets — after the
        // content layer's room-reset work (named boss arenas), ordered
        // against the SET label so this generic plugin names no content.
        app.add_systems(
            sim,
            reset_gravity_on_room_reset
                .in_set(GravitySet::RoomReset)
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::RoomTransition)
                .after(crate::session::reset::ContentRoomResetSet),
        );
    }
}
