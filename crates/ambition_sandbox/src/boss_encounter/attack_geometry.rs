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
use crate::presentation::character_sprites::registry::{AnimationBox, BodyMetrics, PixelRect};

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
    body_pixel_parts: &[crate::presentation::character_sprites::registry::NamedPixelRect],
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
    entry: &crate::presentation::character_sprites::registry::AnimationMetrics,
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
    entry: &crate::presentation::character_sprites::registry::AnimationMetrics,
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
    entry: &crate::presentation::character_sprites::registry::AnimationMetrics,
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
        // Specials' damage routes through their EFFECTS consumers
        // (spawned projectiles / World-anchored hitboxes / minions).
        // Empty volumes here prevent double-counting via
        // `boss_attack_damage`'s strike arm.
        BossAttackProfile::MemorizedVolley
        | BossAttackProfile::LockOnBeam
        | BossAttackProfile::PitTrap
        | BossAttackProfile::RotatingCross
        | BossAttackProfile::MinionCascade => Vec::new(),
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
        BossAttackProfile::DebrisRain => Vec::new(),
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
mod sprite_metadata_derivation_tests {
    use super::*;
    use crate::engine_core::AabbExt;
    use crate::presentation::character_sprites::registry::{NamedPixelRect, PixelRect};

    /// Centered pixel bbox at frame center → world AABB at world_center.
    /// The 128×128 frame with a 64×64 bbox at (32, 32) should map to
    /// a world AABB at world_center with half-size = (16, 16) when the
    /// world_size is (64, 64) (1:1 px/world). Tests the basic
    /// pixel-frame → world-space transform.
    #[test]
    fn world_aabb_from_centered_pixel_rect_lands_at_world_center() {
        let bbox = PixelRect {
            x: 32,
            y: 32,
            w: 64,
            h: 64,
        };
        let world = world_aabb_from_pixel_rect(
            bbox,
            128,
            128,
            ae::Vec2::new(100.0, 200.0),
            ae::Vec2::new(64.0, 64.0),
        );
        // Center of pixel rect = (64, 64) = frame center → world
        // center should be exactly the passed world_center.
        let center = world.center();
        assert!((center.x - 100.0).abs() < 1e-3);
        assert!((center.y - 200.0).abs() < 1e-3);
        // Half-size = (64*0.5 * 0.5_scale, 64*0.5 * 0.5_scale) =
        // 16 since scale = 64/128 = 0.5.
        let half = world.half_size();
        assert!((half.x - 16.0).abs() < 1e-3);
        assert!((half.y - 16.0).abs() < 1e-3);
    }

    /// Off-center bbox should land off-center in world too. A bbox
    /// in the top-left quadrant of the frame should produce a world
    /// AABB above-and-left of the world_center.
    #[test]
    fn world_aabb_from_off_center_bbox_translates_correctly() {
        let bbox = PixelRect {
            x: 0,
            y: 0,
            w: 32,
            h: 32,
        };
        let world = world_aabb_from_pixel_rect(
            bbox,
            128,
            128,
            ae::Vec2::new(500.0, 500.0),
            ae::Vec2::new(64.0, 64.0),
        );
        let center = world.center();
        // Frame center is (64, 64); bbox center is (16, 16); offset
        // (-48, -48). Scaled to world by 64/128 = 0.5 → (-24, -24).
        // World center = (500 - 24, 500 - 24) = (476, 476).
        assert!((center.x - 476.0).abs() < 1e-3);
        assert!((center.y - 476.0).abs() < 1e-3);
    }

    /// Multi-part metadata returns one world AABB per pixel part.
    /// Verifies the "disjointed character pieces" path the user
    /// asked for — three named rects yield three world AABBs in the
    /// same order.
    #[test]
    fn world_space_body_aabbs_emits_one_per_named_part() {
        let parts = vec![
            NamedPixelRect {
                name: "head".to_string(),
                x: 56,
                y: 16,
                w: 16,
                h: 16,
            },
            NamedPixelRect {
                name: "body".to_string(),
                x: 48,
                y: 32,
                w: 32,
                h: 48,
            },
            NamedPixelRect {
                name: "left_hand".to_string(),
                x: 16,
                y: 64,
                w: 16,
                h: 16,
            },
        ];
        let aabbs = world_space_body_aabbs_from_parts(
            &parts,
            None,
            128,
            128,
            ae::Vec2::ZERO,
            ae::Vec2::new(128.0, 128.0),
        );
        assert_eq!(
            aabbs.len(),
            3,
            "multi-part should produce one AABB per named part",
        );
    }

