//! What is left of the death beat that is Mary-O's own.
//!
//! What stays is what this crate still owns: the music, and its authorization.

use super::*;
use ambition_platformer2d::combat::death_rules::DeathInterlude;

/// The death track is DECLARED, not just requested.
///
/// Under provider-relative playback a session plays only the tracks its own
/// audio fragment names, so asking for an undeclared id is gated to silence —
/// the request succeeds, the music does not. That is the exact shape of every
/// other thing in this demo that "worked" while producing nothing, so pin the
/// declaration rather than trusting it.
#[test]
fn the_death_track_is_authorized_by_the_provider_fragment() {
    let mut app = App::new();
    app.add_plugins(crate::provider::MaryOExperiencePlugin);
    let registry = app
        .world()
        .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>()
        .combined_music_registry(crate::provider::MARY_O_EXPERIENCE)
        .expect("Mary-O's audio fragment assembles");
    assert!(
        registry
            .tracks
            .iter()
            .any(|track| track.id == crate::provider::MARY_O_DEATH_MUSIC_TRACK),
        "her death track must be declared by the fragment that authorizes it; \
         got {:?}",
        registry.tracks.iter().map(|t| &t.id).collect::<Vec<_>>()
    );
}

/// She states the dwell the music was written for.
///
/// The number itself is sized by the score (see [`DEATH_DWELL`]), and the
/// declaration is how the engine learns it. A demo that stated no rules would
/// get the engine default — no interlude and no level reset — which is a
/// silent, entirely playable wrong answer, so the statement is worth pinning.
#[test]
fn her_rules_hold_the_level_for_the_length_of_the_death_music() {
    let mut app = App::new();
    app.add_plugins(crate::MaryORulesPlugin::global());
    let rules = app
        .world()
        .get_resource::<ambition_platformer2d::combat::death_rules::DeclaredDeathRules>()
        .expect("Mary-O states her death rules")
        // Standalone, the demo IS the game, so an untagged fixture room is hers.
        .governing(None);
    assert_eq!(
        rules.interlude, DEATH_DWELL,
        "the interlude is the length of `mary_o_you_died`"
    );
    assert_eq!(
        rules.level_reset,
        ambition_platformer2d::combat::death_rules::LevelReset::WhenNoParticipantRemains,
        "the level goes back when nobody is left in play — the same value NSMB \
         co-op would use, with a roster of one"
    );
}

/// It plays for the beat and hands the level's own theme back afterwards.
///
/// The window is the ENGINE's, so this seats one by hand rather than driving a
/// death: what is under test is the claim/release, not who opened the window.
#[test]
fn the_death_music_claims_the_priority_tier_and_releases_it() {
    let mut app = App::new();
    app.add_systems(bevy::prelude::Update, play_death_music);
    let root = app
        .world_mut()
        .spawn((
            ambition_platformer2d::platformer::lifecycle::SessionRoot(session_scope_for_test()),
            ambition_platformer2d::encounter::EncounterMusicRequest::default(),
        ))
        .id();
    let body = app.world_mut().spawn(()).id();

    app.update();
    assert_eq!(
        requested(&app, root),
        None,
        "a level nobody has died on plays its own theme"
    );

    app.world_mut().entity_mut(body).insert(DeathInterlude {
        remaining: 1.0,
        consequence_pending: true,
    });
    app.update();
    assert_eq!(
        requested(&app, root).as_deref(),
        Some(crate::provider::MARY_O_DEATH_MUSIC_TRACK),
        "her death takes the priority tier — the one slot that outranks the room"
    );

    // The window closing is the engine removing the component; a closed-but-
    // present window must read the same way, so check the value too.
    app.world_mut().entity_mut(body).insert(DeathInterlude {
        remaining: 0.0,
        consequence_pending: false,
    });
    app.update();
    assert_eq!(
        requested(&app, root),
        None,
        "and gives it straight back, so the level theme returns on its own"
    );
}

/// A session scope id for a bare test root.
fn session_scope_for_test() -> ambition_platformer2d::platformer::lifecycle::SessionScopeId {
    let mut scope = ambition_platformer2d::platformer::lifecycle::ActiveSessionScope::default();
    scope.begin()
}

fn requested(app: &App, root: bevy::prelude::Entity) -> Option<String> {
    app.world()
        .get::<ambition_platformer2d::encounter::EncounterMusicRequest>(root)
        .and_then(|music| music.priority_track.clone())
}
