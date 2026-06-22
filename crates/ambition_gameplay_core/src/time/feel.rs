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
pub struct SandboxFeelTuning {
    pub bullet_time_scale: f32,
    pub blink_hold_slow_scale: f32,
    pub debug_slowmo_scale: f32,
    pub time_ramp_down_rate: f32,
    pub time_ramp_up_rate: f32,
    pub down_double_tap_window: f32,
    pub up_double_tap_window: f32,
    pub interaction_buffer_time: f32,
    pub attack_hitstop_time: f32,
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
    /// Post-hit invulnerability after enemy/boss knockback.
    pub knockback_invulnerability_time: f32,
    /// Post-respawn invulnerability after lava/spike-style hazard recovery.
    pub hazard_respawn_invulnerability_time: f32,
    /// Hitstop on the receiving side of enemy/boss damage.
    pub player_damage_hitstop_time: f32,
}

impl Default for SandboxFeelTuning {
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
            attack_hitstop_time: 0.055,
            reset_flash_time: 0.18,
            edge_transition_cooldown: 0.14,
            door_transition_cooldown: 0.16,
            edge_transition_flash: 0.24,
            door_transition_flash: 0.24,
            enemy_attack_windup: 0.36,
            enemy_attack_active: 0.20,
            boss_attack_windup: 0.52,
            boss_attack_active: 0.32,
            enemy_knockback_x: 360.0,
            enemy_knockback_y: 260.0,
            boss_knockback_x: 460.0,
            boss_knockback_y: 330.0,
            hitstun_control_scale: 0.18,
            enemy_hitstun_time: 0.24,
            boss_hitstun_time: 0.36,
            knockback_recoil_lock_time: 0.12,
            knockback_invulnerability_time: 0.75,
            hazard_respawn_invulnerability_time: 1.10,
            player_damage_hitstop_time: 0.070,
        }
    }
}

impl SandboxFeelTuning {
    pub fn feature_combat_tuning(self) -> crate::features::FeatureCombatTuning {
        crate::features::FeatureCombatTuning {
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
        let f = SandboxFeelTuning::default();
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
        let f = SandboxFeelTuning::default();
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
        let f = SandboxFeelTuning::default();
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
        let f = SandboxFeelTuning::default();
        assert!(f.edge_transition_flash >= f.edge_transition_cooldown);
        assert!(f.door_transition_flash >= f.door_transition_cooldown);
    }

    #[test]
    fn attack_active_window_is_at_least_one_frame() {
        // 60fps frame is ~16.6ms = 0.017s. Any active hitbox window
        // shorter than a frame would be unhittable; not a useful state.
        let f = SandboxFeelTuning::default();
        assert!(f.enemy_attack_active >= 0.017);
        assert!(f.boss_attack_active >= 0.017);
    }
}
