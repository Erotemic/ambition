//! Live gameplay-feel tuning owned by the combat domain.
//!
//! These values control time scales, input windows, hit reactions, knockback, and
//! combat timings. They are gameplay parameters rather than developer-tool state.

use bevy::prelude::*;

/// The developer-editable MIRROR of [`Platformer2dFeelTuningMonolith`].
///
/// ⛔⛤ **THE INSPECTOR USED TO EDIT THE AUTHORITY ITSELF.**
/// `game/ambition_app/src/dev/mod.rs` installed
/// `ResourceInspectorPlugin::<Platformer2dFeelTuningMonolith>`, so bevy-inspector
/// wrote the resource deterministic simulation reads — no proposal, no
/// admission, no rebase — while `developer_edits_under_rollback.rs` already
/// carried `editing_feel_tuning_mid_timeline_changes_what_history_resimulates_to`
/// naming that exact hole. `Q120`'s inventory of "all five live editor roads" was
/// the list a review had named, not a census of the tree, so this one was never
/// in it. Found by the GPT architecture review 2026-09-14.
///
/// ⭐ The same three stages every other domain now has: editable mirror →
/// pending proposal → admitted authority. The monolith stays the neutral
/// mechanical value a composition without developer tools keeps.
#[derive(Resource, Reflect, Clone, Copy, Debug, Default, Deref, DerefMut)]
#[reflect(Resource)]
pub struct EditableFeelTuning(pub Platformer2dFeelTuningMonolith);

/// The marker type that OWNS this domain. Identity is the type, not the label.
pub struct FeelTuningDomain;

/// This domain's key in `PendingMechanicalEdits`, declared beside the value.
pub fn feel_tuning_domain() -> ambition_platformer2d_core::MechanicalDomain {
    ambition_platformer2d_core::MechanicalDomain::of::<FeelTuningDomain>("feel_tuning")
}

/// Raise a changed feel value as a PROPOSAL.
///
/// ⚠ **`is_added` IS EXCLUDED** for the reason every proposer excludes it: Bevy
/// counts INSERTION as a change, and the mirror is installed before content
/// finishes seeding — proposing that would stop the session the composition had
/// just started, on frame one, every time.
pub fn propose_editable_feel_tuning(
    editable: Res<EditableFeelTuning>,
    mut pending: ResMut<ambition_platformer2d_core::PendingMechanicalEdits>,
) {
    if !editable.is_changed() || editable.is_added() {
        return;
    }
    pending.propose(feel_tuning_domain());
}

/// Copy an ADMITTED feel edit into the value the simulation reads.
///
/// ⛔ Deliberately NOT change-guarded: the guard lives on the PROPOSAL, so an
/// untouched panel raises nothing and this never runs its write. Re-adding
/// `is_changed` here would drop every edit that had to be staged behind a
/// foreign rollback timeline for a frame, which is the whole point of staging it.
pub fn publish_editable_feel_tuning(
    editable: Res<EditableFeelTuning>,
    admission: Option<Res<ambition_platformer2d_core::MechanicalEditAdmission>>,
    mut pending: ResMut<ambition_platformer2d_core::PendingMechanicalEdits>,
    mut active: ResMut<Platformer2dFeelTuningMonolith>,
) {
    if !pending.is_pending(feel_tuning_domain()) {
        return;
    }
    // ⛔ ABSENT ⇒ PUBLISH, matching the resource's own default: a composition
    // with no rollback host has no history an edit could contradict.
    if matches!(
        admission.as_deref(),
        Some(ambition_platformer2d_core::MechanicalEditAdmission::Refuse)
    ) {
        return;
    }
    *active = editable.0;
    pending.take(feel_tuning_domain());
}

/// Live-tunable time/input/combat feel values consumed by sandbox gameplay.
#[derive(Resource, Reflect, Clone, Copy, Debug)]
#[reflect(Resource)]
pub struct Platformer2dFeelTuningMonolith {
    pub bullet_time_scale: f32,
    pub blink_hold_slow_scale: f32,
    pub time_ramp_down_rate: f32,
    pub time_ramp_up_rate: f32,
    pub down_double_tap_window: f32,
    pub up_double_tap_window: f32,
    pub interaction_buffer_time: f32,
    /// How long Up must be held to count as an interact — the hands-free way
    /// into a door. Matches the possession hold, which is the game's other
    /// hold-a-direction gesture.
    pub interaction_hold_time: f32,
    ///  this replaces `attack_hitstop_time` (0.055, attacker) and
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
    /// THE METEOR LOCK — how long a body spiked out of the AIR cannot
    /// recover. A floor under [`Self::knockback_recoil_lock_time`], never an
    /// addition: a meteor is a longer version of the same silence.
    ///
    ///  the value here is a BASELINE that an experience overwrites, exactly
    /// like `di_max_angle` beside it — `DeclaredCombatRules::meteor_lock_time`
    /// is the authority and the damage road folds it in before use. `0.0` is no
    /// meteor rule, which is what an exploration game wants.
    pub meteor_lock_time: f32,
    /// What a CROUCHING victim multiplies an incoming launch by — crouch
    /// cancel. Folded in from `DeclaredCombatRules::crouch_cancel_scale` by the
    /// damage road, exactly like `meteor_lock_time` beside it. `1.0` is no
    /// crouch cancel, which is what an exploration game wants.
    pub crouch_cancel_scale: f32,
    /// What this body's POST-HIT MERCY WINDOW is multiplied by — the fraction
    /// of its road's own window that survives. Folded in from
    /// `DeclaredCombatRules::hit_repeat_window_scale` by the damage road,
    /// exactly like `crouch_cancel_scale` beside it. `1.0` is the window the
    /// road authors, which is what an exploration game wants; `0.0` is a game
    /// with no blanket window, where a strike's own per-target dedup is the
    /// only repeat rule — see the declared field for why a platform fighter
    /// needs that.
    pub hit_repeat_window_scale: f32,
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
            time_ramp_down_rate: 5.0,
            time_ramp_up_rate: 14.0,
            down_double_tap_window: 0.24,
            up_double_tap_window: 0.30,
            interaction_buffer_time: 0.120,
            interaction_hold_time: 2.0,
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
            hit_repeat_window_scale: 1.0,
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

#[cfg(test)]
mod feel_tuning_domain_tests {
    use super::*;
    use bevy::prelude::App;

