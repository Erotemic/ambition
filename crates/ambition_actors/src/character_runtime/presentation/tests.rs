//! A4: the cast's sources are authorized by PRODUCTION code, not by a test.

use super::*;
use crate::character_runtime::{
    CharacterDefinition, CharacterDefinitionAppExt, CharacterRuntimePlugin,
};
use ambition_sfx::PresentationSourceId;

/// One gameplay session owning the speakers, with `ambition` as the primary.
fn session_app() -> App {
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<ambition_platformer_primitives::lifecycle::ActiveSessionScope>();
    begin_session(&mut app, 1);
    app
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
    app.update();

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
    app.update();

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
    app.update();

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
    app.update();
    assert!(is_authorized(&app, "mary_o_demo"), "session one's cast");

    // A different fight, with a different fighter.
    begin_session(&mut app, 2);
    stage(&mut app, "sanic");
    app.update();

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
    app.update();

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
    app.update();
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
    app.update();

    app.world_mut()
        .entity_mut(body)
        .insert(ambition_characters::actor::WornCharacter::new("mary_o"));
    app.update();

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
    app.update();

    let bolt = app
        .world_mut()
        .spawn(ambition_projectiles::ProjectileOwner(firer))
        .id();
    app.update();

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
    app.update();
    app.update();

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
    app.update();
    // A later room in the SAME session, staging somebody else.
    stage(&mut app, "sanic");
    app.update();

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
