use super::*;

#[derive(serde::Deserialize)]
struct GliderParams {
    #[allow(dead_code)]
    rise: f32,
}

#[test]
fn param_schema_registry_catches_typos_at_validate_time() {
    // A technique registers a hydrate check; the content pass runs every
    // authored EffectRef through it. A good ref passes; a missing or mistyped
    // field fails at validate time, not mid-fight.
    let mut reg = ParamSchemaRegistry::default();
    assert!(reg.is_empty());
    reg.register("glider", check_hydrates::<GliderParams>);

    let good = EffectRef {
        key: "glider".into(),
        params: ParamValue::parse("(rise: 320.0)").unwrap(),
    };
    assert!(reg.validate(&good).is_ok());

    // Wrong type for `rise` — fails, naming the offending key.
    let bad = EffectRef {
        key: "glider".into(),
        params: ParamValue::parse("(rise: \"fast\")").unwrap(),
    };
    let err = reg.validate(&bad).expect_err("bad params must fail");
    assert!(err.contains("glider"), "error names the effect key: {err}");

    // An unregistered key always passes — the engine matches no key.
    let unknown = EffectRef::new("some_content_const_technique");
    assert!(reg.validate(&unknown).is_ok());

    // Batch validation collects every failure at once.
    let errs = reg.validate_all([&good, &bad, &unknown]);
    assert_eq!(errs.len(), 1, "only the mistyped ref fails: {errs:?}");
}

/// The seed catalog: one actor-like entity (a moveset + body +
/// presentation) and one prop-like entity (body + presentation only).
/// The actor's `swat` is the SwipeSpec shape as data: three windows,
/// the active one carrying one rect hit volume.
const SEED: &str = r#"
(
    schema_version: 1,
    entities: [
        (
            id: "sandbag_seed",
            contracts: (
                body: Some((half_extents: (15.0, 24.0))),
                presentation: Some((visual_id: "sandbag")),
                moveset: Some((
                    verbs: { "attack": "swat" },
                    moves: [
                        (
                            id: "swat",
                            clip: (clip: "slash", fallbacks: ["idle"]),
                            duration_s: 0.68,
                            windows: [
                                (start_s: 0.0, end_s: 0.28, tag: Startup, volumes: []),
                                (start_s: 0.28, end_s: 0.36, tag: Active, volumes: [
                                    (shape: Rect(offset: (28.0, 0.0), half_extents: (14.0, 10.0)),
                                     damage: 1, knockback: 40.0),
                                ]),
                                (start_s: 0.36, end_s: 0.68, tag: Recovery, volumes: []),
                                (start_s: 0.5, end_s: 0.68, tag: Cancelable(into: ["swat"]), volumes: []),
                            ],
                            events: [
                                (at_s: 0.28, kind: Sfx(cue: "swing_light")),
                            ],
                            gates: (grounded: Some(true)),
                        ),
                    ],
                )),
            ),
        ),
        (
            id: "crate_seed",
            contracts: (
                body: Some((half_extents: (16.0, 16.0))),
                presentation: Some((visual_id: "intro_cart")),
            ),
        ),
    ],
)
"#;

#[test]
fn seed_catalog_parses_and_validates() {
    let doc = EntityCatalogDoc::parse(SEED).unwrap();
    assert!(doc.validate().is_empty(), "{:?}", doc.validate());
    assert_eq!(doc.entities.len(), 2);
    let actor = doc.entity("sandbag_seed").unwrap();
    let moveset = actor.contracts.moveset.as_ref().unwrap();
    let swat = moveset.move_for_verb("attack").unwrap();
    assert_eq!(swat.id, "swat");
    // Prop exposes body+presentation, no moveset — contracts, not
    // categories: nothing marks it "a prop".
    let prop = doc.entity("crate_seed").unwrap();
    assert!(prop.contracts.moveset.is_none());
    assert!(prop.contracts.presentation.is_some());
}

/// A bare move (no windows) with the given id and grounded gate.
fn bare_move(id: &str, grounded: Option<bool>) -> MoveSpec {
    MoveSpec {
        display_name: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.3,
        windows: vec![],
        events: vec![],
        gates: MoveGates {
            grounded,
            ..Default::default()
        },
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: ChargeGesture::default(),
        repeat: None,
        flow: None,
    }
}

// --- CM3: smash-charge scaling + the smash verb class ---

fn startup(end_s: f32) -> MoveWindow {
    MoveWindow {
        start_s: 0.0,
        end_s,
        tag: WindowTag::Startup,
        volumes: vec![],
        sustain_effect: None,
        motion_scale: 1.0,
    }
}

// --- CM7: frame-data introspection ---

#[test]
fn frame_data_derives_startup_active_recovery_cancels_and_reach() {
    let mut m = bare_move("smash_side", None);
    m.duration_s = 0.60;
    m.windows = vec![
        MoveWindow {
            start_s: 0.0,
            end_s: 0.18,
            tag: WindowTag::Startup,
            volumes: vec![],
            sustain_effect: None,
            motion_scale: 1.0,
        },
        MoveWindow {
            start_s: 0.18,
            end_s: 0.26,
            tag: WindowTag::Active,
            volumes: vec![
                HitVolume {
                    // Not a windbox: these fixtures are about authored geometry.
                    hit_sfx: None,
                    shape: VolumeShape::Rect {
                        offset: (28.0, 0.0),
                        half_extents: (16.0, 12.0),
                    },
                    damage: 4,
                    knockback: 100.0,
                    knockback_growth: None,
                    launch_dir: None,
                    on_hit: None,
                    vfx: None,
                    reaction: None,
                },
                HitVolume {
                    // Not a windbox: these fixtures are about authored geometry.
                    hit_sfx: None,
                    shape: VolumeShape::Circle {
                        offset: (30.0, 0.0),
                        radius: 20.0,
                    },
                    damage: 2,
                    knockback: 40.0,
                    knockback_growth: None,
                    launch_dir: None,
                    on_hit: None,
                    vfx: None,
                    reaction: None,
                },
            ],
            sustain_effect: None,
            motion_scale: 1.0,
        },
        MoveWindow {
            start_s: 0.26,
            end_s: 0.42,
            tag: WindowTag::Cancelable {
                into: vec!["jump".to_string(), "dash".to_string()],
                condition: CancelCondition::default(),
            },
            volumes: vec![],
            sustain_effect: None,
            motion_scale: 1.0,
        },
    ];
    let fd = m.frame_data();
    assert_eq!(fd.total_s, 0.60);
    assert!(
        (fd.startup_s - 0.18).abs() < 1e-6,
        "startup = first Active start"
    );
    assert_eq!(fd.active_spans, vec![(0.18, 0.26)]);
    // recovery = duration - last Active end = 0.60 - 0.26.
    assert!((fd.recovery_s - 0.34).abs() < 1e-6, "recovery to move end");
    assert_eq!(fd.cancel_windows.len(), 1);
    assert_eq!(fd.cancel_windows[0].into, vec!["jump", "dash"]);
    assert!((fd.cancel_windows[0].start_s - 0.26).abs() < 1e-6);
    // reach = max(rect 28+16=44, circle 30+20=50) = 50.
    assert!(
        (fd.reach - 50.0).abs() < 1e-6,
        "reach is the farthest volume: {}",
        fd.reach
    );
}

#[test]
fn frame_data_of_a_hitless_move_is_all_startup_no_reach() {
    // A pure-utility move (no Active window): "startup" spans the whole move,
    // reach is zero, no active spans — the brain reads it as unthreatening.
    let mut m = bare_move("taunt", None);
    m.duration_s = 0.5;
    let fd = m.frame_data();
    assert!(fd.active_spans.is_empty());
    assert_eq!(fd.startup_s, 0.5);
    assert_eq!(fd.recovery_s, 0.5);
    assert_eq!(fd.reach, 0.0);
}

#[test]
fn smash_verbs_resolve_distinctly_from_tilt_verbs() {
    // Smash is more verbs: a moveset binds `smash_up` apart from the tilt
    // `attack_up`, resolved by the same verb map. The input side (flick or
    // hold) picks the base verb per game.
    let contract = MovesetContract {
        verbs: [
            ("attack_up".to_string(), "tilt_up_move".to_string()),
            ("smash_up".to_string(), "smash_up_move".to_string()),
        ]
        .into_iter()
        .collect(),
        moves: vec![
            bare_move("tilt_up_move", Some(true)),
            bare_move("smash_up_move", Some(true)),
        ],
    };
    let tilt = contract
        .move_for_directional_verb("attack", AttackDir::Up, true)
        .unwrap();
    let smash = contract
        .move_for_directional_verb("smash", AttackDir::Up, true)
        .unwrap();
    assert_eq!(tilt.id, "tilt_up_move");
    assert_eq!(smash.id, "smash_up_move");
    assert_ne!(tilt.id, smash.id, "smash and tilt are distinct moves");
}

/// The full ability vocabulary, authored as RON: directional verbs, a
/// move-start `start_impulse` lunge, and an `on_hit` pogo volume. A fighter's
/// whole kit is data, not code.
const R2_FIGHTER: &str = r#"
(
    schema_version: 1,
    entities: [(
        id: "data_fighter",
        contracts: (
            moveset: Some((
                verbs: {
                    "attack": "jab",
                    "attack_air_down": "dair",
                },
                moves: [
                    (
                        id: "jab",
                        clip: (clip: "jab", fallbacks: ["idle"]),
                        duration_s: 0.30,
                        windows: [
                            (start_s: 0.04, end_s: 0.14, tag: Active, volumes: [
                                (shape: Rect(offset: (28.0, 0.0), half_extents: (20.0, 14.0)),
                                 damage: 2, knockback: 120.0),
                            ]),
                        ],
                        start_impulse: Some((30.0, 0.0)),
                        smash_charge_mult: 1.0,
                    ),
                    (
                        id: "dair",
                        clip: (clip: "dair", fallbacks: ["idle"]),
                        duration_s: 0.28,
                        gates: (grounded: Some(false)),
                        windows: [
                            (start_s: 0.03, end_s: 0.14, tag: Active, volumes: [
                                (shape: Rect(offset: (0.0, 26.0), half_extents: (18.0, 18.0)),
                                 damage: 3, knockback: 0.0,
                                 on_hit: Some((key: "pogo_bounce"))),
                            ]),
                        ],
                    ),
                ],
            )),
        ),
    )],
)
"#;

