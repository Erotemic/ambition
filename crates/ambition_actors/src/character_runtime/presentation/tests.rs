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
    let mut selection = ambition_audio::selection::ActiveAudioSelection::default();
    selection.select_gameplay(1, "ambition", None, None, Default::default());
    app.insert_resource(selection);
    app
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

    assert!(
        app.world()
            .resource::<ambition_audio::selection::ActiveAudioSelection>()
            .is_sfx_source_authorized(&PresentationSourceId::new("sanic_demo")),
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
    assert!(
        selection
            .sfx_for_source(&PresentationSourceId::new("someone_elses_fighter"))
            .is_none()
    );
}
