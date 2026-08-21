//! **FEEL TUNING — the numbers that decide how the game FEELS, in the crate that
//! owns the rules they modify.**
//!
//! ⭐⭐ **carved out of `ambition_platformer2d_actor_monolith` on 2026-08-21
//! (D33), and the line count is beside the point.** Jon: *"loc is the proxy. the
//! real win is conceptual domain separation."* This struct had NO monolith
//! coupling at all — both `crate::` paths inside it already resolved into
//! `ambition_combat` (`events::DEFAULT_*` and `events::FeatureCombatTuning`).
//! It was simply sitting above the crate that owns everything it touches.
//!
//! ⛔⛔ **and FOUR crates BELOW the monolith were already depending on it in
//! PROSE, which is the shape that made this urgent.** `ambition_combat`,
//! `ambition_platformer2d_core`, `ambition_platformer2d_shared_tangle` and
//! `ambition_platformer2d_runtime` all cite its fields by name in comments and
//! then RESTATE its constants locally, because they could not name the type:
//!
//! ```text
//! core/movement/tests/glide_and_air.rs   const HITSTUN_S = 0.24;  // `…::enemy_hitstun_time`
//! shared_tangle/camera_ease.rs           "…restating 0.070 in this crate"
//! combat/rules.rs                        "`…::default().di_max_angle`"
//! ```
//!
//! A doc block naming your invariant IS a dependency — one the compiler cannot
//! check and that drifts silently the day a number changes.
//!
//! ✔ **FOLLOWED THROUGH the same day, and only ONE of the four could be fixed**
//! — the claim above said "three", and measuring it said otherwise.
//! `ResolvedCombatTuning::default()` READS `di_max_angle` from here now
//! (`6cbf5d2de`) instead of writing `0.0` beside a comment claiming they match.
//! ⚠ the other three CANNOT: `platformer2d_core`'s glide test and
//! `shared_tangle::camera_ease` sit BELOW this crate and cannot name the type at
//! all, so their restatements are the only option available. ⛔ that is
//! structural, not unfinished work, and not a reason to add a dependency edge.
//!
//! ⚠ **the type keeps its name for now, and the name is wrong.** Nothing here is
//! a monolith any more. Renaming is mechanical and compiler-checked across ~116
//! references, and it is deliberately NOT bundled with a move — a rename diff
//! and a relocation diff are hard to review together.

//! Sandbox game-feel tuning.
//!
//! Holds the live-tunable resource that gameplay systems read for time scales,
//! input windows, knockback, hitstun, and combat windup/active timings. These
//! are gameplay knobs (not dev/inspector toggles), so they live in their own
//! module rather than under `dev_tools`.

use bevy::prelude::*;

