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
mod tests {
    //! The portal input-warp + wall-ability suppression adapters (Stage 19
    //! Phase 5a — moved out of the portal crate because they shape INPUT /
    //! mutate the PLAYER's abilities, neither of which the crate owns). These
    //! drive the REAL adapters + the portal-owned marker components.
    use bevy::prelude::*;

    use ambition_actors::actor::{PlayerEntity, PrimaryPlayer};
    use ambition_input::ControlFrame;
    use ambition_portal::{
        PlayerMovementIntent, PortalChannel, PortalEmission, PortalGunColor, PortalInputWarp,
        PortalTransit, PortalTuning,
    };

    use super::{suppress_ledge_grab_during_transit, warp_portal_input};

    const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);

    #[test]
    fn portal_input_warp_transforms_held_input_then_clears() {
        use crate::portal::{apply_movement_intent_to_control, sync_movement_intent_from_control};
        let mut app = App::new();
        app.insert_resource(ControlFrame::default());
        app.init_resource::<PlayerMovementIntent>();
        app.init_resource::<PortalTuning>();
        // The content adapter brackets the core warp: mirror ControlFrame -> intent
        // before the warp, and the warped intent -> ControlFrame after, so this
        // exercises the full content+core chain on the ControlFrame surface exactly
        // as the game does.
        app.add_systems(
            Update,
            (
                sync_movement_intent_from_control,
                warp_portal_input,
                apply_movement_intent_to_control,
            )
                .chain(),
        );
        // A 180° warp (a same-wall pair). Player holds RIGHT (anchor right).
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PortalInputWarp {
                    n_in: Vec2::new(-1.0, 0.0),
                    n_out: Vec2::new(-1.0, 0.0),
                    anchor: Vec2::new(1.0, 0.0),
                },
            ))
            .id();

        // Still holding right → input is warped to LEFT (keeps you moving out).
        app.world_mut().resource_mut::<ControlFrame>().axis_x = 1.0;
        app.update();
        assert!(
            app.world().resource::<ControlFrame>().axis_x < -0.5,
            "held right is warped to left while the warp is active"
        );
        assert!(
            app.world().get::<PortalInputWarp>(player).is_some(),
            "warp persists while held"
        );

        // Release movement → warp drops, input passes through untouched next frame.
        app.world_mut().resource_mut::<ControlFrame>().axis_x = 0.0;
        app.update();
        assert!(
            app.world().get::<PortalInputWarp>(player).is_none(),
            "release drops the warp"
        );

        // Re-arm, then press a clearly different direction (left) → warp drops.
        app.world_mut().entity_mut(player).insert(PortalInputWarp {
            n_in: Vec2::new(-1.0, 0.0),
            n_out: Vec2::new(-1.0, 0.0),
            anchor: Vec2::new(1.0, 0.0),
        });
        app.world_mut().resource_mut::<ControlFrame>().axis_x = -1.0;
        app.update();
        assert!(
            app.world().get::<PortalInputWarp>(player).is_none(),
            "a clearly different direction drops the warp"
        );
    }

    #[test]
    fn wall_ability_suppression_reapplies_every_frame_against_the_loadout_reset() {
        use ambition_actors::actor::BodyAbilities;
        let mut app = App::new();
        app.init_resource::<PortalTuning>();
        // Stand in for the per-frame loadout reset that clobbered the old
        // save-once suppression: re-enable ledge_grab BEFORE the suppressor runs.
        fn reenable_ledge_grab(mut q: Query<&mut BodyAbilities>) {
            for mut a in &mut q {
                a.abilities.ledge_grab = true;
            }
        }
        app.add_systems(
            Update,
            (reenable_ledge_grab, suppress_ledge_grab_during_transit).chain(),
        );
        let player = app
            .world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, BodyAbilities::default()))
            .id();
        app.world_mut()
            .get_mut::<BodyAbilities>(player)
            .unwrap()
            .abilities
            .ledge_grab = true;

        // Not transiting: the reset wins, ledge_grab stays enabled.
        app.update();
        assert!(
            app.world()
                .get::<BodyAbilities>(player)
                .unwrap()
                .abilities
                .ledge_grab
        );

        // Transiting: even though the reset re-enables it first, the suppressor
        // re-applies every frame, so it stays disabled across MANY frames.
        app.world_mut().entity_mut(player).insert(PortalTransit {
            straddling: BLUE,
            crossed: false,
        });
        for _ in 0..5 {
            app.update();
            assert!(
                !app.world()
                    .get::<BodyAbilities>(player)
                    .unwrap()
                    .abilities
                    .ledge_grab,
                "ledge_grab must stay suppressed every frame while transiting"
            );
        }

        // Transit ends: the per-frame reset restores it (no save/restore needed).
        app.world_mut().entity_mut(player).remove::<PortalTransit>();
        app.update();
        assert!(
            app.world()
                .get::<BodyAbilities>(player)
                .unwrap()
                .abilities
                .ledge_grab
        );
    }

    #[test]
    fn emission_guard_strips_input_pushing_back_into_the_exit_wall() {
        use crate::portal::{apply_movement_intent_to_control, sync_movement_intent_from_control};
        let mut app = App::new();
        app.insert_resource(ControlFrame::default());
        app.init_resource::<PlayerMovementIntent>();
        app.init_resource::<PortalTuning>();
        app.add_systems(
            Update,
            (
                sync_movement_intent_from_control,
                warp_portal_input,
                apply_movement_intent_to_control,
            )
                .chain(),
        );
        // Emerging from a right-wall portal — exit_normal points LEFT (into room).
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PortalEmission {
                    exit_normal: Vec2::new(-1.0, 0.0),
                    timer: 1.0,
                },
            ))
            .id();
        // Holding RIGHT (back into the wall) is stripped so physics carries you out.
        app.world_mut().resource_mut::<ControlFrame>().axis_x = 1.0;
        app.update();
        assert!(
            app.world().resource::<ControlFrame>().axis_x.abs() < 0.01,
            "input pushing back into the exit wall is stripped during emergence"
        );
        // Holding LEFT (the emergence direction) passes through untouched.
        app.world_mut().resource_mut::<ControlFrame>().axis_x = -1.0;
        app.update();
        assert!(
            app.world().resource::<ControlFrame>().axis_x < -0.5,
            "input in the emergence direction is preserved"
        );
        let _ = player;
    }
}
