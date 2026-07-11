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
//!   aperture edges. Touches `ambition_actors::actor::BodyAbilities`, so it is Ambition
//!   glue, not crate core.
//! - [`warp_portal_input`] — apply the portal-owned [`PortalInputWarp`] /
//!   [`PortalEmission`] guards (both inserted by
//!   [`portal_player_input_adapter`](super::transit_body_adapter::portal_player_input_adapter)
//!   on a crossing) to the player's live movement intent. This is INPUT shaping,
//!   so it lives in Ambition; the crate just owns the marker components.
//!
//! Both read ONLY portal-owned components ([`PortalTransit`], [`PortalInputWarp`],
//! [`PortalEmission`]) + the content-agnostic [`PlayerMovementIntent`] seam, so
//! the crate emits everything they need without naming the player or input.

use bevy::prelude::*;

use ambition_actors::actor::{PlayerEntity, PrimaryPlayer};
use ambition_portal::pieces::portal_map_vec;
use ambition_portal::{
    PlayerMovementIntent, PortalEmission, PortalInputWarp, PortalTransit, PortalTuning,
};

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

/// While the player is mid-transit, suppress the wall abilities (ledge-grab,
/// cling, wall-jump, wall-climb) so they don't latch onto the carved aperture
/// EDGES — the carve splits the host block, and those new edges read as grabbable
/// ledges / climbable walls, so you'd cling "into" a portal and pop back out the
/// entry instead of sinking through and crossing.
///
/// IMPORTANT — this must re-apply EVERY frame, not set-once. `BodyAbilities` is
/// wholesale-reset to the editable loadout every frame
/// (`sync_live_ability_edits_clusters`: `abilities.abilities = desired`), so a
/// save-once/restore-on-exit pattern is clobbered after a single frame (that was
/// the "disable didn't work" bug). Re-applying each frame is robust against that
/// reset, AND needs no save/restore — when transit ends, the per-frame reset
/// restores the loadout automatically. (The wider structural smell — transient
/// ability mods fighting a per-frame wholesale reset — is noted in TODO.md.)
/// Gated on [`PortalTuning::suppress_wall_abilities`]. Runs before the movement
/// integration.
///
/// Reads the portal-owned [`PortalTransit`] latch and writes the Ambition
/// `BodyAbilities` — so it is content glue, not portal core. Moved out of the
/// portal crate (Stage 19 Phase 5a); identical-sim.
pub fn suppress_ledge_grab_during_transit(
    tuning: Res<PortalTuning>,
    mut players: Query<
        (
            &mut ambition_actors::actor::BodyAbilities,
            Option<&PortalTransit>,
        ),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
) {
    if !tuning.suppress_wall_abilities {
        return;
    }
    for (mut abilities, transiting) in &mut players {
        if transiting.is_some() {
            let a = &mut abilities.abilities;
            a.ledge_grab = false;
            a.wall_cling = false;
            a.wall_jump = false;
            a.wall_climb = false;
        }
    }
}

/// Apply the active portal input effects to the player's movement intent (which
/// the content input adapter mirrors to/from the Ambition `ControlFrame` so the
/// brain / movement see the adjusted axes): the same-wall held-input warp (soft —
/// drops on release or a clearly different direction) and the emergence guard
/// (held input can't push back into the exit wall while it's fresh). Both are
/// deliberately mild so portals never feel like a hard input latch.
///
/// Reads the portal-owned [`PortalInputWarp`] / [`PortalEmission`] guards (set by
/// [`portal_player_input_adapter`](super::transit_body_adapter::portal_player_input_adapter)
/// on a crossing) and MUTATES the content-agnostic [`PlayerMovementIntent`] (the
/// live movement axis for this frame), never the Ambition input type. The content
/// adapter
/// (`sync_movement_intent_from_control` / `apply_movement_intent_to_control`)
/// brackets this system to copy `ControlFrame` axes into the intent before it runs
/// and back out afterward, so the timing and result are byte-identical to mutating
/// `ControlFrame` directly. This is INPUT shaping, so it lives in Ambition (moved
/// out of the portal crate, Stage 19 Phase 5a); identical-sim.
pub fn warp_portal_input(
    time: Option<Res<ambition_time::WorldTime>>,
    mut commands: Commands,
    intent: Option<ResMut<PlayerMovementIntent>>,
    tuning: Res<PortalTuning>,
    mut player: Query<
        (
            Entity,
            Option<&PortalInputWarp>,
            Option<&mut PortalEmission>,
        ),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
) {
    let Some(mut intent) = intent else {
        return;
    };
    let Ok((entity, warp, emission)) = player.single_mut() else {
        return;
    };

    // --- Same-wall held-input warp ---
    if let Some(warp) = warp {
        let raw = intent.dir;
        if raw.length() < tuning.input_held_epsilon {
            commands.entity(entity).remove::<PortalInputWarp>();
        } else if warp.anchor.length() > 0.01
            && raw.normalize_or_zero().dot(warp.anchor.normalize_or_zero())
                < tuning.input_warp_keep_cos
        {
            commands.entity(entity).remove::<PortalInputWarp>();
        } else {
            intent.dir = portal_map_vec(raw, warp.n_in, warp.n_out);
        }
    }

    // --- Emergence guard: strip any held input that pushes back into the wall ---
    if let Some(mut emission) = emission {
        emission.timer -= time.as_deref().map_or(0.0, |t| t.sim_dt());
        if emission.timer <= 0.0 {
            commands.entity(entity).remove::<PortalEmission>();
        } else {
            let raw = intent.dir;
            let into = raw.dot(emission.exit_normal); // < 0 = pushing into the wall
            if into < 0.0 {
                intent.dir = raw - into * emission.exit_normal;
            }
        }
    }
}

#[cfg(test)]
mod tests;
