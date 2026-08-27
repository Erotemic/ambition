use super::*;

fn special_key(key: &str) -> SpecialActionSpec {
    SpecialActionSpec::Special(key.to_string())
}

#[test]
fn special_action_spec_round_trips_through_ron() {
    // Validates the serde derive for content-technique action identifiers. The
    // boss moveset now carries those identifiers as ordinary move effects, while
    // other actors may still author the same generic SpecialActionSpec in RON.
    let spec = SpecialActionSpec::Special("eye_beam".to_string());
    let serialized = ron::to_string(&spec).expect("SpecialActionSpec should serialize to RON");
    let restored: SpecialActionSpec =
        ron::from_str(&serialized).expect("SpecialActionSpec should deserialize from RON");
    assert_eq!(spec, restored);
}

#[test]
fn use_behavior_decides_throw_on_plain_attack() {
    // Auto derives from the verbs: a verb-bearing weapon keeps; a verb-less
    // item throws (the legacy is_pure_throwable rule).
    let axe = HeldItemSpec {
        id: "axe".into(),
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
            windup_s: 0.2,
            active_s: 0.1,
            recover_s: 0.3,
            damage: 3,
            reach_px: 60.0,
        })),
        ranged: None,
        use_behavior: HeldUseBehavior::Auto,
    };
    assert!(
        !axe.throws_on_plain_attack(),
        "a verb-bearing Auto item keeps on use"
    );

    let bare = HeldItemSpec {
        id: "rock".into(),
        melee: None,
        ranged: None,
        use_behavior: HeldUseBehavior::Auto,
    };
    assert!(
        bare.throws_on_plain_attack(),
        "a verb-less Auto item throws on use"
    );

    // Explicit behaviors override the Auto derivation.
    let use_system = HeldItemSpec {
        use_behavior: HeldUseBehavior::UseSystem,
        ..bare.clone()
    };
    assert!(
        !use_system.throws_on_plain_attack(),
        "a UseSystem ability is not thrown by a plain Attack"
    );
    let throw = HeldItemSpec {
        use_behavior: HeldUseBehavior::ThrowOnUse,
        ..axe.clone()
    };
    assert!(
        throw.throws_on_plain_attack(),
        "ThrowOnUse throws even a verb-bearing item"
    );

    // The wired abilities are UseSystem (so a plain Attack drives them, not a throw).
    for id in [
        "blink",
        "grapple",
        "mark_recall",
        "shockwave",
        "volley",
        "puppy_slug_gun",
    ] {
        assert!(
            !held_item_by_id(id).unwrap().throws_on_plain_attack(),
            "{id} should be use-on-attack, not throw-on-attack"
        );
    }
    // The throwables / weapons are not use-system → throw vs keep per Auto.
    assert!(held_item_by_id("bomb").unwrap().throws_on_plain_attack());
    assert!(!held_item_by_id("gun_sword")
        .unwrap()
        .throws_on_plain_attack());
}

#[test]
fn peaceful_action_set_has_no_attacks() {
    let s = ActionSet::peaceful();
    assert!(s.melee.is_none());
    assert!(s.ranged.is_none());
    assert!(s.special.is_none());
    assert_eq!(s.move_style, MoveStyleSpec::Walk);
    assert!(!s.can_attack());
}

#[test]
fn resolve_returns_predictable_request_count_per_intent_subset() {
    // Table-driven coverage: every combo of melee/fire/special bits → predictable request
    // count. Pins that the flat special arm is gone.
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ranged: Some(RangedActionSpec::bolt(500.0, 1)),
        special: Some(special_key("bubble_shield")),
        ..Default::default()
    };
    let cases = [
        (false, false, false, 0),
        (true, false, false, 1),
        (false, true, false, 1),
        (false, false, true, 0), // special is moveset-resolved, not here
        (true, true, false, 2),
        (true, false, true, 1), // only the melee emits
        (false, true, true, 1), // only the fire emits
        (true, true, true, 2),  // melee + fire; special is the moveset's
    ];
    for (melee, fire, special, expected) in cases {
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        frame.melee_pressed = melee;
        frame.fire = if fire {
            Some(crate::actor::control::ActorFireRequest::world_space(
                ae::Vec2::new(1.0, 0.0),
                0.0,
            ))
        } else {
            None
        };
        frame.special_pressed = special;
        let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
        assert_eq!(
            reqs.len(),
            expected,
            "melee={} fire={} special={}",
            melee,
            fire,
            special,
        );
    }
}

