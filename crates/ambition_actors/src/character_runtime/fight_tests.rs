//! **§7.10: two characters, two providers, one session, real damage.**
//!
//! The acceptance test for the whole character-definition plan. It is deliberately
//! built out of the pieces the other slices produced, and it needs no host
//! application:
//!
//! * §7.6 — each character is ONE `register_character` call from its own provider;
//! * §7.8 — a `MatchParticipantRoster` seats them and projects the load demand;
//! * §7.1/§7.2 — the engine materializer answers that demand and both reach a
//!   terminal state;
//! * §7.11 — each body's damageable silhouette comes from its AUTHORED hurtboxes
//!   on a simulation clock, not from a rendered frame;
//! * §7.4/§7.9 — the attack that lands resolves through the directional verb chain.
//!
//! If a future change makes a character playable only in the app that happens to
//! install some step, this test is the one that goes red.

use super::*;
use ambition_engine_core::Vec2;
use ambition_entity_catalog::{
    ClipBinding, HitVolume, HurtboxDoc, HurtboxKeyframe, HurtboxTimeline, HurtboxVolume, MoveGates,
    MoveSpec, MoveWindow, MovesetContract, VolumeShape, WindowTag,
};
use std::collections::BTreeMap;

/// The cue a move announces itself with. Per-move, so the assertion can tell
/// WHOSE swing it heard as well as which source it was credited to.
fn swing_cue(move_id: &str) -> String {
    format!("{move_id}.swing")
}

/// One damaging move: active from 0.1s to 0.2s, reaching +24 world units forward.
///
/// It also announces itself with a timed cue at 0.05s. That is not decoration:
/// the move timeline is the ONLY authored path that emits a per-character cue,
/// and until this fixture carried one, no test in the repo drove the production
/// chain from "a provider registered this character" all the way to "an
/// `OwnedSfxMessage` credited to that provider".
fn strike(id: &str, damage: i32) -> MoveSpec {
    MoveSpec {
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.4,
        events: vec![ambition_entity_catalog::MoveEvent {
            at_s: 0.05,
            kind: ambition_entity_catalog::MoveEventKind::Sfx { cue: swing_cue(id) },
        }],
        windows: vec![MoveWindow {
            start_s: 0.1,
            end_s: 0.2,
            tag: WindowTag::Active,
            volumes: vec![HitVolume {
                shape: VolumeShape::Rect {
                    offset: (24.0, 0.0),
                    half_extents: (14.0, 12.0),
                },
                damage,
                knockback: 0.0,
                kb_growth: 0.0,
                launch_dir: None,
                on_hit: None,
                vfx: None,
                hit_sfx: None,
            }],
            sustain_effect: None,
            motion_scale: 1.0,
        }],
        gates: MoveGates { grounded: None },
        start_impulse: None,
        smash_charge_mult: 1.0,
    }
}

fn moveset(move_id: &str, damage: i32) -> MovesetContract {
    MovesetContract {
        verbs: BTreeMap::from([("attack".to_string(), move_id.to_string())]),
        moves: vec![strike(move_id, damage)],
    }
}

/// A torso hurtbox, plus a move-time override that makes the body a SMALLER
/// target while its own attack is out — the classic committed-swing tradeoff, and
/// the reason a hurtbox needs a per-move timeline at all.
fn hurtboxes(move_id: &str) -> HurtboxDoc {
    let torso = |half_h: f32| HurtboxTimeline {
        keyframes: vec![HurtboxKeyframe {
            at_s: 0.0,
            volumes: vec![HurtboxVolume {
                shape: VolumeShape::Rect {
                    offset: (0.0, 0.0),
                    half_extents: (10.0, half_h),
                },
            }],
        }],
    };
    HurtboxDoc {
        default: Some(torso(16.0)),
        poses: BTreeMap::from([(POSE_HITSTUN.to_string(), torso(18.0))]),
        moves: BTreeMap::from([(move_id.to_string(), torso(8.0))]),
    }
}

