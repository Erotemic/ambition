use super::*;
// Named here because this is where they are used.
use crate::events::HitEvent;
use crate::hitbox::apply_hitbox_damage;
use ambition_characters::brain::action_set::MeleeActionSpec;
use ambition_entity_catalog::{ClipBinding, EffectRef, HitVolume, MoveEvent};
use ambition_sfx::SfxMessage;
use ambition_vfx::vfx::DebrisBurstMessage;
use ambition_vfx::vfx::VfxMessage;
use bevy::prelude::*;

/// The attack direction is facing-relative, not screen-relative. The aim axis arrives
/// screen-local (`+x` = screen-right), but a forward press must read `Forward` no matter which way
/// you face — and a press toward your BACK must read `Back`.
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

    // Facing LEFT (-1): the mirror.
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

/// What this test suite pretends a renderer can draw.
///
///  a STUB, and deliberately so: the real answer is the rows of the shipped FX
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
            MoveEventKind::Vfx { effect, .. } if effect == "shockwave"
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

/// CM5: the content-free dispatcher turns a `Vfx` event into a PAIRED effect
/// request at the owner's position.
///
/// That bypass is what made every authored burst a hand-written PAIR across fourteen fighter
/// tables; 74 of 145 authored `sfx(…)` calls existed only to restate a sound this request
/// derives.
///
///  the sound is deliberately NOT asserted here, and the reason is a crate
/// boundary worth stating: `process_fx_requests` — which fans a request into the
/// effect plus its cue — lives in `ambition_render` and is installed by the
/// host, not by this crate. So the honest claim at THIS seam is *"the dispatcher
/// asks for the pairing, with the right name, position and owner"*; that the
/// pairing then happens is `a_requests_presentation_source_reaches_the_cue_it_pairs`
/// next to the fan-out itself.
#[test]
fn move_event_dispatch_asks_for_a_paired_cosmetic_effect() {
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct Seen(Option<ambition_vfx::FxRequest>);

    fn capture(mut requests: MessageReader<ambition_vfx::FxRequest>, mut seen: ResMut<Seen>) {
        for request in requests.read() {
            seen.0 = Some(request.clone());
        }
    }

    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_vfx::FxRequest>();
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
            world_offset: ae::Vec2::ZERO,
            owner,
            move_id: "smash".into(),
            presentation_source: ambition_sfx::PresentationSourceId::unscoped(),
            kind: MoveEventKind::Vfx {
                // This test is about the NAME reaching the wire.
                effect: "starburst".to_string(),
                at: (0.0, 0.0),
                scale: 1.0,
                sfx: None,
            },
            world_pose: ambition_vfx::FxPose::UPRIGHT,
        });
    app.update();
    let asked = app
        .world()
        .resource::<Seen>()
        .0
        .clone()
        .expect("the Vfx event asked for no effect at all");
    assert_eq!(
        asked.fx,
        ambition_vfx::FxId::new("starburst"),
        "the Vfx event put the authored NAME on the wire — no enum in between",
    );
    assert!(
        asked.sfx.is_none(),
        "the request named an OVERRIDE cue, so the move would still be dictating \
         its own sound instead of taking the one its art already addresses"
    );

    //  and the OTHER arm, because it is the one the corpus needs: ten shipped
    // effect rows pack their sound only as `<cue>.loop`, so a sustained burst
    // says which cue on the burst itself rather than pairing itself with a
    // second event. `sfx: None` above is "say what the art says"; this is "say
    // this instead".
    app.world_mut()
        .resource_mut::<Messages<MoveEventMessage>>()
        .write(MoveEventMessage {
            world_offset: ae::Vec2::ZERO,
            owner,
            move_id: "smash".into(),
            presentation_source: ambition_sfx::PresentationSourceId::unscoped(),
            kind: MoveEventKind::Vfx {
                effect: "starburst".to_string(),
                at: (0.0, 0.0),
                scale: 1.0,
                sfx: Some("vfx.explosion.starburst.loop".to_string()),
            },
            world_pose: ambition_vfx::FxPose::UPRIGHT,
        });
    app.update();
    assert_eq!(
        app.world()
            .resource::<Seen>()
            .0
            .as_ref()
            .and_then(|r| r.sfx),
        Some(ambition_sfx::SfxId::new("vfx.explosion.starburst.loop")),
        "an authored override never reached the request, so a sustained burst \
         could only be expressed as a hand-written pair again",
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
/// ONE MOVE USE STALES ONCE, WHATEVER IT CATCHES.
///
///  the false→true edge of `MovePlayback::landed_hit` already meant exactly
/// "this use connected" — it had to, for the OnHit/OnWhiff cancel windows — so
/// counting there needs no new state and no second system.
///
///  the second half is the poison. A recorder that simply refused to
/// record twice ever would satisfy the first assertion and break the mechanic:
/// staling is per USE, so throwing the same move again has to count again.
#[test]
fn a_swing_that_catches_two_bodies_stales_the_move_once() {
    let mut app = App::new();
    app.add_message::<crate::hitbox::LandedBodyHit>();
    app.add_systems(Update, mark_move_playback_landed_hits);

    let attacker = app
        .world_mut()
        .spawn((
            MovePlayback::new(swat(), 1.0),
            crate::stale::BodyStaleMoves::default(),
        ))
        .id();
    let hitbox = app.world_mut().spawn_empty().id();
    let hash = crate::stale::stale_move_hash("swat");

    let land_on = |app: &mut App, victims: usize| {
        for _ in 0..victims {
            let victim = app.world_mut().spawn_empty().id();
            app.world_mut().write_message(crate::hitbox::LandedBodyHit {
                hitbox,
                attacker,
                victim,
                volume: ae::CombatVolume::Circle {
                    center: ae::Vec2::ZERO,
                    radius: 8.0,
                },
                contact: ae::Vec2::ZERO,
            });
        }
        app.update();
    };
    let worn = |app: &App| {
        app.world()
            .get::<crate::stale::BodyStaleMoves>(attacker)
            .expect("the attacker kept its stale ring")
            .occurrences(hash)
    };

    // ONE swing, TWO bodies.
    land_on(&mut app, 2);
    assert_eq!(
        worn(&app),
        1,
        "a swing that caught two fighters staled the move twice — staling counts \
         move USES, and one swing is one use however many it reaches"
    );

    //  a SECOND use of the same move, and it must count again.
    app.world_mut()
        .entity_mut(attacker)
        .insert(MovePlayback::new(swat(), 1.0));
    land_on(&mut app, 1);
    assert_eq!(
        worn(&app),
        2,
        "throwing the same move a second time did not wear it further, so the \
         recorder is one-shot rather than per-use"
    );
}

/// Attaching it through `#[require]` on the moveset carrier is what lets it leave that bundle
/// without any spawn road having to remember it — and there are two of those roads, which is
/// exactly how a carry list starts.
#[test]
fn a_moveset_brings_its_own_stale_ring() {
    let mut app = App::new();
    let body = app.world_mut().spawn(ActorMoveset(Default::default())).id();
    assert!(
        app.world()
            .get::<crate::stale::BodyStaleMoves>(body)
            .is_some(),
        "inserting a moveset did not bring the stale ring, so staling is silently \
         off for every body whose spawn road does not name it"
    );
    //  and the floor: a body with no moveset does not pay for one.
    let bare = app.world_mut().spawn_empty().id();
    assert!(
        app.world()
            .get::<crate::stale::BodyStaleMoves>(bare)
            .is_none(),
        "a body that cannot attack is carrying combat history anyway"
    );
}

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
    //  victims only. A body-owned melee also publishes the unresolved half
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

/// A body-local blade, which is what the seam returns: `+x` forward, `+y`
/// toward the feet, origin at the body centre. The strike path places it.
fn test_blade_resolver(
    _catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    _cid: Option<&str>,
    animation: &str,
    collision: ae::Vec2,
    _clip_elapsed: Option<f32>,
) -> Option<ae::CombatVolume> {
    (animation == "attack_side").then(|| {
        let hx = collision.x * 0.8;
        let hy = collision.y * 0.4;
        ae::CombatVolume::convex(vec![
            ae::Vec2::new(-hx, -hy),
            ae::Vec2::new(hx, -hy),
            ae::Vec2::new(hx * 1.4, 0.0),
            ae::Vec2::new(hx, hy),
            ae::Vec2::new(-hx, hy),
        ])
    })
}

fn app_with_victim() -> (App, Entity) {
    // The authored-blade path resolves through the install seam exactly like production.
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

/// An attacker mid-move whose SMASH CHARGE is already held to its authored
/// maximum — the state a fully-charged release is in.
///
/// ⛔ `spawn_attacker` above inserts an UNCHARGED playback, and that is correct
/// for every other fixture here. A charge test built on it never enters charge
/// mode at all: before the payoff had one authority, such a fixture still read
/// a full multiplier off the move's timeline, so it passed while asserting a
/// mechanism it never exercised.
fn spawn_charged_attacker(app: &mut App, pos: ae::Vec2, body: ae::Vec2, spec: MoveSpec) -> Entity {
    let mut playback = MovePlayback::new(spec, 1.0).charged_by_smash_gesture(true);
    // A move authoring no payoff resolves no policy and stays uncharged, which
    // is the parity half of the caller's question rather than a broken fixture.
    if let Some(charge) = playback.charge.as_mut() {
        charge.held_s = charge.policy.max_hold_s;
    }
    app.world_mut()
        .spawn((
            ae::CenteredAabb::new(pos, body),
            ae::BodyKinematics {
                pos,
                vel: ae::Vec2::ZERO,
                size: body,
                facing: 1.0,
            },
            ActorFaction::Enemy,
            playback,
        ))
        .id()
}

fn run_seconds(app: &mut App, seconds: f32) {
    let steps = (seconds / 0.016).ceil() as usize;
    for _ in 0..steps {
        app.update();
    }
}

///  A move event authored AT the start of the move must still fire.
///
///  nothing caught it because every fixture in this file authors a NON-ZERO
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
        //  BODY-LOCAL. The cue carries the swing in the attacker's frame so
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

/// core: the authored timeline drives the REAL damage path. No hit during startup; the active
/// window spawns the volume and the standing victim takes the authored damage; the window's
/// exit despawns the box; move completion removes the component. The timed event fires once.
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

/// B1 (fable review §B1): a moveset volume's authored offset is BODY-LOCAL (side, down); the
/// spawned `FollowOwner` hitbox must rotate it into the owner's gravity frame at spawn, so the SAME
/// move lands its box in the same BODY-relative place under every gravity.
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
        // ⛔ THE FIXTURE HAS TO ENTER CHARGE MODE. This used to spawn an
        // ORDINARY playback and run the clock past the Startup window, calling
        // that "fully charged" — but a use with no `MoveCharge` never charged
        // anything, and the doubling it observed came from the timeline road
        // that has since been deleted. It asserted the right outcome through
        // the wrong mechanism, which is why deleting the road turned it red.
        spawn_charged_attacker(
            &mut app,
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(15.0, 24.0),
            charge_move(mult),
        );
        // Into the Active window (t ≈ 0.26), where the volume spawns.
        run_seconds(&mut app, 0.25);
        let mut q = app.world_mut().query::<&Hitbox>();
        let hb = q
            .iter(app.world())
            .next()
            .expect("the active window spawns the volume");
        let knockback = match hb.knockback {
            crate::strike::HitboxKnockback::LaunchSpeed { base, growth } => {
                assert_eq!(growth, None, "charge fixture authors no growth");
                base
            }
            other => panic!("moveset melee must author a launch speed, got {other:?}"),
        };
        (hb.damage, knockback)
    };
    assert!(
        charge_move(2.0).charge_policy().is_some(),
        "the paying fixture resolves no charge policy, so nothing below can \
         observe a charge being paid"
    );
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

///  A LIVE STRIKE KEEPS THE ORIENTATION ITS MOVE STARTED WITH.
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
///  the vacuity guard is the second half: the body's facing must actually
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
    //  the vacuity half: the turn really happened, and the move did NOT adopt it.
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

/// decomposability proof: the SAME MoveSpec value bound to a second, differently-shaped actor
/// lands the same hit — re-binding is data.
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

/// After 0.32s of world time the undilated attacker has already hit; the dilated one is still
/// in startup with a proportionally smaller phase. Its hit arrives ~4x later, and the volume's
/// world-time life stretches with it.
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

/// This is the insert the moveset runtime was missing; without it the whole system was dead in
/// the shipping game.
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
            buffer_combat_action_presses,
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
        (
            resolve_attack_gestures,
            // ⛔ THE REAL CHAIN, and the middle link is not optional. Input
            // leniency sits BETWEEN interpretation and the action authority in
            // production, and it is where a special press acquires its meaning —
            // a fixture that skips it is asking the trigger to read a raw frame
            // no shipped body ever hands it.
            buffer_combat_action_presses,
            trigger_moveset_moves,
        )
            .chain(),
    );

    let make_move = |id: &str| MoveSpec {
        display_name: None,
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
        smash_charge: None,
        repeat: None,
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
        display_name: None,
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
        smash_charge: None,
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
    }
}

