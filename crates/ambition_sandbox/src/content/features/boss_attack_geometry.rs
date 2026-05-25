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

use ambition_engine as ae;

use crate::brain::{BossAttackProfile, BossAttackState};
use crate::presentation::character_sprites::registry::{BodyMetrics, PixelRect};

use super::bosses::{BossBehaviorProfile, BossRuntime, GNU_TON_ANCHOR_Y};

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
    pub is_gnu_ton: bool,
    pub behavior: &'a BossBehaviorProfile,
    pub attack_state: &'a BossAttackState,
    /// Sprite-driven body metrics. `Some` for bosses whose sprite
    /// RON carries `body_metrics` and the derivation system has
    /// snapshotted it. `damageable_volumes` prefers multi-rect
    /// hurtboxes from here over the legacy single-AABB fallback.
    pub sprite_metrics: Option<&'a super::bosses::BossSpriteMetrics>,
}

impl<'a> BossVolumeContext<'a> {
    /// Build the context from a live boss runtime + its attack-state
    /// component. The runtime contributes only body fields (pos /
    /// size / combat_size / is_gnu_ton), not policy.
    pub fn from_runtime(boss: &'a BossRuntime, attack_state: &'a BossAttackState) -> Self {
        Self {
            pos: boss.pos,
            size: boss.size,
            combat_size: boss.combat_size(),
            is_gnu_ton: boss.is_gnu_ton(),
            behavior: &boss.behavior,
            attack_state,
            sprite_metrics: boss.sprite_metrics.as_ref(),
        }
    }
}

/// Active strike volumes — drawn red in the debug overlay and tested
/// against the player body by the damage system. Returns empty when
/// no strike is live (`attack_state.active_profile == None`).
pub fn active_attack_volumes(ctx: &BossVolumeContext) -> Vec<ae::Aabb> {
    let Some(profile) = ctx.attack_state.active_profile.as_ref() else {
        return Vec::new();
    };
    volumes_for_profile(
        profile,
        ctx.pos,
        ctx.size,
        ctx.combat_size,
        ctx.behavior,
        ctx.is_gnu_ton,
    )
}

/// Telegraph volumes — drawn yellow in the debug overlay. Returns
/// empty when nothing is currently telegraphing.
pub fn telegraph_volumes(ctx: &BossVolumeContext) -> Vec<ae::Aabb> {
    let Some(profile) = ctx.attack_state.telegraph_profile.as_ref() else {
        return Vec::new();
    };
    volumes_for_profile(
        profile,
        ctx.pos,
        ctx.size,
        ctx.combat_size,
        ctx.behavior,
        ctx.is_gnu_ton,
    )
}

