//! Pure boss-attack volume math.
//!
//! Final step of the "move boss policy out of `BossRuntime`" migration.
//! `BossRuntime` used to expose:
//!
//! ```text
//! attack_volumes()
//! attack_telegraph_volumes()
//! cycle_pattern_volumes()
//! volumes_for(profile)
//! damageable_aabbs()
//! player_damage(player_body)
//! ```
//!
//! All of those read mirror fields (`attack_timer`, `attack_windup_timer`,
//! `active_strike_profile`, `telegraph_profile`, `pattern_timer`) that
//! the brain wrote into the runtime via `sync_runtime_mirror_from_attack_state`.
//! After this module lands the mirror fields go away and the helpers
//! here read [`BossAttackState`] + [`BossBehaviorProfile`] + the boss's
//! body fields (`pos`, `size`, `combat_size`) directly.
//!
//! No method on `BossRuntime` survives in the final form if it
//! depends on attack state — those become free functions here.

use crate::engine_core as ae;

use bevy::prelude::Component;

use crate::brain::{BossAttackProfile, BossAttackState};
use ambition_sprite_sheet::{AnimationBox, BodyMetrics, PixelRect};

use super::behavior::BossBehaviorProfile;

// =================================================================
// Sprite-metadata-driven body AABB derivation
// =================================================================
//
// The sprite generator emits per-sheet `body_metrics` carrying
// `body_pixel_bbox` (single overall body) and/or `body_pixel_parts`
// (named multi-rect for disjointed-piece characters like a giant
// boss with head + body + arms).
//
// These helpers turn that pixel-space metadata into world-space
// AABBs given the rendered position + render size, so gameplay
// systems (combat_size derivation, damageable_volumes, contact
// damage) can read a single source of truth — the sprite — instead
// of duplicating hardcoded numbers per boss.

/// Derive a single world-space AABB from one pixel rectangle in the
/// sprite-frame coordinate system, given the rendered size and
/// frame dimensions.
///
/// Sprite-frame coords: origin at top-left, y growing downward (the
/// image-space convention the generator emits).
///
/// World coords here: origin at the *center* of the rendered
/// sprite; y also grows downward in Ambition's world.
fn world_aabb_from_pixel_rect(
    bbox: PixelRect,
    frame_width: u32,
    frame_height: u32,
    world_center: ae::Vec2,
    world_size: ae::Vec2,
) -> ae::Aabb {
    let fw = frame_width.max(1) as f32;
    let fh = frame_height.max(1) as f32;
    let scale = ae::Vec2::new(world_size.x / fw, world_size.y / fh);
    let frame_center_x = fw * 0.5;
    let frame_center_y = fh * 0.5;
    let center_offset = ae::Vec2::new(
        (bbox.x as f32 + bbox.w as f32 * 0.5) - frame_center_x,
        (bbox.y as f32 + bbox.h as f32 * 0.5) - frame_center_y,
    );
    let center = world_center + ae::Vec2::new(center_offset.x * scale.x, center_offset.y * scale.y);
    let half = ae::Vec2::new(
        (bbox.w as f32 * 0.5 * scale.x).abs(),
        (bbox.h as f32 * 0.5 * scale.y).abs(),
    );
    ae::Aabb::new(center, half)
}

/// Build the full list of world-space body AABBs for a sprite-driven
/// actor from raw metadata parts. Both the registry's `BodyMetrics`
/// and the gameplay snapshot `BossSpriteMetrics` flow through here
/// — pass `body_pixel_parts` (preferred) and `body_pixel_bbox`
/// (fallback) directly.
///
/// Multi-part input emits one AABB per part; single-piece input
/// emits one AABB from the bbox; empty input returns `Vec::new()`.
/// Callers should treat empty-result as a signal to fall back to
/// the legacy `world_size`-driven AABB rather than the sprite
/// path.
pub fn world_space_body_aabbs_from_parts(
    body_pixel_parts: &[ambition_sprite_sheet::NamedPixelRect],
    body_pixel_bbox: Option<PixelRect>,
    frame_width: u32,
    frame_height: u32,
    world_center: ae::Vec2,
    world_size: ae::Vec2,
) -> Vec<ae::Aabb> {
    if !body_pixel_parts.is_empty() {
        return body_pixel_parts
            .iter()
            .map(|p| {
                world_aabb_from_pixel_rect(
                    p.rect(),
                    frame_width,
                    frame_height,
                    world_center,
                    world_size,
                )
            })
            .collect();
    }
    if let Some(bbox) = body_pixel_bbox {
        return vec![world_aabb_from_pixel_rect(
            bbox,
            frame_width,
            frame_height,
            world_center,
            world_size,
        )];
    }
    Vec::new()
}