fn trigger_gesture_move(include_smash: bool, strong: bool) -> String {
    let mut app = App::new();
    // `buffer_combat_action_presses` decays its windows on the owner's own
    // clock, so the chain needs one.
    app.init_resource::<WorldTime>();
    app.add_systems(
        Update,
        (
            resolve_attack_gestures,
            // ⛔ THE REAL CHAIN, and the middle link is not optional. Input
            // leniency sits BETWEEN interpretation and the action authority in
            // production, and it is where a special press acquires its meaning —
            // a fixture that skips it is asking the trigger to read a raw frame
            // no shipped body ever hands it.
            buffer_combat_action_presses,
            trigger_moveset_moves,
        )
            .chain(),
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
        (
            resolve_attack_gestures,
            // ⛔ THE REAL CHAIN, and the middle link is not optional. Input
            // leniency sits BETWEEN interpretation and the action authority in
            // production, and it is where a special press acquires its meaning —
            // a fixture that skips it is asking the trigger to read a raw frame
            // no shipped body ever hands it.
            buffer_combat_action_presses,
            trigger_moveset_moves,
        )
            .chain(),
    );
    let mv = MoveSpec {
        display_name: None,
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
        smash_charge: None,
        repeat: None,
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
    // The dispatcher asks for PAIRED effects now, so the channel it writes has to exist or the
    // system fails parameter validation.
    app.add_message::<ambition_vfx::FxRequest>();
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
            world_offset: ae::Vec2::ZERO,
            owner,
            move_id: "sig".into(),
            presentation_source: ambition_sfx::PresentationSourceId::new("sanic.moves"),
            kind: MoveEventKind::Sfx {
                cue: "pca.signature".into(),
            },
            world_pose: ambition_vfx::FxPose::UPRIGHT,
        });
    app.world_mut()
        .resource_mut::<Messages<MoveEventMessage>>()
        .write(MoveEventMessage {
            world_offset: ae::Vec2::ZERO,
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
            world_pose: ambition_vfx::FxPose::UPRIGHT,
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

/// A move started with an UPWARD aim fires upward, not sideways.
///
///  the aimed case was the broken one, and the fallback hid it.
/// `ActorControl.fire` is an EDGE cleared every tick, and a ranged move has
/// startup — so by the time its authored fire frame arrives the request that
/// triggered it is gone, and the handler fell through to the body's horizontal
/// FACING. That repairs left-versus-right and flattens every aim that
/// was up, down or diagonal.
///
///  the sibling test above cannot see this: it supplies a live `fire` on
/// the event frame, which is the tier that always worked. The distinguishing
/// input is a playback whose aim was captured at START with NO live edge now —
/// so that is what this drives.
#[test]
fn a_move_started_aiming_up_fires_up_after_its_request_is_cleared() {
    use ambition_characters::brain::action_set::{ActionSet, RangedActionSpec};
    use ambition_characters::brain::ActorActionMessage;
    use ambition_characters::control::ActorControl;
    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ActorActionMessage>();
    // The dispatcher asks for PAIRED effects now, so the channel it writes has to exist or the
    // system fails parameter validation.
    app.add_message::<ambition_vfx::FxRequest>();
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
            world_offset: ae::Vec2::ZERO,
            owner,
            move_id: "fire".into(),
            presentation_source: ambition_sfx::PresentationSourceId::unscoped(),
            kind: MoveEventKind::Ranged,
            world_pose: ambition_vfx::FxPose::UPRIGHT,
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
/// aim at the event frame — so the shared projectile-request consumer fires the
/// shot unchanged and a moveset shot still tracks a strafing target.
#[test]
fn move_event_dispatch_bridges_ranged_to_a_live_aimed_shot() {
    use ambition_characters::actor::control::ActorFireRequest;
    use ambition_characters::brain::action_set::{ActionSet, RangedActionSpec, RangedStyle};
    use ambition_characters::brain::ActorActionMessage;
    use ambition_characters::control::ActorControl;
    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ActorActionMessage>();
    // The dispatcher asks for PAIRED effects now, so the channel it writes has to exist or the
    // system fails parameter validation.
    app.add_message::<ambition_vfx::FxRequest>();
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
            world_offset: ae::Vec2::ZERO,
            owner,
            move_id: "fire".into(),
            presentation_source: ambition_sfx::PresentationSourceId::unscoped(),
            kind: MoveEventKind::Ranged,
            world_pose: ambition_vfx::FxPose::UPRIGHT,
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

/// A body with no live aim fires the way it is FACING, not world-right.
///
/// `frame.fire` is an edge — `clear_edges()` nulls it every tick — and a ranged move has startup,
/// so by the time its fire frame arrives the intent that started it is usually gone. Reported from
/// play as "Maryo's fireball only shoots to her right, not the way she is facing".
#[test]
fn a_ranged_move_without_live_aim_fires_along_the_bodys_facing() {
    use ambition_characters::brain::action_set::{ActionSet, RangedActionSpec};
    use ambition_characters::brain::ActorActionMessage;
    use ambition_characters::control::ActorControl;
    for facing in [-1.0f32, 1.0] {
        let mut app = App::new();
        app.add_message::<MoveEventMessage>();
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<ActorActionMessage>();
        // The dispatcher asks for PAIRED effects now, so the channel it writes has to exist or
        // the system fails parameter validation.
        app.add_message::<ambition_vfx::FxRequest>();
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
                world_offset: ae::Vec2::ZERO,
                owner,
                move_id: "fire".into(),
                presentation_source: ambition_sfx::PresentationSourceId::unscoped(),
                kind: MoveEventKind::Ranged,
                world_pose: ambition_vfx::FxPose::UPRIGHT,
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

#[test]
fn a_fire_intent_triggers_the_ranged_move() {
    use ambition_characters::actor::control::ActorFireRequest;
    use ambition_characters::brain::action_set::RangedActionSpec;
    use ambition_characters::control::ActorControl;

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
    // `buffer_combat_action_presses` decays its windows on the owner's own
    // clock, so the chain needs one.
    app.init_resource::<WorldTime>();
    app.add_systems(
        Update,
        (
            resolve_attack_gestures,
            // ⛔ THE REAL CHAIN, and the middle link is not optional. Input
            // leniency sits BETWEEN interpretation and the action authority in
            // production, and it is where a special press acquires its meaning —
            // a fixture that skips it is asking the trigger to read a raw frame
            // no shipped body ever hands it.
            buffer_combat_action_presses,
            trigger_moveset_moves,
        )
            .chain(),
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

/// Only the `"attack"` move projects a swing.
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
    // `buffer_combat_action_presses` decays its windows on the owner's own
    // clock, so the chain needs one.
    app.init_resource::<WorldTime>();
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
    // `buffer_combat_action_presses` decays its windows on the owner's own
    // clock, so the chain needs one.
    app.init_resource::<WorldTime>();
    app.add_systems(
        Update,
        (
            resolve_attack_gestures,
            // ⛔ THE REAL CHAIN, and the middle link is not optional. Input
            // leniency sits BETWEEN interpretation and the action authority in
            // production, and it is where a special press acquires its meaning —
            // a fixture that skips it is asking the trigger to read a raw frame
            // no shipped body ever hands it.
            buffer_combat_action_presses,
            trigger_moveset_moves,
        )
            .chain(),
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

/// The accumulator now lives on the persistent `MovePlayback`; the projection must COPY it onto the
/// swing so `apply_hitbox_damage` re-emits it as `ignored_targets`.
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

///  An aerial is a COMMITMENT: land mid-move and pay for it, land after
/// the auto-cancel point and land clean.
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

    assert!((paid(Some(0.25), Some(0.50), 0.10) - 0.25).abs() < 1e-6);
    assert_eq!(paid(Some(0.25), Some(0.50), 0.60), 0.0);
    // No auto-cancel authored: the lag applies whenever the move is running.
    assert!((paid(Some(0.25), None, 0.60) - 0.25).abs() < 1e-6);
    //  the poison. A move that authors no landing lag lands the way every
    // move in the game lands today. Opt-in means opt-in.
    assert_eq!(paid(None, None, 0.10), 0.0);
    assert_eq!(paid(None, Some(0.50), 0.10), 0.0);
}

///  A move begun ON THE GROUND never "lands".
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
                head_contact: false,
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
        knockback_growth: None,
        launch_dir: None,
        on_hit: None,
        //  no `vfx` tag: a bladed volume would resolve the fixture manifest
        // blade and every keyframe would swing the SAME authored hull, which is
        // exactly the geometry this test varies.
        vfx: None,
        hit_sfx: None,
        autolink: None,
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

///  one swing is one hit, however many keyframes it was sampled at.
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

///  POISON: a GAP still rehits. A genuine multi-hit move — a drill, a rapid jab — is
/// authored as Active windows with space between them, because that is physically what it is:
/// the box goes away and comes back.
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

/// AN AUTHORED EFFECT FACES THE WAY THE FIGHTER DOES.
///
///  this asserts the pose travels WITH the offset, from one derivation. A
/// pose computed anywhere else could disagree with the position it decorates,
/// which is why both come out of the same expression.
#[test]
fn an_authored_effect_is_mirrored_by_the_facing_its_offset_already_used() {
    let facing_right = ambition_vfx::FxPose::of(1.0, 0.0);
    let facing_left = ambition_vfx::FxPose::of(-1.0, 0.0);

    assert!(
        !facing_right.mirror,
        "a right-facing move mirrored its art, so every effect in the game is \
         now backwards"
    );
    assert!(
        facing_left.mirror,
        "a LEFT-facing move drew its art unmirrored — this is the defect: the \
         offset is mirrored and the picture is not"
    );

    //  non-vacuity: the identity must really be identity, or the assertions
    // above are comparing a pose against a pose that means nothing.
    assert!(
        !ambition_vfx::FxPose::UPRIGHT.mirror && ambition_vfx::FxPose::UPRIGHT.angle == 0.0,
        "UPRIGHT is not the identity, so every emitter that never had an \
         opinion just acquired one"
    );

    //  and the angle is carried, not dropped — a body under sideways gravity
    // stands its effects up the same way it stands itself up.
    let rolled = ambition_vfx::FxPose::of(1.0, std::f32::consts::FRAC_PI_2);
    assert!(
        (rolled.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-6,
        "the frame's rotation did not survive into the pose, so an effect stays \
         world-upright under a gravity flip while its owner does not"
    );
}

// ─── capture context ─────────────────────────────────────────────────────────

/// The four capture moves plus an ordinary jab and special, so a test can ask
/// which one a press reached.
fn capture_context_moveset() -> MovesetContract {
    MovesetContract {
        verbs: std::collections::BTreeMap::from([
            (ATTACK_VERB.to_string(), "jab".to_string()),
            (SPECIAL_VERB.to_string(), "neutral_special".to_string()),
            (GRAB_VERB.to_string(), "grab".to_string()),
            (CAPTURE_PUMMEL_VERB.to_string(), "pummel".to_string()),
            (CAPTURE_THROW_FORWARD_VERB.to_string(), "fthrow".to_string()),
        ]),
        moves: vec![
            gesture_test_move("jab"),
            gesture_test_move("neutral_special"),
            gesture_test_move("grab"),
            gesture_test_move("pummel"),
            gesture_test_move("fthrow"),
        ],
    }
}

/// The same cast with every capture move GATED GROUNDED, which is how the smash
/// capture kit actually authors them (`SmashCaptureRepertoire::bound`).
fn grounded_capture_moveset() -> MovesetContract {
    let mut contract = capture_context_moveset();
    for spec in &mut contract.moves {
        if matches!(spec.id.as_str(), "grab" | "pummel" | "fthrow") {
            spec.gates.grounded = Some(true);
        }
    }
    contract
}

/// Build an app with the trigger chain, a captor carrying `frame`, and — when
/// `held` — a captive already in that captor's hold. Returns the captor.
fn capture_context_app(
    frame: ambition_characters::actor::control::ActorControlFrame,
    held: bool,
) -> (App, Entity) {
    capture_context_app_in(frame, held, capture_context_moveset(), None)
}

/// The same, with an explicit contract and an explicit stance. `grounded: None`
/// leaves the body without a `BodyGroundState` at all, which the selector reads
/// as grounded — the default the simpler fixture above relies on.
fn capture_context_app_in(
    frame: ambition_characters::actor::control::ActorControlFrame,
    held: bool,
    contract: MovesetContract,
    grounded: Option<bool>,
) -> (App, Entity) {
    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.init_resource::<WorldTime>();
    app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
    app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
    app.add_systems(
        Update,
        (
            resolve_attack_gestures,
            // ⛔ THE REAL CHAIN, and the middle link is not optional. Input
            // leniency sits BETWEEN interpretation and the action authority in
            // production, and it is where a special press acquires its meaning —
            // a fixture that skips it is asking the trigger to read a raw frame
            // no shipped body ever hands it.
            buffer_combat_action_presses,
            trigger_moveset_moves,
        )
            .chain(),
    );
    let captor = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                facing: 1.0,
                ..Default::default()
            },
            ActorFaction::Enemy,
            ActorMoveset(contract),
            ActorControl(frame),
        ))
        .id();
    if let Some(on_ground) = grounded {
        app.world_mut()
            .entity_mut(captor)
            .insert(ambition_platformer2d_core::BodyGroundState {
                head_contact: false,
                on_ground,
                ..Default::default()
            });
    }
    if held {
        // ⭐ THE RULESET'S HALF RIDES BESIDE THE RELATION, which is the shape
        // production builds: `acquire_captures` inserts both. Armed, because
        // these fixtures ask what a captor's PRESS resolves to, not whether the
        // stick has been re-centred since the grab.
        app.world_mut().spawn((
            crate::capture::CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(16.0, 0.0),
                prior_gravity_scale: 1.0,
            },
            ambition_characters::smash_capture::SmashHoldState {
                throw_armed: true,
                ..Default::default()
            },
        ));
    }
    app.update();
    (app, captor)
}

fn played(app: &App, body: Entity) -> Option<String> {
    app.world()
        .get::<MovePlayback>(body)
        .map(|pb| pb.spec.id.clone())
}

/// The same fixture with a RAISED GUARD, so the out-of-shield roads can be
/// asked about. Nothing else differs: no policy is declared, which is the
/// unrestricted case every body had before out-of-shield rules existed.
fn shielding_app(frame: ambition_characters::actor::control::ActorControlFrame) -> (App, Entity) {
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
    let body = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                facing: 1.0,
                ..Default::default()
            },
            ActorFaction::Enemy,
            ActorMoveset(capture_context_moveset()),
            ActorControl(frame),
            ambition_platformer2d_core::BodyShieldState {
                active: true,
                ..Default::default()
            },
            // ⛔ the frame carries `shield_held` too: the grab reads the BUTTON,
            // not the body state, so a fixture that raised only the latter would
            // be testing the thing that was wrong.
        ))
        .id();
    app.update();
    (app, body)
}

/// ⭐ ATTACK ON A RAISED GUARD IS A GRAB. Jon, 2026-08-23: *"if you are
/// shielding and press a, that should trigger a grab."* It is the genre's rule
/// and it is how most players grab at all.
///
/// ⛔ THE FALSIFIER IS THE SAME PRESS WITH THE GUARD DOWN, below: without it
/// this test passes on a body that would have grabbed anyway.
#[test]
fn attack_on_a_raised_guard_grabs() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.shield_held = true;
    let (app, body) = shielding_app(frame);
    assert_eq!(
        played(&app, body).as_deref(),
        Some("grab"),
        "attack from behind a guard did not grab - it reached the attack arm, which \
         only lets an UP-aimed press out of shield, so shield+A did nothing at all"
    );
}

/// ⛔ AND THE SAME PRESS WITH NO GUARD IS AN ORDINARY ATTACK, not a grab. The
/// guard is what makes it a grab; a body that grabbed on every neutral press
/// would have no jab.
#[test]
fn the_same_attack_with_no_guard_is_not_a_grab() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    let (app, body) = capture_context_app(frame, false);
    assert_ne!(
        played(&app, body).as_deref(),
        Some("grab"),
        "a neutral attack press with the guard DOWN resolved a grab, so the guard is \
         not what is deciding"
    );
}

