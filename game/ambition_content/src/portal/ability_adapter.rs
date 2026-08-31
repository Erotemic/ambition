//! Ambition portal → player-ability / player-input adapters.
//!
//! Two effects that the generic portal mechanic deliberately does NOT own
//! (per the ownership boundary: the crate owns neither *input* nor *player
//! abilities*), driven off the portal-owned components the crate sets during a
//! crossing:
//!
//! - [`suppress_ledge_grab_during_transit`] — while a body carries the
//!   portal-owned [`PortalTransit`] latch, suppress the player's wall abilities
//!   (ledge-grab / cling / wall-jump / wall-climb) so they don't grab the carved
//!   aperture edges. Touches `ambition_platformer2d_core::BodyAbilities`, so it is Ambition
//!   glue, not crate core.
//! - [`warp_portal_input`] — apply the portal-owned [`PortalInputWarp`] /
//!   [`PortalEmission`] guards (both inserted by
//!   [`portal_player_input_adapter`](super::transit_body_adapter::portal_player_input_adapter)
//!   on a crossing) to the player's live movement intent. This is INPUT shaping,
//!   so it lives in Ambition; the crate just owns the marker components.
//!
//! Both read ONLY portal-owned components ([`PortalTransit`], [`PortalInputWarp`],
//! [`PortalEmission`]) plus the seat frame of whoever drives the body, so the
//! crate emits everything they need without naming the player or input.

use bevy::prelude::*;

use ambition_portal2d::pieces::portal_map_vec;
use ambition_portal2d::{PortalEmission, PortalInputWarp, PortalTransit, PortalTuning};

/// Runtime toggle for [`suppress_ledge_grab_during_transit`]. Default ON; flip it
/// off to play with ledge-grab / wall-movement INTO portals enabled (the
/// "ledge-grab through a portal" experiment — see TODO.md). Toggleable at runtime
/// (e.g. via the inspector) so both behaviors can be tried without a recompile.
///
/// This is an Ambition ability-policy toggle (the suppressed thing is a PLAYER
/// ability), so it lives with the adapter, not in the portal crate.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SuppressWallAbilitiesInPortal(pub bool);

impl Default for SuppressWallAbilitiesInPortal {
    fn default() -> Self {
        Self(true)
    }
}

/// While a body is mid-transit, suppress its wall abilities (ledge-grab,
/// cling, wall-jump, wall-climb) so it doesn't latch onto the carved aperture
/// EDGES — the carve splits the host block, and those new edges read as grabbable
/// ledges / climbable walls, so a body would cling "into" a portal and pop back
/// out the entry instead of sinking through and crossing.
///
/// BODY-GENERIC (relativity): the aperture-edge hazard is a property of transiting, not of
/// being the primary player — a possessed actor (or any wall-able actor) crossing a portal
/// needs the same guard. Gated on [`PortalTuning::suppress_wall_abilities`].
pub fn suppress_ledge_grab_during_transit(
    tuning: Res<PortalTuning>,
    mut bodies: Query<&mut ambition_platformer2d_core::BodyAbilities, With<PortalTransit>>,
) {
    if !tuning.suppress_wall_abilities {
        return;
    }
    for mut abilities in &mut bodies {
        // Equality-guard through `Mut` so an already-suppressed body doesn't
        // trip change detection every frame of a transit.
        let a = abilities.abilities;
        if a.ledge_grab || a.wall_cling || a.wall_jump || a.wall_climb {
            let a = &mut abilities.abilities;
            a.ledge_grab = false;
            a.wall_cling = false;
            a.wall_jump = false;
            a.wall_climb = false;
        }
    }
}

/// When a body's [`PortalTransit`] latch is removed (transit finished or
/// aborted), restore the four wall verbs from its authored
/// [`AbilityBase`](ambition_platformer2d_core::AbilityBase). The primary player gets
/// this for free from the per-frame F3 ability re-sync, but that sync is
/// primary-only — for every other body (a possessed actor, a wall-able enemy)
/// the suppression in [`suppress_ledge_grab_during_transit`] would otherwise be
/// permanent. Restoring from the BASE (not a saved copy) keeps this stateless;
/// if a session mask also gates one of these verbs off for the primary, the F3
/// re-sync re-applies the mask on the next frame.
pub fn restore_wall_abilities_after_transit(
    tuning: Res<PortalTuning>,
    mut removed: RemovedComponents<PortalTransit>,
    mut bodies: Query<(
        &mut ambition_platformer2d_core::BodyAbilities,
        &ambition_platformer2d_core::AbilityBase,
    )>,
) {
    if !tuning.suppress_wall_abilities {
        return;
    }
    for entity in removed.read() {
        let Ok((mut abilities, base)) = bodies.get_mut(entity) else {
            continue;
        };
        let a = &mut abilities.abilities;
        a.ledge_grab = base.abilities.ledge_grab;
        a.wall_cling = base.abilities.wall_cling;
        a.wall_jump = base.abilities.wall_jump;
        a.wall_climb = base.abilities.wall_climb;
    }
}