#[test]
fn resolve_emits_a_melee_request_for_a_dedicated_pogo_press() {
    // Dropping this made the dedicated pogo button dead after the melee-unification
    // (gravity_symmetry's pogo test caught it end-to-end).
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.pogo_pressed = true;
    assert!(
        frame.wants_any_action(),
        "a pogo-only frame genuinely wants an action"
    );
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    assert_eq!(
        reqs.len(),
        1,
        "the dedicated pogo press emits one Melee request"
    );
    assert!(
        matches!(reqs[0], ActionRequest::Melee { .. }),
        "the pogo press resolves to a Melee swing (its AirDown intent is set downstream)"
    );

    // A body with NO melee capability emits nothing on a pogo press (can't pogo).
    let no_melee = ActionSet {
        melee: None,
        ..Default::default()
    };
    assert!(resolve(&no_melee, &frame, ae::Vec2::ZERO).is_empty());
}

#[test]
fn resolve_empty_when_frame_has_no_action_intent() {
    // wants_any_action()=false → resolver always returns empty.
    // Pin the contract so sandbox code that gates resolve()
    // calls behind wants_any_action() can rely on it.
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ranged: Some(RangedActionSpec::bolt(500.0, 1)),
        special: Some(special_key("bubble_shield")),
        move_style: MoveStyleSpec::Walk,
    };
    let frame = crate::actor::control::ActorControlFrame::neutral();
    assert!(!frame.wants_any_action());
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    assert!(reqs.is_empty());
}

#[test]
fn resolve_with_only_ranged_capability_ignores_melee_intent() {
    // ActionSet with ranged-only capability + frame intent
    // melee_pressed+fire returns Ranged only. Pins the
    // capability gate so a brain that emits melee intent on
    // a ranged-only actor doesn't accidentally spawn a hitbox.
    let actions = ActionSet {
        ranged: Some(RangedActionSpec::bolt(500.0, 1)),
        ..Default::default()
    };
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.fire = Some(crate::actor::control::ActorFireRequest::world_space(
        ae::Vec2::new(1.0, 0.0),
        0.0,
    ));
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    assert_eq!(reqs.len(), 1);
    assert!(matches!(reqs[0], ActionRequest::Ranged { .. }));
}

#[test]
fn resolve_passes_attack_axis_through_to_melee_request() {
    // Player tilt (up-tilt / down-air / back-air) carries
    // direction in frame.attack_axis; resolver threads it
    // through so the EFFECTS-stage spawn picks the right
    // hitbox shape.
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.facing = 1.0;
    frame.attack_axis = ae::LocalAxes::new(0.0, -1.0); // up-tilt
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    match reqs[0] {
        ActionRequest::Melee { attack_axis, .. } => {
            assert_eq!(attack_axis, ae::LocalAxes::new(0.0, -1.0));
        }
        _ => panic!("expected Melee"),
    }
}

#[test]
fn resolve_peaceful_action_set_emits_nothing_for_full_intent() {
    // ActionSet::peaceful() has no melee/ranged/special. Even
    // if the brain emits every intent verb, the resolver
    // returns an empty vec — peaceful actors stay peaceful
    // even under arbitrary brain input. Pins the "ActionSet
    // is the authority on capability" invariant.
    let actions = ActionSet::peaceful();
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.fire = Some(crate::actor::control::ActorFireRequest::world_space(
        ae::Vec2::new(1.0, 0.0),
        0.0,
    ));
    frame.special_pressed = true;
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    assert!(
        reqs.is_empty(),
        "ActionSet::peaceful produces no requests regardless of intent"
    );
}

#[test]
fn action_set_default_is_peaceful_baseline() {
    // Default-constructed ActionSet is the peaceful baseline:
    // no attack capability, default move style. Pins the
    // contract that a fresh-spawn actor with default ActionSet
    // can't attack — sandbox code that constructs ActionSets
    // via `..Default::default()` can rely on this.
    let s = ActionSet::default();
    assert!(s.melee.is_none());
    assert!(s.ranged.is_none());
    assert!(s.special.is_none());
    assert!(!s.can_attack());
    assert_eq!(s.move_style, MoveStyleSpec::default());
    // ActionSet::default() == ActionSet::peaceful().
    let p = ActionSet::peaceful();
    assert!(p.melee.is_none() && s.melee.is_none());
}

