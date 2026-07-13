//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::events::HitEvent;
use crate::hitbox::apply_hitbox_damage;
use ambition_sfx::SfxMessage;
use ambition_vfx::vfx::DebrisBurstMessage;
use ambition_vfx::vfx::VfxMessage;
use bevy::prelude::*;

#[test]
fn prefab_registry_expands_sword_slash_from_simple_melee_with_zero_new_code() {
    // A2 / R2.3: `sword_slash` is the `simple_melee` prefab + params, minted
    // by name at roster install — no bespoke builder.
    let reg = MovePrefabRegistry::with_engine_prefabs();
    assert!(!reg.is_empty());
    let params = ambition_entity_catalog::ParamValue::parse(
        "(windup_s: 0.2, active_s: 0.08, recover_s: 0.3, damage: 4, reach_px: 60.0)",
    )
    .unwrap();
    let sword = reg
        .expand("simple_melee", &params, "sword_slash")
        .expect("simple_melee expands");
    assert_eq!(
        sword.id, "sword_slash",
        "expand renames to the roster move id"
    );
    // The authored damage/reach flowed into the Active window's hit volume.
    let active = sword
        .windows
        .iter()
        .find(|w| matches!(w.tag, WindowTag::Active))
        .expect("charge has an Active window");
    assert_eq!(active.volumes.len(), 1);
    assert_eq!(active.volumes[0].damage, 4);
    assert!((sword.duration_s - 0.58).abs() < 1e-5, "0.2+0.08+0.3");
}

#[test]
fn prefab_registry_rejects_unknown_key_and_bad_params() {
    let reg = MovePrefabRegistry::with_engine_prefabs();
    let empty = ambition_entity_catalog::ParamValue::default();
    assert!(
        reg.expand("not_a_prefab", &empty, "x").is_err(),
        "typo'd key"
    );
    // Wrong type for a field fails at expand (install) time.
    let bad = ambition_entity_catalog::ParamValue::parse("(damage: \"lots\")").unwrap();
    assert!(reg.expand("simple_melee", &bad, "x").is_err(), "bad params");
    // Empty params hydrate to the prefab defaults (every field defaults).
    assert!(reg.expand("simple_charge", &empty, "smash").is_ok());
}

/// CM5: a prefab row authors its OWN swing sfx + a cosmetic burst, so the
/// move sounds and looks distinct with zero code. Parity when omitted.
#[test]
fn per_move_presentation_is_authored_on_the_prefab_row() {
    let reg = MovePrefabRegistry::with_engine_prefabs();

    // Default row: the engine-default swing cue, no cosmetic burst (parity).
    let default = reg
        .expand(
            "simple_melee",
            &ambition_entity_catalog::ParamValue::default(),
            "jab",
        )
        .unwrap();
    let sfx_cues: Vec<&str> = default
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            MoveEventKind::Sfx { cue } => Some(cue.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(sfx_cues, vec![SWING_SFX_CUE], "default swing cue");
    assert!(
        !default
            .events
            .iter()
            .any(|e| matches!(e.kind, MoveEventKind::Vfx { .. })),
        "an unauthored row emits no cosmetic burst (parity)"
    );

    // Authored row: a heavy smash with its own thud + a shockwave burst.
    let smash = reg
        .expand(
            "simple_melee",
            &ambition_entity_catalog::ParamValue::parse(
                "(swing_sfx: Some(\"boss.slam\"), swing_vfx: Some(\"shockwave\"))",
            )
            .unwrap(),
            "smash",
        )
        .expect("authored presentation expands");
    assert!(
        smash.events.iter().any(|e| matches!(
            &e.kind,
            MoveEventKind::Sfx { cue } if cue == "boss.slam"
        )),
        "the authored cue replaced the default"
    );
    assert!(
        smash.events.iter().any(|e| matches!(
            &e.kind,
            MoveEventKind::Vfx { effect } if effect == "shockwave"
        )),
        "the authored cosmetic burst rides the timeline"
    );
}

/// CM5: a typo'd cosmetic vfx id fails at expand (startup validation), the
/// same gate a bad prefab key hits — never a silent missing effect.
#[test]
fn a_typod_cosmetic_vfx_id_is_rejected_at_expansion() {
    let reg = MovePrefabRegistry::with_engine_prefabs();
    let bad = ambition_entity_catalog::ParamValue::parse("(swing_vfx: Some(\"kaboom\"))").unwrap();
    let err = reg
        .expand("simple_melee", &bad, "x")
        .expect_err("an unknown cosmetic id must fail validation");
    assert!(
        err.contains("kaboom") && err.contains("unknown cosmetic effect"),
        "the error names the offending id: {err}"
    );
}

/// CM5: the content-free dispatcher turns a `Vfx` event into an explosion
/// burst at the owner's position.
#[test]
fn move_event_dispatch_bridges_vfx_to_a_cosmetic_burst() {
    use ambition_vfx::VfxMessage;
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct Seen(Option<ambition_vfx::ExplosionKind>);

    fn capture(mut vfx: MessageReader<VfxMessage>, mut seen: ResMut<Seen>) {
        for m in vfx.read() {
            if let VfxMessage::Explosion { kind, .. } = m {
                seen.0 = Some(*kind);
            }
        }
    }

    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<ActorActionMessage>();
    app.init_resource::<Seen>();
    let owner = app
        .world_mut()
        .spawn(ae::BodyKinematics {
            pos: ae::Vec2::new(10.0, 20.0),
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(16.0, 24.0),
            facing: 1.0,
        })
        .id();
    app.add_systems(Update, (dispatch_move_events, capture).chain());
    app.world_mut()
        .resource_mut::<Messages<MoveEventMessage>>()
        .write(MoveEventMessage {
            owner,
            move_id: "smash".into(),
            kind: MoveEventKind::Vfx {
                effect: "starburst".to_string(),
            },
        });
    app.update();
    assert_eq!(
        app.world().resource::<Seen>().0,
        Some(ambition_vfx::ExplosionKind::Starburst),
        "the Vfx event resolved to a Starburst explosion burst",
    );
}

#[test]
fn authored_melee_adapter_matches_the_simple_melee_prefab() {
    // The MeleeActionSpec path and the prefab produce the same move for the
    // same timeline — the adapter is byte-identical to the generalized core.
    use ambition_characters::brain::action_set::SwipeSpec;
    let spec = MeleeActionSpec::Swipe(SwipeSpec {
        windup_s: 0.15,
        active_s: 0.1,
        recover_s: 0.18,
        damage: 2,
        reach_px: 40.0,
    });
    let via_adapter = attack_move_from_melee(&spec);
    let via_prefab = simple_melee(&SimpleMeleeParams {
        windup_s: 0.15,
        active_s: 0.1,
        recover_s: 0.18,
        damage: 2,
        reach_px: 40.0,
        knockback: 120.0,
        ..Default::default()
    });
    assert_eq!(via_adapter, via_prefab);
}

/// The seed move: SwipeSpec-as-data (0.28 windup / 0.08 active with one
/// forward rect volume / recovery), one timed Sfx event on the swing.
fn swat() -> MoveSpec {
    let doc = ambition_entity_catalog::EntityCatalogDoc::parse(
        r#"(
            schema_version: 1,
            entities: [(
                id: "seed",
                contracts: (moveset: Some((
                    verbs: {"attack": "swat"},
                    moves: [(
                        id: "swat",
                        clip: (clip: "slash", fallbacks: ["idle"]),
                        duration_s: 0.68,
                        windows: [
                            (start_s: 0.0, end_s: 0.28, tag: Startup, volumes: []),
                            (start_s: 0.28, end_s: 0.36, tag: Active, volumes: [
                                (shape: Rect(offset: (28.0, 0.0), half_extents: (16.0, 12.0)),
                                 damage: 2, knockback: 40.0),
                            ]),
                            (start_s: 0.36, end_s: 0.68, tag: Recovery, volumes: []),
                        ],
                        events: [(at_s: 0.28, kind: Sfx(cue: "swing_light"))],
                    )],
                ))),
            )],
        )"#,
    )
    .unwrap();
    assert!(doc.validate().is_empty());
    doc.entity("seed")
        .unwrap()
        .contracts
        .moveset
        .as_ref()
        .unwrap()
        .move_for_verb("attack")
        .unwrap()
        .clone()
}

