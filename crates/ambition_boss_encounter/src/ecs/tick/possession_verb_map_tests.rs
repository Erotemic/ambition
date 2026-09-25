use super::*;
use crate::behavior::BossBehaviorProfileExt;
use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::brain::{BossAttackProfile, BossCapability};
use ambition_platformer2d_core as ae;

fn rider_behavior() -> crate::pattern::profile::BossBehaviorProfile {
    crate::pattern::profile::BossBehaviorProfile::from_data(
        crate::test_boss_catalog(),
        "gnu_ton_rider",
    )
}

fn melee_frame(axis: ae::LocalAxes) -> ActorControlFrame {
    let mut f = ActorControlFrame::neutral();
    f.melee_pressed = true;
    f.attack_axis = axis;
    f
}

/// The possessed controller's aim resolves through the directional-verb chain
/// over the profile's authored `possessed_verbs`: neutral demonstrates (a slam
/// at the aim), down stomps, up swings the pendulum, special rains apples.
#[test]
fn possessed_verbs_resolve_directionally() {
    let behavior = rider_behavior();
    let cases = [
        (ae::LocalAxes::ZERO, "demonstrate"),          // neutral attack
        (ae::LocalAxes::new(1.0, 0.0), "demonstrate"), // forward attack
        (ae::LocalAxes::new(0.0, 1.0), "stomp"),       // down (+y = toward feet)
        (ae::LocalAxes::new(0.0, -1.0), "pendulum"),   // up
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
    // the base `attack` verb — the demonstration again, never a silent no-op.
    let back = possessed_attack_choice(
        &melee_frame(ae::LocalAxes::new(-1.0, 0.0)),
        &behavior,
        None,
        1.0,
    )
    .expect("back aim falls through the chain to the base attack verb");
    assert_eq!(back.move_id(), "demonstrate");

    let mut special = ActorControlFrame::neutral();
    special.special_pressed = true;
    let got = possessed_attack_choice(&special, &behavior, None, 1.0)
        .expect("the special button resolves the authored 'special' verb");
    assert_eq!(got, BossAttackProfile::Special("apple_rain".to_string()));
}

/// A boss with no possessed verbs keeps the fixed mapping: melee → the
/// primary authored strike (`slot(0)`), special → the signature content
/// special.
#[test]
fn a_boss_without_verbs_keeps_the_legacy_possession_mapping() {
    let behavior = crate::pattern::profile::BossBehaviorProfile::clockwork_warden();
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

/// The Attack and Special a possessed boss DECLARES are the moves its neutral
/// presses FIRE, for a boss with a verb map and for one without.
///
/// The scheme reads the declaration and the boss tick reads the press, so a
/// second resolution that drifted from `possessed_attack_choice` would label a
/// button with a move it does not start.
#[test]
fn the_declared_possessed_actions_are_the_moves_the_presses_fire() {
    use ambition_entity_catalog::action_scheme::{ActionGate, ControlSlot};
    let rider = rider_behavior();
    let rider_cap = BossCapability {
        specials: vec![
            (BossAttackProfile::Special("demonstrate".to_string()), 1.5),
            (BossAttackProfile::Special("apple_rain".to_string()), 3.0),
        ],
    };
    let warden = crate::pattern::profile::BossBehaviorProfile::clockwork_warden();
    let warden_cap = BossCapability {
        specials: vec![
            (BossAttackProfile::Strike("floor_slam".to_string()), 0.3),
            (
                BossAttackProfile::Special("overfit_volley".to_string()),
                2.0,
            ),
        ],
    };
    let mut special = ActorControlFrame::neutral();
    special.special_pressed = true;
    for (behavior, cap, attack, signature) in [
        (&rider, &rider_cap, "demonstrate", "apple_rain"),
        (&warden, &warden_cap, "floor_slam", "overfit_volley"),
    ] {
        let declared = possessed_boss_techniques(behavior, cap);
        let declared_on = |slot: ControlSlot| {
            declared
                .iter()
                .find(|action| action.slot == slot)
                .map(|action| match &action.gate {
                    ActionGate::Technique(id) => id.clone(),
                    other => panic!("{slot:?} must be a technique gate, got {other:?}"),
                })
        };
        let fired = |frame: &ActorControlFrame| {
            possessed_attack_choice(frame, behavior, Some(cap), 1.0).map(|p| p.move_id())
        };
        assert_eq!(declared_on(ControlSlot::Attack).as_deref(), Some(attack));
        assert_eq!(
            declared_on(ControlSlot::Attack),
            fired(&melee_frame(ae::LocalAxes::ZERO))
        );
        assert_eq!(declared_on(ControlSlot::Special).as_deref(), Some(signature));
        assert_eq!(declared_on(ControlSlot::Special), fired(&special));
    }
}