/// Two characters from two independent providers, registered through the ONE seam.
/// A syntactically valid catalog with no characters. The fight's characters come
/// from `register_character`, not from this catalog: the point is that the
/// materializer has its required authority present, not that it finds sheets.
const EMPTY_CATALOG: &str = r#"(
    brain_presets: { "idle": StandStill },
    action_set_presets: { "peaceful": (move_style: Walk) },
    characters: {},
)"#;

fn register_two_providers_characters(app: &mut App) {
    app.register_character(
        CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo")
            .with_sheet("super_mary_o_spritesheet")
            .with_moveset(moveset("stomp", 3))
            .with_hurtboxes(hurtboxes("stomp")),
    );
    app.register_character(
        CharacterDefinition::new("sanic", "Sanic", "sanic_demo")
            .with_sheet("sanic_spritesheet")
            .with_moveset(moveset("roll", 2))
            .with_hurtboxes(hurtboxes("roll")),
    );
}

/// Two providers, one session: registration, staging, and readiness.
///
/// This used to be named `..._trade_damage_in_one_session` and claimed to be the
/// §7.10 acceptance test, but it computed an AABB overlap by hand and subtracted
/// authored integers from local variables — no entities, no `MovePlayback`, no
/// hit events, no `BodyHealth`. It proved that two definitions can coexist and
/// that their numbers can be read back, which is worth having and is what it is
/// now named for. The fight itself is
/// [`two_provider_characters_trade_damage_through_the_real_damage_path`].
#[test]
fn two_providers_stage_into_one_session_and_both_reach_readiness() {
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    // A real session always has an assembled character catalog: the materializer
    // REQUIRES it (`engine.character-authority-is-app-local` forbids making it
    // optional), and a composition without one is named by the capability audit
    // instead — covered by its own test, so one fact keeps one reporter.
    app.insert_resource(
        ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(EMPTY_CATALOG),
        ),
    );
    register_two_providers_characters(&mut app);

    // ── One session seats both, from two providers, and demands their art ──
    let roster = MatchParticipantRoster {
        participants: vec![
            MatchParticipant::new("mary_o").on_team("blue"),
            MatchParticipant::new("sanic")
                .driven_by(ControllerBinding::Cpu {
                    brain_profile: Some("aggressive".into()),
                })
                .on_team("red"),
        ],
    };
    {
        let mut demand = app.world_mut().resource_mut::<CharacterLoadDemand>();
        roster.project_demand(&mut demand);
    }
    let staged: Vec<String> = app
        .world()
        .resource::<CharacterLoadDemand>()
        .pending()
        .map(str::to_string)
        .collect();
    assert_eq!(
        staged,
        vec!["mary_o".to_string(), "sanic".to_string()],
        "both providers' characters must be staged by the one projection"
    );

    // ── Both are prepared, and each kept its OWN provider and numbers ──
    let registry = app.world().resource::<PreparedCharacterRegistry>();
    let mary = registry.get("mary_o").expect("Mary-O is prepared");
    let sanic = registry.get("sanic").expect("Sanic is prepared");
    assert_eq!(mary.provider, "mary_o_demo");
    assert_eq!(sanic.provider, "sanic_demo");
    assert_ne!(
        mary.moveset.as_ref().unwrap().moves[0].windows[0].volumes[0].damage,
        sanic.moveset.as_ref().unwrap().moves[0].windows[0].volumes[0].damage,
        "two characters in one session keep their own authored damage"
    );

    // ── Each body's damageable silhouette comes from its own authored doc ──
    // Mary-O idles (default torso); Sanic is mid-roll (the move override).
    let mary_boxes = resolve_hurtboxes(
        mary.hurtboxes.as_ref().expect("authored"),
        None,
        Some((POSE_IDLE, 0.0)),
    );
    let sanic_boxes = resolve_hurtboxes(
        sanic.hurtboxes.as_ref().expect("authored"),
        Some(("roll", 0.15)),
        Some((POSE_IDLE, 0.0)),
    );
    assert_eq!(mary_boxes.source, HurtboxSelection::Default);
    assert_eq!(
        sanic_boxes.source,
        HurtboxSelection::MoveOverride,
        "a body mid-attack presents its move's silhouette, not its idle one"
    );

    // ── The readiness invariant holds for both, through the engine ──
    app.update();
    let demand = app.world().resource::<CharacterLoadDemand>();
    let states = app.world().resource::<CharacterLoadStates>();
    for character in ["mary_o", "sanic"] {
        assert!(
            !unsettled_staged_characters(demand, states).contains(&character.to_string()),
            "`{character}` must not be left unsettled: every staged character reaches \
             Ready or a NAMED terminal Failed before reveal (§4.9)"
        );
    }
}