/// Convenience wrapper that accepts the registry's `BodyMetrics`
/// struct directly. Equivalent to calling
/// [`world_space_body_aabbs_from_parts`] with the metrics' fields
/// expanded.
pub fn world_space_body_aabbs_from_metrics(
    metrics: &BodyMetrics,
    frame_width: u32,
    frame_height: u32,
    world_center: ae::Vec2,
    world_size: ae::Vec2,
) -> Vec<ae::Aabb> {
    world_space_body_aabbs_from_parts(
        &metrics.body_pixel_parts,
        metrics.body_pixel_bbox,
        frame_width,
        frame_height,
        world_center,
        world_size,
    )
}

/// Tight bounding box around a list of AABBs. Used to collapse
/// multi-part body AABBs into a single `combat_size` for movement
/// + soft world-bounds clamping. `None` for empty input.
pub fn bounding_aabb(parts: &[ae::Aabb]) -> Option<ae::Aabb> {
    let mut iter = parts.iter();
    let first = iter.next()?;
    let mut min = first.min;
    let mut max = first.max;
    for part in iter {
        if part.min.x < min.x {
            min.x = part.min.x;
        }
        if part.min.y < min.y {
            min.y = part.min.y;
        }
        if part.max.x > max.x {
            max.x = part.max.x;
        }
        if part.max.y > max.y {
            max.y = part.max.y;
        }
    }
    let center = (min + max) * 0.5;
    let half = (max - min) * 0.5;
    Some(ae::Aabb::new(center, half))
}

/// All the per-tick inputs the volume helpers need. Owned by the
/// caller so the helpers themselves stay pure.
pub struct BossVolumeContext<'a> {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub combat_size: ae::Vec2,
    pub behavior: &'a BossBehaviorProfile,
    pub attack_state: &'a BossAttackState,
    /// Sprite-driven body metrics. `Some` for bosses whose sprite
    /// RON carries `body_metrics` and the derivation system has
    /// snapshotted it. `damageable_volumes` prefers multi-rect
    /// hurtboxes from here over the legacy single-AABB fallback.
    pub sprite_metrics: Option<&'a crate::boss_encounter::behavior::BossSpriteMetrics>,
    /// Optional frame sample from the live boss sprite animator.
    /// When present and its profile matches the requested attack,
    /// sprite-authored hit/hurt boxes use this exact frame index
    /// instead of re-deriving a frame from attack timers. That keeps
    /// gameplay/debug boxes locked to the rendered animation frame.
    pub animation_frame: Option<&'a BossAnimationFrameSample>,
}

/// Live sprite-animation frame for a boss attack profile.
///
/// The renderer writes this component onto the boss simulation
/// entity when the currently rendered boss row is directly driven by
/// a `BossAttackProfile`. Gameplay/debug volume helpers read it
/// opportunistically and fall back to elapsed-time sampling in
/// headless tests or before sprites have upgraded.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BossAnimationFrameSample {
    /// Gameplay profile that selected the currently-rendered boss row,
    /// or `None` when the rendered row is the idle/rest pose (which is
    /// not driven by any attack profile). An idle sample still carries
    /// the live `frame_index` so the rest-pose hurtbox bobs with the
    /// breathing animation instead of locking to frame 0.
    pub profile: Option<BossAttackProfile>,
    /// Frame index in the currently-rendered boss row.
    pub frame_index: usize,
    /// Runtime sprite-metadata key that should be sampled with
    /// `frame_index`, when the renderer can resolve it. This is
    /// redundant with `profile` for most rows, but keeping the key on
    /// the sample makes the bridge explicit and prevents future
    /// profile↔row alias drift from silently selecting a fallback box.
    pub animation_key: Option<&'static str>,
}