/// The same seed move as a full repertoire, reachable by the `"special"` AND
/// `"attack"` verbs — the shape a body carries in an `ActorMoveset`.
fn swat_moveset() -> MovesetContract {
    MovesetContract {
        verbs: [
            ("special".to_string(), "swat".to_string()),
            ("attack".to_string(), "swat".to_string()),
        ]
        .into_iter()
        .collect(),
        moves: vec![swat()],
    }
}

#[derive(Resource, Default)]
struct Captured {
    hits: Vec<HitEvent>,
    events: Vec<MoveEventMessage>,
    slashes: Vec<VfxMessage>,
}

fn capture(
    mut cap: ResMut<Captured>,
    mut hits: MessageReader<HitEvent>,
    mut evs: MessageReader<MoveEventMessage>,
    mut vfx: MessageReader<VfxMessage>,
) {
    cap.hits.extend(hits.read().cloned());
    cap.events.extend(evs.read().cloned());
    cap.slashes.extend(vfx.read().cloned());
}

/// Headless sim harness: move playback + the REAL hitbox damage path,
/// fixed 16ms sim ticks, a vulnerable player standing in reach.
/// Fixture seam resolver: a fixed convex blade for the `attack_side`
/// clip (what the player manifest authors), `None` for everything else.
fn test_blade_resolver(
    _catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    _cid: Option<&str>,
    animation: &str,
    body_pos: ae::Vec2,
    collision: ae::Vec2,
    _facing: f32,
    _gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    (animation == "attack_side").then(|| {
        let hx = collision.x * 0.8;
        let hy = collision.y * 0.4;
        ae::CombatVolume::convex(vec![
            body_pos + ae::Vec2::new(-hx, -hy),
            body_pos + ae::Vec2::new(hx, -hy),
            body_pos + ae::Vec2::new(hx * 1.4, 0.0),
            body_pos + ae::Vec2::new(hx, hy),
            body_pos + ae::Vec2::new(-hx, hy),
        ])
    })
}

fn app_with_victim() -> (App, Entity) {
    // The authored-blade path resolves through the install seam exactly
    // like production. Tests insert a FIXTURE resolver (a fixed convex
    // blade for the `attack_side` clip) — the seam + convex plumbing is
    // what combat owns; the REAL sprite-data resolution is asserted
    // sprites-side (`character_sprites::attack_hitbox` tests).
    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.insert_resource(
        super::super::authored_volumes::AuthoredAttackVolumeResolver::new(test_blade_resolver),
    );
    app.add_message::<HitEvent>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.init_resource::<Captured>();
    app.init_resource::<WorldTime>();
    app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
    app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
    app.add_systems(
        Update,
        (
            advance_move_playback,
            apply_hitbox_damage,
            // CM4: the connect fact for OnHit/OnWhiff cancels, in its
            // production position (right after damage resolution).
            mark_move_playback_landed_hits,
            capture,
        )
            .chain(),
    );
    let victim = app
        .world_mut()
        .spawn((
            ambition_platformer_primitives::markers::PlayerEntity,
            ActorFaction::Player,
            ambition_engine_core::BodyKinematics {
                pos: ae::Vec2::new(128.0, 100.0),
                size: ae::Vec2::new(28.0, 46.0),
                facing: -1.0,
                ..Default::default()
            },
            // The published combat footprint every body carries (§A6).
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(128.0, 100.0),
                ae::Vec2::new(28.0, 46.0),
            ),
            ambition_engine_core::BodyOffense::default(),
            ambition_engine_core::BodyMotionFacts::default(),
            ambition_engine_core::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
        ))
        .id();
    (app, victim)
}

fn spawn_attacker(app: &mut App, pos: ae::Vec2, body: ae::Vec2, spec: MoveSpec) -> Entity {
    app.world_mut()
        .spawn((
            ae::CenteredAabb::new(pos, body),
            // The playback system resolves the owner's gravity frame from
            // its authoritative kinematics, like every real actor carries.
            ae::BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size: body,
                facing: 1.0,
            },
            ActorFaction::Enemy,
            MovePlayback::new(spec, 1.0),
        ))
        .id()
}

fn run_seconds(app: &mut App, seconds: f32) {
    let steps = (seconds / 0.016).ceil() as usize;
    for _ in 0..steps {
        app.update();
    }
}

/// §7.1 + §7.2 (the bespoke-path parity restored onto the moveset):
/// a bladed (`vfx`-tagged) swing whose clip has an AUTHORED manifest
/// hitbox swings THAT blade — the live hitbox carries the sprite's convex
/// hull, not `simple_melee`'s synthetic rect — and the slash VFX is drawn
/// from the SAME resolved volume, exactly once, at the Active edge.
#[test]
fn bladed_swing_resolves_the_authored_blade_and_draws_its_slash() {
    let (mut app, _victim) = app_with_victim();
    // No `ActorConfig` → the player manifest root; `simple_melee`'s clip
    // is `attack_side`, the authored blade row (a convex poly).
    spawn_attacker(
        &mut app,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(30.0, 48.0),
        simple_melee(&SimpleMeleeParams::default()),
    );
    // Cross the 0.12s windup into the active window.
    run_seconds(&mut app, 0.14);
    let shapes: Vec<Option<ae::VolumeShape>> = {
        let mut q = app.world_mut().query::<&Hitbox>();
        q.iter(app.world()).map(|h| h.shape.clone()).collect()
    };
    assert_eq!(shapes.len(), 1, "the active window's volume is live");
    assert!(
        matches!(shapes[0], Some(ae::VolumeShape::Convex { .. })),
        "the swing carries the AUTHORED convex blade, got {:?}",
        shapes[0],
    );
    let cap = app.world().resource::<Captured>();
    let slashes: Vec<_> = cap
        .slashes
        .iter()
        .filter(|m| matches!(m, VfxMessage::Slash { .. }))
        .collect();
    assert_eq!(slashes.len(), 1, "one slash VFX at the Active edge");
    if let VfxMessage::Slash { kind, dir, .. } = slashes[0] {
        assert_eq!(*kind, ambition_vfx::vfx::SlashKind::Arc);
        assert!(
            dir.x > 0.0,
            "the slash points along the strike (facing +x), got {dir:?}",
        );
    }
}

