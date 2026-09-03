//! §7.10: two characters, two providers, one session, real damage.
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

use ambition_characters::prepared::PreparedCharacterRegistry;
// Stepping a fixture is `finalize_and_update`, not `update`. Bevy's RUNNERS
// close the plugin-composition barrier; `App::update` does not, and character
// preparation publishes its registry there — so a fixture that only updated
// would register a cast and never publish one. Idempotent, so a helper called
// per step costs a set lookup after the first.
use ambition_platformer2d_shared_tangle::app_finalization::{finalize, finalize_and_update};

use super::*;
use ambition_characters::actor::definition::CharacterDefinition;
use ambition_entity_catalog::{
    ClipBinding, HitVolume, HurtboxDoc, HurtboxKeyframe, HurtboxTimeline, HurtboxVolume, MoveGates,
    MoveSpec, MoveWindow, MovesetContract, VolumeShape, WindowTag,
};
use ambition_match::{
    ControllerBinding, MatchParticipant, MatchParticipantRoster, StagesCharacters,
};
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_core::Vec2;
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
                // An ordinary hit, not a gust.
                shape: VolumeShape::Rect {
                    offset: (24.0, 0.0),
                    half_extents: (14.0, 12.0),
                },
                damage,
                knockback: 0.0,
                knockback_growth: None,
                launch_dir: None,
                on_hit: None,
                vfx: None,
                hit_sfx: None,
                reaction: None,
            }],
            sustain_effect: None,
            motion_scale: 1.0,
        }],
        gates: MoveGates::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
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
        ..Default::default()
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
    finalize(&mut app);
    let registry = app.world().resource::<PreparedCharacterRegistry>();
    let mary = registry.get("mary_o").expect("Mary-O is prepared");
    let sanic = registry.get("sanic").expect("Sanic is prepared");
    assert_eq!(mary.provider, "mary_o_demo");
    assert_eq!(sanic.provider, "sanic_demo");
    assert_ne!(
        mary.kit.projectable_moveset().unwrap().moves[0].windows[0].volumes[0].damage,
        sanic.kit.projectable_moveset().unwrap().moves[0].windows[0].volumes[0].damage,
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
    finalize_and_update(&mut app);
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

    finalize(&mut app);
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

use crate::features::apply_feature_hit_events;
use ambition_characters::actor::attack_gesture::{AttackGestureTuning, ResolvedAttackGesture};
use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::control::ActorControl;
use ambition_combat::hitbox::apply_hitbox_damage;
use ambition_combat::moveset::{
    advance_move_playback, resolve_attack_gestures, trigger_moveset_moves, MovePlayback,
};

const TICK: f32 = 1.0 / 60.0;

/// Every `HitEvent` the production emitter produced this run.
///
/// Kept in the harness rather than assembled per test: a fight test that fails
/// with "health did not change" cannot tell you WHICH stage broke — no move
/// triggered, no volume spawned, no overlap, or an overlap whose event nobody
/// consumed. With this, the assertion can say which.
#[derive(Resource, Default)]
struct Traded(Vec<ambition_combat::events::HitEvent>);

fn record_trades(
    mut events: MessageReader<ambition_combat::events::HitEvent>,
    mut out: ResMut<Traded>,
) {
    out.0.extend(events.read().cloned());
}

/// Every cue the production dispatcher handed the audio authority, with the
/// source it was credited to.
#[derive(Resource, Default)]
struct Heard {
    /// Authored cues, by the id the move named.
    played: Vec<(ambition_sfx::SfxId, String)>,
    /// Sources credited with a `Death`. Its own list because `SfxMessage::Death`
    /// carries no id to look up — it is the semantic cue every provider voices
    /// differently, and G1 found it taking the SESSION's source on every body in
    /// the game.
    deaths: Vec<String>,
}

impl Heard {
    /// The source a given cue was credited to, or `None` if it never played.
    fn source_of(&self, cue: &str) -> Option<&str> {
        let id = ambition_sfx::SfxId::new(cue);
        self.played
            .iter()
            .find(|(heard, _)| *heard == id)
            .map(|(_, source)| source.as_str())
    }
}

/// The body-generic reaction decay the actor tick performs
/// (`features::ecs::actors::update` calls `decay_reaction_timers` on exactly this
/// component, once per tick).
///
/// Present because without it the post-hit i-frame window never expires, so this
/// fixture could land a FIRST hit on a body and never a second — three of four
/// strikes in the death test were silently swallowed. Calling the same one function
/// the actor tick calls is cheaper than composing the whole actor update, and is the
/// same fact.
fn decay_reaction_timers(mut bodies: Query<&mut ambition_characters::actor::BodyCombat>) {
    for mut combat in &mut bodies {
        combat.decay_reaction_timers(TICK);
    }
}

fn record_cues(mut messages: MessageReader<ambition_sfx::OwnedSfxMessage>, mut out: ResMut<Heard>) {
    for message in messages.read() {
        match &message.request {
            ambition_sfx::SfxMessage::Play { id, .. } => {
                out.played.push((*id, message.source.as_str().to_string()))
            }
            ambition_sfx::SfxMessage::Death { .. } => {
                out.deaths.push(message.source.as_str().to_string())
            }
            _ => {}
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
    faction: ambition_combat::components::ActorFaction,
) -> Entity {
    finalize(app);
    let prepared = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .get(character_id)
        .expect("the fighter must be registered before it is staged")
        .clone();
    // Keep the body wider than the authored torso so the test can distinguish
    // body bounds from the authored damage silhouette.
    let body = Vec2::new(40.0, 32.0);
    // ...and that is ASSERTED, not commented. Read the authored width off the definition and
    // check it here, so widening the torso breaks the fixture that depends on it being narrow
    // rather than quietly making every silhouette-vs-box assertion in this file vacuous.
    {
        let widest_authored = prepared
            .hurtboxes
            .as_ref()
            .and_then(|doc| doc.default.as_ref())
            .map(|timeline| {
                timeline
                    .keyframes
                    .iter()
                    .flat_map(|keyframe| keyframe.volumes.iter())
                    .map(|volume| match volume.shape {
                        VolumeShape::Rect { half_extents, .. } => half_extents.0,
                        _ => 0.0,
                    })
                    .fold(0.0_f32, f32::max)
            })
            .unwrap_or(0.0);
        assert!(
            widest_authored > 0.0 && widest_authored * 2.0 < body.x,
            "`{character_id}`'s authored torso is {widest_authored} half-wide against \
             a {} half-wide body box: the two geometries must DIFFER or every \
             hit-on-the-silhouette assertion in this file is also satisfied by \
             hitting the box",
            body.x / 2.0
        );
    }
    let aabb = ambition_platformer2d_core::Aabb::new(at, body / 2.0);
    let mut seed = ambition_body_seed::ActorClusterSeed::new(
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
    let (identity, disposition, combat) = crate::features::ecs::enemy_component_snapshot(&seed);
    app.world_mut()
        .spawn((
            (
                ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
                ambition_combat::components::FeatureId::new(character_id),
                ambition_platformer2d_core::CenteredAabb::from_center_size(at, body),
                seed.into_components(),
                ambition_platformer2d_core::movement::MotionModel::default(),
            ),
            (identity, disposition, combat, faction),
            // §7.6 → gameplay: the character's own authored moveset and silhouette are NOT inserted
            // here. `project_prepared_character_definitions` puts them on the body from the
            // registry, which is C3 — the join the plan is for. Deleting it is what makes this test
            // evidence for the engine seam.
            //
            // A fighter WEARS the character it fights as. Without this the body
            // carries a moveset and a silhouette from a provider it cannot name:
            // `ActorClusterSeed` resolves `CombatTuning::sprite_character_id` by
            // DISPLAY NAME out of the assembled catalog, which a registered-only
            // character is absent from, so every cue the body emitted was credited
            // to whoever owned the session.
            ambition_characters::actor::WornCharacter::new(character_id),
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
    app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
    // Both fighters author explicit rectangles, so no blade is resolved from
    // sprite data here; the resolver is still REQUIRED by `advance_move_playback`,
    // and `disabled()` is the content-free answer for a fixture.
    app.insert_resource(
        ambition_combat::authored_volumes::AuthoredAttackVolumeResolver::disabled(),
    );
    app.insert_resource(ambition_combat::events::GameplayBanner::default());
    app.init_resource::<ambition_time::WorldTime>();
    {
        let mut time = app.world_mut().resource_mut::<ambition_time::WorldTime>();
        time.scaled_dt = TICK;
        time.raw_dt = TICK;
    }
    app.add_message::<ambition_combat::events::HitEvent>();
    app.add_message::<ambition_combat::hitbox::LandedBodyHit>();
    app.add_message::<ambition_combat::events::SetFlagRequested>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    // `dispatch_move_events` asks for PAIRED effects now — a visual and the cue its own name
    // addresses — so the request channel has to exist or the system fails parameter validation
    // before it can run.
    app.add_message::<ambition_vfx::FxRequest>();
    app.add_message::<ambition_vfx::vfx::DebrisBurstMessage>();
    app.add_message::<ambition_combat::events::ActorStimulus>();
    app.add_message::<ambition_combat::stocks::BodyKnockedOut>();
    app.add_message::<ambition_damage::WalletShieldSpent>();
    app.add_message::<ambition_combat::moveset::MoveEventMessage>();
    app.add_message::<ambition_characters::brain::ActorActionMessage>();
    // A session whose speakers belong to somebody else.
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
            ambition_combat::moveset::dispatch_move_events,
            crate::character_runtime::hurtbox::resolve_body_hurtboxes,
            crate::features::refresh_body_damageable_volumes,
            apply_hitbox_damage,
            record_trades,
            record_cues,
            apply_feature_hit_events,
            decay_reaction_timers,
        )
            .chain()
            .after(crate::character_runtime::project_prepared_character_definitions),
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

/// §7.10, for real. Two characters from two providers, in one session,
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
        ambition_combat::components::ActorFaction::Enemy,
    );
    let sanic = spawn_fighter(
        &mut app,
        "sanic",
        Vec2::new(22.0, 0.0),
        -1.0,
        ambition_combat::components::ActorFaction::Npc,
    );
    assert_eq!(health(&app, mary), 10);
    assert_eq!(health(&app, sanic), 10);

    // Both swing on the same tick: a fight, not a beating.
    press_attack(&mut app, mary);
    press_attack(&mut app, sanic);
    finalize_and_update(&mut app);
    release_attack(&mut app, mary);
    release_attack(&mut app, sanic);

    // Each authored move is active from 0.1s to 0.2s; run past the window.
    assert!(
        app.world().get::<MovePlayback>(mary).is_some(),
        "pressing the control seam must START Mary-O's authored move — if this \
         fails the trigger chain is not in the path and the rest proves nothing"
    );
    for _ in 0..14 {
        finalize_and_update(&mut app);
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
            heard.played
        );
    }
}

/// The silhouette decides the fight, not the bounding box.
///
/// Nobody may be hit. Without this, the test above passes just as well when damage reads the
/// coarse box — which is exactly the state §7.10 shipped in.
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
        ambition_combat::components::ActorFaction::Enemy,
    );
    let sanic = spawn_fighter(
        &mut app,
        "sanic",
        Vec2::new(52.0, 0.0),
        -1.0,
        ambition_combat::components::ActorFaction::Npc,
    );
    press_attack(&mut app, mary);
    finalize_and_update(&mut app);
    release_attack(&mut app, mary);

    // ── The premise, CHECKED, on the tick the strike is live ──────────────────
    //
    // So walk to the live strike volume and assert what the comment claims — the coarse box
    // overlaps it, the published silhouette does not.
    let mut checked = false;
    for _ in 0..14 {
        finalize_and_update(&mut app);
        // The strike's world box, resolved the SAME way `apply_hitbox_damage`
        // resolves it: a `FollowOwner` hitbox carries a local offset and tracks the
        // owner's box centre, so there is no world rectangle on the entity to read.
        let strike = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<&ambition_combat::hitbox::Hitbox, With<
                ambition_combat::moveset::StrikeVolume,
            >>();
            let hitbox = q.iter(world).next().cloned();
            hitbox.map(|hitbox| {
                let owner_pos = world
                    .get::<ambition_platformer2d_core::CenteredAabb>(hitbox.owner)
                    .map(|aabb| aabb.center)
                    .expect("the attacking fighter carries its coarse box");
                hitbox.world_aabb(owner_pos)
            })
        };
        let Some(strike) = strike else { continue };
        let coarse = app
            .world()
            .get::<ambition_platformer2d_core::CenteredAabb>(sanic)
            .expect("a spawned fighter carries its coarse box")
            .aabb();
        let published = app
            .world()
            .get::<ambition_combat::components::DamageableVolumes>(sanic)
            .expect("the volume publisher ran");
        assert!(
            published.published(),
            "nothing published Sanic's silhouette, so `overlaps the box but not the \
             silhouette` is not a distinction this world can make"
        );
        assert!(
            strike.strict_intersects(coarse),
            "the strike must overlap Sanic's COARSE box ({coarse:?}) or this test \
             proves nothing: a strike that reaches neither geometry misses for the \
             uninteresting reason (strike {strike:?})"
        );
        assert!(
            !published
                .volumes
                .iter()
                .any(|volume| strike.strict_intersects(volume.bounds())),
            "the strike must NOT overlap Sanic's published silhouette \
             ({:?}) — otherwise a hit would be correct and the assertion below is \
             testing the opposite of what it says (strike {strike:?})",
            published.volumes
        );
        checked = true;
    }
    assert!(
        checked,
        "no strike volume was ever live during the window, so the geometry premise \
         was never checked and the health assertion below is satisfied by the move \
         never happening"
    );

    assert_eq!(
        health(&app, sanic),
        10,
        "the strike reaches Sanic's bounding rectangle but not his authored \
         torso, so it must miss; landing it means damage is reading the box ({})",
        trade_report(&app)
    );
}