/// Live-tunable time/input/combat feel values consumed by sandbox gameplay.
#[derive(Resource, Reflect, Clone, Copy, Debug)]
#[reflect(Resource)]
pub struct Platformer2dFeelTuningMonolith {
    pub bullet_time_scale: f32,
    pub blink_hold_slow_scale: f32,
    pub debug_slowmo_scale: f32,
    pub time_ramp_down_rate: f32,
    pub time_ramp_up_rate: f32,
    pub down_double_tap_window: f32,
    pub up_double_tap_window: f32,
    pub interaction_buffer_time: f32,
    /// **Hitlag at a reference-strength connect** — the freeze BOTH bodies take
    /// when a strike lands, scaled by how hard it landed.
    ///
    /// ⛔ this replaces `attack_hitstop_time` (0.055, attacker) and
    /// `player_damage_hitstop_time` (0.070, victim): two unscaled constants at
    /// two sites for one event.
    pub hitlag_time: f32,
    pub reset_flash_time: f32,
    pub edge_transition_cooldown: f32,
    pub door_transition_cooldown: f32,
    pub edge_transition_flash: f32,
    pub door_transition_flash: f32,
    /// Seconds of warning before a basement enemy attack becomes harmful.
    pub enemy_attack_windup: f32,
    /// Seconds an enemy attack hitbox remains active after windup.
    pub enemy_attack_active: f32,
    /// Seconds of warning before a basement boss pattern becomes harmful.
    pub boss_attack_windup: f32,
    /// Seconds a boss attack pattern remains active after windup.
    pub boss_attack_active: f32,
    /// Horizontal velocity applied when normal enemies hurt the player.
    pub enemy_knockback_x: f32,
    /// Upward velocity applied when normal enemies hurt the player.
    pub enemy_knockback_y: f32,
    /// Horizontal velocity applied when bosses hurt the player.
    pub boss_knockback_x: f32,
    /// Upward velocity applied when bosses hurt the player.
    pub boss_knockback_y: f32,
    /// Player-control scale while in hitstun; 0 is no movement authority.
    pub hitstun_control_scale: f32,
    /// Hitstun duration for ordinary enemy/body hits.
    pub enemy_hitstun_time: f32,
    /// Hitstun duration for boss hits.
    pub boss_hitstun_time: f32,
    /// Short HARD control-lock at the start of a knockback: the player is being
    /// thrown and has no input authority — can't steer back in (incl. flight),
    /// can't jump/dash/blink, can't attack. Once it clears the player regains
    /// the attack verb while `*_hitstun_time` / `knockback_invulnerability_time`
    /// keep ticking, so you can swing back the instant the recoil ends — the
    /// Hollow-Knight "get bopped out, then fight back while flashing" feel.
    /// Distinct from hitstun (the longer, softer partial-movement window).
    pub knockback_recoil_lock_time: f32,
    /// **THE METEOR LOCK** — how long a body spiked out of the AIR cannot
    /// recover. A floor under [`Self::knockback_recoil_lock_time`], never an
    /// addition: a meteor is a longer version of the same silence.
    ///
    /// ⚠ **the value here is a BASELINE that an experience overwrites**, exactly
    /// like `di_max_angle` beside it — `DeclaredCombatRules::meteor_lock_time`
    /// is the authority and the damage road folds it in before use. `0.0` is no
    /// meteor rule, which is what an exploration game wants.
    pub meteor_lock_time: f32,
    /// **What a CROUCHING victim multiplies an incoming launch by** — crouch
    /// cancel. Folded in from `DeclaredCombatRules::crouch_cancel_scale` by the
    /// damage road, exactly like `meteor_lock_time` beside it. `1.0` is no
    /// crouch cancel, which is what an exploration game wants.
    pub crouch_cancel_scale: f32,
    /// Post-hit invulnerability after enemy/boss knockback.
    pub knockback_invulnerability_time: f32,
    /// Post-respawn invulnerability after lava/spike-style hazard recovery.
    pub hazard_respawn_invulnerability_time: f32,
    /// Directional-influence budget (CM2), radians: the maximum the victim's
    /// held control may rotate its OWN knockback launch. Reads the victim's
    /// `ActorControl.locomotion` (the same gated input every system reads), so
    /// DI works identically for humans, brains, and RL policies. DEFAULT `0.0`
    /// = no DI (Ambition today, byte-parity); a fighter mode (Super Smash
    /// Siblings) authors a smash-like ≈ 0.31 (18°) to turn it on.
    pub di_max_angle: f32,
}

impl Default for Platformer2dFeelTuningMonolith {
    fn default() -> Self {
        Self {
            bullet_time_scale: 0.125,
            blink_hold_slow_scale: 0.35,
            debug_slowmo_scale: 0.25,
            time_ramp_down_rate: 5.0,
            time_ramp_up_rate: 14.0,
            down_double_tap_window: 0.24,
            up_double_tap_window: 0.30,
            interaction_buffer_time: 0.120,
            hitlag_time: 0.070,
            reset_flash_time: 0.18,
            edge_transition_cooldown: 0.14,
            door_transition_cooldown: 0.16,
            edge_transition_flash: 0.24,
            door_transition_flash: 0.24,
            enemy_attack_windup: crate::events::DEFAULT_ENEMY_ATTACK_WINDUP,
            enemy_attack_active: crate::events::DEFAULT_ENEMY_ATTACK_ACTIVE,
            boss_attack_windup: crate::events::DEFAULT_BOSS_ATTACK_WINDUP,
            boss_attack_active: crate::events::DEFAULT_BOSS_ATTACK_ACTIVE,
            enemy_knockback_x: 360.0,
            enemy_knockback_y: 260.0,
            boss_knockback_x: 460.0,
            boss_knockback_y: 330.0,
            meteor_lock_time: 0.0,
            crouch_cancel_scale: 1.0,
            hitstun_control_scale: 0.18,
            enemy_hitstun_time: 0.24,
            boss_hitstun_time: 0.36,
            knockback_recoil_lock_time: 0.12,
            knockback_invulnerability_time: 0.75,
            hazard_respawn_invulnerability_time: 1.10,
            // DI off by default — Ambition's PvE knockback is unchanged; a
            // fighter demo authors a nonzero budget to enable it.
            di_max_angle: 0.0,
        }
    }
}