impl<'a> BossVolumeContext<'a> {
    /// Build the context from a live boss runtime + its attack-state
    /// component. The runtime contributes only body fields, not
    /// policy. `is_gnu_ton` used to be carried separately for the
    /// hand-tuned volume path; the data-driven sprite_metrics path
    /// makes that special-case unnecessary.
    pub fn from_ref(boss: crate::features::BossRef<'a>, attack_state: &'a BossAttackState) -> Self {
        Self {
            pos: boss.kin.pos,
            size: boss.kin.size,
            combat_size: boss.combat_size(),
            behavior: &boss.config.behavior,
            attack_state,
            sprite_metrics: boss.status.sprite_metrics.as_ref(),
            animation_frame: None,
        }
    }

    pub fn with_animation_frame(
        mut self,
        animation_frame: Option<&'a BossAnimationFrameSample>,
    ) -> Self {
        self.animation_frame = animation_frame;
        self
    }
}

/// Active strike volumes — drawn red in the debug overlay and tested
/// against the player body by the damage system. Returns empty when
/// no strike is live (`attack_state.active_profile == None`).
///
/// Priority: sprite-author-declared per-animation hitbox (from
/// `BossSpriteMetrics::animations[animation_name].hitbox`) wins
/// over the hardcoded `volumes_for_profile` math. So when an
/// adapter declares the FloorSlam hitbox as `(4, 88, 120, 30)` in
/// pixel-frame coords, that's what damages the player — scaled to
/// world by the boss's render size. Falls back to
/// `volumes_for_profile` when the sprite has no per-animation
/// hitbox for this profile.
pub fn active_attack_volumes(ctx: &BossVolumeContext) -> Vec<ae::Aabb> {
    let Some(profile) = ctx.attack_state.active_profile.as_ref() else {
        return Vec::new();
    };
    if let Some(volumes) = sprite_authored_volumes(ctx, profile, ctx.attack_state.active_elapsed) {
        return volumes;
    }
    volumes_for_profile(profile, ctx.pos, ctx.combat_size, ctx.behavior)
}

/// Telegraph volumes — drawn yellow in the debug overlay. Returns
/// empty when nothing is currently telegraphing. Uses the same
/// sprite-authored-then-fallback priority as
/// [`active_attack_volumes`].
pub fn telegraph_volumes(ctx: &BossVolumeContext) -> Vec<ae::Aabb> {
    let Some(profile) = ctx.attack_state.telegraph_profile.as_ref() else {
        return Vec::new();
    };
    if let Some(volumes) = sprite_authored_volumes(ctx, profile, ctx.attack_state.telegraph_elapsed)
    {
        return volumes;
    }
    volumes_for_profile(profile, ctx.pos, ctx.combat_size, ctx.behavior)
}

