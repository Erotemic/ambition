//! A4: the cast's sources are authorized by PRODUCTION code, not by a test.

use ambition_characters::prepared::PreparedCharacterRegistry;
// Stepping a fixture is `finalize_and_update`, not `update`. Bevy's RUNNERS
// close the plugin-composition barrier; `App::update` does not, and character
// preparation publishes its registry there — so a fixture that only updated
// would register a cast and never publish one. Idempotent, so a helper called
// per step costs a set lookup after the first.
use ambition_platformer2d_shared_tangle::app_finalization::finalize_and_update;

use super::*;
use crate::character_runtime::{CharacterDefinitionAppExt, CharacterRuntimePlugin};
use ambition_characters::actor::definition::CharacterDefinition;
use ambition_sfx::PresentationSourceId;

/// One gameplay session owning the speakers, with `ambition_platformer2d` as the primary.
fn session_app() -> App {
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    // The routing markers (`MovesetMelee` / `MovesetRanged`) are DERIVED from the live
    // `ActorMoveset` by a system this plugin owns.
    app.add_plugins(crate::action_scheme::ActionSchemePlugin);
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    app.init_resource::<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>();
    begin_session(&mut app, 1);
    app
}

/// Step the fixture until derived state has settled.
///
/// Two updates, not one. The shipped composition orders the projection, the
/// persona derive, the equipment overlay and the marker reconcile inside one tick
/// of `PlayerInputSet`; this fixture installs the plugins WITHOUT
/// `configure_platformer2d_simulation_phases`, so those sets carry no ordering and the reconcile
/// can run before the projection's commands have applied. Settling is the honest
/// fixture-shaped answer — asserting after one update would be asserting on
/// Bevy's arbitrary intra-set order.
fn settle(app: &mut App) {
    finalize_and_update(app);
    finalize_and_update(app);
}

/// Start a fresh gameplay session: a new scope, and a new audio authority whose
/// authorized-source map starts over with only the session owner in it.
fn begin_session(app: &mut App, owner: u64) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>()
        .begin();
    let mut selection = ambition_audio::selection::ActiveAudioSelection::default();
    selection.select_gameplay(owner, "ambition", None, None, Default::default());
    app.insert_resource(selection);
}

fn is_authorized(app: &App, provider: &str) -> bool {
    app.world()
        .resource::<ambition_audio::selection::ActiveAudioSelection>()
        .is_sfx_source_authorized(&PresentationSourceId::new(provider))
}

/// Stage a character the way a session does: submit demand and let the engine
/// materializer settle it. In an app with no asset pipeline that settles as
/// `NoAssetPipeline` — a named terminal state — which still puts the character in
/// the staged cast, because a fighter whose sheet did not resolve is still in the
/// fight and still needs its cues authorized.
fn stage(app: &mut App, character_id: &str) {
    app.world_mut()
        .resource_mut::<ambition_characters::load_demand::CharacterLoadDemand>()
        .request(character_id);
}

/// The gap this closes. A secondary provider's cue was DENIED in production.
///
/// `write_from` tags a request with the emitting character's provider, and
/// `authorize_sfx_source` is what makes that tag resolvable.
#[test]
fn seating_a_cast_authorizes_every_participants_presentation_source() {
    let mut app = session_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));

    // Before staging: the session owner is authorized and nobody else is.
    let selection = app
        .world()
        .resource::<ambition_audio::selection::ActiveAudioSelection>();
    assert!(
        selection
            .sfx_for_source(&PresentationSourceId::new("sanic_demo"))
            .is_none(),
        "a provider that is not in the cast must not be authorized just because it \
         registered characters — authorization is a property of THIS session's cast"
    );

    stage(&mut app, "mary_o");
    stage(&mut app, "sanic");
    finalize_and_update(&mut app);

    let selection = app
        .world()
        .resource::<ambition_audio::selection::ActiveAudioSelection>();
    for provider in ["mary_o_demo", "sanic_demo"] {
        assert!(
            selection.is_sfx_source_authorized(&PresentationSourceId::new(provider)),
            "`{provider}` is in the staged cast, so its presentation source must be \
             authorized; otherwise every cue it tags correctly is silently denied"
        );
    }
}