    /// Empty parts + present bbox falls back to single-rect path.
    #[test]
    fn world_space_body_aabbs_falls_back_to_single_bbox() {
        let bbox = PixelRect {
            x: 16,
            y: 16,
            w: 96,
            h: 96,
        };
        let aabbs = world_space_body_aabbs_from_parts(
            &[],
            Some(bbox),
            128,
            128,
            ae::Vec2::ZERO,
            ae::Vec2::new(128.0, 128.0),
        );
        assert_eq!(
            aabbs.len(),
            1,
            "single bbox should produce exactly one AABB",
        );
    }

    /// Empty parts + no bbox returns an empty list (callers fall
    /// back to the legacy combat_size path).
    #[test]
    fn world_space_body_aabbs_empty_when_no_metadata() {
        let aabbs = world_space_body_aabbs_from_parts(
            &[],
            None,
            128,
            128,
            ae::Vec2::ZERO,
            ae::Vec2::new(128.0, 128.0),
        );
        assert!(aabbs.is_empty());
    }

    /// `bounding_aabb` returns a tight envelope around a list of
    /// AABBs. Verifies the combat_size derivation path collapses
    /// multi-part bodies into one for movement / clamping.
    #[test]
    fn bounding_aabb_envelops_disjoint_parts() {
        let parts = vec![
            ae::Aabb::new(ae::Vec2::new(0.0, 0.0), ae::Vec2::new(10.0, 10.0)),
            ae::Aabb::new(ae::Vec2::new(50.0, 0.0), ae::Vec2::new(10.0, 10.0)),
        ];
        let bound = bounding_aabb(&parts).expect("non-empty input");
        // Parts span x=[-10,10] and x=[40,60]; envelope x=[-10,60].
        // Y is the same [-10, 10] for both.
        assert!((bound.center().x - 25.0).abs() < 1e-3);
        assert!((bound.center().y - 0.0).abs() < 1e-3);
        let half = bound.half_size();
        assert!((half.x - 35.0).abs() < 1e-3);
        assert!((half.y - 10.0).abs() < 1e-3);
    }

    #[test]
    fn bounding_aabb_returns_none_for_empty_input() {
        assert!(bounding_aabb(&[]).is_none());
    }