/// A FREE BODY'S GRAB PRESS STARTS ITS GRAB.
#[test]
fn a_free_body_pressing_grab_plays_its_grab_move() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.grab_pressed = true;
    let (app, captor) = capture_context_app(frame, false);
    assert_eq!(played(&app, captor).as_deref(), Some("grab"));
}

/// HOLDING SOMEBODY, NEUTRAL ATTACK IS A PUMMEL AND NOT A JAB.
///
/// The same press that swings a jab when free must reach the pummel when a
/// captive is held — that is the whole content of "capture is a context".
#[test]
fn a_captor_pressing_attack_pummels_rather_than_jabbing() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    let (held_app, captor) = capture_context_app(frame.clone(), true);
    assert_eq!(played(&held_app, captor).as_deref(), Some("pummel"));

    let (free_app, free) = capture_context_app(frame, false);
    assert_eq!(
        played(&free_app, free).as_deref(),
        Some("jab"),
        "the fixture is wrong: this press must reach the jab when nobody is held, \
         or the test above proves nothing about the context"
    );
}

/// FORWARD + ATTACK IS THE THROW.
#[test]
fn a_captor_pressing_forward_attack_throws() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.attack_axis = ae::LocalAxes::X;
    let (app, captor) = capture_context_app(frame, true);
    assert_eq!(played(&app, captor).as_deref(), Some("fthrow"));
}

/// ⭐ A DIRECTION ALONE THROWS — no attack press. Jon, 2026-08-23: *"Throw
/// should not require you to press attack and a direction. just pressing the
/// direction after you grab should trigger the throw."*
///
/// That is the genre's rule: a held opponent is thrown by tilting the stick, and
/// Attack is what pummels. The pair above and below stay green because the
/// aimed attack press is consulted FIRST and keeps its exact meaning — this is
/// a second road to the same move, not a replacement.
#[test]
fn a_captor_holding_forward_throws_without_pressing_attack() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    // The stick, and nothing else. No `melee_pressed`.
    frame.attack_axis = ae::LocalAxes::X;
    let (app, captor) = capture_context_app(frame, true);
    assert_eq!(
        played(&app, captor).as_deref(),
        Some("fthrow"),
        "holding forward while gripping a captive did not throw - a throw still \
         needs the attack button it should not need"
    );
}

/// ⛔ AND A NEUTRAL STICK IS NOT A THROW. With no direction there is nothing to
/// throw toward, so a captor standing still keeps its grip rather than
/// resolving some default throw.
///
/// This is the falsifier for the test above: a change that made "holding a
/// captive" alone resolve a throw would pass it and fail this.
#[test]
fn a_captor_holding_nothing_keeps_its_grip() {
    let frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    let (app, captor) = capture_context_app(frame, true);
    assert_eq!(
        played(&app, captor),
        None,
        "a captor with a neutral stick and no press resolved a move anyway"
    );
}

/// AN UNAUTHORED THROW DOES NOTHING — IT DOES NOT BECOME A PUMMEL.
///
/// This fighter has no up-throw. A player who presses up+attack and gets a
/// pummel has been told the fighter has a bad up-throw; getting nothing tells
/// them it has none, which is the truth. A silent substitution is how a roster
/// grows moves nobody authored.
#[test]
fn an_unauthored_throw_direction_plays_nothing_at_all() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.attack_axis = ae::LocalAxes::new(0.0, -1.0);
    let (app, captor) = capture_context_app(frame, true);
    assert_eq!(
        played(&app, captor),
        None,
        "an up press with no authored up-throw reached a move anyway"
    );
}

/// CAPTURE CONTEXT REPLACES THE MENU — a special press finds nothing.
///
/// If it fell through, every ordinary verb would need its own "unless holding"
/// clause and a captor could fire a projectile with somebody in its hands.
#[test]
fn a_captor_cannot_reach_its_ordinary_special() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.special_pressed = true;
    let (app, captor) = capture_context_app(frame.clone(), true);
    assert_eq!(
        played(&app, captor),
        None,
        "a body holding a captive reached its ordinary special"
    );

    let (free_app, free) = capture_context_app(frame, false);
    assert_eq!(
        played(&free_app, free).as_deref(),
        Some("neutral_special"),
        "the fixture is wrong: this press must reach the special when free"
    );
}

// A held weapon owns the Attack press; the wearer's normal attack must not run beside it.

/// A body with its own `attack` verb, pressing it, nothing playing.
fn spawn_pressing_body(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            ActorMoveset(swat_moveset()),
            pressing_attack(),
            ae::BodyKinematics {
                pos: ae::Vec2::new(100.0, 100.0),
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(15.0, 24.0),
                facing: 1.0,
            },
        ))
        .id()
}

fn playing_id(app: &App, body: Entity) -> Option<String> {
    app.world()
        .get::<MovePlayback>(body)
        .map(|pb| pb.spec.id.clone())
}

/// The gun-sword's one verb is a shot, so the press leaves this seam entirely
/// (`fire_held_ranged_system` answers it) — and crucially the wearer's jab does
/// NOT run beside it.
#[test]
fn a_ranged_weapon_in_hand_silences_the_wearers_jab() {
    let mut app = trigger_app();
    let bare = spawn_pressing_body(&mut app);
    let armed = spawn_pressing_body(&mut app);
    app.world_mut()
        .entity_mut(armed)
        .insert(crate::held_items::HeldItem::new(
            ambition_characters::brain::held_item_by_id("gun_sword")
                .expect("gun_sword is a built-in held item"),
        ));
    app.update();

    assert_eq!(
        playing_id(&app, bare).as_deref(),
        Some("swat"),
        "the control: an empty hand still swings the wearer's own attack"
    );
    assert_eq!(
        playing_id(&app, armed),
        None,
        "the jab ran while the gun-sword was held — two claimants on one press"
    );
}

/// A weapon that DOES author a swing answers with its own, not the wearer's.
#[test]
fn a_melee_weapon_in_hand_answers_with_its_own_swing() {
    let mut app = trigger_app();
    let armed = spawn_pressing_body(&mut app);
    app.world_mut()
        .entity_mut(armed)
        .insert(crate::held_items::HeldItem::new(
            ambition_characters::brain::HeldItemSpec {
                id: "test_axe".into(),
                melee: Some(ambition_characters::brain::MeleeActionSpec::Swipe(
                    ambition_characters::brain::SwipeSpec::STRIKER_DEFAULT,
                )),
                ranged: None,
                use_behavior: ambition_characters::brain::HeldUseBehavior::Auto,
            },
        ));
    app.update();

    let played = playing_id(&app, armed).expect("a weapon with a swing answers the press");
    assert_ne!(
        played, "swat",
        "the wearer's own repertoire answered while a weapon was in hand"
    );
}

/// THE PROBE. AN AIRBORNE BODY REACHES NO CAPTURE MOVE, HOLDING OR NOT.
///
/// `SmashCaptureRepertoire::bound` gates its whole vocabulary grounded-only and
/// says why: an aerial grab is a named FUTURE technique, and "a grab that
/// answered an airborne press would be one of them by accident." The selector
/// asked for capture verbs through the ungated exact-verb lookup, so it started
/// them anyway — the free body's grab, and then every pummel and throw a captor
/// carried into the air.
///
/// The invariant outlives the fix: what a press may start is the intersection of
/// what the fighter AUTHORS and what their STANCE permits. It is asserted in
/// both capture contexts because they are different branches of the selector,
/// and fixing one lookup would not have proven the other reads it.
#[test]
fn an_airborne_body_reaches_no_grounded_only_capture_move() {
    let press = |grab: bool| {
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        if grab {
            frame.grab_pressed = true;
        } else {
            frame.melee_pressed = true;
        }
        frame
    };

    // FREE, airborne: the grab does not start.
    let (app, captor) =
        capture_context_app_in(press(true), false, grounded_capture_moveset(), Some(false));
    assert_eq!(
        played(&app, captor),
        None,
        "an airborne press started a grab the capture kit declares grounded-only"
    );

    // HOLDING, airborne: the pummel does not start either.
    let (app, captor) =
        capture_context_app_in(press(false), true, grounded_capture_moveset(), Some(false));
    assert_eq!(
        played(&app, captor),
        None,
        "a captor carried into the air still pummelled, so the capture branch \
         never read the gate its own repertoire authored"
    );

    //  NON-VACUITY, and it is the whole test. Every assertion above passes if
    // the fixture simply never starts a move — so the same presses, same
    // contract, on the GROUND must reach exactly the moves they name.
    let (app, captor) =
        capture_context_app_in(press(true), false, grounded_capture_moveset(), Some(true));
    assert_eq!(
        played(&app, captor).as_deref(),
        Some("grab"),
        "a grounded press stopped reaching its own grab"
    );
    let (app, captor) =
        capture_context_app_in(press(false), true, grounded_capture_moveset(), Some(true));
    assert_eq!(
        played(&app, captor).as_deref(),
        Some("pummel"),
        "a grounded captor stopped reaching its own pummel"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Combat action input buffer.
//
// The invariant under test is the LIFECYCLE, never the window length: a press
// the action authority refuses is re-proposed until it is accepted or expires,
// it is accepted at most once, and the press that is replayed is the press that
// was made.
// ─────────────────────────────────────────────────────────────────────────────

/// A body with one buffering verb chain and nothing else, so a refusal is
/// unambiguously the trigger's and not some other gate's.
fn buffer_app(moveset: MovesetContract, buffer_s: f32) -> (App, Entity) {
    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.init_resource::<WorldTime>();
    app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
    app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
    app.add_systems(
        Update,
        (
            resolve_attack_gestures,
            buffer_combat_action_presses,
            trigger_moveset_moves,
        )
            .chain(),
    );
    let body = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(16.0, 32.0),
                facing: 1.0,
            },
            ActorMoveset(moveset),
            ActorControl(ambition_characters::actor::control::ActorControlFrame::neutral()),
        ))
        .id();
    app.world_mut()
        .get_mut::<AttackGestureTuning>(body)
        .expect("ActorControl requires the gesture tuning")
        .action_buffer_s = buffer_s;
    (app, body)
}

/// A move with no `Cancelable` window: it refuses every replacement for its
/// whole duration, which is what "endlag" means to the trigger.
fn uncancelable(id: &str) -> MoveSpec {
    MoveSpec {
        display_name: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s: 1.0,
        windows: vec![],
        events: vec![],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
    }
}

fn set_frame(
    app: &mut App,
    body: Entity,
    edit: impl FnOnce(&mut ambition_characters::actor::control::ActorControlFrame),
) {
    let mut control = app.world_mut().get_mut::<ActorControl>(body).unwrap();
    edit(&mut control.0);
}

fn playing(app: &App, body: Entity) -> Option<String> {
    app.world()
        .get::<MovePlayback>(body)
        .map(|pb| pb.spec.id.clone())
}

/// The whole point: an attack pressed while the body cannot act starts on the
/// first tick it can, and starts EXACTLY ONE move.
#[test]
fn a_refused_attack_press_is_replayed_once_when_the_authority_accepts_it() {
    let (mut app, body) = buffer_app(swat_moveset(), 0.5);
    app.world_mut()
        .entity_mut(body)
        .insert(MovePlayback::new(uncancelable("endlag"), 1.0));
    set_frame(&mut app, body, |f| {
        f.melee_pressed = true;
        f.melee_held = true;
    });
    app.update();

    assert_eq!(
        playing(&app, body).as_deref(),
        Some("endlag"),
        "the trigger must still refuse the press: this fixture is about a press \
         that arrives while a move is refusing replacement"
    );
    assert!(
        app.world()
            .get::<ae::BodyActionBuffer>(body)
            .unwrap()
            .attack
            > 0.0,
        "the refused press left no buffered window, so it was dropped exactly \
         as it was before the buffer was fed"
    );

    // The button is long since up, and the move ends.
    set_frame(&mut app, body, |f| {
        f.melee_pressed = false;
        f.melee_held = false;
    });
    app.world_mut().entity_mut(body).remove::<MovePlayback>();
    app.update();

    assert_eq!(
        playing(&app, body).as_deref(),
        Some("swat"),
        "the buffered press did not start the move on the first tick the \
         authority could accept it"
    );
    assert_eq!(
        app.world()
            .get::<ae::BodyActionBuffer>(body)
            .unwrap()
            .attack,
        0.0,
        "accepting the action must SPEND the slot"
    );

    // ... and it is spent: the next opening starts nothing.
    app.world_mut().entity_mut(body).remove::<MovePlayback>();
    app.update();
    assert_eq!(
        playing(&app, body),
        None,
        "one press started two moves — the buffered proposal outlived its \
         acceptance"
    );
    assert!(
        app.world()
            .get::<AttackGestureState>(body)
            .unwrap()
            .buffered_press
            .is_none(),
        "the intent outlived its clock; the two are one fact"
    );
}

/// Leniency is a WINDOW, not a queue. A press older than the window is a press
/// the player got wrong.
#[test]
fn a_press_older_than_the_window_is_never_spent() {
    let (mut app, body) = buffer_app(swat_moveset(), 0.05);
    app.world_mut()
        .entity_mut(body)
        .insert(MovePlayback::new(uncancelable("endlag"), 1.0));
    set_frame(&mut app, body, |f| f.melee_pressed = true);
    app.update();
    set_frame(&mut app, body, |f| f.melee_pressed = false);

    // 0.05s of window against 0.016s ticks: four more ticks bury it.
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<ae::BodyActionBuffer>(body)
            .unwrap()
            .attack,
        0.0,
        "the window did not decay"
    );

    app.world_mut().entity_mut(body).remove::<MovePlayback>();
    app.update();
    assert_eq!(
        playing(&app, body),
        None,
        "a press from before the window started a move anyway"
    );
}

