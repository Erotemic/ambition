//! THE per-body frame resolution phase (ADR 0024 frame law).
//!
//! [`resolve_body_motion_frames`] publishes every integrated body's
//! [`ResolvedMotionFrame`] exactly once per sim tick, after the environment's
//! zone snapshot ([`super::GravitySet::ZoneSnapshot`]) and before
//! `Platformer2dSimulationPhase::CoreSimulation` — so controller interpretation (the player brain
//! in `PlayerInput`, actor/possessed brains in `WorldPrep`), body integration,
//! and every combat/ability consumer read the SAME value for the tick.
//!
//! One system, one composition rule ([`FrameEnv::resolve`]), three archetype
//! queries — player bodies (primary + clones), actors, and bosses differ only in
//! where their authored gravity response lives, never in how the frame is
//! composed. No driver, brain, or combat system resolves a frame itself.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::frame_env::{FrameEnv, ResolvedMotionFrame};

use ambition_platformer2d_core::body_clusters::ActorSurfaceState;
use ambition_platformer2d_shared_tangle::gravity::{BaseGravity, GravityField};
use ambition_boss_encounter::BossConfig;
use ambition_combat::actor_tuning::ActorConfig;

/// Resolve and publish the frame for every integrated body.
///
/// - Player bodies (primary, clones, demo avatars): the authored gravity
///   response is the live movement tuning's `gravity`.
/// - Actors and bosses (both carry the unified actor cluster): the response
///   is the body's gravity × `surface.gravity_scale` — an aerial body's 0
///   scale is the zero-acceleration-with-retained-orientation case. The
///   body's gravity is its `AuthoredMovementTuning`'s when it has one, else
///   `config.tuning.movement.gravity`.
/// - A HELD body gets the same zero response: one whose pose another
///   authority owns ([`PoseOwnedExternally`]: a rider in the saddle, a
///   conducted boss) or a captive ([`CapturedBy`]). The constraint moves it, so
///   gravity has nothing to move, and its authored scale is not written to say
///   so — which is why no hold keeps a copy to restore.
///
/// - A DEAD body falls by all of its gravity, whatever its authored scale:
///   see `gravity_scale_of`.
///
/// - Every arm's response then converts to the body's material scale
///   ([`FrameEnv::resolve_body`], SR1): a body at half size falls by half the
///   gravity, so it crosses its own height in the same time and its jump,
///   whose speed the kernel converts the same way, peaks at the same moment
///   at half the height. World force zones and carried acceleration do not
///   convert: they are world accelerations.
///
/// [`PoseOwnedExternally`]: ambition_platformer2d_core::PoseOwnedExternally
/// [`CapturedBy`]: ambition_combat::capture::CapturedBy
pub fn resolve_body_motion_frames(
    env: FrameEnv,
    // A zone acts only on the bodies of its own live room.
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    tuning: Res<ambition_platformer2d_core::ActiveMovementTuning>,
    mut players: Query<
        (
            Entity,
            &ambition_platformer2d_core::BodyKinematics,
            Option<&ambition_platformer2d_shared_tangle::frame_env::CarriedAcceleration>,
            &mut ResolvedMotionFrame,
        ),
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
    mut actors: Query<
        (
            Entity,
            &ambition_platformer2d_core::BodyKinematics,
            Option<&ambition_platformer2d_shared_tangle::frame_env::CarriedAcceleration>,
            &ActorConfig,
            Option<&ambition_platformer2d_core::AuthoredMovementTuning>,
            &ActorSurfaceState,
            // `None`: a body that cannot die, so it is always alive.
            Option<&ambition_characters::actor::BodyHealth>,
            Has<ambition_platformer2d_core::PoseOwnedExternally>,
            Has<ambition_combat::capture::CapturedBy>,
            &mut ResolvedMotionFrame,
        ),
        Without<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
) {
    // ⚠ THE PLAYER ARM DOES NOT MULTIPLY BY `surface.gravity_scale`, AND THAT IS
    // NOT AN OVERSIGHT — it is why the two arms exist. MEASURED 2026-09-05, after
    // the fighter session read this as a hazard: a player entity has NO
    // `ActorSurfaceState` at all. `PlayerSimulationBundle` does not carry one and
    // no avatar or ability path inserts one, so there is no scale here to apply.
    //
    // ⇒ Every writer of `gravity_scale` — the body seed — targets bodies that DO have the component, and those match the
    // `Without<PlayerEntity>` arm below, where the scale IS applied. Possession
    // does not change that: it queries `Without<PlayerEntity>` and leaves the
    // marker on the player box, so a possessed body keeps resolving as an actor.
    //
    // ⛔ IF A PLAYER EVER GAINS `ActorSurfaceState`, THIS ARM SILENTLY IGNORES IT,
    // and NOTHING HERE WILL TELL YOU — said plainly because I tried to guard it
    // and the guards I could write were worse than the gap. A runtime test can
    // only spawn a bare marker (that tests Bevy, not this repo); the real bundle's
    // helpers are private to `avatar/bundles.rs`; and an absence contract greps a
    // single token, which cannot express "these two things must not co-occur"
    // without banning the component's legitimate uses. ⇒ The honest artifact is
    // this measurement, dated, with the mechanism — not a green test that could
    // never go red.
    let player_response = tuning.gravity;
    for (entity, kin, carried, mut resolved) in &mut players {
        resolved.publish_resolved_frame(env.resolve_body(kin, || live.of(entity), player_response, carried.copied().unwrap_or_default()));
    }
    for (entity, kin, carried, config, authored, surface, health, posed, captive, mut resolved) in &mut actors {
        let response = if posed || captive {
            0.0
        } else {
            // The gravity this body is integrated against is the one its
            // authored feel states, when it states one: the same choice the
            // integrator makes for every other number of the feel
            // (`movement_law_of`). Reading only the config here gave every
            // match seat the actor config's gravity whatever its body said.
            authored.map_or(config.tuning.movement.gravity, |a| a.0.gravity)
                * gravity_scale_of(surface, health.is_none_or(|health| health.alive()))
        };
        resolved.publish_resolved_frame(env.resolve_body(kin, || live.of(entity), response, carried.copied().unwrap_or_default()));
    }
}

