//! Ambition identity → portal-transit policy glue.
//!
//! The generic portal core ([`ambition_sandbox::portal::portal_transit`]) drives any body
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
//! [`BodyKinematics`]: ambition_sandbox::platformer_runtime::body::BodyKinematics
//! [`PortalBody`]: ambition_sandbox::portal::PortalBody
//! [`PortalPolicy`]: ambition_sandbox::portal::PortalPolicy

use bevy::prelude::*;

use ambition_sandbox::features::{BodyKinematics, BossConfig};
use ambition_sandbox::player::{PlayerEntity, PrimaryPlayer};
use ambition_sandbox::portal::{
    BodyTeleported, PlayerMovementIntent, PortalBody, PortalBodyTransited, PortalEmission,
    PortalInputWarp, PortalPolicy,
};
use ambition_sandbox::projectile::ProjectileGameplay;

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
        (
            With<BodyKinematics>,
            Without<PortalBody>,
            // Stage 19 Phase 3c-i: projectiles are NOT actors. Once player
            // projectiles become `BodyKinematics` entities (Phase 3c-ii) they
            // would otherwise be auto-tagged `PortalBody` here and swept into
            // actor portal transit. Phase 4 will opt projectiles into transit
            // explicitly with their OWN policy; until then exclude them so the
            // transiting SET stays exactly "player + actors", unchanged.
            Without<ambition_sandbox::projectile::ProjectileGameplay>,
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

/// Stage 19 Phase 4 — the projectile transit demo. Opt every in-flight
/// projectile entity into the ONE generic transit algorithm
/// ([`ambition_sandbox::portal::portal_transit`]) by giving it the [`PortalBody`] marker +
/// a free-flying [`PortalPolicy`]:
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

/// Reproduce the PLAYER's input/trace side effects that used to live inside the
/// old `portal_transit_system`. Reads the generic core's [`PortalBodyTransited`]
/// events and, for the primary-player entity only:
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
        // Warp held input only when the active portal map keeps ordinary
        // horizontal movement expressible and flips it. A floor↔wall 90° turn
        // would rotate a horizontal hold into "up", which the controller can't
        // use as ordinary movement.
        if ev.input_warp && held.length() > PORTAL_INPUT_HELD_EPS {
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
    //! Stage 19 Phase 4 — the projectile-transit DEMO, proven headless.
    //!
    //! A projectile entity (the exact shape the player / enemy spawn consumers
    //! produce: shared [`BodyKinematics`] + [`ProjectileGameplay`]) is opted into
    //! the ONE generic [`portal_transit`] core by the REAL Phase-4 adapter
    //! ([`ensure_projectile_portal_bodies`]). With a portal pair placed, a
    //! projectile fired into portal A must EMERGE from portal B with its velocity
    //! rotated by the pair transform and keep flying past B (no re-orientation —
    //! projectiles have no `ActorRoll`, `reorient: false`).
    //!
    //! Deterministic: no RNG, fixed positions/velocities, explicit `app.update()`
    //! steps. Also guards the no-regression case — a projectile nowhere near a
    //! portal stays on its straight-line path (`transit_step` → `Idle`).

    use bevy::prelude::*;

    use ambition_sandbox::portal::{
        portal_half_extent, portal_transit, PlacedPortal, PortalBody, PortalChannel, PortalGunColor,
    };
    use ambition_sandbox::projectile::{ProjectileFaction, ProjectileGameplay, ProjectileKind};

    use super::ensure_projectile_portal_bodies;

    const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::Blue);
    const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::Orange);

    use ambition_sandbox::platformer_runtime::body::BodyKinematics;

    /// A straight-flying, gravity-free projectile gameplay half (Hadouken: no
    /// bounce, no arc) so the test isolates the portal velocity rotation.
    fn straight_projectile() -> ProjectileGameplay {
        ProjectileGameplay {
            kind: ProjectileKind::Hadouken,
            faction: ProjectileFaction::Player,
            age: 0.0,
            max_lifetime: 100.0,
            gravity: 0.0,
            damage: 1,
            bounces_remaining: 0,
        }
    }

    /// Minimal app: the Phase-4 tagging adapter + the generic transit core, wired
    /// `ensure → transit` exactly as the real plugin orders them.
    fn app_with_transit() -> App {
        let mut app = App::new();
        app.add_message::<ambition_sandbox::portal::PortalBodyEntered>();
        app.add_message::<ambition_sandbox::portal::PortalBodyTransited>();
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
                .get::<ambition_sandbox::portal::PortalTransit>(proj)
                .is_none(),
            "no PortalTransit latch should be set for a projectile away from portals",
        );
    }
}