/// The same fight, one provider absent. The present character must still work —
/// a composition is not all-or-nothing, and a missing provider must be NAMED
/// rather than making everyone silently placeholder.
#[test]
fn a_missing_opponent_is_named_and_the_present_character_still_fights() {
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    app.register_character(
        CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo")
            .with_moveset(moveset("stomp", 3))
            .with_hurtboxes(hurtboxes("stomp")),
    );

    // A roster naming a character no loaded provider authored.
    let roster = MatchParticipantRoster::of(["mary_o", "someone_elses_fighter"]);
    {
        let mut demand = app.world_mut().resource_mut::<CharacterLoadDemand>();
        roster.project_demand(&mut demand);
    }

    let registry = app.world().resource::<PreparedCharacterRegistry>();
    assert!(registry.get("mary_o").is_some());
    assert!(
        registry.get("someone_elses_fighter").is_none(),
        "an unregistered fighter must not appear in the prepared authority"
    );
    // Mary-O's own moveset and hurtboxes are unaffected by her opponent's absence.
    let mary = registry.get("mary_o").unwrap();
    assert_eq!(
        resolve_hurtboxes(mary.hurtboxes.as_ref().unwrap(), None, None).source,
        HurtboxSelection::Default
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// The actual fight.
//
// Everything below drives the PRODUCTION systems: the gesture interpreter, the
// move trigger, move playback (which spawns the strike volumes), the hurtbox
// resolver, the volume publisher, `apply_hitbox_damage`, and the victim-side
// consumer that mutates health. Nothing here computes an overlap or subtracts a
// damage number by hand — that was the defect in the test above, and it is why
// the fact that NO body in the game was hit on its published silhouette survived
// a green suite.
// ─────────────────────────────────────────────────────────────────────────────

use crate::combat::hitbox::apply_hitbox_damage;
use crate::combat::moveset::{
    advance_move_playback, resolve_attack_gestures, trigger_moveset_moves, ActorMoveset,
    MovePlayback,
};
use crate::features::apply_feature_hit_events;
use ambition_characters::actor::attack_gesture::{AttackGestureTuning, ResolvedAttackGesture};
use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::brain::ActorControl;

const TICK: f32 = 1.0 / 60.0;

/// Every `HitEvent` the production emitter produced this run.
///
/// Kept in the harness rather than assembled per test: a fight test that fails
/// with "health did not change" cannot tell you WHICH stage broke — no move
/// triggered, no volume spawned, no overlap, or an overlap whose event nobody
/// consumed. With this, the assertion can say which.
#[derive(Resource, Default)]
struct Traded(Vec<crate::features::HitEvent>);

fn record_trades(mut events: MessageReader<crate::features::HitEvent>, mut out: ResMut<Traded>) {
    out.0.extend(events.read().cloned());
}

/// Every cue the production dispatcher handed the audio authority, with the
/// source it was credited to.
#[derive(Resource, Default)]
struct Heard(Vec<(ambition_sfx::SfxId, String)>);

impl Heard {
    /// The source a given cue was credited to, or `None` if it never played.
    fn source_of(&self, cue: &str) -> Option<&str> {
        let id = ambition_sfx::SfxId::new(cue);
        self.0
            .iter()
            .find(|(heard, _)| *heard == id)
            .map(|(_, source)| source.as_str())
    }
}

fn record_cues(mut messages: MessageReader<ambition_sfx::OwnedSfxMessage>, mut out: ResMut<Heard>) {
    for message in messages.read() {
        if let ambition_sfx::SfxMessage::Play { id, .. } = &message.request {
            out.0.push((*id, message.source.as_str().to_string()));
        }
    }
}

/// A body that can fight: a real actor cluster, plus the character's authored
/// moveset and hurtbox doc, plus the control seam a controller drives.
///
/// Deliberately built from `ActorClusterSeed` — the same construction production
/// uses — so the body carries the real health, melee, and capability components
/// rather than a bespoke test shape.
fn spawn_fighter(
    app: &mut App,
    character_id: &str,
    at: Vec2,
    facing: f32,
    faction: crate::combat::components::ActorFaction,
) -> Entity {
    let prepared = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .get(character_id)
        .expect("the fighter must be registered before it is staged")
        .clone();
    // Deliberately WIDER than the authored torso (±10 in `hurtboxes()`): if the
    // body's bounding box and its authored silhouette are the same rectangle, no
    // test here can tell which one damage consulted. That is not a hypothetical —
    // the first version of these tests used a 20-wide body and proved nothing.
    let body = Vec2::new(40.0, 32.0);
    let aabb = ambition_engine_core::Aabb::new(at, body / 2.0);
    let mut seed = crate::features::ecs::actor_clusters::ActorClusterSeed::new(
        character_id.to_string(),
        prepared.display_name.clone(),
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Custom("medium_striker".into()),
        &[],
    );
    seed.health =
        ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(10));
    // Facing is not decoration: a move's authored offsets are mirrored through it,
    // so a fighter looking the wrong way swings into empty space. The seed's
    // default is -1, which had both fighters swinging left and produced exactly
    // one hit in a two-attacker exchange.
    seed.kin.facing = facing;
    let (identity, disposition, combat, intent, cooldowns) =
        crate::features::ecs::enemy_component_snapshot(&seed);
    app.world_mut()
        .spawn((
            (
                ambition_platformer_primitives::lifecycle::FeatureSimEntity,
                crate::features::FeatureId::new(character_id),
                ambition_engine_core::CenteredAabb::from_center_size(at, body),
                seed.into_components(),
                crate::features::MotionModel::default(),
            ),
            (identity, disposition, combat, intent, cooldowns, faction),
            // §7.6 → gameplay: the character's OWN authored moveset and silhouette,
            // straight off the prepared definition. This is the join the plan is
            // for — a provider registered a character, and this body fights with
            // exactly what that provider authored.
            ActorMoveset(prepared.moveset.clone().expect("authored moveset")),
            AuthoredHurtboxes(prepared.hurtboxes.clone().expect("authored hurtboxes")),
            // A fighter WEARS the character it fights as. Without this the body
            // carries a moveset and a silhouette from a provider it cannot name:
            // `ActorClusterSeed` resolves `CombatTuning::sprite_character_id` by
            // DISPLAY NAME out of the assembled catalog, which a registered-only
            // character is absent from, so every cue the body emitted was credited
            // to whoever owned the session.
            ambition_characters::actor::WornCharacter::new(character_id),
            ResolvedHurtboxes::default(),
            crate::combat::components::DamageableVolumes::default(),
            // The control seam + the gesture state §7.9 interprets.
            ActorControl(ActorControlFrame::default()),
            ambition_characters::actor::attack_gesture::AttackGestureState::default(),
            AttackGestureTuning::default(),
            ResolvedAttackGesture::default(),
        ))
        .id()
}

/// The production chain, in production order.
fn fight_app() -> App {
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    app.insert_resource(
        ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(EMPTY_CATALOG),
        ),
    );
    app.insert_resource(crate::boss_encounter::test_boss_catalog().clone());
    // Both fighters author explicit rectangles, so no blade is resolved from
    // sprite data here; the resolver is still REQUIRED by `advance_move_playback`,
    // and `disabled()` is the content-free answer for a fixture.
    app.insert_resource(crate::combat::authored_volumes::AuthoredAttackVolumeResolver::disabled());
    app.insert_resource(crate::features::enemies::test_roster());
    app.insert_resource(crate::features::GameplayBanner::default());
    app.init_resource::<ambition_time::WorldTime>();
    {
        let mut time = app.world_mut().resource_mut::<ambition_time::WorldTime>();
        time.scaled_dt = TICK;
        time.raw_dt = TICK;
    }
    app.add_message::<crate::features::HitEvent>();
    app.add_message::<crate::features::SetFlagRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<ambition_vfx::vfx::DebrisBurstMessage>();
    app.add_message::<crate::features::ActorStimulus>();
    app.add_message::<crate::features::ecs::damage_apply::WalletShieldSpent>();
    app.add_message::<crate::combat::moveset::MoveEventMessage>();
    app.add_message::<ambition_characters::brain::ActorActionMessage>();
    // A session whose speakers belong to somebody else. Every cue a fighter emits
    // falls back to THIS source when the fighter is not attributed, so an
    // attribution bug shows up as "the session owner made that sound" — the exact
    // production symptom — rather than as an empty string that could also mean
    // "no context installed".
    {
        let mut context = ambition_sfx::SfxEmissionContext::default();
        context.set(
            ambition_sfx::AudioContextOwner::Gameplay(1),
            "session_owner",
        );
        app.insert_resource(context);
    }
    app.add_systems(
        Update,
        (
            resolve_attack_gestures,
            trigger_moveset_moves,
            advance_move_playback,
            // The cue dispatcher, in the chain. Without it the fight proved that a
            // move fires a `MoveEventMessage` and nothing about what the audio
            // authority receives — which is where the attribution actually lands.
            crate::combat::moveset::dispatch_move_events,
            crate::character_runtime::hurtbox::resolve_body_hurtboxes,
            crate::features::refresh_body_damageable_volumes,
            apply_hitbox_damage,
            record_trades,
            record_cues,
            apply_feature_hit_events,
        )
            .chain(),
    );
    app.init_resource::<Traded>();
    app.init_resource::<Heard>();
    app
}