    /// Doubling the spawn size doubles the derived world AABB on
    /// both axes (with identical sprite metadata). Pins the
    /// "boss in the intro is 2× larger" change — the intro arena's
    /// BossSpawn went from 64×80 → 128×160 and the runtime's
    /// combat_size derived from the SAME `body_pixel_bbox` MUST
    /// scale 2× in both dimensions. If this test breaks the
    /// sprite-metadata-driven body math diverged from the spawn
    /// AABB.
    /// End-to-end pin: `damageable_volumes` MUST return the
    /// per-animation hurtbox when the boss's sprite metrics
    /// carries one for the current animation. If a future change
    /// breaks the wire (derive doesn't copy `animations`, the
    /// consumer's lookup falls through silently, the
    /// boss_animation_for_profile mapping drops, etc.) the cyan
    /// debug box stops growing during attacks — which is the
    /// exact regression the user just reported.
    ///
    /// Builds a fake `BossSpriteMetrics` with a clearly-distinct
    /// per-animation hurtbox for `side_sweep`, sets
    /// `attack_state.active_profile = Some(SideSweep)`, and
    /// asserts the consumer returns an AABB matching the wide
    /// `side_sweep` hurtbox (~128 wide) rather than the static
    /// `body_pixel_bbox` (~106 wide).
    #[test]
    fn damageable_volumes_uses_per_animation_hurtbox_during_attack() {
        use crate::boss_encounter::behavior::{BossBehaviorProfile, BossSpriteMetrics};
        use crate::brain::{BossAttackProfile, BossAttackState};
        use crate::presentation::character_sprites::registry::{
            AnimationBox, AnimationMetrics, PixelRect,
        };
        use std::collections::HashMap;

        // Build a sprite-metrics snapshot with a distinct
        // `side_sweep` hurtbox (much wider than the static body
        // bbox) so we can prove the consumer picked the
        // per-animation one.
        let mut animations: HashMap<String, AnimationMetrics> = HashMap::new();
        animations.insert(
            "side_sweep".to_string(),
            AnimationMetrics {
                hurtbox: Some(AnimationBox {
                    parts: Vec::new(),
                    bbox: Some(PixelRect {
                        x: 1,
                        y: 5,
                        w: 127,
                        h: 86,
                    }),
                    frames: Vec::new(),
                }),
                hitbox: None,
                frame_duration_secs: None,
            },
        );
        let metrics = BossSpriteMetrics {
            frame_width: 128,
            frame_height: 128,
            body_pixel_bbox: Some(PixelRect {
                x: 8,
                y: 5,
                w: 106,
                h: 83,
            }),
            body_pixel_parts: Vec::new(),
            // Match the BOSS_SHEET render: `max(boss.size) * 1.6`
            // = `160 * 1.6` = `256` for a (128,160) spawn.
            sprite_render_size: ae::Vec2::new(256.0, 256.0),
            // Test fixture: zero offset keeps `boss.aabb()` centered
            // on `boss.pos` (the pre-offset behavior) so the
            // half-size assertion below doesn't have to factor in
            // body-center bias.
            combat_offset: ae::Vec2::ZERO,
            animations,
        };

        let mut behavior = BossBehaviorProfile::clockwork_warden();
        behavior.combat_size = Some(ae::Vec2::new(54.0, 56.0));
        let mut attack_state = BossAttackState::default();
        attack_state.active_profile = Some(BossAttackProfile::SideSweep);

        let ctx = BossVolumeContext {
            pos: ae::Vec2::new(640.0, 656.0),
            size: ae::Vec2::new(128.0, 160.0),
            combat_size: ae::Vec2::new(54.0, 56.0),
            behavior: &behavior,
            attack_state: &attack_state,
            sprite_metrics: Some(&metrics),
            animation_frame: None,
        };
        let volumes = damageable_volumes(&ctx);
        assert_eq!(volumes.len(), 1);
        let half = volumes[0].half_size();
        // side_sweep hurtbox: 127 wide / 128 frame × 256 render =
        // 254 wide. Half = 127. Static body bbox at render scale
        // would give 106/2 * 2 = 106. So we expect half.x > 120 to
        // pin the per-animation path.
        assert!(
            half.x > 120.0,
            "expected per-animation side_sweep hurtbox (wider than static body); got half.x = {} (would be ~106 if falling back to body_pixel_bbox)",
            half.x,
        );
    }