/// Apply the active portal input effects to the DRIVEN body's movement intent
/// (which the content input adapter mirrors to/from the Ambition `ControlFrame`
/// so the brain / movement see the adjusted axes): the same-wall held-input warp
/// (soft — drops on release or a clearly different direction) and the emergence
/// guard (held input can't push back into the exit wall while it's fresh). Both
/// are deliberately mild so portals never feel like a hard input latch.
///
/// ⭐ **PER SEAT.** Each warped body names its own [`DrivingParticipant`], and the
/// guards shape THAT seat's frame. This used to shape one global
/// `PlayerMovementIntent` — the local player's single input stream — which made
/// portal input warping primary-only: a second player crossing a portal had
/// their held movement rotated for player one, or not at all.
///
/// Reads the portal-owned [`PortalInputWarp`] / [`PortalEmission`] guards (set by
/// [`portal_player_input_adapter`](super::transit_body_adapter::portal_player_input_adapter) on a
/// crossing) and edits the seat's frame through `shape_seat_frame`.
pub fn warp_portal_input(
    time: Option<Res<ambition_time::WorldTime>>,
    mut commands: Commands,
    tuning: Res<PortalTuning>,
    latches: Option<Res<ambition_characters::control::SlotControlLatches>>,
    rollback: Option<Res<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>>,
    mut slots: ResMut<ambition_characters::control::SlotControls>,
    mut raw: ResMut<ambition_characters::control::SeatRawFrames>,
    mut bodies: Query<(
        Entity,
        &ambition_characters::control::DrivingParticipant,
        Option<&PortalInputWarp>,
        Option<&mut PortalEmission>,
    )>,
) {
    let sim_dt = time.as_deref().map_or(0.0, |t| t.sim_dt());
    for (entity, driver, warp, emission) in &mut bodies {
        if warp.is_none() && emission.is_none() {
            continue;
        }
        let slot = driver.0;
        let frame = ambition_platformer2d_actor_monolith::control::seat_frame_this_tick(
            latches.as_deref(),
            rollback.as_deref(),
            &slots,
            &raw,
            slot,
        );
        let mut dir = bevy::prelude::Vec2::new(frame.axis_x, frame.axis_y);

        // Same-wall held-input warp: a hold that survives the crossing is mapped
        // through the portal, and one that is released or clearly redirected
        // drops the guard.
        if let Some(warp) = warp {
            if dir.length() < tuning.input_held_epsilon {
                commands.entity(entity).remove::<PortalInputWarp>();
            } else if warp.anchor.length() > 0.01
                && dir.normalize_or_zero().dot(warp.anchor.normalize_or_zero())
                    < tuning.input_warp_keep_cos
            {
                commands.entity(entity).remove::<PortalInputWarp>();
            } else {
                dir = portal_map_vec(dir, warp.n_in, warp.n_out, tuning.convention.map_convention());
            }
        }

        // Emergence guard: strip held input that pushes back into the exit wall
        // while it is fresh.
        if let Some(mut emission) = emission {
            emission.timer -= sim_dt;
            if emission.timer <= 0.0 {
                commands.entity(entity).remove::<PortalEmission>();
            } else {
                let into = dir.dot(emission.exit_normal);
                if into < 0.0 {
                    dir -= into * emission.exit_normal;
                }
            }
        }

        ambition_platformer2d_actor_monolith::control::shape_seat_frame(
            latches.as_deref(),
            rollback.as_deref(),
            &mut slots,
            &mut raw,
            slot,
            |frame| {
                frame.axis_x = dir.x;
                frame.axis_y = dir.y;
            },
        );
    }
}

#[cfg(test)]
mod tests;
