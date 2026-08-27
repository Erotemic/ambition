use super::*;
use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::brain::{BossAttackProfile, BossCapability};
use ambition_platformer2d_core as ae;
use ambition_boss_encounter::behavior::BossBehaviorProfileExt;

fn rider_behavior() -> ambition_boss_encounter::behavior::BossBehaviorProfile {
    ambition_boss_encounter::behavior::BossBehaviorProfile::from_data(
        ambition_boss_encounter::test_boss_catalog(),
        "gnu_ton_rider",
    )
}

fn melee_frame(axis: ae::LocalAxes) -> ActorControlFrame {
    let mut f = ActorControlFrame::neutral();
    f.melee_pressed = true;
    f.attack_axis = axis;
    f
}

/// G5: the possessed controller's aim resolves through the directional-verb
/// chain over the profile's authored `possessed_verbs` — neutral sweeps,
/// down slams, up raises the shockwave, special rains apples. The resolved
/// ids are exactly the `limb_routing` keys, so aboard the giant these ARE
/// the limb verbs.
#[test]
fn possessed_verbs_resolve_directionally() {
    let behavior = rider_behavior();
    let cases = [
        (ae::LocalAxes::ZERO, "hand_sweep"),          // neutral attack
        (ae::LocalAxes::new(1.0, 0.0), "hand_sweep"), // forward attack
        (ae::LocalAxes::new(0.0, 1.0), "hand_slam"),  // down (+y = toward feet)
        (ae::LocalAxes::new(0.0, -1.0), "converging_shockwave"), // up
    ];
    for (axis, expected) in cases {
        let got = possessed_attack_choice(&melee_frame(axis), &behavior, None, 1.0)
            .unwrap_or_else(|| panic!("aim {axis:?} resolves a move"));
        assert_eq!(
            got.move_id(),
            expected,
            "aim {axis:?} should command '{expected}'",
        );
    }
    // Back-aim: no authored `attack_back`, so the chain falls through to
    // the base `attack` verb — the sweep again, never a silent no-op.
    let back = possessed_attack_choice(
        &melee_frame(ae::LocalAxes::new(-1.0, 0.0)),
        &behavior,
        None,
        1.0,
    )
    .expect("back aim falls through the chain to the base attack verb");
    assert_eq!(back.move_id(), "hand_sweep");

    let mut special = ActorControlFrame::neutral();
    special.special_pressed = true;
    let got = possessed_attack_choice(&special, &behavior, None, 1.0)
        .expect("the special button resolves the authored 'special' verb");
    assert_eq!(got, BossAttackProfile::Special("apple_rain".to_string()));
}

/// A boss that authors NO possessed verbs keeps the legacy deterministic
/// mapping byte-for-byte: melee → the primary authored strike (`slot(0)`),
/// special → the signature content special. No behavior change for every
/// existing possessable boss.
#[test]
fn a_boss_without_verbs_keeps_the_legacy_possession_mapping() {
    let behavior = ambition_boss_encounter::behavior::BossBehaviorProfile::clockwork_warden();
    assert!(behavior.possessed_verbs.is_empty());
    let cap = BossCapability {
        specials: vec![
            (BossAttackProfile::Strike("floor_slam".to_string()), 0.3),
            (
                BossAttackProfile::Special("overfit_volley".to_string()),
                2.0,
            ),
        ],
    };

    // Melee (any aim — no verbs means direction cannot rebind it).
    let got = possessed_attack_choice(
        &melee_frame(ae::LocalAxes::new(0.0, 1.0)),
        &behavior,
        Some(&cap),
        1.0,
    )
    .expect("legacy fallback: primary strike");
    assert_eq!(got, BossAttackProfile::Strike("floor_slam".to_string()));

    let mut special = ActorControlFrame::neutral();
    special.special_pressed = true;
    let got = possessed_attack_choice(&special, &behavior, Some(&cap), 1.0)
        .expect("legacy fallback: signature special");
    assert_eq!(
        got,
        BossAttackProfile::Special("overfit_volley".to_string())
    );

    // No input → no intent.
    assert!(
        possessed_attack_choice(&ActorControlFrame::neutral(), &behavior, Some(&cap), 1.0)
            .is_none()
    );
}