/// Pull sprite-author-declared hitbox rectangles for the given
/// attack profile from `ctx.sprite_metrics.animations`. Returns
/// `None` (not empty) when the sprite has no hitbox for this
/// animation; the caller falls back to the hardcoded
/// `volumes_for_profile` math. Returns an empty `Vec` when the
/// sprite has an entry but no usable rects (defensive).
fn sprite_authored_volumes(
    ctx: &BossVolumeContext,
    profile: &BossAttackProfile,
    animation_elapsed_s: f32,
) -> Option<Vec<ae::Aabb>> {
    let metrics = ctx.sprite_metrics?;
    // Use the SPRITE RENDER SIZE (not `ctx.size`) — that's the
    // world-space extent of the visible sprite quad. `ctx.size` is
    // the LDtk spawn AABB which is smaller than the rendered sprite
    // (collision_scale > 1.0 in every sheet spec). Using ctx.size
    // would render hitboxes at half the visible size of the attack.
    let world_size = sprite_world_size(metrics, ctx.size);
    for animation in crate::boss_encounter::behavior::boss_animation_keys_for_profile(profile) {
        let Some(entry) = metrics.animations.get(*animation) else {
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
        let aabbs = world_space_animation_box_aabbs(
            hitbox,
            selected_frame,
            metrics.frame_width,
            metrics.frame_height,
            ctx.pos,
            world_size,
        );
        if !aabbs.is_empty() {
            return Some(aabbs);
        }
    }
    None
}

/// Choose the world-space size to scale sprite-pixel rects against.
/// Prefer the metrics-captured render size (set by
/// `derive_boss_sprite_metrics` from the sheet spec's
/// `collision_scale`). Fall back to `ctx.size` when the snapshot
/// didn't capture one — test fixtures that build `BossSpriteMetrics`
/// by hand can leave `sprite_render_size = Vec2::ZERO` to opt out.
fn sprite_world_size(
    metrics: &crate::boss_encounter::behavior::BossSpriteMetrics,
    fallback: ae::Vec2,
) -> ae::Vec2 {
    if metrics.sprite_render_size.x > 0.0 && metrics.sprite_render_size.y > 0.0 {
        metrics.sprite_render_size
    } else {
        fallback
    }
}

fn animation_frame_index(
    entry: &ambition_sprite_sheet::AnimationMetrics,
    elapsed_s: f32,
) -> Option<usize> {
    let frame_duration = entry.frame_duration_secs?;
    if frame_duration <= 0.0 {
        return None;
    }
    Some((elapsed_s.max(0.0) / frame_duration).floor() as usize)
}

fn authored_animation_frame_index(
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

/// Idle-pose frame index. Mirrors [`authored_animation_frame_index`]
/// for the rest pose: prefer the live rendered frame carried by an
/// idle (`profile: None`) sample so the rest-pose hurtbox bobs with
/// the breathing animation, falling back to elapsed-time sampling
/// (which, with the idle elapsed of 0, would lock to frame 0).
fn idle_animation_frame_index(
    ctx: &BossVolumeContext,
    entry: &ambition_sprite_sheet::AnimationMetrics,
    elapsed_s: f32,
) -> Option<usize> {
    if let Some(sample) = ctx.animation_frame {
        if sample.profile.is_none() {
            return Some(sample.frame_index);
        }
    }
    animation_frame_index(entry, elapsed_s)
}

fn push_unique_animation_key<'a>(keys: &mut Vec<&'a str>, key: &'a str) {
    if !key.is_empty() && !keys.iter().any(|existing| *existing == key) {
        keys.push(key);
    }
}

fn runtime_animation_keys<'a>(
    ctx: &'a BossVolumeContext<'a>,
    active_profile: Option<&'a BossAttackProfile>,
    rest_keys: &'a [&'a str],
) -> Vec<&'a str> {
    let mut keys = Vec::new();
    if let (Some(sample), Some(profile)) = (ctx.animation_frame, active_profile) {
        if sample.profile.as_ref() == Some(profile) {
            if let Some(animation_key) = sample.animation_key {
                push_unique_animation_key(&mut keys, animation_key);
            }
        }
    }
    let mapped_keys = active_profile
        .map(crate::boss_encounter::behavior::boss_animation_keys_for_profile)
        .unwrap_or(rest_keys);
    for key in mapped_keys {
        push_unique_animation_key(&mut keys, key);
    }
    keys
}

fn world_space_animation_box_aabbs(
    box_: &AnimationBox,
    frame_index: Option<usize>,
    frame_width: u32,
    frame_height: u32,
    world_center: ae::Vec2,
    world_size: ae::Vec2,
) -> Vec<ae::Aabb> {
    if let Some(index) = frame_index {
        if let Some(frame) = box_
            .frames
            .get(index.min(box_.frames.len().saturating_sub(1)))
        {
            if frame.is_populated() {
                return world_space_body_aabbs_from_parts(
                    &frame.parts,
                    frame.bbox,
                    frame_width,
                    frame_height,
                    world_center,
                    world_size,
                );
            }
        }
    }
    world_space_body_aabbs_from_parts(
        &box_.parts,
        box_.bbox,
        frame_width,
        frame_height,
        world_center,
        world_size,
    )
}

