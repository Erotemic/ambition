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





#[cfg(test)]
mod sample_key_agrees_with_profile_keys_tests {
    use super::*;
    use ambition_characters::brain::BossAttackProfile;

    /// **Does the sample's KEY name a row the PROFILE claims?**
    ///
    /// This single fact decides whether the `BossAnim`→`CharacterAnim` fold's
    /// first slice is a rename or a redesign, and it is why that slice is
    /// blocked on this file rather than on the four readers everyone looks at.
    ///
    /// `BossAnimationFrameSample` carries two identities: `profile` (which
    /// attack is driving) and `animation_key` (which row is rendered). Four
    /// production sites compare the PROFILE. Replacing them with a key
    /// comparison — the thing that would let the sample drop `BossAttackProfile`
    /// and become character-generic — is only safe if the key the WRITER emits
    /// is always a row that profile claims.
    ///
    /// [`boss_animation_key_for_sample`] deliberately overrides the key for four
    /// GNU-ton profiles ("the damageable head/body box should follow the
    /// rendered row"), so the two are NOT the same string. The question is
    /// whether the override still lands inside
    /// `boss_animation_keys_for_profile`'s list — and for these it does:
    ///
    /// ```text
    ///   head_descent          → "head_down"   ∈ [gnu_head_descent, head_down]
    ///   converging_shockwave  → "hand_slam"   ∈ [gnu_shockwave, hand_slam]
    ///   hand_slam             → "hand_slam"   ∈ [gnu_hand_slam, hand_slam]
    ///   hand_sweep            → "hand_sweep"  ∈ [gnu_hand_sweep, hand_sweep]
    /// ```
    ///
    /// ⚠ **a fifth override is NOT covered here and is the open half**:
    /// `("apple_rain", SpikeHalo) => "head_down"`. `apple_rain` is a
    /// `Special`, so its key list comes from the App-local `BossCatalog` at
    /// runtime, and the engine cannot know whether `head_down` is in it. That
    /// case has to be answered against a real catalog, not here.
    #[test]
    fn every_hardcoded_sample_key_names_a_row_its_profile_claims() {
        let catalog = ambition_boss_encounter::test_boss_catalog();
        for move_id in [
            "head_descent",
            "converging_shockwave",
            "hand_slam",
            "hand_sweep",
        ] {
            let profile = BossAttackProfile::Strike(move_id.to_string());
            let anim = boss_anim_for_attack_profile(&profile)
                .unwrap_or_else(|| panic!("{move_id} maps to a boss anim"));
            let key = boss_animation_key_for_sample(catalog, &profile, anim)
                .unwrap_or_else(|| panic!("{move_id} yields a sample key"));
            let claimed =
                ambition_boss_encounter::behavior::boss_animation_keys_for_profile(catalog, &profile);
            assert!(
                claimed.iter().any(|candidate| *candidate == key),
                "the sample writer emits `{key}` for `{move_id}`, and the profile \
                 claims {claimed:?}. If the key is not among them, then swapping the \
                 four profile-identity checks to a key comparison changes which \
                 hitbox this boss presents — and the animator fold stops being a \
                 rename"
            );
        }
    }
}