    /// Pin the scale-to-render-size fix: when `sprite_render_size`
    /// is 2× `ctx.size`, the cyan hurtbox must be 2× bigger than
    /// when `sprite_render_size` is zeroed (legacy path). Without
    /// this, the user's complaint — "in the sprites the box covers
    /// the boss head, but in game it is the old boxes" — comes back
    /// because the visible sprite renders 1.6× bigger than `boss.size`
    /// but the hurtbox would scale by `boss.size` only.
    #[test]
    fn damageable_volumes_samples_per_frame_hurtbox_from_animation_elapsed() {
        use crate::boss_encounter::behavior::{BossBehaviorProfile, BossSpriteMetrics};
        use crate::brain::{BossAttackProfile, BossAttackState};
        use crate::presentation::character_sprites::registry::{
            AnimationBox, AnimationBoxFrame, AnimationMetrics, NamedPixelRect,
        };
        use std::collections::HashMap;

        let mut animations: HashMap<String, AnimationMetrics> = HashMap::new();
        animations.insert(
            "gnu_head_descent".to_string(),
            AnimationMetrics {
                frame_duration_secs: Some(0.1),
                hurtbox: Some(AnimationBox {
                    // Coarse fallback is deliberately different from
                    // the sampled frame so the assertion proves that
                    // the per-frame path was taken.
                    parts: vec![NamedPixelRect {
                        name: "head".to_string(),
                        x: 45,
                        y: 45,
                        w: 10,
                        h: 10,
                    }],
                    bbox: None,
                    frames: vec![
                        AnimationBoxFrame {
                            parts: vec![NamedPixelRect {
                                name: "head".to_string(),
                                x: 45,
                                y: 10,
                                w: 10,
                                h: 10,
                            }],
                            bbox: None,
                        },
                        AnimationBoxFrame {
                            parts: vec![NamedPixelRect {
                                name: "head".to_string(),
                                x: 45,
                                y: 30,
                                w: 10,
                                h: 10,
                            }],
                            bbox: None,
                        },
                    ],
                }),
                hitbox: None,
            },
        );
        let metrics = BossSpriteMetrics {
            frame_width: 100,
            frame_height: 100,
            body_pixel_bbox: None,
            body_pixel_parts: Vec::new(),
            sprite_render_size: ae::Vec2::new(100.0, 100.0),
            combat_offset: ae::Vec2::ZERO,
            animations,
        };
        let behavior = BossBehaviorProfile::gnu_ton();
        let mut attack_state = BossAttackState::default();
        attack_state.active_profile = Some(BossAttackProfile::HeadDescent);
        attack_state.active_elapsed = 0.15; // frame index 1 at 0.1s/frame.

        let ctx = BossVolumeContext {
            pos: ae::Vec2::ZERO,
            size: ae::Vec2::new(100.0, 100.0),
            combat_size: ae::Vec2::new(100.0, 100.0),
            behavior: &behavior,
            attack_state: &attack_state,
            sprite_metrics: Some(&metrics),
            animation_frame: None,
        };

        let volumes = damageable_volumes(&ctx);
        assert_eq!(volumes.len(), 1);
        assert!(
            (volumes[0].center().y - -15.0).abs() < 1e-3,
            "expected frame-1 head center y=-15, got {:?}",
            volumes[0]
        );
    }

    #[test]
    fn animation_frame_sample_overrides_elapsed_frame_for_authored_boxes() {
        use crate::boss_encounter::behavior::{BossBehaviorProfile, BossSpriteMetrics};
        use crate::brain::{BossAttackProfile, BossAttackState};
        use crate::presentation::character_sprites::registry::{
            AnimationBox, AnimationBoxFrame, AnimationMetrics, NamedPixelRect,
        };
        use std::collections::HashMap;

        let mut animations: HashMap<String, AnimationMetrics> = HashMap::new();
        animations.insert(
            "gnu_head_descent".to_string(),
            AnimationMetrics {
                frame_duration_secs: Some(0.1),
                hurtbox: Some(AnimationBox {
                    parts: Vec::new(),
                    bbox: None,
                    frames: vec![
                        AnimationBoxFrame {
                            parts: vec![NamedPixelRect {
                                name: "head".to_string(),
                                x: 45,
                                y: 10,
                                w: 10,
                                h: 10,
                            }],
                            bbox: None,
                        },
                        AnimationBoxFrame {
                            parts: vec![NamedPixelRect {
                                name: "head".to_string(),
                                x: 45,
                                y: 70,
                                w: 10,
                                h: 10,
                            }],
                            bbox: None,
                        },
                    ],
                }),
                hitbox: None,
            },
        );
        let metrics = BossSpriteMetrics {
            frame_width: 100,
            frame_height: 100,
            body_pixel_bbox: None,
            body_pixel_parts: Vec::new(),
            sprite_render_size: ae::Vec2::new(100.0, 100.0),
            combat_offset: ae::Vec2::ZERO,
            animations,
        };
        let behavior = BossBehaviorProfile::gnu_ton();
        let mut attack_state = BossAttackState::default();
        attack_state.active_profile = Some(BossAttackProfile::HeadDescent);
        attack_state.active_elapsed = 0.15; // elapsed alone would pick frame 1.
        let visual_frame = BossAnimationFrameSample {
            profile: Some(BossAttackProfile::HeadDescent),
            frame_index: 0,
            animation_key: Some("gnu_head_descent"),
        };

        let ctx = BossVolumeContext {
            pos: ae::Vec2::ZERO,
            size: ae::Vec2::new(100.0, 100.0),
            combat_size: ae::Vec2::new(100.0, 100.0),
            behavior: &behavior,
            attack_state: &attack_state,
            sprite_metrics: Some(&metrics),
            animation_frame: Some(&visual_frame),
        };

        let volumes = damageable_volumes(&ctx);
        assert_eq!(volumes.len(), 1);
        assert!(
            (volumes[0].center().y - -35.0).abs() < 1e-3,
            "expected visual frame 0 head center y=-35, got {:?}",
            volumes[0]
        );
    }