/// §7.1 fallback: a bladed swing whose clip authors NO manifest row keeps
/// the synthetic rect (payload intact — the hit still lands) and still
/// draws its slash. Nothing regresses for unmanifested characters.
#[test]
fn unauthored_clip_falls_back_to_the_synthetic_rect_and_still_slashes() {
    let (mut app, _victim) = app_with_victim();
    let mut spec = simple_melee(&SimpleMeleeParams {
        reach_px: 60.0,
        ..Default::default()
    });
    // A clip no sprite ever authors → manifest miss. (Even `attack_up`
    // resolves a real upward hull now, so use a nonsense row.)
    spec.clip.clip = "no_such_authored_row".to_string();
    spec.clip.fallbacks.clear();
    spawn_attacker(
        &mut app,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(30.0, 48.0),
        spec,
    );
    run_seconds(&mut app, 0.14);
    let shapes: Vec<Option<ae::VolumeShape>> = {
        let mut q = app.world_mut().query::<&Hitbox>();
        q.iter(app.world()).map(|h| h.shape.clone()).collect()
    };
    assert_eq!(shapes.len(), 1);
    assert!(
        shapes[0].is_none(),
        "manifest miss → the synthetic rect path (shape None), got {:?}",
        shapes[0],
    );
    let cap = app.world().resource::<Captured>();
    assert_eq!(
        cap.slashes
            .iter()
            .filter(|m| matches!(m, VfxMessage::Slash { .. }))
            .count(),
        1,
        "the fallback swing still draws its slash",
    );
    assert_eq!(cap.hits.len(), 1, "the fallback rect still lands its hit");
}

/// W9 core: the authored timeline drives the REAL damage path. No hit
/// during startup; the active window spawns the volume and the standing
/// victim takes the authored damage; the window's exit despawns the box;
/// move completion removes the component. The timed event fires once.
#[test]
fn data_driven_move_lands_a_hit_through_the_real_path() {
    let (mut app, _victim) = app_with_victim();
    let attacker = spawn_attacker(
        &mut app,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(15.0, 24.0),
        swat(),
    );

    // Startup: nothing live, nothing hit, no event yet.
    run_seconds(&mut app, 0.20);
    {
        let cap = app.world().resource::<Captured>();
        assert!(cap.hits.is_empty(), "no hit during startup");
        assert!(cap.events.is_empty(), "no event during startup");
    }
    assert_eq!(count_hitboxes(&mut app), 0);

    // Cross into the active window: volume live, hit lands, event fired.
    run_seconds(&mut app, 0.12);
    assert_eq!(count_hitboxes(&mut app), 1, "active window volume is live");
    {
        let cap = app.world().resource::<Captured>();
        assert_eq!(cap.hits.len(), 1, "the swat landed exactly once");
        assert_eq!(cap.events.len(), 1, "swing event fired exactly once");
        assert!(matches!(
            &cap.events[0].kind,
            MoveEventKind::Sfx { cue } if cue == "swing_light"
        ));
    }

    // Past the window: box despawned. Past the move: component removed.
    run_seconds(&mut app, 0.1);
    assert_eq!(count_hitboxes(&mut app), 0, "window exit despawns the box");
    run_seconds(&mut app, 0.3);
    assert!(
        app.world().get::<MovePlayback>(attacker).is_none(),
        "finished move retires its playback"
    );
    let cap = app.world().resource::<Captured>();
    assert_eq!(cap.hits.len(), 1, "no double hit across the whole move");
}

/// B1 (fable review §B1): a moveset volume's authored offset is BODY-LOCAL
/// (side, down); the spawned `FollowOwner` hitbox must rotate it into the
/// owner's gravity frame at spawn, so the SAME move lands its box in the same
/// BODY-relative place under every gravity. Regression guard for the old
/// screen-frame spawn: an unrotated offset put an above-the-head strike into
/// the effective ceiling under sideways/inverted gravity, forking against the
/// gravity-aware player melee path.
#[test]
fn moveset_hitboxes_spawn_in_the_owner_gravity_frame() {
    // Authored body-local rect: forward (side +28) AND above the head
    // (down −20), non-square half so a 90° rotation is observable.
    fn overhead_swat() -> MoveSpec {
        let doc = ambition_entity_catalog::EntityCatalogDoc::parse(
            r#"(
                schema_version: 1,
                entities: [(
                    id: "seed",
                    contracts: (moveset: Some((
                        verbs: {"attack": "overhead"},
                        moves: [(
                            id: "overhead",
                            clip: (clip: "slash", fallbacks: ["idle"]),
                            duration_s: 0.68,
                            windows: [
                                (start_s: 0.0, end_s: 0.28, tag: Startup, volumes: []),
                                (start_s: 0.28, end_s: 0.36, tag: Active, volumes: [
                                    (shape: Rect(offset: (28.0, -20.0), half_extents: (16.0, 12.0)),
                                     damage: 2, knockback: 40.0),
                                ]),
                                (start_s: 0.36, end_s: 0.68, tag: Recovery, volumes: []),
                            ],
                            events: [],
                        )],
                    ))),
                )],
            )"#,
        )
        .unwrap();
        doc.entity("seed")
            .unwrap()
            .contracts
            .moveset
            .as_ref()
            .unwrap()
            .move_for_verb("attack")
            .unwrap()
            .clone()
    }

    // Spawn under `gravity` (facing +1), run into the 0.28–0.36 active window,
    // and read the live `FollowOwner` hitbox's world-frame offset + half.
    fn spawn_and_read(gravity: ae::Vec2) -> (ae::Vec2, ae::Vec2) {
        let (mut app, _victim) = app_with_victim();
        let attacker = spawn_attacker(
            &mut app,
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(15.0, 24.0),
            overhead_swat(),
        );
        // The owner's per-tick resolved frame is the rotation authority now —
        // publish the test gravity on the BODY, as the resolution phase would.
        let mut frame = ambition_platformer_primitives::frame_env::ResolvedMotionFrame::default();
        frame.publish(ae::MotionFrame::from_direction(gravity, 900.0));
        app.world_mut().entity_mut(attacker).insert(frame);
        run_seconds(&mut app, 0.31); // t ≈ 0.32, inside the active window
        let mut state = app.world_mut().query::<&Hitbox>();
        let hb = state
            .iter(app.world())
            .next()
            .expect("active window spawns the volume");
        match hb.anchor {
            HitboxAnchor::FollowOwner { local_offset } => (local_offset, hb.half_extent),
            _ => panic!("a moveset volume must anchor FollowOwner"),
        }
    }

    let authored_local = ae::Vec2::new(28.0, -20.0); // facing +1
    let authored_half = ae::Vec2::new(16.0, 12.0);
    for dir in [
        ae::Vec2::new(0.0, 1.0),  // down (baseline)
        ae::Vec2::new(1.0, 0.0),  // right
        ae::Vec2::new(0.0, -1.0), // up
        ae::Vec2::new(-1.0, 0.0), // left
    ] {
        let (world_offset, world_half) = spawn_and_read(dir);
        let frame = ae::AccelerationFrame::new(dir);
        // The stored WORLD offset, read back into the BODY frame, is invariant
        // across gravities — the symmetry property an unrotated spawn breaks.
        let recovered = frame.to_local(world_offset);
        assert!(
            (recovered - authored_local).length() < 1e-3,
            "dir {dir:?}: the body-local strike offset must be gravity-invariant; \
             got {recovered:?}, want {authored_local:?}"
        );
        // The half-extent rotates too: (16,12) → (12,16) at 90°.
        let expected_half = frame.to_world_half(authored_half);
        assert!(
            (world_half - expected_half).length() < 1e-3,
            "dir {dir:?}: half-extent must rotate with gravity; got {world_half:?}, \
             want {expected_half:?}"
        );
    }
}

