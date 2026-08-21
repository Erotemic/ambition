//! ECS read-only lookup helpers for sprite/animation systems.
//!
//! Presentation code calls these by id to drive enemy/npc/boss sprite
//! swaps, hit-flash, and animation rows without taking on a query for
//! every feature family itself.

use super::*;

/// Advance every non-player actor's movement-driven anim overlays (landing /
/// dash-startup) one frame, via the SAME [`crate::features::advance_body_anim_overlays`]
/// the player tick runs — so `ambition_character_sprites::pick_actor_anim` can show
/// those poses (fable review §A9). The home player ([`crate::actor::PlayerEntity`])
/// is excluded (it advances its own overlays in the player tick), so no body is
/// advanced twice; a possessed non-player body IS advanced here. Uses `sim_dt`
/// (world-anchored animation), so the poses pause and slow with the sim. Scheduled
/// right before [`rebuild_actor_anim_index`] (its reader) and skipped headless
/// with it — these overlays are presentation-only.
pub fn advance_actor_anim_overlays(
    world_time: Res<ambition_time::WorldTime>,
    mut actors: Query<
        (
            &ambition_platformer2d_core::BodyMotionFacts,
            &mut crate::actor::BodyAnimFacts,
        ),
        Without<crate::actor::PlayerEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    for (facts, mut anim) in &mut actors {
        crate::features::advance_body_anim_overlays(facts.dashing, &mut anim, dt);
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



// ✔ FOUR boss animation helpers LEFT for `ambition_boss_encounter::anim` on
// 2026-08-21 (D33): every type they read is that crate's, and `ambition_sim_view`
// was reaching them through this crate while already depending on it directly.
// What stays here is not boss -- chest, breakable, and the actor overlay advance
// that needs this crate's own `advance_body_anim_overlays`.
pub use ambition_boss_encounter::anim::{
    boss_anim_state_for, ecs_boss_anim_state, ecs_boss_anim_state_and_entity,
    ecs_boss_animation_frame_sample,
};
