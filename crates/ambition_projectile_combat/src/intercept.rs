//! Prepare and commit one projectile interception.
//!
//! Combat owner, allegiance and trajectory change together. Presentation and
//! control stay with the shot. The caller owns contact cues and rewards.

use ambition_platformer2d_core::BodyKinematics;
use ambition_projectiles::entity::ProjectileOwner;
use bevy::prelude::*;

use ambition_combat::allegiance::AttackAllegiance;

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

/// A checked response, ready for one body and ownership commit.
#[derive(Clone,Debug)]
pub struct PreparedInterception {
    body: BodyKinematics,
    reflects: bool,
}

impl PreparedInterception {
    pub fn body(&self) -> &BodyKinematics { &self.body }
}

impl ProjectileInterception {
    /// Resolve the response in the physical contact chart before any write.
    pub fn prepare(&self, mut body: BodyKinematics,
        frame: ambition_platformer2d_core::frame::material_cast::ContactVelocityFrame,
    ) -> Result<PreparedInterception,ambition_platformer2d_core::frame::NumericFault> {
        use ambition_platformer2d_core::frame::{NumericFault,Sim2};
        match self {
            Self::Reflect {speed_scale} => {
                if !speed_scale.is_finite() {return Err(NumericFault::NonFinite);}
                if *speed_scale<0.0 {return Err(NumericFault::InvalidScale);}
                let physical=frame.to_physical(body.vel)?;
                let reflected=Sim2::IDENTITY.vector_from_wide(-physical.as_dvec2()*f64::from(*speed_scale))?;
                body.vel=frame.to_source(reflected)?;
                Ok(PreparedInterception {body,reflects:true})
            }
            Self::Consume => Ok(PreparedInterception {body,reflects:false}),
        }
    }
}