/// CM3: a fully-charged release scales the spawned hitbox's damage AND
/// knockback by `smash_charge_mult`; `1.0` is byte-parity.
#[test]
fn a_charged_release_scales_the_spawned_hitbox() {
    fn charge_move(mult: f32) -> MoveSpec {
        let ron = format!(
            r#"(
                schema_version: 1,
                entities: [(
                    id: "seed",
                    contracts: (moveset: Some((
                        verbs: {{"attack": "smash"}},
                        moves: [(
                            id: "smash",
                            clip: (clip: "slash", fallbacks: ["idle"]),
                            duration_s: 0.5,
                            smash_charge_mult: {mult},
                            windows: [
                                (start_s: 0.0, end_s: 0.2, tag: Startup, volumes: []),
                                (start_s: 0.2, end_s: 0.4, tag: Active, volumes: [
                                    (shape: Rect(offset: (28.0, 0.0), half_extents: (16.0, 12.0)),
                                     damage: 5, knockback: 100.0),
                                ]),
                            ],
                        )],
                    ))),
                )],
            )"#
        );
        let doc = ambition_entity_catalog::EntityCatalogDoc::parse(&ron).unwrap();
        doc.entity("seed")
            .unwrap()
            .contracts
            .moveset
            .as_ref()
            .unwrap()
            .move_for_verb("attack")
            .unwrap()
            .clone()
    }
    let read = |mult: f32| -> (i32, f32) {
        let (mut app, _v) = app_with_victim();
        spawn_attacker(
            &mut app,
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(15.0, 24.0),
            charge_move(mult),
        );
        // Run into the Active window (t ≈ 0.26): the charge window (0..0.2)
        // is fully elapsed, so the release is fully charged.
        run_seconds(&mut app, 0.25);
        let mut q = app.world_mut().query::<&Hitbox>();
        let hb = q
            .iter(app.world())
            .next()
            .expect("the active window spawns the volume");
        (hb.damage, hb.knockback_strength)
    };
    // Parity: unit mult leaves the authored values exactly.
    assert_eq!(read(1.0), (5, 100.0));
    // Full charge at 2.0 doubles both.
    let (dmg, kb) = read(2.0);
    assert_eq!(dmg, 10, "damage doubles at full charge");
    assert!(
        (kb - 200.0).abs() < 1e-3,
        "knockback doubles at full charge: {kb}"
    );
}

/// W9 decomposability proof: the SAME MoveSpec value bound to a second,
/// differently-shaped actor lands the same hit — re-binding is data.
#[test]
fn rebinding_the_same_move_to_another_actor_is_data_only() {
    let (mut app, _victim) = app_with_victim();
    // A "goblin": different body, different position, same move data.
    spawn_attacker(
        &mut app,
        ae::Vec2::new(156.0, 100.0), // attacks leftward…
        ae::Vec2::new(12.0, 18.0),
        swat(),
    );
    // …so flip its facing to reach the victim at x=128.
    let goblin = app
        .world_mut()
        .query_filtered::<Entity, With<MovePlayback>>()
        .iter(app.world())
        .next()
        .unwrap();
    app.world_mut()
        .get_mut::<MovePlayback>(goblin)
        .unwrap()
        .facing = -1.0;

    run_seconds(&mut app, 0.40);
    let cap = app.world().resource::<Captured>();
    assert_eq!(
        cap.hits.len(),
        1,
        "the goblin lands the player-authored move with zero Rust changes"
    );
}

/// W9 relativity proof: a 0.25x-dilated attacker's move — windows AND
/// picture — runs at quarter speed. After 0.32s of world time the
/// undilated attacker has already hit; the dilated one is still in
/// startup with a proportionally smaller phase. Its hit arrives ~4x
/// later, and the volume's world-time life stretches with it.
#[test]
fn dilated_owner_slows_windows_and_picture_together() {
    let (mut app, _victim) = app_with_victim();
    let dilated = spawn_attacker(
        &mut app,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(15.0, 24.0),
        swat(),
    );
    app.world_mut()
        .entity_mut(dilated)
        .insert(ProperTimeScale(0.25));

    run_seconds(&mut app, 0.32);
    {
        let cap = app.world().resource::<Captured>();
        assert!(cap.hits.is_empty(), "dilated attacker is still winding up");
        let playback = app.world().get::<MovePlayback>(dilated).unwrap();
        // ~0.32s world → ~0.08s proper → phase ~0.12, picture in startup.
        assert!(
            playback.phase() < 0.28 / 0.68,
            "picture is slaved to the slow clock"
        );
    }

    // Four times the world time reaches the same proper-time window.
    run_seconds(&mut app, 1.0);
    let cap = app.world().resource::<Captured>();
    assert_eq!(cap.hits.len(), 1, "the dilated swat lands, just later");
}

fn count_hitboxes(app: &mut App) -> usize {
    app.world_mut().query::<&Hitbox>().iter(app.world()).count()
}

/// A two-window (two Active spans) move authored as data — a light poke into a
/// heavier follow-up, the shape of the player-robot's "Theorem Chain".
fn two_hit_combo() -> MoveSpec {
    let doc = ambition_entity_catalog::EntityCatalogDoc::parse(
        r#"(
            schema_version: 1,
            entities: [(
                id: "combo",
                contracts: (moveset: Some((
                    verbs: {"special": "chain"},
                    moves: [(
                        id: "chain",
                        clip: (clip: "slash", fallbacks: ["idle"]),
                        duration_s: 0.72,
                        windows: [
                            (start_s: 0.0, end_s: 0.14, tag: Startup, volumes: []),
                            (start_s: 0.14, end_s: 0.22, tag: Active, volumes: [
                                (shape: Rect(offset: (28.0, 0.0), half_extents: (18.0, 14.0)),
                                 damage: 2, knockback: 90.0),
                            ]),
                            (start_s: 0.22, end_s: 0.36, tag: Recovery, volumes: []),
                            (start_s: 0.36, end_s: 0.46, tag: Active, volumes: [
                                (shape: Rect(offset: (30.0, 0.0), half_extents: (20.0, 16.0)),
                                 damage: 3, knockback: 160.0),
                            ]),
                            (start_s: 0.46, end_s: 0.72, tag: Recovery, volumes: []),
                        ],
                    )],
                ))),
            )],
        )"#,
    )
    .unwrap();
    assert!(
        doc.validate().is_empty(),
        "the two-hit combo is well-formed"
    );
    doc.entity("combo")
        .unwrap()
        .contracts
        .moveset
        .as_ref()
        .unwrap()
        .move_for_verb("special")
        .unwrap()
        .clone()
}