/// Damageable hurtbox volumes — where the player's attacks register
/// as hits. Single-piece bosses use one AABB derived from
/// combat_size; multi-part bosses (sprite RON carrying
/// `body_pixel_parts`) emit one AABB per piece so head/body/arms
/// hit independently. Animation boxes may also carry per-frame
/// samples so large moving parts like GNU-ton's head can track the
/// drawn pose instead of one coarse per-animation rectangle.
pub fn damageable_volumes(ctx: &BossVolumeContext) -> Vec<ae::Aabb> {
    // Priority (uniform across every boss now that GNU-ton's
    // hand-tuned head path was migrated into its spritesheet RON
    // — `gnu_ton_boss_spritesheet.ron` carries per-animation
    // `hurtbox.parts` that this function picks up below):
    //   1. Per-animation hurtbox for the currently-playing animation
    //      (attack frames with extended arms get a wider hurtbox
    //      than the rest pose; GNU-ton's "gnu_head_descent" anim
    //      carves out a head-only hurtbox at the descent position).
    //   2. Static `body_pixel_parts` (multi-rect body for disjointed
    //      characters).
    //   3. Static `body_pixel_bbox` (single-rect alpha bbox).
    //   4. `combat_size`-driven fallback (legacy bosses without
    //      sprite metadata).
    if let Some(metrics) = ctx.sprite_metrics {
        // Scale pixel rects to the visible sprite size, not the
        // smaller LDtk spawn AABB. See `sprite_world_size` for
        // the rationale.
        let world_size = sprite_world_size(metrics, ctx.size);
        // (1) Per-animation hurtbox. The current animation is
        // derived from the boss's `BossAttackState` —
        // `active_profile`'s animation when a strike is live,
        // `telegraph_profile`'s when a windup is showing,
        // `"rest"` otherwise. This matches the visible sprite
        // pose so a side-sweep's extended arms register as
        // damageable, while the rest pose's tight body bbox
        // wins when the boss is idle.
        let active_profile = ctx
            .attack_state
            .active_profile
            .as_ref()
            .or(ctx.attack_state.telegraph_profile.as_ref());
        let rest_keys: &[&str] = &["rest"];
        let active_keys = runtime_animation_keys(ctx, active_profile, rest_keys);
        let animation_elapsed_s = if ctx.attack_state.active_profile.is_some() {
            ctx.attack_state.active_elapsed
        } else if ctx.attack_state.telegraph_profile.is_some() {
            ctx.attack_state.telegraph_elapsed
        } else {
            0.0
        };
        for active_anim in active_keys {
            let Some(entry) = metrics.animations.get(active_anim) else {
                continue;
            };
            let Some(box_) = entry.hurtbox.as_ref() else {
                continue;
            };
            if !box_.is_populated() {
                continue;
            }
            let frame_index = match active_profile {
                Some(profile) => {
                    authored_animation_frame_index(ctx, profile, entry, animation_elapsed_s)
                }
                None => idle_animation_frame_index(ctx, entry, animation_elapsed_s),
            };
            let aabbs = world_space_animation_box_aabbs(
                box_,
                frame_index,
                metrics.frame_width,
                metrics.frame_height,
                ctx.pos,
                world_size,
            );
            if !aabbs.is_empty() {
                return aabbs;
            }
        }
        // (2) Static multi-part body.
        if !metrics.body_pixel_parts.is_empty() {
            let mut parts = Vec::with_capacity(metrics.body_pixel_parts.len());
            for part in &metrics.body_pixel_parts {
                parts.push(world_aabb_from_pixel_rect(
                    part.rect(),
                    metrics.frame_width,
                    metrics.frame_height,
                    ctx.pos,
                    world_size,
                ));
            }
            return parts;
        }
        // (3) Static single-rect body.
        if let Some(bbox) = metrics.body_pixel_bbox {
            return vec![world_aabb_from_pixel_rect(
                bbox,
                metrics.frame_width,
                metrics.frame_height,
                ctx.pos,
                world_size,
            )];
        }
    }
    // (4) Legacy fallback: combat_size-driven single AABB.
    vec![ae::Aabb::new(ctx.pos, ctx.combat_size * 0.5)]
}

/// Body-contact damage AABB. Stays at the runtime's combat envelope
/// — body contact is "you ran into the boss", not a discrete strike.
/// Pure function so future cleanup can lift this off `BossRuntime`
/// too without rewriting callers.
pub fn body_damage_aabb(pos: ae::Vec2, combat_size: ae::Vec2) -> ae::Aabb {
    ae::Aabb::new(pos, combat_size * 0.5)
}

