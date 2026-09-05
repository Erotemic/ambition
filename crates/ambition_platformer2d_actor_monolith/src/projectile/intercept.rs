//! INTERCEPTING A SHOT IN FLIGHT, as one named operation.
//!
//! ⭐⭐ THIS IS NOT A NEW MECHANIC — IT IS PARRY'S EXISTING ONE, GIVEN A NAME.
//! `reflect_parried_shot` already re-owned a projectile, rewrote its allegiance
//! and reversed its velocity, and that is almost exactly what a reflector, an
//! absorber and a redirect all do. The reason to lift it out is that every
//! future interception would otherwise be written against the projectile's
//! components directly, and each one would decide for itself which of the axes
//! below travel together.
//!
//! ⛔⛔ SIX AXES, AND THEY ARE INDEPENDENT. A reflected shot changes who owns it
//! for damage, whose side it is on, and where it is going. It does NOT change
//! what it looks like, who it is attributed to in a kill feed, or who is steering
//! it — a reflected guided missile is the case that proves "owner" cannot mean
//! all six at once. This operation touches exactly three: combat owner,
//! allegiance, trajectory. Presentation provenance stays with the original shot,
//! which is why the parry clang uses the PARRIER's voice while the bolt keeps its
//! own.
//!
//! ⛔ AND IT EMITS NO CUES. The caller owns those: a parry clangs, a reflector
//! hums, an absorber swallows, and a domain operation that played one of those
//! would make the other two wrong.

use ambition_platformer2d_core::BodyKinematics;
use ambition_projectiles::entity::ProjectileOwner;
use bevy::prelude::*;

use super::allegiance::ProjectileAllegiance;

/// What an interception DOES to the shot it caught.
#[derive(Clone, Debug, PartialEq)]
pub enum ProjectileInterception {
    /// Send it back where it came from, under the interceptor's authority.
    ///
    /// `speed_scale` multiplies the incoming speed — above `1.0` returns it
    /// faster than it arrived, which is what makes a parry a reward rather than
    /// a delay.
    Reflect { speed_scale: f32 },
    /// Take it out of the world.
    ///
    /// ⛔ THE OPERATION DOES NOT DECIDE WHAT THAT IS WORTH. An absorber that
    /// heals, one that fills a gauge and one that simply deletes the shot are
    /// the same interception and three different consequences, and the
    /// consequence belongs to whoever authored the move.
    Consume,
}

/// Apply `response` to a caught projectile, under `interceptor`'s authority.
///
/// Returns `true` when the projectile survives the interception, so a caller can
/// tell "it is going back" from "it is gone" without inspecting the world again.
pub fn intercept_projectile(
    commands: &mut Commands,
    projectile: Entity,
    kin: &mut BodyKinematics,
    interceptor: Entity,
    interceptor_allegiance: ProjectileAllegiance,
    response: &ProjectileInterception,
) -> bool {
    match response {
        ProjectileInterception::Reflect { speed_scale } => {
            // ⭐ OWNER AND ALLEGIANCE CHANGE TOGETHER, always. They answer two
            // different questions — who is credited, and whose side it is on —
            // and a shot whose owner moved without its allegiance is one that
            // damages the body that just saved itself.
            commands
                .entity(projectile)
                .insert((ProjectileOwner(interceptor), interceptor_allegiance));
            kin.vel = -kin.vel * *speed_scale;
            true
        }
        ProjectileInterception::Consume => {
            commands.entity(projectile).despawn();
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat::components::ActorFaction;

    fn allegiance(faction: ActorFaction) -> ProjectileAllegiance {
        ProjectileAllegiance {
            faction,
            team: None,
        }
    }

    fn world_with_shot(vel: ambition_platformer2d_core::Vec2) -> (App, Entity, Entity) {
        let mut app = App::new();
        let interceptor = app.world_mut().spawn_empty().id();
        let shot = app
            .world_mut()
            .spawn((
                BodyKinematics {
                    vel,
                    ..Default::default()
                },
                ProjectileOwner(interceptor),
                allegiance(ActorFaction::Enemy),
            ))
            .id();
        (app, shot, interceptor)
    }

    /// A reflected shot turns around, speeds up, and changes hands — all three.
    ///
    /// ⛔ THE THIRD IS THE ONE A REWRITE WOULD DROP. Reversing the velocity is
    /// the visible half; without the owner AND allegiance moving together the
    /// bolt flies back and still belongs to the body that fired it, so it passes
    /// through its target and can hit the parrier who just earned it.
    #[test]
    fn a_reflected_shot_changes_hands_as_well_as_direction() {
        let (mut app, shot, interceptor) =
            world_with_shot(ambition_platformer2d_core::Vec2::new(100.0, 0.0));
        let mine = allegiance(ActorFaction::Player);
        let survived = {
            let mut commands = app.world_mut().commands();
            let mut kin = BodyKinematics {
                vel: ambition_platformer2d_core::Vec2::new(100.0, 0.0),
                ..Default::default()
            };
            let out = intercept_projectile(
                &mut commands,
                shot,
                &mut kin,
                interceptor,
                mine.clone(),
                &ProjectileInterception::Reflect { speed_scale: 1.3 },
            );
            assert!(
                kin.vel.x < 0.0,
                "a reflected shot kept flying forward: {:?}",
                kin.vel
            );
            assert!(
                kin.vel.length() > 100.0,
                "a reflected shot came back no faster than it arrived ({}), so \
                 parrying a projectile is a delay rather than a reward",
                kin.vel.length()
            );
            out
        };
        app.world_mut().flush();
        assert!(survived, "a reflected shot reported itself destroyed");
        assert_eq!(
            app.world().get::<ProjectileAllegiance>(shot),
            Some(&mine),
            "the reflected shot kept the FIRER's allegiance, so it is still \
             hostile to the body that just parried it"
        );
    }

    /// A consumed shot leaves the world, and says so.
    #[test]
    fn a_consumed_shot_is_gone_and_reports_it() {
        let (mut app, shot, interceptor) =
            world_with_shot(ambition_platformer2d_core::Vec2::new(100.0, 0.0));
        let survived = {
            let mut commands = app.world_mut().commands();
            let mut kin = BodyKinematics::default();
            intercept_projectile(
                &mut commands,
                shot,
                &mut kin,
                interceptor,
                allegiance(ActorFaction::Player),
                &ProjectileInterception::Consume,
            )
        };
        app.world_mut().flush();
        assert!(
            !survived,
            "consuming a shot reported it as surviving, so a caller would go on \
             to steer a projectile that no longer exists"
        );
        assert!(
            app.world().get_entity(shot).is_err(),
            "a consumed shot is still in the world"
        );
    }
}
