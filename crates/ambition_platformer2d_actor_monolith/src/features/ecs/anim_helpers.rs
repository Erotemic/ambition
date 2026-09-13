//! ECS read-only lookup helpers for sprite/animation systems.
//!
//! Presentation code calls these by id to drive enemy/npc/boss sprite
//! swaps, hit-flash, and animation rows without taking on a query for
//! every feature family itself.

use super::*;
use ambition_combat::components::{BreakableFeature, ChestFeature, FeatureId, Opened};

/// Advance every body's animation overlay clocks, exactly once per simulation
/// tick, on ONE clock.
///
/// ⛔⛤ **THIS USED TO EXCLUDE `PlayerEntity` AND THAT LEFT PLAYER 2 WITH NO
/// TICKING OWNER AT ALL.** The partition was
/// `Without<PlayerEntity>` here and `PrimaryPlayerOnly` + `single_mut()` in
/// `control::cleanup_timers_system`, so:
///
/// ```text
/// not a player               → this system
/// PlayerEntity + PrimaryPlayer → the player tick
/// PlayerEntity, no PrimaryPlayer → NOTHING
/// ```
///
/// `PlayerEntity`'s own documentation says *zero or many may exist*, and a
/// second/guest/remote player body carries it WITHOUT `PrimaryPlayer`. Every
/// timer this function decays — slash, shoot, wall-jump, interact, death, land,
/// dash-startup — is armed by SHARED combat and movement behaviour that does not
/// care which slot a body belongs to, so for player two they were armed and then
/// never decayed: a pose stuck on forever.
///
/// ⛔⛤ **AND THE TWO ROADS RAN ON DIFFERENT CLOCKS.** This one passes
/// `WorldTime::sim_dt` (world-anchored: poses pause and slow with the
/// simulation); the player road passed `Time::delta_secs` (decays while gameplay
/// is suspended). Same rollback-registered `BodyAnimFacts` fields, two
/// semantics, decided by whether the body happened to carry a marker. ⇒ One
/// body-generic tick, one clock; genuinely player-specific presentation state
/// (`PlayerBlinkCameraState`) stays in the player system, which is what it was
/// always for.
///
/// ⚠ **THE FIX IS THE MISSING FILTER, NOT A SECOND QUERY.** Adding a
/// "second player" road would have kept the split and added a third population
/// to keep in sync. The body-generic fact wants one owner over the whole
/// population.
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

/// ECS chest-opened lookup for sprite swapping.
pub fn ecs_chest_opened(
    id: &str,
    chests: &Query<(&FeatureId, Option<&Opened>), With<ChestFeature>>,
) -> Option<bool> {
    chests
        .iter()
        .find(|(feature_id, _)| feature_id.as_str() == id)
        .map(|(_, opened)| opened.is_some())
}

/// ECS breakable-state lookup for sprite swapping.
pub fn ecs_breakable_state(
    id: &str,
    breakables: &Query<(&FeatureId, &BreakableFeature)>,
) -> Option<ambition_interaction::BreakableState> {
    breakables
        .iter()
        .find(|(feature_id, _)| feature_id.as_str() == id)
        .map(|(_, breakable)| breakable.breakable.state)
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
    use bevy::prelude::*;

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
        world_time.scaled_dt = DT;
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

    /// The player road keeps the state that really is the player's, and keeps
    /// its own clock for it.
    ///
    /// ⚠ Present because the fix REMOVED a call from `cleanup_timers_system`,
    /// and a fix that quietly took the blink-camera decay with it would leave a
    /// camera ease that never finishes — visible, and nothing else watches it.
    #[test]
    fn the_player_tick_still_decays_the_blink_camera_it_owns() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(DT));
        app.insert_resource(time);

        let primary = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PlayerBlinkCameraState {
                    blink_in_timer: ARMED,
                    camera_snap_timer: ARMED,
                    ..Default::default()
                },
            ))
            .id();

        app.add_systems(Update, crate::control::cleanup_timers_system);
        app.update();

        let blink = app
            .world()
            .get::<PlayerBlinkCameraState>(primary)
            .expect("player");
        let expected = ARMED - DT;
        assert!(
            (blink.blink_in_timer - expected).abs() < 1e-6,
            "the blink-in ease should still decay in the player tick: {}",
            blink.blink_in_timer
        );
        assert!(
            (blink.camera_snap_timer - expected).abs() < 1e-6,
            "the camera-snap ease should still decay in the player tick: {}",
            blink.camera_snap_timer
        );
    }
}