    #[test]
    fn idle_rest_hurtbox_follows_the_live_animation_frame() {
        // Regression for the "GNU-ton head hurtbox locks to frame 0
        // while idle" bug. At rest there is no active/telegraph
        // profile, so `damageable_volumes` used to sample the rest
        // animation at elapsed 0 → frame 0 forever, even as the
        // rendered breathing pose bobbed. An idle `BossAnimationFrameSample`
        // (`profile: None`) now feeds the live frame index through.
        use crate::boss_encounter::behavior::{BossBehaviorProfile, BossSpriteMetrics};
        use crate::brain::BossAttackState;
        use crate::presentation::character_sprites::registry::{
            AnimationBox, AnimationBoxFrame, AnimationMetrics, NamedPixelRect,
        };
        use std::collections::HashMap;

        let mut animations: HashMap<String, AnimationMetrics> = HashMap::new();
        animations.insert(
            "rest".to_string(),
            AnimationMetrics {
                frame_duration_secs: Some(0.1),
                hurtbox: Some(AnimationBox {
                    parts: Vec::new(),
                    bbox: None,
                    frames: vec![
                        AnimationBoxFrame {
                            parts: vec![NamedPixelRect {
                                name: "head".to_string(),
                                x: 45,
                                y: 10, // center 15 → world y -35
                                w: 10,
                                h: 10,
                            }],
                            bbox: None,
                        },
                        AnimationBoxFrame {
                            parts: vec![NamedPixelRect {
                                name: "head".to_string(),
                                x: 45,
                                y: 70, // center 75 → world y +25
                                w: 10,
                                h: 10,
                            }],
                            bbox: None,
                        },
                    ],
                }),
                hitbox: None,
            },
        );
        let metrics = BossSpriteMetrics {
            frame_width: 100,
            frame_height: 100,
            body_pixel_bbox: None,
            body_pixel_parts: Vec::new(),
            sprite_render_size: ae::Vec2::new(100.0, 100.0),
            combat_offset: ae::Vec2::ZERO,
            animations,
        };
        let behavior = BossBehaviorProfile::gnu_ton();
        // Fully idle: no active or telegraph profile.
        let attack_state = BossAttackState::default();

        // Without a sample, elapsed 0 locks to frame 0 (y = -35).
        let ctx0 = BossVolumeContext {
            pos: ae::Vec2::ZERO,
            size: ae::Vec2::new(100.0, 100.0),
            combat_size: ae::Vec2::new(100.0, 100.0),
            behavior: &behavior,
            attack_state: &attack_state,
            sprite_metrics: Some(&metrics),
            animation_frame: None,
        };
        let v0 = damageable_volumes(&ctx0);
        assert_eq!(v0.len(), 1);
        assert!(
            (v0[0].center().y - -35.0).abs() < 1e-3,
            "idle without a sample should hold frame 0 (y=-35), got {:?}",
            v0[0]
        );

        // An idle sample (profile None) at frame 1 bobs the hurtbox down.
        let idle_frame = BossAnimationFrameSample {
            profile: None,
            frame_index: 1,
            animation_key: Some("rest"),
        };
        let ctx1 = BossVolumeContext {
            animation_frame: Some(&idle_frame),
            ..ctx0
        };
        let v1 = damageable_volumes(&ctx1);
        assert_eq!(v1.len(), 1);
        assert!(
            (v1[0].center().y - 25.0).abs() < 1e-3,
            "idle sample frame 1 should move the head hurtbox to y=+25, got {:?}",
            v1[0]
        );
    }

