//! Sprite animation-frame sampling — picks the authored/runtime frame and its
//! world-space animation-box AABBs that the volume queries read.
//!
//! Resolves which animation row + frame a boss is showing (preferring a live
//! `BossAnimationFrameSample` over elapsed-time sampling), the world size to
//! scale against (`sprite_world_size`), and the per-frame hit/hurtbox AABBs
//! (`sprite_authored_volumes`, `world_space_animation_box_aabbs`). Consumed by
//! `mod`'s `active_attack_volumes` / `telegraph_volumes` / `damageable_volumes`.

use super::*;

pub(super) fn sprite_authored_volumes(
    ctx: &BossVolumeContext,
    profile: &BossAttackProfile,
    animation_elapsed_s: f32,
) -> Option<Vec<ae::CombatVolume>> {
    let metrics = ctx.sprite_metrics?;
    // Use the sprite render size, not `ctx.size`: it is the world-space extent
    // of the visible sprite quad. `ctx.size` is the LDtk spawn AABB, which is
    // smaller (collision_scale > 1.0 in every sheet spec), so hitboxes would be
    // about half the visible attack size.
    let world_size = sprite_world_size(metrics, ctx.size);
    for animation in crate::behavior::boss_animation_keys_for_profile(ctx.boss_catalog, profile) {
        let Some(entry) = metrics.animations.get(&animation) else {
            continue;
        };
        let Some(hitbox) = entry.hitbox.as_ref() else {
            continue;
        };
        if !hitbox.is_populated() {
            continue;
        }
        let selected_frame =
            authored_animation_frame_index(ctx, profile, entry, animation_elapsed_s);
        let volumes = world_space_animation_box_volumes(
            hitbox,
            selected_frame,
            metrics.frame_width,
            metrics.frame_height,
            ctx.pos,
            world_size,
        );
        if !volumes.is_empty() {
            return Some(volumes);
        }
    }
    None
}

pub(super) fn authored_animation_frame_index(
    ctx: &BossVolumeContext,
    profile: &BossAttackProfile,
    entry: &ambition_sprite_sheet::AnimationMetrics,
    elapsed_s: f32,
) -> Option<usize> {
    if let Some(sample) = ctx.animation_frame {
        if sample.profile.as_ref() == Some(profile) {
            return Some(sample.frame_index);
        }
    }
    animation_frame_index(entry, elapsed_s)
}

pub(super) fn push_unique_animation_key(keys: &mut Vec<String>, key: &str) {
    if !key.is_empty() && !keys.iter().any(|existing| *existing == key) {
        keys.push(key.to_string());
    }
}

/// The animation keys a runtime lookup will try, with their two provenances
/// kept apart.
///
/// The sample key is a fallback that can hide missing profile rows. It cannot
/// simply be removed: `apple_rain` is a `Special` absent from the content
/// crate's `special_animation_keys()`, so its profile claims nothing and the
/// sample key is the only thing that finds its damageable row. Removing it
/// changes a live boss's hurtbox, which is a content decision (tracked in
/// `awaiting-maintainer-decision.md`).
///
/// So the two sources are kept apart. [`Self::in_lookup_order`] rebuilds the
/// exact list (same order, same dedup), and
/// [`Self::only_the_sample_names_a_key`] makes the rescue testable. When the
/// content decision lands, that predicate becomes false for `apple_rain`.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct RuntimeAnimationKeys {
    /// The key the live sample names, present only when the sample's profile
    /// is the one being resolved. It rescues a profile that claims no rows.
    pub(super) sample_key: Option<String>,
    /// The keys the profile itself claims (or `rest_keys` when there is no
    /// profile). This is what a key-based rule would actually consult.
    pub(super) claimed: Vec<String>,
}

impl RuntimeAnimationKeys {
    /// The flat list the resolver tries, in order. Byte-identical to what the
    /// single `Vec<String>` produced: sample key first, then claimed keys, empty
    /// entries dropped and duplicates removed by `push_unique_animation_key`.
    pub(super) fn in_lookup_order(&self) -> Vec<String> {
        let mut keys: Vec<String> = Vec::new();
        if let Some(sample_key) = self.sample_key.as_deref() {
            push_unique_animation_key(&mut keys, sample_key);
        }
        for key in &self.claimed {
            push_unique_animation_key(&mut keys, key);
        }
        keys
    }

    /// True when the sample's own key is the only thing naming a row: this
    /// profile's rows are found only because the sample rescued it, and a
    /// key-based rule would miss and fall back to elapsed-time sampling.
    ///
    /// Only tests call it. It is allowed dead code, not `#[cfg(test)]`, because
    /// the type's doc links to it and a cfg'd item would break that link in a
    /// normal build.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn only_the_sample_names_a_key(&self) -> bool {
        self.sample_key.is_some() && self.claimed.iter().all(|key| key.is_empty())
    }
}

pub(super) fn runtime_animation_keys(
    ctx: &BossVolumeContext,
    active_profile: Option<&BossAttackProfile>,
    rest_keys: &[&'static str],
) -> RuntimeAnimationKeys {
    let mut sample_key = None;
    if let (Some(sample), Some(profile)) = (ctx.animation_frame, active_profile) {
        if sample.profile.as_ref() == Some(profile) {
            sample_key = sample
                .animation_key
                .as_deref()
                .filter(|key| !key.is_empty())
                .map(str::to_string);
        }
    }
    let claimed = active_profile
        .map(|profile| crate::behavior::boss_animation_keys_for_profile(ctx.boss_catalog, profile))
        .unwrap_or_else(|| rest_keys.iter().map(|key| (*key).to_string()).collect());
    RuntimeAnimationKeys {
        sample_key,
        claimed,
    }
}