#[test]
fn the_full_r2_vocabulary_is_authorable_as_ron() {
    let doc = EntityCatalogDoc::parse(R2_FIGHTER).unwrap();
    assert!(doc.validate().is_empty(), "{:?}", doc.validate());
    let ms = doc
        .entity("data_fighter")
        .unwrap()
        .contracts
        .moveset
        .as_ref()
        .unwrap();
    // Directional resolution off authored verbs: aerial + down → the dair,
    // grounded neutral → the jab (the aerial-only dair is gate-skipped).
    let dair = ms
        .move_for_directional_verb("attack", AttackDir::Down, false)
        .unwrap();
    assert_eq!(dair.id, "dair");
    let jab = ms
        .move_for_directional_verb("attack", AttackDir::Down, true)
        .unwrap();
    assert_eq!(jab.id, "jab", "grounded skips the aerial-only dair");
    // The jab carries its authored move-start lunge.
    assert_eq!(jab.start_impulse, Some((30.0, 0.0)));
    // The dair's Active volume carries the pogo on-hit technique.
    let vol = dair
        .windows
        .iter()
        .flat_map(|w| &w.volumes)
        .next()
        .expect("dair has an active volume");
    assert_eq!(
        vol.on_hit.as_ref().expect("dair volume authors on_hit").key,
        "pogo_bounce",
    );
}

#[test]
fn directional_verb_chain_orders_most_specific_first() {
    assert_eq!(
        directional_verb_chain("attack", AttackDir::Down, false),
        vec!["attack_air_down", "attack_down", "attack_air", "attack"],
    );
    assert_eq!(
        directional_verb_chain("attack", AttackDir::Down, true),
        vec!["attack_down", "attack"],
    );
    assert_eq!(
        directional_verb_chain("attack", AttackDir::Forward, false),
        vec![
            "attack_air_forward",
            "attack_forward",
            "attack_air",
            "attack",
        ],
    );
    assert_eq!(
        directional_verb_chain("attack", AttackDir::Forward, true),
        vec!["attack_forward", "attack"],
    );
    assert_eq!(
        directional_verb_chain("attack", AttackDir::Neutral, true),
        vec!["attack"],
    );
    assert_eq!(
        directional_verb_chain("attack", AttackDir::Neutral, false),
        vec!["attack_air", "attack"],
    );
}

/// The running grab: the capture kit's version of the dash attack.
///
/// Four cases: a running grounded body uses its running grab; a standing body
/// never does; an airborne running body never does (a run is a ground stance,
/// and the capture kit is grounded); and a contract without the variant
/// resolves its press to the plain grab.
#[test]
fn a_running_body_reaches_with_its_running_grab() {
    let with_dash_grab = MovesetContract {
        verbs: BTreeMap::from([
            ("grab".to_string(), "grab".to_string()),
            ("grab_dash".to_string(), "grab_run".to_string()),
        ]),
        moves: vec![
            bare_move("grab", Some(true)),
            bare_move("grab_run", Some(true)),
        ],
    };
    let pick = |grounded, running| {
        with_dash_grab
            .move_for_flat_verb("grab", grounded, running)
            .map(|mv| mv.id.clone())
    };
    assert_eq!(
        pick(true, true).as_deref(),
        Some("grab_run"),
        "a run did not own the press"
    );
    assert_eq!(
        pick(true, false).as_deref(),
        Some("grab"),
        "a standing grab became the running one"
    );
    // Nothing, not the standing grab. Both variants here are
    // `grounded: Some(true)`, the gate the capture kit authors on its whole
    // vocabulary. An airborne press must not start a grounded-only move.
    assert_eq!(
        pick(false, true),
        None,
        "an airborne press started a grounded-only grab"
    );
    assert_eq!(
        pick(false, false),
        None,
        "an airborne press started a grounded-only grab"
    );

    // The gate refuses it, not the verb: an ungated standing grab still
    // answers an airborne press. Without this, the assertions above would also
    // pass if the lookup had stopped resolving `base`.
    let ungated = MovesetContract {
        verbs: BTreeMap::from([("grab".to_string(), "grab".to_string())]),
        moves: vec![bare_move("grab", None)],
    };
    assert_eq!(
        ungated
            .move_for_flat_verb("grab", false, false)
            .map(|mv| mv.id.as_str()),
        Some("grab"),
        "an ungated grab must still answer an airborne press"
    );

    // The word is spelled once. `GRAB_DASH_VERB` exists because the binding
    // table needs a `&'static str`; it must match the suffix rule.
    assert_eq!(
        super::GRAB_DASH_VERB,
        super::dash_stance_verb(super::GRAB_VERB)
    );

    // Positive control: a contract with no running grab is unchanged.
    let without = MovesetContract {
        verbs: BTreeMap::from([("grab".to_string(), "grab".to_string())]),
        moves: vec![bare_move("grab", Some(true))],
    };
    assert_eq!(
        without
            .move_for_flat_verb("grab", true, true)
            .unwrap()
            .id
            .clone(),
        "grab",
        "a contract without the variant must resolve exactly as before"
    );
}

/// The dash attack is a stance, and it outranks the direction.
///
/// Four cases: a dashing body gets its dash attack even with a direction held;
/// a standing body never does; an airborne dashing body never does (a dash is
/// a ground stance); and a fighter that authors none resolves as before.
#[test]
fn a_running_body_gets_its_dash_attack_before_any_direction() {
    let with_dash = MovesetContract {
        verbs: BTreeMap::from([
            ("attack".to_string(), "attack".to_string()),
            ("attack_forward".to_string(), "ftilt".to_string()),
            ("attack_dash".to_string(), "dash".to_string()),
        ]),
        moves: vec![
            bare_move("attack", None),
            bare_move("ftilt", Some(true)),
            bare_move("dash", Some(true)),
        ],
    };
    let pick = |grounded, running| {
        with_dash
            .move_for_attack("attack", AttackDir::Forward, grounded, running)
            .unwrap()
            .id
            .clone()
    };
    assert_eq!(pick(true, true), "dash", "the direction beat the gait");
    assert_eq!(
        pick(true, false),
        "ftilt",
        "a standing press became a dash attack"
    );
    assert_eq!(
        pick(false, true),
        "attack",
        "a dash attack was thrown in the air"
    );

    // The word is spelled once: the selector and the runtime's vocabulary
    // both build this verb through `dash_stance_verb`.
    assert_eq!(super::dash_stance_verb("attack"), "attack_dash");

    //  the floor: a fighter with no dash attack is untouched.
    let without = MovesetContract {
        verbs: BTreeMap::from([
            ("attack".to_string(), "attack".to_string()),
            ("attack_forward".to_string(), "ftilt".to_string()),
        ]),
        moves: vec![bare_move("attack", None), bare_move("ftilt", Some(true))],
    };
    assert_eq!(
        without
            .move_for_attack("attack", AttackDir::Forward, true, true)
            .unwrap()
            .id,
        "ftilt",
    );
}

#[test]
fn directional_resolution_falls_back_and_respects_gates() {
    // Only `attack` authored: every direction resolves to it.
    let base_only = MovesetContract {
        verbs: BTreeMap::from([("attack".to_string(), "attack".to_string())]),
        moves: vec![bare_move("attack", None)],
    };
    assert_eq!(
        base_only
            .move_for_directional_verb("attack", AttackDir::Down, false)
            .unwrap()
            .id,
        "attack",
    );

    // An aerial-only down-air (a pogo host): aerial+down picks it; the
    // grounded chain skips it (gate) and falls through to `attack`.
    let with_dair = MovesetContract {
        verbs: BTreeMap::from([
            ("attack".to_string(), "attack".to_string()),
            ("attack_air_down".to_string(), "dair".to_string()),
        ]),
        moves: vec![bare_move("attack", None), bare_move("dair", Some(false))],
    };
    assert_eq!(
        with_dair
            .move_for_directional_verb("attack", AttackDir::Down, false)
            .unwrap()
            .id,
        "dair",
    );
    assert_eq!(
        with_dair
            .move_for_directional_verb("attack", AttackDir::Down, true)
            .unwrap()
            .id,
        "attack",
    );

    // A grounded-only `attack_down` (a down-tilt) is chosen grounded but
    // skipped for an airborne body — gate-respecting fallthrough.
    let with_dtilt = MovesetContract {
        verbs: BTreeMap::from([
            ("attack".to_string(), "attack".to_string()),
            ("attack_down".to_string(), "dtilt".to_string()),
        ]),
        moves: vec![bare_move("attack", None), bare_move("dtilt", Some(true))],
    };
    assert_eq!(
        with_dtilt
            .move_for_directional_verb("attack", AttackDir::Down, true)
            .unwrap()
            .id,
        "dtilt",
    );
    assert_eq!(
        with_dtilt
            .move_for_directional_verb("attack", AttackDir::Down, false)
            .unwrap()
            .id,
        "attack",
    );
}

#[test]
fn round_trips_through_ron() {
    let doc = EntityCatalogDoc::parse(SEED).unwrap();
    let text = doc.to_ron().unwrap();
    let back = EntityCatalogDoc::parse(&text).unwrap();
    assert_eq!(doc, back);
}