    #[test]
    fn gnu_head_descent_accepts_visual_row_alias_for_runtime_boxes() {
        use crate::boss_encounter::behavior::{BossBehaviorProfile, BossSpriteMetrics};
        use crate::brain::{BossAttackProfile, BossAttackState};
        use crate::presentation::character_sprites::registry::{
            AnimationBox, AnimationBoxFrame, AnimationMetrics, NamedPixelRect,
        };
        use std::collections::HashMap;

        let mut animations: HashMap<String, AnimationMetrics> = HashMap::new();
        animations.insert(
            "head_down".to_string(),
            AnimationMetrics {
                frame_duration_secs: Some(0.1),
                hurtbox: Some(AnimationBox {
                    parts: Vec::new(),
                    bbox: None,
                    frames: vec![
                        AnimationBoxFrame {
                            parts: vec![NamedPixelRect {
                                name: "head".to_string(),
                                x: 40,
                                y: 10,
                                w: 20,
                                h: 20,
                            }],
                            bbox: None,
                        },
                        AnimationBoxFrame {
                            parts: vec![NamedPixelRect {
                                name: "head".to_string(),
                                x: 40,
                                y: 70,
                                w: 20,
                                h: 20,
                            }],
                            bbox: None,
                        },
                    ],
                }),
                hitbox: Some(AnimationBox {
                    parts: Vec::new(),
                    bbox: None,
                    frames: vec![
                        AnimationBoxFrame {
                            parts: vec![NamedPixelRect {
                                name: "head_hit".to_string(),
                                x: 35,
                                y: 5,
                                w: 30,
                                h: 30,
                            }],
                            bbox: None,
                        },
                        AnimationBoxFrame {
                            parts: vec![NamedPixelRect {
                                name: "head_hit".to_string(),
                                x: 35,
                                y: 65,
                                w: 30,
                                h: 30,
                            }],
                            bbox: None,
                        },
                    ],
                }),
            },
        );

        let metrics = BossSpriteMetrics {
            frame_width: 100,
            frame_height: 100,
            body_pixel_bbox: None,
            body_pixel_parts: Vec::new(),
            sprite_render_size: ae::Vec2::new(100.0, 100.0),
            combat_offset: ae::Vec2::ZERO,
            animations,
        };
        let behavior = BossBehaviorProfile::gnu_ton();
        let mut attack_state = BossAttackState::default();
        attack_state.active_profile = Some(BossAttackProfile::HeadDescent);
        attack_state.active_elapsed = 0.15;
        let ctx = BossVolumeContext {
            pos: ae::Vec2::ZERO,
            size: ae::Vec2::new(100.0, 100.0),
            combat_size: ae::Vec2::new(100.0, 100.0),
            behavior: &behavior,
            attack_state: &attack_state,
            sprite_metrics: Some(&metrics),
            animation_frame: None,
        };

        let hurt = damageable_volumes(&ctx);
        assert_eq!(hurt.len(), 1);
        assert!(
            (hurt[0].center().y - 30.0).abs() < 1e-3,
            "expected head_down alias hurtbox frame 1 at y=30, got {:?}",
            hurt[0]
        );

        let hit = active_attack_volumes(&ctx);
        assert_eq!(hit.len(), 1);
        assert!(
            (hit[0].center().y - 30.0).abs() < 1e-3,
            "expected head_down alias hitbox frame 1 at y=30, got {:?}",
            hit[0]
        );
    }

