//! ECS read-only lookup helpers for sprite/animation systems.
//!
//! Presentation code calls these by id to drive enemy/npc/boss sprite
//! swaps, hit-flash, and animation rows without taking on a query for
//! every feature family itself.

use super::*;

/// Advance all body animation overlays once per simulation tick using
/// `WorldTime::sim_dt`, including primary and remote player bodies.
/// Player-only presentation state has a separate owner.
pub fn advance_body_anim_overlay_clocks(
    world_time: Res<ambition_time::WorldTime>,
    mut bodies: Query<(
        &ambition_platformer2d_core::BodyMotionFacts,
        &mut ambition_characters::actor::BodyAnimFacts,
    )>,
) {
    let dt = world_time.sim_dt();
    for (facts, mut anim) in &mut bodies {
        ambition_characters::actor::advance_body_anim_overlays(facts.dashing, &mut anim, dt);
    }
}

// `ecs_boss_name` is GONE: the boss's static identity (name + behavior id) is
// materialized into `BossRenderIndex` (see `rebuild_boss_render_index`), which
// `upgrade_boss_sprites` reads by id — so binding a boss sheet no longer
// live-queries the boss clusters.


#[cfg(test)]
mod every_player_slot_gets_its_overlays_ticked {
    //! ⛔⛤ **PLAYER TWO'S ANIMATION POSES NEVER DECAYED, AND BOTH ROADS LOOKED
    //! CORRECT IN ISOLATION.**
    //!
    //! `advance_body_anim_overlay_clocks` excluded `Without<PlayerEntity>`
    //! because "the player advances its own"; `control::cleanup_timers_system`
    //! advanced "the player" through `PrimaryPlayerOnly` + `single_mut()`. Each
    //! comment was true about the population it named. Neither named the body
    //! that carries `PlayerEntity` WITHOUT `PrimaryPlayer` — which is exactly
    //! what a second, guest, or remote player body is, and `PlayerEntity`'s own
    //! doc says *zero or many may exist*.
    //!
    //! ⭐⭐ **THIS ARM RUNS BOTH SYSTEMS TOGETHER, WHICH IS THE ONLY WAY IT CAN
    //! SEE THE DEFECT.** A test of either system alone passes under the old
    //! split: the actor road correctly skips players, and the player road
    //! correctly ticks the primary. The hole is in the PARTITION, so the
    //! subject has to be the partition.
    //!
    //! ⚠ And it asserts the actor decays exactly ONE dt, not merely "decayed":
    //! the obvious wrong fix — widening the actor query while leaving the call
    //! in the player tick — makes the primary player decay TWICE, and a
    //! `< armed` assertion cannot tell the two apart.

    use super::*;
    use ambition_characters::actor::BodyAnimFacts;
    use ambition_platformer2d_core::BodyMotionFacts;
    use ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState;
    use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

    const ARMED: f32 = 1.0;
    const DT: f32 = 0.25;

    fn armed_body() -> (BodyMotionFacts, BodyAnimFacts) {
        (
            BodyMotionFacts::default(),
            BodyAnimFacts {
                slash_anim_timer: ARMED,
                interact_anim_timer: ARMED,
                ..Default::default()
            },
        )
    }

    #[test]
    fn a_second_player_body_has_its_anim_overlays_advanced_like_every_other_body() {
        let mut app = App::new();
        let mut world_time = ambition_time::WorldTime::default();
        world_time.set_sim_dt(DT);
        app.insert_resource(world_time);
        // The player road's own clock, set to the SAME step so a double tick is
        // the only way a body can lose 2 × DT.
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(DT));
        app.insert_resource(time);

        let primary = app
            .world_mut()
            .spawn((
                armed_body(),
                PlayerEntity,
                PrimaryPlayer,
                PlayerBlinkCameraState::default(),
            ))
            .id();
        // The body the old partition could not see: a player slot that is not
        // the home avatar.
        let second = app
            .world_mut()
            .spawn((armed_body(), PlayerEntity))
            .id();
        let actor = app.world_mut().spawn(armed_body()).id();

        app.add_systems(
            Update,
            (
                advance_body_anim_overlay_clocks,
                crate::control::cleanup_timers_system,
            ),
        );
        app.update();

        let expected = ARMED - DT;
        for (entity, who) in [
            (primary, "the primary player"),
            (second, "the SECOND player"),
            (actor, "a non-player actor"),
        ] {
            let anim = app.world().get::<BodyAnimFacts>(entity).expect("body");
            assert!(
                (anim.slash_anim_timer - expected).abs() < 1e-6,
                "{who}'s slash overlay should have decayed by exactly one \
                 simulation step ({DT}): expected {expected}, got {}",
                anim.slash_anim_timer
            );
            assert!(
                (anim.interact_anim_timer - expected).abs() < 1e-6,
                "{who}'s interact overlay should have decayed by exactly one \
                 simulation step ({DT}): expected {expected}, got {}",
                anim.interact_anim_timer
            );
        }
    }

    #[test]
    fn every_blink_presentation_uses_wall_time_even_when_simulation_is_paused() {
        let mut app = App::new();
        let mut world_time = ambition_time::WorldTime::default();
        world_time.set_wall_dt(DT);
        world_time.set_sim_dt(0.0);
        app.insert_resource(world_time);
        let primary = app.world_mut().spawn((PlayerEntity, PrimaryPlayer,
            PlayerBlinkCameraState { blink_in_timer: ARMED, camera_snap_timer: ARMED, ..Default::default() },
        )).id();
        let second = app.world_mut().spawn((PlayerEntity,
            PlayerBlinkCameraState { blink_in_timer: DT, camera_snap_timer: DT / 2.0, ..Default::default() },
        )).id();
        let observer = app.world_mut().spawn(PlayerBlinkCameraState::default()).id();
        app.world_mut().spawn(armed_body());
        app.add_systems(Update, crate::control::cleanup_timers_system);
        app.update();
        for (body, expected) in [(primary, ARMED - DT), (second, 0.0), (observer, 0.0)] {
            let blink = app.world().get::<PlayerBlinkCameraState>(body).unwrap();
            assert_eq!(blink.blink_in_timer, expected);
            assert_eq!(blink.camera_snap_timer, expected);
        }
    }
}