/// A held "beam": a 0.30s window that SUSTAINS an `Effect` every active frame.
fn beam_move() -> MoveSpec {
    let doc = ambition_entity_catalog::EntityCatalogDoc::parse(
        r#"(
            schema_version: 1,
            entities: [(
                id: "caster",
                contracts: (moveset: Some((
                    verbs: {"special": "beam"},
                    moves: [(
                        id: "beam",
                        clip: (clip: "special", fallbacks: ["idle"]),
                        duration_s: 0.40,
                        windows: [
                            (start_s: 0.0, end_s: 0.30, tag: Active, volumes: [],
                             sustain_effect: Some((key: "beam_tick"))),
                            (start_s: 0.30, end_s: 0.40, tag: Recovery, volumes: []),
                        ],
                    )],
                ))),
            )],
        )"#,
    )
    .unwrap();
    assert!(doc.validate().is_empty());
    doc.entity("caster")
        .unwrap()
        .contracts
        .moveset
        .as_ref()
        .unwrap()
        .move_for_verb("special")
        .unwrap()
        .clone()
}

/// The HELD-special primitive (fable review §A1, the shape the boss fold needs):
/// a window carrying a `sustain_effect` emits its `Effect` EVERY frame it is
/// active (not one-shot), and STOPS the frame the window ends — so a consuming
/// technique gets the continuous "active this tick" signal the boss's
/// apple_rain-style specials run on. Pins the per-frame sustain.
#[test]
fn a_sustained_effect_window_emits_its_effect_every_active_frame() {
    let (mut app, _victim) = app_with_victim();
    let _caster = spawn_attacker(
        &mut app,
        ae::Vec2::new(400.0, 100.0), // far from the victim — no hits, just the sustain
        ae::Vec2::new(15.0, 24.0),
        beam_move(),
    );
    // Run PAST the sustain window (0.30s) but within the move (0.40s).
    run_seconds(&mut app, 0.36);
    let cap = app.world().resource::<Captured>();
    let beam_ticks = cap
        .events
        .iter()
        .filter(|e| matches!(&e.kind, MoveEventKind::Effect(effect) if effect.key == "beam_tick"))
        .count();
    // ~0.30s / 0.016 ≈ 18 active frames; robustly many, and it stopped (the
    // move is 0.40s but the sustain window ended at 0.30s → not every frame).
    assert!(
        (15..=19).contains(&beam_ticks),
        "the sustain fired once per active frame (~18), got {beam_ticks}"
    );
}

/// Smash-like MULTI-HIT expressivity (fable review §A1): a single authored move
/// with TWO Active windows lands TWO distinct hits on a standing victim — the
/// first window's box despawns before the second spawns, and each carries its
/// own `HitboxHits`, so the combo reads as two strikes, not one lingering box.
/// Pins that the moveset runtime expresses combos, not just single swings.
#[test]
fn a_two_window_move_lands_two_distinct_hits() {
    let (mut app, _victim) = app_with_victim();
    let _attacker = spawn_attacker(
        &mut app,
        ae::Vec2::new(104.0, 100.0),
        ae::Vec2::new(15.0, 24.0),
        two_hit_combo(),
    );
    // Run the whole move. Two Active windows → two hits.
    run_seconds(&mut app, 0.75);
    let cap = app.world().resource::<Captured>();
    assert_eq!(
        cap.hits.len(),
        2,
        "the two-window combo lands exactly two distinct hits"
    );
    assert_eq!(cap.hits[0].damage, 2, "first window's authored damage");
    assert_eq!(cap.hits[1].damage, 3, "second window's authored damage");
}

/// Phase-0 keystone (fable review §A1, Path B): the PRODUCTION trigger — a body
/// carrying an `ActorMoveset` whose control frame presses `special` starts the
/// matching move (no test hand-inserts `MovePlayback`), and the move lands its
/// authored hit through the real path. This is the insert the moveset runtime
/// was missing; without it the whole system was dead in the shipping game.
#[test]
fn a_control_verb_edge_triggers_the_moveset_move_and_lands_it() {
    // Self-contained app: the full production chain registered ONCE
    // (trigger → advance → damage → capture) + a victim in reach.
    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<super::super::authored_volumes::AuthoredAttackVolumeResolver>();
    app.add_message::<HitEvent>();
    app.add_message::<SfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.init_resource::<Captured>();
    app.init_resource::<WorldTime>();
    app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
    app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
    app.add_systems(
        Update,
        (
            trigger_moveset_moves,
            advance_move_playback,
            apply_hitbox_damage,
            capture,
        )
            .chain(),
    );
    app.world_mut().spawn((
        ambition_platformer_primitives::markers::PlayerEntity,
        ActorFaction::Player,
        ambition_engine_core::BodyKinematics {
            pos: ae::Vec2::new(128.0, 100.0),
            size: ae::Vec2::new(28.0, 46.0),
            facing: -1.0,
            ..Default::default()
        },
        ae::CenteredAabb::from_center_size(ae::Vec2::new(128.0, 100.0), ae::Vec2::new(28.0, 46.0)),
        ambition_engine_core::BodyOffense::default(),
        ambition_engine_core::BodyMotionFacts::default(),
        ambition_engine_core::BodyShieldState::default(),
        ambition_characters::actor::BodyCombat::default(),
    ));
    // A body that OWNS a repertoire and is pressing `special` this frame — but
    // is NOT hand-given a MovePlayback. The trigger must start the move.
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.special_pressed = true;
    app.world_mut().spawn((
        ae::CenteredAabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(15.0, 24.0)),
        ae::BodyKinematics {
            pos: ae::Vec2::new(100.0, 100.0),
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(15.0, 24.0),
            facing: 1.0,
        },
        ActorFaction::Enemy,
        ActorMoveset(swat_moveset()),
        ActorControl(frame),
    ));

    // Through one move: the verb edge started it, the active window landed the
    // authored hit exactly once (0.68s move; stop before it can re-trigger).
    run_seconds(&mut app, 0.5);
    let cap = app.world().resource::<Captured>();
    assert_eq!(
        cap.hits.len(),
        1,
        "the special verb edge triggered the move and it landed its hit"
    );
    assert_eq!(cap.events.len(), 1, "the move's timed Sfx event fired once");
}

/// A move authoring `start_impulse` lunges the body toward its facing at
/// trigger — the self-motion the flat directional swings applied at
/// `start_attack`, now move DATA the player-melee fold rides.
#[test]
fn a_move_start_impulse_lunges_the_body_toward_facing() {
    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.init_resource::<WorldTime>();
    app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
    app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
    app.add_systems(Update, trigger_moveset_moves);
    let mv = MoveSpec {
        id: ATTACK_VERB.into(),
        clip: ClipBinding {
            clip: "x".into(),
            fallbacks: vec![],
        },
        duration_s: 0.3,
        windows: vec![],
        events: vec![],
        gates: Default::default(),
        start_impulse: Some((150.0, 0.0)),
        smash_charge_mult: 1.0,
    };
    let mut verbs = std::collections::BTreeMap::new();
    verbs.insert(ATTACK_VERB.to_string(), ATTACK_VERB.to_string());
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    let body = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(28.0, 46.0),
                facing: -1.0,
            },
            ActorFaction::Enemy,
            ActorMoveset(MovesetContract {
                verbs,
                moves: vec![mv],
            }),
            ActorControl(frame),
        ))
        .id();
    app.update();
    let vel = app.world().get::<ae::BodyKinematics>(body).unwrap().vel;
    // facing = -1 → forward is -x; default gravity → no rotation.
    assert!(
        (vel.x + 150.0).abs() < 1.0,
        "the move lunged the body toward its facing, vel={vel:?}"
    );
    assert!(
        vel.y.abs() < 1.0,
        "a horizontal lunge adds no vertical velocity, vel={vel:?}"
    );
}

