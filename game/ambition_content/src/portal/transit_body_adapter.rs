//! Ambition identity → portal-transit policy glue.
//!
//! The generic portal core drives any [`BodyKinematics`] + [`PortalBody`] +
//! [`PortalPolicy`] through a placed pair without naming player, boss, enemy, or
//! projectile. This module supplies those Ambition identities.
//!
//! It tags actors and projectiles with the right transit policy, then mirrors the
//! primary player's input/trace side effects after a generic transit event so the
//! controller sees `PortalEmission` / `PortalInputWarp` on the same frame.
//!
//! [`BodyKinematics`]: ambition_actors::platformer_runtime::body::BodyKinematics
//! [`PortalBody`]: ambition_portal::PortalBody
//! [`PortalPolicy`]: ambition_portal::PortalPolicy

use bevy::prelude::*;

use ambition_actors::actor::{PlayerEntity, PrimaryPlayer};
use ambition_actors::avatar::trail::TrailContinuityBreak;
use ambition_actors::features::{BodyKinematics, BossConfig};
use ambition_portal::{
    BodyTeleported, PlayerMovementIntent, PortalBody, PortalBodyTransited, PortalEmission,
    PortalInputWarp, PortalPolicy, PortalTuning,
};
use ambition_projectiles::ProjectileGameplay;

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
        (
            With<BodyKinematics>,
            Without<PortalBody>,
            // Projectiles are not actors; a dedicated adapter opts them into transit
            // with projectile-specific policy.
            Without<ambition_projectiles::ProjectileGameplay>,
        ),
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

/// Mirror the `portal_reverses_facing` gameplay setting into the global
/// [`PortalTuning::reorient_facing`] knob each frame so toggling it in the
/// settings menu takes effect live (the portal core reads the tuning resource on
/// every transit). The gameplay setting defaults OFF — so by default the player
/// keeps the same facing through a same-wall portal turn-around — while the portal
/// crate's own default stays ON for standalone use.
///
/// Change-guarded so it only writes (and trips Bevy change detection) when the
/// user actually flips the setting.
pub fn sync_portal_reorient_from_settings(
    // Optional: headless / unit-test apps may run portal transit without the
    // settings resource. Absent → leave the portal crate's default (ON).
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut tuning: ResMut<PortalTuning>,
) {
    let Some(settings) = settings else {
        return;
    };
    let want = settings.gameplay.portal_reverses_facing;
    if tuning.reorient_facing != want {
        tuning.reorient_facing = want;
    }
}

/// Opt every in-flight projectile entity into the generic transit algorithm by
/// giving it the [`PortalBody`] marker plus a free-flying [`PortalPolicy`]:
///
/// - `reorient: false` — a projectile is not an actor; it has no `ActorRoll` and
///   no facing-to-aperture concept. Its velocity is rotated by the pair
///   transform (that is core/default, in `transit_step`), and it just keeps
///   flying out the exit.
/// - `carry_velocity: true` — write the rotated exit velocity so a fireball
///   fired into portal A emerges from portal B travelling in the mapped
///   direction (the whole point of the demo).
///
/// Projectiles are EXCLUDED from [`ensure_portal_bodies`] (which is
/// `Without<ProjectileGameplay>`, so the actor transit set is unchanged); this
/// dedicated system opts them in with their own policy. Idempotent
/// (`Without<PortalBody>`), so it is cheap to run every frame and tolerates
/// late spawns. Both pools (`PlayerProjectile` + `EnemyProjectile`) carry the
/// shared [`BodyKinematics`] + [`ProjectileGameplay`], so filtering on the
/// gameplay marker covers every projectile regardless of pool.
///
/// A projectile nowhere near a portal is unaffected: `transit_step` returns
/// `Idle`, so this is a pure no-op for the non-portal case.
pub fn ensure_projectile_portal_bodies(
    mut commands: Commands,
    projectiles: Query<
        Entity,
        (
            With<BodyKinematics>,
            With<ProjectileGameplay>,
            Without<PortalBody>,
        ),
    >,
) {
    for entity in &projectiles {
        commands.entity(entity).insert((
            PortalBody,
            PortalPolicy {
                reorient: false,
                carry_velocity: true,
            },
        ));
    }
}

/// Give every transferred body its CARRIED run momentum: the WORLD-imparted
/// part of the mapped exit velocity's run-axis component becomes
/// `BodyFlightState::carried_run` — the floor the hands-off air stop assist
/// decays toward — so a portal fling is conserved (Portal physics) while
/// ordinary jump drift keeps the tight stop-on-release feel (Hollow Knight
/// control). Runs after `portal_transit` the same frame, when
/// `BodyKinematics::vel` is already the mapped exit velocity. Actor-generic:
/// any transferred body carrying the flight cluster gets it — no
/// player-casing.
///
/// The transfer ROTATES momentum; it must not reclassify it. The
/// controller-owned run (pre-transfer run velocity minus the old carried
/// floor — walk/drift the stop assist was already braking) stays
/// controller-owned through the portal: naively flooring the WHOLE exit run
/// velocity promoted a walk-in residual into a permanent fling, and a
/// vertical ground-pair bounce then marched sideways ~residual px/s per leg
/// until it landed off the aperture and thudded dead (the momentum-kill
/// regression Jon reported as "I don't come all the way back up"). A fall's
/// gravity-earned speed is world-imparted, so a genuine fling (fall in, wall
/// out) still floors at full strength.
pub fn apply_portal_carried_momentum(
    gravity: Option<Res<ambition_actors::platformer_runtime::gravity::GravityField>>,
    mut transited: MessageReader<PortalBodyTransited>,
    mut bodies: Query<(
        &BodyKinematics,
        &mut ambition_actors::actor::BodyFlightState,
    )>,
) {
    use ambition_portal::pieces::portal_map_vec;
    let gravity_dir =
        ambition_actors::platformer_runtime::gravity::gravity_dir_or_default(gravity.as_deref());
    let side = ambition_engine_core::AccelerationFrame::new(gravity_dir).side;
    for ev in transited.read() {
        let Ok((kin, mut flight)) = bodies.get_mut(ev.body) else {
            continue;
        };
        // `kin.vel` is the mapped exit velocity; the map is an isometry, so
        // the swapped-normal map is its inverse (pinned at 45° in pieces).
        let pre_vel = portal_map_vec(kin.vel, ev.exit_normal, ev.enter_normal);
        let controller_run = pre_vel.dot(side) - flight.carried_run;
        let mapped_controller =
            portal_map_vec(controller_run * side, ev.enter_normal, ev.exit_normal);
        flight.carried_run = kin.vel.dot(side) - mapped_controller.dot(side);
    }
}