/// The part of its gravity that a body falls by: its authored
/// `ActorSurfaceState::gravity_scale` while it is alive (an aerial body: 0),
/// and all of it after it dies.
///
/// ⭐ THE SIMPLE CORPSE FALL (Jon, 2026-10-23: "when flying things die they
/// fall up. They should fall down. at normal gravity"). A dead body has no
/// will: the actor integrator moves it with a neutral control frame and turns
/// its flight off. This gives it its weight again, so a flyer, a hoverer or
/// any other body falls under the normal gravity of its room, and a death blow
/// that sends it up only delays the fall. The rule is the same for every body
/// kind. A revive gives the authored scale back, because this function reads
/// the scale and never writes it.
///
/// A ragdoll can replace this rule later. A device with low power can keep it.
pub fn gravity_scale_of(surface: &ActorSurfaceState, alive: bool) -> f32 {
    if alive {
        surface.gravity_scale
    } else {
        1.0
    }
}

/// Compile-time reminder that bosses resolve through the actor query above:
/// a boss carries the unified actor cluster (`ActorConfig` + surface), so the
/// `Without<PlayerEntity>` query matches it — no third arm exists.
#[allow(dead_code)]
fn bosses_resolve_through_the_actor_arm(_: &BossConfig) {}

#[cfg(test)]
mod tests {
    use super::*;

    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_shared_tangle::frame_env::{collect_force_zones, ForceZones};
    use ambition_platformer2d_shared_tangle::gravity::{
        collect_gravity_zones, BaseGravity, GravityField, GravityZone, GravityZones,
    };

    fn resolver_app() -> App {
        let mut app = App::new();
        app.init_resource::<GravityField>();
        app.init_resource::<BaseGravity>();
        app.init_resource::<GravityZones>();
        app.init_resource::<ForceZones>();
        app.init_resource::<ambition_platformer2d_core::ActiveMovementTuning>();
        app.add_systems(
            Update,
            (
                collect_gravity_zones,
                collect_force_zones,
                resolve_body_motion_frames,
                resolve_active_gravity,
            )
                .chain(),
        );
        app
    }

