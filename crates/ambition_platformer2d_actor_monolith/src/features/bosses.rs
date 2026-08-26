//! Boss runtime glue for constructing the shared data-driven attack moveset.
//!
//! Boss brains emit [`BossAttackProfile`] intent; `trigger_boss_attack_moves`
//! starts the corresponding move, and `MovePlayback` is the sole attack timeline
//! for geometry and content-technique specials alike.

// TODO(compat-remove): migrate remaining boss-pattern callers to
// `ambition_characters::brain::boss_pattern`, then remove these re-exports.
#[cfg(test)]
use ambition_characters::brain::boss_pattern::BossAttackPattern;
pub use ambition_characters::brain::boss_pattern::{BossAttackProfile, BossMovementProfile};
// `BossPattern` and `BossPatternStep` only show up inside the
// scripted profiles, which now live in `boss_profiles.ron`. They're
// still publicly accessible via `ambition_characters::brain::boss_pattern`; we
// just don't re-export them here anymore.

// The engine retains only the generic boss machinery (profile/spec/resolver) below.

// TODO(compat-remove): migrate remaining behavior-profile callers to
// `ambition_boss_encounter::behavior`, then remove these re-exports.
#[cfg(test)]
use ambition_boss_encounter::behavior::canonical_boss_id_from;
pub use ambition_boss_encounter::behavior::{
    boss_animation_keys_for_profile, ActorSpriteMetrics, BossBehaviorProfile, BossRewardProfile,
};

/// Aggressor push for a boss strike (matches the old `sync_boss_strike_hitboxes`
/// / `boss_attack_damage` strike arm). Carried on the geometry move's hit volume.
pub const BOSS_STRIKE_KNOCKBACK: f32 = 1.25;

/// Timestamp for the telegraph's rising-edge `MoveEvent`s (BD3 cue/vfx). Move
/// events fire on the first clock crossing STRICTLY past `at_s`
/// (`at_s > t_prev`), so an exact `0.0` would never fire on a move started at
/// `t0 = 0`; one millisecond is inside the very first sim tick of any windup.
pub const TELEGRAPH_EDGE_S: f32 = 0.001;

/// Build the boss's data-driven attack MOVESET from its capability repertoire —
/// ONE moveset move per authored strike profile, so EVERY boss strike runs through
/// the SAME moveset runtime an actor's swing does (fable review §A1: the moveset is
/// the boss's melee system too, retiring the bespoke `sync_boss_strike_hitboxes`):
///
/// - A content-technique `Special(key)` profile → a move whose single window
///   SUSTAINS `Effect{key}` for the strike duration, so the technique fires every
///   frame the strike is live (the `apple_rain`-style per-frame signal) through the
///   `Effect{key}`→`Special{key}` bridge — no body-mounted hit volume.
/// - A geometry profile (FloorSlam / SideSweep / HazardColumn / …) → a move
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
    combat_size: ambition_platformer2d_core::Vec2,
    telegraph_windows: &[(
        ambition_characters::brain::BossAttackProfile,
        f32,
        Option<ambition_characters::brain::boss_pattern::TelegraphSpec>,
    )],
) -> Option<ambition_combat::moveset::ActorMoveset> {
    use ambition_entity_catalog::{
        ClipBinding, EffectRef, HitVolume, MoveEvent, MoveEventKind, MoveSpec, MoveWindow,
        MovesetContract, VolumeShape, WindowTag,
    };
    use ambition_platformer2d_core::AabbExt;
    let telegraph_for = |profile: &ambition_characters::brain::BossAttackProfile| -> (
        f32,
        Option<&ambition_characters::brain::boss_pattern::TelegraphSpec>,
    ) {
        telegraph_windows
            .iter()
            .find(|(p, _, _)| p == profile)
            .map(|(_, t, spec)| (t.max(0.0), spec.as_ref()))
            .unwrap_or((0.0, None))
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
            let (tel, telegraph_spec) = telegraph_for(profile);
            let active_start = tel;
            let active_end = tel + strike_s;
            // BD3 anticipation as MOVE data: the authored telegraph cue/vfx fire as
            // one-shot `MoveEvent`s on the windup's rising edge, through the SAME
            // `dispatch_move_events` channel every actor move uses. Authored just
            // after t=0 (events fire on the first clock crossing, `at_s > t_prev`);
            // a move started at the strike edge (`t0 = tel`) never crosses them, so
            // a skipped windup correctly plays no anticipation.
            let mut events: Vec<MoveEvent> = Vec::new();
            if tel > 0.0 {
                if let Some(spec) = telegraph_spec {
                    if let Some(cue) = spec.cue.clone() {
                        events.push(MoveEvent {
                            at_s: TELEGRAPH_EDGE_S,
                            kind: MoveEventKind::Sfx { cue },
                        });
                    }
                    if let Some(effect) = spec.vfx.clone() {
                        events.push(MoveEvent {
                            at_s: TELEGRAPH_EDGE_S,
                            kind: MoveEventKind::Vfx {
                                effect,
                                at: (0.0, 0.0),
                                scale: 1.0,
                                sfx: None,
                            },
                        });
                    }
                }
            }
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
                    ambition_boss_encounter::attack_geometry::volumes_for_profile(
                        profile,
                        ambition_platformer2d_core::Vec2::ZERO,
                        combat_size,
                        behavior,
                    )
                    .into_iter()
                    .map(|aabb| {
                        let c = aabb.center();
                        let h = aabb.half_size();
                        HitVolume {
                            // An ordinary hit, not a gust.
                            hit_sfx: None,
                            shape: VolumeShape::Rect {
                                offset: (c.x, c.y),
                                half_extents: (h.x, h.y),
                            },
                            damage: behavior.attack_damage.max(1),
                            knockback: BOSS_STRIKE_KNOCKBACK,
                            knockback_growth: None,
                            launch_dir: None,
                            on_hit: None,
                            // Boss geometry strikes are data-shaped volumes, not
                            // bladed swings: no slash VFX, no manifest override.
                            vfx: None,
                            reaction: None,
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
                // A boss move's label is its title-cased id.
                display_name: None,
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
                    // The boss's authored strike-speed throttle IS the move's
                    // motion lock: while the strike is live the body's steering
                    // intent is scaled down at integration, so the boss cannot
                    // outrun its own committed strike — for ANY controller
                    // (autonomous pattern or possessing player alike; formerly a
                    // brain-side damping that possession bypassed).
                    motion_scale: behavior.strike_speed_scale.clamp(0.0, 1.0),
                }],
                events,
                gates: Default::default(),
                start_impulse: None,
                smash_charge_mult: 1.0,
                smash_charge: None,
                charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
                repeat: None,
                landing_lag_s: None,
                autocancel_after_s: None,
                sprite_spin_hz: None,
            })
        })
        .collect();
    (!moves.is_empty()).then(|| {
        ambition_combat::moveset::ActorMoveset(MovesetContract {
            verbs: std::collections::BTreeMap::new(),
            moves,
        })
    })
}

// `step_duration` moved to `ambition_characters::brain::boss_pattern`.

#[cfg(test)]
mod boss_profile_data_tests;
#[cfg(test)]
mod canonical_boss_id_tests;
#[cfg(test)]
mod scripted_pattern_tests;