#[test]
fn action_set_can_attack_detects_melee_or_ranged() {
    let mut s = ActionSet::peaceful();
    assert!(!s.can_attack());
    s.melee = Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT));
    assert!(s.can_attack());
    s.melee = None;
    s.ranged = Some(RangedActionSpec::bolt(380.0, 1));
    assert!(s.can_attack());
    // Special alone doesn't count as "attacks".
    s.ranged = None;
    s.special = Some(special_key("bubble_shield"));
    assert!(!s.can_attack());
}

#[test]
fn resolve_no_intent_yields_no_requests() {
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    let frame = crate::actor::control::ActorControlFrame::neutral();
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    assert!(reqs.is_empty());
}

#[test]
fn resolve_melee_pressed_emits_one_melee_request() {
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.facing = 1.0;
    let reqs = resolve(&actions, &frame, ae::Vec2::new(10.0, 5.0));
    assert_eq!(reqs.len(), 1);
    match reqs[0] {
        ActionRequest::Melee {
            spec,
            origin,
            facing,
            ..
        } => {
            assert!(matches!(spec, MeleeActionSpec::Swipe(_)));
            assert_eq!(origin, ae::Vec2::new(10.0, 5.0));
            assert_eq!(facing, 1.0);
        }
        _ => panic!("expected Melee request"),
    }
}

#[test]
fn resolve_melee_pressed_without_capability_emits_nothing() {
    // Puppy slug: brain emits melee_pressed = false today, but
    // even if a possessor presses melee while inhabiting one,
    // it has no melee capability and nothing fires.
    let actions = ActionSet::peaceful();
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    assert!(reqs.is_empty());
}

#[test]
fn resolve_two_actionsets_differ_by_capability() {
    // Same brain intent, different ActionSets → different
    // requests. This is the core "possession is cheap"
    // invariant: swap brains, keep the body's ActionSet.
    let goblin = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ..Default::default()
    };
    let brute = ActionSet {
        melee: Some(MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT)),
        ..Default::default()
    };
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.facing = 1.0;
    let g = resolve(&goblin, &frame, ae::Vec2::ZERO);
    let b = resolve(&brute, &frame, ae::Vec2::ZERO);
    assert_eq!(g.len(), 1);
    assert_eq!(b.len(), 1);
    match (&g[0], &b[0]) {
        (ActionRequest::Melee { spec: gs, .. }, ActionRequest::Melee { spec: bs, .. }) => {
            assert!(matches!(gs, MeleeActionSpec::Swipe(_)));
            assert!(matches!(bs, MeleeActionSpec::Lunge(_)));
        }
        _ => panic!("expected two Melee requests"),
    }
}

#[test]
fn resolve_fire_pressed_emits_ranged_request() {
    let actions = ActionSet {
        ranged: Some(RangedActionSpec::rock(400.0, 1)),
        ..Default::default()
    };
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.fire = Some(crate::actor::control::ActorFireRequest::world_space(
        ae::Vec2::new(1.0, 0.0),
        0.0, // placeholder; speed comes from ActionSet
    ));
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    assert_eq!(reqs.len(), 1);
    match &reqs[0] {
        ActionRequest::Ranged {
            spec,
            dir,
            dir_policy,
            ..
        } => {
            assert_eq!(spec.speed(), 400.0);
            assert_eq!(*dir, ae::Vec2::new(1.0, 0.0));
            assert_eq!(*dir_policy, ae::GameplayFramePolicy::WorldSpace);
        }
        _ => panic!("expected Ranged"),
    }
}

#[test]
fn resolve_preserves_controlled_body_local_fire_policy() {
    let actions = ActionSet {
        ranged: Some(RangedActionSpec::bolt(500.0, 1)),
        ..Default::default()
    };
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.fire = Some(
        crate::actor::control::ActorFireRequest::controlled_body_local(
            ae::Vec2::new(0.0, -1.0),
            0.0,
        ),
    );
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    match &reqs[0] {
        ActionRequest::Ranged {
            dir, dir_policy, ..
        } => {
            assert_eq!(*dir, ae::Vec2::new(0.0, -1.0));
            assert_eq!(*dir_policy, ae::GameplayFramePolicy::ControlledBodyLocal);
        }
        _ => panic!("expected Ranged"),
    }
}