#[test]
fn move_timeline_queries_answer_the_sim() {
    let doc = EntityCatalogDoc::parse(SEED).unwrap();
    let moveset = doc
        .entity("sandbag_seed")
        .unwrap()
        .contracts
        .moveset
        .as_ref()
        .unwrap();
    let swat = moveset.move_by_id("swat").unwrap();
    // Proper-time queries: nothing live during startup, one volume
    // mid-active, nothing during recovery.
    assert_eq!(swat.active_volumes_at(0.1).count(), 0);
    assert_eq!(swat.active_volumes_at(0.30).count(), 1);
    assert_eq!(swat.active_volumes_at(0.5).count(), 0);
    // Phase is normalized move progress — what the clip samples by.
    assert!((swat.phase_at(0.34) - 0.5).abs() < 1e-6);
    assert_eq!(swat.phase_at(2.0), 1.0);
}

#[test]
fn validators_catch_structural_violations() {
    let bad = r#"
    (
        schema_version: 1,
        entities: [
            (
                id: "bad",
                contracts: (
                    moveset: Some((
                        verbs: { "attack": "missing" },
                        moves: [
                            (
                                id: "broken",
                                clip: (clip: ""),
                                duration_s: 0.5,
                                windows: [
                                    (start_s: 0.4, end_s: 0.9, tag: Startup, volumes: []),
                                    (start_s: 0.0, end_s: 0.2, tag: Recovery, volumes: [
                                        (shape: Circle(offset: (0.0, 0.0), radius: 0.0),
                                         damage: 1, knockback: 0.0),
                                    ]),
                                    (start_s: 0.2, end_s: 0.4, tag: Cancelable(into: ["nowhere"]), volumes: []),
                                ],
                                events: [ (at_s: 0.9, kind: Effect((key: "boom"))) ],
                            ),
                        ],
                    )),
                ),
            ),
            ( id: "bad", contracts: () ),
        ],
    )
    "#;
    let doc = EntityCatalogDoc::parse(bad).unwrap();
    let errors = doc.validate();
    let has = |f: &dyn Fn(&CatalogError) -> bool| errors.iter().any(|e| f(e));
    assert!(has(&|e| matches!(
        e,
        CatalogError::DuplicateEntityId { .. }
    )));
    assert!(has(&|e| matches!(e, CatalogError::WindowOutOfRange { .. })));
    assert!(has(&|e| matches!(
        e,
        CatalogError::VolumesOnInactiveWindow { .. }
    )));
    assert!(has(&|e| matches!(e, CatalogError::DegenerateVolume { .. })));
    assert!(has(&|e| matches!(
        e,
        CatalogError::UnknownCancelTarget { .. }
    )));
    assert!(has(&|e| matches!(e, CatalogError::UnknownVerbMove { .. })));
    assert!(has(&|e| matches!(e, CatalogError::EventOutOfRange { .. })));
    assert!(has(&|e| matches!(e, CatalogError::EmptyClipBinding { .. })));
}

/// The timeline is queried in the owner's proper time, so a dilated actor at
/// 0.25× world rate reaches its active window after 4× the world time. The
/// caller integrates proper time from the owner's dt; the schema holds no
/// world time.
#[test]
fn proper_time_integration_is_callers_dt_sum() {
    let doc = EntityCatalogDoc::parse(SEED).unwrap();
    let moveset = doc
        .entity("sandbag_seed")
        .unwrap()
        .contracts
        .moveset
        .as_ref()
        .unwrap();
    let swat = moveset.move_by_id("swat").unwrap();
    // Simulate a 0.25×-dilated owner: 60 world frames of 16ms reach only
    // 0.24s proper — still in startup. An undilated owner is active.
    let dilated: f32 = (0..60).map(|_| 0.016 * 0.25).sum();
    let undilated: f32 = (0..60).map(|_| 0.016).sum();
    assert_eq!(swat.active_volumes_at(dilated).count(), 0);
    assert_eq!(swat.active_volumes_at(undilated - 0.65).count(), 1);
}

fn hurt_rect(offset: (f32, f32), half_extents: (f32, f32)) -> HurtboxVolume {
    HurtboxVolume {
        shape: VolumeShape::Rect {
            offset,
            half_extents,
        },
    }
}

fn hurt_circle(offset: (f32, f32), radius: f32) -> HurtboxVolume {
    HurtboxVolume {
        shape: VolumeShape::Circle { offset, radius },
    }
}

fn hurt_timeline(keyframes: Vec<HurtboxKeyframe>) -> HurtboxTimeline {
    HurtboxTimeline { keyframes }
}

#[test]
fn hurtbox_timeline_is_piecewise_constant_and_supports_multiple_volumes() {
    let standing = hurt_rect((0.0, 0.0), (8.0, 16.0));
    let compressed = hurt_circle((0.0, 2.0), 10.0);
    let tail = hurt_rect((-12.0, 0.0), (5.0, 4.0));
    let timeline = hurt_timeline(vec![
        HurtboxKeyframe {
            at_s: 0.0,
            volumes: vec![standing],
        },
        HurtboxKeyframe {
            at_s: 0.2,
            volumes: vec![compressed, tail],
        },
    ]);

    assert_eq!(timeline.volumes_at(-0.001), Some(&[standing][..]));
    assert_eq!(timeline.volumes_at(0.199), Some(&[standing][..]));
    assert_eq!(
        timeline.volumes_at(0.2),
        Some(&[compressed, tail][..]),
        "the keyframe owns its exact start time"
    );
    assert_eq!(timeline.volumes_at(99.0), Some(&[compressed, tail][..]));
    assert_eq!(timeline.volumes_at(f32::NAN), None);
}

#[test]
fn hurtbox_doc_selects_move_then_pose_then_default() {
    let default = hurt_rect((0.0, 0.0), (8.0, 16.0));
    let crouch = hurt_rect((0.0, -5.0), (9.0, 10.0));
    let roll_start = hurt_circle((0.0, 0.0), 9.0);
    let roll_late = hurt_circle((6.0, 0.0), 7.0);
    let doc = HurtboxDoc {
        default: Some(hurt_timeline(vec![HurtboxKeyframe {
            at_s: 0.0,
            volumes: vec![default],
        }])),
        poses: BTreeMap::from([(
            "crouch".to_string(),
            hurt_timeline(vec![HurtboxKeyframe {
                at_s: 0.0,
                volumes: vec![crouch],
            }]),
        )]),
        moves: BTreeMap::from([(
            "roll".to_string(),
            hurt_timeline(vec![
                HurtboxKeyframe {
                    at_s: 0.0,
                    volumes: vec![roll_start],
                },
                HurtboxKeyframe {
                    at_s: 0.3,
                    volumes: vec![roll_late],
                },
            ]),
        )]),
    };

    assert_eq!(
        doc.volumes_for(Some(("roll", 0.4)), Some(("crouch", 0.0))),
        Some(&[roll_late][..])
    );
    assert_eq!(
        doc.volumes_for(Some(("unknown", 0.0)), Some(("crouch", 0.0))),
        Some(&[crouch][..])
    );
    assert_eq!(
        doc.volumes_for(None, Some(("unknown", 0.0))),
        Some(&[default][..])
    );

    let unauthored = HurtboxDoc::default();
    assert_eq!(unauthored.volumes_for(None, None), None);
}

#[test]
fn hurtbox_validation_names_the_exact_profile_keyframe_and_volume() {
    let doc = HurtboxDoc {
        default: Some(HurtboxTimeline::default()),
        poses: BTreeMap::from([(
            " ".to_string(),
            hurt_timeline(vec![HurtboxKeyframe {
                at_s: 0.1,
                volumes: vec![hurt_circle((0.0, 0.0), 0.0)],
            }]),
        )]),
        moves: BTreeMap::from([(
            "roll".to_string(),
            hurt_timeline(vec![
                HurtboxKeyframe {
                    at_s: 0.0,
                    volumes: vec![hurt_circle((0.0, 0.0), 4.0)],
                },
                HurtboxKeyframe {
                    at_s: 0.0,
                    volumes: vec![],
                },
            ]),
        )]),
    };

    let errors = doc.validate();
    assert!(errors.contains(&HurtboxError::EmptyTimeline {
        source: HurtboxSource::Default,
    }));
    assert!(errors.contains(&HurtboxError::EmptySourceId {
        source: HurtboxSource::Pose(" ".to_string()),
    }));
    assert!(errors.contains(&HurtboxError::FirstKeyframeNotZero {
        source: HurtboxSource::Pose(" ".to_string()),
    }));
    assert!(errors.contains(&HurtboxError::DegenerateVolume {
        source: HurtboxSource::Pose(" ".to_string()),
        keyframe: 0,
        volume: 0,
    }));
    assert!(errors.contains(&HurtboxError::NonIncreasingKeyframeTime {
        source: HurtboxSource::Move("roll".to_string()),
        index: 1,
    }));
    assert!(errors.contains(&HurtboxError::EmptyKeyframe {
        source: HurtboxSource::Move("roll".to_string()),
        index: 1,
    }));
}

