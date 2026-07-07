//! Unified body kinematics for every controllable platformer body.
//!
//! Systems that hold multiple mutable [`BodyKinematics`] queries must prove
//! them disjoint with marker filters (`With<PlayerEntity>`, `With<ActorConfig>`,
//! `With<BossConfig>`, plus `Without<...>` guards where needed). Do that with
//! filters, never by re-splitting the component.

// The definition lives in `ambition_engine_core` (ADR 0019); re-export it from
// the runtime so existing `body::BodyKinematics` paths remain stable.
pub use ambition_engine_core::BodyKinematics;

use bevy::prelude::*;

/// Marks the single body whose position drives the room's live gravity
/// resolution (the active player). The runtime's `resolve_active_gravity`
/// queries `(&BodyKinematics, With<PrimaryBody>)` so it stays content-free; the
/// host (`ambition_actors`) adds this marker to its primary player entity.
///
/// Distinct from [`crate::markers::PrimaryPlayer`]: `PrimaryBody` is the
/// gravity-relevant body, `PrimaryPlayer` is the presentation/HUD-followed
/// player. The spawn bundle attaches both to the same entity today, but gravity
/// filters only on `PrimaryBody` so it never depends on the player-specific
/// marker.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PrimaryBody;
