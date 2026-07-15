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
use ambition_platformer_primitives::markers::ControlledSubject;
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

/// While a body is mid-transit, suppress its wall abilities (ledge-grab,
/// cling, wall-jump, wall-climb) so it doesn't latch onto the carved aperture
/// EDGES — the carve splits the host block, and those new edges read as grabbable
/// ledges / climbable walls, so a body would cling "into" a portal and pop back
/// out the entry instead of sinking through and crossing.
///
/// BODY-GENERIC (relativity): the aperture-edge hazard is a property of
/// transiting, not of being the primary player — a possessed actor (or any
/// wall-able actor) crossing a portal needs the same guard. Suppression
/// re-applies every frame while the [`PortalTransit`] latch is present (robust
/// against the primary player's per-frame F3 ability re-sync), and
/// [`restore_wall_abilities_after_transit`] puts the verbs back from the body's
/// own authored [`AbilityBase`](ambition_engine_core::AbilityBase) when the
/// latch is removed — bodies outside the F3 re-sync (everything that isn't the
/// primary player) would otherwise stay stripped forever.
/// Gated on [`PortalTuning::suppress_wall_abilities`]. Runs before the movement
/// integration.
///
/// Reads the portal-owned [`PortalTransit`] latch and writes the Ambition
/// `BodyAbilities` — so it is content glue, not portal core. Moved out of the
/// portal crate (Stage 19 Phase 5a); identical-sim.
pub fn suppress_ledge_grab_during_transit(
    tuning: Res<PortalTuning>,
    mut bodies: Query<&mut ambition_actors::actor::BodyAbilities, With<PortalTransit>>,
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
/// [`AbilityBase`](ambition_engine_core::AbilityBase). The primary player gets
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
        &mut ambition_actors::actor::BodyAbilities,
        &ambition_engine_core::AbilityBase,
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
/// The body whose guards shape the input is the CONTROLLED subject (a possessed
/// actor while possessing, else the home avatar) — `PlayerMovementIntent` is the
/// local player's one input stream, and the guards only mean anything on the
/// body that stream is driving. Same resolution as the portal-gun use path
/// (`portal_input_adapter_system`).
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
    controlled: Option<Res<ControlledSubject>>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
    mut bodies: Query<(
        Entity,
        Option<&PortalInputWarp>,
        Option<&mut PortalEmission>,
    )>,
) {
    let Some(mut intent) = intent else {
        return;
    };
    let subject = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok());
    let Some((entity, warp, emission)) = subject.and_then(|s| bodies.get_mut(s).ok()) else {
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