#[test]
fn entity_catalog_parses_and_validates_authored_hurtboxes() {
    let ron = r#"
(
    schema_version: 1,
    entities: [
        (
            id: "ball_fighter",
            contracts: (
                body: Some((half_extents: (10.0, 16.0))),
                hurtboxes: Some((
                    default: Some((keyframes: [
                        (at_s: 0.0, volumes: [
                            (shape: Rect(offset: (0.0, 0.0), half_extents: (10.0, 16.0))),
                        ]),
                    ])),
                    poses: {
                        "crouch": (keyframes: [
                            (at_s: 0.0, volumes: [
                                (shape: Rect(offset: (0.0, -4.0), half_extents: (11.0, 10.0))),
                            ]),
                        ]),
                    },
                    moves: {
                        "roll": (keyframes: [
                            (at_s: 0.0, volumes: [
                                (shape: Circle(offset: (0.0, 0.0), radius: 9.0)),
                            ]),
                            (at_s: 0.25, volumes: [
                                (shape: Circle(offset: (5.0, 0.0), radius: 8.0)),
                            ]),
                        ]),
                    },
                )),
                moveset: Some((
                    verbs: { "attack": "roll" },
                    moves: [
                        (
                            id: "roll",
                            clip: (clip: "roll", fallbacks: []),
                            duration_s: 0.8,
                            windows: [],
                            events: [],
                            gates: (grounded: None),
                        ),
                    ],
                )),
            ),
        ),
    ],
)
"#;

    let doc = EntityCatalogDoc::parse(ron).expect("hurtbox document parses as RON");
    assert!(doc.validate().is_empty(), "{:?}", doc.validate());
    let hurtboxes = doc
        .entity("ball_fighter")
        .and_then(|entity| entity.contracts.hurtboxes.as_ref())
        .expect("character carries authored hurtboxes");
    assert_eq!(
        hurtboxes.volumes_for(Some(("roll", 0.3)), Some(("crouch", 0.0))),
        Some(&[hurt_circle((5.0, 0.0), 8.0)][..])
    );
}

