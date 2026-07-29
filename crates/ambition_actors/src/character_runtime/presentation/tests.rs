//! A4: the cast's sources are authorized by PRODUCTION code, not by a test.

// Stepping a fixture is `finalize_and_update`, not `update`. Bevy's RUNNERS
// close the plugin-composition barrier; `App::update` does not, and character
// preparation publishes its registry there — so a fixture that only updated
// would register a cast and never publish one. Idempotent, so a helper called
// per step costs a set lookup after the first.
use ambition_platformer_primitives::app_finalization::finalize_and_update;

use super::*;
use crate::character_runtime::{
    CharacterDefinition, CharacterDefinitionAppExt, CharacterRuntimePlugin,
};
use ambition_sfx::PresentationSourceId;

/// One gameplay session owning the speakers, with `ambition` as the primary.
fn session_app() -> App {
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    // The routing markers (`MovesetMelee` / `MovesetRanged`) are DERIVED from the
    // live `ActorMoveset` by a system this plugin owns. Installing it here is not
    // fixture padding: a projection test that asserts on routing while the
    // deriver is absent is asserting on whatever the projection happened to write,
    // which is exactly the arrangement that hid the `ActorMoveset` retraction bug.
    app.add_plugins(crate::action_scheme::ActionSchemePlugin);
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<crate::character_sprites::AuthoredSheets>();
    app.init_resource::<ambition_platformer_primitives::lifecycle::ActiveSessionScope>();
    begin_session(&mut app, 1);
    app
}

/// Step the fixture until derived state has settled.
///
/// Two updates, not one. The shipped composition orders the projection, the
/// persona derive, the equipment overlay and the marker reconcile inside one tick
/// of `PlayerInputSet`; this fixture installs the plugins WITHOUT
/// `configure_sandbox_sets`, so those sets carry no ordering and the reconcile
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
        .resource_mut::<ambition_platformer_primitives::lifecycle::ActiveSessionScope>()
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
        .resource_mut::<crate::character_runtime::CharacterLoadDemand>()
        .request(character_id);
}

/// **The gap this closes.** A secondary provider's cue was DENIED in production.
///
/// `write_from` tags a request with the emitting character's provider, and
/// `authorize_sfx_source` is what makes that tag resolvable. Only a rendered test
/// ever called the second one, so in a real session a correctly-tagged cue from
/// any provider other than the session owner hit the audio authority, found no
/// authorization, and was dropped — with nothing reported, because the request was
/// well-formed and the refusal was silent.
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

/// **A room stages a display name, and the right provider is authorized.**
///
/// Rooms author characters by the name a designer typed — `demand_room_character_sheets`
/// pushes `enemy.name` and an interactable's `character_id` straight through — while
/// every provider map is keyed by stable id. The load ledger records the token it
/// was handed, so a cast read off that ledger asked for the provider of `"Mary-O"`,
/// found nothing, and skipped her: the character loaded correctly and was silent.
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

/// **A later session does not authorize the previous session's cast.**
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

/// **A13.** A body's cues are attributed to ITS character's provider.
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
/// It deliberately does not claim that every emitter reads it, which is what an
/// earlier version of this comment said while eighty-six call sites still wrote
/// through the session context (GPT 5.6, 2026-07-26 — the claim was wider than the
/// code). What each emitter is credited with is asserted where that emitter runs:
/// `fight_tests::a_dying_body_dies_in_its_own_voice` for the death branch,
/// `two_provider_characters_trade_damage_through_the_real_damage_path` for the move
/// timeline, and [`a_projectile_keeps_its_firers_source_after_the_firer_is_gone`]
/// for a bolt. A cue with no such test is a cue nobody has checked.
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

/// **G1: a projectile lands in the voice of whoever fired it — including after
/// that body is gone.**
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

/// **G5: within one session the cast ACCUMULATES, and that is the contract.**
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

/// **C3: registering a character reaches a body that never wore anything.**
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
        id: "swat".to_string(),
        clip: ClipBinding {
            clip: "swat".to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.2,
        events: vec![],
        windows: vec![],
        gates: MoveGates { grounded: None },
        start_impulse: None,
        smash_charge_mult: 1.0,
    };
    let moveset = MovesetContract {
        verbs: std::collections::BTreeMap::from([(
            crate::combat::moveset::ATTACK_VERB.to_string(),
            "swat".to_string(),
        )]),
        moves: vec![swat],
    };
    app.register_character(
        CharacterDefinition::new("badnik", "Badnik", "sanic_demo").with_moveset(moveset),
    );

    let actor = app
        .world_mut()
        .spawn(crate::combat::CombatTuning {
            sprite_character_id: Some("badnik".to_string()),
            ..Default::default()
        })
        .id();
    settle(&mut app);

    assert!(
        app.world()
            .get::<crate::combat::moveset::ActorMoveset>(actor)
            .is_some(),
        "a spawned actor is identified by its combat tuning's sprite character, so \
         the registry's authored moveset must reach it too — otherwise C3 covers \
         the player and nothing else in the room"
    );
    assert!(
        app.world()
            .get::<crate::combat::moveset::MovesetMelee>(actor)
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
        .spawn(crate::combat::CombatTuning {
            sprite_character_id: Some("somebody_elses_fighter".to_string()),
            ..Default::default()
        })
        .id();
    finalize_and_update(&mut app);

    assert!(app
        .world()
        .get::<crate::combat::moveset::ActorMoveset>(actor)
        .is_none());
}