    /// The editor road in the order the composition runs it.
    fn app_with_the_feel_domain(
        admission: ambition_platformer2d_core::MechanicalEditAdmission,
    ) -> App {
        let mut app = App::new();
        app.init_resource::<Platformer2dFeelTuningMonolith>();
        app.insert_resource(EditableFeelTuning(Platformer2dFeelTuningMonolith::default()));
        app.init_resource::<ambition_platformer2d_core::PendingMechanicalEdits>();
        app.insert_resource(admission);
        app.add_systems(
            bevy::app::Update,
            (propose_editable_feel_tuning, publish_editable_feel_tuning).chain(),
        );
        // ⚠ ONE UPDATE FIRST, so the `is_added` frame is behind us. A proposer
        // that fired on insertion would stop the session on frame one, every
        // time, and an arm that never ran that frame could not tell.
        app.update();
        assert!(
            !app.world()
                .resource::<ambition_platformer2d_core::PendingMechanicalEdits>()
                .is_pending(feel_tuning_domain()),
            "installing the mirror proposed an edit: `is_added` is not excluded",
        );
        app
    }

    /// A double-tap WINDOW is as mechanical as a value gets — it decides whether
    /// a press becomes a dash. Editing it must travel the admission road.
    fn edit_the_window(app: &mut App) -> f32 {
        let mut editable = app.world_mut().resource_mut::<EditableFeelTuning>();
        editable.down_double_tap_window *= 2.0;
        editable.down_double_tap_window
    }

    #[test]
    fn an_admitted_feel_edit_reaches_the_value_simulation_reads() {
        let mut app = app_with_the_feel_domain(
            ambition_platformer2d_core::MechanicalEditAdmission::Publish,
        );
        let before = app
            .world()
            .resource::<Platformer2dFeelTuningMonolith>()
            .down_double_tap_window;
        let edited = edit_the_window(&mut app);
        assert_ne!(
            edited, before,
            "the edit changed nothing, so the assertion below would hold anyway",
        );

        app.update();

        assert_eq!(
            app.world()
                .resource::<Platformer2dFeelTuningMonolith>()
                .down_double_tap_window,
            edited,
            "an ADMITTED feel edit never reached the authority simulation reads",
        );
        assert!(
            !app.world()
                .resource::<ambition_platformer2d_core::PendingMechanicalEdits>()
                .is_pending(feel_tuning_domain()),
            "the domain was not drained by its own publisher",
        );
    }

    /// ⛔⛤ AND A REFUSED EDIT DOES NOT MOVE THE AUTHORITY — the whole reason this
    /// road exists. Until 2026-09-14 the inspector wrote
    /// `Platformer2dFeelTuningMonolith` itself, so there was no state in which a
    /// foreign or unhealthy rollback timeline could decline the edit at all.
    #[test]
    fn a_refused_feel_edit_stays_in_the_editor_and_the_authority_does_not_move() {
        let mut app = app_with_the_feel_domain(
            ambition_platformer2d_core::MechanicalEditAdmission::Refuse,
        );
        let before = app
            .world()
            .resource::<Platformer2dFeelTuningMonolith>()
            .down_double_tap_window;
        let edited = edit_the_window(&mut app);

        app.update();

        assert_eq!(
            app.world()
                .resource::<Platformer2dFeelTuningMonolith>()
                .down_double_tap_window,
            before,
            "a REFUSED feel edit moved the value deterministic simulation reads",
        );
        assert_eq!(
            app.world()
                .resource::<EditableFeelTuning>()
                .down_double_tap_window,
            edited,
            "the refusal discarded the developer's value instead of staging it, \
             which makes a refusal indistinguishable from a silent drop",
        );
        assert!(
            app.world()
                .resource::<ambition_platformer2d_core::PendingMechanicalEdits>()
                .is_pending(feel_tuning_domain()),
            "the refused proposal was drained, so it can never be published when \
             the refusal lifts",
        );
    }
}