#[test]
fn melee_spec_defaults_have_positive_durations() {
    // Every authored default's phase timings (windup + active +
    // recover) must be strictly positive — a zero windup means
    // the attack has no telegraph for the player to read, and a
    // zero active means no hitbox window. Pins the design
    // requirement that telegraphs live inside the attack
    // animation rather than in a separate spec.
    let s = SwipeSpec::STRIKER_DEFAULT;
    assert!(s.windup_s > 0.0 && s.active_s > 0.0 && s.recover_s > 0.0);
    let l = LungeSpec::BRUTE_DEFAULT;
    assert!(l.windup_s > 0.0 && l.active_s > 0.0 && l.recover_s > 0.0);
    let p = PunchSpec::SANDBAG_DEFAULT;
    assert!(p.windup_s > 0.0 && p.active_s > 0.0 && p.recover_s > 0.0);
}

#[test]
fn melee_attack_uniform_helpers_match_concrete_field_lookup() {
    // total_duration_s / damage / reach_px on MeleeActionSpec
    // should equal the same field on the inner spec struct
    // for every variant. Pins the helper consistency so a
    // future spec-struct field rename doesn't cause the
    // accessors to silently return stale values.
    for spec in [
        MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
        MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT),
        MeleeActionSpec::PunchWeak(PunchSpec::SANDBAG_DEFAULT),
    ] {
        assert!(spec.total_duration_s() > 0.0);
        assert!(spec.damage() > 0);
        assert!(spec.reach_px() > 0.0);
    }
}

#[test]
fn action_request_label_covers_all_melee_variants() {
    // Every MeleeActionSpec variant maps to a distinct
    // "melee_*" label. Future Spec variants must update
    // ActionRequest::label() too — this test catches a drop.
    let specs = [
        MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
        MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT),
        MeleeActionSpec::Slam(SlamSpec {
            windup_s: 0.3,
            active_s: 0.1,
            recover_s: 0.4,
            damage: 2,
            reach_px: 40.0,
            hop_height_px: 60.0,
        }),
        MeleeActionSpec::Bite(BiteSpec {
            windup_s: 0.18,
            active_s: 0.08,
            recover_s: 0.25,
            damage: 1,
            reach_px: 22.0,
        }),
        MeleeActionSpec::PunchWeak(PunchSpec::SANDBAG_DEFAULT),
    ];
    let mut labels = Vec::new();
    for spec in specs {
        let req = ActionRequest::Melee {
            spec,
            origin: ae::Vec2::ZERO,
            facing: 1.0,
            attack_axis: ae::LocalAxes::ZERO,
        };
        let label = req.label();
        assert!(label.starts_with("melee_"), "{}", label);
        labels.push(label);
    }
    // Ensure all labels are distinct (no two variants share
    // a label — would break grep-friendly diagnostics).
    let mut sorted = labels.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        labels.len(),
        "every melee variant should have a distinct label"
    );
}

#[test]
fn action_request_label_returns_per_variant_string() {
    let melee = ActionRequest::Melee {
        spec: MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
        origin: ae::Vec2::ZERO,
        facing: 1.0,
        attack_axis: ae::LocalAxes::ZERO,
    };
    assert_eq!(melee.label(), "melee_swipe");

    let lunge = ActionRequest::Melee {
        spec: MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT),
        origin: ae::Vec2::ZERO,
        facing: 1.0,
        attack_axis: ae::LocalAxes::ZERO,
    };
    assert_eq!(lunge.label(), "melee_lunge");

    let ranged = ActionRequest::Ranged {
        spec: RangedActionSpec::bolt(380.0, 1),
        origin: ae::Vec2::ZERO,
        dir: ae::Vec2::new(1.0, 0.0),
        dir_policy: ae::GameplayFramePolicy::WorldSpace,
        commitment: RangedCommitment::Attempt,
    };
    assert_eq!(ranged.label(), "ranged_bolt");

    let special = ActionRequest::Special {
        spec: special_key("bubble_shield"),
        params: Default::default(),
    };
    assert_eq!(special.label(), "special");
}

#[test]
fn action_request_display_includes_kind_and_origin() {
    let req = ActionRequest::Melee {
        spec: MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
        origin: ae::Vec2::new(10.0, 20.0),
        facing: 1.0,
        attack_axis: ae::LocalAxes::ZERO,
    };
    let s = format!("{}", req);
    assert!(s.contains("melee_swipe"));
    assert!(s.contains("facing"));

    let req2 = ActionRequest::Special {
        spec: special_key("bubble_shield"),
        params: Default::default(),
    };
    assert_eq!(format!("{}", req2), "special");
}

