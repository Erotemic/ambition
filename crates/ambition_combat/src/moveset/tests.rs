//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
// ⚠ these arrived through `mod.rs`'s imports until 2026-08-12, when making
// `prefabs.rs`'s coupling explicit (P1.7) showed they were only there to feed
// a glob. Named here because this is where they are used.
use crate::events::HitEvent;
use crate::hitbox::apply_hitbox_damage;
use ambition_characters::brain::action_set::MeleeActionSpec;
use ambition_entity_catalog::{ClipBinding, EffectRef, HitVolume, MoveEvent};
use ambition_sfx::SfxMessage;
use ambition_vfx::vfx::DebrisBurstMessage;
use ambition_vfx::vfx::VfxMessage;
use bevy::prelude::*;

/// **The attack direction is facing-relative, not screen-relative.** The aim
/// axis arrives screen-local (`+x` = screen-right), but a forward press must read
/// `Forward` no matter which way you face — and a press toward your BACK must read
/// `Back`. Regression pin for the "move left + attack fired right" bug: facing was
/// not folded into `attack_dir_from_axis`, so a left-facing forward press
/// misclassified as `Back` and fired the aerial back-attack the wrong way.
#[test]
fn attack_dir_is_relative_to_facing() {
    // Facing RIGHT (+1): screen-right is forward, screen-left is back.
    assert_eq!(
        attack_dir_from_axis(ae::LocalAxes::new(1.0, 0.0), 1.0),
        AttackDir::Forward
    );
    assert_eq!(
        attack_dir_from_axis(ae::LocalAxes::new(-1.0, 0.0), 1.0),
        AttackDir::Back
    );

    // Facing LEFT (-1): the mirror. Pressing screen-LEFT is now FORWARD (the bug
    // case — must be Forward, not Back), pressing screen-right is Back.
    assert_eq!(
        attack_dir_from_axis(ae::LocalAxes::new(-1.0, 0.0), -1.0),
        AttackDir::Forward
    );
    assert_eq!(
        attack_dir_from_axis(ae::LocalAxes::new(1.0, 0.0), -1.0),
        AttackDir::Back
    );

    // Vertical is gravity-local and facing-independent: Up (toward the head) is
    // `y < 0` under either facing; Down is `y > 0`.
    for facing in [1.0, -1.0] {
        assert_eq!(
            attack_dir_from_axis(ae::LocalAxes::new(0.0, -1.0), facing),
            AttackDir::Up
        );
        assert_eq!(
            attack_dir_from_axis(ae::LocalAxes::new(0.0, 1.0), facing),
            AttackDir::Down
        );
    }

    // A neutral (no-aim) press is the plain jab regardless of facing.
    assert_eq!(
        attack_dir_from_axis(ae::LocalAxes::ZERO, 1.0),
        AttackDir::Neutral
    );
    assert_eq!(
        attack_dir_from_axis(ae::LocalAxes::ZERO, -1.0),
        AttackDir::Neutral
    );
}