/// The buffer replays the PRESS, not the stick. A press classified as a smash
/// stays a smash even after the flick that classified it has aged out — which
/// is exactly what re-reading the live input at spend time would lose.
#[test]
fn a_buffered_smash_is_still_a_smash_after_its_flick_expires() {
    let moveset = MovesetContract {
        verbs: [
            (ATTACK_VERB.to_string(), "jab".to_string()),
            ("smash_forward".to_string(), "fsmash".to_string()),
        ]
        .into_iter()
        .collect(),
        moves: vec![uncancelable("jab"), uncancelable("fsmash")],
    };
    let (mut app, body) = buffer_app(moveset, 0.5);
    app.world_mut()
        .entity_mut(body)
        .insert(MovePlayback::new(uncancelable("endlag"), 1.0));

    // A forward flick, then the press inside its window: a forward smash.
    set_frame(&mut app, body, |f| f.attack_axis = ae::LocalAxes::X);
    app.update();
    set_frame(&mut app, body, |f| f.melee_pressed = true);
    app.update();
    set_frame(&mut app, body, |f| f.melee_pressed = false);

    // The stick stays forward, so the flick can never re-arm, and its four-tick
    // window runs out. A press resolved HERE would be a forward TILT.
    for _ in 0..6 {
        app.update();
    }

    app.world_mut().entity_mut(body).remove::<MovePlayback>();
    app.update();
    assert_eq!(
        playing(&app, body).as_deref(),
        Some("fsmash"),
        "the buffered press was re-resolved against the stale stick instead of \
         being replayed, so a buffered smash came out as a tilt"
    );
}

/// The grab is a bare edge with no intent to carry, and it buffers through the
/// same window and the same spend.
#[test]
fn a_refused_grab_press_is_replayed_and_spent() {
    let moveset = MovesetContract {
        verbs: [(GRAB_VERB.to_string(), "grab".to_string())]
            .into_iter()
            .collect(),
        moves: vec![uncancelable("grab")],
    };
    let (mut app, body) = buffer_app(moveset, 0.5);
    app.world_mut()
        .entity_mut(body)
        .insert(MovePlayback::new(uncancelable("endlag"), 1.0));
    set_frame(&mut app, body, |f| f.grab_pressed = true);
    app.update();
    set_frame(&mut app, body, |f| f.grab_pressed = false);
    assert!(app.world().get::<ae::BodyActionBuffer>(body).unwrap().grab > 0.0);

    app.world_mut().entity_mut(body).remove::<MovePlayback>();
    app.update();
    assert_eq!(playing(&app, body).as_deref(), Some("grab"));
    assert_eq!(
        app.world().get::<ae::BodyActionBuffer>(body).unwrap().grab,
        0.0,
        "accepting the grab must spend its slot"
    );
}

/// A body whose ruleset turns leniency off behaves exactly as it did before the
/// buffer was fed: the press is spendable only on the tick it arrives.
#[test]
fn a_zero_window_restores_the_drop_it_replaced() {
    let (mut app, body) = buffer_app(swat_moveset(), 0.0);
    app.world_mut()
        .entity_mut(body)
        .insert(MovePlayback::new(uncancelable("endlag"), 1.0));
    set_frame(&mut app, body, |f| f.melee_pressed = true);
    app.update();
    set_frame(&mut app, body, |f| f.melee_pressed = false);
    app.world_mut().entity_mut(body).remove::<MovePlayback>();
    app.update();
    assert_eq!(
        playing(&app, body),
        None,
        "a zero window still queued the press"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// True hold/release smash charge.
//
// The invariants are the STATE MACHINE — freeze, accrue, release once, freeze
// the payoff — and the two ends of the authored range. The numbers below are
// the fixture's own authoring, never a feel judgement.
// ─────────────────────────────────────────────────────────────────────────────

/// ⛔⛔ STRICTLY INSIDE THE WINDUP, and it used to be `0.10` — the same instant
/// `CHARGE_ACTIVE_AT_S` opens the strike. Active membership is
/// `start_s <= t < end_s`, so a charge frozen there stands inside a live hitbox,
/// which is the whole defect `a_held_charge_has_no_live_strike_and_releases_into_one`
/// exists to refuse. The fixture authored it, so the fixture reproduced it.
const CHARGE_HOLD_AT_S: f32 = 0.05;
/// Where the fixture's strike goes live — FLUSH against the windup, because
/// that is how a smash is ordinarily authored and it is the shape that made the
/// derived hold point illegal.
const CHARGE_ACTIVE_AT_S: f32 = 0.10;
const CHARGE_MAX_HOLD_S: f32 = 0.20;
const CHARGE_MULT: f32 = 2.0;

/// A forward smash: startup, one active window, recovery — with a charge policy
/// short enough to run in a handful of 0.016s ticks.
fn charging_smash() -> MoveSpec {
    MoveSpec {
        display_name: None,
        id: "fsmash".to_string(),
        clip: ClipBinding {
            clip: "fsmash".to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.30,
        windows: vec![
            MoveWindow {
                start_s: 0.0,
                end_s: CHARGE_ACTIVE_AT_S,
                tag: WindowTag::Startup,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            MoveWindow {
                start_s: CHARGE_ACTIVE_AT_S,
                end_s: 0.16,
                tag: WindowTag::Active,
                volumes: vec![HitVolume {
                    hit_sfx: None,
                    shape: ambition_entity_catalog::VolumeShape::Rect {
                        offset: (24.0, 0.0),
                        half_extents: (16.0, 12.0),
                    },
                    damage: 5,
                    knockback: 100.0,
                    knockback_growth: None,
                    launch_dir: None,
                    on_hit: None,
                    vfx: None,
                    autolink: None,
                }],
                sustain_effect: None,
                motion_scale: 1.0,
            },
            MoveWindow {
                start_s: 0.16,
                end_s: 0.30,
                tag: WindowTag::Recovery,
                volumes: vec![],
                sustain_effect: None,
                motion_scale: 1.0,
            },
        ],
        events: vec![],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: CHARGE_MULT,
        smash_charge: Some(ambition_entity_catalog::SmashChargeSpec {
            hold_at_s: CHARGE_HOLD_AT_S,
            max_hold_s: CHARGE_MAX_HOLD_S,
        }),
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
    }
}

/// The whole production chain a press travels, so a charge is exercised the way
/// a fighter actually charges one rather than by hand-driving the playback.
fn smash_charge_app() -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<super::super::authored_volumes::AuthoredAttackVolumeResolver>();
    app.add_message::<HitEvent>();
    app.add_message::<crate::hitbox::LandedBodyHit>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.init_resource::<WorldTime>();
    app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
    app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
    app.add_systems(
        Update,
        (
            resolve_attack_gestures,
            buffer_combat_action_presses,
            trigger_moveset_moves,
            advance_move_playback,
        )
            .chain(),
    );
    let moveset = MovesetContract {
        verbs: [
            ("smash_forward".to_string(), "fsmash".to_string()),
            (ATTACK_VERB.to_string(), "jab".to_string()),
            ("attack_forward".to_string(), "fsmash".to_string()),
        ]
        .into_iter()
        .collect(),
        moves: vec![charging_smash(), uncancelable("jab")],
    };
    let body = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(16.0, 32.0),
                facing: 1.0,
            },
            ActorFaction::Player,
            ActorMoveset(moveset),
            ActorControl(ambition_characters::actor::control::ActorControlFrame::neutral()),
        ))
        .id();
    (app, body)
}

/// Press Attack forward as a SMASH on this tick. `hold` decides whether the
/// button is still down afterwards.
fn press_smash(app: &mut App, body: Entity, hold: bool) {
    set_frame(app, body, |f| {
        f.attack_axis = ae::LocalAxes::X;
        f.melee_pressed = true;
        f.melee_strong_hint = true;
        f.melee_held = hold;
        f.melee_released = !hold;
    });
    app.update();
    set_frame(app, body, |f| {
        f.melee_pressed = false;
        f.melee_strong_hint = false;
        f.melee_released = false;
    });
}

fn charge_of(app: &App, body: Entity) -> MoveCharge {
    app.world()
        .get::<MovePlayback>(body)
        .expect("the smash is still playing")
        .charge
        .expect("this use entered charge mode")
}

fn scale_of(app: &App, body: Entity) -> f32 {
    app.world()
        .get::<MovePlayback>(body)
        .expect("the smash is still playing")
        .charge_scale()
}

/// A tap is the floor of the range: the timeline reaches the hold point, sees
/// no held button, and fires with no charge at all.
#[test]
fn a_tapped_smash_releases_at_the_minimum_multiplier() {
    let (mut app, body) = smash_charge_app();
    press_smash(&mut app, body, false);
    for _ in 0..8 {
        app.update();
    }
    let charge = charge_of(&app, body);
    assert_eq!(
        charge.released_fraction,
        Some(0.0),
        "a tap bought charge it never held"
    );
    assert!(
        (scale_of(&app, body) - 1.0).abs() < 1e-5,
        "a tapped smash must land at the identity multiplier"
    );
    assert!(
        app.world()
            .get::<MovePlayback>(body)
            .unwrap()
            .smash_charge_fraction()
            .is_none(),
        "a released move must publish no charge for presentation to pulse on"
    );
}

/// The mechanic itself: while the button is down the move's own clock stands
/// still at the authored hold point and the charge grows instead.
#[test]
fn a_held_smash_freezes_its_timeline_and_accumulates_charge() {
    let (mut app, body) = smash_charge_app();
    press_smash(&mut app, body, true);
    for _ in 0..8 {
        app.update();
    }
    let frozen_t = app.world().get::<MovePlayback>(body).unwrap().t;
    assert!(
        (frozen_t - CHARGE_HOLD_AT_S).abs() < 1e-4,
        "the timeline did not stop at the authored hold point (t = {frozen_t})"
    );
    let first = charge_of(&app, body).held_s;
    assert!(first > 0.0, "the hold bought no charge");
    for _ in 0..3 {
        app.update();
    }
    assert!(
        charge_of(&app, body).held_s > first,
        "the charge stopped growing while the button was still down"
    );
    assert_eq!(
        app.world().get::<MovePlayback>(body).unwrap().t,
        frozen_t,
        "the clock moved while the charge was still building"
    );
    let published = app
        .world()
        .get::<MovePlayback>(body)
        .unwrap()
        .smash_charge_fraction()
        .expect("a charging body publishes its fraction");
    assert!(
        (0.0..=1.0).contains(&published),
        "the published charge fraction must be normalized, got {published}"
    );
}

/// The other end of the range, and the auto-release: holding past the maximum
/// fires the move at exactly the authored cap and never above it.
#[test]
fn a_full_hold_auto_releases_at_exactly_the_authored_cap() {
    let (mut app, body) = smash_charge_app();
    press_smash(&mut app, body, true);
    // The button is NEVER let go: reaching the maximum has to fire the move by
    // itself, and stopping on that tick is how the payoff is read before the
    // timeline finishes and the component is removed.
    let mut released = None;
    for _ in 0..40 {
        app.update();
        let Some(pb) = app.world().get::<MovePlayback>(body) else {
            break;
        };
        if let Some(charge) = pb.charge {
            if !charge.charging() {
                released = Some((charge.fraction(), pb.charge_scale()));
                break;
            }
        }
    }
    let (fraction, scale) = released.expect("the hold never auto-released at its maximum");
    assert!(
        (fraction - 1.0).abs() < 1e-5,
        "a full hold must be a full charge, got {fraction}"
    );
    assert!(
        (scale - CHARGE_MULT).abs() < 1e-5,
        "a full hold must land exactly the authored cap, got {scale}"
    );
}

/// One use, one payoff. The fraction is frozen at release and applies uniformly
/// to every hit the move still has left, so a multi-hit smash cannot charge
/// itself further between pulses.
#[test]
fn the_released_fraction_is_frozen_for_the_rest_of_the_move() {
    let (mut app, body) = smash_charge_app();
    press_smash(&mut app, body, true);
    // Hold briefly, then let go part-way through the charge.
    for _ in 0..10 {
        app.update();
    }
    set_frame(&mut app, body, |f| {
        f.melee_held = false;
        f.melee_released = true;
    });
    app.update();
    set_frame(&mut app, body, |f| f.melee_released = false);

    let at_release = charge_of(&app, body);
    let frozen = at_release
        .released_fraction
        .expect("letting go must freeze the payoff");
    assert!(
        frozen > 0.0 && frozen < 1.0,
        "the fixture meant to release PART-WAY; got {frozen}"
    );
    let scale = scale_of(&app, body);

    // Run the rest of the move — startup remainder, active, recovery.
    for _ in 0..8 {
        app.update();
        let Some(pb) = app.world().get::<MovePlayback>(body) else {
            break;
        };
        assert_eq!(
            pb.charge.unwrap().released_fraction,
            Some(frozen),
            "the payoff moved after release: later timeline progress is still \
             being read as charge"
        );
        assert!((pb.charge_scale() - scale).abs() < 1e-6);
    }
}

/// ⛔ A CHARGING FIGHTER DOES NOT WALK. Jon, 2026-08-23: *"when the character is
/// charging their smash attack, they should not be able to walk or move."*
///
/// The steering lock is a fact about CHARGING, not about a move's authoring: a
/// smash's Startup window carries the default `motion_scale: 1.0` like every
/// other window, so before this the body kept full steering while its clock
/// stood still and could walk the whole stage in its windup pose.
///
/// ⚠ The three states are asserted together because only their CONTRAST says
/// the rule is about the freeze rather than about the move: on the way to the
/// hold point the body is still swinging and keeps its authored motion, at the
/// hold point it is rooted, and after release it has its authored motion back.
#[test]
fn a_frozen_charge_roots_the_body_and_a_released_one_does_not() {
    let (mut app, body) = smash_charge_app();
    press_smash(&mut app, body, true);
    app.update();

    let playback = |app: &App| {
        app.world()
            .entity(body)
            .get::<MovePlayback>()
            .expect("the smash is playing")
            .clone()
    };

    // Held long enough to be sitting on the hold point. `press_smash(.., true)`
    // leaves `melee_held` set, so simply stepping keeps the button down.
    for _ in 0..6 {
        app.update();
    }
    let held = playback(&app);
    assert!(
        held.rooted_by_charge(),
        "the premise: the timeline is frozen at the hold point, t = {}",
        held.t
    );
    assert_eq!(
        held.motion_scale_now(),
        0.0,
        "a body frozen in its charge kept {} of its steering - it can walk while charging",
        held.motion_scale_now()
    );

    // Let go: the swing resumes and the body gets its authored motion back.
    set_frame(&mut app, body, |f| {
        f.melee_pressed = false;
        f.melee_held = false;
    });
    for _ in 0..3 {
        app.update();
    }
    let released = playback(&app);
    assert!(
        !released.rooted_by_charge(),
        "the charge was released, so nothing should still be rooting the body"
    );
    assert_eq!(
        released.motion_scale_now(),
        released.spec.motion_scale_at(released.t),
        "after release the move's OWN authored motion lock is the only authority again"
    );
}

