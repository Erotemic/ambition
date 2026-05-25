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

use super::bosses::{BossBehaviorProfile, BossRuntime, GNU_TON_ANCHOR_Y};

/// All the per-tick inputs the volume helpers need. Owned by the
/// caller so the helpers themselves stay pure.
pub struct BossVolumeContext<'a> {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub combat_size: ae::Vec2,
    pub is_gnu_ton: bool,
    pub behavior: &'a BossBehaviorProfile,
    pub attack_state: &'a BossAttackState,
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
/// as hits. Most bosses are damageable on their whole body AABB;
/// GNU-ton exposes only the head, which descends to player level
/// during a `GnuHeadDescent` telegraph or strike.
pub fn damageable_volumes(ctx: &BossVolumeContext) -> Vec<ae::Aabb> {
    if !ctx.is_gnu_ton {
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