/// Phase-0 keystone: the EFFECT dispatch — the moveset runtime only NAMES
/// events; `dispatch_move_events` resolves an `Sfx{cue}` to a positioned
/// `SfxMessage` and BRIDGES an `Effect{key}` to the SAME
/// `ActorActionMessage::Special{Special(key)}` the brain special path emits, so
/// a data-driven move fires a content technique with zero new plumbing (the
/// exact seam the boss `Special(key)` profiles reuse).
#[test]
fn move_event_dispatch_bridges_sfx_to_sound_and_effect_to_special() {
    use ambition_characters::brain::ActorActionMessage;
    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<SfxMessage>();
    app.add_message::<ActorActionMessage>();
    app.add_systems(Update, dispatch_move_events);
    let owner = app
        .world_mut()
        .spawn(ae::BodyKinematics {
            pos: ae::Vec2::new(42.0, 7.0),
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(16.0, 24.0),
            facing: 1.0,
        })
        .id();
    app.world_mut()
        .resource_mut::<Messages<MoveEventMessage>>()
        .write(MoveEventMessage {
            owner,
            move_id: "sig".into(),
            kind: MoveEventKind::Sfx {
                cue: "pca.signature".into(),
            },
        });
    app.world_mut()
        .resource_mut::<Messages<MoveEventMessage>>()
        .write(MoveEventMessage {
            owner,
            move_id: "sig".into(),
            kind: MoveEventKind::Effect(EffectRef {
                key: "pca_glider".into(),
                // A1: authored params must SURVIVE the bridge so the keyed
                // technique can hydrate them.
                params: ambition_entity_catalog::ParamValue::parse("(rise: 320.0)")
                    .expect("param RON parses"),
            }),
        });
    app.update();

    let sfx: Vec<SfxMessage> = app
        .world_mut()
        .resource_mut::<Messages<SfxMessage>>()
        .drain()
        .collect();
    assert_eq!(sfx.len(), 1, "the Sfx event played one sound");
    assert!(
        matches!(sfx[0], SfxMessage::Play { pos, .. } if pos == ae::Vec2::new(42.0, 7.0)),
        "played at the owner's position"
    );
    let acts: Vec<ActorActionMessage> = app
        .world_mut()
        .resource_mut::<Messages<ActorActionMessage>>()
        .drain()
        .collect();
    assert_eq!(
        acts.len(),
        1,
        "the Effect event bridged to one Special action"
    );
    assert_eq!(acts[0].actor, owner);
    let ActionRequest::Special { spec, params } = &acts[0].request else {
        panic!("the Effect event bridged to a Special action");
    };
    assert!(matches!(spec, SpecialActionSpec::Special(k) if k == "pca_glider"));
    // The authored params rode through the bridge and hydrate on the far
    // side (the first real consumer — a G3 limb technique / demo move —
    // reads them exactly this way).
    #[derive(serde::Deserialize)]
    struct GliderParams {
        rise: f32,
    }
    let hydrated: GliderParams = params.hydrate().expect("params hydrate");
    assert_eq!(
        hydrated.rise, 320.0,
        "authored params survived the dispatch"
    );
}

/// Ranged subsumption (option A): a `MoveEventKind::Ranged` fire event BRIDGES to
/// the SAME `ActorActionMessage::Ranged` the flat `frame.fire` resolver emits —
/// carrying the owner's authored `ActionSet.ranged` spec and SAMPLING its LIVE
/// aim at the event frame — so the existing enemy-projectile consumer fires the
/// shot unchanged and a moveset shot still tracks a strafing target.
#[test]
fn move_event_dispatch_bridges_ranged_to_a_live_aimed_shot() {
    use ambition_characters::actor::control::ActorFireRequest;
    use ambition_characters::brain::action_set::{ActionSet, RangedActionSpec};
    use ambition_characters::brain::{ActorActionMessage, ActorControl};
    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<SfxMessage>();
    app.add_message::<ActorActionMessage>();
    app.add_systems(Update, dispatch_move_events);

    let mut control = ActorControl::default();
    // Live aim this frame: a world-space up-right shot toward a strafing target.
    control.0.fire = Some(ActorFireRequest::world_space(
        ae::Vec2::new(0.6, -0.8),
        240.0,
    ));
    let owner = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: ae::Vec2::new(100.0, 50.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(16.0, 24.0),
                facing: 1.0,
            },
            ActionSet {
                ranged: Some(RangedActionSpec::Bolt {
                    speed: 240.0,
                    damage: 3,
                }),
                ..Default::default()
            },
            control,
        ))
        .id();
    app.world_mut()
        .resource_mut::<Messages<MoveEventMessage>>()
        .write(MoveEventMessage {
            owner,
            move_id: "fire".into(),
            kind: MoveEventKind::Ranged,
        });
    app.update();

    let acts: Vec<ActorActionMessage> = app
        .world_mut()
        .resource_mut::<Messages<ActorActionMessage>>()
        .drain()
        .collect();
    assert_eq!(
        acts.len(),
        1,
        "the Ranged event bridged to one Ranged action"
    );
    match &acts[0].request {
        ActionRequest::Ranged {
            spec, origin, dir, ..
        } => {
            assert!(matches!(spec, RangedActionSpec::Bolt { damage: 3, .. }));
            assert_eq!(*origin, ae::Vec2::new(100.0, 50.0), "origin = owner pos");
            assert_eq!(*dir, ae::Vec2::new(0.6, -0.8), "dir SAMPLED from live aim");
        }
        other => panic!("expected ActionRequest::Ranged, got {other:?}"),
    }
}

/// Ranged subsumption slice 2: `build_actor_moveset` folds `ActionSet.ranged`
/// into a `"ranged"`-verb fire move (Startup → fire event → Recovery, no hit
/// volume), and `trigger_moveset_moves` starts it on a `frame.fire` intent — the
/// same trigger seam melee/specials use.
#[test]
fn a_fire_intent_triggers_the_ranged_move() {
    use ambition_characters::actor::control::ActorFireRequest;
    use ambition_characters::brain::action_set::RangedActionSpec;
    use ambition_characters::brain::ActorControl;

    let contract = build_actor_moveset(
        None,
        None,
        Some(&RangedActionSpec::Bolt {
            speed: 240.0,
            damage: 3,
        }),
    )
    .expect("a ranged weapon → a moveset with a fire move");
    let fire = contract
        .move_for_verb(RANGED_VERB)
        .expect("the ranged verb maps to the fire move");
    assert_eq!(fire.id, RANGED_VERB);
    assert!(
        fire.windows.iter().all(|w| w.volumes.is_empty()),
        "a shot carries no melee hit volume — the projectile is the damage"
    );
    assert_eq!(
        fire.events
            .iter()
            .filter(|e| e.kind == MoveEventKind::Ranged)
            .count(),
        1,
        "exactly one fire event"
    );

    let mut app = App::new();
    app.add_systems(Update, trigger_moveset_moves);
    let mut control = ActorControl::default();
    control.0.fire = Some(ActorFireRequest::world_space(
        ae::Vec2::new(1.0, 0.0),
        240.0,
    ));
    let body = app
        .world_mut()
        .spawn((
            ActorMoveset(contract),
            control,
            ae::BodyKinematics {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(16.0, 24.0),
                facing: 1.0,
            },
        ))
        .id();
    app.update();
    let pb = app
        .world()
        .get::<MovePlayback>(body)
        .expect("the fire intent started the ranged move");
    assert_eq!(pb.spec.id, RANGED_VERB);
}