#[test]
fn melee_spec_uniform_accessors_return_per_variant_fields() {
    let s = MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT);
    assert_eq!(s.damage(), SwipeSpec::STRIKER_DEFAULT.damage);
    assert_eq!(s.reach_px(), SwipeSpec::STRIKER_DEFAULT.reach_px);
    assert!(s.total_duration_s() > 0.0);

    let l = MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT);
    assert_eq!(l.damage(), LungeSpec::BRUTE_DEFAULT.damage);
    assert_eq!(l.reach_px(), LungeSpec::BRUTE_DEFAULT.reach_px);

    let p = MeleeActionSpec::PunchWeak(PunchSpec::SANDBAG_DEFAULT);
    assert_eq!(p.damage(), PunchSpec::SANDBAG_DEFAULT.damage);
    assert!(p.total_duration_s() > 0.0);
}

#[test]
fn ranged_spec_speed_accessor_returns_per_variant_speed() {
    assert_eq!(RangedActionSpec::rock(410.0, 1).speed(), 410.0);
    assert_eq!(RangedActionSpec::arrow(520.0, 2).speed(), 520.0);
    assert_eq!(RangedActionSpec::pistol(600.0, 1).speed(), 600.0);
    assert_eq!(RangedActionSpec::bolt(380.0, 1).speed(), 380.0);
}

#[test]
fn ranged_spec_damage_accessor_returns_per_variant_damage() {
    // Mirror of the speed accessor test: damage() must pull
    // from each variant's `damage` field independently. Pins
    // the per-variant routing so a future field rename can't
    // silently return the wrong variant's damage.
    assert_eq!(RangedActionSpec::rock(0.0, 1).damage(), 1,);
    assert_eq!(RangedActionSpec::arrow(0.0, 3).damage(), 3,);
    assert_eq!(RangedActionSpec::pistol(0.0, 2).damage(), 2,);
    assert_eq!(RangedActionSpec::bolt(0.0, 4).damage(), 4,);
}

#[test]
fn resolve_multi_intent_emits_multi_request() {
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Bite(BiteSpec {
            windup_s: 0.2,
            active_s: 0.1,
            recover_s: 0.3,
            damage: 1,
            reach_px: 22.0,
        })),
        ranged: Some(RangedActionSpec::bolt(380.0, 1)),
        special: Some(special_key("boss_spotlight")),
        move_style: MoveStyleSpec::Float,
    };
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.fire = Some(crate::actor::control::ActorFireRequest::world_space(
        ae::Vec2::new(0.0, -1.0),
        0.0,
    ));
    frame.special_pressed = true;
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    // Melee + fire emit; `special_pressed` does NOT — the moveset subsumes the flat
    // special arm (§A1), so a pressed special resolves to a MovePlayback elsewhere,
    // not an `ActionRequest::Special` here.
    assert_eq!(reqs.len(), 2);
    assert!(
        !reqs
            .iter()
            .any(|r| matches!(r, ActionRequest::Special { .. })),
        "the flat special arm is retired — no Special request from resolve"
    );
}

// ── A CHARGED SHOT IS A DIFFERENT SHOT ───────────────────────────────────────

fn charging_cannon() -> RangedActionSpec {
    RangedActionSpec::bolt(500.0, 4)
        .with_flight(ProjectileFlight::STRAIGHT)
        .with_charge(RangedCharge {
            damage_mult: 3.0,
            speed_mult: 1.5,
            size_mult: 2.0,
            visuals: vec!["t1".into(), "t2".into(), "t3".into()],
        })
}