/// A move reached through another verb plays its plain timeline. Chargeability
/// is a fact about the PRESS, not about the multiplier the move happens to
/// author.
#[test]
fn a_non_smash_use_of_the_same_move_never_charges() {
    let (mut app, body) = smash_charge_app();
    // A forward TILT: same direction, same resolved move (`attack_forward` maps
    // to `fsmash` in this fixture), no smash strength. The stick is deflected
    // PART-WAY — past the directional deadzone, short of the flick threshold —
    // which is exactly how a person throws a tilt.
    set_frame(&mut app, body, |f| {
        f.attack_axis = ae::LocalAxes::new(
            ambition_characters::actor::attack_gesture::TILT_DEFLECTION,
            0.0,
        );
        f.melee_pressed = true;
        f.melee_held = true;
    });
    app.update();
    let pb = app
        .world()
        .get::<MovePlayback>(body)
        .expect("the tilt started a move");
    assert_eq!(
        pb.spec.id, "fsmash",
        "the fixture meant to reach the SAME move"
    );
    assert!(
        pb.charge.is_none(),
        "a tilt froze its timeline: charge mode is being entered from the \
         multiplier instead of from the gesture"
    );
    // ... and it never freezes.
    set_frame(&mut app, body, |f| f.melee_pressed = false);
    // 0.30s of move against 0.016s ticks.
    for _ in 0..25 {
        app.update();
        if app.world().get::<MovePlayback>(body).is_none() {
            return;
        }
    }
    panic!("the un-charged use never finished its timeline");
}

/// A dilated fighter charges as slowly as it swings: the charge spends the
/// owner's proper time, which is the same clock the move's windows advance on.
#[test]
fn charge_accrues_in_the_owners_proper_time() {
    let mut held = Vec::new();
    for scale in [1.0f32, 0.5] {
        let (mut app, body) = smash_charge_app();
        app.world_mut()
            .entity_mut(body)
            .insert(ambition_time::ProperTimeScale(scale));
        press_smash(&mut app, body, true);
        for _ in 0..10 {
            app.update();
        }
        held.push(charge_of(&app, body).held_s);
    }
    assert!(
        held[0] > held[1] * 1.5,
        "a half-speed fighter charged nearly as fast as a normal one \
         ({:?}) — the charge is not on the owner's clock",
        held
    );
}

/// The charge is rollback state, so it must reach the session checksum: two
/// peers whose held smash differs land different damage from the same move.
#[test]
fn the_charge_is_part_of_the_playback_checksum() {
    use ambition_platformer2d_core::snapshot::SnapshotResolve;
    let encode = |pb: &MovePlayback| {
        let mut out = Vec::new();
        pb.encode_ref(&mut out);
        out
    };
    let base = MovePlayback::new(charging_smash(), 1.0).charged_by_smash_gesture(true);
    let mut longer = base.clone();
    longer.charge.as_mut().unwrap().held_s = 0.12;
    let mut released = base.clone();
    released.charge.as_mut().unwrap().released_fraction = Some(0.6);
    assert_ne!(
        encode(&base),
        encode(&longer),
        "how long a smash has been held is invisible to the checksum"
    );
    assert_ne!(
        encode(&base),
        encode(&released),
        "the frozen payoff is invisible to the checksum"
    );
    assert_ne!(
        encode(&base),
        encode(&MovePlayback::new(charging_smash(), 1.0)),
        "whether a use charges at all is invisible to the checksum"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Move invulnerability and super armor.
//
// `WindowTag::Invuln` and `WindowTag::Armor` were authoring vocabulary the
// runtime consumed nowhere. What is pinned here is that each becomes a fact the
// EXISTING rule reads — one eligibility gate, one reaction — and that both
// retract when the window closes.
// ─────────────────────────────────────────────────────────────────────────────

/// Invuln over `[0.0, 0.1)`, Armor over `[0.1, 0.2)`, plain after that.
fn defended_move() -> MoveSpec {
    let window = |start_s: f32, end_s: f32, tag: WindowTag| MoveWindow {
        start_s,
        end_s,
        tag,
        volumes: vec![],
        sustain_effect: None,
        motion_scale: 1.0,
    };
    MoveSpec {
        display_name: None,
        id: "defended".to_string(),
        clip: ClipBinding {
            clip: "defended".to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.3,
        windows: vec![
            window(0.0, 0.1, WindowTag::Invuln),
            window(0.1, 0.2, WindowTag::Armor),
        ],
        events: vec![],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
    }
}

fn defense_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_systems(Update, project_move_defense_windows);
    let body = app
        .world_mut()
        .spawn((
            ambition_characters::actor::BodyCombat::default(),
            ambition_characters::actor::BodyHealth::restored(
                ambition_characters::actor::Health::new(100),
                0,
                Default::default(),
            ),
        ))
        .id();
    (app, body)
}

fn defense_of(app: &App, body: Entity) -> (bool, bool) {
    let world = app.world();
    (
        world
            .get::<ambition_characters::actor::BodyHealth>(body)
            .unwrap()
            .health
            .invulnerable
            .holds(ambition_characters::actor::Invulnerability::MOVE),
        world
            .get::<ambition_characters::actor::BodyCombat>(body)
            .unwrap()
            .armored,
    )
}

/// Each authored window is in force for exactly its own span, and the body is
/// plain outside both — read off the two facts the rest of combat consults, not
/// off the timeline.
#[test]
fn the_authored_defensive_windows_answer_for_exactly_their_own_spans() {
    let (mut app, body) = defense_app();
    for (t, expected) in [
        (0.05, (true, false)),
        (0.15, (false, true)),
        (0.25, (false, false)),
    ] {
        app.world_mut()
            .entity_mut(body)
            .insert(MovePlayback::new_at(defended_move(), 1.0, t));
        app.update();
        assert_eq!(
            defense_of(&app, body),
            expected,
            "at t = {t} the move's authored windows resolved (invuln, armored) \
             = {:?}",
            defense_of(&app, body)
        );
    }
}

/// ⭐ A grant cleared only when somebody remembers is a grant that never
/// clears. The projection runs for every combat body, move or no move, so the
/// move ENDING is what retracts it.
#[test]
fn a_move_that_ends_takes_its_grants_with_it() {
    let (mut app, body) = defense_app();
    app.world_mut()
        .entity_mut(body)
        .insert(MovePlayback::new_at(defended_move(), 1.0, 0.05));
    app.update();
    assert_eq!(defense_of(&app, body), (true, false));

    // The move is over — the component is gone, exactly as
    // `advance_move_playback` leaves it.
    app.world_mut().entity_mut(body).remove::<MovePlayback>();
    app.update();
    assert_eq!(
        defense_of(&app, body),
        (false, false),
        "a body with no move at all is still holding the last one's \
         intangibility"
    );
}

/// Chargeability, intangibility and armor are per-USE facts of the move that is
/// playing; a body playing an ordinary move is granted nothing.
#[test]
fn an_ordinary_move_grants_neither() {
    let (mut app, body) = defense_app();
    app.world_mut()
        .entity_mut(body)
        .insert(MovePlayback::new(uncancelable("plain"), 1.0));
    app.update();
    assert_eq!(defense_of(&app, body), (false, false));
}

// ─────────────────────────────────────────────────────────────────────────────
// The jab chain and the flurry.
//
// Both are authored: a chain is a cancel table read forwards, and a loop is a
// stretch of one move's own timeline. Neither knows a fighter.
// ─────────────────────────────────────────────────────────────────────────────

/// The full production chain INCLUDING the clock, because a chain and a loop
/// are both questions about where the move's own timeline is.
fn playing_app(moveset: MovesetContract) -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<super::super::authored_volumes::AuthoredAttackVolumeResolver>();
    app.add_message::<HitEvent>();
    app.add_message::<crate::hitbox::LandedBodyHit>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.init_resource::<WorldTime>();
    app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
    app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
    app.add_systems(
        Update,
        (
            resolve_attack_gestures,
            buffer_combat_action_presses,
            trigger_moveset_moves,
            advance_move_playback,
        )
            .chain(),
    );
    let body = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(16.0, 32.0),
                facing: 1.0,
            },
            ActorFaction::Player,
            ActorMoveset(moveset),
            ActorControl(ambition_characters::actor::control::ActorControlFrame::neutral()),
        ))
        .id();
    app.world_mut()
        .get_mut::<AttackGestureTuning>(body)
        .expect("ActorControl requires the gesture tuning")
        .action_buffer_s = 0.0;
    (app, body)
}

fn chain_link(
    id: &str,
    into: &[&str],
    repeat: Option<ambition_entity_catalog::MoveLoop>,
) -> MoveSpec {
    let mut windows = vec![MoveWindow {
        start_s: 0.0,
        end_s: 0.10,
        tag: WindowTag::Active,
        volumes: vec![],
        sustain_effect: None,
        motion_scale: 1.0,
    }];
    if !into.is_empty() {
        windows.push(MoveWindow {
            start_s: 0.05,
            end_s: 0.30,
            tag: WindowTag::Cancelable {
                into: into.iter().map(|s| (*s).to_string()).collect(),
                condition: ambition_entity_catalog::CancelCondition::Always,
            },
            volumes: vec![],
            sustain_effect: None,
            motion_scale: 1.0,
        });
    }
    MoveSpec {
        display_name: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.30,
        windows,
        events: vec![],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        repeat,
        landing_lag_s: None,
        autocancel_after_s: None,
    }
}

/// ⭐ A FOLLOW-UP PRESS TAKES THE SUCCESSOR, and the route ends where the
/// authoring stops naming one.
///
/// Every fighter had one jab and it was a full commitment every time, because a
/// second press inside the cancel window resolved to the same verb and
/// restarted jab 1. The window already named where to go; nothing read it
/// forwards.
#[test]
fn a_follow_up_press_walks_the_authored_jab_chain() {
    let moveset = MovesetContract {
        verbs: [(ATTACK_VERB.to_string(), "jab".to_string())]
            .into_iter()
            .collect(),
        moves: vec![
            chain_link("jab", &["jab2"], None),
            chain_link("jab2", &["jab3"], None),
            chain_link("jab3", &[], None),
        ],
    };
    let (mut app, body) = playing_app(moveset);
    for expected in ["jab", "jab2", "jab3"] {
        set_frame(&mut app, body, |f| f.melee_pressed = true);
        app.update();
        set_frame(&mut app, body, |f| f.melee_pressed = false);
        assert_eq!(
            playing(&app, body).as_deref(),
            Some(expected),
            "the press should have reached {expected}"
        );
        // Into the successor's own cancel window before the next press.
        for _ in 0..4 {
            app.update();
        }
    }
    // jab3 names nobody: the route is over and a fourth press restarts it.
    set_frame(&mut app, body, |f| f.melee_pressed = true);
    app.update();
    assert_eq!(
        playing(&app, body).as_deref(),
        Some("jab3"),
        "a move that nominates no successor should refuse the press outright, \
         not walk somewhere the author never named"
    );
}

/// ⭐⭐ A HELD BUTTON WALKS THE STRING, WITH ONE PRESS AND NO MASH.
///
/// The genre's own input for continuing a jab: hold Attack and the string
/// advances. It matters more than taste here because it is the input this
/// game's BODIES produce. `MovePlayback`'s other two sustained mechanics both
/// read `ResolvedAttackGesture::held` — the smash charge waits on it, the flurry
/// loop repeats on it — and the chain read the press EDGE alone. Measured
/// 2026-08-23 over a 90-second George mirror: the two fighter brains hold Attack
/// for 960 body-ticks and produce ONE fresh neutral jab press in the whole
/// match, and the trace of that single jab shows `held: None` on every tick
/// after the one it started on. The chain's only entrance was an input nobody
/// gives.
#[test]
fn a_held_button_walks_the_string_with_no_second_press() {
    let moveset = MovesetContract {
        verbs: [(ATTACK_VERB.to_string(), "jab".to_string())]
            .into_iter()
            .collect(),
        moves: vec![
            chain_link("jab", &["jab2"], None),
            chain_link("jab2", &["jab3"], None),
            chain_link("jab3", &[], None),
        ],
    };
    let (mut app, body) = playing_app(moveset);
    // ONE press, then the button simply stays down.
    set_frame(&mut app, body, |f| {
        f.melee_pressed = true;
        f.melee_held = true;
    });
    app.update();
    set_frame(&mut app, body, |f| {
        f.melee_pressed = false;
        f.melee_held = true;
    });
    let mut walked = vec![playing(&app, body).unwrap_or_default()];
    for _ in 0..24 {
        app.update();
        if let Some(id) = playing(&app, body) {
            if walked.last() != Some(&id) {
                walked.push(id);
            }
        }
    }
    assert_eq!(
        walked,
        vec!["jab".to_string(), "jab2".to_string(), "jab3".to_string()],
        "a held button should walk the authored string; it played {walked:?}"
    );
}

/// ⛔ AND A HOLD MAY NOT START ONE. The sustain continues what is already
/// playing and nothing else — a held button that could open a move would turn
/// every fighter into an auto-attacker the moment somebody rested a thumb.
#[test]
fn a_held_button_never_starts_a_move_by_itself() {
    let moveset = MovesetContract {
        verbs: [(ATTACK_VERB.to_string(), "jab".to_string())]
            .into_iter()
            .collect(),
        moves: vec![
            chain_link("jab", &["jab2"], None),
            chain_link("jab2", &[], None),
        ],
    };
    let (mut app, body) = playing_app(moveset);
    set_frame(&mut app, body, |f| f.melee_held = true);
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(
        playing(&app, body),
        None,
        "a held button with nothing playing started a move"
    );
}

