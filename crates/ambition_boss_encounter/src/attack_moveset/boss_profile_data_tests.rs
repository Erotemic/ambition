use super::*;
use crate::behavior::BossBehaviorProfileExt;
use ambition_characters::brain::boss_pattern::BossAttackPattern;

/// `assets/data/boss_profiles.ron` must carry a row for every
/// boss the codebase has a constructor for. Without this, the
/// `from_data` lookup would panic at the first spawn of a
/// missing boss.
#[test]
fn ron_carries_every_known_boss() {
    for id in [
        "clockwork_warden",
        "mockingbird",
        "gnu_ton_rider",
        "smirking_behemoth_boss",
    ] {
        // `from_data` panics with a clear message when the row is
        // missing (the registry static is private to behavior.rs).
        let _ = BossBehaviorProfile::from_data(crate::test_boss_catalog(), id);
    }
}

/// Spot-check the legacy pre-data values for a divergent
/// archetype: the Clockwork Warden's macro tuning and attack
/// damage. Catches accidental tuning drift on the row the
/// player notices first.
#[test]
fn legacy_baseline_pins() {
    let warden = BossBehaviorProfile::clockwork_warden();
    assert_eq!(warden.id, "clockwork_warden");
    assert_eq!(warden.attack_damage, 2);
    assert_eq!(warden.body_damage, 1);
    assert!((warden.strike_speed_scale - 0.20).abs() < f32::EPSILON);
    assert!((warden.macro_tuning.too_close_distance - 110.0).abs() < f32::EPSILON);
    assert!((warden.macro_tuning.engage_max_duration_s - 9.0).abs() < f32::EPSILON);
    let gnu = BossBehaviorProfile::gnu_ton_rider();
    assert_eq!(gnu.body_damage, 0);
    assert_eq!(gnu.attacks.len(), 5);
    let mocker = BossBehaviorProfile::mockingbird();
    assert!(matches!(mocker.attack_pattern, BossAttackPattern::Cycle));
}

/// The authored profile is a CONTACT chase: `engage_distance = 0`, no standoff ring,
/// `suppress_attacks_while_moving`. Its Approach was ended by a centre-to-centre test against a 4px
/// epsilon, which a 208px-wide body cannot satisfy, so it approached forever and stayed silent
/// forever.
#[test]
fn a_contact_boss_standing_against_its_target_fires_its_authored_attack() {
    use ambition_characters::brain::{
        BossAttackIntent, BossAttackProfile, BossMacroState, BossPatternCfg, BossPatternContext,
        BossPatternState,
    };
    use ambition_platformer2d_core as ae;

    let behavior =
        BossBehaviorProfile::from_data(crate::test_boss_catalog(), "smirking_behemoth_boss");
    let combat_size = behavior
        .combat_size
        .expect("the Smirking Behemoth authors its own body box");
    assert!(
        behavior.macro_tuning.contact_chase_mode(),
        "this test is about a CONTACT chase; the profile stopped authoring one",
    );

    let mut cfg = BossPatternCfg::neutral_test();
    cfg.aggressiveness = 1.0;
    cfg.pattern = behavior.attack_pattern.clone();
    cfg.movement = behavior.movement.clone();
    cfg.combat_size = combat_size;
    cfg.macro_tuning = behavior.macro_tuning;
    // The encounter id seeds the boss's one deterministic random stream, and
    // this profile's idle beat is probabilistic — so it has to be the real one
    // or the draw under test is not the draw that ships.
    cfg.encounter_id = behavior.id.clone();
    cfg.spawn = ae::Vec2::new(640.0, 400.0);

    let actor_pos = cfg.spawn;
    // An ordinary body pressed against the boss's left flank — where a player
    // who has run at this boss ends up, because contact is what stops them.
    let target_body_size = ae::Vec2::new(32.0, 64.0);
    let target_pos = ae::Vec2::new(
        actor_pos.x - (combat_size.x + target_body_size.x) * 0.5,
        actor_pos.y,
    );

    let mut state = BossPatternState::default();
    let mut intent = BossAttackIntent::default();
    let mut out = ambition_characters::actor::control::ActorControlFrame::neutral();
    let mut fired_after_s: Option<f32> = None;
    let dt = 1.0 / 60.0;
    // Ten seconds. What it is NOT is the pre-fix behaviour: a boss whose contact chase never
    // closes gets one attacking tick per `approach_duration_s` (8s), which stretched this
    // three-beat script to minutes of wall clock.
    for tick in 0..600 {
        let ctx = BossPatternContext {
            encounter_phase: crate::BossEncounterPhase::Phase1,
            actor_pos,
            target_pos,
            target_body_size,
            world_size: ae::Vec2::new(1_280.0, 768.0),
            front_wall_clearance: None,
            dt,
            actor_facing: -1.0,
            hp_current: 100,
            hp_max: 100,
            live_attack: None,
        };
        crate::pattern::tick_boss_pattern(&cfg, &mut state, &ctx, &mut out, &mut intent);
        let fired = intent
            .active_profile
            .as_ref()
            .or(intent.telegraph_profile.as_ref());
        if let (None, Some(profile)) = (fired_after_s, fired) {
            assert_eq!(
                *profile,
                BossAttackProfile::Special("eye_beam".to_string()),
                "the only attack this boss authors is the eye beam",
            );
            fired_after_s = Some(tick as f32 * dt);
        }
    }

    assert_eq!(
        state.macro_state,
        BossMacroState::Engage,
        "a target touching the boss has closed its contact chase",
    );
    let fired_after_s = fired_after_s.expect(
        "a contact boss standing against its target must reach its authored attack within 10s",
    );
    assert!(
        fired_after_s < 10.0,
        "the eye beam arrived at {fired_after_s}s",
    );
}
