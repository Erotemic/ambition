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
    // Use the SPRITE RENDER SIZE (not `ctx.size`) — that's the
    // world-space extent of the visible sprite quad. `ctx.size` is
    // the LDtk spawn AABB which is smaller than the rendered sprite
    // (collision_scale > 1.0 in every sheet spec). Using ctx.size
    // would render hitboxes at half the visible size of the attack.
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
/// Removing the push left all 21 derivation tests green, which is how the gap was found.
///
/// the push cannot simply be deleted. `apple_rain` is a `Special` absent from
/// the content crate's `special_animation_keys()`, so its profile claims NOTHING
/// and the sample key is the only thing that finds its damageable row. Deleting
/// the push changes a live boss's hurtbox. That is a CONTENT decision and it sits
/// in `awaiting-maintainer-decision.md`.
///
/// what is not blocked is telling the two apart. Behaviour is unchanged —
/// [`Self::in_lookup_order`] rebuilds the exact list, same order, same dedup —
/// but the rescue is now a property something can assert on
/// ([`Self::only_the_sample_names_a_key`]) instead of a comment. The day the
/// content decision lands, that predicate goes false for `apple_rain` and the
/// fold's precondition is a test result rather than an argument.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct RuntimeAnimationKeys {
    /// The key the LIVE SAMPLE names, present only when the sample's profile is
    /// the one being resolved. The fallback that rescues a profile claiming no
    /// rows — and the term that makes a key-based rule untestable while it is
    /// folded into the list below.
    pub(super) sample_key: Option<String>,
    /// The keys the PROFILE itself claims (or `rest_keys` when there is no
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

    /// The circularity, as a predicate. True when the sample's own key is the
    /// only thing naming a row — i.e. this profile's rows are found only because
    /// the sample rescued it, and a key-based rule would miss and fall back to
    /// elapsed-time sampling.
    /// a TEST is its only caller, deliberately — the doc above says the point
    /// is that the rescue becomes "a test result rather than an argument", so a
    /// production caller was never the goal. Silenced rather than `#[cfg(test)]`
    /// because the type's own doc links to it, and a cfg'd item breaks that link
    /// in a normal build.
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
