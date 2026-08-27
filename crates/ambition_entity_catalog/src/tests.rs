use super::*;

#[derive(serde::Deserialize)]
struct GliderParams {
    #[allow(dead_code)]
    rise: f32,
}

#[test]
fn param_schema_registry_catches_typos_at_validate_time() {
    // AJ1 / A1: a technique registers a hydrate check; the content pass
    // runs every authored EffectRef through it. A good ref passes; a
    // missing/mistyped field fails at validate time, not mid-fight.
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
    // CM3 smash class = MORE VERBS (AJ1): a moveset binds `smash_up` distinct
    // from the tilt `attack_up`, resolved by the SAME generic verb map. The
    // input side (flick vs. hold) picks the base verb per game.
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

/// The full R2 ability vocabulary, authored entirely as RON: directional
/// verbs, a move-start `start_impulse` lunge, and an `on_hit` pogo volume.
/// The I7 acceptance — a fighter's whole kit is DATA, not code.
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

/// THE DASH ATTACK IS A STANCE, AND IT OUTRANKS THE DIRECTION.
///
///  four cases, and each kills a different wrong version: a dashing body gets
/// its dash attack even with a direction held (or the tilt would keep winning),
/// a STANDING body never does (or every forward tilt is now a dash attack), an
/// AIRBORNE dashing body never does (a dash is a ground stance), and a fighter
/// that authors none resolves exactly what it did before — which is the property
/// that lets this ship without touching thirteen non-smash movesets.
/// **The RUNNING GRAB — the capture kit's half of the dash attack.**
///
/// ⛔ four cases, each killing a different wrong version: a running grounded
/// body reaches with its running grab, a STANDING body never does (or every
/// grab became the committed one), an AIRBORNE running body never does (a run
/// is a ground stance, and the capture kit is GROUNDED for v1), and a contract
/// without the variant resolves its press to the plain grab byte for byte —
/// which is what lets this ship without touching a single moveset.
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
    // ⛔ NOTHING, not the standing grab. Both variants here are `grounded: Some(true)`
    // -- the gate the capture kit authors on its whole vocabulary, because an
    // aerial grab is a named FUTURE technique. This assertion used to read
    // `"grab"`, which said out loud that an airborne press starts a grounded-only
    // move; the selector agreed with it, so the pair was a bug and its receipt.
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

    // ⭐ and the gate is what refuses it, not the verb: an UNGATED standing grab
    // still answers an airborne press. Without this the assertions above would
    // also pass if the lookup had simply stopped resolving `base`.
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

    // ⛔ **the WORD is spelled once.** `GRAB_DASH_VERB` exists only because the
    // binding table needs a `&'static str`; if it ever drifts from the suffix
    // rule, the selector asks for a verb the vocabulary does not declare.
    assert_eq!(
        super::GRAB_DASH_VERB,
        super::dash_stance_verb(super::GRAB_VERB)
    );

    // ⛔ the floor: a contract with no running grab is untouched.
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

    //  the WORD is spelled once. The selector builds this verb through
    // `dash_stance_verb` and so does the runtime's vocabulary; a table keyed by
    // a hand-typed `"attack_dash"` here would keep passing after a rename that
    // left the runtime unable to resolve the move.
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

/// The relativity contract, pinned as behavior: the timeline is queried
/// in the OWNER'S proper time, so a dilated actor advancing at 0.25×
/// world rate reaches its active window after 4× the world time — by
/// construction, because the caller integrates proper time from the
/// owner's dt. The schema carries no world-time anywhere.
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

/// A ZERO-WIDTH window is legal; an INVERTED one is not.
///
///  this is only safe because every window predicate is the half-open
/// `start_s <= t < end_s` (`moveset/mod.rs`), so nothing can fire inside a
/// zero-width window — it is a label on a boundary, not a span. If a predicate
/// ever becomes inclusive at the end, this stops being free and the validator
/// should tighten again.
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
// Timed authored self-displacement (`MoveEventKind::Impulse`) and the LIFT
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
    }
}

/// A move that SETS an against-gravity speed advertises it; one that only ADDS
/// to the body's own does not.
///
///  the distinction is the whole reason [`ImpulseMode`] has two variants, and
/// pinning it here is what stops a lunging jab from reading as a recovery
/// special to every consumer downstream. An `Add` produces a speed only in
/// company with whatever the body was already doing, so no static reader can
/// name one — and a reader that pretended otherwise would price a jab's 120px/s
/// hop as a way home.
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

/// A DOWNWARD commanded impulse is not lift. A dive is the same primitive
/// pointed the other way, and a consumer looking for a way home must not find
/// one in it — the sign is the only thing separating the two, so it gets its own
/// guard rather than riding on the test above.
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

/// The strongest lift wins, and a tie breaks on the EARLIER moment. Two
/// bursts on one timeline is a legal thing to author (a hop into a rise), and
/// which one a policy plans around must not depend on declaration order.
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

/// A COMMANDED VELOCITY IS A VECTOR, AND BOTH HALVES COME FROM THE SAME
/// EVENT.
///
/// Every downstream reader then planned around a move nobody wrote.
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

    //  poison: the side is read off the WINNING event, never off whichever
    // impulse happens to be first. Here the strong rise carries no side and the
    // weak one carries a huge one; a derivation that mixed them would report a
    // move that does not exist.
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

    // A move that commands its owner BACKWARDS says so with a sign rather than
    // by being invisible — the recoil of firing forwards is a real authored
    // shape and it must not read as a way home in the wrong direction.
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