/// The provider comes from whichever declaration exists — including a character
/// that lives ONLY on the registration seam.
///
/// The catalog owners map is built by catalog-fragment assembly, so a
/// registered-only character has no entry there. If authorization consulted only
/// that map, the newest and most deliberately-declared characters would be exactly
/// the ones whose cues got dropped.
#[test]
fn a_registered_only_character_still_names_its_provider() {
    let mut app = session_app();
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    stage(&mut app, "sanic");
    finalize_and_update(&mut app);

    assert!(app
        .world()
        .resource::<ambition_audio::selection::ActiveAudioSelection>()
        .is_sfx_source_authorized(&PresentationSourceId::new("sanic_demo")),);
}

/// A room stages a display name, and the right provider is authorized.
///
/// Rooms author characters by the name a designer typed — `demand_room_character_sheets` pushes
/// `enemy.name` and an interactable's `character_id` straight through — while every provider
/// map is keyed by stable id.
#[test]
fn staging_a_character_by_display_name_authorizes_its_provider() {
    let mut app = session_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    stage(&mut app, "Mary-O");
    finalize_and_update(&mut app);

    assert!(
        is_authorized(&app, "mary_o_demo"),
        "a demand spelled with the DISPLAY name must still authorize the provider \
         of `mary_o`: rooms author display names, and a cast keyed by demand \
         spelling matches nothing in either provider map"
    );
    let states = app
        .world()
        .resource::<crate::character_runtime::CharacterLoadStates>();
    assert!(
        states.cast().contains("mary_o"),
        "the cast holds canonical ids"
    );
    assert!(
        states.staged_tokens().any(|token| token == "Mary-O"),
        "and the ledger still reports the spelling that was demanded, which is the \
         whole reason the two are separate"
    );
}

/// A later session does not authorize the previous session's cast.
///
/// The load ledger is append-only across rooms AND across sessions, so reading the
/// cast off it meant every character the process had ever loaded was authorized in
/// every subsequent fight. `select_gameplay` builds a fresh authorized-source map;
/// this is what makes that reset mean something.
#[test]
fn a_new_session_does_not_inherit_the_previous_casts_providers() {
    let mut app = session_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));

    stage(&mut app, "mary_o");
    finalize_and_update(&mut app);
    assert!(is_authorized(&app, "mary_o_demo"), "session one's cast");

    // A different fight, with a different fighter.
    begin_session(&mut app, 2);
    stage(&mut app, "sanic");
    finalize_and_update(&mut app);

    assert!(is_authorized(&app, "sanic_demo"), "session two's cast");
    assert!(
        !is_authorized(&app, "mary_o_demo"),
        "Mary-O is not in this fight. Authorizing her provider anyway is how a \
         fifty-character roster ends up authorizing fifty providers after an \
         evening of play — and it makes the two-characters-authorize-two-providers \
         invariant untestable in a long-running process"
    );
    let states = app
        .world()
        .resource::<crate::character_runtime::CharacterLoadStates>();
    assert!(
        states.staged_tokens().any(|token| token == "mary_o"),
        "the load HISTORY still remembers her — it is a diagnostic ledger, and \
         forgetting what failed to load two rooms ago is a different bug"
    );
}

/// A character nobody claims is skipped, not guessed at.
///
/// The load ledger already reports unknown characters with a reason; inventing a
/// provider here would authorize a source no emitter will ever tag, and a second
/// complaint about one fact is how a log stops being read.
#[test]
fn an_unclaimed_character_authorizes_nothing() {
    let mut app = session_app();
    stage(&mut app, "someone_elses_fighter");
    finalize_and_update(&mut app);

    let selection = app
        .world()
        .resource::<ambition_audio::selection::ActiveAudioSelection>();
    assert!(selection
        .sfx_for_source(&PresentationSourceId::new("someone_elses_fighter"))
        .is_none());
}