/// Compute the per-tick boss → player damage event, if any.
///
/// Pure: reads the brain's `BossAttackState` (which strike is live,
/// which profile) + the boss body fields + the behavior's damage
/// scalars. Replaces `BossRuntime::player_damage`, which used to
/// poll mirror fields on the runtime.
///
/// Returns `Some(HitEvent)` when:
///   - A strike is live (`attack_state.active_profile.is_some()`)
///     and one of its volumes overlaps `player_body`, OR
///   - The boss body has positive `body_damage` and overlaps the
///     player.
///
/// Body contact wins only if the strike arm didn't fire — same
/// priority order as the legacy `BossRuntime::player_damage`.
///
/// `player_entity` is the player whose body is being tested; it's
/// stamped on the returned event's `target` so the player-side
/// reader lands the hit on that player rather than primary. The
/// caller (`update_ecs_bosses`) reads each boss's `ActorTarget` to
/// pick the per-boss victim and passes it down here.
pub fn boss_attack_damage(
    ctx: &BossVolumeContext,
    player_entity: bevy::prelude::Entity,
    player_body: ae::Aabb,
) -> Option<crate::features::HitEvent> {
    use crate::engine_core::AabbExt;
    use crate::mechanics::combat::events::{HitEvent, HitKnockback, HitMode, HitSource, HitTarget};
    use crate::mechanics::combat::util::midpoint;

    let signum_or = |x: f32, fallback: f32| {
        if x.abs() < f32::EPSILON {
            fallback
        } else {
            x.signum()
        }
    };

    // Strike arm: the brain's `active_profile` is the single source
    // of truth for "there's a live boss hitbox right now".
    if ctx.attack_state.active_profile.is_some() {
        let volumes = active_attack_volumes(ctx);
        if let Some(volume) = volumes
            .into_iter()
            .find(|volume| volume.strict_intersects(player_body))
        {
            return Some(HitEvent {
                volume,
                damage: ctx.behavior.attack_damage.max(1),
                source: HitSource::BossAttack,
                attacker: None,
                target: HitTarget::Player(player_entity),
                mode: HitMode::Knockback,
                knockback: Some(HitKnockback {
                    dir: signum_or(player_body.center().x - ctx.pos.x, 1.0),
                    strength: 1.25,
                    source_pos: ctx.pos,
                    impact_pos: midpoint(player_body.center(), volume.center()),
                }),
                ignored_targets: Vec::new(),
            });
        }
    }

    // Body-contact arm: same priority as the legacy
    // `BossRuntime::player_damage` — only fires when no strike
    // landed, and only when the behavior opts into body damage.
    let body_damage_amount = ctx.behavior.body_damage;
    if body_damage_amount > 0 {
        // Apply the sprite-derived body offset so the body-contact
        // zone lines up with the visible body (same offset
        // `boss.aabb()` applies). Without this, the magenta debug
        // box and the actual body-contact damage zone sit below the
        // visible sprite and the player can stand "inside" the
        // visible body without taking contact damage.
        let combat_offset = ctx
            .sprite_metrics
            .map(|m| m.combat_offset)
            .unwrap_or(ae::Vec2::ZERO);
        let body = body_damage_aabb(ctx.pos + combat_offset, ctx.combat_size);
        if body.strict_intersects(player_body) {
            return Some(HitEvent {
                volume: body,
                damage: body_damage_amount,
                source: HitSource::BossBody,
                attacker: None,
                target: HitTarget::Player(player_entity),
                mode: HitMode::Knockback,
                knockback: Some(HitKnockback {
                    dir: signum_or(player_body.center().x - ctx.pos.x, 1.0),
                    // Body contact should be a real displacement threat.
                    // Smirking Behemoth is designed to run the player down;
                    // a light bump let players face-tank the body and walk
                    // through to the far side of the arena.
                    strength: 2.6,
                    source_pos: ctx.pos,
                    impact_pos: midpoint(player_body.center(), body.center()),
                }),
                ignored_targets: Vec::new(),
            });
        }
    }
    None
}