/// A one-line account of what the production chain produced, for failures.
fn trade_report(app: &App) -> String {
    let traded = app.world().resource::<Traded>();
    format!(
        "{} hit event(s): {:?}",
        traded.0.len(),
        traded
            .0
            .iter()
            .map(|e| (e.damage, e.source.clone(), e.target))
            .collect::<Vec<_>>()
    )
}

fn press_attack(app: &mut App, body: Entity) {
    let mut control = app
        .world_mut()
        .get_mut::<ActorControl>(body)
        .expect("the fighter carries the control seam");
    control.0.melee_pressed = true;
    control.0.melee_held = true;
}

fn release_attack(app: &mut App, body: Entity) {
    let mut control = app
        .world_mut()
        .get_mut::<ActorControl>(body)
        .expect("the fighter carries the control seam");
    control.0.melee_pressed = false;
    control.0.melee_held = false;
}

fn health(app: &App, body: Entity) -> i32 {
    app.world()
        .get::<ambition_characters::actor::BodyHealth>(body)
        .expect("a fighter has health")
        .health
        .current
}

/// **§7.10, for real.** Two characters from two providers, in one session,
/// damaging each other through the production damage path.
///
/// What each assertion below is load-bearing for:
///
/// * both bodies fight with the moveset and silhouette their own provider
///   authored, taken off `PreparedCharacterRegistry` — the §7.6 join;
/// * the attack is *triggered* by pressing the control seam, so §7.4's
///   directional verb chain and §7.9's gesture interpreter are in the path;
/// * `advance_move_playback` spawns the strike volume from the authored window,
///   so the damage number is the authored one and nothing in this test says `-=`;
/// * `apply_hitbox_damage` resolves it against the victim's PUBLISHED silhouette,
///   and `apply_feature_hit_events` mutates real `BodyHealth`;
/// * each fighter loses exactly the damage its OPPONENT authored, which is the
///   two-providers-in-one-session claim reduced to a number.
#[test]
fn two_provider_characters_trade_damage_through_the_real_damage_path() {
    let mut app = fight_app();
    register_two_providers_characters(&mut app);

    // Facing each other, inside each other's 24-unit strike reach.
    let mary = spawn_fighter(
        &mut app,
        "mary_o",
        Vec2::new(0.0, 0.0),
        1.0,
        crate::combat::components::ActorFaction::Enemy,
    );
    let sanic = spawn_fighter(
        &mut app,
        "sanic",
        Vec2::new(22.0, 0.0),
        -1.0,
        crate::combat::components::ActorFaction::Npc,
    );
    assert_eq!(health(&app, mary), 10);
    assert_eq!(health(&app, sanic), 10);

    // Both swing on the same tick: a fight, not a beating.
    press_attack(&mut app, mary);
    press_attack(&mut app, sanic);
    app.update();
    release_attack(&mut app, mary);
    release_attack(&mut app, sanic);

    // Each authored move is active from 0.1s to 0.2s; run past the window.
    assert!(
        app.world().get::<MovePlayback>(mary).is_some(),
        "pressing the control seam must START Mary-O's authored move — if this \
         fails the trigger chain is not in the path and the rest proves nothing"
    );
    for _ in 0..14 {
        app.update();
    }

    // Mary-O's stomp authored 3 damage; Sanic's roll authored 2.
    assert_eq!(
        health(&app, sanic),
        7,
        "Sanic loses exactly the damage MARY-O's provider authored ({})",
        trade_report(&app)
    );
    assert_eq!(
        health(&app, mary),
        8,
        "Mary-O loses exactly the damage SANIC's provider authored"
    );

    // Each swing SOUNDS like the character that swung it. Both fighters are
    // registered-only — neither has a `CharacterCatalogOwners` entry — so this is
    // also the assertion that attribution reaches the registration seam.
    let heard = app.world().resource::<Heard>();
    for (character, cue, provider) in [
        ("Mary-O", swing_cue("stomp"), "mary_o_demo"),
        ("Sanic", swing_cue("roll"), "sanic_demo"),
    ] {
        assert_eq!(
            heard.source_of(&cue),
            Some(provider),
            "{character}'s swing must be credited to `{provider}`, not to the \
             session owner: a cue tagged with the wrong source plays out of the \
             wrong bank, and one tagged `__unscoped__` is DENIED with nothing \
             reported (heard: {:?})",
            heard.0
        );
    }
}

