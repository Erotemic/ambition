//! Generic entity-marker components shared by reusable mechanics.
//!
//! These are pure `Component` markers — query filters with no fields and no
//! sandbox-internal dependencies. They live in the runtime crate so portal,
//! gravity, and other extracted mechanics can query the player / simulated
//! feature entities without depending on the sandbox's `player` or `features`
//! modules. The host (`ambition_gameplay_core`) re-exports them from their original
//! paths so existing call sites compile unchanged.

use bevy::prelude::*;

/// Marker for **a player entity** — there may eventually be more than
/// one. Use this when a query wants every player regardless of locality
/// or which slot they occupy.
///
/// The game currently spawns exactly one player, with `PlayerSlot(0)`,
/// [`PrimaryPlayer`], and `LocalPlayer` all attached. Systems that
/// want the camera/HUD/dev-tool target should filter on `PrimaryPlayer`
/// (or use the helpers in the sandbox's `player::queries`) rather than
/// assuming the only `PlayerEntity` is *the* player.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerEntity;

/// Marks the **home avatar** / respawn identity — the ORIGINAL body, its save
/// identity, respawn anchor, and inventory owner. Exactly one entity carries it.
///
/// IMPORTANT: `PrimaryPlayer` does NOT mean "the currently controlled body". The
/// controlled body is whichever entity carries `Brain::Player(PlayerSlot::PRIMARY)`
/// — during possession that is a DIFFERENT entity (the possessed actor). Input,
/// abilities, camera, portal viewer, and the melee lifecycle derive from the
/// `ControlledSubject` resource (`abilities::traversal::possession`), not from this
/// marker. Reserve `PrimaryPlayer` for genuinely home-body concerns: respawn,
/// sandbox reset, save sync, spawn-clone-relative-to, heal fallback, and the HUD /
/// debug subject (which still show the home avatar's stats by design).
///
/// Distinct from `LocalPlayer` because in a future split-screen
/// build the local players would each be `LocalPlayer` but only one
/// would be `PrimaryPlayer` (e.g. the host's view in a guest-joined
/// session).
///
/// Distinct from [`crate::body::PrimaryBody`] too: `PrimaryPlayer` names the
/// *player* the presentation layer follows, while `PrimaryBody` names the body
/// whose position drives the room's live gravity resolution. Today the spawn
/// bundle attaches both to the same entity, but they are kept separate so the
/// content-free gravity runtime never has to filter on the player-specific
/// marker.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrimaryPlayer;

/// Query filter selecting the ONE primary player body — `With<PlayerEntity>` AND
/// `With<PrimaryPlayer>`. A pure composition of two markers that both live here,
/// so it belongs beside them: reusable mechanics + presentation can filter on the
/// camera/HUD/dev-tool target without depending on the sandbox's `player` module.
pub type PrimaryPlayerOnly = (With<PlayerEntity>, With<PrimaryPlayer>);
