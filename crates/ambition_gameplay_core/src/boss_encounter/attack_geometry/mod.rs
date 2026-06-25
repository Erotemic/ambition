//! Pure boss-attack volume math (no ECS, no mutation).
//!
//! Free functions that derive world-space AABBs for a boss's active strike,
//! telegraph, damageable hurtbox, and body-contact zone, then resolve the
//! per-tick boss -> player `HitEvent`. Inputs are bundled in
//! [`BossVolumeContext`] (body fields + `BossAttackState` + optional
//! [`ActorSpriteMetrics`] + an optional [`BossAnimationFrameSample`]); helpers
//! prefer sprite-author-declared hit/hurtboxes and fall back to
//! `volumes_for_profile`'s hardcoded geometry per `BossAttackProfile`.
//! Submodules: `aabb` (pixel-rect -> world-AABB derivation), `frame`
//! (animation-frame sampling). Distinct from the engine's collision system —
//! this is boss-attack-specific geometry only.

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;

use bevy::prelude::Component;

use crate::brain::{BossAttackProfile, BossAttackState};
use ambition_sprite_sheet::{AnimationBox, BodyMetrics, PixelRect};

use super::behavior::{ActorSpriteMetrics, BossBehaviorProfile};

mod aabb;
mod frame;
pub use aabb::*;
use frame::*;

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
    pub sprite_metrics: Option<&'a crate::boss_encounter::behavior::ActorSpriteMetrics>,
    /// Optional frame sample from the live boss sprite animator.
    /// When present and its profile matches the requested attack,
    /// sprite-authored hit/hurt boxes use this exact frame index
    /// instead of re-deriving a frame from attack timers. That keeps
    /// gameplay/debug boxes locked to the rendered animation frame.
    pub animation_frame: Option<&'a BossAnimationFrameSample>,
    /// Boss facing (sign of x). The sprite flips horizontally to face the
    /// player, so an off-center body's hurtboxes must mirror too — otherwise
    /// they land on the wrong side when the boss faces left. `1.0` = right
    /// (no mirror), `< 0.0` = flipped. See [`mirror_x_if_flipped`].
    pub facing: f32,
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
    /// Build the context from a live boss view + its attack-state component.
    /// The boss contributes only body fields, not policy; volume selection is
    /// data-driven via `sprite_metrics`.
    pub fn from_ref(boss: crate::features::BossRef<'a>, attack_state: &'a BossAttackState) -> Self {
        Self {
            pos: boss.kin.pos,
            size: boss.kin.size,
            combat_size: boss.combat_size(),
            behavior: &boss.config.behavior,
            attack_state,
            sprite_metrics: boss.status.sprite_metrics.as_ref(),
            animation_frame: None,
            facing: boss.kin.facing,
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

/// The currently-playing animation row, resolved for hit/hurt-box sampling:
/// the ordered candidate row keys to try, the elapsed time within that row
/// (for deriving a frame from `frame_duration`), and an optional exact frame
/// index from a live animator that overrides the elapsed derivation.
///
/// This is the ONE actor-specific input the shared hurtbox math needs — each
/// actor knows how it picks its current pose (a boss maps an attack profile to
/// rows; a player/enemy just reports its current animation), but once resolved
/// the world-space volume derivation is identical for all of them.
pub struct AnimationSelection {
    pub keys: Vec<&'static str>,
    pub elapsed_s: f32,
    pub live_frame_index: Option<usize>,
}

/// Actor-neutral surface the shared combat-geometry math reads to derive an
/// actor's collision box and damageable hurtbox. Player, NPC, Enemy, and Boss
/// each implement it; the boss is just the richest impl (its `hurtbox_selection`
/// folds in the attack-profile → animation-row mapping). Engine-first: another
/// platformer's actor type unifies onto the same volume math by implementing
/// this trait.
pub trait CombatGeometry {
    fn body_pos(&self) -> ae::Vec2;
    /// LDtk spawn size — the fallback world scale when no sprite render size
    /// was captured, and the size of the legacy single-AABB hurtbox.
    fn body_size(&self) -> ae::Vec2;
    fn facing(&self) -> f32;
    /// Collision envelope; defaults to the body size.
    fn combat_size(&self) -> ae::Vec2 {
        self.body_size()
    }
    /// World offset from `body_pos` to the collision-box center (off-center
    /// bodies). Mirrored with facing by the implementor. Defaults to zero.
    fn combat_offset(&self) -> ae::Vec2 {
        ae::Vec2::ZERO
    }
    /// The actor's reference-frame "down": gravity at its position, or a clung
    /// surface normal for a wall-walker. The body/hurt box orients to this so a
    /// sideways-gravity body's footprint lies along the wall — the relativity
    /// principle. Defaults to screen-down `(0, 1)`; the box is identity under
    /// vertical gravity, so upright play is byte-for-byte unchanged.
    fn frame_down(&self) -> ae::Vec2 {
        ae::Vec2::new(0.0, 1.0)
    }
    fn sprite_metrics(&self) -> Option<&ActorSpriteMetrics>;
    /// The current pose for hurtbox sampling (rest/idle when not attacking).
    fn hurtbox_selection(&self) -> AnimationSelection;
}

/// An actor's collision AABB — its combat-size body box oriented to its
/// reference frame and shifted by any off-center `combat_offset`. The single
/// way to ask "where is this actor's body" across player / NPC / enemy / boss.
pub fn collision_aabb(g: &impl CombatGeometry) -> ae::Aabb {
    let half = ae::AccelerationFrame::new(g.frame_down()).to_world_half(g.combat_size() * 0.5);
    ae::Aabb::new(g.body_pos() + g.combat_offset(), half)
}

/// A minimal [`CombatGeometry`] for an actor whose hurtbox is just its
/// frame-oriented collision box — no per-animation sprite metrics. This is the
/// player and ordinary enemies today: build it from a body's pos / size /
/// facing and its reference-frame down, and `damageable_volumes` /
/// [`collision_aabb`] yield the same box they used before, now through the one
/// shared path. (Sprite metrics — pose-accurate, multi-part hurtboxes — are a
/// later opt-in: populate them and the same call lights up automatically.)
pub struct SimpleActorGeometry {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub facing: f32,
    pub frame_down: ae::Vec2,
}

impl CombatGeometry for SimpleActorGeometry {
    fn body_pos(&self) -> ae::Vec2 {
        self.pos
    }
    fn body_size(&self) -> ae::Vec2 {
        self.size
    }
    fn facing(&self) -> f32 {
        self.facing
    }
    fn frame_down(&self) -> ae::Vec2 {
        self.frame_down
    }
    fn sprite_metrics(&self) -> Option<&ActorSpriteMetrics> {
        None
    }
    fn hurtbox_selection(&self) -> AnimationSelection {
        AnimationSelection {
            keys: Vec::new(),
            elapsed_s: 0.0,
            live_frame_index: None,
        }
    }
}

impl CombatGeometry for BossVolumeContext<'_> {
    fn body_pos(&self) -> ae::Vec2 {
        self.pos
    }
    fn body_size(&self) -> ae::Vec2 {
        self.size
    }
    fn facing(&self) -> f32 {
        self.facing
    }
    fn combat_size(&self) -> ae::Vec2 {
        self.combat_size
    }
    fn sprite_metrics(&self) -> Option<&ActorSpriteMetrics> {
        self.sprite_metrics
    }
    fn hurtbox_selection(&self) -> AnimationSelection {
        // The current animation is the live strike's, else the windup's, else
        // rest. Matches the visible sprite pose so a side-sweep's extended arms
        // register as damageable while the rest pose's tight bbox wins idle.
        let active_profile = self
            .attack_state
            .active_profile
            .as_ref()
            .or(self.attack_state.telegraph_profile.as_ref());
        let keys = runtime_animation_keys(self, active_profile, &["rest"]);
        let elapsed_s = if self.attack_state.active_profile.is_some() {
            self.attack_state.active_elapsed
        } else if self.attack_state.telegraph_profile.is_some() {
            self.attack_state.telegraph_elapsed
        } else {
            0.0
        };
        // A live frame sample overrides elapsed derivation only when it matches
        // the pose being sampled (same profile, or an idle sample for rest).
        let live_frame_index = self.animation_frame.and_then(|sample| match active_profile {
            Some(profile) => {
                (sample.profile.as_ref() == Some(profile)).then_some(sample.frame_index)
            }
            None => sample.profile.is_none().then_some(sample.frame_index),
        });
        AnimationSelection {
            keys,
            elapsed_s,
            live_frame_index,
        }
    }
}

/// Active strike volumes — drawn red in the debug overlay and tested
/// against the player body by the damage system. Returns empty when
/// no strike is live (`attack_state.active_profile == None`).
///
/// Priority: sprite-author-declared per-animation hitbox (from
/// `ActorSpriteMetrics::animations[animation_name].hitbox`) wins
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

/// Pull sprite-author-declared hitbox rectangles for the given
/// attack profile from `ctx.sprite_metrics.animations`. Returns
/// `None` (not empty) when the sprite has no hitbox for this
/// animation; the caller falls back to the hardcoded
/// `volumes_for_profile` math. Returns an empty `Vec` when the
/// sprite has an entry but no usable rects (defensive).

/// Damageable hurtbox volumes — where the player's attacks register
/// as hits. Single-piece bosses use one AABB derived from
/// combat_size; multi-part bosses (sprite RON carrying
/// `body_pixel_parts`) emit one AABB per piece so head/body/arms
/// hit independently. Animation boxes may also carry per-frame
/// samples so large moving parts like GNU-ton's head can track the
/// drawn pose instead of one coarse per-animation rectangle.
/// Reflect each AABB's center across the vertical line `axis_x` when `facing`
/// is leftward (`< 0`), leaving sizes unchanged. The boss sprite mirrors to
/// face the player, so an off-center body's hit/hurt boxes must mirror with it;
/// for a centered body this is a no-op (center already on the axis).
pub(crate) fn mirror_x_if_flipped(
    mut aabbs: Vec<ae::Aabb>,
    axis_x: f32,
    facing: f32,
) -> Vec<ae::Aabb> {
    if facing >= 0.0 {
        return aabbs;
    }
    for aabb in &mut aabbs {
        let c = aabb.center();
        let half = aabb.half_size();
        *aabb = ae::Aabb::new(ae::Vec2::new(2.0 * axis_x - c.x, c.y), half);
    }
    aabbs
}

pub fn damageable_volumes(g: &impl CombatGeometry) -> Vec<ae::Aabb> {
    mirror_x_if_flipped(damageable_volumes_unmirrored(g), g.body_pos().x, g.facing())
}

/// Body hurtbox volumes in the sprite's UNFLIPPED frame. `damageable_volumes`
/// mirrors these to the actor's current facing. Actor-neutral: every input is
/// read through the [`CombatGeometry`] trait, so player / enemy / boss share
/// one hurtbox derivation.
fn damageable_volumes_unmirrored(g: &impl CombatGeometry) -> Vec<ae::Aabb> {
    // Priority:
    //   1. Per-animation hurtbox for the currently-playing animation
    //      (attack frames with extended arms get a wider hurtbox than the
    //      rest pose; a multi-part actor's per-pose rows carve out body
    //      pieces — e.g. GNU-ton's head-only descent hurtbox).
    //   2. Static `body_pixel_parts` (multi-rect body for disjointed actors).
    //   3. Static `body_pixel_bbox` (single-rect alpha bbox).
    //   4. `combat_size`-driven fallback (actors without sprite metadata).
    if let Some(metrics) = g.sprite_metrics() {
        // Scale pixel rects to the visible sprite size, not the smaller LDtk
        // spawn AABB. See `sprite_world_size` for the rationale.
        let world_size = sprite_world_size(metrics, g.body_size());
        let pos = g.body_pos();
        // (1) Per-animation hurtbox for the actor's current pose. The actor
        // resolves which row(s) it is showing; the sampling is uniform.
        let sel = g.hurtbox_selection();
        for active_anim in &sel.keys {
            let Some(entry) = metrics.animations.get(*active_anim) else {
                continue;
            };
            let Some(box_) = entry.hurtbox.as_ref() else {
                continue;
            };
            if !box_.is_populated() {
                continue;
            }
            // A live animator frame wins; otherwise derive from elapsed.
            let frame_index = sel
                .live_frame_index
                .or_else(|| animation_frame_index(entry, sel.elapsed_s));
            let aabbs = world_space_animation_box_aabbs(
                box_,
                frame_index,
                metrics.frame_width,
                metrics.frame_height,
                pos,
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
                    pos,
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
                pos,
                world_size,
            )];
        }
    }
    // (4) Fallback: combat_size-driven single AABB, oriented to the actor's
    // reference frame (identity under vertical gravity, so bosses — which keep
    // the default screen-down frame — are unchanged).
    let half = ae::AccelerationFrame::new(g.frame_down()).to_world_half(g.combat_size() * 0.5);
    vec![ae::Aabb::new(g.body_pos(), half)]
}