    fn spawn_player(app: &mut App, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::markers::PlayerEntity,
                ambition_platformer2d_core::BodyKinematics { material: Default::default(),
                    pos,
                    vel: ae::Vec2::ZERO,
                    canonical_size: ae::Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                ResolvedMotionFrame::default(),
            ))
            .id()
    }

    fn frame_of(app: &App, body: Entity) -> ae::MotionFrame {
        app.world()
            .get::<ResolvedMotionFrame>(body)
            .expect("integrated bodies carry a resolved frame")
            .get()
    }

    #[test]
    fn a_zone_straddling_body_resolves_by_overlap_for_every_consumer() {
        let mut app = resolver_app();
        app.world_mut().spawn(GravityZone {
            aabb: ae::Aabb::new(ae::Vec2::new(200.0, 0.0), ae::Vec2::new(30.0, 60.0)),
            dir: ae::Vec2::new(0.0, -1.0),
        });
        // The body's CENTER is outside the zone; its AABB overlaps it.
        let body = spawn_player(&mut app, ae::Vec2::new(160.0, 0.0));
        app.update();
        let frame = frame_of(&app, body);
        assert_eq!(
            frame.down(),
            ae::Vec2::new(0.0, -1.0),
            "the one published frame uses the body-overlap rule; a consumer \
             cannot see a different (center-point) frame because none exists"
        );
    }

    /// OW1 (customer 2): two live rooms, and a sideways column in the first.
    /// Alice in the first room and Bob in the second stand at the same place:
    /// Alice is turned and Bob stands under the ambient. Control: with Bob's
    /// room unknown (no stamp, two rooms live) he feels the column as before.
    #[test]
    fn a_gravity_zone_turns_only_the_bodies_of_its_own_live_room() {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            InRoomInstance, LiveRoomInstance, RoomInstanceRoot,
        };
        let mut app = resolver_app();
        let first = LiveRoomInstance::ACTIVATION;
        let second = first.next();
        for room in [first, second] {
            app.world_mut().spawn((RoomInstanceRoot, room));
        }
        app.world_mut().spawn((
            GravityZone {
                aabb: ae::Aabb::new(ae::Vec2::new(300.0, 0.0), ae::Vec2::new(40.0, 120.0)),
                dir: ae::Vec2::new(1.0, 0.0),
            },
            InRoomInstance(first),
        ));
        let here = ae::Vec2::new(300.0, 0.0);
        let alice = spawn_player(&mut app, here);
        app.world_mut().entity_mut(alice).insert(InRoomInstance(first));
        let bob = spawn_player(&mut app, here);
        app.world_mut().entity_mut(bob).insert(InRoomInstance(second));
        let unknown = spawn_player(&mut app, here);
        app.update();
        assert_eq!(frame_of(&app, alice).down(), ae::Vec2::new(1.0, 0.0), "Alice, in the column's room");
        assert_eq!(frame_of(&app, bob).down(), ae::Vec2::new(0.0, 1.0), "Bob, in the other live room");
        assert_eq!(
            frame_of(&app, unknown).down(),
            ae::Vec2::new(1.0, 0.0),
            "a body whose room cannot be told feels the column, as before"
        );
    }

    #[test]
    fn two_bodies_in_different_fields_each_get_their_own_frame() {
        let mut app = resolver_app();
        app.world_mut().spawn(GravityZone {
            aabb: ae::Aabb::new(ae::Vec2::new(300.0, 0.0), ae::Vec2::new(40.0, 120.0)),
            dir: ae::Vec2::new(1.0, 0.0),
        });
        let inside = spawn_player(&mut app, ae::Vec2::new(300.0, 0.0));
        let outside = spawn_player(&mut app, ae::Vec2::new(0.0, 0.0));
        app.update();
        assert_eq!(frame_of(&app, inside).down(), ae::Vec2::new(1.0, 0.0));
        assert_eq!(frame_of(&app, outside).down(), ae::Vec2::new(0.0, 1.0));
    }

    #[test]
    fn actor_response_scales_gravity_but_keeps_orientation_at_zero() {
        use ambition_combat::actor_tuning::ActorConfig;

        let mut app = resolver_app();
        let mut tuning = ambition_combat::actor_tuning::ActorTuning::default();
        tuning.movement.gravity = 800.0;
        let config = ActorConfig {
            tuning,
            brain: ambition_entity_catalog::placements::CharacterBrain::Passive,
            // A fixture body, not a seated CPU twin.
            preserves_mirror_symmetry: false,
        };
        let aerial = app
            .world_mut()
            .spawn((
                ambition_platformer2d_core::BodyKinematics { material: Default::default(),
                    pos: ae::Vec2::new(50.0, 50.0),
                    vel: ae::Vec2::ZERO,
                    canonical_size: ae::Vec2::new(20.0, 20.0),
                    facing: 1.0,
                },
                config,
                ActorSurfaceState {
                    surface_normal: ae::Vec2::new(0.0, -1.0),
                    gravity_scale: 0.0,
                },
                ResolvedMotionFrame::default(),
            ))
            .id();
        app.update();
        let frame = frame_of(&app, aerial);
        assert_eq!(frame.acceleration(), ae::Vec2::ZERO, "aerial: zero pull");
        assert_eq!(
            frame.down(),
            ae::Vec2::new(0.0, 1.0),
            "zero acceleration retains the environment-defined orientation"
        );
    }

    /// A held body's gravity is a fact of the hold. The frame gives it no pull,
    /// and its authored scale stays as authored, so nothing has to restore it
    /// when the hold ends.
    #[test]
    fn a_held_body_resolves_no_pull() {
        use ambition_combat::actor_tuning::ActorConfig;

        let mut app = resolver_app();
        let mut tuning = ambition_combat::actor_tuning::ActorTuning::default();
        tuning.movement.gravity = 800.0;
        let config = ActorConfig {
            tuning,
            brain: ambition_entity_catalog::placements::CharacterBrain::Passive,
            preserves_mirror_symmetry: false,
        };
        let body = |x: f32| {
            (
                ambition_platformer2d_core::BodyKinematics { material: Default::default(),
                    pos: ae::Vec2::new(x, 50.0),
                    vel: ae::Vec2::ZERO,
                    canonical_size: ae::Vec2::new(20.0, 20.0),
                    facing: 1.0,
                },
                config.clone(),
                ActorSurfaceState {
                    surface_normal: ae::Vec2::new(0.0, -1.0),
                    gravity_scale: 1.0,
                },
                ResolvedMotionFrame::default(),
            )
        };
        let free = app.world_mut().spawn(body(50.0)).id();
        let held = app
            .world_mut()
            .spawn((body(90.0), ambition_platformer2d_core::PoseOwnedExternally::only(ambition_platformer2d_core::PoseHold::Ride)))
            .id();
        let captor = app.world_mut().spawn_empty().id();
        let captive = app
            .world_mut()
            .spawn((
                body(130.0),
                ambition_combat::capture::CapturedBy {
                    captor,
                    hold_offset_local: ae::Vec2::new(16.0, 0.0),
                    hold_point: None,
                },
            ))
            .id();
        app.update();
        assert_eq!(frame_of(&app, captive).acceleration(), ae::Vec2::ZERO, "captive: zero pull");
        assert_eq!(
            frame_of(&app, free).acceleration(),
            ae::Vec2::new(0.0, 800.0),
            "control: the same body with nobody holding it falls"
        );
        assert_eq!(frame_of(&app, held).acceleration(), ae::Vec2::ZERO, "held: zero pull");
        assert_eq!(frame_of(&app, held).down(), ae::Vec2::new(0.0, 1.0), "held: orientation kept");
        assert_eq!(
            app.world().get::<ActorSurfaceState>(held).unwrap().gravity_scale,
            1.0,
            "the hold wrote nothing to the authored scale"
        );
    }

    /// Schedule-ordering evidence (ADR 0024): the frame resolution phase runs
    /// BEFORE `Platformer2dSimulationPhase::PlayerInput` — the earliest CoreSimulation consumer —
    /// so a probe there observes THIS tick's zone-resolved frame, not last
    /// tick's. Uses the real `configure_platformer2d_simulation_phases` + `GravityPlugin` wiring.
    #[test]
    fn the_frame_is_resolved_before_the_first_core_simulation_consumer() {
        use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase;

        #[derive(Resource, Default)]
        struct ProbeSawZoneFrame(bool);

        fn player_input_probe(
            mut saw: ResMut<ProbeSawZoneFrame>,
            frames: Query<
                &ResolvedMotionFrame,
                With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            >,
        ) {
            if let Ok(frame) = frames.single() {
                saw.0 = frame.down() == ae::Vec2::new(0.0, -1.0);
            }
        }

        let mut app = App::new();
        crate::schedule::configure_platformer2d_simulation_phases(&mut app);
        app.world_mut()
            .spawn(ambition_platformer2d_shared_tangle::lifecycle::SessionRoot(
                ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(0),
            ));
        app.insert_resource(ambition_platformer2d_shared_tangle::time::SimDt { dt: 0.016 });
        app.add_message::<ambition_combat::events::RoomReplayAdmitted>();
        app.add_plugins(crate::gravity::GravityPlugin);
        app.init_resource::<ambition_platformer2d_core::ActiveMovementTuning>();
        app.init_resource::<ProbeSawZoneFrame>();
        app.add_systems(
            Update,
            player_input_probe.in_set(Platformer2dSimulationPhase::PlayerInput),
        );

        app.world_mut().spawn(GravityZone {
            aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(60.0, 60.0)),
            dir: ae::Vec2::new(0.0, -1.0),
        });
        spawn_player(&mut app, ae::Vec2::ZERO);
        app.update();
        assert!(
            app.world().resource::<ProbeSawZoneFrame>().0,
            "a PlayerInput consumer must observe the SAME tick's resolved frame"
        );
    }

    #[test]
    fn the_gravity_field_mirror_agrees_with_the_primary_bodys_frame() {
        let mut app = resolver_app();
        app.world_mut().spawn(GravityZone {
            aabb: ae::Aabb::new(ae::Vec2::new(0.0, 0.0), ae::Vec2::new(60.0, 60.0)),
            dir: ae::Vec2::new(-1.0, 0.0),
        });
        let body = spawn_player(&mut app, ae::Vec2::ZERO);
        app.world_mut()
            .entity_mut(body)
            .insert(ambition_platformer2d_shared_tangle::body::PrimaryBody);
        app.update();
        assert_eq!(
            app.world().resource::<GravityField>().dir,
            frame_of(&app, body).down(),
            "presentation mirror derives from the SAME resolved artifact"
        );
    }

    /// The resolver's frame for a player body of `scale`, beside a control
    /// with no zone, when `force` (a world force zone over the body) is set.
    fn resolved_frames(scales: &[f32], force: Option<ae::Vec2>) -> Vec<ae::MotionFrame> {
        use ambition_platformer2d_shared_tangle::frame_env::ForceZone;
        let mut app = resolver_app();
        let here = ae::Vec2::new(50_000.0, 50_000.0);
        if let Some(accel) = force {
            app.world_mut().spawn(ForceZone { aabb: ae::Aabb::new(here, ae::Vec2::splat(10_000.0)), accel });
        }
        let bodies: Vec<Entity> = scales.iter().map(|&scale| {
            let body = spawn_player(&mut app, here);
            app.world_mut().get_mut::<ae::BodyKinematics>(body).unwrap().material =
                ae::frame::BodyMaterial::new(scale, bevy::math::Mat2::IDENTITY).unwrap();
            body
        }).collect();
        app.update();
        bodies.into_iter().map(|body| frame_of(&app, body)).collect()
    }

    /// One body of `scale` falling free for `ticks` under `frame`, stepped by
    /// the movement kernel as the integrator steps it: its fall, tick by tick.
    fn free_fall(frame: ae::MotionFrame, scale: f32, ticks: usize) -> Vec<f32> {
        let tuning = ae::ActiveMovementTuning::default();
        let start = ae::Vec2::splat(50_000.0);
        let world = ae::World::new("sr1 free fall", ae::Vec2::splat(100_000.0), start, Vec::new());
        let mut body = ae::BodyClusterScratch::new_with_abilities(start, ae::AbilitySet::default());
        body.kinematics.material = ae::frame::BodyMaterial::new(scale, bevy::math::Mat2::IDENTITY).unwrap();
        let mut model = ae::movement::MotionModel::axis_swept(tuning.axis_swept_params());
        (0..ticks).map(|_| {
            let result = ae::movement::step_motion(&mut model, &mut body.as_mut(), ae::movement::MotionStepContext {
                world: &world,
                admission: None,
                input: Default::default(),
                frame,
                facing_intent: 0.0,
                dt: 1.0 / 60.0,
                contact: ae::movement::body_contact::BodyContactField::NONE,
                pose_owned_externally: false,
                recovery_commitment_outstanding: false,
            });
            assert!(result.events.motion_refusal.is_none());
            body.kinematics.pos.y - start.y
        }).collect()
    }

    /// SR1 (SCALE-FEEL): a body at half or twice the size falls and jumps in
    /// its own units at the same times. Through the resolver that publishes
    /// the frame, the kernel that steps it, and the navigation probe built
    /// from the body's key: each normalized displacement agrees at 0.5, 1
    /// and 2, and the jump's apex comes on the same step. A world force zone
    /// is the counterexample: it moves every body by the same world distance.
    #[test]
    fn a_scaled_body_falls_and_jumps_in_its_own_units_at_the_same_times() {
        let scales = [0.5, 1.0, 2.0];
        let frames = resolved_frames(&scales, None);
        let gravity = ae::ActiveMovementTuning::default().gravity;
        for (scale, frame) in scales.iter().zip(&frames) {
            assert_eq!(frame.gravity_acceleration().y, gravity * scale, "the response converts at {scale}");
            assert_eq!(frame.down(), ae::Vec2::Y, "and the orientation does not");
        }

        // Free fall, normalized by each body's scale.
        let falls: Vec<Vec<f32>> = scales.iter().zip(&frames)
            .map(|(&scale, &frame)| free_fall(frame, scale, 45).into_iter().map(|dy| dy / scale).collect())
            .collect();
        assert!(falls[1][44] > 100.0, "the control falls: {:?}", falls[1][44]);
        for fall in &falls {
            for (tick, (a, b)) in fall.iter().zip(&falls[1]).enumerate() {
                assert!((a - b).abs() <= 1e-3 * b.abs().max(1.0), "tick {tick}: {a} against {b}");
            }
        }

        // The standing probe: the navigation key's body, under the same frame.
        let tuning = ae::ActiveMovementTuning::default();
        let walker = ae::BodyClusterScratch::new_with_abilities(ae::Vec2::ZERO, ae::AbilitySet::default());
        let empty = ae::World::new("sr1 probe", ae::Vec2::splat(1_000.0), ae::Vec2::ZERO, Vec::new());
        let envelopes: Vec<_> = scales.iter().zip(&frames).map(|(&scale, &frame)| {
            let key = ambition_platformer2d_world::navigation::NavGraphKey::new(
                &empty, &[], ae::movement::MotionModelSpec::AxisSwept(tuning.axis_swept_params()),
                ae::AbilitySet::default(), walker.kinematics.canonical_size,
                ae::frame::BodyMaterial::new(scale, bevy::math::Mat2::IDENTITY).unwrap(), walker.base_size, frame);
            ambition_platformer2d_world::navigation::TraversalEnvelope::measure(
                &key.probe_body(), frame, Default::default()).expect("an aligned frame is measured")
        }).collect();
        let apex_step = |envelope: &ambition_platformer2d_world::navigation::TraversalEnvelope| {
            let apex = envelope.apex_rise();
            envelope.jump.iter().position(|sample| sample.rise == apex).unwrap()
        };
        assert!(envelopes[1].apex_rise() > 20.0, "the control jumps: {}", envelopes[1].apex_rise());
        for (scale, envelope) in scales.iter().zip(&envelopes) {
            let (rise, control) = (envelope.apex_rise() / scale, envelopes[1].apex_rise());
            assert!((rise - control).abs() <= 1e-3 * control, "{scale}: apex {rise} against {control}");
            assert_eq!(apex_step(envelope), apex_step(&envelopes[1]), "{scale}: the apex comes on the same step");
            let (body, control) = (envelope.body_height / scale, envelopes[1].body_height);
            assert!((body - control).abs() <= 1e-4 * control, "{scale}: the probe is the body's size");
        }

        // The counterexample: a world force is no body's response. What it
        // adds to each fall is the same WORLD distance at every scale.
        let forced = resolved_frames(&scales, Some(ae::Vec2::new(0.0, 300.0)));
        let pushed: Vec<f32> = scales.iter().zip(&forced).zip(&frames)
            .map(|((&scale, &forced), &free)| free_fall(forced, scale, 30)[29] - free_fall(free, scale, 30)[29])
            .collect();
        assert!(pushed[1] > 10.0, "the force moves the control: {}", pushed[1]);
        for (scale, push) in scales.iter().zip(&pushed) {
            assert!((push - pushed[1]).abs() <= 1e-2 * pushed[1], "{scale}: the force's {push} world units against {}", pushed[1]);
        }
    }

    /// A response that cannot be derived is refused before it is published:
    /// the frame keeps its orientation and carries no gravity, and the
    /// movement kernel will not step the body under it.
    #[test]
    fn a_response_that_overflows_is_refused_and_moves_nothing() {
        let mut app = resolver_app();
        app.world_mut().resource_mut::<ae::ActiveMovementTuning>().0.gravity = f32::MAX;
        let body = spawn_player(&mut app, ae::Vec2::ZERO);
        app.world_mut().get_mut::<ae::BodyKinematics>(body).unwrap().material =
            ae::frame::BodyMaterial::new(4.0, bevy::math::Mat2::IDENTITY).unwrap();
        let control = spawn_player(&mut app, ae::Vec2::ZERO);
        app.update();
        let frame = frame_of(&app, body);
        assert!(frame.response_refused());
        assert_eq!(frame.gravity_acceleration(), ae::Vec2::ZERO);
        assert_eq!(frame.down(), ae::Vec2::Y);
        assert!(!frame_of(&app, control).response_refused(), "control: the identity scale converts nothing");

        let start = ae::Vec2::splat(500.0);
        let world = ae::World::new("sr1 refusal", ae::Vec2::splat(1_000.0), start, Vec::new());
        let mut scratch = ae::BodyClusterScratch::new_with_abilities(start, ae::AbilitySet::default());
        scratch.kinematics.vel = ae::Vec2::new(40.0, 0.0);
        let mut model = ae::movement::MotionModel::axis_swept(ae::ActiveMovementTuning::default().axis_swept_params());
        let result = ae::movement::step_motion(&mut model, &mut scratch.as_mut(), ae::movement::MotionStepContext {
            world: &world, admission: None, input: Default::default(), frame, facing_intent: 0.0,
            dt: 1.0 / 60.0, contact: ae::movement::body_contact::BodyContactField::NONE,
            pose_owned_externally: false, recovery_commitment_outstanding: false,
        });
        assert_eq!(scratch.kinematics.pos, start, "nothing moved under a refused frame");
        assert_eq!(result.events.motion_refusal.map(|refusal| refusal.reason),
            Some(ae::frame::thread::RefusalReason::Numeric(ae::frame::NumericFault::InvalidFrame)));
    }
}