/// **What this test suite pretends a renderer can draw.**
///
/// ⚠ a STUB, and deliberately so: the real answer is the rows of the shipped FX
/// spritesheets (`ambition_sprite_sheet::fx::is_authored_effect`), and this
/// crate must not link a presentation-asset crate to expand a prefab — a
/// headless RL build expands prefabs and has no image decoder. What is under
/// test here is the registry's REFUSAL, not the vocabulary: an id the oracle
/// rejects must not survive expansion. The vocabulary itself is pinned where it
/// lives, against the sheets.
fn drawable(effect: &str) -> bool {
    matches!(
        effect,
        "classic_burst" | "burst_round" | "shockwave" | "smoke_burst" | "starburst" | "sonic_boom"
    )
}

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
        .expand("simple_melee", &params, "sword_slash", drawable)
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
        reg.expand("not_a_prefab", &empty, "x", drawable).is_err(),
        "typo'd key"
    );
    // Wrong type for a field fails at expand (install) time.
    let bad = ambition_entity_catalog::ParamValue::parse("(damage: \"lots\")").unwrap();
    assert!(
        reg.expand("simple_melee", &bad, "x", drawable).is_err(),
        "bad params"
    );
    // Empty params hydrate to the prefab defaults (every field defaults).
    assert!(reg
        .expand("simple_charge", &empty, "smash", drawable)
        .is_ok());
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
            drawable,
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
            drawable,
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
        .expand("simple_melee", &bad, "x", drawable)
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
    struct Seen(Option<ambition_vfx::FxId>);

    fn capture(mut vfx: MessageReader<VfxMessage>, mut seen: ResMut<Seen>) {
        for m in vfx.read() {
            if let VfxMessage::Effect { fx, .. } = m {
                seen.0 = Some(*fx);
            }
        }
    }

    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
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
            presentation_source: ambition_sfx::PresentationSourceId::unscoped(),
            kind: MoveEventKind::Vfx {
                effect: "starburst".to_string(),
            },
        });
    app.update();
    assert_eq!(
        app.world().resource::<Seen>().0,
        Some(ambition_vfx::FxId::new("starburst")),
        "the Vfx event put the authored NAME on the wire — no enum in between",
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

/// CM8: the authored contact sound flows from the melee spec onto the strike
/// volume — the first link of the authoring chain volume → hitbox → event →
/// victim reaction. This is what lets a roster row give a character its own hit
/// sound so a sword and a claw are heard apart.
#[test]
fn an_authored_hit_sfx_rides_the_swing_volume() {
    let active_volume = |m: &MoveSpec| {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .and_then(|w| w.volumes.first())
            .cloned()
            .expect("the Active window carries the strike volume")
    };

    let with_sound = simple_melee(&SimpleMeleeParams {
        hit_sfx: Some("player.slash".to_string()),
        ..Default::default()
    });
    assert_eq!(
        active_volume(&with_sound).hit_sfx.as_deref(),
        Some("player.slash"),
        "the spec's authored contact sound reaches the volume"
    );

    // An unauthored swing carries no strike sound — the victim's own default
    // hurt sound plays instead (parity with pre-CM8).
    let plain = simple_melee(&SimpleMeleeParams::default());
    assert_eq!(active_volume(&plain).hit_sfx, None);
}

/// The robot-player presentation is an identity-scoped overlay: it replaces
/// only the engine-default air swing and fills only unauthored slash contacts.
/// Explicit character/weapon SFX remain authoritative.
#[test]
fn player_robot_slash_overlay_preserves_authored_sfx() {
    let mut default_move = simple_melee(&SimpleMeleeParams::default());
    // The down-air's rebound, as the variant builder authors it: a paramless
    // `pogo_bounce` carrying no cue of its own.
    for window in &mut default_move.windows {
        for volume in &mut window.volumes {
            volume.on_hit = Some(ambition_entity_catalog::EffectRef::new(
                crate::on_hit::POGO_BOUNCE_KEY,
            ));
        }
    }
    let mut authored_move = simple_melee(&SimpleMeleeParams {
        swing_sfx: Some("weapon.authored.swing".to_string()),
        hit_sfx: Some("weapon.authored.hit".to_string()),
        ..Default::default()
    });
    authored_move.id = "attack_authored".to_string();

    let mut moveset = ambition_entity_catalog::MovesetContract {
        verbs: Default::default(),
        moves: vec![default_move, authored_move],
    };
    apply_player_robot_slash_sfx(&mut moveset);

    fn sfx_event(m: &MoveSpec) -> Option<&str> {
        m.events.iter().find_map(|event| match &event.kind {
            MoveEventKind::Sfx { cue } => Some(cue.as_str()),
            _ => None,
        })
    }
    fn hit_sfx(m: &MoveSpec) -> Option<&str> {
        m.windows
            .iter()
            .flat_map(|window| &window.volumes)
            .find(|volume| {
                matches!(
                    volume.vfx.as_deref(),
                    Some(SLASH_ARC_VFX) | Some(SLASH_POKE_VFX)
                )
            })
            .and_then(|volume| volume.hit_sfx.as_deref())
    }

    assert_eq!(
        sfx_event(&moveset.moves[0]),
        Some(PLAYER_ROBOT_SWING_SFX_CUE)
    );
    assert_eq!(
        hit_sfx(&moveset.moves[0]),
        Some(PLAYER_ROBOT_IMPACT_SFX_CUE)
    );
    assert_eq!(sfx_event(&moveset.moves[1]), Some("weapon.authored.swing"));
    assert_eq!(hit_sfx(&moveset.moves[1]), Some("weapon.authored.hit"));

    // The rebound cue rides the pogo EFFECT, not a character-id branch in the
    // technique: the overlay authors it, and the un-overlaid rise survives.
    let pogo = moveset.moves[0]
        .windows
        .iter()
        .flat_map(|window| &window.volumes)
        .find_map(|volume| volume.on_hit.as_ref())
        .expect("the default move kept its pogo effect");
    assert_eq!(
        crate::on_hit::pogo_sfx_from(pogo),
        Some(ambition_sfx::ids::PLAYER_ROBOT_SLASH_IMPACT_POGO),
    );
    assert_eq!(
        crate::on_hit::pogo_rise_from(pogo),
        crate::on_hit::pogo_rise_from(&ambition_entity_catalog::EffectRef::new(
            crate::on_hit::POGO_BOUNCE_KEY
        )),
    );
    // A body that never ran the overlay says nothing, so the technique falls
    // back to the engine's generic pogo cue.
    assert_eq!(
        crate::on_hit::pogo_sfx_from(&ambition_entity_catalog::EffectRef::new(
            crate::on_hit::POGO_BOUNCE_KEY
        )),
        None,
    );
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
    // ⚠ **victims only.** A body-owned melee also publishes the unresolved half
    // of the same strike — the geometry broadcast for breakables and bosses,
    // which name no body. Every assertion in this module counts hits ON a body,
    // so folding the two together would make a one-victim swing look like
    // fourteen hits (one per active tick).
    cap.hits.extend(
        hits.read()
            .filter(|e| matches!(e.target, crate::events::HitTarget::Body(_)))
            .cloned(),
    );
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
    app.add_message::<crate::hitbox::LandedBodyHit>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
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
            ambition_platformer2d_shared_tangle::markers::PlayerEntity,
            ActorFaction::Player,
            ambition_platformer2d_core::BodyKinematics {
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
            ambition_platformer2d_core::BodyOffense::default(),
            ambition_platformer2d_core::BodyMotionFacts::default(),
            ambition_platformer2d_core::BodyShieldState::default(),
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

/// ⛔ A move event authored AT the start of the move must still fire.
///
/// The player's swipe is `windup_s: 0.0` on purpose — "the arc and the swing cue
/// all land on the frame of the press" — which puts its SFX event at
/// `at_s == 0.0`. `MovePlayback::new_at` pre-marks events as already fired so
/// that SEEKING past them does not retro-fire them, and it used `at_s <= t0`,
/// so at `t0 == 0.0` the swing event was fired-before-it-began. The player's
/// swing was silent from 2026-07-26 until 2026-08-02 and no test saw it.
///
/// ⚠ nothing caught it because every fixture in this file authors a NON-ZERO
/// event time — `one_tick_sfx_move` below uses `0.01`. The boundary was the one
/// value the production config actually uses.
#[test]
fn an_event_authored_at_the_first_instant_is_not_pre_fired() {
    let mut spec = simple_melee(&SimpleMeleeParams {
        windup_s: 0.0,
        ..SimpleMeleeParams::default()
    });
    spec.events = vec![MoveEvent {
        at_s: 0.0,
        kind: MoveEventKind::Sfx {
            cue: "player.robot.slash.air".to_string(),
        },
    }];
    let playback = MovePlayback::new(spec, 1.0);
    assert!(
        !playback.fired[0],
        "an event at t=0 was pre-marked fired, so it can never sound"
    );
}

fn one_tick_sfx_move(cue: &str) -> MoveSpec {
    let mut spec = simple_melee(&SimpleMeleeParams::default());
    spec.events = vec![MoveEvent {
        at_s: 0.01,
        kind: MoveEventKind::Sfx {
            cue: cue.to_string(),
        },
    }];
    spec
}

/// Authored move events capture their CHARACTER PROVIDER at the move clock,
/// before the event enters the external-effect quarantine. A crossover match
/// can therefore play Sanic and Mary-O cues under one session owner without
/// guessing from whichever provider supplies the session's background music.
#[test]
fn move_events_capture_character_provider_presentation_sources() {
    let (mut app, _victim) = app_with_victim();
    app.insert_resource(
        ambition_characters::actor::character_catalog::CharacterCatalogOwners(
            std::collections::BTreeMap::from([
                ("sanic".to_string(), "sanic".to_string()),
                ("mary_o".to_string(), "mary_o".to_string()),
            ]),
        ),
    );

    let worn = spawn_attacker(
        &mut app,
        ae::Vec2::new(70.0, 100.0),
        ae::Vec2::new(16.0, 24.0),
        one_tick_sfx_move("sanic.spin"),
    );
    app.world_mut()
        .entity_mut(worn)
        .insert(ambition_characters::actor::WornCharacter::new("sanic"));

    let actor = spawn_attacker(
        &mut app,
        ae::Vec2::new(170.0, 100.0),
        ae::Vec2::new(16.0, 24.0),
        one_tick_sfx_move("mary_o.jump"),
    );
    app.world_mut()
        .entity_mut(actor)
        .insert(crate::components::CombatTuning {
            sprite_character_id: Some("mary_o".to_string()),
            ..Default::default()
        });

    app.update();

    let messages = app.world().resource::<Messages<MoveEventMessage>>();
    let mut cursor = messages.get_cursor();
    let sources: std::collections::BTreeSet<String> = cursor
        .read(messages)
        .map(|message| message.presentation_source.as_str().to_string())
        .collect();
    assert_eq!(
        sources,
        std::collections::BTreeSet::from(["mary_o".to_string(), "sanic".to_string()])
    );
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
    if let VfxMessage::Slash {
        kind, pose, shape, ..
    } = slashes[0]
    {
        assert_eq!(*kind, ambition_vfx::vfx::SlashKind::Arc);
        assert_eq!(*pose, ambition_vfx::vfx::SlashPose::Side);
        let ae::SwingShape::Sweep {
            origin,
            dir,
            length,
            near_half,
            far_half,
        } = *shape
        else {
            panic!("a forward blade arc is a sweep, got {shape:?}");
        };
        assert!(
            dir.x > 0.0,
            "the slash points along the strike (facing +x), got {dir:?}",
        );
        // ⚠ BODY-LOCAL. The cue carries the swing in the attacker's frame so
        // presentation can re-place it on a moving body every frame; a
        // world-space origin here would mean the effect is nailed to the ground
        // while the hitbox that drew it follows the owner.
        assert!(
            origin.length() < 200.0,
            "the origin is relative to the attacker, not the world: {origin:?}",
        );
        // The cue carries the SWING's own proportions, not a bounding square.
        // `test_blade_resolver`'s blade spans 2.4x the body half-width forward
        // and 2x the body's 0.4-height across — a 1.5:1 shape — and the cue
        // should say 1.5:1. The scalar this replaced could only ever say 1:1.
        // This is the assertion that fails if anyone reduces the volume to
        // `.bounds()` on the way out again.
        let aspect = length / (far_half * 2.0);
        assert!(
            (aspect - 1.5).abs() < 0.05,
            "the quad takes the blade's proportions (expected ~1.5:1): \
             length {length}, width {}",
            far_half * 2.0,
        );
        assert_eq!(
            near_half, far_half,
            "this fixture blade is a nosed rectangle, not a flaring arc — it \
             does not widen, and the cue should not invent a flare",
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

/// The AERIALS bind `air_*` clips, not `attack_air*` — that rename is what lets
/// them resolve their authored hit polys — and the pose match only knew the
/// grounded names, so the up-air and the down-air drew the horizontal crescent.
/// Rotation hid it: the art pointed the right way, and was the wrong art.
///
/// Restore either arm below to `_ => Side` and this goes red; that is the only
/// thing standing between the aerials and one generic swoosh.
#[test]
fn the_aerials_select_their_own_slash_pose_not_the_side_arc() {
    for (clip, expected) in [
        ("air_up", ambition_vfx::vfx::SlashPose::Up),
        ("air_down", ambition_vfx::vfx::SlashPose::Down),
    ] {
        let (mut app, _victim) = app_with_victim();
        let mut spec = simple_melee(&SimpleMeleeParams::default());
        spec.clip.clip = clip.to_string();
        spec.clip.fallbacks.clear();
        spawn_attacker(
            &mut app,
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(30.0, 48.0),
            spec,
        );
        run_seconds(&mut app, 0.14);
        let cap = app.world().resource::<Captured>();
        let slash = cap
            .slashes
            .iter()
            .find(|m| matches!(m, VfxMessage::Slash { .. }))
            .unwrap_or_else(|| panic!("the `{clip}` attack should emit a slash VFX"));
        if let VfxMessage::Slash { pose, .. } = slash {
            assert_eq!(*pose, expected, "`{clip}` draws its own row");
        }
    }
}

#[test]
fn upward_attack_selects_the_upward_slash_pose() {
    let (mut app, _victim) = app_with_victim();
    let mut spec = simple_melee(&SimpleMeleeParams::default());
    spec.clip.clip = "attack_up".to_string();
    spec.clip.fallbacks.clear();
    spawn_attacker(
        &mut app,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(30.0, 48.0),
        spec,
    );
    run_seconds(&mut app, 0.14);
    let cap = app.world().resource::<Captured>();
    let slash = cap
        .slashes
        .iter()
        .find(|m| matches!(m, VfxMessage::Slash { .. }))
        .expect("the upward attack should emit a slash VFX");
    if let VfxMessage::Slash { pose, .. } = slash {
        assert_eq!(*pose, ambition_vfx::vfx::SlashPose::Up);
    }
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
        let mut frame =
            ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default();
        frame.publish_resolved_frame(ae::MotionFrame::from_direction(gravity, 900.0));
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
                            landing_lag_s: None,
                            autocancel_after_s: None,
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
        let knockback = match hb.knockback {
            crate::strike::HitboxKnockback::LaunchSpeed { base, growth } => {
                assert_eq!(growth, 0.0, "charge fixture authors no growth");
                base
            }
            other => panic!("moveset melee must author a launch speed, got {other:?}"),
        };
        (hb.damage, knockback)
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

/// ⭐⭐ **A LIVE STRIKE KEEPS THE ORIENTATION ITS MOVE STARTED WITH.**
///
/// Locomotion facing is mutable and changes for reasons that have nothing to do
/// with the swing in flight — a stick nudge, a wall contact, a brain retarget.
/// If an active hitbox read it, a fighter could turn mid-animation and have the
/// blade teleport to the other side of its own body, hitting somebody it never
/// swung at and missing the one it did.
///
/// `MovePlayback` captures `facing` at move start and every strike volume is
/// mirrored through THAT, never through `BodyKinematics::facing`. The mechanism
/// already existed; nothing pinned it, which is how it would have been
/// refactored away by a well-meaning "use the body's facing, it's right there".
///
/// ⛔ **the vacuity guard is the second half**: the body's facing must actually
/// have flipped, or this asserts nothing.
#[test]
fn a_live_strike_keeps_the_facing_its_move_started_with() {
    let (mut app, victim) = app_with_victim();
    // The victim sits to the RIGHT of the attacker (`app_with_victim` places it
    // at x=128; the attacker spawns at x=100 looking +x).
    let attacker = spawn_attacker(
        &mut app,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(12.0, 18.0),
        swat(),
    );

    // Into the startup window, before the Active edge — the strike is committed
    // but no volume is live yet, which is exactly when a turn is most tempting.
    run_seconds(&mut app, 0.10);
    assert!(
        app.world().resource::<Captured>().hits.is_empty(),
        "the fixture must still be winding up, or the turn below happens too late"
    );

    // Turn the BODY around, the way locomotion would.
    {
        let mut kin = app
            .world_mut()
            .get_mut::<ae::BodyKinematics>(attacker)
            .unwrap();
        kin.facing = -1.0;
    }
    // ⛔ the vacuity half: the turn really happened, and the move did NOT adopt it.
    assert_eq!(
        app.world()
            .get::<ae::BodyKinematics>(attacker)
            .unwrap()
            .facing,
        -1.0,
        "the body must genuinely be facing the other way"
    );
    assert_eq!(
        app.world().get::<MovePlayback>(attacker).unwrap().facing,
        1.0,
        "the MOVE keeps the orientation it committed to — this is the snapshot"
    );

    run_seconds(&mut app, 0.40);
    let cap = app.world().resource::<Captured>();
    assert_eq!(
        cap.hits.len(),
        1,
        "the swing still lands on the body it was aimed at, despite the turn"
    );
    assert_eq!(cap.hits[0].target, crate::events::HitTarget::Body(victim));
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
    app.add_message::<crate::hitbox::LandedBodyHit>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
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
            resolve_attack_gestures,
            trigger_moveset_moves,
            advance_move_playback,
            apply_hitbox_damage,
            capture,
        )
            .chain(),
    );
    app.world_mut().spawn((
        ambition_platformer2d_shared_tangle::markers::PlayerEntity,
        ActorFaction::Player,
        ambition_platformer2d_core::BodyKinematics {
            pos: ae::Vec2::new(128.0, 100.0),
            size: ae::Vec2::new(28.0, 46.0),
            facing: -1.0,
            ..Default::default()
        },
        ae::CenteredAabb::from_center_size(ae::Vec2::new(128.0, 100.0), ae::Vec2::new(28.0, 46.0)),
        ambition_platformer2d_core::BodyOffense::default(),
        ambition_platformer2d_core::BodyMotionFacts::default(),
        ambition_platformer2d_core::BodyShieldState::default(),
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

/// Directional special selection uses the same facing-relative resolver as
/// attacks, while preserving the base `special` fallback for existing content.
#[test]
fn a_forward_special_selects_the_directional_move() {
    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.init_resource::<WorldTime>();
    app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
    app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
    app.add_systems(
        Update,
        (resolve_attack_gestures, trigger_moveset_moves).chain(),
    );

    let make_move = |id: &str| MoveSpec {
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.3,
        windows: vec![],
        events: vec![],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        landing_lag_s: None,
        autocancel_after_s: None,
    };
    let moveset = MovesetContract {
        verbs: std::collections::BTreeMap::from([
            (SPECIAL_VERB.to_string(), "neutral_special".to_string()),
            ("special_forward".to_string(), "forward_special".to_string()),
        ]),
        moves: vec![make_move("neutral_special"), make_move("forward_special")],
    };
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.special_pressed = true;
    frame.attack_axis = ae::LocalAxes::X;
    let body = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                facing: 1.0,
                ..Default::default()
            },
            ActorFaction::Enemy,
            ActorMoveset(moveset),
            ActorControl(frame),
        ))
        .id();

    app.update();
    assert_eq!(
        app.world().get::<MovePlayback>(body).unwrap().spec.id,
        "forward_special"
    );
}

fn gesture_test_move(id: &str) -> MoveSpec {
    MoveSpec {
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.3,
        windows: vec![],
        events: vec![],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        landing_lag_s: None,
        autocancel_after_s: None,
    }
}

fn trigger_gesture_move(include_smash: bool, strong: bool) -> String {
    let mut app = App::new();
    app.add_systems(
        Update,
        (resolve_attack_gestures, trigger_moveset_moves).chain(),
    );
    let mut verbs = std::collections::BTreeMap::from([(
        "attack_forward".to_string(),
        "forward_tilt".to_string(),
    )]);
    let mut moves = vec![gesture_test_move("forward_tilt")];
    if include_smash {
        verbs.insert("smash_forward".to_string(), "forward_smash".to_string());
        moves.push(gesture_test_move("forward_smash"));
    }

    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    // Above the directional deadzone but below the flick threshold, so this is
    // a tilt unless the explicit device-independent strong hint is present.
    frame.attack_axis = ae::LocalAxes::new(0.6, 0.0);
    frame.melee_strong_hint = strong;
    let body = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                facing: 1.0,
                ..Default::default()
            },
            ActorMoveset(MovesetContract { verbs, moves }),
            ActorControl(frame),
        ))
        .id();
    app.update();
    app.world()
        .get::<MovePlayback>(body)
        .expect("the semantic attack edge starts a move")
        .spec
        .id
        .clone()
}

#[test]
fn attack_gesture_selects_tilt_or_smash_from_the_same_direction() {
    assert_eq!(trigger_gesture_move(true, false), "forward_tilt");
    assert_eq!(trigger_gesture_move(true, true), "forward_smash");
}

#[test]
fn strong_attack_falls_back_to_the_ordinary_directional_move() {
    assert_eq!(
        trigger_gesture_move(false, true),
        "forward_tilt",
        "existing movesets need not author a smash vocabulary"
    );
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
    app.add_systems(
        Update,
        (resolve_attack_gestures, trigger_moveset_moves).chain(),
    );
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
        landing_lag_s: None,
        autocancel_after_s: None,
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
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
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
            presentation_source: ambition_sfx::PresentationSourceId::new("sanic.moves"),
            kind: MoveEventKind::Sfx {
                cue: "pca.signature".into(),
            },
        });
    app.world_mut()
        .resource_mut::<Messages<MoveEventMessage>>()
        .write(MoveEventMessage {
            owner,
            move_id: "sig".into(),
            presentation_source: ambition_sfx::PresentationSourceId::unscoped(),
            kind: MoveEventKind::Effect(EffectRef {
                key: "pca_glider".into(),
                // A1: authored params must SURVIVE the bridge so the keyed
                // technique can hydrate them.
                params: ambition_entity_catalog::ParamValue::parse("(rise: 320.0)")
                    .expect("param RON parses"),
            }),
        });
    app.update();

    let sfx: Vec<ambition_sfx::OwnedSfxMessage> = app
        .world_mut()
        .resource_mut::<Messages<ambition_sfx::OwnedSfxMessage>>()
        .drain()
        .collect();
    assert_eq!(sfx.len(), 1, "the Sfx event played one sound");
    assert_eq!(sfx[0].source.as_str(), "sanic.moves");
    assert!(
        matches!(sfx[0].request, SfxMessage::Play { pos, .. } if pos == ae::Vec2::new(42.0, 7.0)),
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

/// **A move started with an UPWARD aim fires upward, not sideways.**
///
/// ⛔ **the aimed case was the broken one, and the fallback hid it.**
/// `ActorControl.fire` is an EDGE cleared every tick, and a ranged move has
/// startup — so by the time its authored fire frame arrives the request that
/// triggered it is gone, and the handler fell through to the body's horizontal
/// FACING. That repairs left-versus-right (queue D8) and flattens every aim that
/// was up, down or diagonal (GPT 5.6 review, 2026-08-04).
///
/// ⚠ **the sibling test above cannot see this**: it supplies a live `fire` on
/// the event frame, which is the tier that always worked. The distinguishing
/// input is a playback whose aim was captured at START with NO live edge now —
/// so that is what this drives.
#[test]
fn a_move_started_aiming_up_fires_up_after_its_request_is_cleared() {
    use ambition_characters::brain::action_set::{ActionSet, RangedActionSpec};
    use ambition_characters::brain::{ActorActionMessage, ActorControl};
    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ActorActionMessage>();
    app.add_systems(Update, dispatch_move_events);

    let up = ae::Vec2::new(0.0, -1.0);
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
                ranged: Some(RangedActionSpec::bolt(240.0, 3)),
                ..Default::default()
            },
            // No live fire edge — it was cleared during the move's startup, which
            // is the whole point.
            ActorControl::default(),
            // Any authored move will do — the aim is what is under test, and
            // `swat()` is the fixture the rest of this file already builds.
            MovePlayback::new(swat(), 1.0)
                .with_aim(Some((up, ae::GameplayFramePolicy::WorldSpace))),
        ))
        .id();
    app.world_mut()
        .resource_mut::<Messages<MoveEventMessage>>()
        .write(MoveEventMessage {
            owner,
            move_id: "fire".into(),
            presentation_source: ambition_sfx::PresentationSourceId::unscoped(),
            kind: MoveEventKind::Ranged,
        });
    app.update();

    let acts: Vec<ActorActionMessage> = app
        .world_mut()
        .resource_mut::<Messages<ActorActionMessage>>()
        .drain()
        .collect();
    assert_eq!(acts.len(), 1, "the Ranged event bridged to one action");
    match &acts[0].request {
        ActionRequest::Ranged {
            dir, dir_policy, ..
        } => {
            assert_eq!(
                *dir, up,
                "the move was started aiming UP and fired {dir:?} — the startup \
                 cleared the request and the shot fell back to facing"
            );
            assert!(
                matches!(dir_policy, ae::GameplayFramePolicy::WorldSpace),
                "the aim's FRAME travels with it: a world-space up is not a \
                 body-local up under non-default gravity"
            );
        }
        other => panic!("expected a Ranged action, got {other:?}"),
    }
}

