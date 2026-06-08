//! Ambition identity → portal-transit policy glue.
//!
//! The generic portal core ([`crate::portal::portal_transit`]) drives any body
//! carrying [`BodyKinematics`] + a [`PortalBody`] marker + a [`PortalPolicy`]
//! through a placed pair. It never names Player / Boss / Enemy — that identity
//! lives here. These adapters:
//!
//! - **Tag bodies** ([`ensure_portal_bodies`]): add the marker + the correct
//!   policy to exactly the entities that transited before this unification —
//!   the primary player and every non-player actor with a `BodyKinematics`.
//! - **Reproduce the player input bits** ([`portal_player_input_adapter`]): read
//!   the core's [`PortalBodyTransited`] event and, for the player only, emit the
//!   [`BodyTeleported`] trace message and insert the `PortalEmission` /
//!   `PortalInputWarp` input components — exactly as the old player-specific
//!   transit system did inline, on the same frame the controller runs.
//!
//! [`BodyKinematics`]: crate::platformer_runtime::body::BodyKinematics
//! [`PortalBody`]: crate::portal::PortalBody
//! [`PortalPolicy`]: crate::portal::PortalPolicy

use bevy::prelude::*;

use crate::features::{BodyKinematics, BossConfig};
use crate::player::{PlayerEntity, PrimaryPlayer};
use crate::portal::{
    BodyTeleported, PlayerMovementIntent, PortalBody, PortalBodyTransited, PortalEmission,
    PortalInputWarp, PortalPolicy,
};

/// Movement-axis magnitude above which a held input warps on a same-wall
/// turn-around. Mirrors the old `PORTAL_INPUT_HELD_EPS` in portal core (kept in
/// sync — this is the Ambition side of the same threshold).
const PORTAL_INPUT_HELD_EPS: f32 = 0.25;
/// Seconds the [`PortalEmission`] guard protects a fresh exit. Mirrors the old
/// `PORTAL_EMISSION_TIME` constant in portal core.
const PORTAL_EMISSION_TIME: f32 = 0.18;

/// Ensure every body that transited before the unification carries the portal
/// transit opt-in. Maps Ambition identity → behavioral [`PortalPolicy`]:
///
/// - **player** (`PlayerEntity` + `PrimaryPlayer`) → `{ reorient: true,
///   carry_velocity: true }` (re-orients to the exit aperture and carries the
///   rotated velocity).
/// - **boss** (marked by `BossConfig`) → `{ reorient: false, carry_velocity:
///   false }` (floats; the old no-velocity path; facing follows the brain).
/// - **other actors** (enemies / NPCs — any remaining `BodyKinematics`) →
///   `{ reorient: false, carry_velocity: true }` (carry momentum; facing follows
///   AI).
///
/// The SET of bodies that transit must stay IDENTICAL to before: player + all
/// actors. Idempotent — only adds the marker/policy to entities lacking
/// `PortalBody`, so it is cheap to run every frame and tolerates late spawns.
pub fn ensure_portal_bodies(
    mut commands: Commands,
    bodies: Query<
        (Entity, Option<&PrimaryPlayer>, Option<&BossConfig>),
        (With<BodyKinematics>, Without<PortalBody>),
    >,
    players: Query<(), (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    for (entity, primary, boss) in &bodies {
        let policy = if primary.is_some() && players.get(entity).is_ok() {
            // Primary player: re-orients + carries velocity.
            PortalPolicy {
                reorient: true,
                carry_velocity: true,
            }
        } else if boss.is_some() {
            // Boss: floats, no velocity write, facing follows the brain.
            PortalPolicy {
                reorient: false,
                carry_velocity: false,
            }
        } else {
            // Enemies / NPCs: carry momentum, facing follows AI.
            PortalPolicy {
                reorient: false,
                carry_velocity: true,
            }
        };
        commands.entity(entity).insert((PortalBody, policy));
    }
}

/// Reproduce the PLAYER's input/trace side effects that used to live inside the
/// old `portal_transit_system`. Reads the generic core's [`PortalBodyTransited`]
/// events and, for the primary-player entity only:
///
/// - emits [`BodyTeleported`] (so the gameplay trace treats the position snap as
///   intentional and doesn't auto-dump on it),
/// - inserts the [`PortalEmission`] emergence guard (held input can't push back
///   into the exit wall for a short window), and
/// - inserts the [`PortalInputWarp`] same-wall held-input warp **iff** the
///   crossing was a `facing_flip` turn-around AND a movement input is held.
///
/// `PlayerMovementIntent` / `PortalEmission` / `PortalInputWarp` are INPUT and
/// must never be referenced by the portal core. This runs `.after(portal_transit)`
/// and `.before` the player controller so these components exist the same frame
/// the controller runs (as they did when transit inserted them inline).
pub fn portal_player_input_adapter(
    mut commands: Commands,
    intent: Option<Res<PlayerMovementIntent>>,
    mut transited: MessageReader<PortalBodyTransited>,
    mut teleported: MessageWriter<BodyTeleported>,
    players: Query<(), (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let held = intent.as_deref().map_or(Vec2::ZERO, |i| i.dir);
    for ev in transited.read() {
        // Only the primary player carries input/trace side effects; actors don't.
        if players.get(ev.body).is_err() {
            continue;
        }
        // Trace: the position snap is intentional.
        teleported.write(BodyTeleported { body: ev.body });
        // Protect the emergence so the floored exit velocity carries the body
        // out before held input can fight it.
        commands.entity(ev.body).insert(PortalEmission {
            exit_normal: ev.exit_normal,
            timer: PORTAL_EMISSION_TIME,
        });
        // Warp the held input only on the same-wall turn-around (where the warp
        // stays horizontally expressible); a floor↔wall 90° turn would rotate a
        // horizontal hold into "up", which the controller can't use.
        if ev.facing_flip && held.length() > PORTAL_INPUT_HELD_EPS {
            commands.entity(ev.body).insert(PortalInputWarp {
                n_in: ev.enter_normal,
                n_out: ev.exit_normal,
                anchor: held,
            });
        }
    }
}