/// A13. A body's cues are attributed to ITS character's provider.
///
/// `write_from` had exactly one caller — the moveset timeline — so jump, dash,
/// damage and death all took their source from the single global
/// `SfxEmissionContext` and were attributed to whoever owned the session. In a
/// crossover fight that means Sanic's jump plays out of Ambition's bank.
///
/// This asserts the DERIVATION only: the same body, two different characters, two
/// different sources — and no component at all for a body wearing nothing, which
/// is materially different from an empty source.
///
/// What each emitter is credited with is asserted where that emitter runs:
/// `fight_tests:a_dying_body_dies_in_its_own_voice` for the death branch,
/// `two_provider_characters_trade_damage_through_the_real_damage_path` for the move timeline,
/// and [`a_projectile_keeps_its_firers_source_after_the_firer_is_gone`] for a bolt. A cue with
/// no such test is a cue nobody has checked.
#[test]
fn a_body_emits_under_its_own_characters_provider() {
    let mut app = session_app();
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));

    let sanic_body = app
        .world_mut()
        .spawn(ambition_characters::actor::WornCharacter::new("sanic"))
        .id();
    let mary_body = app
        .world_mut()
        .spawn(ambition_characters::actor::WornCharacter::new("mary_o"))
        .id();
    let bare_body = app.world_mut().spawn_empty().id();
    finalize_and_update(&mut app);
    // Guard against the whole test being vacuous: if the derivation never ran, all
    // three lookups would be `None` and the two positive assertions below would be
    // the only thing failing — which reads as a wiring bug, not as "nothing ran".
    assert!(
        app.world()
            .get::<ambition_sfx::BodyPresentationSource>(sanic_body)
            .is_some(),
        "the derivation system did not run at all"
    );

    let source_of = |app: &App, entity| {
        app.world()
            .get::<ambition_sfx::BodyPresentationSource>(entity)
            .map(|source| source.id().as_str().to_string())
    };
    assert_eq!(source_of(&app, sanic_body).as_deref(), Some("sanic_demo"));
    assert_eq!(source_of(&app, mary_body).as_deref(), Some("mary_o_demo"));
    assert_eq!(
        source_of(&app, bare_body),
        None,
        "a body wearing no character gets NO component: absent means `ask the \
         session`, which is the honest answer for a hazard, and is a different \
         fact from `belongs to nobody`"
    );
}

/// Putting on a different identity re-attributes the body's cues.
///
/// The whole point of deriving this every tick rather than at spawn: possession,
/// transformation, and an assist swap all change who a body sounds like.
#[test]
fn changing_worn_identity_changes_the_bodys_source() {
    let mut app = session_app();
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    let body = app
        .world_mut()
        .spawn(ambition_characters::actor::WornCharacter::new("sanic"))
        .id();
    finalize_and_update(&mut app);

    app.world_mut()
        .entity_mut(body)
        .insert(ambition_characters::actor::WornCharacter::new("mary_o"));
    finalize_and_update(&mut app);

    assert_eq!(
        app.world()
            .get::<ambition_sfx::BodyPresentationSource>(body)
            .map(|source| source.id().as_str().to_string())
            .as_deref(),
        Some("mary_o_demo"),
    );
}

/// G1: a projectile lands in the voice of whoever fired it — including after
/// that body is gone.
///
/// The impact and the detonation are emitted by the BOLT, so an attribution that
/// chased the owner back through `ProjectileOwner` at impact time attributed every
/// orphaned shot to the session. Stamping at spawn is what makes the bolt's own
/// provenance outlive its firer, and the second half of this test is the whole
/// reason for the stamp rather than the lookup.
#[test]
fn a_projectile_keeps_its_firers_source_after_the_firer_is_gone() {
    let mut app = session_app();
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));
    let firer = app
        .world_mut()
        .spawn(ambition_characters::actor::WornCharacter::new("sanic"))
        .id();
    finalize_and_update(&mut app);

    let bolt = app
        .world_mut()
        .spawn(ambition_projectiles::ProjectileOwner(firer))
        .id();
    finalize_and_update(&mut app);

    let source_of = |app: &App, entity| {
        app.world()
            .get::<ambition_sfx::BodyPresentationSource>(entity)
            .map(|source| source.id().as_str().to_string())
    };
    assert_eq!(
        source_of(&app, bolt).as_deref(),
        Some("sanic_demo"),
        "the bolt must inherit its firer's source at spawn"
    );

    app.world_mut().entity_mut(firer).despawn();
    finalize_and_update(&mut app);
    finalize_and_update(&mut app);

    assert_eq!(
        source_of(&app, bolt).as_deref(),
        Some("sanic_demo"),
        "and must KEEP it once the firer is gone — a shot in flight when its \
         character dies still impacts in that character's voice. This is also the \
         assertion that the per-tick derivation does not retract a source it did \
         not grant: it owns bodies, not everything that carries the component"
    );
}