/// Ranged subsumption (option A): a `MoveEventKind::Ranged` fire event BRIDGES to
/// the SAME `ActorActionMessage::Ranged` the flat `frame.fire` resolver emits —
/// carrying the owner's authored `ActionSet.ranged` spec and SAMPLING its LIVE
/// aim at the event frame — so the existing enemy-projectile consumer fires the
/// shot unchanged and a moveset shot still tracks a strafing target.
#[test]
fn move_event_dispatch_bridges_ranged_to_a_live_aimed_shot() {
    use ambition_characters::actor::control::ActorFireRequest;
    use ambition_characters::brain::action_set::{ActionSet, RangedActionSpec, RangedStyle};
    use ambition_characters::brain::{ActorActionMessage, ActorControl};
    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
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
                ranged: Some(RangedActionSpec::bolt(240.0, 3)),
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
            presentation_source: ambition_sfx::PresentationSourceId::unscoped(),
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
            assert!(matches!(
                spec,
                RangedActionSpec {
                    style: RangedStyle::Bolt,
                    damage: 3,
                    ..
                }
            ));
            assert_eq!(*origin, ae::Vec2::new(100.0, 50.0), "origin = owner pos");
            assert_eq!(*dir, ae::Vec2::new(0.6, -0.8), "dir SAMPLED from live aim");
        }
        other => panic!("expected ActionRequest::Ranged, got {other:?}"),
    }
}