impl Platformer2dFeelTuningMonolith {
    pub fn feature_combat_tuning(self) -> crate::events::FeatureCombatTuning {
        crate::events::FeatureCombatTuning {
            enemy_attack_windup: self.enemy_attack_windup,
            enemy_attack_active: self.enemy_attack_active,
            boss_attack_windup: self.boss_attack_windup,
            boss_attack_active: self.boss_attack_active,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_finite_and_positive_where_expected() {
        let f = Platformer2dFeelTuningMonolith::default();
        // Time-domain scales between (0, 1] (slow-mo etc.).
        assert!(f.bullet_time_scale > 0.0 && f.bullet_time_scale <= 1.0);
        assert!(f.blink_hold_slow_scale > 0.0 && f.blink_hold_slow_scale <= 1.0);
        assert!(f.debug_slowmo_scale > 0.0 && f.debug_slowmo_scale <= 1.0);
        // Hitstun control scale is also < 1 (player loses authority briefly).
        assert!(f.hitstun_control_scale >= 0.0 && f.hitstun_control_scale < 1.0);
        // Time windows / cooldowns are positive.
        assert!(f.down_double_tap_window > 0.0);
        assert!(f.up_double_tap_window > 0.0);
        assert!(f.interaction_buffer_time > 0.0);
        // Boss attack windups should be longer than enemy windups
        // (otherwise the boss telegraph is less readable than a
        // basic enemy's, which would surprise playtesters).
        assert!(f.boss_attack_windup > f.enemy_attack_windup);
        // Boss knockback / hitstun should be punchier than enemy.
        assert!(f.boss_knockback_x > f.enemy_knockback_x);
        assert!(f.boss_hitstun_time >= f.enemy_hitstun_time);
        // The recoil control-lock is a brief hard lock at the FRONT of the
        // hitstun window, so it must be positive and shorter than the (base)
        // hitstun it sits inside — otherwise it would outlast the window it's
        // supposed to be the opening of.
        assert!(f.knockback_recoil_lock_time > 0.0);
        assert!(f.knockback_recoil_lock_time < f.boss_hitstun_time);
        // Hazard respawn invuln should be at least as long as
        // knockback invuln (ordinary contact is less punishing than
        // a hazard wipe).
        assert!(f.hazard_respawn_invulnerability_time >= f.knockback_invulnerability_time);
    }

    #[test]
    fn feature_combat_tuning_extracts_attack_windows() {
        let f = Platformer2dFeelTuningMonolith::default();
        let combat = f.feature_combat_tuning();
        assert_eq!(combat.enemy_attack_windup, f.enemy_attack_windup);
        assert_eq!(combat.enemy_attack_active, f.enemy_attack_active);
        assert_eq!(combat.boss_attack_windup, f.boss_attack_windup);
        assert_eq!(combat.boss_attack_active, f.boss_attack_active);
    }

    #[test]
    fn time_ramp_recovers_faster_than_it_slows() {
        // Entering slow-mo should be readable (a slower ramp-down lets
        // the player feel it kick in); recovering to normal speed
        // should be snappy. This invariant guards against accidentally
        // swapping the two in defaults.
        let f = Platformer2dFeelTuningMonolith::default();
        assert!(
            f.time_ramp_up_rate > f.time_ramp_down_rate,
            "time_ramp_up_rate should be faster than time_ramp_down_rate \
             so recovery feels snappy",
        );
    }

    #[test]
    fn transition_cooldowns_match_their_flash_durations_or_shorter() {
        // A cooldown shorter than the flash means the player could
        // re-enter a transition while the flash from the previous one
        // is still on screen — visible double-trigger.
        let f = Platformer2dFeelTuningMonolith::default();
        assert!(f.edge_transition_flash >= f.edge_transition_cooldown);
        assert!(f.door_transition_flash >= f.door_transition_cooldown);
    }

    #[test]
    fn attack_active_window_is_at_least_one_frame() {
        // 60fps frame is ~16.6ms = 0.017s. Any active hitbox window
        // shorter than a frame would be unhittable; not a useful state.
        let f = Platformer2dFeelTuningMonolith::default();
        assert!(f.enemy_attack_active >= 0.017);
        assert!(f.boss_attack_active >= 0.017);
    }
}