/// G5: within one session the cast ACCUMULATES, and that is the contract.
///
/// Pinned rather than left to be inferred, because "this session's cast" reads like
/// a live roster and is not one. `ActiveAudioSelection` has no revoke — a source is
/// added by `authorize_sfx_source` and the map is only cleared by starting a new
/// session — so a cast that shrank could not un-authorize anybody, and a resource
/// that shrank would imply a revocation nothing performs.
///
/// The counterpart is [`a_new_session_does_not_inherit_the_previous_casts_providers`]:
/// the session is where this resets, and that is the boundary the fifty-character
/// roster problem actually lives on.
#[test]
fn a_second_room_in_one_session_adds_to_the_cast_rather_than_replacing_it() {
    let mut app = session_app();
    app.register_character(CharacterDefinition::new("mary_o", "Mary-O", "mary_o_demo"));
    app.register_character(CharacterDefinition::new("sanic", "Sanic", "sanic_demo"));

    stage(&mut app, "mary_o");
    finalize_and_update(&mut app);
    // A later room in the SAME session, staging somebody else.
    stage(&mut app, "sanic");
    finalize_and_update(&mut app);

    let states = app
        .world()
        .resource::<crate::character_runtime::CharacterLoadStates>();
    assert_eq!(
        states.cast().ids().collect::<Vec<_>>(),
        ["mary_o", "sanic"],
        "one session's capability set is the union of what it staged"
    );
    assert!(is_authorized(&app, "mary_o_demo") && is_authorized(&app, "sanic_demo"));
}

