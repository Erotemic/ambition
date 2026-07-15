//! Boss runtime glue for the actor simulation: the [`boss_special_for_profile`]
//! resolver that maps a `BossAttackProfile::Special(key)` to a
//! `SpecialActionSpec` (the open seam `tick_boss_brains_system` uses to emit
//! `ActorActionMessage::Special`). The boss PROFILE/pattern vocabulary and
//! sprite metrics now live in `ambition_characters::brain::boss_pattern` and
//! `crate::boss_encounter::behavior` and are re-exported here for legacy paths.

// Boss policy vocabulary (`BossMovementProfile`, `BossPatternStep`,
// `BossPattern`, `BossAttackPattern`, `BossAttackProfile`,
// `step_duration`) moved to `ambition_characters::brain::boss_pattern` per the
// "move boss policy out of BossRuntime" migration. Re-exported here
// because `BossBehaviorProfile` and the volumes / construction code
// below still reference them by their old `content::features::bosses`
// path — those references stay legal via the re-export while call
// sites migrate to the brain-module path at their leisure.
#[cfg(test)]
use ambition_characters::brain::boss_pattern::BossAttackPattern;
pub use ambition_characters::brain::boss_pattern::{BossAttackProfile, BossMovementProfile};
// `BossPattern` and `BossPatternStep` only show up inside the
// scripted profiles, which now live in `boss_profiles.ron`. They're
// still publicly accessible via `ambition_characters::brain::boss_pattern`; we
// just don't re-export them here anymore.

// `BossTickOutputs` (previously: `projectile_spawns: Vec<…>`) was
// deleted with Task B of the actor/brain follow-up plan. Apple-rain
// spawning moved to `spawn_apple_rain_from_special_messages` (an
// EFFECTS-stage consumer driven by `ActorActionMessage::Special`).
// Future boss specials follow the same pattern — one consumer per
// `SpecialActionSpec` variant — instead of accumulating side-channel
// `Vec`s the caller flushes.

// All boss-special tuning numbers (apple-rain cadence, overfit-volley sampling,
// minima-trap / saddle-point / gradient-cascade params, the eye-beam tuning)
// moved to `ambition_content::bosses::specials` with the Techniques themselves —
// the engine names no boss special's params. The engine retains only the generic
// boss machinery (profile/spec/resolver) below.

// `GNU_TON_ANCHOR_Y`, `GNU_TON_COLLISION_SCALE`, `GNU_TON_FRAME_HEIGHT`,
// and `gnu_ton_sprite_scale` were retired in the 2026-05-26
// data-driven migration. The GNU-ton per-animation hit / hurt-box
// geometry lives in `gnu_ton_boss_spritesheet.ron`'s
// `body_metrics.animations` map and flows through the generic
// `world_aabb_from_pixel_rect` transform the gradient sentinel uses.

// `BossBehaviorProfile` / `BossRewardProfile` / `ActorSpriteMetrics` /
// `canonical_boss_id_from` / `boss_animation_keys_for_profile` moved to
// `crate::boss_encounter::behavior` (Stage 20 / A2 stretch): the boss
// PROFILE vocabulary is machinery (data-driven via boss_profiles.ron);
// this module keeps the named special-spec resolver + tuning consts.
#[cfg(test)]
use crate::boss_encounter::behavior::canonical_boss_id_from;
pub use crate::boss_encounter::behavior::{
    boss_animation_keys_for_profile, ActorSpriteMetrics, BossBehaviorProfile, BossRewardProfile,
};

/// Boss-side resolver for `Special`-flavored `BossAttackProfile`s.
///
/// The Gradient Sentinel carries multiple distinct specials
/// (MemorizedVolley, PitTrap, RotatingCross, MinionCascade) — more
/// than the single `ActionSet::special` slot can express. Rather
/// than grow `ActionSet` or `ActorControlFrame` for one boss, the
/// `tick_boss_brains_system` calls this function when the brain
/// commits to a special-flavored profile and writes the resulting
/// `ActorActionMessage::Special { spec }` directly via
/// `MessageWriter`. The boss's `ActionSet.special` is set to `None`
/// for multi-special bosses so the generic resolver doesn't fire a
/// duplicate.
///
/// `None` means the profile doesn't have a registered special spec
/// — the consumer should treat that as a no-op (defensive against
/// schedule edits that introduce a profile before the spec wiring
/// lands).
pub fn boss_special_for_profile(
    profile: &ambition_characters::brain::BossAttackProfile,
) -> Option<ambition_characters::brain::SpecialActionSpec> {
    use ambition_characters::brain::SpecialActionSpec;
    // Open seam: a `Special` beat carries its content-technique key; the
    // brain emits it verbatim as `SpecialActionSpec::Special(key)` and the
    // matching content Technique reads its own params + emits the effects.
    // Ordinary (geometry) profiles never route through here — they damage
    // via `boss_attack_damage` reading `BossAttackState` directly, so they
    // map to `None`. The engine names no boss special.
    profile
        .special_key()
        .map(|key| SpecialActionSpec::Special(key.to_string()))
}

/// Aggressor push for a boss strike (matches the old `sync_boss_strike_hitboxes`
/// / `boss_attack_damage` strike arm). Carried on the geometry move's hit volume.
pub const BOSS_STRIKE_KNOCKBACK: f32 = 1.25;