/// ⛔⛔ A HOLD REACHES A SUCCESSOR BY MOVE ID, NEVER BY VERB.
///
/// A cancel window's `into` list mixes two things: MOVE IDS, which are the rest
/// of a string, and VERB NAMES, which are a ROUTE into another part of the
/// vocabulary. George Boo'l's jab names `smash` and `special` — his one way from
/// his fast half to his slow half, bought by connecting. A route is a deliberate
/// DIRECTED press; if a hold could take one, resting on the button after a jab
/// would throw a smash.
#[test]
fn a_hold_cannot_buy_a_route_named_by_verb() {
    let moveset = MovesetContract {
        verbs: [
            (ATTACK_VERB.to_string(), "jab".to_string()),
            (SMASH_VERB.to_string(), "smash_forward".to_string()),
        ]
        .into_iter()
        .collect(),
        // The window names the VERB, exactly as George's does.
        moves: vec![
            chain_link("jab", &["smash"], None),
            chain_link("smash_forward", &[], None),
        ],
    };
    let (mut app, body) = playing_app(moveset);
    set_frame(&mut app, body, |f| {
        f.melee_pressed = true;
        f.melee_held = true;
    });
    app.update();
    set_frame(&mut app, body, |f| {
        f.melee_pressed = false;
        f.melee_held = true;
    });
    for _ in 0..12 {
        app.update();
        assert_ne!(
            playing(&app, body).as_deref(),
            Some("smash_forward"),
            "a held button bought a verb-named route"
        );
    }
}

/// A move that nominates nothing chains nowhere — the press restarts it exactly
/// as it always did.
#[test]
fn a_move_with_no_named_successor_still_restarts() {
    let moveset = MovesetContract {
        verbs: [(ATTACK_VERB.to_string(), "jab".to_string())]
            .into_iter()
            .collect(),
        // Cancelable into ITSELF: the pre-chain authoring, which must keep
        // meaning "press again and throw it again".
        moves: vec![chain_link("jab", &["jab"], None)],
    };
    let (mut app, body) = playing_app(moveset);
    set_frame(&mut app, body, |f| f.melee_pressed = true);
    app.update();
    set_frame(&mut app, body, |f| f.melee_pressed = false);
    for _ in 0..4 {
        app.update();
    }
    let t_before = app.world().get::<MovePlayback>(body).unwrap().t;
    set_frame(&mut app, body, |f| f.melee_pressed = true);
    app.update();
    let pb = app.world().get::<MovePlayback>(body).unwrap();
    assert_eq!(pb.spec.id, "jab");
    assert!(
        pb.t < t_before,
        "a self-naming cancel window stopped restarting its own move"
    );
}

/// ⭐ THE FLURRY LOOPS WHILE THE BUTTON IS DOWN and leaves on the release, into
/// whatever the move authors after the loop — one timeline, not a second move.
#[test]
fn a_held_flurry_repeats_its_authored_window_and_exits_on_release() {
    let looped = ambition_entity_catalog::MoveLoop {
        from_s: 0.02,
        to_s: 0.08,
        max_s: 1.0,
    };
    let moveset = MovesetContract {
        verbs: [(ATTACK_VERB.to_string(), "rapid".to_string())]
            .into_iter()
            .collect(),
        moves: vec![chain_link("rapid", &[], Some(looped))],
    };
    let (mut app, body) = playing_app(moveset);
    set_frame(&mut app, body, |f| {
        f.melee_pressed = true;
        f.melee_held = true;
    });
    app.update();
    set_frame(&mut app, body, |f| f.melee_pressed = false);
    // Well past `to_s`, and past the move's own duration: only the loop can
    // keep it alive this long.
    for _ in 0..30 {
        app.update();
    }
    let pb = app
        .world()
        .get::<MovePlayback>(body)
        .expect("the held flurry ended while the button was still down");
    assert!(
        pb.t < looped.to_s + 1e-3,
        "the clock ran past the loop's end while held (t = {})",
        pb.t
    );
    assert!(pb.looped_s > 0.0, "no lap was ever counted");

    // Let go: the move leaves the loop and finishes.
    set_frame(&mut app, body, |f| {
        f.melee_held = false;
        f.melee_released = true;
    });
    for _ in 0..30 {
        app.update();
        if app.world().get::<MovePlayback>(body).is_none() {
            return;
        }
    }
    panic!("the flurry never left its loop after the button came up");
}

/// The other exit, and the reason it exists: a held button is not a stall.
#[test]
fn a_flurry_held_forever_still_ends() {
    let looped = ambition_entity_catalog::MoveLoop {
        from_s: 0.02,
        to_s: 0.08,
        max_s: 0.20,
    };
    let moveset = MovesetContract {
        verbs: [(ATTACK_VERB.to_string(), "rapid".to_string())]
            .into_iter()
            .collect(),
        moves: vec![chain_link("rapid", &[], Some(looped))],
    };
    let (mut app, body) = playing_app(moveset);
    set_frame(&mut app, body, |f| {
        f.melee_pressed = true;
        f.melee_held = true;
    });
    app.update();
    set_frame(&mut app, body, |f| f.melee_pressed = false);
    for _ in 0..90 {
        app.update();
        if app.world().get::<MovePlayback>(body).is_none() {
            return;
        }
    }
    panic!("a flurry held past its authored maximum never ended, so holding the button is a stall");
}

// ─────────────────────────────────────────────────────────────────────────────
// Sweetspot and sourspot.
//
// One move, two authored volumes, and a victim that must take exactly one of
// them — the one the author wrote first.
// ─────────────────────────────────────────────────────────────────────────────

/// A move whose Active window authors `volumes` in the given order.
fn two_spot_move(id: &str, volumes: Vec<HitVolume>) -> MoveSpec {
    let mut spec = uncancelable(id);
    spec.duration_s = 0.3;
    spec.windows = vec![MoveWindow {
        start_s: 0.0,
        end_s: 0.2,
        tag: WindowTag::Active,
        volumes,
        sustain_effect: None,
        motion_scale: 1.0,
    }];
    spec
}

fn spot(offset: (f32, f32), damage: i32) -> HitVolume {
    HitVolume {
        hit_sfx: None,
        shape: ambition_entity_catalog::VolumeShape::Rect {
            offset,
            half_extents: (30.0, 30.0),
        },
        damage,
        knockback: 100.0,
        knockback_growth: None,
        launch_dir: None,
        on_hit: None,
        vfx: None,
        autolink: None,
    }
}

/// The full chain plus damage resolution, with one victim standing where BOTH
/// volumes reach it.
fn two_spot_app(volumes: Vec<HitVolume>) -> (App, Entity) {
    let moveset = MovesetContract {
        verbs: [(ATTACK_VERB.to_string(), "two_spots".to_string())]
            .into_iter()
            .collect(),
        moves: vec![two_spot_move("two_spots", volumes)],
    };
    let (mut app, body) = playing_app(moveset);
    app.init_resource::<Captured>();
    app.add_systems(
        Update,
        (apply_hitbox_damage, capture)
            .chain()
            .after(advance_move_playback),
    );
    app.world_mut().spawn((
        ActorFaction::Enemy,
        ae::CenteredAabb::from_center_size(ae::Vec2::new(24.0, 0.0), ae::Vec2::new(20.0, 40.0)),
        ambition_platformer2d_core::BodyOffense::default(),
        ambition_platformer2d_core::BodyMotionFacts::default(),
        ambition_platformer2d_core::BodyShieldState::default(),
        ambition_characters::actor::BodyCombat::default(),
        ambition_characters::actor::BodyHealth::restored(
            ambition_characters::actor::Health::new(100),
            0,
            Default::default(),
        ),
    ));
    (app, body)
}

fn swing_and_collect(app: &mut App, body: Entity) -> Vec<i32> {
    set_frame(app, body, |f| f.melee_pressed = true);
    app.update();
    set_frame(app, body, |f| f.melee_pressed = false);
    for _ in 0..12 {
        app.update();
    }
    app.world()
        .resource::<Captured>()
        .hits
        .iter()
        .filter(|h| matches!(h.target, crate::events::HitTarget::Body(_)))
        .map(|h| h.damage)
        .collect()
}

/// ⭐ ONE SWING IS ONE HIT, and it is the volume the author wrote first.
///
/// Measured before this rule existed: a single press with a tip and a base
/// overlapping one body produced TWO damage events and two knockbacks, 15 then
/// 4. A move with a good and a bad way to land it read as "land it twice",
/// which is what made the sweetspot vocabulary unusable rather than merely
/// missing.
#[test]
fn a_body_reached_by_both_spots_takes_only_the_first_authored_one() {
    let (mut app, body) = two_spot_app(vec![spot((20.0, 0.0), 15), spot((22.0, 0.0), 4)]);
    assert_eq!(
        swing_and_collect(&mut app, body),
        vec![15],
        "one swing landed more than one of its own volumes on one body"
    );
}

/// The order is the AUTHORING, not the damage: reverse the list and the weak
/// one wins, because an author who writes the base first meant the base.
#[test]
fn reversing_the_authored_order_reverses_which_spot_lands() {
    let (mut app, body) = two_spot_app(vec![spot((22.0, 0.0), 4), spot((20.0, 0.0), 15)]);
    assert_eq!(
        swing_and_collect(&mut app, body),
        vec![4],
        "the arbitration is picking by damage or by query order rather than by \
         the order the move lists its volumes in"
    );
}