/// Damageable hurtbox volumes — where the player's attacks register
/// as hits. Single-piece bosses use one AABB derived from
/// combat_size; multi-part bosses (sprite RON carrying
/// `body_pixel_parts`) emit one AABB per piece so head/body/arms
/// hit independently. GNU-ton's hand-tuned head/descent path
/// stays as-is until the multi-rect metadata is authored.
pub fn damageable_volumes(ctx: &BossVolumeContext) -> Vec<ae::Aabb> {
    // Sprite-driven multi-rect hurtboxes win when available. The
    // sprite RON's `body_pixel_parts` is the authored multi-part
    // representation for disjointed-piece characters (giant boss
    // with head + body + arms). Each part becomes its own
    // hurtbox so the player's attacks register on each piece
    // independently.
    if !ctx.is_gnu_ton {
        if let Some(metrics) = ctx.sprite_metrics {
            if !metrics.body_pixel_parts.is_empty() {
                let mut parts = Vec::with_capacity(metrics.body_pixel_parts.len());
                for part in &metrics.body_pixel_parts {
                    parts.push(world_aabb_from_pixel_rect(
                        part.rect(),
                        metrics.frame_width,
                        metrics.frame_height,
                        ctx.pos,
                        ctx.size,
                    ));
                }
                return parts;
            }
            if let Some(bbox) = metrics.body_pixel_bbox {
                return vec![world_aabb_from_pixel_rect(
                    bbox,
                    metrics.frame_width,
                    metrics.frame_height,
                    ctx.pos,
                    ctx.size,
                )];
            }
        }
        // Legacy fallback: combat_size-driven single AABB.
        return vec![ae::Aabb::new(ctx.pos, ctx.combat_size * 0.5)];
    }
    // GNU-ton's head is always damageable (the descent windows just
    // move it down to player level so the player doesn't have to
    // climb). Pre-migration this returned an empty list outside the
    // GnuHeadDescent strike, which made the boss invulnerable in
    // Phase1 (no descent beat) and therefore unkillable.
    let head_descending = matches!(
        ctx.attack_state.active_profile,
        Some(BossAttackProfile::GnuHeadDescent)
    ) || matches!(
        ctx.attack_state.telegraph_profile,
        Some(BossAttackProfile::GnuHeadDescent)
    );
    let head_design_y = if head_descending {
        // Held-low position during descent telegraph + strike.
        // Matches the generator's `_draw_head_down` target y=30.
        30.0
    } else {
        // Rest position high above the shoulder. Matches the
        // generator's REST_HEAD_Y.
        -75.0
    };
    vec![gnu_ton_part_aabb(
        ctx.pos,
        ctx.size,
        ae::Vec2::new(0.0, head_design_y),
        ae::Vec2::new(92.0, 74.0),
    )]
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
/// Returns `Some(PlayerDamageEvent)` when:
///   - A strike is live (`attack_state.active_profile.is_some()`)
///     and one of its volumes overlaps `player_body`, OR
///   - The boss body has positive `body_damage` and overlaps the
///     player.
///
/// Body contact wins only if the strike arm didn't fire — same
/// priority order as the legacy `BossRuntime::player_damage`.
pub fn boss_attack_damage(
    ctx: &BossVolumeContext,
    player_body: ae::Aabb,
) -> Option<crate::features::PlayerDamageEvent> {
    use super::util::midpoint;
    use crate::features::{PlayerDamageEvent, PlayerDamageMode, PlayerDamageSource};
    use ambition_engine::AabbExt;

    // Strike arm: the brain's `active_profile` is the single source
    // of truth for "there's a live boss hitbox right now".
    if ctx.attack_state.active_profile.is_some() {
        let volumes = active_attack_volumes(ctx);
        if let Some(volume) = volumes
            .into_iter()
            .find(|volume| volume.strict_intersects(player_body))
        {
            let signum_or = |x: f32, fallback: f32| {
                if x.abs() < f32::EPSILON {
                    fallback
                } else {
                    x.signum()
                }
            };
            return Some(PlayerDamageEvent {
                mode: PlayerDamageMode::Knockback,
                source: PlayerDamageSource::BossAttack,
                source_pos: ctx.pos,
                impact_pos: midpoint(player_body.center(), volume.center()),
                knockback_dir: signum_or(player_body.center().x - ctx.pos.x, 1.0),
                strength: 1.25,
                amount: ctx.behavior.attack_damage.max(1),
                // Boss AI targets primary today; per-target routing
                // arrives with OVERNIGHT-TODO #17.6.
                target: None,
            });
        }
    }

    // Body-contact arm: same priority as the legacy
    // `BossRuntime::player_damage` — only fires when no strike
    // landed, and only when the behavior opts into body damage.
    let body_damage_amount = ctx.behavior.body_damage;
    if body_damage_amount > 0 {
        let body = body_damage_aabb(ctx.pos, ctx.combat_size);
        if body.strict_intersects(player_body) {
            let signum_or = |x: f32, fallback: f32| {
                if x.abs() < f32::EPSILON {
                    fallback
                } else {
                    x.signum()
                }
            };
            return Some(PlayerDamageEvent {
                mode: PlayerDamageMode::Knockback,
                source: PlayerDamageSource::BossBody,
                source_pos: ctx.pos,
                impact_pos: midpoint(player_body.center(), body.center()),
                knockback_dir: signum_or(player_body.center().x - ctx.pos.x, 1.0),
                strength: 1.0,
                amount: body_damage_amount,
                target: None,
            });
        }
    }
    None
}

/// World-space hitbox volumes for a specific attack profile. Pure
/// function of the profile + body fields. GNU-ton dispatches to its
/// part-anchored math; ordinary bosses use the generic
/// origin + offset shapes.
pub fn volumes_for_profile(
    attack: &BossAttackProfile,
    pos: ae::Vec2,
    size: ae::Vec2,
    combat_size: ae::Vec2,
    behavior: &BossBehaviorProfile,
    is_gnu_ton: bool,
) -> Vec<ae::Aabb> {
    if is_gnu_ton {
        // Design-space anchors match the regenerated 768×576 GNU-ton
        // sprite: hands rest at x=±235, slam strike peaks at y=195
        // (below the leg hooves at design +175), shockwave fires at
        // floor level.
        match attack {
            BossAttackProfile::GnuHandSlam => {
                return vec![
                    gnu_ton_part_aabb(
                        pos,
                        size,
                        ae::Vec2::new(-235.0, 195.0),
                        ae::Vec2::new(78.0, 60.0),
                    ),
                    gnu_ton_part_aabb(
                        pos,
                        size,
                        ae::Vec2::new(235.0, 195.0),
                        ae::Vec2::new(78.0, 60.0),
                    ),
                ];
            }
            BossAttackProfile::GnuHandSweep => {
                return vec![
                    gnu_ton_part_aabb(
                        pos,
                        size,
                        ae::Vec2::new(-185.0, 20.0),
                        ae::Vec2::new(140.0, 60.0),
                    ),
                    gnu_ton_part_aabb(
                        pos,
                        size,
                        ae::Vec2::new(185.0, 20.0),
                        ae::Vec2::new(140.0, 60.0),
                    ),
                ];
            }
            BossAttackProfile::GnuHeadDescent => {
                return vec![gnu_ton_part_aabb(
                    pos,
                    size,
                    ae::Vec2::new(0.0, 30.0),
                    ae::Vec2::new(92.0, 74.0),
                )];
            }
            BossAttackProfile::GnuShockwave => {
                return vec![gnu_ton_part_aabb(
                    pos,
                    size,
                    ae::Vec2::new(0.0, 195.0),
                    ae::Vec2::new(300.0, 18.0),
                )];
            }
            // Apple-rain damage routes through the spawned projectile
            // bodies, not a stationary AABB on the boss. Returning
            // empty here keeps `apply_boss_attack_damage` from
            // double-counting contact-on-boss while the apples are
            // in flight.
            BossAttackProfile::GnuAppleRain => {
                return Vec::new();
            }
            // Gradient Sentinel profiles never land on GNU-ton (the
            // boss is single-encounter), but a defensive empty arm
            // keeps the match exhaustive without crashing if some
            // future test fixture cross-wires the encounters.
            BossAttackProfile::OverfitVolley
            | BossAttackProfile::MinimaTrap
            | BossAttackProfile::SaddlePoint
            | BossAttackProfile::GradientCascade
            | BossAttackProfile::GradientLane => {
                return Vec::new();
            }
            _ => {}
        }
    }
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
        BossAttackProfile::GradientLane => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, 0.0),
            ae::Vec2::new(size.x * 0.30, size.y * 1.80),
        )],
        // Specials' damage routes through their EFFECTS consumers
        // (spawned projectiles / World-anchored hitboxes / minions).
        // Empty volumes here prevent double-counting via
        // `boss_attack_damage`'s strike arm.
        BossAttackProfile::OverfitVolley
        | BossAttackProfile::MinimaTrap
        | BossAttackProfile::SaddlePoint
        | BossAttackProfile::GradientCascade => Vec::new(),
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
        BossAttackProfile::GnuHandSlam => vec![
            ae::Aabb::new(
                origin + ae::Vec2::new(-size.x * 0.40, size.y * 0.25),
                ae::Vec2::new(size.x * 0.14, size.y * 0.60),
            ),
            ae::Aabb::new(
                origin + ae::Vec2::new(size.x * 0.40, size.y * 0.25),
                ae::Vec2::new(size.x * 0.14, size.y * 0.60),
            ),
        ],
        BossAttackProfile::GnuHandSweep => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.15),
            ae::Vec2::new(size.x * 0.85, size.y * 0.28),
        )],
        BossAttackProfile::GnuHeadDescent => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.05),
            ae::Vec2::new(size.x * 0.32, size.y * 0.38),
        )],
        BossAttackProfile::GnuShockwave => vec![ae::Aabb::new(
            origin + ae::Vec2::new(0.0, size.y * 0.48),
            ae::Vec2::new(size.x * 0.90, size.y * 0.08),
        )],
        BossAttackProfile::GnuAppleRain => Vec::new(),
    }
}

/// GNU-ton part-AABB math. Pure function of body pos + sprite size
/// + design-space coordinates from the sprite generator.
pub fn gnu_ton_part_aabb(
    pos: ae::Vec2,
    size: ae::Vec2,
    design_center: ae::Vec2,
    design_half_size: ae::Vec2,
) -> ae::Aabb {
    let scale = gnu_ton_sprite_scale(size);
    let center = pos
        + ae::Vec2::new(
            design_center.x * scale,
            (design_center.y - GNU_TON_ANCHOR_Y) * scale,
        );
    ae::Aabb::new(center, design_half_size * scale)
}

const GNU_TON_COLLISION_SCALE: f32 = 4.5;
const GNU_TON_FRAME_HEIGHT: f32 = 576.0;

fn gnu_ton_sprite_scale(collision_size: ae::Vec2) -> f32 {
    collision_size.x.max(collision_size.y).max(8.0) * GNU_TON_COLLISION_SCALE / GNU_TON_FRAME_HEIGHT
}

#[cfg(test)]
mod sprite_metadata_derivation_tests {
    use super::*;
    use crate::presentation::character_sprites::registry::{NamedPixelRect, PixelRect};
    use ambition_engine::AabbExt;

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
