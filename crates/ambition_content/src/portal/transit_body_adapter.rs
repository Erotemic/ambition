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
//! [`BodyKinematics`]: ambition_gameplay_core::platformer_runtime::body::BodyKinematics
//! [`PortalBody`]: ambition_gameplay_core::portal::PortalBody
//! [`PortalPolicy`]: ambition_gameplay_core::portal::PortalPolicy

use bevy::prelude::*;

use ambition_gameplay_core::actor::{PlayerEntity, PrimaryPlayer};
use ambition_gameplay_core::features::{BodyKinematics, BossConfig};
use ambition_gameplay_core::player::trail::TrailContinuityBreak;
use ambition_gameplay_core::portal::{
    BodyTeleported, PlayerMovementIntent, PortalBody, PortalBodyTransited, PortalEmission,
    PortalInputWarp, PortalPolicy, PortalTuning,
};
use ambition_gameplay_core::projectile::ProjectileGameplay;

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
            Without<ambition_gameplay_core::projectile::ProjectileGameplay>,
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
    gravity: Option<Res<ambition_gameplay_core::platformer_runtime::gravity::GravityField>>,
    mut transited: MessageReader<PortalBodyTransited>,
    mut bodies: Query<(
        &BodyKinematics,
        &mut ambition_gameplay_core::actor::BodyFlightState,
    )>,
) {
    use ambition_gameplay_core::portal::pieces::portal_map_vec;
    let gravity_dir = ambition_gameplay_core::platformer_runtime::gravity::gravity_dir_or_default(
        gravity.as_deref(),
    );
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

/// Apply player-only input/trace side effects after generic portal transit.
/// Reads [`PortalBodyTransited`] events and, for the primary-player entity only:
///
/// - emits [`BodyTeleported`] (so the gameplay trace treats the position snap as
///   intentional and doesn't auto-dump on it),
/// - inserts the [`PortalEmission`] emergence guard (held input can't push back
///   into the exit wall for a short window), and
/// - inserts the [`PortalInputWarp`] held-input warp **iff** this convention's
///   map flips horizontal movement and a movement input is held.
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
mod projectile_transit_tests {
    //! Headless projectile-transit tests for the generic portal core plus the real
    //! projectile adapter. A projectile near a pair should emerge with rotated
    //! velocity; one nowhere near a portal should keep its straight-line path.

    use bevy::prelude::*;

    use ambition_gameplay_core::portal::{
        portal_half_extent, portal_transit, PlacedPortal, PortalBody, PortalChannel, PortalGunColor,
    };
    use ambition_gameplay_core::projectile::ProjectileGameplay;

    use super::ensure_projectile_portal_bodies;

    const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
    const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::ORANGE);

    use ambition_gameplay_core::platformer_runtime::body::BodyKinematics;

    /// A straight-flying, gravity-free projectile gameplay half (Hadouken: no
    /// bounce, no arc) so the test isolates the portal velocity rotation.
    fn straight_projectile() -> ProjectileGameplay {
        ProjectileGameplay {
            age: 0.0,
            max_lifetime: 100.0,
            gravity: 0.0,
            damage: 1,
            bounces_remaining: 0,
            world_hit: ambition_gameplay_core::projectile::WorldHitPolicy::ExpireOnContact,
        }
    }

    /// Minimal app: projectile tagging adapter + generic transit core, wired
    /// `ensure → transit` as in the real plugin.
    fn app_with_transit() -> App {
        let mut app = App::new();
        app.add_message::<ambition_gameplay_core::portal::PortalBodyEntered>();
        app.add_message::<ambition_gameplay_core::portal::PortalBodyTransited>();
        app.init_resource::<ambition_gameplay_core::portal::PortalTuning>();
        app.add_systems(
            Update,
            (ensure_projectile_portal_bodies, portal_transit).chain(),
        );
        app
    }

    /// Place a left-wall portal (normal +x) and a right-wall portal (normal -x),
    /// the same pair the actor transit unit test uses.
    fn place_wall_pair(app: &mut App) {
        app.world_mut().spawn(PlacedPortal {
            channel: BLUE,
            pos: Vec2::new(20.0, 200.0),
            normal: Vec2::new(1.0, 0.0),
            half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
        });
        app.world_mut().spawn(PlacedPortal {
            channel: ORANGE,
            pos: Vec2::new(380.0, 200.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: portal_half_extent(Vec2::new(1.0, 0.0)),
        });
    }

    #[test]
    fn projectile_fired_into_portal_a_emerges_from_portal_b_with_rotated_velocity() {
        let mut app = app_with_transit();
        place_wall_pair(&mut app);

        // A small projectile at the blue (left-wall) portal, flying INTO it
        // (-x, toward the +x-normal face). Speed 400 px/s — well above the
        // MIN_EXIT_SPEED floor (220) so the assertion measures the pure rotation.
        let proj = app
            .world_mut()
            .spawn((
                BodyKinematics {
                    pos: Vec2::new(20.0, 200.0),
                    vel: Vec2::new(-400.0, 0.0),
                    size: Vec2::new(8.0, 8.0),
                    facing: -1.0,
                },
                straight_projectile(),
            ))
            .id();

        // Frame 1 tags + begins (leading edge in the opening); frame 2 transfers
        // (the centroid is already on the plane) — same two-frame aperture cadence
        // the actor transit test relies on.
        app.update();
        // The adapter must have opted the projectile in.
        assert!(
            app.world().get::<PortalBody>(proj).is_some(),
            "ensure_projectile_portal_bodies must tag the projectile PortalBody",
        );
        app.update();

        let kin = app.world().get::<BodyKinematics>(proj).unwrap();
        // Emerged from the orange portal (x=380, normal -x): a transited body pops
        // out clear of the exit, so it sits just inside the room on the far side.
        assert!(
            kin.pos.x > 300.0,
            "projectile should emerge from the orange portal on the far side, pos={:?}",
            kin.pos,
        );
        // Velocity rotated by the pair transform: the body emerges travelling ALONG
        // the EXIT normal (orange faces -x, into the room), so it flies out of B and
        // keeps going — exactly the demo claim. (Entry normal +x → exit normal -x is
        // the wall↔wall 180° map, which reverses the horizontal component.)
        assert!(
            kin.vel.x < 0.0,
            "exit velocity must be rotated to travel along the orange normal (-x), vel={:?}",
            kin.vel,
        );
        // Speed preserved by the rotation (400 px/s is above the MIN_EXIT_SPEED
        // floor, so no flooring masks it).
        assert!(
            (kin.vel.length() - 400.0).abs() < 1.0,
            "the rotation preserves speed (~400 px/s), got {:?}",
            kin.vel,
        );
        // It KEEPS flying past B (not stalled in the aperture): the exit speed is
        // well above zero along the emergence direction.
        assert!(
            kin.vel.length() > 100.0,
            "the projectile keeps flying out of B, vel={:?}",
            kin.vel,
        );
        // No re-orientation: facing is unchanged (reorient:false for projectiles).
        assert_eq!(
            kin.facing, -1.0,
            "a projectile is not re-oriented by transit (reorient:false), facing={}",
            kin.facing,
        );
    }

    #[test]
    fn projectile_nowhere_near_a_portal_flies_straight_through() {
        // No-regression guard: with a portal pair placed but the projectile far
        // from both, transit is a pure no-op and the body keeps its velocity.
        let mut app = app_with_transit();
        place_wall_pair(&mut app);

        let proj = app
            .world_mut()
            .spawn((
                BodyKinematics {
                    pos: Vec2::new(200.0, 50.0), // far from both wall portals
                    vel: Vec2::new(150.0, 0.0),
                    size: Vec2::new(8.0, 8.0),
                    facing: 1.0,
                },
                straight_projectile(),
            ))
            .id();

        app.update();
        app.update();

        let kin = app.world().get::<BodyKinematics>(proj).unwrap();
        // transit_step → Idle: velocity untouched (portal_transit does NOT
        // integrate motion — that is the Combat-set step system's job — so the
        // body stays exactly where it was spawned with its velocity intact).
        assert_eq!(
            kin.vel,
            Vec2::new(150.0, 0.0),
            "a projectile away from any portal must not be touched by transit, vel={:?}",
            kin.vel,
        );
        assert!(
            app.world()
                .get::<ambition_gameplay_core::portal::PortalTransit>(proj)
                .is_none(),
            "no PortalTransit latch should be set for a projectile away from portals",
        );
    }
}