/// A tap is the shot the fighter always had; a full hold is the whole ladder.
#[test]
fn a_charge_scales_the_shot_it_releases() {
    let base = charging_cannon();
    let tap = base.at_charge(0.0);
    assert_eq!(tap.damage, base.damage, "a tap paid a charge it never held");
    assert_eq!(tap.speed, base.speed);
    assert_eq!(
        tap.flight.map(|f| f.half_extent),
        base.flight.map(|f| f.half_extent)
    );

    let full = base.at_charge(1.0);
    assert_eq!(full.damage, 12, "4 damage at a 3x hold");
    assert!((full.speed - 750.0).abs() < 1e-3, "{}", full.speed);
    let grown = full.flight.expect("the fixture authors flight").half_extent;
    let plain = base.flight.expect("the fixture authors flight").half_extent;
    assert!(
        (grown.x - plain.x * 2.0).abs() < 1e-3 && (grown.y - plain.y * 2.0).abs() < 1e-3,
        "the hit volume did not grow with the ball: {grown:?} vs {plain:?}"
    );

    // Halfway is halfway, not a step: damage is continuous even though the
    // LOOK is not.
    assert_eq!(base.at_charge(0.5).damage, 8);
}

/// ⛔ A SPEC WITH NO LADDER IS UNTOUCHED AT EVERY FRACTION. Every ranged action
/// that existed before charging did takes this path, and a change here is a
/// silent buff to the whole cast.
#[test]
fn a_shot_that_does_not_charge_is_the_same_shot_at_every_fraction() {
    let plain = RangedActionSpec::bolt(500.0, 4).with_flight(ProjectileFlight::STRAIGHT);
    assert!(
        plain.charge.is_none(),
        "the fixture opted in, so this proves nothing"
    );
    for fraction in [0.0, 0.25, 0.5, 0.75, 1.0] {
        assert_eq!(
            plain.at_charge(fraction),
            plain,
            "an uncharged shot changed at fraction {fraction}"
        );
    }
}

/// The LOOK steps, and the top of the ladder is reachable.
///
/// ⛔ THE POISON IS `1.0`. Indexing by `fraction * len` without the clamp runs
/// one past the end at a full charge, which is the tier a player only ever sees
/// after doing the thing the mechanic asks of them.
#[test]
fn the_visual_tier_steps_and_a_full_charge_reaches_the_last_rung() {
    let cannon = charging_cannon();
    let tier = |f: f32| cannon.at_charge(f).visual.expect("the ladder names a look");
    assert_eq!(tier(0.0), "t1");
    assert_eq!(tier(0.32), "t1");
    assert_eq!(tier(0.34), "t2");
    assert_eq!(tier(0.67), "t3");
    assert_eq!(
        tier(1.0),
        "t3",
        "a full charge fell off the end of the ladder"
    );

    // An empty ladder keeps whatever look the shot already had.
    let mut no_looks = charging_cannon();
    no_looks.visual = Some("plain".into());
    no_looks
        .charge
        .as_mut()
        .expect("authored above")
        .visuals
        .clear();
    assert_eq!(no_looks.at_charge(1.0).visual.as_deref(), Some("plain"));
}

/// ⭐⭐ THE PRODUCTION CONSTRUCTOR'S OWN NUMBERS, because the dynamics test
/// cannot reach them.
///
/// ⛔⛔ `a_boomerang_turns_around_and_returns_to_where_it_was_thrown` lives in
/// `ambition_projectiles`, which does not depend on this crate, so it hand-writes
/// `boomerang_return_s = OUT_S` and `max_lifetime = OUT_S * 2.0` and drives the
/// integrator with those. That proves the PHYSICS and pins nothing about what
/// content actually gets: restoring the old `+ 0.15` here would leave every
/// dynamics assertion green while the shipped ponytail expired 79px behind the
/// hand again (GPT 5.6, 2026-08-27).
///
/// ⭐ SO THIS ASSERTS THE RULE, NOT A NUMBER. The return acceleration is
/// `-v0 / out_s`, which puts the shot back at the throw point at exactly
/// `2 · out_s` — there is no tuning term that could be right. Across the range,
/// because a single sample cannot tell `2·out_s` from `out_s + 0.34`.
#[test]
fn the_boomerang_constructor_authors_exactly_the_round_trip() {
    for out_s in [0.25_f32, 0.34, 0.5, 1.0] {
        let flight = ProjectileFlight::boomerang(out_s);
        assert_eq!(
            flight.boomerang_return_s,
            Some(out_s),
            "the turnaround is not where the caller asked for it"
        );
        assert_eq!(
            flight.max_lifetime,
            out_s * 2.0,
            "`boomerang({out_s})` lives {} seconds where the round trip takes \
             {} — a tail that outlives its return flies on PAST the hand that \
             threw it, backwards and fast, which is what the old `+ 0.15` did",
            flight.max_lifetime,
            out_s * 2.0,
        );
    }
}