/// G1: a body dies in its OWN voice.
///
/// The move timeline was the one emitter that read `BodyPresentationSource`, so
/// every other body-owned sound — the block clang, the armor loss, the pogo, the
/// ability cast, the projectile impact, and this, the death — was attributed to
/// whoever owned the session. A crossover fight therefore had two characters who
/// swung in their own voices and died in the host's.
///
/// This drives the same production chain as the trade test, and keeps swinging
/// until Mary-O's authored stomp actually kills Sanic, so the assertion is about the
/// real `apply_actor_hit` death branch rather than a synthesized message.
#[test]
fn a_dying_body_dies_in_its_own_voice() {
    let mut app = fight_app();
    register_two_providers_characters(&mut app);

    let mary = spawn_fighter(
        &mut app,
        "mary_o",
        Vec2::new(0.0, 0.0),
        1.0,
        ambition_combat::components::ActorFaction::Enemy,
    );
    let sanic = spawn_fighter(
        &mut app,
        "sanic",
        Vec2::new(22.0, 0.0),
        -1.0,
        ambition_combat::components::ActorFaction::Npc,
    );

    // Mary-O's stomp authors 3 damage against 10 HP, and a struck body holds
    // i-frames for a moment, so this is a sequence of separate swings — not one
    // sustained overlap that would be gated down to a single hit.
    for _ in 0..8 {
        if health(&app, sanic) <= 0 {
            break;
        }
        press_attack(&mut app, mary);
        finalize_and_update(&mut app);
        release_attack(&mut app, mary);
        for _ in 0..20 {
            finalize_and_update(&mut app);
        }
    }

    // The premise: he actually died on the production path. Without this the
    // source assertion below passes vacuously whenever nothing died at all.
    assert!(
        health(&app, sanic) <= 0,
        "Sanic never died, so this test says nothing about the death cue's \
         attribution (mary={} sanic={} {})",
        health(&app, mary),
        health(&app, sanic),
        trade_report(&app)
    );

    let heard = app.world().resource::<Heard>();
    assert_eq!(
        heard.deaths.as_slice(),
        ["sanic_demo"],
        "the death must be credited to SANIC's provider — the body that died — \
         not to `session_owner`, which is what every death in the game was \
         attributed to before G1 (cues heard: {:?})",
        heard.played
    );
}
