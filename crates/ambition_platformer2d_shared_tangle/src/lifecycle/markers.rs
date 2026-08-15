//! Lifetime-scope marker components (room / mode) and the rendered room-visual
//! markers, all runtime-owned so sim systems can tag entities without importing
//! presentation.
//!
//! ⛔ **every scope spelled here has a sweep that enforces it.** The two that did
//! not — `RunScopedEntity` and `PersistentEntity` — were deleted 2026-08-15 after
//! the D125 census found zero producers and zero consumers for either. A lifetime
//! you can declare and nothing enforces is worse than one that does not exist:
//! `spawn_run_scoped` read as "dies with the run" at a call site and produced an
//! entity that never died, with no test able to notice. The run lifetime is
//! [`super::SessionScopedEntity`], which carries a [`super::SessionScopeId`] and
//! is swept by `despawn_retired_session_entities`; "survives everything" is
//! spelled by carrying no scope marker, because every sweep is presence-driven.

use bevy::prelude::*;

/// Lifetime-scope marker: despawn when the current authored room is unloaded.
///
/// This marker is deliberately runtime-owned so simulation-only entities and
/// rendered room visuals can share the same lifecycle policy without depending
/// on presentation modules.
#[derive(Component, Default)]
pub struct RoomScopedEntity;

/// **RESIDENCY, SUSPENDED: this entity is in another entity's CUSTODY.**
///
/// A room-scoped entity is normally *resident* in the active room, and a room
/// CHANGE retires everything resident in the room it is leaving. An object a
/// body is CARRYING is not in the room — it is in the body, and it crosses the
/// boundary with whoever carries it. Its residency is therefore its holder's,
/// which is the whole of what custody means.
///
/// ⛔ **the LIFETIME is unchanged, and that is deliberate.** The entity keeps
/// [`RoomScopedEntity`]; the marker is never taken away, so no query that
/// requires the scope silently loses sight of it and the sandbox reset — which
/// destroys the world a resident belongs to *and* empties the hand that holds
/// this — still sweeps it. What is suspended is RESIDENCY, and only for the
/// sweep a room CHANGE runs ([`RoomResident`]). Retracting the suspension is a
/// RESET to the default (resident), never a retraction of the scope itself.
///
/// ⭐ **and residency resumes in whatever room is active then.**
/// [`RoomScopedEntity`] carries no room id — the sweep is presence-driven
/// against the room being left — so an object released from custody two rooms
/// later is resident in THAT room, and the next transition out of it retires
/// the object correctly. Nothing has to remember where it came from.
///
/// ⚠ **body-generic, and item-free vocabulary.** `0` is whatever entity took
/// custody: a couch seat, a possessed actor, an NPC. Nothing that reads this
/// knows items exist, which is the point — a room transition must not learn
/// about an inventory entourage in order to stop destroying one.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InCustodyOf(pub Entity);

/// **THE roster a room CHANGE retires**: scoped to the room, and actually
/// resident in it.
///
/// Spelled once, here, because the alternative is each host repeating
/// `With<RoomScopedEntity>` and only some of them learning about
/// [`InCustodyOf`] — the classic fork where two sweeps disagree about who lives
/// in the room. ⛔ the sandbox reset deliberately does NOT use this: a reset
/// destroys the world its residents live in, hands included.
pub type RoomResident = (With<RoomScopedEntity>, Without<InCustodyOf>);

/// Marker for a RENDERED room-scoped entity — a visual the presentation layer
/// draws/syncs for the current room. Presentation systems query `With<RoomVisual>`
/// to filter to the active room's rendered entities; the required
/// [`RoomScopedEntity`] gives it the room-unload/reset teardown automatically.
///
/// Lives here (not in `presentation`) deliberately: the marker is content-free
/// vocabulary, so sim systems can tag the visual entities they spawn WITHOUT
/// importing a presentation module (the whole point of the runtime-owned
/// lifecycle markers above).
#[derive(Component, Default, Clone)]
#[require(RoomScopedEntity)]
pub struct RoomVisual;

/// Lifetime-scope marker: despawn when the named GAME MODE deactivates.
///
/// A mode is the demo-hosting seam (decomposition D-C): the active room's
/// `RoomMetadata::mode` names which ruleset owns the room, so a mode-scoped
/// entity SURVIVES room transitions inside its own mode and dies the moment the
/// active room's mode is something else. That is a distinct lifetime from
/// [`RoomScopedEntity`] (dies every room load) and
/// [`SessionScopedEntity`](super::SessionScopedEntity) (dies with the
/// activation) — a hosted demo's mode-owner entity carries its rules' resources
/// across every room in its own zone.
///
/// The marker lives here with its lifetime-scope siblings; the sweep that
/// consumes it needs the active room's metadata and therefore lives a tier up
/// (`ambition_platformer2d_runtime::mode_scope`), exactly as the `RoomScopedEntity` sweep
/// lives above this crate.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ModeScopedEntity(pub String);

/// Marker on the player's rendered sprite entity. Content-free (a tag the renderer
/// queries + gameplay systems that manipulate the player visual reference); lives
/// here so neither side imports a presentation module to name it.
#[derive(Component, Clone, Default)]
pub struct PlayerVisual;

/// Marker for simulation-side feature entities spawned from the active room.
/// They are deliberately separate from presentation `FeatureVisual` sprites;
/// visible builds keep using the existing visual entities and look up live ECS
/// state by `FeatureId`. Lifecycle vocabulary: a room-scoped sim marker that
/// lives beside the other runtime-owned scope markers.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeatureSimEntity;

/// Marker on the rendered loading-zone indicator entity (keyed by zone `id`).
/// World/room systems spawn + reconcile these; content-free so they need no
/// presentation import.
#[derive(Component, Clone, Debug)]
pub struct LoadingZoneVisual {
    pub id: String,
}