/// World-space hitbox volumes for a specific attack profile. Pure
/// function of the profile + body fields. Used as the fallback path
/// when the boss has no `sprite_metrics`-driven per-animation
/// hitbox. The gradient sentinel and (since 2026-05-26) GNU-ton
/// route through `sprite_authored_volumes` instead — the match
/// arms here are still required for bosses whose sprite RONs don't
/// yet carry per-animation hitbox.parts.
pub fn volumes_for_profile(
    attack: &BossAttackProfile,
    pos: ae::Vec2,
    combat_size: ae::Vec2,
    behavior: &BossBehaviorProfile,
) -> Vec<ae::Aabb> {
    // The function used to take a separate `size` (boss spawn AABB)
    // because the GNU-ton arm scaled its design coords against it;
    // the data-driven migration retired that path, so combat_size is
    // the only size input the remaining arms need.
    let size = combat_size;
    let origin = pos + behavior.attack_origin_offset;
    match attack {
        BossAttackProfile::FloorSlam => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.5 + 22.0),
            ae::Vec2::new(size.x * 0.75, 18.0),
        )],
        BossAttackProfile::SideSweep => vec![
            ae::Aabb::new(
                origin + ae::Vec2::new(-size.x * 0.50, 0.0),
                ae::Vec2::new(size.x * 0.25, size.y * 0.72),
            ),
            ae::Aabb::new(
                origin + ae::Vec2::new(size.x * 0.50, 0.0),
                ae::Vec2::new(size.x * 0.25, size.y * 0.72),
            ),
        ],
        BossAttackProfile::FullBodyPulse => vec![ae::Aabb::new(origin, size * 0.70)],
        // Gradient Sentinel's vertical hazard column: tall narrow
        // rectangle centered on the boss x, extending well above and
        // below the boss body so jumping over is hard but lateral
        // dodge is easy. World-y span uses 1.8× the boss body height
        // — enough to span a typical sandbox arena's mid-air play
        // space without being absurdly tall. The Gradient Sentinel
        // sways ±130 px around its anchor (`AnchorSway` movement
        // profile), so the lane sweeps with the boss naturally.
        BossAttackProfile::HazardColumn => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, 0.0),
            ae::Vec2::new(size.x * 0.30, size.y * 1.80),
        )],
        // Every content special (`Special(_)`) routes its damage through
        // its own Technique's EFFECTS consumer (spawned projectiles /
        // World-anchored hitboxes / minions), so it has no body-mounted
        // melee volume — empty here prevents double-counting via
        // `boss_attack_damage`'s strike arm. This covers what used to be
        // MemorizedVolley / LockOnBeam / PitTrap / RotatingCross /
        // MinionCascade / DebrisRain.
        BossAttackProfile::Special(_) => Vec::new(),
        BossAttackProfile::WingSweep => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.08),
            ae::Vec2::new(size.x * 0.56, size.y * 0.42),
        )],
        BossAttackProfile::DiveLane => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.42),
            ae::Vec2::new(size.x * 0.22, size.y * 0.72),
        )],
        BossAttackProfile::Broadside => vec![
            ae::Aabb::new(
                origin + ae::Vec2::new(-size.x * 0.34, 0.0),
                ae::Vec2::new(size.x * 0.18, size.y * 0.84),
            ),
            ae::Aabb::new(
                origin + ae::Vec2::new(size.x * 0.34, 0.0),
                ae::Vec2::new(size.x * 0.18, size.y * 0.84),
            ),
        ],
        // GNU-ton fallbacks (only fire if a non-gnu-ton boss
        // somehow inherits a Gnu* profile — none today; preserved
        // so a future actor can adopt them without crashing).
        BossAttackProfile::HandSlam => vec![
            ae::Aabb::new(
                origin + ae::Vec2::new(-size.x * 0.40, size.y * 0.25),
                ae::Vec2::new(size.x * 0.14, size.y * 0.60),
            ),
            ae::Aabb::new(
                origin + ae::Vec2::new(size.x * 0.40, size.y * 0.25),
                ae::Vec2::new(size.x * 0.14, size.y * 0.60),
            ),
        ],
        BossAttackProfile::HandSweep => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.15),
            ae::Vec2::new(size.x * 0.85, size.y * 0.28),
        )],
        BossAttackProfile::HeadDescent => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.05),
            ae::Vec2::new(size.x * 0.32, size.y * 0.38),
        )],
        BossAttackProfile::ConvergingShockwave => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.48),
            ae::Vec2::new(size.x * 0.90, size.y * 0.08),
        )],
    }
}

// `gnu_ton_part_aabb` / `gnu_ton_sprite_scale` /
// `GNU_TON_COLLISION_SCALE` / `GNU_TON_FRAME_HEIGHT` were retired
// in the 2026-05-26 data-driven migration. GNU-ton's per-animation
// hit/hurt-box geometry now lives in
// `gnu_ton_boss_spritesheet.ron`'s `body_metrics.animations` map,
// and `world_aabb_from_pixel_rect` (the generic pixel→world
// transform that the gradient sentinel uses) produces the same
// world AABBs the hand-tuned math used to.

#[cfg(test)]
mod sprite_metadata_derivation_tests;