/// Mirror the PRIMARY body's resolved frame into the presentation-facing
/// [`GravityField`]. This is the one writer of `GravityField.dir`, and it is a
/// MIRROR, not a resolver: the authoritative per-body resolution is the frame
/// resolution phase publishing [`ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame`]; this
/// derives the global presentation value (camera roll, gravity visuals, HUD)
/// from that same artifact so no second zone-overlap computation exists. Falls
/// back to the ambient of the sole live room when no primary body exists yet
/// (menus, tests).
pub fn resolve_active_gravity(
    base: Option<Res<BaseGravity>>,
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    bodies: Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame, With<ambition_platformer2d_shared_tangle::body::PrimaryBody>>,
    mut gravity: ResMut<GravityField>,
) {
    let base_dir = base.map_or(ambition_platformer2d_core::DEFAULT_GRAVITY_DIR, |base| {
        base.dir_in(live.sole())
    });
    gravity.dir = bodies
        .single()
        .map_or(base_dir, |frame| frame.down())
        .normalize_or_zero();
}

#[cfg(test)]
mod primary_gravity_tests {
    use super::*;

    #[test]
    fn gravity_field_mirrors_the_primary_bodys_resolved_frame() {
        let mut app = App::new();
        app.init_resource::<GravityField>();
        app.init_resource::<BaseGravity>();
        app.add_systems(Update, resolve_active_gravity);

        // No primary body yet → the ambient direction.
        app.update();
        assert!(
            app.world().resource::<GravityField>().dir.y > 0.0,
            "no primary body: ambient down"
        );

        let player = app
            .world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::body::PrimaryBody,
                ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
            ))
            .id();
        app.world_mut()
            .get_mut::<ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>(player)
            .unwrap()
            .publish_resolved_frame(ambition_platformer2d_core::MotionFrame::from_direction(
                Vec2::new(0.0, -1.0),
                900.0,
            ));
        app.update();
        assert!(
            app.world().resource::<GravityField>().dir.y < 0.0,
            "the field mirrors the primary body's resolved frame, not a second resolution"
        );
    }

}