#[test]
fn catalog_validation_wraps_hurtbox_problems_with_the_entity_id() {
    let doc = EntityCatalogDoc {
        schema_version: 1,
        entities: vec![EntityDef {
            id: "bad_body".to_string(),
            contracts: EntityContracts {
                hurtboxes: Some(HurtboxDoc {
                    default: Some(HurtboxTimeline::default()),
                    moves: BTreeMap::from([(
                        "missing_move".to_string(),
                        hurt_timeline(vec![HurtboxKeyframe {
                            at_s: 0.0,
                            volumes: vec![hurt_circle((0.0, 0.0), 4.0)],
                        }]),
                    )]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        }],
    };

    assert_eq!(
        doc.validate(),
        vec![
            CatalogError::Hurtbox {
                entity: "bad_body".to_string(),
                problem: HurtboxError::EmptyTimeline {
                    source: HurtboxSource::Default,
                },
            },
            CatalogError::UnknownHurtboxMove {
                entity: "bad_body".to_string(),
                move_id: "missing_move".to_string(),
            },
        ]
    );
}

#[test]
fn move_hurtbox_keyframes_must_fit_inside_the_move_clock() {
    let mut roll = bare_move("roll", None);
    roll.duration_s = 0.2;
    let doc = EntityCatalogDoc {
        schema_version: 1,
        entities: vec![EntityDef {
            id: "ball_fighter".to_string(),
            contracts: EntityContracts {
                hurtboxes: Some(HurtboxDoc {
                    moves: BTreeMap::from([(
                        "roll".to_string(),
                        hurt_timeline(vec![
                            HurtboxKeyframe {
                                at_s: 0.0,
                                volumes: vec![hurt_circle((0.0, 0.0), 4.0)],
                            },
                            HurtboxKeyframe {
                                at_s: 0.3,
                                volumes: vec![hurt_circle((3.0, 0.0), 4.0)],
                            },
                        ]),
                    )]),
                    ..Default::default()
                }),
                moveset: Some(MovesetContract {
                    verbs: BTreeMap::from([("attack".to_string(), "roll".to_string())]),
                    moves: vec![roll],
                }),
                ..Default::default()
            },
        }],
    };

    assert_eq!(
        doc.validate(),
        vec![CatalogError::HurtboxKeyframeOutOfMoveRange {
            entity: "ball_fighter".to_string(),
            move_id: "roll".to_string(),
            index: 1,
        }]
    );
}

/// A zero-width window is legal; an inverted one is not.
///
/// This is safe only because every window predicate is the half-open
/// `start_s <= t < end_s`, so nothing fires inside a zero-width window. If a
/// predicate becomes end-inclusive, tighten the validator.
#[test]
fn a_zero_width_window_is_legal_but_an_inverted_one_is_not() {
    let doc_with = |windows: &str| {
        format!(
            r#"
        (
            schema_version: 1,
            entities: [
                (
                    id: "fighter",
                    contracts: (
                        moveset: Some((
                            verbs: {{ "attack": "swing" }},
                            moves: [
                                (
                                    id: "swing",
                                    clip: (clip: "attack_side"),
                                    duration_s: 0.5,
                                    windows: [{windows}],
                                    events: [],
                                ),
                            ],
                        )),
                    ),
                ),
            ],
        )
        "#
        )
    };

    let out_of_range = |source: String| {
        EntityCatalogDoc::parse(&source)
            .unwrap()
            .validate()
            .into_iter()
            .any(|e| matches!(e, CatalogError::WindowOutOfRange { .. }))
    };

    // No windup: Startup collapses onto the frame Active begins.
    assert!(
        !out_of_range(doc_with(
            r#"(start_s: 0.0, end_s: 0.0, tag: Startup, volumes: []),
               (start_s: 0.0, end_s: 0.3, tag: Active, volumes: []),
               (start_s: 0.3, end_s: 0.5, tag: Recovery, volumes: [])"#
        )),
        "a zero-width Startup window is what `simple_melee` emits for windup_s: 0.0"
    );

    // Inverted is still a mistake, and must stay one.
    assert!(
        out_of_range(doc_with(
            r#"(start_s: 0.3, end_s: 0.1, tag: Startup, volumes: [])"#
        )),
        "loosening `>=` to `>` must not have made an INVERTED window legal too"
    );
}

// ---------------------------------------------------------------------------
// Timed authored self-displacement (`MoveEventKind::Impulse`) and the lift
// affordance derived from it.
// ---------------------------------------------------------------------------

/// A minimal move: one Startup window, one Active window carrying one volume.
fn timed_move(id: &str, duration_s: f32, events: Vec<MoveEvent>) -> MoveSpec {
    MoveSpec {
        display_name: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: "attack".into(),
            fallbacks: Vec::new(),
        },
        duration_s,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: 0.10,
                tag: WindowTag::Startup,
                volumes: Vec::new(),
                motion_scale: 1.0,
                sustain_effect: None,
            },
            MoveWindow {
                start_s: 0.10,
                end_s: 0.18,
                tag: WindowTag::Active,
                volumes: vec![HitVolume {
                    // Not a windbox: these fixtures are about authored geometry.
                    shape: VolumeShape::Rect {
                        offset: (20.0, 0.0),
                        half_extents: (16.0, 12.0),
                    },
                    damage: 4,
                    knockback: 60.0,
                    knockback_growth: Some(1.0),
                    launch_dir: None,
                    on_hit: None,
                    vfx: None,
                    hit_sfx: None,
                    reaction: None,
                }],
                motion_scale: 1.0,
                sustain_effect: None,
            },
        ],
        events,
        gates: MoveGates::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: ChargeGesture::default(),
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
    }
}

/// A move that sets an against-gravity speed advertises it; one that only
/// adds to the body's own speed does not.
///
/// An `Add` produces a speed only together with the body's current motion, so
/// no static reader can name one. Otherwise a jab's small hop would read as a
/// recovery.
#[test]
fn only_a_commanded_impulse_advertises_lift() {
    let commanded = timed_move(
        "ascend",
        0.9,
        vec![MoveEvent {
            at_s: 0.20,
            kind: MoveEventKind::Impulse {
                local: (0.0, -980.0),
                mode: ImpulseMode::Set,
            },
        }],
    );
    let frames = commanded.frame_data();
    assert_eq!(frames.lift_speed, 980.0);
    assert_eq!(frames.lift_at_s, 0.20);

    let mut additive = commanded.clone();
    additive.events[0].kind = MoveEventKind::Impulse {
        local: (0.0, -980.0),
        mode: ImpulseMode::Add,
    };
    assert_eq!(
        additive.frame_data().lift_speed,
        0.0,
        "an ADDITIVE up-impulse commands no speed — its result is whatever the \
         body was already doing, so nothing downstream may read it as a way home"
    );

    // The identity case: an ordinary strike lifts nobody.
    assert_eq!(
        timed_move("jab", 0.3, Vec::new()).frame_data().lift_speed,
        0.0
    );
}

/// A downward commanded impulse is not lift. Only the sign separates a dive
/// from a rise, so it has its own test.
#[test]
fn a_commanded_dive_is_not_a_lift() {
    let dive = timed_move(
        "plunge",
        0.6,
        vec![MoveEvent {
            at_s: 0.12,
            kind: MoveEventKind::Impulse {
                local: (0.0, 1200.0),
                mode: ImpulseMode::Set,
            },
        }],
    );
    assert_eq!(dive.frame_data().lift_speed, 0.0);
}

/// The strongest lift wins, and a tie breaks on the earlier moment. Two
/// bursts on one timeline are legal (a hop into a rise), and the result must
/// not depend on declaration order.
#[test]
fn the_strongest_lift_wins_and_ties_break_on_the_earlier_moment() {
    let two = timed_move(
        "double_rise",
        1.2,
        vec![
            MoveEvent {
                at_s: 0.50,
                kind: MoveEventKind::Impulse {
                    local: (0.0, -400.0),
                    mode: ImpulseMode::Set,
                },
            },
            MoveEvent {
                at_s: 0.20,
                kind: MoveEventKind::Impulse {
                    local: (0.0, -900.0),
                    mode: ImpulseMode::Set,
                },
            },
        ],
    );
    let frames = two.frame_data();
    assert_eq!((frames.lift_speed, frames.lift_at_s), (900.0, 0.20));

    let tied = timed_move(
        "tied",
        1.2,
        vec![
            MoveEvent {
                at_s: 0.50,
                kind: MoveEventKind::Impulse {
                    local: (0.0, -900.0),
                    mode: ImpulseMode::Set,
                },
            },
            MoveEvent {
                at_s: 0.20,
                kind: MoveEventKind::Impulse {
                    local: (0.0, -900.0),
                    mode: ImpulseMode::Set,
                },
            },
        ],
    );
    assert_eq!(tied.frame_data().lift_at_s, 0.20);
}

/// A commanded velocity is a vector, and both halves come from the same event.
#[test]
fn a_diagonal_command_reports_both_of_its_halves() {
    let grapple = timed_move(
        "grapple",
        1.0,
        vec![MoveEvent {
            at_s: 0.16,
            kind: MoveEventKind::Impulse {
                local: (980.0, -300.0),
                mode: ImpulseMode::Set,
            },
        }],
    );
    let frames = grapple.frame_data();
    assert_eq!(frames.lift_speed, 300.0);
    assert_eq!(frames.lift_at_s, 0.16);
    assert_eq!(
        frames.lift_side, 980.0,
        "the half that actually crosses the gap must survive the derivation"
    );

    // The side is read from the winning event. Here the strong rise has no
    // side and the weak one has a large one; mixing them would report a move
    // that does not exist.
    let mixed = timed_move(
        "mixed",
        1.4,
        vec![
            MoveEvent {
                at_s: 0.10,
                kind: MoveEventKind::Impulse {
                    local: (900.0, -100.0),
                    mode: ImpulseMode::Set,
                },
            },
            MoveEvent {
                at_s: 0.40,
                kind: MoveEventKind::Impulse {
                    local: (0.0, -800.0),
                    mode: ImpulseMode::Set,
                },
            },
        ],
    );
    let frames = mixed.frame_data();
    assert_eq!((frames.lift_speed, frames.lift_at_s), (800.0, 0.40));
    assert_eq!(frames.lift_side, 0.0);

    // A move that commands its owner backwards says so with a sign (for
    // example recoil from firing forwards).
    let recoil = timed_move(
        "recoil",
        0.8,
        vec![MoveEvent {
            at_s: 0.12,
            kind: MoveEventKind::Impulse {
                local: (-560.0, -120.0),
                mode: ImpulseMode::Set,
            },
        }],
    );
    assert_eq!(recoil.frame_data().lift_side, -560.0);

    // The identity case: a move that lifts nobody has no side either.
    assert_eq!(
        timed_move("jab", 0.3, Vec::new()).frame_data().lift_side,
        0.0
    );
}

/// An `Impulse` event parses from RON with `mode` omitted, as the additive
/// meaning `start_impulse` has.
#[test]
fn an_authored_impulse_defaults_to_the_additive_meaning() {
    let parsed: MoveEventKind = ron::from_str("Impulse(local: (0.0, -900.0))")
        .expect("an impulse with no mode is authorable");
    assert_eq!(
        parsed,
        MoveEventKind::Impulse {
            local: (0.0, -900.0),
            mode: ImpulseMode::Add,
        }
    );
}

/// A move with a charge multiplier and no authored policy gets a derived one:
/// the hold sits inside the move's own leading Startup window.
#[test]
fn a_smash_charge_policy_is_derived_from_the_moves_own_windup() {
    let mut spec = MoveSpec {
        display_name: None,
        id: "fsmash".to_string(),
        clip: ClipBinding {
            clip: "fsmash".to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.5,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: 0.3,
                tag: WindowTag::Startup,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            MoveWindow {
                start_s: 0.3,
                end_s: 0.4,
                tag: WindowTag::Active,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
        ],
        events: vec![],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.7,
        smash_charge: None,
        charge_gesture: ChargeGesture::default(),
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        flow: None,
    };
    let derived = spec.charge_policy().expect("a paying smash charges");
    // The hold sits early in the windup. The rest of the windup and every
    // Active window play on release.
    assert_eq!(
        derived.hold_at_s,
        0.3 * CHARGE_POSE_FRACTION,
        "the charge pose left the windup"
    );
    // The invariant, not only the number: a held charge must not stand inside
    // a live strike. Active membership is `start_s <= t < end_s`, so a hold at
    // the first Active instant is already inside it.
    let first_active = spec
        .windows
        .iter()
        .filter(|w| matches!(w.tag, WindowTag::Active))
        .map(|w| w.start_s)
        .fold(f32::MAX, f32::min);
    assert!(
        derived.hold_at_s < first_active,
        "the charge freezes at {} and the first strike goes live at {first_active}",
        derived.hold_at_s
    );
    assert_eq!(derived.max_hold_s, SmashChargeSpec::DEFAULT_MAX_HOLD_S);

    // A move that pays nothing for a hold must not freeze its timeline for one.
    let mut unpaid = spec.clone();
    unpaid.smash_charge_mult = 1.0;
    assert!(unpaid.charge_policy().is_none());

    // Authoring overrides the derivation...
    spec.smash_charge = Some(SmashChargeSpec {
        hold_at_s: 0.12,
        max_hold_s: 0.8,
        stores: false,
        roots: true,
        sustain: ChargeSustain::WhileHeld,
    });
    assert_eq!(spec.charge_policy().unwrap().hold_at_s, 0.12);

    // ... including all the way to "this smash does not hold".
    spec.smash_charge = Some(SmashChargeSpec {
        hold_at_s: 0.12,
        max_hold_s: 0.0,
        stores: false,
        roots: true,
        sustain: ChargeSustain::WhileHeld,
    });
    assert!(
        spec.charge_policy().is_none(),
        "a zero maximum is how a move says it cannot be charged"
    );
}

/// The frame data carries the move's own charge point, and `None` for a move
/// that does not charge. `startup_s` (when the first hit lands) is not the
/// charge start.
#[test]
fn frame_data_reports_the_charge_hold_point_and_only_for_a_charging_move() {
    let mut m = bare_move("smash", None);
    m.windows = vec![startup(0.4)];
    assert_eq!(
        m.frame_data().charge_hold_at_s,
        None,
        "a move authoring no payoff does not charge, so it has no hold point"
    );
    m.smash_charge_mult = 2.0;
    let policy = m.charge_policy().expect("a paying smash resolves a policy");
    assert_eq!(m.frame_data().charge_hold_at_s, Some(policy.hold_at_s));
}

/// Authoring must not put a hitbox inside a held charge.
///
/// The derived hold point is clamped before the first Active instant, but an
/// authored `smash_charge` overrides that clamp, so validation must catch a
/// bad override.
#[test]
fn an_authored_charge_hold_inside_a_live_strike_fails_validation() {
    let make = |hold_at_s: f32| {
        let mut m = bare_move("smash", None);
        m.duration_s = 0.5;
        m.smash_charge_mult = 2.0;
        m.windows = vec![
            MoveWindow {
                start_s: 0.0,
                end_s: 0.2,
                tag: WindowTag::Startup,
                volumes: vec![],
                motion_scale: 1.0,
                sustain_effect: None,
            },
            MoveWindow {
                start_s: 0.2,
                end_s: 0.4,
                tag: WindowTag::Active,
                volumes: vec![],
                motion_scale: 1.0,
                sustain_effect: None,
            },
        ];
        m.smash_charge = Some(SmashChargeSpec {
            hold_at_s,
            max_hold_s: 0.8,
            stores: false,
            roots: true,
            sustain: ChargeSustain::WhileHeld,
        });
        m
    };
    let problems = |m: MoveSpec| -> Vec<CatalogError> {
        let doc = EntityCatalogDoc {
            schema_version: 1,
            entities: vec![EntityDef {
                id: "fighter".into(),
                contracts: EntityContracts {
                    moveset: Some(MovesetContract {
                        verbs: Default::default(),
                        moves: vec![m],
                    }),
                    ..Default::default()
                },
            }],
        };
        doc.validate()
    };

    // On the first Active instant is already inside it: membership is
    // `start_s <= t < end_s`.
    let at_the_edge = problems(make(0.2));
    assert!(
        at_the_edge
            .iter()
            .any(|e| matches!(e, CatalogError::ChargeHoldOutsideWindup { .. })),
        "a charge frozen exactly where the strike goes live was accepted: \
         {at_the_edge:?}"
    );
    assert!(
        problems(make(0.3))
            .iter()
            .any(|e| matches!(e, CatalogError::ChargeHoldOutsideWindup { .. })),
        "a charge frozen past the first strike was accepted"
    );
    // ...and a pose inside the windup is fine, or the check would just refuse
    // every authored policy.
    assert!(
        !problems(make(0.05))
            .iter()
            .any(|e| matches!(e, CatalogError::ChargeHoldOutsideWindup { .. })),
        "a legal charge pose inside the windup was refused"
    );
}

/// The inverse must agree with the composer. Every id `directional_verb_chain`
/// builds reduces to its base, including aerial forms: `_air_forward` would
/// reduce to `attack_air` under a naive "strip the last suffix" rule.
#[test]
fn every_composed_verb_id_reduces_to_the_base_it_was_built_from() {
    use crate::{base_verb_of, dash_stance_verb, directional_verb_chain, AttackDir};

    for base in ["attack", "smash", "special", "grab"] {
        for dir in [
            AttackDir::Neutral,
            AttackDir::Forward,
            AttackDir::Back,
            AttackDir::Up,
            AttackDir::Down,
        ] {
            for grounded in [true, false] {
                for verb in directional_verb_chain(base, dir, grounded) {
                    assert_eq!(
                        base_verb_of(&verb),
                        base,
                        "{verb} was built from {base} and must reduce to it"
                    );
                }
            }
        }
        assert_eq!(base_verb_of(&dash_stance_verb(base)), base);
    }
    // An id built from no known suffix is its own base.
    assert_eq!(base_verb_of("taunt"), "taunt");
}

/// A broad rule means the character's own moves, and only this table knows
/// which those are.
#[test]
fn a_broad_cancel_rule_resolves_into_the_moves_it_admits() {
    use crate::MovesetContract;

    // `bare_move` is this file's own fixture: the smallest real `MoveSpec`.
    let move_named = |id: &str| bare_move(id, None);
    let contract = MovesetContract {
        verbs: [
            ("attack", "jab"),
            ("attack_forward", "ftilt"),
            ("smash_forward", "fsmash"),
            ("special", "shark"),
            ("grab", "grab_move"),
            ("grab_dash", "running_grab"),
            ("capture_throw_forward", "fthrow"),
            ("taunt", "taunt_move"),
            ("ranged", "shot"),
        ]
        .into_iter()
        .map(|(verb, mv)| (verb.to_string(), mv.to_string()))
        .collect(),
        moves: vec![
            move_named("jab"),
            move_named("ftilt"),
            move_named("fsmash"),
            move_named("shark"),
            move_named("grab_move"),
            move_named("running_grab"),
            move_named("fthrow"),
            move_named("taunt_move"),
            move_named("shot"),
            // Bound to no verb at all: reachable only by name.
            move_named("jab2"),
        ],
    };
    let ids = |into: &[&str]| {
        contract
            .cancel_targets(&into.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .iter()
            .map(|mv| mv.id.clone())
            .collect::<Vec<_>>()
    };

    // The attack family, which a smash belongs to.
    assert_eq!(ids(&["any_attack"]), vec!["jab", "ftilt", "fsmash"]);
    // A special is not in it: `cancel_names_for` gives a special its own
    // namespace, so `any_attack` must not include it.
    assert!(!ids(&["any_attack"]).contains(&"shark".to_string()));
    assert_eq!(ids(&["special"]), vec!["shark"]);
    // `smash` is narrower than `attack`.
    assert_eq!(ids(&["smash"]), vec!["fsmash"]);
    assert_eq!(ids(&["attack"]), vec!["jab", "ftilt", "fsmash"]);
    // `any_attack` does not include grabs, throws, taunts or ranged shots.
    // Each passes exactly one name on the trigger road, so otherwise a cancel
    // window could turn a swing into a throw.
    for outsider in ["grab_move", "fthrow", "taunt_move", "shot"] {
        assert!(
            !ids(&["any_attack", "attack", "smash"]).contains(&outsider.to_string()),
            "{outsider} is not in the attack family"
        );
    }
    // Each answers to its own full verb, not a reduced base. A running grab
    // answers to `grab`: `grab_dash` passes `[GRAB_VERB]` like the standing
    // one, so the base reduction is correct here and wrong for a capture verb.
    assert_eq!(ids(&["grab"]), vec!["grab_move", "running_grab"]);
    assert_eq!(ids(&["capture_throw_forward"]), vec!["fthrow"]);
    assert_eq!(ids(&["ranged"]), vec!["shot"]);
    // `capture_throw` is the base of the throw's verb and names nothing: the
    // capture road passes the full verb.
    assert!(ids(&["capture_throw"]).is_empty());
    // A literal move id admits exactly that move, bound or not.
    assert_eq!(ids(&["jab2"]), vec!["jab2"]);
    // And a list is a union.
    assert_eq!(ids(&["special", "jab2"]), vec!["shark", "jab2"]);
}

/// Renaming a move renames every reference to it, and nothing else. A missed
/// reference is a dead button in a match.
///
/// A verb class such as `"any_attack"` is not a move this table defines. A
/// rename that touched it would unhook every cancel window that names one.
#[test]
fn remapping_ids_follows_every_reference_and_leaves_verb_classes_alone() {
    let mut jab = bare_move("polygon_jab", None);
    jab.windows.push(MoveWindow {
        start_s: 0.0,
        end_s: 0.1,
        tag: WindowTag::Cancelable {
            into: vec!["polygon_tilt_up".to_string(), "any_attack".to_string()],
            condition: CancelCondition::default(),
        },
        volumes: vec![],
        sustain_effect: None,
        motion_scale: 1.0,
    });
    // The refusal fallback is a move id too.
    jab.gates.when_refused = Some("polygon_tilt_up".to_string());
    // And one the table does not own, which must stay unchanged, like
    // `any_attack`.
    //
    // The name contains "polygon" on purpose. The rename is
    // `polygon -> author`, so a foreign id without "polygon" would be
    // unchanged even if the code renamed blindly. Membership in `by_old` is
    // the real distinction, so the fixture must make the two behaviors differ.
    let mut stranger = bare_move("polygon_stranger", None);
    stranger.gates.when_refused = Some("polygon_ghost".to_string());
    let mut contract = MovesetContract {
        verbs: std::collections::BTreeMap::from([
            ("attack".to_string(), "polygon_jab".to_string()),
            ("attack_up".to_string(), "polygon_tilt_up".to_string()),
        ]),
        moves: vec![jab, bare_move("polygon_tilt_up", None), stranger],
    };

    contract.remap_move_ids(|id| id.replace("polygon", "director"));

    assert_eq!(
        contract
            .moves
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        vec!["director_jab", "director_tilt_up", "director_stranger"],
        "the moves themselves were not renamed"
    );
    assert_eq!(
        contract.verbs.get("attack_up").map(String::as_str),
        Some("director_tilt_up"),
        "a verb still resolves to a name no move answers to — one dead button"
    );
    let into = contract.moves[0]
        .windows
        .iter()
        .find_map(|w| match &w.tag {
            WindowTag::Cancelable { into, .. } => Some(into.clone()),
            _ => None,
        })
        .expect("the fixture authored a cancel window");
    assert_eq!(
        into,
        vec!["director_tilt_up".to_string(), "any_attack".to_string()],
        "a cancel target was left pointing at the old name, or the VERB CLASS \
         beside it was renamed — which unhooks every window that names one"
    );
    assert_eq!(
        contract.moves[0].gates.when_refused.as_deref(),
        Some("director_tilt_up"),
        "a refusal variant was left pointing at the OLD id. A cloned fighter's \
         priced move would fall through to the ORIGINAL fighter's move, or — \
         because an unknown id degrades to 'no fallback' by design — to nothing \
         at all, silently, which is the dead button the field exists to end"
    );
    assert_eq!(
        contract.moves[2].gates.when_refused.as_deref(),
        Some("polygon_ghost"),
        "an id this table does not own was renamed anyway, exactly as renaming \
         a verb class would be wrong"
    );
}

/// A windbox must not author damage.
///
/// Rejected, not zeroed: discarding a typed number turns a content error into
/// a mystery.
///
/// The zero-damage case matters too: a rule that refused every windbox would
/// pass the first assertion and make the mechanic unauthorable.
#[test]
fn a_windbox_that_authors_damage_is_rejected_and_a_zero_damage_one_is_not() {
    let catalog = |damage: i32| {
        let mut gust = bare_move("gust", None);
        gust.duration_s = 0.5;
        gust.windows = vec![MoveWindow {
            start_s: 0.1,
            end_s: 0.3,
            tag: WindowTag::Active,
            volumes: vec![HitVolume {
                hit_sfx: None,
                shape: VolumeShape::Circle {
                    offset: (0.0, 0.0),
                    radius: 12.0,
                },
                damage,
                knockback: 40.0,
                knockback_growth: None,
                launch_dir: None,
                on_hit: None,
                vfx: None,
                reaction: Some(VolumeReaction::Windbox(WindboxVolume { repeating: true })),
            }],
            sustain_effect: None,
            motion_scale: 1.0,
        }];
        EntityCatalogDoc {
            schema_version: 1,
            entities: vec![EntityDef {
                id: "gusty".to_string(),
                contracts: EntityContracts {
                    moveset: Some(MovesetContract {
                        verbs: BTreeMap::from([("special".to_string(), "gust".to_string())]),
                        moves: vec![gust],
                    }),
                    ..Default::default()
                },
            }],
        }
    };

    let errors = catalog(10).validate();
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, CatalogError::WindboxWithDamage { damage: 10, .. })),
        "a windbox authoring damage 10 was accepted; the contract says a gust \
         does nothing but push, and nothing was enforcing it: {errors:?}"
    );

    assert!(
        !catalog(0)
            .validate()
            .iter()
            .any(|e| matches!(e, CatalogError::WindboxWithDamage { .. })),
        "a windbox authoring NO damage was rejected, which makes the mechanic \
         unauthorable rather than validated"
    );
}