    #[test]
    fn damageable_volumes_scales_to_sprite_render_size() {
        use crate::boss_encounter::behavior::{BossBehaviorProfile, BossSpriteMetrics};
        use crate::brain::BossAttackState;
        use crate::engine_core::AabbExt;
        use crate::presentation::character_sprites::registry::PixelRect;
        use std::collections::HashMap;

        let bbox = PixelRect {
            x: 8,
            y: 5,
            w: 106,
            h: 83,
        };
        let behavior = BossBehaviorProfile::clockwork_warden();
        let attack_state = BossAttackState::default();

        let legacy_metrics = BossSpriteMetrics {
            frame_width: 128,
            frame_height: 128,
            body_pixel_bbox: Some(bbox),
            body_pixel_parts: Vec::new(),
            // Zero render size → consumer falls back to ctx.size
            // (the pre-fix behavior).
            sprite_render_size: ae::Vec2::ZERO,
            combat_offset: ae::Vec2::ZERO,
            animations: HashMap::new(),
        };
        let render_metrics = BossSpriteMetrics {
            frame_width: 128,
            frame_height: 128,
            body_pixel_bbox: Some(bbox),
            body_pixel_parts: Vec::new(),
            sprite_render_size: ae::Vec2::new(256.0, 256.0),
            combat_offset: ae::Vec2::ZERO,
            animations: HashMap::new(),
        };

        let legacy_ctx = BossVolumeContext {
            pos: ae::Vec2::ZERO,
            size: ae::Vec2::new(128.0, 160.0),
            combat_size: ae::Vec2::new(54.0, 56.0),
            behavior: &behavior,
            attack_state: &attack_state,
            sprite_metrics: Some(&legacy_metrics),
            animation_frame: None,
        };
        let render_ctx = BossVolumeContext {
            pos: ae::Vec2::ZERO,
            size: ae::Vec2::new(128.0, 160.0),
            combat_size: ae::Vec2::new(54.0, 56.0),
            behavior: &behavior,
            attack_state: &attack_state,
            sprite_metrics: Some(&render_metrics),
            animation_frame: None,
        };
        let legacy = damageable_volumes(&legacy_ctx)[0];
        let render = damageable_volumes(&render_ctx)[0];

        // ctx.size = (128, 160) → scale (1, 1.25) → body half (53, 51.875).
        // sprite_render_size = (256, 256) → scale (2, 2) → body half (106, 83).
        // Render must be ~2× legacy on x and ≥1.5× on y.
        let lx = legacy.half_size().x;
        let rx = render.half_size().x;
        let ly = legacy.half_size().y;
        let ry = render.half_size().y;
        assert!(
            rx > lx * 1.8,
            "sprite_render_size scaling should ~2× the x half-extent; legacy={lx} render={rx}",
        );
        assert!(
            ry > ly * 1.5,
            "sprite_render_size scaling should ≥1.5× the y half-extent; legacy={ly} render={ry}",
        );
    }

    #[test]
    fn world_space_body_aabbs_doubles_when_spawn_doubles() {
        let bbox = PixelRect {
            x: 8,
            y: 5,
            w: 106,
            h: 83,
        };
        let half_at_1x = world_space_body_aabbs_from_parts(
            &[],
            Some(bbox),
            128,
            128,
            ae::Vec2::ZERO,
            ae::Vec2::new(64.0, 80.0),
        )[0]
        .half_size();
        let half_at_2x = world_space_body_aabbs_from_parts(
            &[],
            Some(bbox),
            128,
            128,
            ae::Vec2::ZERO,
            ae::Vec2::new(128.0, 160.0),
        )[0]
        .half_size();
        let ratio_x = half_at_2x.x / half_at_1x.x;
        let ratio_y = half_at_2x.y / half_at_1x.y;
        assert!(
            (ratio_x - 2.0).abs() < 1e-3,
            "2× spawn must produce 2× x-extent; got ratio {ratio_x}",
        );
        assert!(
            (ratio_y - 2.0).abs() < 1e-3,
            "2× spawn must produce 2× y-extent; got ratio {ratio_y}",
        );
    }

    /// Larger world_size (e.g. boss lab boss at 150×185 vs intro at
    /// 64×80) scales the body AABB proportionally — the same pixel
    /// bbox yields a bigger world AABB. This is the scaling promise
    /// of the sprite-metadata-driven approach: one source of body
    /// shape, multiple sizes.
    #[test]
    fn world_space_body_aabbs_scales_with_world_size() {
        let bbox = PixelRect {
            x: 8,
            y: 8,
            w: 112,
            h: 112,
        };
        let small = world_space_body_aabbs_from_parts(
            &[],
            Some(bbox),
            128,
            128,
            ae::Vec2::ZERO,
            ae::Vec2::new(64.0, 80.0), // first_system_boss spawn size
        );
        let large = world_space_body_aabbs_from_parts(
            &[],
            Some(bbox),
            128,
            128,
            ae::Vec2::ZERO,
            ae::Vec2::new(150.0, 185.0), // boss-lab spawn size
        );
        let small_half = small[0].half_size();
        let large_half = large[0].half_size();
        // Same fraction of the frame → large should be roughly the
        // ratio (150/64, 185/80) bigger than small.
        let ratio_x = large_half.x / small_half.x;
        let ratio_y = large_half.y / small_half.y;
        assert!((ratio_x - 150.0 / 64.0).abs() < 1e-3);
        assert!((ratio_y - 185.0 / 80.0).abs() < 1e-3);
    }
}