/// ⛔ AND A MOVE THAT IS GENUINELY MULTI-HIT IS UNTOUCHED. Volumes that do not
/// overlap the same body still each land: the rule stands down a loser only
/// where a better-ranked sibling actually REACHES the victim, so a drill whose
/// two boxes cover different bodies is not quietly halved.
#[test]
fn volumes_that_do_not_both_reach_the_body_both_still_land() {
    let (mut app, body) = two_spot_app(vec![spot((20.0, 0.0), 15), spot((400.0, 0.0), 4)]);
    // A SECOND body, standing in the far volume and nowhere near the near one.
    app.world_mut().spawn((
        ActorFaction::Enemy,
        ae::CenteredAabb::from_center_size(ae::Vec2::new(400.0, 0.0), ae::Vec2::new(20.0, 40.0)),
        ambition_platformer2d_core::BodyOffense::default(),
        ambition_platformer2d_core::BodyMotionFacts::default(),
        ambition_platformer2d_core::BodyShieldState::default(),
        ambition_characters::actor::BodyCombat::default(),
        ambition_characters::actor::BodyHealth::restored(
            ambition_characters::actor::Health::new(100),
            0,
            Default::default(),
        ),
    ));
    let mut landed = swing_and_collect(&mut app, body);
    landed.sort_unstable();
    assert_eq!(
        landed,
        vec![4, 15],
        "each body should take the one volume that reaches IT — the rule is \
         standing a volume down globally instead of per victim"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// THE STRIKE PULSE — one continuous Active interval, one per-victim ledger.
//
// ⭐⭐ The sweetspot arbitration resolves a TIE: which of two volumes takes a
// body reached by BOTH on one tick. That is not "one swing lands once", and the
// tests above cannot tell the two apart because they stand the victim where
// both volumes overlap at once. These stand it where only ONE reaches, and move
// it.
// ─────────────────────────────────────────────────────────────────────────────

/// A narrow spot, so a body can be inside one and outside its sibling.
fn narrow_spot(offset: (f32, f32), damage: i32) -> HitVolume {
    HitVolume {
        hit_sfx: None,
        shape: ambition_entity_catalog::VolumeShape::Rect {
            offset,
            half_extents: (8.0, 30.0),
        },
        damage,
        knockback: 100.0,
        knockback_growth: None,
        launch_dir: None,
        on_hit: None,
        vfx: None,
        autolink: None,
    }
}

/// The two-spot chain with the victim's position under the caller's control,
/// and a move whose Active stretch is authored by the caller.
fn pulse_app(spec: MoveSpec, victim_x: f32) -> (App, Entity, Entity) {
    let moveset = MovesetContract {
        verbs: [(ATTACK_VERB.to_string(), spec.id.clone())]
            .into_iter()
            .collect(),
        moves: vec![spec],
    };
    let (mut app, body) = playing_app(moveset);
    app.init_resource::<Captured>();
    app.add_systems(
        Update,
        (apply_hitbox_damage, capture)
            .chain()
            .after(advance_move_playback),
    );
    let victim = app
        .world_mut()
        .spawn((
            ActorFaction::Enemy,
            ae::CenteredAabb::from_center_size(
                ae::Vec2::new(victim_x, 0.0),
                ae::Vec2::new(8.0, 40.0),
            ),
            ambition_platformer2d_core::BodyOffense::default(),
            ambition_platformer2d_core::BodyMotionFacts::default(),
            ambition_platformer2d_core::BodyShieldState::default(),
            ambition_characters::actor::BodyCombat::default(),
            ambition_characters::actor::BodyHealth::restored(
                ambition_characters::actor::Health::new(100),
                0,
                Default::default(),
            ),
        ))
        .id();
    (app, body, victim)
}

fn stand_at(app: &mut App, victim: Entity, x: f32) {
    let mut aabb = app
        .world_mut()
        .get_mut::<ae::CenteredAabb>(victim)
        .expect("the victim publishes a footprint");
    aabb.center = ae::Vec2::new(x, 0.0);
}

/// Swing, then walk the victim through `stations` — one tick each.
fn swing_across(app: &mut App, body: Entity, victim: Entity, stations: &[f32]) -> Vec<i32> {
    set_frame(app, body, |f| f.melee_pressed = true);
    app.update();
    set_frame(app, body, |f| f.melee_pressed = false);
    for x in stations {
        stand_at(app, victim, *x);
        app.update();
    }
    for _ in 0..12 {
        app.update();
    }
    app.world()
        .resource::<Captured>()
        .hits
        .iter()
        .filter(|h| matches!(h.target, crate::events::HitTarget::Body(_)))
        .map(|h| h.damage)
        .collect()
}

/// A move whose Active stretch is TWO CONTIGUOUS windows with DIFFERENT volume
/// counts and orders — the case the by-index handoff could not express.
fn two_keyframe_move(id: &str, first: Vec<HitVolume>, second: Vec<HitVolume>) -> MoveSpec {
    let mut spec = uncancelable(id);
    spec.duration_s = 0.4;
    spec.windows = vec![
        MoveWindow {
            start_s: 0.0,
            end_s: 0.05,
            tag: WindowTag::Active,
            volumes: first,
            sustain_effect: None,
            motion_scale: 1.0,
        },
        MoveWindow {
            start_s: 0.05,
            end_s: 0.25,
            tag: WindowTag::Active,
            volumes: second,
            sustain_effect: None,
            motion_scale: 1.0,
        },
    ];
    spec
}

/// ⭐⭐ SOUR THEN SWEET IS ONE HIT.
///
/// The victim is struck by the far volume on one tick and steps into the near
/// one on the next. Before the pulse ledger the near volume's OWN ledger was
/// empty and no sibling reached the body at that instant, so the same swing
/// landed a second time for a second damage and a second knockback.
#[test]
fn stepping_from_the_sour_spot_into_the_sweet_one_is_still_one_hit() {
    let (mut app, body, victim) = pulse_app(
        two_spot_move(
            "two_spots",
            vec![narrow_spot((0.0, 0.0), 15), narrow_spot((60.0, 0.0), 4)],
        ),
        60.0,
    );
    assert_eq!(
        swing_across(&mut app, body, victim, &[60.0, 60.0, 0.0, 0.0]),
        vec![4],
        "one swing landed twice across ticks: the sour spot, then the sweet one"
    );
}

/// ⭐⭐ AND SWEET THEN SOUR, which is the same defect the other way round and
/// would survive a fix that only remembered the BEST hit.
#[test]
fn stepping_from_the_sweet_spot_into_the_sour_one_is_still_one_hit() {
    let (mut app, body, victim) = pulse_app(
        two_spot_move(
            "two_spots",
            vec![narrow_spot((0.0, 0.0), 15), narrow_spot((60.0, 0.0), 4)],
        ),
        0.0,
    );
    assert_eq!(
        swing_across(&mut app, body, victim, &[0.0, 0.0, 60.0, 60.0]),
        vec![15],
        "one swing landed twice across ticks: the sweet spot, then the sour one"
    );
}

/// ⛔⛔ AND ACROSS A KEYFRAME THAT CHANGES THE VOLUME COUNT AND ORDER.
///
/// The predecessor handed a closing window's ledgers to the next one BY VOLUME
/// INDEX — "volume `v` hands to volume `v`" — so a swing that authors one
/// volume in its first keyframe and two in its second gave the first's memory to
/// whichever volume happened to be written first, and the other started empty.
/// The pulse owns the ledger, so the ordinal cannot matter.
#[test]
fn a_keyframe_that_changes_its_volumes_does_not_reset_the_pulse() {
    let (mut app, body, victim) = pulse_app(
        two_keyframe_move(
            "two_keyframes",
            // One volume, far out.
            vec![narrow_spot((60.0, 0.0), 4)],
            // Two volumes, near one FIRST — so index 0 is a different volume in
            // the second keyframe than it was in the first.
            vec![narrow_spot((0.0, 0.0), 15), narrow_spot((60.0, 0.0), 4)],
        ),
        60.0,
    );
    assert_eq!(
        swing_across(&mut app, body, victim, &[60.0, 0.0, 0.0, 0.0]),
        vec![4],
        "the pulse's ledger did not survive a keyframe whose volume list changed \
         shape, so the swing landed again out of the new window"
    );
}

/// ⭐ AND A REAL GAP EARNS A SECOND HIT, which is what a multi-hit IS.
///
/// the poison for all three above: a fix that simply never let a move
/// hit twice would pass them and delete the drill, the rapid jab and every
/// pulsing move in the genre.
#[test]
fn a_gap_in_active_time_starts_a_new_pulse_and_hits_again() {
    let mut spec = uncancelable("two_pulses");
    spec.duration_s = 0.5;
    spec.windows = vec![
        MoveWindow {
            start_s: 0.0,
            end_s: 0.05,
            tag: WindowTag::Active,
            volumes: vec![narrow_spot((0.0, 0.0), 15)],
            sustain_effect: None,
            motion_scale: 1.0,
        },
        // THE GAP: no Active coverage between 0.05 and 0.20.
        MoveWindow {
            start_s: 0.20,
            end_s: 0.35,
            tag: WindowTag::Active,
            volumes: vec![narrow_spot((0.0, 0.0), 15)],
            sustain_effect: None,
            motion_scale: 1.0,
        },
    ];
    let (mut app, body, victim) = pulse_app(spec, 0.0);
    assert_eq!(
        swing_across(&mut app, body, victim, &[0.0, 0.0, 0.0, 0.0]),
        vec![15, 15],
        "a move whose Active stretch has a GAP in it is a multi-hit, and its \
         second pulse must reach a body its first already hit"
    );
}

/// ⭐⭐ D196: A BUFFERED SPECIAL IS REPLAYED VERBATIM, NOT RE-READ OFF THE STICK.
///
/// ⛔⛔ `BodyActionBuffer::special` was a bare TIMER while the attack slot beside
/// it carried a whole intent, and the buffer's own doc already said why that is
/// wrong: *"a buffered press must be replayed verbatim rather than reinterpreted
/// from the live stick later."* Replay called `attack_dir_from_axis` on the LIVE
/// frame, so:
///
/// ```text
///   press Up+Special while the action is refused  -> buffered
///   let the stick centre                          -> the axis is Neutral now
///   the refusal lifts                             -> a NEUTRAL special comes out
/// ```
///
/// Out of shield it was worse: the out-of-shield rule asks whether the press
/// RISES, so a buffered up-special replayed off a centred stick no longer even
/// QUALIFIED as one.
#[test]
fn a_buffered_up_special_replays_as_an_up_special_after_the_stick_centres() {
    let make = |id: &str, duration_s: f32| MoveSpec {
        display_name: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s,
        windows: vec![],
        events: vec![],
        gates: Default::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        repeat: None,
        landing_lag_s: None,
        autocancel_after_s: None,
    };
    let moveset = MovesetContract {
        verbs: std::collections::BTreeMap::from([
            (ATTACK_VERB.to_string(), "jab".to_string()),
            (SPECIAL_VERB.to_string(), "neutral_special".to_string()),
            ("special_up".to_string(), "up_special".to_string()),
        ]),
        moves: vec![
            make("jab", 0.05),
            make("neutral_special", 0.3),
            make("up_special", 0.3),
        ],
    };
    let (mut app, body) = playing_app(moveset);
    // A real leniency window, so the refused press survives to be replayed.
    app.world_mut()
        .get_mut::<AttackGestureTuning>(body)
        .expect("ActorControl requires the gesture tuning")
        .action_buffer_s = 0.20;

    // The body commits to a jab, so the special below is REFUSED rather than
    // accepted — a move is playing and it authors no cancel window.
    set_frame(&mut app, body, |f| f.melee_pressed = true);
    app.update();
    assert_eq!(
        app.world()
            .get::<MovePlayback>(body)
            .map(|pb| pb.spec.id.clone()),
        Some("jab".to_string()),
        "the fixture never started the move that refuses the special"
    );

    // UP + SPECIAL, into the refusal. `-y` is up: the axis is gravity-relative.
    set_frame(&mut app, body, |f| {
        f.melee_pressed = false;
        f.special_pressed = true;
        f.attack_axis = ae::LocalAxes::new(0.0, -1.0);
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<MovePlayback>(body)
            .map(|pb| pb.spec.id.clone()),
        Some("jab".to_string()),
        "the special was ACCEPTED during the jab, so this fixture never exercised \
         the buffer at all"
    );

    // The stick centres and the button comes up. Everything the replay could
    // read off the live frame now says NEUTRAL.
    set_frame(&mut app, body, |f| {
        f.special_pressed = false;
        f.attack_axis = ae::LocalAxes::ZERO;
    });
    for _ in 0..6 {
        app.update();
        if app
            .world()
            .get::<MovePlayback>(body)
            .is_some_and(|pb| pb.spec.id != "jab")
        {
            break;
        }
    }

    assert_eq!(
        app.world()
            .get::<MovePlayback>(body)
            .map(|pb| pb.spec.id.clone()),
        Some("up_special".to_string()),
        "the buffered press came back as a different move than the one that was \
         made — the replay re-read the direction off a stick that has since \
         centred"
    );
}

/// ⛔⛔ A USE THAT NEVER ENTERED CHARGE MODE PAYS NOTHING, at every instant a
/// strike could spawn.
///
/// The deleted road read `smash_charge_mult` against how far the clock had run
/// through the leading Startup window. That sounds like a partial payoff and is
/// not one: a strike volume only spawns INSIDE an Active window, Active begins
/// where that Startup window ends, and the fraction clamps — so the timeline
/// reading was the FULL multiplier on every hit of every non-charging use.
///
/// ⭐ THE ASSERTION SWEEPS THE ACTIVE WINDOW rather than sampling `t = 0`,
/// which is the sample that made the old road look like an interpolation.
#[test]
fn a_move_used_without_the_smash_gesture_lands_at_unit_scale() {
    let spec = charging_smash();
    assert!(
        spec.smash_charge_mult > 1.0,
        "the fixture authors no payoff, so this cannot observe one being denied"
    );
    let mut pb = MovePlayback::new(spec, 1.0).charged_by_smash_gesture(false);
    let mut sampled = 0;
    while !pb.finished() {
        assert_eq!(
            pb.charge_scale(),
            1.0,
            "a use that never charged is paying the smash multiplier at t={}",
            pb.t
        );
        sampled += 1;
        pb.t += 1.0 / 60.0;
    }
    assert!(
        sampled > 1,
        "the sweep never advanced, so it proved nothing"
    );
}

/// The other direction, or the fix above would just have deleted the payoff:
/// a real smash gesture held to full still pays the whole multiplier.
#[test]
fn a_held_smash_gesture_still_pays_its_multiplier() {
    let mut pb = MovePlayback::new(charging_smash(), 1.0).charged_by_smash_gesture(true);
    let charge = pb
        .charge
        .expect("the fixture authors a policy, so a smash charges");
    assert_eq!(pb.charge_scale(), 1.0, "an untouched charge is a tap");
    pb.charge = Some({
        let mut full = charge;
        full.held_s = CHARGE_MAX_HOLD_S;
        full
    });
    assert!(
        (pb.charge_scale() - CHARGE_MULT).abs() < 1e-6,
        "a fully held smash no longer pays: {}",
        pb.charge_scale()
    );
}

/// ⛔⛔ A HELD CHARGE HAS NO LIVE STRIKE, ON EVERY HELD TICK — and the strike
/// appears only on release, on entering Active.
///
/// The defect this pins was structural rather than authored: the derived hold
/// point came from the leading Startup window's `end_s`, ordinary smash
/// authoring lays Active directly against Startup, and Active membership is
/// `start_s <= t < end_s`. So the freeze landed on the FIRST ACTIVE INSTANT and
/// a fighter could stand in a charge with a live hitbox already spawned, for as
/// long as it liked.
///
/// ⭐ IT HOLDS THE BUTTON THE WAY A HAND DOES. `ResolvedAttackGesture::held` is
/// what the playback reads to keep charging, so this fixture writes that and
/// then CLEARS it — a charge released by a real release, not by a field poked
/// to its maximum.
///
/// ⭐ AND IT DOES NOT TRUST THE AUTHORING: the move lays its Active window flush
/// against its Startup, which is the shape that produced the bug, so a hold
/// point drifting back to the window edge fails here rather than being hidden
/// by a fixture that left a gap.
#[test]
fn a_held_charge_has_no_live_strike_and_releases_into_one() {
    use ambition_characters::actor::attack_gesture::{AttackGestureIntent, ResolvedAttackGesture};

    let spec = charging_smash();
    let startup_end = spec
        .windows
        .iter()
        .find(|w| matches!(w.tag, ambition_entity_catalog::WindowTag::Startup))
        .map(|w| w.end_s)
        .expect("the fixture authors a windup");
    let first_active = spec
        .windows
        .iter()
        .filter(|w| matches!(w.tag, ambition_entity_catalog::WindowTag::Active))
        .map(|w| w.start_s)
        .fold(f32::MAX, f32::min);
    assert_eq!(
        startup_end, first_active,
        "the fixture leaves a gap between windup and strike, so it cannot \
         observe a freeze landing inside one"
    );

    let (mut app, _victim) = app_with_victim();
    let attacker = spawn_attacker(
        &mut app,
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(15.0, 24.0),
        spec,
    );
    let held = AttackGestureIntent {
        direction: AttackDir::Forward,
        strength: ambition_characters::actor::attack_gesture::AttackStrength::Smash,
        posture: ambition_characters::actor::attack_gesture::AttackPosture::Grounded,
        phase: ambition_characters::actor::attack_gesture::AttackInputPhase::Hold,
    };
    app.world_mut().entity_mut(attacker).insert((
        MovePlayback::new(charging_smash(), 1.0).charged_by_smash_gesture(true),
        ResolvedAttackGesture {
            pressed: None,
            held: Some(held),
            released: None,
            special: None,
        },
    ));

    let live_strikes = |app: &mut App| -> usize {
        let mut q = app.world_mut().query::<&StrikeVolume>();
        q.iter(app.world()).filter(|v| v.owner == attacker).count()
    };

    // ── held ────────────────────────────────────────────────────────────────
    // ⛔ SHORTER THAN `max_hold_s`, deliberately. A full charge is LOADED, not
    // stored — the move fires itself at the authored maximum whether or not the
    // button is down — so a longer hold here would be observing the auto-release
    // rather than the hold.
    let mut rooted_ticks = 0;
    for _ in 0..12 {
        app.update();
        let Some(pb) = app.world().entity(attacker).get::<MovePlayback>() else {
            panic!("the move fired before its authored maximum with the button down");
        };
        if pb.rooted_by_charge() {
            rooted_ticks += 1;
            let t = pb.t;
            assert!(
                t < first_active,
                "the charge froze at t={t}, and the first strike goes live at \
                 {first_active}"
            );
            assert_eq!(
                live_strikes(&mut app),
                0,
                "a fighter is holding a charge with a live strike volume \
                 already spawned"
            );
        }
    }
    assert!(
        rooted_ticks > 3,
        "the charge never rooted, so nothing above observed a held charge \
         ({rooted_ticks} rooted tick(s))"
    );
    assert_eq!(
        live_strikes(&mut app),
        0,
        "the whole hold produced a live strike"
    );

    // ── released ────────────────────────────────────────────────────────────
    app.world_mut()
        .entity_mut(attacker)
        .insert(ResolvedAttackGesture {
            pressed: None,
            held: None,
            released: None,
            special: None,
        });
    let mut saw_strike = false;
    for _ in 0..120 {
        app.update();
        let Some(pb) = app.world().entity(attacker).get::<MovePlayback>() else {
            break;
        };
        let t = pb.t;
        if live_strikes(&mut app) > 0 {
            saw_strike = true;
            assert!(
                t >= first_active,
                "a strike volume exists at t={t}, before the first Active \
                 window opens at {first_active}"
            );
        }
    }
    assert!(
        saw_strike,
        "releasing the charge never produced a strike at all, so the freeze is \
         eating the swing rather than delaying it"
    );
}

/// ⛔⛔ A DIRECTION HELD BEFORE THE GRAB IS NOT A THROW COMMAND.
///
/// You walk into a grab, so the stick that reached it is already pointing
/// somewhere. Reading the live axis on the first captive tick threw the victim
/// the instant the capture landed — no pummel, no choice, and no way to hold
/// somebody while you decided. The genre's rule is *press a direction after the
/// grab connects*, and a press needs a baseline.
///
/// ⭐ THE REAL ARMING SYSTEM IS IN THE CHAIN. `arm_smash_throw_edge` is what
/// sets the edge in production, and it is what sets it here — a fixture that
/// poked `throw_armed` itself would prove only that the field is readable.
#[test]
fn a_direction_held_through_the_grab_does_not_throw_until_it_is_pressed_again() {
    use ambition_characters::actor::control::ActorControlFrame;

    let mut app = App::new();
    app.add_message::<MoveEventMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.init_resource::<WorldTime>();
    app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
    app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
    app.add_systems(
        Update,
        (
            crate::capture::systems::arm_smash_throw_edge,
            resolve_attack_gestures,
            buffer_combat_action_presses,
            trigger_moveset_moves,
        )
            .chain(),
    );

    // Walking into the grab: forward on the stick before it connects.
    let mut frame = ActorControlFrame::default();
    frame.attack_axis = ae::LocalAxes::X;
    let captor = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                facing: 1.0,
                ..Default::default()
            },
            ActorFaction::Enemy,
            ActorMoveset(capture_context_moveset()),
            ActorControl(frame),
        ))
        .id();
    app.world_mut().spawn((
        crate::capture::CapturedBy {
            captor,
            hold_offset_local: ae::Vec2::new(16.0, 0.0),
            prior_gravity_scale: 1.0,
        },
        // As acquisition writes it. THE FIXTURE DOES NOT ARM THIS.
        ambition_characters::smash_capture::SmashHoldState::default(),
    ));

    let set_axis = |app: &mut App, axis: ae::LocalAxes| {
        let mut entity = app.world_mut().entity_mut(captor);
        let mut control = entity
            .get_mut::<ActorControl>()
            .expect("the captor carries a control frame");
        control.0.attack_axis = axis;
    };

    // ── still holding forward ───────────────────────────────────────────────
    for _ in 0..8 {
        app.update();
        assert_eq!(
            played(&app, captor),
            None,
            "a direction that was already held when the grab connected threw \
             the captive; the captor never pressed anything"
        );
        set_axis(&mut app, ae::LocalAxes::X);
    }

    // ── back to neutral: the edge arms ──────────────────────────────────────
    set_axis(&mut app, ae::LocalAxes::ZERO);
    app.update();
    assert_eq!(
        played(&app, captor),
        None,
        "centring the stick threw the captive — neutral is not a direction"
    );

    // ── and now a direction IS a press ──────────────────────────────────────
    set_axis(&mut app, ae::LocalAxes::X);
    app.update();
    assert_eq!(
        played(&app, captor).as_deref(),
        Some("fthrow"),
        "a direction pressed after the capture did not throw, so the edge \
         armed and then refused the press it was armed for"
    );
}