/// **A form change must not leave the previous character's fight behind.**
///
/// Insert-only projection kept the old moveset, the old silhouette, and the old
/// `MovesetMelee` routing marker whenever the new definition was quieter than the
/// old one (GPT 5.6, 2026-07-27). That is not hypothetical: Sanic's super form and
/// Mary-O's power tiers are exactly this — one body, a new worn character — and a
/// stale melee marker keeps diverting attacks into a move timeline the new form
/// does not have.
#[test]
fn wearing_a_quieter_character_retracts_the_previous_ones_moves() {
    use ambition_entity_catalog::{ClipBinding, MoveGates, MoveSpec, MovesetContract};

    let mut app = session_app();
    let swat = MoveSpec {
        id: "swat".to_string(),
        clip: ClipBinding {
            clip: "swat".to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.2,
        events: vec![],
        windows: vec![],
        gates: MoveGates { grounded: None },
        start_impulse: None,
        smash_charge_mult: 1.0,
    };
    app.register_character(
        CharacterDefinition::new("armed", "Armed", "demo").with_moveset(MovesetContract {
            verbs: std::collections::BTreeMap::from([(
                crate::combat::moveset::ATTACK_VERB.to_string(),
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
            .get::<crate::combat::moveset::ActorMoveset>(body)
            .is_some(),
        "the armed form projects its moveset"
    );

    assert!(
        app.world()
            .get::<crate::combat::moveset::MovesetMelee>(body)
            .is_some(),
        "a moveset authoring `attack` routes melee through the move timeline"
    );

    app.world_mut()
        .entity_mut(body)
        .insert(ambition_characters::actor::WornCharacter::new("unarmed"));
    settle(&mut app);

    // ⚠ NOT `is_none()`, which is what this asserted when it was written.
    //
    // `ActorMoveset` must SURVIVE the identity change, because
    // `apply_worn_character_gameplay` takes it as a required query column: a body
    // that loses the component stops matching the PERSONA DERIVE ENTIRELY and
    // never gets a name, an action set or an identity kit again. Removing it
    // looked like careful retraction and was silent, permanent damage (GPT 5.6,
    // 2026-07-27). This fixture could not see it, because it installs
    // `CharacterRuntimePlugin` alone and the persona derive is not in it — the
    // exact shape of the trap.
    //
    // Replacing the VALUE on a swap belongs to the persona derive, which is the
    // single writer for a worn body; that half is pinned in
    // `wearing_a_quieter_character_replaces_the_previous_moveset` beside it, and
    // the routing that follows the value is pinned by
    // `routing_markers_are_derived_from_whatever_wrote_the_moveset`.
    assert!(
        app.world()
            .get::<crate::combat::moveset::ActorMoveset>(body)
            .is_some(),
        "the component the persona derive requires must not be removed by a \
         projection; the persona derive is the writer that replaces its VALUE"
    );
    assert_eq!(
        app.world()
            .get::<super::ProjectedCharacterKit>(body)
            .map(|kit| kit.0.as_str()),
        Some("unarmed"),
        "the projection must record the CURRENT identity, or the next swap \
         retracts against the wrong definition"
    );
}

/// **The routing markers follow the moveset, whoever wrote it.**
///
/// The catalog persona path replaces `ActorMoveset` wholesale on a kit swap and
/// has never touched `MovesetMelee` / `MovesetRanged`, so before these became
/// derived, a swap between two CATALOG characters left the previous one's
/// routing attached — a case the prepared-registry projection could not fix
/// because it never runs for a character it does not know.
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
                id: "m".to_string(),
                clip: ClipBinding {
                    clip: "m".to_string(),
                    fallbacks: vec![],
                },
                duration_s: 0.2,
                events: vec![],
                windows: vec![],
                gates: MoveGates { grounded: None },
                start_impulse: None,
                smash_charge_mult: 1.0,
            }],
        }
    }

    let mut app = App::new();
    app.add_systems(
        bevy::app::Update,
        crate::combat::moveset::reconcile_moveset_routing_markers,
    );
    let body = app
        .world_mut()
        .spawn(crate::combat::moveset::ActorMoveset(contract(
            crate::combat::moveset::ATTACK_VERB,
        )))
        .id();
    finalize_and_update(&mut app);
    assert!(app
        .world()
        .get::<crate::combat::moveset::MovesetMelee>(body)
        .is_some());
    assert!(app
        .world()
        .get::<ambition_characters::brain::MovesetRanged>(body)
        .is_none());

    // A swap to a ranged-only moveset must move the routing with it — both ways
    // in one step, which is the case a one-directional "insert if present" misses.
    *app.world_mut()
        .get_mut::<crate::combat::moveset::ActorMoveset>(body)
        .unwrap() =
        crate::combat::moveset::ActorMoveset(contract(crate::combat::moveset::RANGED_VERB));
    finalize_and_update(&mut app);
    assert!(
        app.world()
            .get::<crate::combat::moveset::MovesetMelee>(body)
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