/// Every place a move can name a technique, and the walk that has to find all of
/// them.
mod effect_sites {
    use super::bare_move;
    use crate::{
        EffectRef, EffectSite, FlowNode, FlowSignal, HitVolume, MoveEvent, MoveEventKind, MoveSpec,
        MoveWindow, ParamValue, TechniqueFlow, VolumeShape, WindowTag,
    };

    fn effect(key: &str) -> EffectRef {
        EffectRef {
            key: key.to_string(),
            params: ParamValue::default(),
        }
    }

    fn volume(on_hit: Option<EffectRef>) -> HitVolume {
        HitVolume {
            shape: VolumeShape::Circle {
                offset: (0.0, 0.0),
                radius: 8.0,
            },
            damage: 1,
            knockback: 1.0,
            knockback_growth: None,
            launch_dir: None,
            reaction: None,
            on_hit,
            vfx: None,
            hit_sfx: None,
        }
    }

    /// A move that names a technique from all four sites, each with its own key
    /// so a walk that finds three cannot pass by finding one twice.
    fn move_naming_every_site() -> MoveSpec {
        let mut spec = bare_move("kitchen_sink", None);
        spec.windows = vec![MoveWindow {
            start_s: 0.0,
            end_s: 0.2,
            tag: WindowTag::Active,
            volumes: vec![volume(None), volume(Some(effect("site.on_hit")))],
            motion_scale: 1.0,
            sustain_effect: Some(effect("site.sustain")),
        }];
        spec.events = vec![
            MoveEvent {
                at_s: 0.05,
                kind: MoveEventKind::Ranged,
            },
            MoveEvent {
                at_s: 0.1,
                kind: MoveEventKind::Effect(effect("site.event")),
            },
        ];
        spec.flow = Some(TechniqueFlow {
            nodes: vec![
                FlowNode::Wait {
                    on: FlowSignal::Connected,
                    timeout_s: 0.2,
                    then: 1,
                    on_timeout: 2,
                },
                FlowNode::Emit {
                    effect: effect("site.flow"),
                    then: 2,
                },
                FlowNode::Finish,
            ],
        });
        spec
    }

