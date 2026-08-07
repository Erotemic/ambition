//! Generic entity-marker components shared by reusable mechanics.
//!
//! These are pure `Component` markers — query filters with no fields and no
//! sandbox-internal dependencies. They live in the runtime crate so portal,
//! gravity, and other extracted mechanics can query the player / simulated
//! feature entities without depending on the sandbox's `player` or `features`
//! modules. The host (`ambition_platformer2d_actor_monolith`) re-exports them from their original
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

/// The entity currently driven by the primary local control authority.
///
/// `None` is only expected during startup/load frames before the primary player
/// brain has been resolved. This lives with the content-free player markers so
/// presentation/host adapters can follow the controlled body without depending
/// on the sandbox actor-systems crate.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlledSubject(pub Option<Entity>);

/// **The bodies to frame when no local authority is driving one.**
///
/// ⛔ **the answer to "what does a CPU-versus-CPU match look like", and until
/// this existed the answer was NOTHING.** The camera resolved its subject from
/// [`ControlledSubject`] and returned without one — correct for exploration,
/// where a session always has a driven body, and silently fatal for a match
/// that legitimately has no local participant. Jon's own run: *"when I seated 2
/// CPUs and pressed start, nothing shows up. No stage."*
///
/// ⭐ **a DECLARATION, not a guess.** Whoever knows what the session is about
/// publishes the cast; the resolver frames it. That is the difference between
/// this and the camera scanning for bodies on its own — a scan would have to
/// decide which bodies matter, which is exactly the question the publisher
/// already knows the answer to and the camera never can.
///
/// Empty means nothing has been declared, which is the ordinary case: with a
/// controlled subject present this is not consulted at all. Ordered by the
/// publisher (a match orders by SEAT) so the framing is stable frame to frame
/// rather than by whatever order a query happened to iterate —
/// see the Bevy entity-ordering traps this repo has been bitten by.
#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct FramedCast(pub Vec<Entity>);

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