/// An `Impulse` event round-trips through RON with `mode` omitted, so an
/// authored timeline that says only `Impulse(local: (0, -900))` parses as the
/// additive meaning `start_impulse` always had rather than failing to load.
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

/// The charge policy every shipped fighter relies on is DERIVED, not authored:
/// the hold sits at the end of the move's own leading Startup window.
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
    };
    let derived = spec.charge_policy().expect("a paying smash charges");
    // ⭐ THE HOLD SITS WHERE THE WINDUP BEGINS, and this assertion read `0.3` —
    // the Startup window's END — until 2026-08-23. That froze the body at the
    // instant the strike was about to come out, with the whole windup already
    // played and the hitbox one frame away, which reads as a fighter paused
    // mid-swing rather than one winding up.
    //
    // Jon: *"it needs to hold on the first frames of the smash animation, before
    // letting the rest of the animation, which actually has the hitboxes,
    // play."* That is the genre's charge pose. Everything after this instant —
    // the rest of the windup and every Active window — plays on release.
    assert_eq!(
        derived.hold_at_s,
        0.3 * CHARGE_POSE_FRACTION,
        "the charge pose left the windup"
    );
    // ⛔⛔ AND THE INVARIANT, not just the number: a held charge must not be
    // able to stand inside a live strike. Active membership is
    // `start_s <= t < end_s`, so a hold AT the first Active instant is already
    // inside it — which is exactly what deriving from the windup's `end_s` did.
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
    });
    assert_eq!(spec.charge_policy().unwrap().hold_at_s, 0.12);

    // ... including all the way to "this smash does not hold".
    spec.smash_charge = Some(SmashChargeSpec {
        hold_at_s: 0.12,
        max_hold_s: 0.0,
        stores: false,
    });
    assert!(
        spec.charge_policy().is_none(),
        "a zero maximum is how a move says it cannot be charged"
    );
}

/// The frame data a brain reads carries the move's own charge point, and says
/// `None` for a move that does not charge at all.
///
/// ⛔ the fallback a reader would otherwise use is `startup_s`, which is when
/// the first HIT lands. Deriving "when does the charge begin" from that is only
/// right by coincidence.
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

/// ⛔⛔ AUTHORING MAY NOT PUT A HITBOX INSIDE A HELD CHARGE.
///
/// The derived hold point is clamped strictly before the first Active instant,
/// but an authored `smash_charge` overrides that clamp — so the override is
/// where a malformed pose can still get in. `rooted_by_charge` is true from the
/// freeze onward and the button may hold it indefinitely, so a hold at or past
/// the first live volume is a fighter standing still with a strike out.
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

    // ON the first Active instant is already INSIDE it: membership is
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

/// ⭐⭐ RENAMING A MOVE RENAMES EVERY REFERENCE TO IT, AND NOTHING ELSE.
///
/// A move id lives in THREE places inside a contract, and this walk used to be
/// written in a CONTENT crate — so every future id-bearing field on a `MoveSpec`
/// was an obligation on a file that would never hear about it. Missing one is
/// not a red test: it is one dead button in a match.
///
/// ⛔ THE VERB-CLASS ARM IS THE ONE THAT CONSTRAINS. `"any_attack"` is a cancel
/// CLASS, not a move this table defines, and a rename that touched it would
/// silently unhook every cancel window that names one.
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
    let mut contract = MovesetContract {
        verbs: std::collections::BTreeMap::from([
            ("attack".to_string(), "polygon_jab".to_string()),
            ("attack_up".to_string(), "polygon_tilt_up".to_string()),
        ]),
        moves: vec![jab, bare_move("polygon_tilt_up", None)],
    };

    contract.remap_move_ids(|id| id.replace("polygon", "author"));

    assert_eq!(
        contract
            .moves
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        vec!["author_jab", "author_tilt_up"],
        "the moves themselves were not renamed"
    );
    assert_eq!(
        contract.verbs.get("attack_up").map(String::as_str),
        Some("author_tilt_up"),
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
        vec!["author_tilt_up".to_string(), "any_attack".to_string()],
        "a cancel target was left pointing at the old name, or the VERB CLASS \
         beside it was renamed — which unhooks every window that names one"
    );
}

/// ⛔⛔ A WINDBOX MAY NOT AUTHOR DAMAGE, AND THE TYPE USED TO LET IT.
///
/// `VolumeReaction::Windbox` promises "pushes its victim and does nothing else
/// — no damage, no hitstun, no shield". The runtime honours the hitstun and the
/// shield; `damage` was published exactly as an ordinary hit's is, so the
/// contract lived in a doc comment and in every fixture's good manners.
///
/// ⭐ REJECTED RATHER THAN ZEROED, because throwing away a number somebody
/// deliberately typed is how a content error turns into a mystery about why a
/// move does nothing.
///
/// ⭐⭐ THE ZERO-DAMAGE ARM IS THE POINT OF THE SECOND HALF. A rule that
/// refused every windbox would satisfy the first assertion perfectly while
/// making the mechanic unauthorable, which is the shape a validation ships
/// broken in.
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