    /// A move names techniques from four places, and the walk must find all
    /// of them.
    ///
    /// Each site carries its own key, so a walk that finds three of four
    /// cannot pass by finding one twice.
    #[test]
    fn the_walk_finds_a_technique_named_from_every_site() {
        let spec = move_naming_every_site();
        let found: Vec<(EffectSite, String)> = spec
            .effect_refs()
            .into_iter()
            .map(|(site, effect)| (site, effect.key.clone()))
            .collect();
        assert_eq!(
            found,
            vec![
                (
                    EffectSite::VolumeOnHit {
                        window: 0,
                        volume: 1
                    },
                    "site.on_hit".to_string()
                ),
                (
                    EffectSite::WindowSustain { window: 0 },
                    "site.sustain".to_string()
                ),
                (EffectSite::Event { event: 1 }, "site.event".to_string()),
                (EffectSite::FlowEmit { node: 1 }, "site.flow".to_string()),
            ],
            "the walk missed a site, or reported one at the wrong path"
        );
    }

    /// The path is what an author reads, so it names the index that identifies
    /// the site among its siblings — not just the field.
    #[test]
    fn a_site_prints_the_path_an_author_can_find() {
        let printed: Vec<String> = move_naming_every_site()
            .effect_refs()
            .into_iter()
            .map(|(site, _)| site.to_string())
            .collect();
        assert_eq!(
            printed,
            vec![
                "windows[0].volumes[1].on_hit",
                "windows[0].sustain_effect",
                "events[1].kind",
                "flow.nodes[1]",
            ]
        );
    }

    /// A move that names nothing reports nothing. Without this, a walk that
    /// invented a reference would still pass the rows above.
    #[test]
    fn a_move_with_no_technique_reports_none() {
        assert!(bare_move("plain", None).effect_refs().is_empty());
    }
}

/// What a composition actually installed, and what an authored reference is
/// allowed to name.
mod technique_support {
    use crate::{
        check_hydrates, EffectRef, EffectSite, NestedReferences, ParamValue, TechniqueDelivery,
        TechniqueOffer, TechniqueParams, TechniqueRefusal, TechniqueSupport,
    };

    #[derive(serde::Deserialize)]
    struct Offset {
        #[allow(dead_code)]
        offset: (f32, f32),
    }

    fn checked(owner: &'static str) -> TechniqueOffer {
        TechniqueOffer {
            owner,
            params: TechniqueParams::Checked(check_hydrates::<Offset>),
            references: NestedReferences::None,
            delivery: TechniqueDelivery::Action,
        }
    }

    fn paramless(owner: &'static str) -> TechniqueOffer {
        TechniqueOffer {
            owner,
            params: TechniqueParams::None,
            references: NestedReferences::None,
            delivery: TechniqueDelivery::Action,
        }
    }

    fn on_hit_only(owner: &'static str) -> TechniqueOffer {
        TechniqueOffer {
            owner,
            params: TechniqueParams::None,
            references: NestedReferences::None,
            delivery: TechniqueDelivery::OnHit,
        }
    }

    /// `pogo_bounce` is consumed from `OnHitEffectMessage` only. Authored at a
    /// timeline event, a window's sustain slot or a flow `Emit`, it becomes an
    /// `ActorActionMessage::Special`, which that handler never reads, so the
    /// move plays and the technique does nothing. Admission must refuse it.
    #[test]
    fn an_on_hit_technique_authored_in_a_timeline_slot_is_refused() {
        let mut support = TechniqueSupport::default();
        support
            .declare("pogo_bounce", on_hit_only("ambition_combat::on_hit"))
            .expect("first claim");
        let authored = effect("pogo_bounce", "()");

        for site in [
            EffectSite::WindowSustain { window: 0 },
            EffectSite::Event { event: 0 },
            EffectSite::FlowEmit { node: 0 },
        ] {
            let refusal = support.admit_at(Some(&site), &authored);
            assert!(
                matches!(refusal, Err(TechniqueRefusal::WrongSite { .. })),
                "{site:?} reaches only the action road, and an on-hit handler \
                 never reads it; got {refusal:?}"
            );
        }
    }

    /// Positive control: the site the handler does read must pass. A check
    /// that refused every site would pass the test above.
    #[test]
    fn the_site_an_on_hit_technique_does_read_is_admitted() {
        let mut support = TechniqueSupport::default();
        support
            .declare("pogo_bounce", on_hit_only("ambition_combat::on_hit"))
            .expect("first claim");

        assert_eq!(
            support.admit_at(
                Some(&EffectSite::VolumeOnHit {
                    window: 0,
                    volume: 0
                }),
                &effect("pogo_bounce", "()")
            ),
            Ok(())
        );
    }

    /// Control: an action-road technique is the mirror. The three action sites
    /// pass and the on-hit site is refused.
    #[test]
    fn an_action_technique_is_refused_at_an_on_hit_site() {
        let mut support = TechniqueSupport::default();
        support
            .declare("smash.sleep", paramless("ambition_demo_smash::sing"))
            .expect("first claim");
        let authored = effect("smash.sleep", "()");

        assert_eq!(
            support.admit_at(Some(&EffectSite::Event { event: 0 }), &authored),
            Ok(())
        );
        assert!(matches!(
            support.admit_at(
                Some(&EffectSite::VolumeOnHit {
                    window: 0,
                    volume: 0
                }),
                &authored
            ),
            Err(TechniqueRefusal::WrongSite { .. })
        ));
    }

    fn effect(key: &str, ron_text: &str) -> EffectRef {
        EffectRef {
            key: key.to_string(),
            params: ParamValue::parse(ron_text).expect("the fixture's params parse"),
        }
    }

    /// An unknown key is refused (for example `smash.teleprot`), so it does not
    /// reach the runtime as a move that plays and does nothing.
    #[test]
    fn a_key_nothing_installed_declares_is_refused() {
        let mut support = TechniqueSupport::default();
        support
            .declare("smash.teleport", checked("traversal::teleport"))
            .expect("a fresh support table accepts the first claim");
        assert_eq!(
            support.admit(&effect("smash.teleprot", "(offset: (1.0, 2.0))")),
            Err(TechniqueRefusal::Unknown {
                key: "smash.teleprot".to_string()
            }),
        );
        // Positive control: the correctly spelled key is still admitted, or the
        // row above passes for a table that refuses everything.
        assert_eq!(
            support.admit(&effect("smash.teleport", "(offset: (1.0, 2.0))")),
            Ok(())
        );
    }

    /// A second claim on one key is a conflict, not a replacement. A silent
    /// replacement would quietly disable a validator.
    ///
    /// Reported by key and claimed owner, not by comparing checks: a
    /// `ParamCheck` is a function pointer, and process-local values must not
    /// enter a registration's identity.
    #[test]
    fn two_capabilities_cannot_claim_one_technique() {
        let mut support = TechniqueSupport::default();
        support
            .declare("smash.teleport", checked("traversal::teleport"))
            .expect("the first claim is accepted");
        let conflict = support
            .declare("smash.teleport", checked("some_other::teleport"))
            .expect_err("a second claim on one key must be refused");
        assert_eq!(conflict.held_by, "traversal::teleport");
        assert_eq!(conflict.claimed_by, "some_other::teleport");
        // The first capability's contract survives the refused claim.
        assert_eq!(
            support.offer("smash.teleport").map(|offer| offer.owner),
            Some("traversal::teleport"),
            "the refused second claim replaced the first anyway"
        );
    }

    /// Paramless is a contract: parameters authored for a technique that takes
    /// none must be refused, not ignored.
    #[test]
    fn a_paramless_technique_refuses_authored_parameters() {
        let mut support = TechniqueSupport::default();
        support
            .declare("smash.counter", paramless("smash_counter"))
            .expect("accepted");
        assert_eq!(support.admit(&effect("smash.counter", "()")), Ok(()));
        assert!(matches!(
            support.admit(&effect("smash.counter", "(offset: (1.0, 2.0))")),
            Err(TechniqueRefusal::UnexpectedParams { .. })
        ));
    }

    /// A key that IS installed with params that do not hydrate is refused with
    /// the technique's own message, not a generic one.
    #[test]
    fn parameters_that_do_not_hydrate_are_refused_by_the_techniques_own_check() {
        let mut support = TechniqueSupport::default();
        support
            .declare("smash.teleport", checked("traversal::teleport"))
            .expect("accepted");
        match support.admit(&effect("smash.teleport", "(offset: \"not a pair\")")) {
            Err(TechniqueRefusal::BadParams { key, owner, detail }) => {
                assert_eq!(key, "smash.teleport");
                assert_eq!(owner, "traversal::teleport");
                assert!(
                    !detail.is_empty(),
                    "the refusal carried no sentence an author can act on"
                );
            }
            other => panic!("expected a parameter refusal, got {other:?}"),
        }
    }

    /// Every refusal at once, so an author fixes a move in one pass.
    #[test]
    fn a_batch_reports_every_refusal_rather_than_the_first() {
        let mut support = TechniqueSupport::default();
        support
            .declare("smash.teleport", checked("traversal::teleport"))
            .expect("accepted");
        let refs = vec![
            effect("smash.teleprot", "()"),
            effect("smash.teleport", "(offset: 3)"),
            effect("smash.teleport", "(offset: (1.0, 2.0))"),
        ];
        assert_eq!(
            support.admit_all(&refs).len(),
            2,
            "a batch reported {:?}",
            support.admit_all(&refs)
        );
    }
}

