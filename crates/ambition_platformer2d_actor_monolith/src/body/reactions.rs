//! Body reaction clocks for every controller and body role.

use ambition_characters::actor::BodyCombat;
use ambition_platformer2d_core::BodyKinematics;
use ambition_time::{ProperTimeScale, WorldTime};
use bevy::prelude::*;

/// Advance each body's recovery before control acceptance and body execution.
/// Hitstop uses the unscaled step so a freeze cannot stop its own expiry.
pub fn advance_body_reactions(
    time: Res<WorldTime>,
    mut bodies: Query<(&mut BodyCombat, Option<&ProperTimeScale>), With<BodyKinematics>>,
) {
    for (mut combat, scale) in &mut bodies {
        combat.decay_reaction_timers(
            time.entity_dt(ProperTimeScale::or_default(scale)),
            time.wall_dt(),
        );
    }
}

#[cfg(test)]
mod tests {
    use ambition_characters::actor::{BodyCombat, BodyHealth, Health};
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_combat::components::{
        ActorDisposition, ActorIdentity, ActorTarget, BossPhase, CenteredAabb,
    };
    use ambition_platformer2d_core::{movement::MotionModel, BodyKinematics, Vec2};
    use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
    use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
    use ambition_time::{ProperTimeScale, WorldTime};
    use bevy::prelude::*;

    fn app(sim_dt: f32) -> App {
        let mut app = App::new();
        app.insert_resource(WorldTime::new(0.1, sim_dt));
        app.add_systems(Update, super::advance_body_reactions);
        app
    }

    fn bodies(app: &mut App, scale: Option<f32>) -> [Entity; 3] {
        let bodies = std::array::from_fn(|_| {
            let body = app
                .world_mut()
                .spawn((
                    BodyKinematics {
                        pos: Vec2::ZERO,
                        vel: Vec2::ZERO,
                        size: Vec2::splat(16.0),
                        facing: 1.0,
                    },
                    BodyCombat {
                        damage_invuln_timer: 1.0,
                        hitstun_timer: 1.0,
                        hitstop_timer: 1.0,
                        ..Default::default()
                    },
                ))
                .id();
            if let Some(scale) = scale {
                app.world_mut()
                    .entity_mut(body)
                    .insert(ProperTimeScale(scale));
            }
            body
        });
        app.world_mut()
            .entity_mut(bodies[0])
            .insert((PlayerEntity, DrivingParticipant(PlayerSlot::PRIMARY)));
        app.world_mut().entity_mut(bodies[1]).insert((
            FeatureSimEntity,
            ActorIdentity::new("actor", "Actor"),
            ActorDisposition::Hostile,
            ActorTarget::default(),
            CenteredAabb::new(Vec2::ZERO, Vec2::splat(8.0)),
            MotionModel::default(),
        ));
        app.world_mut().entity_mut(bodies[2]).insert((
            FeatureSimEntity,
            BodyHealth::new(Health::new(3)),
            BossPhase::Active,
        ));
        bodies
    }

    #[test]
    fn reaction_clocks_follow_body_time_across_roles_and_driver_handoffs() {
        for scale in [None, Some(0.5), Some(2.0)] {
            let mut app = app(0.025);
            let bodies = bodies(&mut app, scale);
            app.update();
            let dt = 0.025 * scale.unwrap_or(1.0);
            for body in bodies {
                let combat = app.world().get::<BodyCombat>(body).unwrap();
                assert!(
                    (combat.hitstun_timer - (1.0 - dt)).abs() < 1e-6,
                    "{body:?}: wrong recovery clock"
                );
                assert!((combat.damage_invuln_timer - (1.0 - dt)).abs() < 1e-6);
                assert!(
                    (combat.hitstop_timer - 0.9).abs() < 1e-6,
                    "{body:?}: freeze expiry stopped with body time"
                );
            }
            app.world_mut()
                .entity_mut(bodies[0])
                .remove::<DrivingParticipant>();
            app.world_mut()
                .entity_mut(bodies[1])
                .insert(DrivingParticipant(PlayerSlot::PRIMARY));
            app.update();
            for body in bodies {
                let combat = app.world().get::<BodyCombat>(body).unwrap();
                assert!((combat.hitstun_timer - (1.0 - 2.0 * dt)).abs() < 1e-6);
                assert!((combat.hitstop_timer - 0.8).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn a_stopped_simulation_preserves_recovery_but_expires_every_bodys_hitstop() {
        let mut app = app(0.0);
        let bodies = bodies(&mut app, Some(0.5));
        for _ in 0..11 {
            app.update();
        }
        for body in bodies {
            let combat = app.world().get::<BodyCombat>(body).unwrap();
            assert_eq!(combat.hitstun_timer, 1.0);
            assert_eq!(combat.damage_invuln_timer, 1.0);
            assert_eq!(combat.hitstop_timer, 0.0);
        }
    }
}
