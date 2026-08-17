//! Unified body kinematics for every controllable platformer body.
//!
//! Systems that hold multiple mutable [`BodyKinematics`] queries must prove
//! them disjoint with marker filters (`With<PlayerEntity>`, `With<ActorConfig>`,
//! `With<BossConfig>`, plus `Without<...>` guards where needed). Do that with
//! filters, never by re-splitting the component.

// The definition lives in `ambition_platformer2d_core` (ADR 0019); re-export it from
// the runtime so existing `body::BodyKinematics` paths remain stable.
pub use ambition_platformer2d_core::BodyKinematics;

use bevy::prelude::*;

/// Marks the single body whose position drives the room's live gravity
/// resolution (the active player). The runtime's `resolve_active_gravity`
/// queries `(&BodyKinematics, With<PrimaryBody>)` so it stays content-free; the
/// host (`ambition_platformer2d_actor_monolith`) adds this marker to its primary player entity.
///
/// Distinct from [`crate::markers::PrimaryPlayer`]: `PrimaryBody` is the
/// gravity-relevant body, `PrimaryPlayer` is the presentation/HUD-followed
/// player. The spawn bundle attaches both to the same entity today, but gravity
/// filters only on `PrimaryBody` so it never depends on the player-specific
/// marker.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PrimaryBody;

/// Emitted the frame a mount dies and its rider dismounts (the
/// `(dead-mount, still-mounted)` dissolution the mount coupling enforces).
/// Carries both entities so a consumer can react to either side.
///
/// This is a body FACT crossing out of the mount coupling — deliberately NOT
/// routed through the `EncounterGate` script bus (that channel is
/// script-vocabulary). The boss-encounter bridge subscribes to turn it into a
/// `mount_died` external phase trigger — the boss whose mount died fights on
/// foot in an authored mini-phase (ADR 0020; Q19). Any other system may
/// subscribe to the same message later without touching this one.
///
/// ⭐ **it lives HERE, below the domains, because two of them share it.** The
/// writer is the mount coupling in the actor monolith and the reader is
/// `ambition_boss_encounter`; a message owned by one of the two would make the
/// other depend on it for a type carrying nothing but a pair of entities. Same
/// shape, and the same reason, as `FeatureInteractionSet` being put here so a
/// carved module could still name the ordering it participates in.
#[derive(Message, Clone, Copy, Debug)]
pub struct MountDied {
    pub mount: Entity,
    pub rider: Entity,
}