/// Build the boss's data-driven attack MOVESET from its capability repertoire —
/// ONE moveset move per authored strike profile, so EVERY boss strike runs through
/// the SAME moveset runtime an actor's swing does (fable review §A1: the moveset is
/// the boss's melee system too, retiring the bespoke `sync_boss_strike_hitboxes`):
///
/// - A content-technique **`Special(key)`** profile → a move whose single window
///   SUSTAINS `Effect{key}` for the strike duration, so the technique fires every
///   frame the strike is live (the `apple_rain`-style per-frame signal) through the
///   `Effect{key}`→`Special{key}` bridge — no body-mounted hit volume.
/// - A **geometry** profile (FloorSlam / SideSweep / HazardColumn / …) → a move
///   whose Active window carries the profile's static hit volumes as BODY-LOCAL
///   [`HitVolume`]s, derived from `volumes_for_profile` at a body-local origin (the
///   world-space math cancels the boss position, leaving a constant local offset).
///   `advance_move_playback` then spawns/despawns the strike hitbox through the ONE
///   shared hitbox pipeline (`apply_hitbox_damage`'s Boss branch), exactly as the
///   old per-tick sync did — minus the sprite-frame-tracking geometry (a
///   parameterizable fidelity detail; the static fallback approximates it).
///
/// Keyed by [`BossAttackProfile::move_id`]; `trigger_boss_attack_moves` resolves the
/// active profile via `move_by_id`, not an input verb. `None` if the boss authors no
/// strike at all.
pub fn boss_attack_moveset(
    capability: &ambition_characters::brain::BossCapability,
    behavior: &BossBehaviorProfile,
    combat_size: ambition_engine_core::Vec2,
    telegraph_windows: &[(ambition_characters::brain::BossAttackProfile, f32)],
) -> Option<crate::combat::moveset::ActorMoveset> {
    use ambition_engine_core::AabbExt;
    use ambition_entity_catalog::{
        ClipBinding, EffectRef, HitVolume, MoveSpec, MoveWindow, MovesetContract, VolumeShape,
        WindowTag,
    };
    let telegraph_for = |profile: &ambition_characters::brain::BossAttackProfile| -> f32 {
        telegraph_windows
            .iter()
            .find(|(p, _)| p == profile)
            .map(|(_, t)| t.max(0.0))
            .unwrap_or(0.0)
    };
    let moves: Vec<MoveSpec> = capability
        .specials
        .iter()
        .filter_map(|(profile, strike_s)| {
            let strike_s = strike_s.max(0.05);
            // The move spans the whole telegraph→strike as ONE timeline: its Active
            // window opens at `tel` and closes at `tel + strike` (E53). A move started
            // at `t0 = tel` (strike edge / possession) is live immediately; one started
            // at `t0 = 0` plays the telegraph first. Either way the projected
            // `active_elapsed` folds in the telegraph offset because it reads the move's
            // own clock `t`.
            let tel = telegraph_for(profile);
            let active_start = tel;
            let active_end = tel + strike_s;
            let (volumes, sustain_effect) = if let Some(key) = profile.special_key() {
                (Vec::new(), Some(EffectRef::new(key)))
            } else {
                // Geometry strike: `volumes_for_profile` at a ZERO body origin yields
                // AABBs centered on the profile's body-local offset (the boss position
                // cancels: origin = pos + attack_origin_offset, center = origin +
                // offset, local = center - pos). Convert each to a body-local
                // `HitVolume` the move runtime mirrors by facing + rotates into the
                // gravity frame at spawn.
                let volumes: Vec<HitVolume> =
                    crate::boss_encounter::attack_geometry::volumes_for_profile(
                        profile,
                        ambition_engine_core::Vec2::ZERO,
                        combat_size,
                        behavior,
                    )
                    .into_iter()
                    .map(|aabb| {
                        let c = aabb.center();
                        let h = aabb.half_size();
                        HitVolume {
                            shape: VolumeShape::Rect {
                                offset: (c.x, c.y),
                                half_extents: (h.x, h.y),
                            },
                            damage: behavior.attack_damage.max(1),
                            knockback: BOSS_STRIKE_KNOCKBACK,
                            kb_growth: 0.0,
                            launch_dir: None,
                            on_hit: None,
                            // Boss geometry strikes are data-shaped volumes, not
                            // bladed swings: no slash VFX, no manifest override.
                            vfx: None,
                        }
                    })
                    .collect();
                // A geometry profile with no authored volume (defensive) contributes
                // no move — skip it rather than a hitless Active window.
                if volumes.is_empty() {
                    return None;
                }
                (volumes, None)
            };
            Some(MoveSpec {
                id: profile.move_id(),
                clip: ClipBinding {
                    clip: "attack".to_string(),
                    fallbacks: vec!["idle".to_string()],
                },
                duration_s: active_end,
                windows: vec![MoveWindow {
                    start_s: active_start,
                    end_s: active_end,
                    tag: WindowTag::Active,
                    volumes,
                    sustain_effect,
                }],
                events: Vec::new(),
                gates: Default::default(),
                start_impulse: None,
                smash_charge_mult: 1.0,
            })
        })
        .collect();
    (!moves.is_empty()).then(|| {
        crate::combat::moveset::ActorMoveset(MovesetContract {
            verbs: std::collections::BTreeMap::new(),
            moves,
        })
    })
}

// `step_duration` moved to `ambition_characters::brain::boss_pattern`.

#[cfg(test)]
mod boss_profile_data_tests;
#[cfg(test)]
mod boss_special_resolver_tests;
#[cfg(test)]
mod canonical_boss_id_tests;
#[cfg(test)]
mod scripted_pattern_tests;
