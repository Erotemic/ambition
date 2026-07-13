//! Gravity-zone mechanic plugin.
//!
//! Owns the registration that used to live inside `ambition_portal` (Stage 6
//! follow-up): the ambient-gravity resources, the per-frame gravity-zone
//! snapshot (oscillate → collect), the room-reset gravity reset, and the
//! ambient gravity-flip switch. This is a *gravity mechanic*, so it owns its own
//! scheduling and must not depend on `ambition_portal`.
//!
//! Note: `crate::physics::BaseGravity` (the ambient-gravity resource) STAYS in
//! `crate::physics` because it is read widely; this plugin only owns the
//! gravity-ZONE behavior (zones / switches that flip the ambient + their
//! per-frame snapshot), initializing the shared resources so the mechanic is
//! self-contained when installed.

use bevy::prelude::*;

use super::lifecycle::reset_gravity_on_room_reset;
use ambition_platformer_primitives::frame_env::{collect_force_zones, FrameResolveSet};
use ambition_platformer_primitives::schedule::SimScheduleExt;

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
        // `crate::physics` (read widely) but the gravity mechanic owns making
        // sure they (and the per-frame `GravityZones` snapshot) exist.
        app.init_resource::<crate::physics::GravityField>();
        app.init_resource::<crate::physics::BaseGravity>();
        app.init_resource::<crate::physics::GravityZones>();
        app.init_resource::<ambition_platformer_primitives::frame_env::ForceZones>();

        // Snapshot all gravity + force zones once per frame BEFORE the frame
        // resolution phase reads them, so every body can resolve its local frame
        // from this tick's environment. Portal carve publishing pins
        // `.after(collect_gravity_zones)` so the combined cadence is
        // byte-identical to the pre-extraction `PortalSet::GravityAndCarves`
        // chain.
        app.add_systems(
            sim,
            (
                crate::physics::oscillate_gravity_zones,
                crate::physics::collect_gravity_zones,
                collect_force_zones,
            )
                .chain()
                .in_set(GravitySet::ZoneSnapshot)
                .before(crate::schedule::SandboxSet::CoreSimulation),
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
                .before(crate::schedule::SandboxSet::CoreSimulation),
        );
        app.add_systems(
            sim,
            (
                super::resolve::resolve_body_motion_frames,
                crate::physics::resolve_active_gravity,
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
                .in_set(crate::schedule::SandboxSet::RoomTransition)
                .after(crate::session::reset::ContentRoomResetSet),
        );
    }
}
