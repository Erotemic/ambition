//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

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
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.3,
        windows: vec![],
        events: vec![],
        gates: MoveGates { grounded },
        start_impulse: None,
        smash_charge_mult: 1.0,
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

#[test]
fn charge_scale_interpolates_by_fraction_and_is_parity_at_unit_mult() {
    let mut m = bare_move("charge", None);
    m.windows = vec![startup(0.4)];
    // Parity: default mult 1.0 -> always 1.0, whatever the fraction.
    assert_eq!(m.charge_scale_at(0.0), 1.0);
    assert_eq!(m.charge_scale_at(0.4), 1.0);
    // Opt in: fraction is elapsed / charge-window length, clamped.
    m.smash_charge_mult = 3.0;
    assert!((m.charge_fraction_at(0.0) - 0.0).abs() < 1e-6);
    assert!((m.charge_fraction_at(0.2) - 0.5).abs() < 1e-6);
    assert!((m.charge_fraction_at(0.4) - 1.0).abs() < 1e-6);
    assert!(
        (m.charge_fraction_at(0.9) - 1.0).abs() < 1e-6,
        "clamps at full"
    );
    // scale = 1 + frac*(mult-1): 0 -> 1, 0.5 -> 2, 1 -> 3.
    assert!((m.charge_scale_at(0.0) - 1.0).abs() < 1e-6);
    assert!((m.charge_scale_at(0.2) - 2.0).abs() < 1e-6);
    assert!((m.charge_scale_at(0.4) - 3.0).abs() < 1e-6);
}

#[test]
fn no_startup_window_is_fully_charged_instantly() {
    let mut m = bare_move("jab", None);
    m.smash_charge_mult = 2.0; // no Startup window -> full charge at once
    assert_eq!(m.charge_fraction_at(0.0), 1.0);
    assert_eq!(m.charge_scale_at(0.0), 2.0);
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
                    hit_sfx: None,
                    shape: VolumeShape::Rect {
                        offset: (28.0, 0.0),
                        half_extents: (16.0, 12.0),
                    },
                    damage: 4,
                    knockback: 100.0,
                    kb_growth: 0.0,
                    launch_dir: None,
                    on_hit: None,
                    vfx: None,
                },
                HitVolume {
                    hit_sfx: None,
                    shape: VolumeShape::Circle {
                        offset: (30.0, 0.0),
                        radius: 20.0,
                    },
                    damage: 2,
                    knockback: 40.0,
                    kb_growth: 0.0,
                    launch_dir: None,
                    on_hit: None,
                    vfx: None,
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
        directional_verb_chain("attack", AttackDir::Neutral, true),
        vec!["attack"],
    );
    assert_eq!(
        directional_verb_chain("attack", AttackDir::Neutral, false),
        vec!["attack_air", "attack"],
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