/// **A body with no live aim fires the way it is FACING, not world-right.**
///
/// `frame.fire` is an edge — `clear_edges()` nulls it every tick — and a ranged
/// move has startup, so by the time its fire frame arrives the intent that
/// started it is usually gone. That fallback used to be a bare `(1.0, 0.0)`,
/// which `dir_to_world` resolves through the acceleration frame alone, so every
/// such shot went world-RIGHT whichever way the body looked. Reported from play
/// as "Maryo's fireball only shoots to her right, not the way she is facing".
#[test]
fn a_ranged_move_without_live_aim_fires_along_the_bodys_facing() {
    use ambition_characters::brain::action_set::{ActionSet, RangedActionSpec};
    use ambition_characters::brain::{ActorActionMessage, ActorControl};
    for facing in [-1.0f32, 1.0] {
        let mut app = App::new();
        app.add_message::<MoveEventMessage>();
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<ActorActionMessage>();
        app.add_systems(Update, dispatch_move_events);

        // No `control.0.fire`: the intent was cleared before the fire frame.
        let owner = app
            .world_mut()
            .spawn((
                ae::BodyKinematics {
                    pos: ae::Vec2::new(100.0, 50.0),
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(16.0, 24.0),
                    facing,
                },
                ActionSet {
                    ranged: Some(RangedActionSpec::bolt(240.0, 3)),
                    ..Default::default()
                },
                ActorControl::default(),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Messages<MoveEventMessage>>()
            .write(MoveEventMessage {
                owner,
                move_id: "fire".into(),
                presentation_source: ambition_sfx::PresentationSourceId::unscoped(),
                kind: MoveEventKind::Ranged,
            });
        app.update();

        let acts: Vec<ActorActionMessage> = app
            .world_mut()
            .resource_mut::<Messages<ActorActionMessage>>()
            .drain()
            .collect();
        match &acts[0].request {
            ActionRequest::Ranged { dir, .. } => assert_eq!(
                dir.x.signum(),
                facing,
                "a body facing {facing} fired along {dir:?}"
            ),
            other => panic!("expected ActionRequest::Ranged, got {other:?}"),
        }
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

    let contract = build_actor_moveset(None, None, Some(&RangedActionSpec::bolt(240.0, 3)), None)
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
    app.add_systems(
        Update,
        (resolve_attack_gestures, trigger_moveset_moves).chain(),
    );
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
        Some(&RangedActionSpec::rock(300.0, 1)),
        None,
    )
    .expect("melee + ranged → a moveset");
    let fire = contract.move_for_verb(RANGED_VERB).unwrap().clone();
    let attack = contract.move_for_verb(ATTACK_VERB).unwrap().clone();
    // ⭐ **AND A SPECIAL, the third input** (added 2026-08-13, GPT 5.6 review).
    // `MovePlayback` is not melee — it carries ranged AND specials — and a patch
    // that read "a playback exists" as "this body is attacking" was proposed and
    // reverted the same day (ledger D107). Ranged was already poisoned here;
    // special took the same code path and was not, so the third arm of the rule
    // was untested.
    let mut special = attack.clone();
    special.id = "special".to_string();

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
    // Playing a SPECIAL → no swing, for the same reason as the ranged shot.
    let specialing = app
        .world_mut()
        .spawn((
            MovesetMelee,
            BodyMelee::default(),
            MovePlayback::new(special, 1.0),
        ))
        .id();
    app.update();
    assert!(
        app.world()
            .get::<BodyMelee>(specialing)
            .unwrap()
            .swing
            .is_none(),
        "a body mid-SPECIAL reads as mid-swing — `is_melee_swing_move` is \
         classifying by playback presence rather than by verb, which is exactly \
         the phantom swing this projection exists to prevent"
    );
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

/// Routing derives from the melee VERB family, not from requiring a base
/// `attack` entry. A directional-only fighter is a valid moveset and must still
/// receive the presentation marker.
#[test]
fn a_directional_only_smash_route_derives_the_melee_marker() {
    let smash = gesture_test_move("forward_smash");
    let contract = MovesetContract {
        verbs: std::collections::BTreeMap::from([("smash_forward".to_string(), smash.id.clone())]),
        moves: vec![smash],
    };

    let mut app = App::new();
    app.add_systems(Update, reconcile_moveset_routing_markers);
    let body = app.world_mut().spawn(ActorMoveset(contract)).id();
    app.update();
    assert!(
        app.world().get::<MovesetMelee>(body).is_some(),
        "any attack/smash verb family routes through the melee read-model"
    );
}

/// A dedicated smash verb is still a melee read-model even though its move id
/// need not begin with `attack`. Gameplay no longer depends on this projection,
/// but animation/movement observers must still see the body as mid-swing.
#[test]
fn a_smash_verb_projects_the_melee_read_model() {
    let smash = gesture_test_move("forward_smash");
    let contract = MovesetContract {
        verbs: std::collections::BTreeMap::from([("smash_forward".to_string(), smash.id.clone())]),
        moves: vec![smash.clone()],
    };

    let mut app = App::new();
    app.add_systems(Update, project_moveset_melee_to_body_melee);
    let body = app
        .world_mut()
        .spawn((
            MovesetMelee,
            BodyMelee::default(),
            ActorMoveset(contract),
            MovePlayback::new(smash, 1.0),
        ))
        .id();

    app.update();
    assert!(
        app.world().get::<BodyMelee>(body).unwrap().swing.is_some(),
        "a smash-bound move must present as a melee swing"
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
    app.add_systems(
        Update,
        (resolve_attack_gestures, trigger_moveset_moves).chain(),
    );
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
        motion_scale: 1.0,
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
        grants: vec![EquipmentGrant::Ranged(RangedActionSpec::bolt(420.0, 6))],
        ..Default::default()
    };
    let mut worn = WornEquipment::default();
    worn.equip(flower);

    // A peaceful body: no ranged verb in its derived moveset.
    let actions = ActionSet::peaceful();
    let before = build_actor_moveset(None, actions.melee.as_ref(), actions.ranged.as_ref(), None);
    assert!(
        before.map_or(true, |m| m.move_for_verb(RANGED_VERB).is_none()),
        "no flower, no ranged move"
    );

    // Equip the flower: the grant confers the ranged verb, and the moveset
    // derivation turns it into a fireable move.
    let mut equipped = ActionSet::peaceful();
    apply_equipment_grants(&mut equipped, &worn);
    let moveset = build_actor_moveset(
        None,
        equipped.melee.as_ref(),
        equipped.ranged.as_ref(),
        None,
    )
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
    let moveset = build_actor_moveset(None, after.melee.as_ref(), after.ranged.as_ref(), None);
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
        grants: vec![EquipmentGrant::Ranged(RangedActionSpec::bolt(420.0, 6))],
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

/// ⭐⭐ **An aerial is a COMMITMENT: land mid-move and pay for it, land after
/// the auto-cancel point and land clean.**
///
/// The platform-fighter rule that makes spacing an aerial a decision rather
/// than a free action. Three cases, and the third is the poison — a move that
/// authors nothing must land exactly the way it always did, because this
/// mechanic is opt-in and every shipped move has opted out.
#[test]
fn landing_out_of_an_aerial_costs_its_authored_lag_unless_it_autocancelled() {
    use ambition_platformer2d_core::BodyGroundState;

    // (authored lag, authored autocancel, move clock at touchdown) -> lag paid
    let paid = |lag: Option<f32>, autocancel: Option<f32>, t: f32| -> f32 {
        let mut app = App::new();
        app.add_systems(Update, super::resolve_aerial_landings);
        let mut spec = swat();
        spec.landing_lag_s = lag;
        spec.autocancel_after_s = autocancel;
        let mut playback = MovePlayback::new(spec, 1.0);
        playback.t = t;
        let body = app
            .world_mut()
            .spawn((
                playback,
                BodyGroundState::default(),
                ambition_characters::actor::BodyCombat::default(),
            ))
            .id();
        // Tick one: still airborne, so no edge.
        app.update();
        assert!(
            app.world().get::<MovePlayback>(body).is_some(),
            "an airborne move must survive a tick that is not a landing"
        );
        // Tick two: touch down.
        app.world_mut()
            .get_mut::<BodyGroundState>(body)
            .unwrap()
            .on_ground = true;
        app.update();
        app.world()
            .get::<ambition_characters::actor::BodyCombat>(body)
            .expect("the body survives its own landing")
            .landing_lag_timer
    };

    // Landed early, inside the commitment: the authored lag is owed.
    assert!((paid(Some(0.25), Some(0.50), 0.10) - 0.25).abs() < 1e-6);
    // Landed past the auto-cancel point: clean.
    assert_eq!(paid(Some(0.25), Some(0.50), 0.60), 0.0);
    // No auto-cancel authored: the lag applies whenever the move is running.
    assert!((paid(Some(0.25), None, 0.60) - 0.25).abs() < 1e-6);
    // ⛔ **the poison.** A move that authors no landing lag lands the way every
    // move in the game lands today. Opt-in means opt-in.
    assert_eq!(paid(None, None, 0.10), 0.0);
    assert_eq!(paid(None, Some(0.50), 0.10), 0.0);
}

/// ⛔ **A move begun ON THE GROUND never "lands".**
///
/// The landing EDGE is what costs, not the grounded state — otherwise a jab, a
/// down-tilt and every other grounded move would pay an aerial's lag on the
/// frame it started. This is why the previous grounded-ness rides on the
/// playback rather than being re-derived from the body.
#[test]
fn a_grounded_move_never_pays_landing_lag() {
    use ambition_platformer2d_core::BodyGroundState;

    let mut app = App::new();
    app.add_systems(Update, super::resolve_aerial_landings);
    let mut spec = swat();
    spec.landing_lag_s = Some(0.25);
    let body = app
        .world_mut()
        .spawn((
            MovePlayback::new(spec, 1.0),
            BodyGroundState {
                on_ground: true,
                ..Default::default()
            },
            ambition_characters::actor::BodyCombat::default(),
        ))
        .id();
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<ambition_characters::actor::BodyCombat>(body)
            .unwrap()
            .landing_lag_timer,
        0.0,
        "standing on the floor is not landing"
    );
    assert!(
        app.world().get::<MovePlayback>(body).is_some(),
        "and the grounded move is not cancelled by the ground it started on"
    );
}

/// A three-keyframe hitbox track: one swing whose shape sweeps forward across
/// its active portion, authored as contiguous `Active` windows.
///
/// Each keyframe reaches further than the last, so the victim standing at the
/// far end is only overlapped by the LAST one — which is the point of a track,
/// and also what makes the double-hit failure mode invisible to a test whose
/// victim stands under every segment.
fn swept_track(reaches: [f32; 3], segment_s: f32) -> MoveSpec {
    let windup = 0.05;
    let mut spec = simple_melee(&SimpleMeleeParams {
        windup_s: windup,
        active_s: segment_s * 3.0,
        recover_s: 0.05,
        ..Default::default()
    });
    let volume = |reach: f32| HitVolume {
        shape: ambition_entity_catalog::VolumeShape::Rect {
            offset: (reach, 0.0),
            half_extents: (8.0, 20.0),
        },
        damage: 5,
        knockback: 0.0,
        knockback_growth: 0.0,
        launch_dir: None,
        on_hit: None,
        // ⚠ no `vfx` tag: a bladed volume would resolve the fixture manifest
        // blade and every keyframe would swing the SAME authored hull, which is
        // exactly the geometry this test varies.
        vfx: None,
        hit_sfx: None,
    };
    spec.windows = vec![MoveWindow {
        start_s: 0.0,
        end_s: windup,
        tag: WindowTag::Startup,
        volumes: vec![],
        sustain_effect: None,
        motion_scale: 1.0,
    }];
    for (i, reach) in reaches.iter().enumerate() {
        let start = windup + segment_s * i as f32;
        spec.windows.push(MoveWindow {
            start_s: start,
            end_s: start + segment_s,
            tag: WindowTag::Active,
            volumes: vec![volume(*reach)],
            sustain_effect: None,
            motion_scale: 1.0,
        });
    }
    let last = windup + segment_s * 3.0;
    spec.windows.push(MoveWindow {
        start_s: last,
        end_s: spec.duration_s,
        tag: WindowTag::Recovery,
        volumes: vec![],
        sustain_effect: None,
        motion_scale: 1.0,
    });
    spec.events.clear();
    spec
}

/// ⭐⭐ **one swing is one hit, however many keyframes it was sampled at.**
///
/// The victim sits inside every segment of the track, so each of the three
/// windows spawns a box that overlaps it. Without the contiguity handoff each
/// box carries its own fresh `HitboxHits` and the swing lands THREE times —
/// a four-keyframe sword arc would deal quadruple damage, which is what makes
/// authored tracks unusable rather than merely wrong.
#[test]
fn a_contiguous_hitbox_track_lands_one_hit_per_victim() {
    let (mut app, victim) = app_with_victim();
    // Victim at x=128, attacker at x=100 with a 30-wide body: reaches of
    // 20/26/32 px all overlap the victim's 28-wide box.
    spawn_attacker(
        &mut app,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(30.0, 48.0),
        swept_track([20.0, 26.0, 32.0], 0.05),
    );
    run_seconds(&mut app, 0.30);

    let hits: Vec<_> = app
        .world()
        .resource::<Captured>()
        .hits
        .iter()
        .filter(|e| e.target == crate::events::HitTarget::Body(victim))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "⛔ a swept track re-hit once per keyframe: {} hits",
        hits.len()
    );
}

/// ⛔ **POISON: a GAP still rehits.** A genuine multi-hit move — a drill, a
/// rapid jab — is authored as Active windows with space between them, because
/// that is physically what it is: the box goes away and comes back. If the
/// handoff carried across a gap too, every multi-hit move in the game would
/// silently become single-hit, and this whole mechanism would be a damage nerf
/// wearing a bug fix's clothes.
#[test]
fn a_gap_between_active_windows_is_a_fresh_strike() {
    let (mut app, victim) = app_with_victim();
    let mut spec = swept_track([20.0, 20.0, 20.0], 0.05);
    // Push the second and third keyframes later, opening a gap after the first.
    for window in spec.windows.iter_mut() {
        if matches!(window.tag, WindowTag::Active) && window.start_s > 0.05 {
            window.start_s += 0.05;
            window.end_s += 0.05;
        }
    }
    spec.duration_s += 0.10;
    spawn_attacker(
        &mut app,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(30.0, 48.0),
        spec,
    );
    run_seconds(&mut app, 0.45);

    let hits = app
        .world()
        .resource::<Captured>()
        .hits
        .iter()
        .filter(|e| e.target == crate::events::HitTarget::Body(victim))
        .count();
    assert!(
        hits >= 2,
        "⛔ a gap means the strike ENDED; the next window is a new one, got {hits} hits"
    );
}
