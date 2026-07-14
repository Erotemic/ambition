//! THE per-body frame resolution phase (ADR 0024 frame law).
//!
//! [`resolve_body_motion_frames`] publishes every integrated body's
//! [`ResolvedMotionFrame`] exactly once per sim tick, after the environment's
//! zone snapshot ([`super::GravitySet::ZoneSnapshot`]) and before
//! `SandboxSet::CoreSimulation` — so controller interpretation (the player brain
//! in `PlayerInput`, actor/possessed brains in `WorldPrep`), body integration,
//! and every combat/ability consumer read the SAME value for the tick.
//!
//! One system, one composition rule ([`FrameEnv::resolve`]), three archetype
//! queries — player bodies (primary + clones), actors, and bosses differ only in
//! where their authored gravity response lives, never in how the frame is
//! composed. No driver, brain, or combat system resolves a frame itself.

use bevy::prelude::*;

use ambition_platformer_primitives::frame_env::{FrameEnv, ResolvedMotionFrame};

use crate::features::ecs::actor_clusters::ActorConfig;
use crate::features::ecs::boss_clusters::BossConfig;
use crate::features::ActorSurfaceState;

/// Resolve and publish the frame for every integrated body.
///
/// - **Player bodies** (primary, clones, demo avatars): the authored gravity
///   response is the live movement tuning's `gravity`.
/// - **Actors and bosses** (both carry the unified actor cluster): the response
///   is `config.tuning.movement.gravity × surface.gravity_scale` — an aerial or
///   mounted body's 0 scale is the zero-acceleration-with-retained-orientation
///   case.
pub fn resolve_body_motion_frames(
    env: FrameEnv,
    editable_tuning: Res<ambition_dev_tools::dev_tools::EditableMovementTuning>,
    mut players: Query<
        (&crate::actor::BodyKinematics, &mut ResolvedMotionFrame),
        With<crate::actor::PlayerEntity>,
    >,
    mut actors: Query<
        (
            &crate::actor::BodyKinematics,
            &ActorConfig,
            &ActorSurfaceState,
            &mut ResolvedMotionFrame,
        ),
        Without<crate::actor::PlayerEntity>,
    >,
) {
    let player_response = editable_tuning.as_engine().gravity;
    for (kin, mut resolved) in &mut players {
        resolved.publish(env.resolve(kin.aabb(), player_response));
    }
    for (kin, config, surface, mut resolved) in &mut actors {
        let response = config.tuning.movement.gravity * surface.gravity_scale;
        resolved.publish(env.resolve(kin.aabb(), response));
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
    use ambition_engine_core as ae;
    use ambition_platformer_primitives::frame_env::{collect_force_zones, ForceZones};
    use ambition_platformer_primitives::gravity::{
        collect_gravity_zones, BaseGravity, GravityField, GravityZone, GravityZones,
    };

    fn resolver_app() -> App {
        let mut app = App::new();
        app.init_resource::<GravityField>();
        app.init_resource::<BaseGravity>();
        app.init_resource::<GravityZones>();
        app.init_resource::<ForceZones>();
        app.init_resource::<ambition_dev_tools::dev_tools::EditableMovementTuning>();
        app.add_systems(
            Update,
            (
                collect_gravity_zones,
                collect_force_zones,
                resolve_body_motion_frames,
                ambition_platformer_primitives::gravity::resolve_active_gravity,
            )
                .chain(),
        );
        app
    }

    fn spawn_player(app: &mut App, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                crate::actor::PlayerEntity,
                crate::actor::BodyKinematics {
                    pos,
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(24.0, 40.0),
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
        use crate::features::ecs::actor_clusters::ActorConfig;
        use crate::features::ActorSpawnState;
        let mut app = resolver_app();
        let mut tuning = crate::features::ecs::actor_tuning::ActorTuning::default();
        tuning.movement.gravity = 800.0;
        let config = ActorConfig {
            id: "aerial".into(),
            name: "aerial".into(),
            tuning,
            brain_spec: crate::features::ecs::actor_tuning::CharacterBrainSpec::default(),
            brain: ambition_entity_catalog::placements::CharacterBrain::Passive,
            spawn: ActorSpawnState {
                pos: ae::Vec2::new(50.0, 50.0),
                size: ae::Vec2::new(20.0, 20.0),
            },
            sprite_override_npc_name: None,
            sprite_character_id: None,
        };
        let aerial = app
            .world_mut()
            .spawn((
                crate::actor::BodyKinematics {
                    pos: ae::Vec2::new(50.0, 50.0),
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(20.0, 20.0),
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

    /// Schedule-ordering evidence (ADR 0024): the frame resolution phase runs
    /// BEFORE `SandboxSet::PlayerInput` — the earliest CoreSimulation consumer —
    /// so a probe there observes THIS tick's zone-resolved frame, not last
    /// tick's. Uses the real `configure_sandbox_sets` + `GravityPlugin` wiring.
    #[test]
    fn the_frame_is_resolved_before_the_first_core_simulation_consumer() {
        use ambition_platformer_primitives::schedule::SandboxSet;

        #[derive(Resource, Default)]
        struct ProbeSawZoneFrame(bool);

        fn player_input_probe(
            mut saw: ResMut<ProbeSawZoneFrame>,
            frames: Query<&ResolvedMotionFrame, With<crate::actor::PlayerEntity>>,
        ) {
            if let Ok(frame) = frames.single() {
                saw.0 = frame.down() == ae::Vec2::new(0.0, -1.0);
            }
        }

        let mut app = App::new();
        crate::schedule::configure_sandbox_sets(&mut app);
        app.world_mut().spawn(ambition_platformer_primitives::lifecycle::SessionRoot(
            ambition_platformer_primitives::lifecycle::SessionScopeId(0),
        ));
        app.insert_resource(ambition_platformer_primitives::time::SimDt { dt: 0.016 });
        app.add_message::<crate::features::ResetRoomFeaturesEvent>();
        app.add_plugins(crate::gravity::GravityPlugin);
        app.init_resource::<ambition_dev_tools::dev_tools::EditableMovementTuning>();
        app.init_resource::<ProbeSawZoneFrame>();
        app.add_systems(Update, player_input_probe.in_set(SandboxSet::PlayerInput));

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
            .insert(ambition_platformer_primitives::body::PrimaryBody);
        app.update();
        assert_eq!(
            app.world().resource::<GravityField>().dir,
            frame_of(&app, body).down(),
            "presentation mirror derives from the SAME resolved artifact"
        );
    }
}