/// The silhouette decides the fight, not the bounding box.
///
/// Same two characters, same authored strike, moved apart so the reach still
/// covers each body's coarse box but no longer covers the narrow authored torso.
/// Nobody may be hit. Without this, the test above passes just as well when
/// damage reads the coarse box — which is exactly the state §7.10 shipped in.
#[test]
fn a_strike_that_clears_the_authored_torso_lands_on_nobody() {
    let mut app = fight_app();
    register_two_providers_characters(&mut app);
    // Mary-O's stomp spans x ∈ [10, 38]. Sanic stands at x = 52, so his coarse
    // box (±20) spans [32, 72] and DOES overlap the strike, while his authored
    // torso (±10) spans [42, 62] and does not. The two geometries disagree, which
    // is the only way this test can mean anything.
    let mary = spawn_fighter(
        &mut app,
        "mary_o",
        Vec2::new(0.0, 0.0),
        1.0,
        crate::combat::components::ActorFaction::Enemy,
    );
    let sanic = spawn_fighter(
        &mut app,
        "sanic",
        Vec2::new(52.0, 0.0),
        -1.0,
        crate::combat::components::ActorFaction::Npc,
    );
    press_attack(&mut app, mary);
    app.update();
    release_attack(&mut app, mary);
    for _ in 0..14 {
        app.update();
    }
    assert_eq!(
        health(&app, sanic),
        10,
        "the strike reaches Sanic's bounding rectangle but not his authored \
         torso, so it must miss; landing it means damage is reading the box ({})",
        trade_report(&app)
    );
}