/// Body-contact damage AABB at the boss's combat envelope — body contact is
/// "you ran into the boss", not a discrete strike.
pub fn body_damage_aabb(pos: ae::Vec2, combat_size: ae::Vec2) -> ae::Aabb {
    ae::Aabb::new(pos, combat_size * 0.5)
}

/// Compute the per-tick boss → player damage event, if any.
///
/// Pure: reads the brain's `BossAttackState` (which strike is live,
/// which profile) + the boss body fields + the behavior's damage
/// scalars.
///
/// Returns `Some(HitEvent)` when:
///   - A strike is live (`attack_state.active_profile.is_some()`)
///     and one of its volumes overlaps `player_body`, OR
///   - The boss body has positive `body_damage` and overlaps the
///     player.
///
/// Body contact wins only if the strike arm didn't fire.
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
    use crate::combat::events::{HitEvent, HitKnockback, HitMode, HitSource, HitTarget};
    use crate::combat::util::midpoint;
    use ambition_engine_core::AabbExt;

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
                volume: volume.into(),
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

    // Body-contact arm: only fires when no strike landed, and only when the
    // behavior opts into body damage.
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
        // Mirror the body offset to the boss's facing (the sprite flips), so the
        // contact zone matches the visible body on both sides — consistent with
        // `BossRef::combat_offset` and the mirrored hurtboxes above.
        let combat_offset = if ctx.facing < 0.0 {
            ae::Vec2::new(-combat_offset.x, combat_offset.y)
        } else {
            combat_offset
        };
        let body = body_damage_aabb(ctx.pos + combat_offset, ctx.combat_size);
        if body.strict_intersects(player_body) {
            return Some(HitEvent {
                volume: body.into(),
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
    // `combat_size` is the only size input the volume arms need.
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
        // `boss_attack_damage`'s strike arm.
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
// GNU-ton's per-animation hit/hurt-box geometry lives in
// `gnu_ton_boss_spritesheet.ron`'s `body_metrics.animations` map, derived via
// the generic `world_aabb_from_pixel_rect` pixel→world transform (the same one
// the gradient sentinel uses).

#[cfg(test)]
mod sprite_metadata_derivation_tests;

#[cfg(test)]
mod simple_geometry_tests {
    use super::*;

    fn geom(frame_down: ae::Vec2) -> SimpleActorGeometry {
        SimpleActorGeometry {
            pos: ae::Vec2::new(10.0, 20.0),
            size: ae::Vec2::new(30.0, 48.0),
            facing: 1.0,
            frame_down,
        }
    }

    #[test]
    fn upright_gravity_is_the_plain_centered_box() {
        // Under screen-down gravity the oriented body box is identity — upright
        // play stays byte-for-byte the old `kin.aabb()` / CenteredAabb.
        let aabb = collision_aabb(&geom(ae::Vec2::new(0.0, 1.0)));
        assert_eq!(aabb.center(), ae::Vec2::new(10.0, 20.0));
        assert_eq!(aabb.half_size(), ae::Vec2::new(15.0, 24.0));
        // The single damageable volume agrees with the collision box.
        let vols = damageable_volumes(&geom(ae::Vec2::new(0.0, 1.0)));
        assert_eq!(vols.len(), 1);
        assert_eq!(vols[0].half_size(), ae::Vec2::new(15.0, 24.0));
    }

    #[test]
    fn sideways_gravity_lays_the_body_along_the_wall() {
        // Under sideways gravity the footprint rotates: width<->height swap so
        // the body lies along the wall (the relativity principle). Same box the
        // gizmo's `aabb_oriented` draws.
        let aabb = collision_aabb(&geom(ae::Vec2::new(1.0, 0.0)));
        assert_eq!(aabb.center(), ae::Vec2::new(10.0, 20.0));
        assert_eq!(aabb.half_size(), ae::Vec2::new(24.0, 15.0));
    }
}