/// Regression (ranged-fold): a body that is BOTH `MovesetMelee` and playing its
/// `"ranged"` (or any non-`"attack"`) move must NOT get a phantom `BodyMelee.swing`
/// — otherwise the movement pipeline reads it as "mid-attack" and freezes the
/// firing body in place (this froze the PCA's chase in `actor_phase_split`). Only
/// the `"attack"` move projects a swing.
#[test]
fn a_ranged_move_does_not_project_a_phantom_melee_swing() {
    use ambition_characters::brain::action_set::{MeleeActionSpec, RangedActionSpec, SwipeSpec};
    // Same body carries both a melee AND a ranged move (both verbs).
    let contract = build_actor_moveset(
        None,
        Some(&MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
        Some(&RangedActionSpec::Rock {
            speed: 300.0,
            damage: 1,
        }),
    )
    .expect("melee + ranged → a moveset");
    let fire = contract.move_for_verb(RANGED_VERB).unwrap().clone();
    let attack = contract.move_for_verb(ATTACK_VERB).unwrap().clone();

    let mut app = App::new();
    app.add_systems(Update, project_moveset_melee_to_body_melee);

    // Playing the RANGED move → no swing (the body isn't attacking).
    let firing = app
        .world_mut()
        .spawn((
            MovesetMelee,
            BodyMelee::default(),
            MovePlayback::new(fire, 1.0),
        ))
        .id();
    // Playing the ATTACK move → a swing (the read-model the flat swing published).
    let swinging = app
        .world_mut()
        .spawn((
            MovesetMelee,
            BodyMelee::default(),
            MovePlayback::new(attack, 1.0),
        ))
        .id();
    app.update();
    assert!(
        app.world()
            .get::<BodyMelee>(firing)
            .unwrap()
            .swing
            .is_none(),
        "a firing body must not read as mid-swing"
    );
    assert!(
        app.world()
            .get::<BodyMelee>(swinging)
            .unwrap()
            .swing
            .is_some(),
        "the attack move still projects its swing read-model"
    );
}

// -----------------------------------------------------------------------
// CM4 — cancel tables: the timeline IS the cancel table.
// -----------------------------------------------------------------------

use ambition_entity_catalog::{CancelCondition, MoveWindow};

/// A minimal trigger-only harness: the ONE trigger seam + a body holding a
/// verb on its control frame while a move plays.
fn trigger_app() -> App {
    let mut app = App::new();
    app.add_systems(Update, trigger_moveset_moves);
    app
}

fn pressing_attack() -> ActorControl {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::default();
    frame.melee_pressed = true;
    ActorControl(frame)
}

fn spawn_mover(app: &mut App, playing: MoveSpec, control: ActorControl) -> Entity {
    app.world_mut()
        .spawn((
            ActorMoveset(swat_moveset()),
            control,
            ae::BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(15.0, 24.0),
                facing: 1.0,
            },
            MovePlayback::new(playing, 1.0),
        ))
        .id()
}

/// A distinct playing move so the replacement is observable by id, with an
/// optional cancel window appended to its timeline.
fn playing_move(cancel: Option<MoveWindow>) -> MoveSpec {
    let mut spec = swat();
    spec.id = "first".to_string();
    if let Some(w) = cancel {
        spec.windows.push(w);
    }
    spec
}

fn cancel_window(into: &[&str], condition: CancelCondition) -> MoveWindow {
    MoveWindow {
        start_s: 0.0,
        end_s: 0.68,
        tag: WindowTag::Cancelable {
            into: into.iter().map(|s| s.to_string()).collect(),
            condition,
        },
        volumes: vec![],
        sustain_effect: None,
    }
}

/// PARITY PIN: with no `Cancelable` window authored, a verb press during a
/// playing move is rejected exactly as before CM4 — the playback keeps
/// playing the same move.
#[test]
fn no_cancel_window_rejects_a_new_move_byte_identically() {
    let mut app = trigger_app();
    let body = spawn_mover(&mut app, playing_move(None), pressing_attack());
    app.update();
    let pb = app
        .world()
        .get::<MovePlayback>(body)
        .expect("still playing");
    assert_eq!(pb.spec.id, "first", "the playing move is untouched");
}

/// A covering `Always` cancel window naming `any_attack` lets the pressed
/// attack REPLACE the playing move same-frame.
#[test]
fn cancel_window_starts_the_new_move_same_frame() {
    let mut app = trigger_app();
    let body = spawn_mover(
        &mut app,
        playing_move(Some(cancel_window(
            &["any_attack"],
            CancelCondition::Always,
        ))),
        pressing_attack(),
    );
    app.update();
    let pb = app.world().get::<MovePlayback>(body).expect("playing");
    assert_eq!(pb.spec.id, "swat", "canceled into the attack move");
    assert_eq!(pb.t, 0.0, "the new move starts from its own zero");
}

/// A cancel window that names something ELSE (a specific other id) refuses
/// the attack — `into` membership is the gate, not the window's existence.
#[test]
fn cancel_window_gates_on_the_into_list() {
    let mut app = trigger_app();
    let body = spawn_mover(
        &mut app,
        playing_move(Some(cancel_window(&["jump"], CancelCondition::Always))),
        pressing_attack(),
    );
    app.update();
    let pb = app.world().get::<MovePlayback>(body).expect("playing");
    assert_eq!(pb.spec.id, "first", "an attack is not a jump");
}

/// `OnHit` opens only after the move CONNECTED (the combo confirm); a whiff
/// stays locked, and setting the landed fact unlocks the same press.
#[test]
fn on_hit_cancel_requires_the_landed_fact() {
    let mut app = trigger_app();
    let body = spawn_mover(
        &mut app,
        playing_move(Some(cancel_window(&["any_attack"], CancelCondition::OnHit))),
        pressing_attack(),
    );
    app.update();
    assert_eq!(
        app.world().get::<MovePlayback>(body).unwrap().spec.id,
        "first",
        "whiffing: OnHit stays locked"
    );
    app.world_mut()
        .get_mut::<MovePlayback>(body)
        .unwrap()
        .landed_hit = true;
    app.update();
    assert_eq!(
        app.world().get::<MovePlayback>(body).unwrap().spec.id,
        "swat",
        "the connect fact opens the combo"
    );
}

/// `OnWhiff` is the inverse: open while the move has NOT connected, locked
/// once it has (a bail-out window, not a combo window).
#[test]
fn on_whiff_cancel_locks_after_a_connect() {
    let mut app = trigger_app();
    let body = spawn_mover(
        &mut app,
        playing_move(Some(cancel_window(
            &["any_attack"],
            CancelCondition::OnWhiff,
        ))),
        pressing_attack(),
    );
    app.world_mut()
        .get_mut::<MovePlayback>(body)
        .unwrap()
        .landed_hit = true;
    app.update();
    assert_eq!(
        app.world().get::<MovePlayback>(body).unwrap().spec.id,
        "first",
        "a connected move refuses its whiff escape"
    );
}