/// ⛔ THE ATTACK PRESS DOES NOT WAIT FOR THE EDGE, and it must not: a press IS
/// an edge — the input event, not the stick's position — so Attack+direction
/// throws exactly as it always did and Attack alone still pummels. Both are
/// asked here on a capture that has NEVER been armed, which is the state every
/// grab starts in and the state that would break them if the gate were put in
/// the wrong place.
#[test]
fn an_attack_press_throws_and_pummels_on_a_capture_that_never_armed() {
    use ambition_characters::actor::control::ActorControlFrame;

    let build = |axis: ae::LocalAxes| {
        let mut app = App::new();
        app.add_message::<MoveEventMessage>();
        app.add_message::<ambition_vfx::vfx::VfxMessage>();
        app.init_resource::<WorldTime>();
        app.world_mut().resource_mut::<WorldTime>().scaled_dt = 0.016;
        app.world_mut().resource_mut::<WorldTime>().raw_dt = 0.016;
        app.add_systems(
            Update,
            (
                crate::capture::systems::arm_smash_throw_edge,
                resolve_attack_gestures,
                buffer_combat_action_presses,
                trigger_moveset_moves,
            )
                .chain(),
        );
        let mut frame = ActorControlFrame::default();
        frame.attack_axis = axis;
        frame.melee_pressed = true;
        let captor = app
            .world_mut()
            .spawn((
                ae::BodyKinematics {
                    facing: 1.0,
                    ..Default::default()
                },
                ActorFaction::Enemy,
                ActorMoveset(capture_context_moveset()),
                ActorControl(frame),
            ))
            .id();
        app.world_mut().spawn((
            crate::capture::CapturedBy {
                captor,
                hold_offset_local: ae::Vec2::new(16.0, 0.0),
                prior_gravity_scale: 1.0,
            },
            ambition_characters::smash_capture::SmashHoldState::default(),
        ));
        app.update();
        played(&app, captor)
    };

    assert_eq!(
        build(ae::LocalAxes::X).as_deref(),
        Some("fthrow"),
        "Attack+forward stopped throwing on an unarmed capture — the edge was \
         put in front of the press road as well as the stick road"
    );
    assert_eq!(
        build(ae::LocalAxes::ZERO).as_deref(),
        Some("pummel"),
        "a neutral attack press stopped pummelling"
    );
}

/// ⭐⭐ THE WHOLE CHARGE CHAIN, DETERMINISTICALLY, WITH NO MATCH IN IT.
///
/// The claim "a CPU charges its smashes" was guarded only by
/// `the_cpu_charges_a_smash_and_techs_a_landing_in_some_match`, which plays
/// three ninety-second matches and asks whether a charge appeared in any of
/// them. That test is real and worth keeping, but it cannot say WHERE a break
/// is: it has gone red for a dodge-semantics change and for a charge-payoff
/// change, neither of which touches charging, and each time the reading was
/// "the sampled matches stopped producing openings".
///
/// This walks the production chain end to end instead:
///
/// ```text
///   fighter brain, in Advantage      (the opponent is in hitstun)
///        ↓ melee_pressed + melee_strong_hint
///   resolve_attack_gestures           → AttackStrength::Smash
///        ↓
///   trigger_moveset_moves             → MovePlayback, charged_by_smash_gesture
///        ↓ Attack still held
///   the clock reaches the hold point  → MoveCharge arms, fraction rises
///        ↓ release
///   the frozen fraction pays the hit
/// ```
///
/// A break anywhere in it fails HERE, on a fixture with one opponent that never
/// moves and a brain with no reaction delay and no noise.
#[test]
fn a_fighter_brain_charges_a_smash_through_the_real_chain() {
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::fighter::{
        decision::tick_fighter, FighterBrainProfile, FighterCfg, FighterState,
    };
    use ambition_characters::perception::{BodyPhase, PerceivedActor, SelfView, WorldView};

    // No reaction delay, no APM cap, no execution noise: every tick of this is
    // the policy, not the sampling.
    let cfg = FighterCfg::new(FighterBrainProfile {
        level: 5,
        reaction_ms: 0.0,
        apm_cap: 0.0,
        execution_noise: 0.0,
        rollout_depth: 0,
        rollout_k: 0,
        read_weight: 0.5,
        utility_weights: Default::default(),
    });
    let mut state = FighterState::new(&cfg, 0x5EED);

    // The opponent is IN HITSTUN and within reach: `Situation::Advantage`, which
    // is the one situation whose charge budget is a full hold.
    let view = WorldView {
        self_view: SelfView {
            pos: ae::Vec2::new(300.0, 300.0),
            gravity_down: ae::Vec2::new(0.0, 1.0),
            faction: ActorFaction::Player,
            alive: true,
            on_ground: true,
            ..Default::default()
        },
        actors: vec![PerceivedActor {
            id: "foe".to_string(),
            pos: ae::Vec2::new(340.0, 300.0),
            faction: ActorFaction::Enemy,
            hostile_to_self: true,
            alive: true,
            on_ground: true,
            phase: BodyPhase::Hitstun,
            ..Default::default()
        }],
        // ⛔ A REAL STAGE. A default `StageView` has zero-sized bounds, and the
        // brain normalises positions against them — the fixture's first output
        // frame carried `locomotion.x = NaN`, which loses every comparison it
        // is in, so the policy silently declined to press anything at all.
        stage: ambition_characters::perception::StageView {
            bounds: ae::Aabb::new(ae::Vec2::new(400.0, 300.0), ae::Vec2::new(400.0, 300.0)),
        },
        ..Default::default()
    };
    // ⛔ THE KIT IS THE POINT. A brain selects from `attack_kit`, so an idle
    // snapshot carries no smash to select and the assertion below would be
    // measuring an empty repertoire rather than a policy. The candidate is
    // built from the SAME `charging_smash()` spec the body plays, so the frame
    // data the brain reasons about and the timeline the charge freezes on are
    // one authoring.
    use ambition_characters::brain::fighter::options::{
        ActionLegality, AttackBinding, AttackCandidate, AttackVerb,
    };
    let mut snapshot = ambition_characters::brain::BrainSnapshot::idle();
    snapshot.attack_kit = vec![AttackCandidate {
        move_id: charging_smash().id.clone(),
        frames: charging_smash().frame_data(),
        binding: AttackBinding {
            verb: AttackVerb::Smash,
            direction: AttackDir::Forward,
        },
        legality: ActionLegality::Now,
    }];
    snapshot.actor_pos = ae::Vec2::new(300.0, 300.0);
    snapshot.actor_facing = 1.0;
    snapshot.actor_on_ground = true;
    snapshot.target_pos = ae::Vec2::new(340.0, 300.0);
    snapshot.target_alive = true;
    snapshot.alive = true;
    snapshot.world_size = ae::Vec2::new(800.0, 600.0);

    // ── the body half: the SAME frames the brain emits, into the real chain ──
    let (mut app, body) = smash_charge_app();

    let mut out = ActorControlFrame::neutral();
    let mut pressed_smash = false;
    let mut armed_charge = false;
    let mut peak_fraction = 0.0f32;
    let mut rooted_ticks = 0usize;
    for _ in 0..240 {
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
        pressed_smash |= out.melee_pressed && out.melee_strong_hint;
        let published = out.clone();
        set_frame(&mut app, body, move |f| *f = published.clone());
        app.update();
        if let Some(pb) = app.world().get::<MovePlayback>(body) {
            if let Some(charge) = pb.charge {
                armed_charge = true;
                peak_fraction = peak_fraction.max(charge.fraction());
                if pb.rooted_by_charge() {
                    rooted_ticks += 1;
                }
            }
        }
    }

    assert!(
        pressed_smash,
        "the brain never asked for a SMASH against an opponent in hitstun at \
         melee range, so nothing below can observe a charge"
    );
    assert!(
        armed_charge,
        "a smash the brain pressed never resolved a `MoveCharge` — the gesture \
         reached the body without its SMASH strength, or the move resolved no \
         charge policy"
    );
    assert!(
        rooted_ticks > 0,
        "the charge armed but the timeline never froze, so the body walked \
         straight through the hold point"
    );
    assert!(
        peak_fraction > 0.0,
        "the charge froze but never accrued: the brain let go of Attack before \
         the clock reached the hold point, which is the exact defect \
         `hold_ticks` exists to prevent ({rooted_ticks} rooted tick(s))"
    );
}

/// ⛔⛔ A RECOVERY IS ONCE PER AIRTIME, AND WITHOUT THAT A PLATFORM FIGHTER HAS
/// NO BOTTOM BLASTZONE.
///
/// Measured at the source 2026-08-24: `MoveSpec` carries no cooldown, no cost
/// and no per-airtime rule, and `MoveGates` knew only `grounded` — which cannot
/// tell the second use in one airtime from the first. A fighter authoring a
/// rising special could press it forever and could only be killed by a launch
/// that outran its own recovery.
///
/// ⭐ ALL THREE STATES, because a budget is a fence and a fence needs both
/// sides: an ordinary move never asks; a recovery with charges left is allowed;
/// the same recovery with none is refused.
#[test]
fn a_recovery_is_refused_once_its_budget_is_spent() {
    use ambition_entity_catalog::{MoveGates, MoveSpec};

    let recovery = |gates: MoveGates| MoveSpec {
        gates,
        ..ambition_characters::moveset_authoring::strike(
            ambition_characters::moveset_authoring::Strike {
                id: "rise",
                clip: "attack_up",
                startup_s: 0.05,
                active_s: 0.05,
                recover_s: 0.10,
                offset: (0.0, -10.0),
                half_extents: (10.0, 10.0),
                damage: 1,
                knockback: 10.0,
                knockback_growth: 0.0,
                launch_dir: None,
                on_hit: None,
            },
        )
    };
    let spends = recovery(MoveGates {
        spends_recovery: true,
        ..Default::default()
    });
    let ordinary = recovery(MoveGates::default());

    assert!(
        super::afford_recovery(&spends, Some(1)),
        "a fighter with a recovery left was refused one"
    );
    assert!(
        !super::afford_recovery(&spends, Some(0)),
        "a fighter with NO recovery left got another — it can never be killed \
         off the side or the bottom"
    );
    assert!(
        super::afford_recovery(&ordinary, Some(0)),
        "an ordinary move asked the recovery budget, so an exhausted fighter \
         cannot jab"
    );
    assert!(
        super::afford_recovery(&spends, None),
        "a body with no jump cluster is a bare fixture, not a fighter with an \
         exhausted budget"
    );
}

/// ... AND BEING RE-SEATED GIVES IT BACK. Landing, catching the ledge, being
/// grabbed and a respawn all run the landing-class refresh, which is the one
/// place the budget is restored — and deliberately NOT a hit, which would refund
/// a recovery to the fighter being edge-guarded.
#[test]
fn the_landing_class_refresh_restores_the_recovery_budget() {
    let abilities = ae::BodyAbilities::default();
    let mut dash = ae::BodyDashState::default();
    let mut dodge = ae::BodyDodgeState::default();
    let mut jump = ae::BodyJumpState {
        recovery_charges: 0,
        ..Default::default()
    };
    ae::refresh_movement_resources_clusters(&abilities, &mut dash, &mut jump, &mut dodge, 1);
    assert_eq!(
        jump.recovery_charges,
        ae::DEFAULT_RECOVERY_CHARGES,
        "being re-seated did not give the recovery back, so one trip offstage \
         retires the fighter"
    );
    assert!(
        ae::DEFAULT_RECOVERY_CHARGES > 0,
        "poison: the default is zero, so the assertion above would hold for a \
         refresh that restores nothing"
    );
}