/// Complete the kernel-body half of the portal-transit authority (ADR 0024).
///
/// The portal core moves ANY `BodyKinematics` — including cluster-less
/// projectiles — so it cannot reconcile kernel body state itself. For every
/// transited body that IS a kernel body (full movement clusters + an explicit
/// `MotionModel`), run the shared transit reconciliation: departure contacts
/// invalidated, wall cling and ledge grab released, a riding momentum body
/// arrives Airborne, an attached crawler arrives detached, and the §3.1 motion
/// record collapses to the arrival point. Runs `.after(portal_transit)` in the
/// same set so the reconciled state is what the next movement tick sees.
pub fn reconcile_kernel_bodies_after_portal_transit(
    mut transited: MessageReader<PortalBodyTransited>,
    mut bodies: Query<(
        ambition_engine_core::BodyClusterQueryData,
        &mut ambition_actors::features::MotionModel,
    )>,
) {
    for ev in transited.read() {
        let Ok((mut cluster_item, mut motion_model)) = bodies.get_mut(ev.body) else {
            // A cluster-less transiting body (a projectile) has nothing to
            // reconcile.
            continue;
        };
        let mut clusters = cluster_item.as_clusters_mut();
        ambition_engine_core::movement::reconcile_transit(&mut motion_model, &mut clusters);
    }
}

/// Apply driven-body input/trace side effects after generic portal transit.
/// Reads [`PortalBodyTransited`] events and, for the CONTROLLED subject only
/// (a possessed actor while possessing, else the home avatar):
///
/// - emits [`BodyTeleported`] (so the gameplay trace treats the position snap as
///   intentional and doesn't auto-dump on it),
/// - inserts the [`PortalEmission`] emergence guard (held input can't push back
///   into the exit wall for a short window), and
/// - inserts the [`PortalInputWarp`] held-input warp **iff** this convention's
///   map flips horizontal movement and a movement input is held.
///
/// These are INPUT-feel guards, so they follow the body the local input stream
/// is driving — possess an actor and send it through a portal, and its
/// emergence is protected exactly like the home avatar's (previously it got
/// none of these and possession-through-a-portal felt wrong). Autonomous
/// actors still carry no input guards; their brains hold no input to warp.
///
/// `PlayerMovementIntent` / `PortalEmission` / `PortalInputWarp` are INPUT and
/// must never be referenced by the portal core. This runs `.after(portal_transit)`
/// and `.before` the player controller so these components exist the same frame
/// the controller runs (as they did when transit inserted them inline).
pub fn portal_player_input_adapter(
    mut commands: Commands,
    intent: Option<Res<PlayerMovementIntent>>,
    tuning: Res<PortalTuning>,
    mut transited: MessageReader<PortalBodyTransited>,
    mut teleported: MessageWriter<BodyTeleported>,
    mut trail_breaks: MessageWriter<TrailContinuityBreak>,
    controlled: Option<Res<ambition_platformer_primitives::markers::ControlledSubject>>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let held = intent.as_deref().map_or(Vec2::ZERO, |i| i.dir);
    let subject = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok());
    for ev in transited.read() {
        // Only the DRIVEN body carries input/trace side effects; autonomous
        // actors don't.
        if Some(ev.body) != subject {
            continue;
        }
        // Trace: the position snap is intentional.
        teleported.write(BodyTeleported { body: ev.body });
        // Trail: the body remained continuous in the quotient space, but its
        // ordinary world coordinates snapped. Emit the neutral trail seam so
        // the trail chunks instead of drawing a fake line across the room.
        trail_breaks.write(TrailContinuityBreak {
            body: ev.body,
            resume_at: ev.exit_pos,
        });
        // Protect the emergence so the floored exit velocity carries the body
        // out before held input can fight it.
        commands.entity(ev.body).insert(PortalEmission {
            exit_normal: ev.exit_normal,
            timer: tuning.emission_time_s,
        });
        // Warp held input only when the active portal map keeps ordinary
        // horizontal movement expressible and flips it. A floor↔wall 90° turn
        // would rotate a horizontal hold into "up", which the controller can't
        // use as ordinary movement.
        if ev.input_warp && held.length() > tuning.input_held_epsilon {
            commands.entity(ev.body).insert(PortalInputWarp {
                n_in: ev.enter_normal,
                n_out: ev.exit_normal,
                anchor: held,
            });
        }
    }
}

#[cfg(test)]
mod projectile_transit_tests;