/// A `jump` cancel entry ENDS the move on the jump edge — the playback is
/// removed (the locomotion path performs the jump itself from the same
/// frame); no new move starts.
#[test]
fn jump_cancel_ends_the_move_early() {
    let mut app = trigger_app();
    let mut frame = ambition_characters::actor::control::ActorControlFrame::default();
    frame.jump_pressed = true;
    let body = spawn_mover(
        &mut app,
        playing_move(Some(cancel_window(&["jump"], CancelCondition::Always))),
        ActorControl(frame),
    );
    app.update();
    assert!(
        app.world().get::<MovePlayback>(body).is_none(),
        "the jump edge ended the move"
    );
}

/// The connect fact is set by the REAL hit path: an attacker whose Active
/// window overlaps a victim gets `landed_hit = true` the frame the hit
/// resolves (the harness runs `mark_move_playback_landed_hits` in its
/// production position).
#[test]
fn the_real_hit_path_sets_the_landed_fact() {
    let (mut app, _victim) = app_with_victim();
    let attacker = spawn_attacker(
        &mut app,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(15.0, 24.0),
        swat(),
    );
    run_seconds(&mut app, 0.20);
    assert!(
        !app.world()
            .get::<MovePlayback>(attacker)
            .unwrap()
            .landed_hit,
        "startup: nothing connected yet"
    );
    run_seconds(&mut app, 0.12);
    assert!(
        app.world()
            .get::<MovePlayback>(attacker)
            .unwrap()
            .landed_hit,
        "the active-window connect set the fact"
    );
}

/// A3 behavioral grant, through the real moveset derivation: equipping the
/// flower-analog (a `Ranged` grant) overlays `ActionSet.ranged`, and
/// `build_actor_moveset` derives a `"ranged"` verb → a `simple_ranged`-backed
/// move. Unequip is its inverse. This is the exit-test assertion "verb map gains
/// the ranged move; unequip removes it".
#[test]
fn a3_flower_grant_adds_and_removes_a_ranged_verb_in_the_derived_moveset() {
    use ambition_characters::brain::action_set::{ActionSet, RangedActionSpec};
    use ambition_characters::equipment::{
        apply_equipment_grants, EquipmentGrant, EquipmentRow, WornEquipment,
    };

    let flower = EquipmentRow {
        id: "fire_flower".to_string(),
        grants: vec![EquipmentGrant::Ranged(RangedActionSpec::Bolt {
            speed: 420.0,
            damage: 6,
        })],
        ..Default::default()
    };
    let mut worn = WornEquipment::default();
    worn.equip(flower);

    // A peaceful body: no ranged verb in its derived moveset.
    let actions = ActionSet::peaceful();
    let before = build_actor_moveset(None, actions.melee.as_ref(), actions.ranged.as_ref());
    assert!(
        before.map_or(true, |m| m.move_for_verb(RANGED_VERB).is_none()),
        "no flower, no ranged move"
    );

    // Equip the flower: the grant confers the ranged verb, and the moveset
    // derivation turns it into a fireable move.
    let mut equipped = ActionSet::peaceful();
    apply_equipment_grants(&mut equipped, &worn);
    let moveset = build_actor_moveset(None, equipped.melee.as_ref(), equipped.ranged.as_ref())
        .expect("a ranged verb yields a moveset");
    assert!(
        moveset.move_for_verb(RANGED_VERB).is_some(),
        "the flower's ranged verb is in the derived moveset"
    );

    // Unequip: rebuild from the emptied worn set — the verb is gone (nothing was
    // baked to outlive the row).
    worn.unequip("fire_flower");
    let mut after = ActionSet::peaceful();
    apply_equipment_grants(&mut after, &worn);
    let moveset = build_actor_moveset(None, after.melee.as_ref(), after.ranged.as_ref());
    assert!(
        moveset.map_or(true, |m| m.move_for_verb(RANGED_VERB).is_none()),
        "unequip removes the ranged move"
    );
}

/// The A3 equip contract `equip_equipment_row`: a grant-free row is read-time only
/// (no moveset, no action-set change); a grant-bearing row overlays the verb and
/// re-derives a moveset that PRESERVES the body's existing verbs.
#[test]
fn a3_equip_equipment_row_is_read_time_for_plain_rows_and_rebuilds_for_grants() {
    use ambition_characters::brain::action_set::{ActionSet, RangedActionSpec};
    use ambition_characters::equipment::{EquipmentGrant, EquipmentRow, WornEquipment};
    use ambition_entity_catalog::MovesetContract;

    let mut actions = ActionSet::peaceful();
    let mut worn = WornEquipment::default();

    // A grant-free grow-cap analog: returns None, touches neither.
    let grow_cap = EquipmentRow {
        id: "grow_cap".to_string(),
        ..Default::default()
    };
    assert!(
        equip_equipment_row(&mut actions, &mut worn, None, grow_cap).is_none(),
        "a grant-free row wires no moveset — its effect is read-time"
    );
    assert!(actions.ranged.is_none(), "no grant, no action-set change");
    assert!(worn.wears("grow_cap"), "but it is recorded as worn");

    // A grant-bearing spark-blossom analog, equipped OVER a body that already has a
    // signature verb: the rebuilt moveset gains "ranged" AND keeps the signature.
    let mut current = MovesetContract::default();
    current
        .verbs
        .insert("special".to_string(), "chain".to_string());
    let blossom = EquipmentRow {
        id: "spark_blossom".to_string(),
        grants: vec![EquipmentGrant::Ranged(RangedActionSpec::Bolt {
            speed: 420.0,
            damage: 6,
        })],
        ..Default::default()
    };
    let rebuilt = equip_equipment_row(&mut actions, &mut worn, Some(&current), blossom)
        .expect("a granted verb rebuilds a moveset");
    assert!(
        rebuilt.move_for_verb(RANGED_VERB).is_some(),
        "the granted ranged verb is fireable"
    );
    assert!(
        rebuilt.verbs.contains_key("special"),
        "the body's existing signature verb survives the equip"
    );
    assert!(
        actions.ranged.is_some(),
        "the grant overlaid the action set"
    );
    assert!(worn.wears("spark_blossom"));
}

/// Regression (2026-07-12): a `MovesetMelee` body's `BodyMelee.swing` is a
/// read-model that `project_moveset_melee_to_body_melee` rebuilds EVERY frame.
/// The one-hit-per-target dedup (`hit_targets`, folded in by the downstream
/// Volume resolver) used to live on that ephemeral swing, so it was wiped every
/// tick — and the player's slash/pogo re-hit + re-fired the hit SFX on every
/// active frame ("multi-hit on objects, lots of SFX at once"). The accumulator
/// now lives on the persistent `MovePlayback`; the projection must COPY it onto
/// the swing so `apply_hitbox_damage` re-emits it as `ignored_targets`.
#[test]
fn the_moveset_projection_carries_the_hit_dedup_accumulator() {
    let mut app = App::new();
    let mut playback = MovePlayback::new(simple_melee(&SimpleMeleeParams::default()), 1.0);
    playback.hit_targets = vec!["enemy:already_struck".to_string()];
    let body = app
        .world_mut()
        .spawn((playback, BodyMelee::default(), MovesetMelee))
        .id();
    app.add_systems(Update, project_moveset_melee_to_body_melee);
    app.update();

    let melee = app.world().get::<BodyMelee>(body).unwrap();
    let swing = melee.swing.as_ref().expect("a melee move projects a swing");
    assert_eq!(
        swing.hit_targets,
        vec!["enemy:already_struck".to_string()],
        "the projected swing must carry the move's persistent hit-dedup set, or \
         every active tick re-hits the same target (multi-hit / SFX spam)"
    );
}