/// C3: registering a character reaches a body that never wore anything.
///
/// A spawned actor carries no `WornCharacter` — production inserts that only for the
/// player. Its identity is the sprite character its `CombatTuning` names, which is
/// the same chain [`publish_body_presentation_sources`] reads, and the projection
/// has to use it or C3 covers the player and nothing else in the room.
#[test]
fn a_spawned_actor_with_no_worn_character_still_gets_the_registered_moveset() {
    use ambition_entity_catalog::{ClipBinding, MoveGates, MoveSpec, MovesetContract};

    let mut app = session_app();

    // The smallest move that is still a move: the projection cares about the verb
    // table, not about what the swing does.
    let swat = MoveSpec {
        display_name: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        id: "swat".to_string(),
        clip: ClipBinding {
            clip: "swat".to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.2,
        events: vec![],
        windows: vec![],
        gates: MoveGates::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
    };
    let moveset = MovesetContract {
        verbs: std::collections::BTreeMap::from([(
            ambition_combat::moveset::ATTACK_VERB.to_string(),
            "swat".to_string(),
        )]),
        moves: vec![swat],
    };
    app.register_character(
        CharacterDefinition::new("badnik", "Badnik", "sanic_demo").with_moveset(moveset),
    );

    let actor = app
        .world_mut()
        .spawn(ambition_combat::CombatTuning {
            sprite_character_id: Some("badnik".to_string()),
            ..Default::default()
        })
        .id();
    settle(&mut app);

    assert!(
        app.world()
            .get::<ambition_combat::moveset::ActorMoveset>(actor)
            .is_some(),
        "a spawned actor is identified by its combat tuning's sprite character, so \
         the registry's authored moveset must reach it too — otherwise C3 covers \
         the player and nothing else in the room"
    );
    assert!(
        app.world()
            .get::<ambition_combat::moveset::MovesetMelee>(actor)
            .is_some(),
        "and the `attack` verb routes its melee through the move timeline, the same \
         marker `ActorClusterSeed` derives from an authored moveset"
    );
}

/// A body whose character is NOT registered keeps whatever built it.
///
/// The projection fills in from the registry; it does not clear. A catalog-built
/// actor in a composition with an empty registry must be untouched, which is the
/// ordinary state of every actor in the game today.
#[test]
fn an_unregistered_character_leaves_the_body_as_its_spawn_built_it() {
    let mut app = session_app();
    let actor = app
        .world_mut()
        .spawn(ambition_combat::CombatTuning {
            sprite_character_id: Some("somebody_elses_fighter".to_string()),
            ..Default::default()
        })
        .id();
    finalize_and_update(&mut app);

    assert!(app
        .world()
        .get::<ambition_combat::moveset::ActorMoveset>(actor)
        .is_none());
}

/// Recharacterization retracts moveset and melee-routing state that the new
/// character does not author.
#[test]
fn wearing_a_quieter_character_retracts_the_previous_ones_moves() {
    use ambition_entity_catalog::{ClipBinding, MoveGates, MoveSpec, MovesetContract};

    let mut app = session_app();
    let swat = MoveSpec {
        display_name: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        id: "swat".to_string(),
        clip: ClipBinding {
            clip: "swat".to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.2,
        events: vec![],
        windows: vec![],
        gates: MoveGates::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
    };
    app.register_character(
        CharacterDefinition::new("armed", "Armed", "demo").with_moveset(MovesetContract {
            verbs: std::collections::BTreeMap::from([(
                ambition_combat::moveset::ATTACK_VERB.to_string(),
                "swat".to_string(),
            )]),
            moves: vec![swat],
        }),
    );
    app.register_character(CharacterDefinition::new("unarmed", "Unarmed", "demo"));

    let body = app
        .world_mut()
        .spawn(ambition_characters::actor::WornCharacter::new("armed"))
        .id();
    settle(&mut app);
    assert!(
        app.world()
            .get::<ambition_combat::moveset::ActorMoveset>(body)
            .is_some(),
        "the armed form projects its moveset"
    );

    assert!(
        app.world()
            .get::<ambition_combat::moveset::MovesetMelee>(body)
            .is_some(),
        "a moveset authoring `attack` routes melee through the move timeline"
    );

    app.world_mut()
        .entity_mut(body)
        .insert(ambition_characters::actor::WornCharacter::new("unarmed"));
    settle(&mut app);

    // NOT `is_none()`, which is what this asserted when it was written.
    //
    // `ActorMoveset` must SURVIVE the identity change, because `apply_worn_character_gameplay`
    // takes it as a required query column: a body that loses the component stops matching the
    // PERSONA DERIVE ENTIRELY and never gets a name, an action set or an identity kit again.
    // This fixture could not see it, because it installs `CharacterRuntimePlugin` alone and the
    // persona derive is not in it — the exact shape of the trap.
    //
    // Replacing the VALUE on a swap belongs to the persona derive, which is the
    // single writer for a worn body; that half is pinned in
    // `wearing_a_quieter_character_replaces_the_previous_moveset` beside it, and
    // the routing that follows the value is pinned by
    // `routing_markers_are_derived_from_whatever_wrote_the_moveset`.
    assert!(
        app.world()
            .get::<ambition_combat::moveset::ActorMoveset>(body)
            .is_some(),
        "the component the persona derive requires must not be removed by a \
         projection; the persona derive is the writer that replaces its VALUE"
    );
    assert_eq!(
        app.world()
            .get::<super::ProjectedCharacterKit>(body)
            .map(|kit| kit.id.as_str()),
        Some("unarmed"),
        "the projection must record the CURRENT identity, or the next swap \
         retracts against the wrong definition"
    );
}

/// The routing markers follow the moveset, whoever wrote it.
///
/// Driven by writing the moveset directly, which is exactly what an unknown
/// third writer would do.
#[test]
fn routing_markers_are_derived_from_whatever_wrote_the_moveset() {
    use ambition_entity_catalog::{ClipBinding, MoveGates, MoveSpec, MovesetContract};

    fn contract(verb: &str) -> MovesetContract {
        MovesetContract {
            verbs: std::collections::BTreeMap::from([(verb.to_string(), "m".to_string())]),
            moves: vec![MoveSpec {
                display_name: None,
                landing_lag_s: None,
                autocancel_after_s: None,
                sprite_spin_hz: None,
                equips: None,
                id: "m".to_string(),
                clip: ClipBinding {
                    clip: "m".to_string(),
                    fallbacks: vec![],
                },
                duration_s: 0.2,
                events: vec![],
                windows: vec![],
                gates: MoveGates::default(),
                start_impulse: None,
                smash_charge_mult: 1.0,
                smash_charge: None,
                charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
                repeat: None,
            }],
        }
    }

    let mut app = App::new();
    app.add_systems(
        bevy::app::Update,
        ambition_combat::moveset::reconcile_moveset_routing_markers,
    );
    let body = app
        .world_mut()
        .spawn(ambition_combat::moveset::ActorMoveset(contract(
            ambition_combat::moveset::ATTACK_VERB,
        )))
        .id();
    finalize_and_update(&mut app);
    assert!(app
        .world()
        .get::<ambition_combat::moveset::MovesetMelee>(body)
        .is_some());
    assert!(app
        .world()
        .get::<ambition_characters::brain::MovesetRanged>(body)
        .is_none());

    // A swap to a ranged-only moveset must move the routing with it — both ways
    // in one step, which is the case a one-directional "insert if present" misses.
    *app.world_mut()
        .get_mut::<ambition_combat::moveset::ActorMoveset>(body)
        .unwrap() =
        ambition_combat::moveset::ActorMoveset(contract(ambition_combat::moveset::RANGED_VERB));
    finalize_and_update(&mut app);
    assert!(
        app.world()
            .get::<ambition_combat::moveset::MovesetMelee>(body)
            .is_none(),
        "melee routing outlived a moveset with no `attack` verb"
    );
    assert!(
        app.world()
            .get::<ambition_characters::brain::MovesetRanged>(body)
            .is_some(),
        "a moveset authoring `ranged` was not routed through the move timeline, so \
         the shot falls back to the flat emitter and never samples live aim"
    );
}

/// `CharacterCatalogGeneration` existed for a day with no production reader — X4
/// was marked done on the strength of a counter nothing compared against. The
/// projection early-exits when the worn id is unchanged, so replacing the cast
/// underneath a body left it wearing the PREVIOUS cast's moves while every check
/// stayed green, because the id it wore was still the id it wore.
///
/// Same id, new cast, different moves: the exact case the id-only comparison
/// could not see. This test fails on the version of `project_prepared_character_definitions`
/// that compared ids alone — verified by reverting the comparison before trusting
/// the green.
#[test]
fn replacing_the_cast_reprojects_a_body_wearing_the_same_character() {
    use ambition_entity_catalog::{ClipBinding, MoveGates, MoveSpec, MovesetContract};

    fn one_move(id: &str) -> MovesetContract {
        MovesetContract {
            verbs: std::collections::BTreeMap::from([(
                ambition_combat::moveset::ATTACK_VERB.to_string(),
                id.to_string(),
            )]),
            moves: vec![MoveSpec {
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
                duration_s: 0.2,
                events: vec![],
                windows: vec![],
                gates: MoveGates::default(),
                start_impulse: None,
                smash_charge_mult: 1.0,
                smash_charge: None,
                charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
                repeat: None,
            }],
        }
    }

    fn projected_move(app: &App, body: Entity) -> Option<String> {
        app.world()
            .get::<ambition_combat::moveset::ActorMoveset>(body)
            .and_then(|moveset| moveset.0.moves.first())
            .map(|spec| spec.id.clone())
    }

    let mut app = session_app();
    app.register_character(
        CharacterDefinition::new("hero", "Hero", "demo").with_moveset(one_move("old_swing")),
    );
    let body = app
        .world_mut()
        .spawn(ambition_characters::actor::WornCharacter::new("hero"))
        .id();
    settle(&mut app);
    assert_eq!(
        projected_move(&app, body).as_deref(),
        Some("old_swing"),
        "the first cast must reach the body at all, or the replacement below \
         would pass vacuously"
    );

    // THE CAST IS REPLACED. Same id, same display name, different moves — a hot
    // reload, or a second composition's registry landing on a running session.
    let replacement = crate::character_runtime::prepare_and_finalize_for_test(
        CharacterDefinition::new("hero", "Hero", "demo").with_moveset(one_move("new_swing")),
        &ambition_characters::prepared::CharacterBindings::default(),
    )
    .prepared;
    app.world_mut()
        .resource_mut::<PreparedCharacterRegistry>()
        .insert_prepared(replacement);
    settle(&mut app);

    assert_eq!(
        projected_move(&app, body).as_deref(),
        Some("new_swing"),
        "the body kept the retired cast's moves. Its worn id never changed, so an \
         id-only comparison reports 'already projected' — which is why the cast \
         generation has to be part of what the body remembers, not merely a \
         counter something could compare if it thought to"
    );
}

/// A character that becomes UNAUTHORED in a new cast loses what it granted.
///
/// For a same-id replacement whose new definition authors nothing, the lookup returns the new,
/// empty definition — so nothing is retracted and the body keeps the retired hurtbox document
/// forever, with the projection reporting success.
///
/// Historical ownership is not a property of the new authority.
#[test]
fn a_character_that_stops_authoring_hurtboxes_has_them_retracted() {
    use ambition_entity_catalog::HurtboxDoc;

    fn doc() -> HurtboxDoc {
        HurtboxDoc {
            default: None,
            poses: std::collections::BTreeMap::new(),
            moves: std::collections::BTreeMap::new(),
        }
    }

    let mut app = session_app();
    app.register_character(
        CharacterDefinition::new("armored", "Armored", "demo").with_hurtboxes(doc()),
    );
    let body = app
        .world_mut()
        .spawn(ambition_characters::actor::WornCharacter::new("armored"))
        .id();
    settle(&mut app);
    assert!(
        app.world()
            .get::<super::super::AuthoredHurtboxes>(body)
            .is_some(),
        "the first cast must grant the hurtbox doc at all"
    );

    // Same id, new cast, authoring NOTHING.
    let stripped = crate::character_runtime::prepare_and_finalize_for_test(
        CharacterDefinition::new("armored", "Armored", "demo"),
        &ambition_characters::prepared::CharacterBindings::default(),
    )
    .prepared;
    app.world_mut()
        .resource_mut::<PreparedCharacterRegistry>()
        .insert_prepared(stripped);
    settle(&mut app);

    assert!(
        app.world()
            .get::<super::super::AuthoredHurtboxes>(body)
            .is_none(),
        "the body kept the retired cast's authored hurtboxes: retraction asked the \
         NEW registry what the OLD definition granted, and the new one grants \
         nothing, so it removed nothing"
    );
}

/// `CharacterDefinition.body` has existed since §4.11 with no consumer anywhere
/// in the repository: a provider could author `SpriteAuthored { world_per_pixel }`
/// and receive a body of some other size entirely. `SpritePosedBody` — which
/// carries exactly that number and drives the collision box, sprite quad and
/// offset off the art every tick — was inserted from ONE place: a bespoke
/// app-side system in the Mary-O snake matching on a display name. Body geometry
/// was still declared through a second seam, which is the problem
/// `register_character` exists to delete.
#[test]
fn a_character_authoring_a_sprite_body_gets_a_posed_body() {
    let mut app = session_app();
    let mut shaped = CharacterDefinition::new("serpent", "Serpent", "demo").with_sheet("robot");
    shaped.body = Some(
        ambition_characters::actor::definition::BodySource::SpriteAuthored {
            world_per_pixel: 2.5,
        },
    );
    app.register_character(shaped);
    app.register_character(CharacterDefinition::new("plain", "Plain", "demo").with_sheet("robot"));

    let body = app
        .world_mut()
        .spawn(ambition_characters::actor::WornCharacter::new("serpent"))
        .id();
    settle(&mut app);

    let posed = app
        .world()
        .get::<ambition_sprite_sheet::character::SpritePosedBody>(body)
        .expect("an authored sprite body must reach the body it describes");
    assert_eq!(
        posed.target, "robot",
        "the posed body reads the AUTHORED sheet"
    );
    assert_eq!(
        posed.world_per_pixel, 2.5,
        "and the authored scale, which is the whole of what this field says"
    );

    // And it is RETRACTED on a change of identity, like every other grant this
    // system makes — otherwise a body that becomes a plain character keeps
    // resolving its box off the previous one's art.
    app.world_mut()
        .entity_mut(body)
        .insert(ambition_characters::actor::WornCharacter::new("plain"));
    settle(&mut app);
    assert!(
        app.world()
            .get::<ambition_sprite_sheet::character::SpritePosedBody>(body)
            .is_none(),
        "the previous character's posed body survived an identity change, so the \
         body keeps deriving its collision box from art it no longer wears"
    );
}
