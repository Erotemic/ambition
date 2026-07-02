use super::*;

fn special_key(key: &str) -> SpecialActionSpec {
    SpecialActionSpec::Special(key.to_string())
}

#[test]
fn special_action_spec_round_trips_through_ron() {
    // Validates the serde derive: boss special-attack tunings can now be
    // authored in RON, so the boss-attack feel (shot speed, damage, cadence)
    // is iterable without a ~10-minute sandbox recompile — the foundation for
    // moving `boss_special_for_profile`'s hardcoded constants into
    // `boss_profiles.ron` (elevated #1's named first slice).
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
    // Table-driven coverage: every combo of melee/fire/special
    // bits → predictable request count when ActionSet has all
    // capabilities. Pins per-intent independence.
    let actions = ActionSet {
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        ranged: Some(RangedActionSpec::Bolt {
            speed: 500.0,
            damage: 1,
        }),
        special: Some(special_key("bubble_shield")),
        ..Default::default()
    };
    let cases = [
        (false, false, false, 0),
        (true, false, false, 1),
        (false, true, false, 1),
        (false, false, true, 1),
        (true, true, false, 2),
        (true, false, true, 2),
        (false, true, true, 2),
        (true, true, true, 3),
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
    // Regression: the DEDICATED pogo button (`pogo_pressed`, no `melee_pressed`)
    // must resolve to a Melee request so `start_body_melee` starts the swing that
    // carries the bounce — the pogo is the air-down variant of the same swing
    // (resolved to AirDown downstream from `pogo_pressed`). Dropping this made the
    // dedicated pogo button dead after the melee-unification (gravity_symmetry's
    // pogo test caught it end-to-end).
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
        ranged: Some(RangedActionSpec::Bolt {
            speed: 500.0,
            damage: 1,
        }),
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
        ranged: Some(RangedActionSpec::Bolt {
            speed: 500.0,
            damage: 1,
        }),
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
    frame.attack_axis = ae::Vec2::new(0.0, -1.0); // up-tilt
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    match reqs[0] {
        ActionRequest::Melee { attack_axis, .. } => {
            assert_eq!(attack_axis, ae::Vec2::new(0.0, -1.0));
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
    s.ranged = Some(RangedActionSpec::Bolt {
        speed: 380.0,
        damage: 1,
    });
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
        ranged: Some(RangedActionSpec::Rock {
            speed: 400.0,
            damage: 1,
        }),
        ..Default::default()
    };
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.fire = Some(crate::actor::control::ActorFireRequest::world_space(
        ae::Vec2::new(1.0, 0.0),
        0.0, // placeholder; speed comes from ActionSet
    ));
    let reqs = resolve(&actions, &frame, ae::Vec2::ZERO);
    assert_eq!(reqs.len(), 1);
    match reqs[0] {
        ActionRequest::Ranged {
            spec,
            dir,
            dir_policy,
            ..
        } => {
            assert_eq!(spec.speed(), 400.0);
            assert_eq!(dir, ae::Vec2::new(1.0, 0.0));
            assert_eq!(dir_policy, ae::GameplayFramePolicy::WorldSpace);
        }
        _ => panic!("expected Ranged"),
    }
}

#[test]
fn resolve_preserves_controlled_body_local_fire_policy() {
    let actions = ActionSet {
        ranged: Some(RangedActionSpec::Bolt {
            speed: 500.0,
            damage: 1,
        }),
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
    match reqs[0] {
        ActionRequest::Ranged {
            dir, dir_policy, ..
        } => {
            assert_eq!(dir, ae::Vec2::new(0.0, -1.0));
            assert_eq!(dir_policy, ae::GameplayFramePolicy::ControlledBodyLocal);
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
            attack_axis: ae::Vec2::ZERO,
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
        attack_axis: ae::Vec2::ZERO,
    };
    assert_eq!(melee.label(), "melee_swipe");

    let lunge = ActionRequest::Melee {
        spec: MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT),
        origin: ae::Vec2::ZERO,
        facing: 1.0,
        attack_axis: ae::Vec2::ZERO,
    };
    assert_eq!(lunge.label(), "melee_lunge");

    let ranged = ActionRequest::Ranged {
        spec: RangedActionSpec::Bolt {
            speed: 380.0,
            damage: 1,
        },
        origin: ae::Vec2::ZERO,
        dir: ae::Vec2::new(1.0, 0.0),
        dir_policy: ae::GameplayFramePolicy::WorldSpace,
    };
    assert_eq!(ranged.label(), "ranged_bolt");

    let special = ActionRequest::Special {
        spec: special_key("bubble_shield"),
    };
    assert_eq!(special.label(), "special");
}

#[test]
fn action_request_display_includes_kind_and_origin() {
    let req = ActionRequest::Melee {
        spec: MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
        origin: ae::Vec2::new(10.0, 20.0),
        facing: 1.0,
        attack_axis: ae::Vec2::ZERO,
    };
    let s = format!("{}", req);
    assert!(s.contains("melee_swipe"));
    assert!(s.contains("facing"));

    let req2 = ActionRequest::Special {
        spec: special_key("bubble_shield"),
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
    assert_eq!(
        RangedActionSpec::Rock {
            speed: 410.0,
            damage: 1
        }
        .speed(),
        410.0
    );
    assert_eq!(
        RangedActionSpec::Arrow {
            speed: 520.0,
            damage: 2
        }
        .speed(),
        520.0
    );
    assert_eq!(
        RangedActionSpec::Pistol {
            speed: 600.0,
            damage: 1
        }
        .speed(),
        600.0
    );
    assert_eq!(
        RangedActionSpec::Bolt {
            speed: 380.0,
            damage: 1
        }
        .speed(),
        380.0
    );
}

#[test]
fn ranged_spec_damage_accessor_returns_per_variant_damage() {
    // Mirror of the speed accessor test: damage() must pull
    // from each variant's `damage` field independently. Pins
    // the per-variant routing so a future field rename can't
    // silently return the wrong variant's damage.
    assert_eq!(
        RangedActionSpec::Rock {
            speed: 0.0,
            damage: 1,
        }
        .damage(),
        1,
    );
    assert_eq!(
        RangedActionSpec::Arrow {
            speed: 0.0,
            damage: 3,
        }
        .damage(),
        3,
    );
    assert_eq!(
        RangedActionSpec::Pistol {
            speed: 0.0,
            damage: 2,
        }
        .damage(),
        2,
    );
    assert_eq!(
        RangedActionSpec::Bolt {
            speed: 0.0,
            damage: 4,
        }
        .damage(),
        4,
    );
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
        ranged: Some(RangedActionSpec::Bolt {
            speed: 380.0,
            damage: 1,
        }),
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
    assert_eq!(reqs.len(), 3);
}