/// Flow validation, which exists because every one of these failures is silent.
mod technique_flow {
    use crate::{EffectRef, FlowNode, FlowSignal, MoveContact, ParamValue, TechniqueFlow};

    fn emit(then: u16) -> FlowNode {
        FlowNode::Emit {
            effect: EffectRef {
                key: "smash.whatever".to_string(),
                params: ParamValue::default(),
            },
            then,
        }
    }

    /// A transition past the end of the list is refused by name and index.
    ///
    /// At runtime the interpreter finds no node at the index, so the sequence
    /// silently stops.
    #[test]
    fn a_transition_past_the_end_is_refused() {
        let flow = TechniqueFlow {
            nodes: vec![emit(7), FlowNode::Finish],
        };
        let problems = flow.problems();
        assert!(
            problems.iter().any(|p| p.contains("past the last node")),
            "a transition to node 7 of a 2-node flow was accepted: {problems:?}"
        );
    }

    /// A flow with no reachable `Finish` is refused, even though it has one.
    ///
    /// Node 1 is a `Finish` nothing reaches, and node 0 loops to itself.
    /// Presence and reachability differ; reachability is the invariant.
    #[test]
    fn a_finish_nothing_reaches_is_not_a_finish() {
        let flow = TechniqueFlow {
            nodes: vec![emit(0), FlowNode::Finish],
        };
        let problems = flow.problems();
        assert!(
            problems
                .iter()
                .any(|p| p.contains("no `Finish` is reachable")),
            "a flow whose only Finish is unreachable was accepted, so the move \
             would stay under its control forever: {problems:?}"
        );
    }

    /// A reachable `Finish` behind a branch satisfies it — the check is not
    /// "node 0 is a Finish".
    #[test]
    fn a_finish_reached_through_a_branch_is_enough() {
        let flow = TechniqueFlow {
            nodes: vec![
                FlowNode::Branch {
                    on: FlowSignal::Connected,
                    then: 1,
                    otherwise: 1,
                },
                FlowNode::Finish,
            ],
        };
        assert_eq!(
            flow.problems(),
            Vec::<String>::new(),
            "a flow that reaches its Finish through a branch was reported broken"
        );
    }

    /// A wait that can never time out is refused.
    ///
    /// An authored "wait forever" is always a bug: every step after it never
    /// runs.
    #[test]
    fn a_wait_with_no_patience_is_refused() {
        let flow = TechniqueFlow {
            nodes: vec![
                FlowNode::Wait {
                    on: FlowSignal::Connected,
                    timeout_s: 0.0,
                    then: 1,
                    on_timeout: 1,
                },
                FlowNode::Finish,
            ],
        };
        let problems = flow.problems();
        assert!(
            problems.iter().any(|p| p.contains("never expires")),
            "a wait with a zero timeout was accepted: {problems:?}"
        );
    }

    /// A cycle on one branch and a `Finish` on the other is refused.
    ///
    /// `reaches_finish` is existential: here it finds `Finish` down the `then`
    /// road, while the `otherwise` road loops. The road taken depends on
    /// whether the strike connected, so the move would end or loop depending
    /// on the match.
    ///
    /// The per-tick node budget is not this check: it limits how fast a loop
    /// spins, not whether one exists.
    #[test]
    fn a_cycle_on_one_branch_is_refused_though_the_other_finishes() {
        let flow = TechniqueFlow {
            nodes: vec![
                FlowNode::Branch {
                    on: FlowSignal::Connected,
                    then: 1,
                    otherwise: 2,
                },
                FlowNode::Finish,
                // The looping road: emit, then back to the branch.
                emit(0),
            ],
        };
        // The premise. Without this, the test could pass because the Finish is
        // unreachable, which is a different defect with its own case.
        assert!(
            !flow
                .problems()
                .iter()
                .any(|p| p.contains("no `Finish` is reachable")),
            "this fixture is supposed to have a REACHABLE Finish; if it does not, \
             it is exercising the older check rather than the cycle one"
        );
        let problems = flow.problems();
        assert!(
            problems.iter().any(|p| p.contains("the flow loops")),
            "a flow that terminates on one branch and loops on the other was \
             accepted: {problems:?}"
        );
    }

    /// `f32::INFINITY > 0.0` is true, so a positive-only check admits an
    /// infinite timeout. NaN is also asserted so the pair stays together.
    #[test]
    fn a_wait_forever_spelled_as_a_number_is_refused() {
        let wait_for = |timeout_s: f32| {
            TechniqueFlow {
                nodes: vec![
                    FlowNode::Wait {
                        on: FlowSignal::Connected,
                        timeout_s,
                        then: 1,
                        on_timeout: 1,
                    },
                    FlowNode::Finish,
                ],
            }
            .problems()
        };
        for forever in [f32::INFINITY, f32::NAN] {
            let problems = wait_for(forever);
            assert!(
                problems.iter().any(|p| p.contains("never expires")),
                "a wait of {forever} was accepted, which is the unbounded wait the \
                 mandatory timeout exists to forbid: {problems:?}"
            );
        }
        // Positive control: an ordinary finite timeout is still admitted, or the
        // rows above pass for a check that refuses every wait.
        assert_eq!(
            wait_for(0.25),
            Vec::<String>::new(),
            "an ordinary quarter-second wait was reported broken"
        );
    }

    /// The bound is the version's stated budget. Edges are `u16`, the cursor's
    /// width, so a wrap cannot happen and this test checks the width only.
    ///
    /// Both sides of the boundary are tested, so the limit is known to be
    /// reachable.
    #[test]
    fn the_node_bound_admits_its_own_limit_and_refuses_one_more() {
        let chain = |count: u16| {
            let mut nodes: Vec<FlowNode> = (1..count).map(emit).collect();
            nodes.push(FlowNode::Finish);
            TechniqueFlow { nodes }
        };
        assert_eq!(
            chain(crate::MAX_TECHNIQUE_FLOW_NODES as u16).problems(),
            Vec::<String>::new(),
            "a flow of exactly the admitted width was refused"
        );
        let problems = chain(crate::MAX_TECHNIQUE_FLOW_NODES as u16 + 1).problems();
        assert!(
            problems
                .iter()
                .any(|p| p.contains("version 1 admits at most")),
            "a flow one node past the bound was accepted: {problems:?}"
        );
    }

    /// A node nothing arrives at is authored intent that never runs.
    #[test]
    fn a_node_nothing_arrives_at_is_refused() {
        let flow = TechniqueFlow {
            nodes: vec![
                emit(1),
                FlowNode::Finish,
                // Stranded: no transition names index 2.
                emit(1),
            ],
        };
        let problems = flow.problems();
        assert!(
            problems.iter().any(|p| p.contains("node 2 is unreachable")),
            "a stranded node was accepted, so whatever it was authored to do \
             silently never happens: {problems:?}"
        );
    }

    /// An empty flow is refused rather than treated as "no flow".
    #[test]
    fn an_empty_flow_is_refused() {
        let problems = TechniqueFlow { nodes: Vec::new() }.problems();
        assert!(
            problems.iter().any(|p| p.contains("no nodes")),
            "an empty flow was accepted: {problems:?}"
        );
    }

    /// `Overlapped` is not `Connected`, which is the distinction a blocked
    /// strike turns on.
    ///
    /// A flow author can get this wrong: `overlapped` is the staling fact and
    /// a shield sets it, so a hit-confirm written against it continues on a
    /// blocked strike.
    #[test]
    fn a_blocked_strike_overlaps_without_connecting() {
        let blocked = MoveContact {
            overlapped: true,
            connected: false,
            blocked: true,
        };
        assert!(FlowSignal::Overlapped.satisfied_by(blocked));
        assert!(FlowSignal::Blocked.satisfied_by(blocked));
        assert!(
            !FlowSignal::Connected.satisfied_by(blocked),
            "a blocked strike satisfied `Connected`, so every hit-confirm flow \
             would continue on a shielded hit"
        );
    }

    /// An authored edge that does not fit the runtime cursor is refused by the
    /// type, at the deserialization boundary, before any validator runs.
    ///
    /// Rust-literal fixtures cannot write this bug (it is a compile error), but
    /// a character loaded from data can. With a narrowing cast, `then: 65536`
    /// would pass the dangling-edge check and jump to node 0 at runtime. The
    /// edge is `u16`, so serde refuses the number and names the field.
    #[test]
    fn an_authored_edge_wider_than_the_cursor_is_refused_at_the_boundary() {
        // Round-tripped, not hand-written: a RON literal that failed to parse
        // for a spelling reason would pass the assertion below while testing
        // nothing.
        let authored = TechniqueFlow {
            nodes: vec![emit(1), FlowNode::Finish],
        };
        let in_width = ron::ser::to_string(&authored).expect("a flow serializes");
        assert!(
            in_width.contains("then:1"),
            "the fixture no longer spells the edge the way this test rewrites it: {in_width}"
        );
        let over_width = in_width.replace("then:1", "then:65536");
        let refused = ron::from_str::<TechniqueFlow>(&over_width);
        assert!(
            refused.is_err(),
            "an edge of 65536 was accepted; at runtime the cursor narrows it to \
             node 0 and the flow loops forever: {refused:?}"
        );
        // Positive control: without it, the row above passes for any document
        // this parser rejects, not only for the width.
        let admitted =
            ron::from_str::<TechniqueFlow>(&in_width).expect("the in-width control must parse");
        assert_eq!(
            admitted.problems(),
            Vec::<String>::new(),
            "the in-width control flow was reported broken"
        );
    }
}