/// Apply `response` to a caught projectile, under `interceptor`'s authority.
///
/// Returns `true` when the projectile survives the interception, so a caller can
/// tell "it is going back" from "it is gone" without inspecting the world again.
///
/// ⛔⛔ `#[must_use]` BECAUSE IGNORING THIS ANSWER WAS A SHIPPED BUG. The absorber
/// branch of `step_projectiles` dropped it and continued its victim loop; the
/// despawn is a DEFERRED command, so a swallowed shot stayed live for the rest of
/// the tick and could strike a second body and then fall through to
/// boss/breakable/world resolution. Found by the 2026-09-05 review, not by a
/// test.
///
/// ⇒ The operation already knew the shot was gone and said so. The repair for
/// that class is not care at the call site: it is making the answer impossible to
/// drop silently. A caller that genuinely does not need it — a `Reflect`, which
/// always survives — now has to say so with a `let _ =` and a reason, which is
/// the difference between a decision and an oversight.
#[must_use = "a consumed projectile is GONE for the rest of the tick: check the               survival answer and terminate the shot's collision step, or say in               a comment why this interception cannot destroy it"]
pub fn intercept_projectile(
    commands: &mut Commands,
    projectile: Entity,
    kin: &mut BodyKinematics,
    interceptor: Entity,
    interceptor_allegiance: AttackAllegiance,
    response: &PreparedInterception,
) -> bool {
    match response.reflects {
        true => {
            // ⭐ OWNER AND ALLEGIANCE CHANGE TOGETHER, always. They answer two
            // different questions — who is credited, and whose side it is on —
            // and a shot whose owner moved without its allegiance is one that
            // damages the body that just saved itself.
            commands
                .entity(projectile)
                .insert((ProjectileOwner(interceptor), interceptor_allegiance))
                // ⛔⛤ AND THE OLD SHOOTER'S MOVE OCCURRENCE GOES WITH THE
                // OWNERSHIP, because it is no longer TRUE of this shot. The
                // interceptor did not make it with that move — nobody did.
                //
                // ⛔⛔ IT IS NOT MERELY UNTIDY, IT IS A CROSS-BODY COLLISION.
                // `MoveOccurrence` counts PER ENTITY from 0, so the same small
                // integers are live on many bodies at once. The verdict channel
                // carries `(attacker, instance)`; reflection rewrites the
                // attacker and, until this line, left the instance. A stale
                // `Some(0)` then met the interceptor's own occurrence 0 — the
                // expected case early in a fight — and credited a move that
                // never fired the shot.
                //
                // ⚠ NO PREDICATE CAN REPAIR THIS. The number alone cannot say
                // WHOSE it is, so the only defence is clearing it at the
                // boundary where the pair stops being consistent. See
                // `moveset::verdict_belongs_to`.
                //
                // ⭐ IF A MOVE CAUSED THE REFLECTION AND SHOULD OWN THE HIT,
                // STAMP THAT OCCURRENCE EXPLICITLY. Unclaimed is the default and
                // it is a real answer; inherited is never one.
                .remove::<ambition_projectiles::FiredByMoveInstance>()
                // The interceptor's weapon did not fire it either, so it
                // counts against neither weapon's `max_live`.
                .remove::<ambition_projectiles::WeaponShot>();
            *kin = response.body;
            true
        }
        false => {
            commands.entity(projectile).despawn();
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::ActorFaction;

    fn allegiance(faction: ActorFaction) -> AttackAllegiance {
        AttackAllegiance {
            driver: None,
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
            let response=ProjectileInterception::Reflect {speed_scale:1.3}
                .prepare(kin,ambition_platformer2d_core::frame::material_cast::ContactVelocityFrame::IDENTITY).unwrap();
            let out = intercept_projectile(
                &mut commands,
                shot,
                &mut kin,
                interceptor,
                mine.clone(),
                &response,
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
            app.world().get::<AttackAllegiance>(shot),
            Some(&mine),
            "the reflected shot kept the FIRER's allegiance, so it is still \
             hostile to the body that just parried it"
        );
    }

    /// ⛔⛤ AND A REFLECTED SHOT DROPS THE FIRER'S MOVE OCCURRENCE — the fourth
    /// thing that travels with ownership, and the one a rewrite would drop
    /// because it is a REMOVAL rather than an insert.
    ///
    /// ⛔⛔ `MoveOccurrence` counts per entity from 0, so a stale stamp is not
    /// an inert leftover: it is a number that COLLIDES with the new owner's own
    /// occurrence, and most loudly at 0, which both bodies hold early in a
    /// fight. The verdict channel carries `(attacker, instance)`; reflection
    /// rewrote the attacker and left the instance, so the pair stopped being
    /// internally consistent and `verdict_belongs_to` had no way to tell.
    ///
    /// ⚠ THE CONTROL ARM IS THE HALF THAT MAKES THIS A TEST. An assertion that
    /// a component is absent passes on a fixture that never added it, on a
    /// despawn, and on a typo in the type name. So this asserts the stamp was
    /// THERE first.
    #[test]
    fn a_reflected_shot_drops_the_firers_move_occurrence() {
        let (mut app, shot, interceptor) =
            world_with_shot(ambition_platformer2d_core::Vec2::new(100.0, 0.0));
        app.world_mut()
            .entity_mut(shot)
            .insert(ambition_projectiles::FiredByMoveInstance(0));
        assert_eq!(
            app.world()
                .get::<ambition_projectiles::FiredByMoveInstance>(shot)
                .map(|s| s.0),
            Some(0),
            "the premise: the shot carries the FIRER's occurrence before the \
             reflection, so its absence afterwards is a claim about the \
             reflection and not about the fixture"
        );

        let mut kin = BodyKinematics {
            vel: ambition_platformer2d_core::Vec2::new(100.0, 0.0),
            ..Default::default()
        };
        {
            let mut commands = app.world_mut().commands();
            let response=ProjectileInterception::Reflect {speed_scale:1.3}
                .prepare(kin,ambition_platformer2d_core::frame::material_cast::ContactVelocityFrame::IDENTITY).unwrap();
            let _ = intercept_projectile(
                &mut commands,
                shot,
                &mut kin,
                interceptor,
                allegiance(ActorFaction::Player),
                &response,
            );
        }
        app.world_mut().flush();

        assert_eq!(
            app.world()
                .get::<ambition_projectiles::FiredByMoveInstance>(shot),
            None,
            "the reflected shot still names the FIRER's use of their move. The \
             interceptor's own occurrence counts from 0 on their body, so that \
             stale number credits a move that never fired this shot -- and the \
             collision is the EXPECTED case early in a fight, when both are 0"
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
                &ProjectileInterception::Consume.prepare(BodyKinematics::default(),
                    ambition_platformer2d_core::frame::material_cast::ContactVelocityFrame::IDENTITY).unwrap(),
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
