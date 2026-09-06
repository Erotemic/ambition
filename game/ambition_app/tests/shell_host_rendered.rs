//! X1 — rendered (no-window) ownership across the host lifecycle.
//!
//! Drives the REAL visible composition (`build_visible_app` — the exact App
//! the desktop binary runs, minus the window/wgpu backend) through
//! title → Ambition gameplay → title → Sanic gameplay → title, and asserts
//! presentation OWNERSHIP at every stop:
//!
//! - the host cameras exist from boot and survive every transition
//!   (host-owned infrastructure, not gameplay leakage);
//! - the title screen shows the launcher UI and ZERO gameplay presentation
//!   (no room visuals, no HUD text, no LDtk spine roots, no player);
//! - an Ambition session draws its LDtk room + HUD, all session-scoped;
//! - a Sanic session draws through the SAME provider-agnostic
//!   `SessionRoomVisualsPlugin` — no per-game visual wiring in the host;
//! - Quit to Home retires every session-owned visual exactly.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use ambition_app::app::{shell_host, VisibleRenderMode};
use ambition_platformer2d::game_shell::{
    ActiveFrontendAuthority, ActiveGameplaySession, BasicSequenceRoot, BasicShellUiRoot,
    FrontendOwnedEntity, FrontendPresentationKind, GameplayInputOwner, GameplaySessionWorldRoot,
    PreparedSessionRegistry, PresentationOwnershipClass, PresentationOwnershipPolicy, ShellCommand,
    ShellLauncherCommand, ShellRouter,
};
use ambition_platformer2d::load::LoadCoordinator;
use ambition_platformer2d::load_presentation::{
    BasicLoadRoot, LoadActivityState, LoadForegroundState,
};
use ambition_platformer2d::platformer::lifecycle::{
    ActiveSessionScope, RoomVisual, SessionScopedEntity,
};
use ambition_platformer2d::render::rendering::HudText;

/// The real visible composition, stepped on a PINNED timestep.
///
/// `app.update()` under Bevy's default `TimeUpdateStrategy::Automatic` advances the clock by REAL
/// elapsed wall-clock, so the number of `FixedUpdate` steps a single `update()` runs depends on how
/// busy the machine is. These tests count audio-playback events, and a session emits its own
/// legitimate cues as its gameplay advances — so under `Automatic` a loaded machine silently runs
/// more sim per frame and the counts move.
///
/// Pinning the timestep makes `settle()` mean an exact number of sim frames on
/// any machine, exactly as the sibling `shell_host_startup` module already does.
fn rendered_app() -> App {
    let mut app = ambition_app::app::build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    app
}

/// Let a routed request land: while the shell holds a PENDING route its
/// preparation barrier is loading — and since the first room's art is part of
/// that barrier (`prepare-first-room-art`), the wait is a real decode, not a
/// fixed count of frames. Six more updates after it settle the presentation.
fn settle(app: &mut App) {
    // A written `GoTo` becomes a pending route on the next update; a
    // `ShellLauncherCommand` takes one more (the launcher turns it into a
    // `GoTo` first). Wait for the route to APPEAR, then for it to settle —
    // a fixed "one update" here read the launcher's relaunch as already done
    // the frame before it started, which was masked while a relaunched
    // room's cast was still resident from the first session and stopped being
    // masked when room exits began retiring it (2026-09-02).
    for _ in 0..8 {
        app.update();
        if app.world().resource::<ShellRouter>().pending.is_some() {
            break;
        }
    }
    for _ in 0..1200 {
        if app.world().resource::<ShellRouter>().pending.is_none() {
            break;
        }
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    for _ in 0..6 {
        app.update();
    }
}

fn active_route(app: &App) -> Option<String> {
    app.world()
        .resource::<ShellRouter>()
        .active
        .as_ref()
        .map(|active| active.route_id.as_str().to_owned())
}

fn count<C: Component>(app: &mut App) -> usize {
    let mut query = app.world_mut().query_filtered::<Entity, With<C>>();
    query.iter(app.world()).count()
}

fn main_cameras(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<Entity, With<ambition_platformer2d::platformer::camera_layers::MainCamera>>();
    query.iter(app.world()).count()
}

fn launcher_ui_roots(app: &mut App) -> usize {
    count::<BasicShellUiRoot>(app)
}

fn frontend_kind(app: &mut App, kind: FrontendPresentationKind) -> usize {
    let mut query = app.world_mut().query::<&FrontendOwnedEntity>();
    query
        .iter(app.world())
        .filter(|owned| owned.kind == kind)
        .count()
}

/// The track the music director currently has on the base channel (empty =
/// silence). This is the REAL playback state the director writes, not merely the
/// selection — `build_visible_app` composes the actual audio director against
/// the in-memory recording backend, so no physical audio device is opened.
fn active_music_track(app: &App) -> String {
    app.world()
        .resource::<ambition_platformer2d::audio::library::MusicPlaybackState>()
        .active_track()
        .to_string()
}

fn assert_recording_audio_output(app: &App) {
    assert_eq!(
        *app.world()
            .resource::<ambition_platformer2d::audio::AudioOutputMode>(),
        ambition_platformer2d::audio::AudioOutputMode::Recording,
        "no-window tests must record accepted playback without issuing device play commands"
    );
    let backend = app
        .world()
        .resource::<ambition_platformer2d::audio::AudioBackendState>();
    assert_eq!(
        backend.mode,
        ambition_platformer2d::audio::AudioOutputMode::Recording
    );
    assert!(
        !backend.device_backend_installed,
        "recording tests must not initialize Kira's physical-device backend",
    );
}

fn assert_title_ownership(app: &mut App, context: &str) {
    assert_eq!(
        active_route(app),
        Some(shell_host::AMBITION_LAUNCHER_ROUTE.to_owned()),
        "{context}: launcher route active"
    );
    assert_eq!(
        main_cameras(app),
        1,
        "{context}: exactly one host main camera"
    );
    assert_eq!(
        launcher_ui_roots(app),
        1,
        "{context}: exactly one launcher/frontend UI root owns the title"
    );
    assert_eq!(
        frontend_kind(app, FrontendPresentationKind::HostCamera),
        1,
        "{context}: exactly one host world camera",
    );
    assert_eq!(
        frontend_kind(app, FrontendPresentationKind::FrontendUiCamera),
        1,
        "{context}: exactly one host UI camera",
    );
    assert_eq!(
        frontend_kind(app, FrontendPresentationKind::LauncherRoot),
        1,
        "{context}: launcher root has explicit frontend ownership",
    );
    assert_eq!(
        count::<BasicSequenceRoot>(app),
        0,
        "{context}: startup root retired"
    );
    assert_eq!(
        count::<BasicLoadRoot>(app),
        0,
        "{context}: loading root retired"
    );
    assert!(
        app.world().resource::<ActiveGameplaySession>().0.is_none(),
        "{context}: no gameplay-session authority",
    );
    assert!(
        app.world()
            .resource::<ActiveSessionScope>()
            .current()
            .is_none(),
        "{context}: no active gameplay scope",
    );
    assert_eq!(
        count::<GameplaySessionWorldRoot>(app),
        0,
        "{context}: no gameplay world"
    );
    assert_eq!(
        count::<GameplayInputOwner>(app),
        0,
        "{context}: no gameplay input owner"
    );
    assert_eq!(
        count::<ambition_platformer2d::platformer::markers::PlayerEntity>(app),
        0,
        "{context}: no player entity",
    );
    assert_eq!(
        count::<ambition_platformer2d::render::hud::PlayerHudRoot>(app),
        0,
        "{context}: no gameplay HUD root",
    );
    assert_eq!(
        count::<ambition_platformer2d::render::dialog_ui::DialogOverlayRoot>(app),
        0,
        "{context}: no gameplay dialog root",
    );
    assert_eq!(
        count::<ambition_platformer2d::menu::map::MapMenuRoot>(app),
        0,
        "{context}: no gameplay map root",
    );
    assert_eq!(
        count::<ambition_platformer2d::render::rendering::moving_platforms::MovingPlatformVisual>(
            app,
        ),
        0,
        "{context}: no moving-platform presentation",
    );
    assert!(
        ambition_platformer2d::platformer::lifecycle::session_world_entity(app.world()).is_none(),
        "{context}: no canonical gameplay-world root exists",
    );
    assert!(
        app.world().resource::<PreparedSessionRegistry>().is_empty(),
        "{context}: no prepared-session identity survives",
    );
    assert!(
        app.world().resource::<LoadCoordinator>().is_empty(),
        "{context}: no provider load transaction survives",
    );
    assert!(
        app.world()
            .resource::<LoadForegroundState>()
            .active
            .is_none(),
        "{context}: no loading foreground survives",
    );
    assert!(
        app.world().resource::<LoadActivityState>().active.is_none(),
        "{context}: no loading activity survives",
    );
    let policy = app.world().resource::<PresentationOwnershipPolicy>();
    assert_eq!(
        policy.class("map"),
        Some(PresentationOwnershipClass::GameplaySession),
    );
    assert_eq!(
        policy.class("kaleidoscope"),
        Some(PresentationOwnershipClass::Frontend),
    );
    assert_eq!(
        policy.class("developer_overlays"),
        Some(PresentationOwnershipClass::Frontend),
    );
    assert_eq!(
        policy.class("debug_presentation"),
        Some(PresentationOwnershipClass::Frontend),
    );
    let frontend = app
        .world()
        .resource::<ActiveFrontendAuthority>()
        .0
        .as_ref()
        .expect("title owns exact frontend authority");
    assert_eq!(
        frontend.route_id.as_str(),
        shell_host::AMBITION_LAUNCHER_ROUTE
    );
    let selection = app
        .world()
        .resource::<ambition_platformer2d::audio::selection::ActiveAudioSelection>();
    assert!(
        matches!(
            selection.owner(),
            Some(ambition_platformer2d::sfx::AudioContextOwner::Frontend(_))
        ),
        "{context}: title owns a frontend audio context",
    );
    assert_eq!(
        count::<RoomVisual>(app),
        0,
        "{context}: zero room visuals under the title"
    );
    assert_eq!(count::<HudText>(app), 0, "{context}: zero gameplay HUD");
    assert_eq!(
        count::<SessionScopedEntity>(app),
        0,
        "{context}: zero session-owned entities at the title"
    );
}

#[test]
fn rendered_ownership_across_the_title_and_two_games() {
    let mut app = rendered_app();
    assert_recording_audio_output(&app);
    settle(&mut app);
    assert_title_ownership(&mut app, "boot title");

    // ── Ambition ───────────────────────────────────────────────────────
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle(&mut app);
    assert_eq!(
        active_route(&app),
        Some(shell_host::AMBITION_GAMEPLAY_ROUTE.to_owned()),
        "ambition session active"
    );
    assert!(
        count::<RoomVisual>(&mut app) > 0,
        "ambition: the LDtk room draws"
    );
    assert_eq!(count::<HudText>(&mut app), 1, "ambition: the HUD exists");
    assert_eq!(
        main_cameras(&mut app),
        1,
        "ambition: still exactly one host main camera"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_title_ownership(&mut app, "title after ambition");

    // ── Sanic, through the SAME generic session visuals ────────────────
    app.world_mut()
        .write_message(ShellCommand::GoTo("sanic_gameplay".into()));
    settle(&mut app);
    assert_eq!(
        active_route(&app),
        Some("sanic_gameplay".to_owned()),
        "sanic session active"
    );
    assert!(
        count::<RoomVisual>(&mut app) > 0,
        "sanic: the speedway draws through the provider-agnostic session visuals"
    );
    assert_eq!(
        count::<HudText>(&mut app),
        0,
        "sanic: Ambition's HUD does not leak into another provider's session"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_title_ownership(&mut app, "title after sanic");

    // ── Mary-O, through the SAME generic session visuals ───────────────
    app.world_mut()
        .write_message(ShellCommand::GoTo("mary_o_gameplay".into()));
    settle(&mut app);
    assert_eq!(
        active_route(&app),
        Some("mary_o_gameplay".to_owned()),
        "mary-o session active"
    );
    assert!(
        count::<RoomVisual>(&mut app) > 0,
        "mary-o: the 1-1 room draws through the provider-agnostic session visuals"
    );
    assert_eq!(
        main_cameras(&mut app),
        1,
        "mary-o: still exactly one host main camera"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_title_ownership(&mut app, "title after mary-o");

    // The launcher still works after the whole cycle: relaunch Ambition
    // through the real launcher command path.
    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    settle(&mut app);
    assert_eq!(
        active_route(&app),
        Some(shell_host::AMBITION_GAMEPLAY_ROUTE.to_owned()),
        "relaunch through the launcher lands in Ambition"
    );
    assert!(count::<RoomVisual>(&mut app) > 0, "relaunch draws again");
}

/// Provider-relative music at the PLAYBACK layer (Issues 1–3).
///
/// Drives the real visible composition — which runs the actual music director,
/// `MusicIntent`, and `MusicPlaybackState` — and asserts what the base channel
/// actually plays at each stop:
///
/// - title plays the host's configured frontend theme (whatever the host named);
/// - Ambition gameplay plays an Ambition-authored gameplay track (not the theme);
/// - Quit to Home restores the frontend theme;
/// - Sanic gameplay plays Sanic's own track — never Ambition's residue;
/// - Mary-O gameplay is DELIBERATELY silent (a music-less provider stops
///   playback rather than retaining the previous track).
#[test]
fn provider_relative_music_drives_the_base_channel() {
    let mut app = rendered_app();
    assert_recording_audio_output(&app);
    settle(&mut app);
    // The host's configured theme, read from the host rather than hardcoded:
    // which song plays is content, and pinning the name here would mean editing
    // this test every time the title music changes. What is worth asserting is
    // that the title plays THAT track, that gameplay replaces it, and that Quit
    // to Home restores it.
    let title = app
        .world()
        .resource::<ambition_platformer2d::audio::selection::FrontendAudioRegistry>()
        .host_default()
        .and_then(|profile| profile.title_track())
        .expect("the shell host configures a title theme")
        .to_owned();
    assert_eq!(
        active_music_track(&app),
        title,
        "the title plays the host's configured frontend theme"
    );

    // Ambition: a gameplay track takes over from the title theme.
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle(&mut app);
    let ambition_track = active_music_track(&app);
    assert!(
        !ambition_track.is_empty() && ambition_track != title,
        "ambition gameplay plays an authored gameplay track, not the title theme \
         (got {ambition_track:?})"
    );

    // Quit to Home restores the frontend theme.
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(
        active_music_track(&app),
        title,
        "Quit to Home restores the frontend policy (the title theme resumes)"
    );

    // Sanic plays ITS track — the Ambition track that just played is still
    // resident in the combined library, but this provider does not authorize it.
    app.world_mut()
        .write_message(ShellCommand::GoTo("sanic_gameplay".into()));
    settle(&mut app);
    assert_eq!(
        active_music_track(&app),
        "you_are_too_slow",
        "Sanic plays its own authored track, never Ambition's"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(active_music_track(&app), title);

    // Mary-O authors its own "Support Theme": provider-relative audio switches
    // to Mary-O's track, never Sanic's you_are_too_slow or the retained title.
    app.world_mut()
        .write_message(ShellCommand::GoTo("mary_o_gameplay".into()));
    settle(&mut app);
    assert_eq!(
        active_music_track(&app),
        "support_theme",
        "Mary-O plays its own authored theme, never the previous provider's track"
    );
}

/// A provider's own frontend screen plays the score written for it.
///
/// unacceptable. … This current design is not elegant if games cant share
/// assets."*
///
/// Smash's character select has a score of its own
/// (`super_smash_siblings_character_select`). It played in the standalone smash
/// app and could not play here, because frontend audio was ONE process-global
/// resource and the last composition to install it won — so in a host composing
/// seven providers, six of them had no way to say what their own screens sound
/// like.
///
/// the assertion is the PLAYBACK, not the declaration. Reading a profile
/// back out of a registry passes on a singleton too; what distinguishes the two
/// designs is which song reaches the base channel on a route the host does not
/// own.
///
/// The launcher's own theme is asserted on both sides of the visit: a per-route
/// answer that clobbered the host's answer would be the same defect pointing the
/// other way.
#[test]
fn a_providers_own_frontend_route_plays_the_score_written_for_it() {
    let mut app = rendered_app();
    assert_recording_audio_output(&app);
    settle(&mut app);

    let title = active_music_track(&app);
    assert!(
        !title.is_empty(),
        "vacuity: the launcher must be playing the host's theme for this to mean anything",
    );
    assert_ne!(
        title,
        ambition_demo_smash::SMASH_SELECT_TRACK,
        "vacuity: the host's theme and smash's select score must be different songs",
    );

    // Smash's character select is a frontend route of the PROVIDER's, reached
    // inside the multi-game host — the exact composition where the singleton
    // could not express an answer.
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_demo_smash::SMASH_SELECT_ROUTE.into(),
    ));
    settle(&mut app);
    assert_eq!(
        active_route(&app),
        Some(ambition_demo_smash::SMASH_SELECT_ROUTE.to_owned()),
        "the host reached smash's select route",
    );
    assert_eq!(
        active_music_track(&app),
        ambition_demo_smash::SMASH_SELECT_TRACK,
        "smash's select screen plays the score written for it, in the Ambition host",
    );

    // ...and the host's own frontend routes still play the host's own theme.
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_LAUNCHER_ROUTE.into(),
    ));
    settle(&mut app);
    assert_eq!(
        active_music_track(&app),
        title,
        "returning to the host's own screen restores the host's own theme",
    );
}

fn play_owned_sfx(
    app: &mut App,
    request: ambition_platformer2d::sfx::SfxMessage,
) -> Option<ambition_platformer2d::audio::render::SfxPlaybackRecord> {
    let source = app
        .world()
        .resource::<ambition_platformer2d::audio::selection::ActiveAudioSelection>()
        .primary_sfx_source()
        .cloned()
        .expect("active audio context publishes a primary SFX source");
    play_owned_sfx_from(app, source, request)
}

/// How long [`play_owned_sfx_from`] waits for a request to be played.
///
/// Only the "nothing should play" arm ever runs this out; a real playback lands
/// on the first update.
const PLAYBACK_SETTLE_FRAMES: usize = 120;

fn play_owned_sfx_from(
    app: &mut App,
    source: ambition_platformer2d::sfx::PresentationSourceId,
    request: ambition_platformer2d::sfx::SfxMessage,
) -> Option<ambition_platformer2d::audio::render::SfxPlaybackRecord> {
    let owner = app
        .world()
        .resource::<ambition_platformer2d::audio::selection::ActiveAudioSelection>()
        .owner();
    app.world_mut()
        .write_message(ambition_platformer2d::sfx::OwnedSfxMessage {
            owner,
            source,
            request,
        });
    // ⛔⛔ READ THE RECORD THAT CHANGED, NOT THE LATCH AFTER A FIXED COUNT.
    // `last_played` is a LATCH, and this used to run exactly two updates and
    // then read it. Traced per update 2026-08-31: a request's record appears on
    // the FIRST update after the message and IS NOT STILL THERE ON THE SECOND —
    // so the old read was one frame too late and returned an OLDER record. It
    // did not fail; it answered with the previous cue, and the caller asserted
    // against that. A crossover-audio test therefore reported
    // `presentation_source == "ambition"`: not a routing bug, not an
    // authorization bug, a stale latch read one frame late.
    //
    // ⛔ AND THE FRAME BUDGET IS NOT WHAT FIXES IT — that was checked, because a
    // green test plus a plausible story is how a wrong explanation gets written
    // down permanently. Poisoning the bound to TWO still passes. The operative
    // change is comparing against the record this call came in with.
    //
    // ⚠ Two updates was enough for the schedule as it stood, which is why this
    // only surfaced when the campaign added systems to `Update`; eight EMPTY
    // ones reproduce it. Every catalog, bank and audio owner the caller reads is
    // already correct at frame ZERO and never moves — traced. The COUNT was the
    // only variable, which is exactly what a fixed frame budget hides.
    let before = app
        .world()
        .resource::<ambition_platformer2d::audio::render::SfxPlaybackState>()
        .last_played
        .clone();
    for _ in 0..PLAYBACK_SETTLE_FRAMES {
        app.update();
        let now = app
            .world()
            .resource::<ambition_platformer2d::audio::render::SfxPlaybackState>()
            .last_played
            .clone();
        if now != before {
            return now;
        }
    }
    // ⭐ THE BOUND EXISTS FOR THE OTHER ARM. This test also asserts that STALE
    // work is REJECTED — a request that must never play. That arm is the one
    // that runs the loop out, and a generous bound makes "nothing played" a
    // stronger claim than the old two frames did.
    //
    // Deliberately the pre-existing record rather than a panic: a caller that
    // EXPECTS no playback reads it unchanged, and this helper must not decide
    // that for them.
    before
}

/// Frontend and gameplay contexts share one exact ownership mechanism while
/// resolving their actual provider-authored source definitions.
#[test]
fn provider_relative_sfx_resolves_the_real_source_and_rejects_stale_work() {
    use ambition_platformer2d::audio::render::SfxSourceKind;
    use ambition_platformer2d::sfx::{ids, AudioContextOwner, OwnedSfxMessage, SfxMessage};

    let mut app = rendered_app();
    assert_recording_audio_output(&app);
    settle(&mut app);

    let menu = play_owned_sfx(
        &mut app,
        SfxMessage::Play {
            id: ids::UI_MENU_MOVE,
            pos: Vec2::ZERO,
        },
    )
    .expect("the title owns and resolves its menu-move SFX");
    assert_eq!(
        menu.provider_id,
        ambition_content::AMBITION_CONTENT_PROVIDER
    );
    assert!(matches!(menu.owner, AudioContextOwner::Frontend(_)));
    assert_eq!(menu.id, ids::UI_MENU_MOVE);

    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    // ⛔ SIX FRAMES WAS A GUESS THAT HELD UNTIL THE SCHEDULE MOVED. See
    // `play_owned_sfx_from`: the fragility was never here, it was in assuming a
    // request is played within a fixed number of updates.
    settle(&mut app);
    let ambition_dash = play_owned_sfx(&mut app, SfxMessage::Dash { pos: Vec2::ZERO })
        .expect("Ambition resolves its Dash source");
    assert_eq!(
        ambition_dash.provider_id,
        ambition_content::AMBITION_CONTENT_PROVIDER
    );
    assert!(matches!(
        ambition_dash.owner,
        AudioContextOwner::Gameplay(_)
    ));

    // One crossover session owns the speakers while each authored package
    // keeps its own cue namespace and source definitions. Authorize Sanic as a
    // secondary presentation source without changing Ambition's primary music
    // or audio provider, then request the SAME logical Dash id from Sanic.
    let sanic_sfx = app
        .world()
        .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>()
        .sfx_for("sanic")
        .cloned();
    let sanic_bank_ids = app
        .world()
        .resource::<ambition_platformer2d::audio::catalog::SfxBankRegistry>()
        .ids_for("sanic");
    app.world_mut()
        .resource_mut::<ambition_platformer2d::audio::selection::ActiveAudioSelection>()
        .authorize_sfx_source("sanic.cast", "sanic", sanic_sfx, sanic_bank_ids);
    let crossover_sanic_dash = play_owned_sfx_from(
        &mut app,
        "sanic.cast".into(),
        SfxMessage::Dash { pos: Vec2::ZERO },
    )
    .expect("an authorized secondary source resolves inside the same session");
    assert_eq!(
        crossover_sanic_dash.presentation_source.as_str(),
        "sanic.cast"
    );
    assert_eq!(crossover_sanic_dash.provider_id, "sanic");
    assert_eq!(crossover_sanic_dash.source.kind, SfxSourceKind::Procedural);
    assert_ne!(
        crossover_sanic_dash.source.fingerprint, ambition_dash.source.fingerprint,
        "source identity, not the active primary provider, selects the authored Dash"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::audio::selection::ActiveAudioSelection>()
            .provider_id(),
        Some(ambition_content::AMBITION_CONTENT_PROVIDER),
        "secondary SFX authorization must not replace the session's primary provider"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert!(
        app.world()
            .resource::<ambition_platformer2d::audio::render::SfxPlaybackState>()
            .last_played
            .is_none(),
        "returning home clears gameplay SFX playback ownership"
    );

    app.world_mut()
        .write_message(ShellCommand::GoTo("sanic_gameplay".into()));
    settle(&mut app);
    let first_sanic_owner = app
        .world()
        .resource::<ambition_platformer2d::audio::selection::ActiveAudioSelection>()
        .owner()
        .expect("Sanic owns audio");
    let sanic_dash = play_owned_sfx(&mut app, SfxMessage::Dash { pos: Vec2::ZERO })
        .expect("Sanic resolves its authored procedural Dash");
    assert_eq!(sanic_dash.provider_id, "sanic");
    assert_eq!(sanic_dash.source.kind, SfxSourceKind::Procedural);
    assert_ne!(
        sanic_dash.source.fingerprint, ambition_dash.source.fingerprint,
        "the same logical Dash id resolves from the active provider's actual definition"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo("mary_o_gameplay".into()));
    settle(&mut app);
    let rejected_before = app
        .world()
        .resource::<ambition_platformer2d::audio::render::SfxPlaybackState>()
        .rejected_unauthorized;
    assert!(
        play_owned_sfx(&mut app, SfxMessage::Dash { pos: Vec2::ZERO }).is_none(),
        "Mary-O's explicit empty fragment means deliberate SFX silence"
    );
    assert!(
        app.world()
            .resource::<ambition_platformer2d::audio::render::SfxPlaybackState>()
            .rejected_unauthorized
            > rejected_before
    );

    // Same-provider relaunch poison: a queued request carrying Sanic A's exact
    // owner must not play during a fresh Sanic B session.
    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    app.world_mut()
        .write_message(ShellCommand::GoTo("sanic_gameplay".into()));
    settle(&mut app);
    let current_owner = app
        .world()
        .resource::<ambition_platformer2d::audio::selection::ActiveAudioSelection>()
        .owner()
        .expect("fresh Sanic session owns audio");
    assert_ne!(first_sanic_owner, current_owner);
    let rejected_before = app
        .world()
        .resource::<ambition_platformer2d::audio::render::SfxPlaybackState>()
        .rejected_wrong_owner;
    app.world_mut().write_message(OwnedSfxMessage {
        owner: Some(first_sanic_owner),
        source: "sanic".into(),
        request: SfxMessage::Dash { pos: Vec2::ZERO },
    });
    app.update();
    app.update();
    let playback = app
        .world()
        .resource::<ambition_platformer2d::audio::render::SfxPlaybackState>();
    // `audio_play_sfx_messages` routes each message down EXACTLY ONE branch:
    // wrong-owner rejection and acceptance are mutually exclusive `continue`
    // arms. So "the stale request was rejected exactly once" is both necessary
    // and sufficient to prove it never reached the real playback path.
    //
    // Deliberately NOT `accepted_playbacks == before`: the fresh Sanic session
    // legitimately emits its OWN cues while it runs, so that assertion fails
    // whenever real sim time elapses in this window — it only ever passed
    // because an unpinned clock let these frames run almost no simulation.
    assert_eq!(
        playback.rejected_wrong_owner,
        rejected_before + 1,
        "the stale Sanic-A request must take the wrong-owner rejection path, \
         which is the same thing as never reaching playback",
    );
}

/// What a stranger reads on the first screen.
///
/// The real composed launcher must expose the intended game-selection wording and relative
/// text/control sizing; a page-model unit test cannot verify the rendered composition.
///
/// This asserts the words and the RELATIVE sizes rather than exact pixel values.
/// Pinning "the footer is 3.8" would make a design tweak a test failure while
/// still passing if somebody made the rows tiny too; what matters is that the
/// footer is legible next to the thing it sits under.
///
/// and it may never identify a text node by its STRING alone.
/// `"Ambition"` is on this screen TWICE and always was: it is the title of the
/// launcher AND the label of the first game in the roster, which is the game
/// called Ambition. The title is a `MenuNode::Text`, authored as a PERCENTAGE
/// of viewport height and spawned as `FontSize::Vh`; the row label is a
/// control's child (Bevy's `TextFont` default, `FontSize::Px(20.0)` —
/// `spawn_control` sets the font HANDLE and nothing else). A global
/// `find(label == "Ambition")` therefore returns whichever of the two the
/// query's archetype order reaches first, which is not a property of the
/// launcher at all — display text is not identity. THE UNIT IS THE ROLE: a
/// `Vh` size is one of the menu's own typographic nodes, a `Px` one is a
/// control's label.
#[test]
fn the_title_screen_says_choose_game_and_is_readable() {
    let mut app = rendered_app();
    settle(&mut app);

    // Every text under the launcher UI root, tagged with whether it is one of
    // the menu's own typographic nodes or a control's label.
    let launcher = {
        let mut roots = app
            .world_mut()
            .query_filtered::<Entity, With<BasicShellUiRoot>>();
        let mut found = roots.iter(app.world());
        let root = found.next().expect("the launcher UI root is not on screen");
        assert!(
            found.next().is_none(),
            "more than one launcher UI root; this test would be reading an \
             arbitrary one"
        );
        root
    };
    let mut parents = app.world_mut().query::<(Entity, &ChildOf)>();
    let parent_of: std::collections::HashMap<Entity, Entity> =
        parents.iter(app.world()).map(|(e, c)| (e, c.0)).collect();
    let under_launcher = |mut entity: Entity| -> bool {
        for _ in 0..32 {
            match parent_of.get(&entity) {
                Some(parent) if *parent == launcher => return true,
                Some(parent) => entity = *parent,
                None => return false,
            }
        }
        false
    };

    let mut texts = app.world_mut().query::<(Entity, &Text, &TextFont)>();
    let rendered: Vec<(String, FontSize)> = texts
        .iter(app.world())
        .filter(|(entity, ..)| under_launcher(*entity))
        .map(|(_, text, font)| (text.0.clone(), font.font_size))
        .collect();
    assert!(
        !rendered.is_empty(),
        "the launcher rendered no text at all, so nothing below is about the \
         title screen"
    );

    assert!(
        rendered.iter().any(|(label, _)| label == "Choose Game"),
        "the game-select screen still heads itself with a verb: {:?}",
        rendered.iter().map(|(l, _)| l).collect::<Vec<_>>()
    );
    assert!(
        !rendered.iter().any(|(label, _)| label == "Play"),
        "'Play' is still on the select screen; it belongs on the confirm button"
    );

    // ⛔ THE VIEWPORT IS STATED HERE, BY THIS TEST, and it has to be: this
    // harness is `VisibleRenderMode::NoWindow`, which composes no window and no
    // render app at all, so the UI render target measures 0x0 and `Vh` resolves
    // to nothing. 1080 is the height the launcher's sizes were eyeballed at, so
    // it is the number that makes the assertions below mean what their authors
    // intended. It is a TEST's reference frame, not the engine's: a real build
    // resolves against the live UI target.
    const REFERENCE_VIEWPORT: bevy::math::Vec2 = bevy::math::Vec2::new(1920.0, 1080.0);
    let rem = app.world().resource::<bevy::text::RemSize>().0;

    // Assert the LAUNCHER's own text roles by role, not a global min/max over every text node.
    //
    // `typography_sized` selects among the launcher's `MenuNode::Text` nodes
    // only — never a control's label — and REQUIRES the match to be unique.
    let typography_sized = |matches: &dyn Fn(&str) -> bool, wanted: &str| -> f32 {
        let hits: Vec<FontSize> = rendered
            .iter()
            // "NOT Px", not "is Vh": a control's label is Bevy's `TextFont`
            // default, `FontSize::Px(20.0)`, and the menu's own typographic
            // nodes are the ones authored in a viewport unit. Asking for `Vh`
            // by name would let a wrong axis leave the population silently
            // rather than fail on its resolved size.
            .filter(|(label, size)| !matches!(size, FontSize::Px(_)) && matches(label))
            .map(|(_, size)| *size)
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "expected exactly one launcher text node for {wanted}, found \
             {}: {rendered:?}",
            hits.len()
        );
        // Resolved through the engine's own `eval` — the same call Bevy's text
        // pipeline makes — so what this asserts is the number of pixels a player
        // on a 1080-tall display sees, not the authored percentage.
        hits[0].eval(REFERENCE_VIEWPORT, rem)
    };

    // The title. It rendered at FIVE PIXELS, and the cause was not the launcher:
    // `MenuNode::Text`'s size had no documented unit and the two renderer
    // backends read it as two different things. The kaleidoscope passed it to
    // Lunex's `Rh` (percent of height); `bevy_ui` assigned it to
    // `TextFont::font_size` (pixels). Every call site in the tree was authored
    // as a percentage, so every heading the flat renderer drew was two to five
    // pixels tall. `FontSize::Vh` is that percentage, spelled in the engine's
    // own vocabulary, so there is no longer a conversion to get wrong.
    let title = typography_sized(&|label| label == "Ambition", "the title");
    assert!(
        title >= 32.0,
        "the title renders at {title:.1}px — this is the units bug, not a taste \
         question"
    );

    let footer = typography_sized(&|label| label.contains("Enter launches"), "the footer");
    assert!(
        footer >= 14.0,
        "the footer renders at {footer:.1}px, which is small print in the sense \
         of being unreadable rather than in the sense of being a footer"
    );

    // And it stays SUBORDINATE.
    assert!(
        footer < title,
        "the footer ({footer:.1}px) is no smaller than the title ({title:.1}px)"
    );
}

/// The title screen must not advertise gameplay verbs while a menu owns input.
/// Assert authored `Visibility`, not render-computed `ViewVisibility`, because
/// this headless composition does not run the render visibility pass.
#[test]
fn the_title_screen_does_not_show_gameplay_touch_buttons() {
    use ambition_platformer2d::touch_input::layout::TouchActionButton;

    let mut app = rendered_app();
    settle(&mut app);

    let mut buttons = app.world_mut().query::<(&TouchActionButton, &Visibility)>();
    let all: Vec<(TouchActionButton, Visibility)> = buttons
        .iter(app.world())
        .map(|(action, visibility)| (*action, *visibility))
        .collect();
    assert!(
        !all.is_empty(),
        "no touch buttons exist at all, so this proves nothing about which ones \
         are shown"
    );

    // Start and Reset are deliberately exempt: shell-shaped verbs, and a phone
    // with no keyboard needs its way out of a game.
    //
    // Jump and Interact are exempt too now, and that is a CHANGE
    // rather than a loosening. This screen publishes
    // `ControlPrompt { context: Menu, menu_confirm: Some("Play") }` — measured —
    // and `touch_action_available`'s Menu branch has always admitted exactly the
    // menu-confirm buttons plus the menu row. So on the game-select screen those
    // two ARE the Play button: they wear its label and pressing one picks the
    // game. The test's original premise, "gameplay verbs that do nothing there",
    // was true of Attack and Dash and never of these.
    //
    // The invariant kept here is the one that was always meant: nothing is advertised that does
    // nothing.
    let menu_functional = |action: &TouchActionButton| {
        matches!(
            action,
            TouchActionButton::Start
                | TouchActionButton::Reset
                | TouchActionButton::Jump
                | TouchActionButton::Interact
        )
    };
    let advertised: Vec<TouchActionButton> = all
        .iter()
        .filter(|(action, _)| !menu_functional(action))
        .filter(|(_, visibility)| *visibility != Visibility::Hidden)
        .map(|(action, _)| *action)
        .collect();

    // And the other half, which the original could not state: the screen must OFFER a confirm.
    let confirms: Vec<TouchActionButton> = all
        .iter()
        .filter(|(action, _)| {
            matches!(
                action,
                TouchActionButton::Jump | TouchActionButton::Interact
            )
        })
        .filter(|(_, visibility)| *visibility != Visibility::Hidden)
        .map(|(action, _)| *action)
        .collect();
    assert!(
        !confirms.is_empty(),
        "the game-select screen shows no confirm control at all, so a phone \
         player has no on-screen way to pick a game — which is the report this \
         behaviour came from"
    );

    assert!(
        advertised.is_empty(),
        "the game-select screen advertises gameplay verbs that do nothing \
         there: {advertised:?}"
    );
}

/// The title screen's Menu button does something now.
///
/// *"Some form of the 'Menu' probably should be available here, so you can
/// change global engine properties like audio mute. Currently the touch menu
/// icon does nothing."* It did nothing because the shell's pause menu returned
/// early with no live session, so the Start intent — keyboard Escape, controller
/// Start, and the touch HUD's "Menu" button, which all fold to the same
/// semantic edge — was decoration on the one screen where "how do I mute this"
/// gets asked.
///
/// Driven through the REAL composed host with REAL key presses. The shell's own unit tests can
/// write it because they run without that producer; an app-level test that did the same would be
/// testing a value nothing reads.
///
/// The other half of what this proves: the shipped host actually HAS a `UserSettings` for the
/// menu to edit.
#[test]
fn the_title_screen_menu_opens_and_mutes_the_game() {
    use ambition_platformer2d::persistence::settings::UserSettings;

    let mut app = rendered_app();
    settle(&mut app);

    /// One frame with `key` down, then one with it up. Both halves matter: these
    /// are just-pressed edges, so a key held across updates fires once and a key
    /// never released fires nothing again.
    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(key);
        app.update();
    }

    assert!(
        app.world().get_resource::<UserSettings>().is_some(),
        "the shipped host has no `UserSettings`, so the menu's audio rows would \
         write nowhere — they would look live and change nothing"
    );

    tap(&mut app, KeyCode::Escape);
    // ⛔⛔ THE ROAD CHANGED, THE CAPABILITY DID NOT — and this assertion is why
    // the change was safe to make. Escape used to open the shell PAUSE menu OVER
    // the launcher: two live menus, settings unusable, the winner apparently
    // random (Jon, 2026-09-05). The pause menu yields to the launcher now.
    //
    // ⚠ That fix ALSO removed the only route to "how do I mute this" from the
    // one screen that asks it, and this test caught exactly that. So Start now
    // reaches the launcher's own SETTINGS TAB instead — same gesture, same
    // capability, one menu owning input.
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::game_shell::ShellLauncherState>()
            .tab,
        ambition_platformer2d::game_shell::LauncherTab::Settings,
        "Escape on the title screen did not reach the settings tab"
    );
    assert!(
        !app.world()
            .resource::<ambition_platformer2d::game_shell::ShellPauseMenu>()
            .open,
        "the shell pause menu opened OVER the launcher again — the stacking bug"
    );

    // Isolate `UserSettings`: startup loads and saves the real settings file, so
    // this test must not mutate developer configuration outside the process.
    //
    // So: snapshot everything, assert a FLIP rather than a value, and put the
    // whole resource back. Leaving somebody's game muted because a test ran is
    // not a trade this test gets to make.
    let snapshot = app.world().resource::<UserSettings>().clone();
    let muted = |app: &App| app.world().resource::<UserSettings>().audio.muted;
    let before = muted(&app);

    // Walk the rows until one FLIPS mute, by the SETTING changing rather than a
    // hardcoded row index: the row order is the shell's business and muting is
    // this test's.
    //
    // ⚠ RIGHT, not Enter. On the settings tab confirm does nothing on purpose —
    // routing it through `LaunchSelected` would start a game from the settings
    // screen — so left/right adjust the focused control and up/down move.
    let mut toggled = false;
    for _ in 0..8 {
        tap(&mut app, KeyCode::ArrowRight);
        if muted(&app) != before {
            toggled = true;
            break;
        }
        tap(&mut app, KeyCode::ArrowDown);
    }

    *app.world_mut().resource_mut::<UserSettings>() = snapshot;
    app.update();

    assert!(
        toggled,
        "walked the whole title-screen menu without finding a row that mutes"
    );
}

/// The FPS counter is drawn IN FRONT OF the launcher, not merely present.
///
/// ⛔⛔ "AN FPS ENTITY EXISTS" IS NOT THE PROPERTY. The counter's whole job is to
/// be readable while something else is on screen, and the launcher is an opaque
/// full-screen UI: a counter that exists behind it is a counter nobody can read.
/// The old hand-rolled overlay had no `GlobalZIndex` at all and got away with it
/// by accident of spawn order.
///
/// ⭐ IT ASKS BEVY'S OWN ANSWER. `UiStack.uinodes` is the computed back-to-front
/// draw order — the list the UI renderer walks — so "last entry" IS "frontmost",
/// not a proxy for it. Asserting a z-index number instead would re-derive bevy's
/// sorting rule in the test and agree with itself.
#[test]
fn the_fps_counter_draws_in_front_of_the_launcher() {
    use bevy::dev_tools::fps_overlay::FPS_OVERLAY_ZINDEX;
    use bevy::ui::UiStack;

    let mut app = rendered_app();
    settle(&mut app);

    let stack = app.world().resource::<UiStack>().uinodes.clone();

    // ⭐ THE PREMISE, and it is not decoration: an empty or one-node stack would
    // make "the overlay is last" true and meaningless.
    let launcher_nodes = {
        let mut roots = app
            .world_mut()
            .query_filtered::<Entity, With<BasicShellUiRoot>>();
        roots.iter(app.world()).count()
    };
    assert!(
        launcher_nodes > 0,
        "the launcher composed no UI root, so there is nothing for the counter \
         to be in front of"
    );
    assert!(
        stack.len() > 2,
        "the UI stack holds {} node(s); with the launcher on screen this should \
         be the whole composition, and a near-empty stack makes the assertion \
         below vacuous",
        stack.len()
    );

    // The overlay's own root: upstream's markers are private, but the z-index
    // constant is public and is the one thing the root is guaranteed to carry.
    let overlay_root = {
        let mut roots = app.world_mut().query::<(Entity, &GlobalZIndex)>();
        roots
            .iter(app.world())
            .find(|(_, z)| z.0 == FPS_OVERLAY_ZINDEX)
            .map(|(entity, _)| entity)
            .expect("the FPS overlay root is composed in every visible host")
    };

    let mut parents = app.world_mut().query::<(Entity, &ChildOf)>();
    let parent_of: std::collections::HashMap<Entity, Entity> =
        parents.iter(app.world()).map(|(e, c)| (e, c.0)).collect();
    let under_overlay = |mut entity: Entity| -> bool {
        for _ in 0..8 {
            if entity == overlay_root {
                return true;
            }
            match parent_of.get(&entity) {
                Some(parent) => entity = *parent,
                None => return false,
            }
        }
        false
    };

    let frontmost = *stack
        .last()
        .expect("the stack is non-empty, asserted above");
    assert!(
        under_overlay(frontmost),
        "the frontmost UI node is {frontmost:?}, which is not part of the FPS \
         overlay — something in the launcher draws over the counter"
    );
}

/// The FPS counter carries a drop shadow, so it survives a light background.
///
/// ⛔ UPSTREAM CANNOT BE ASKED FOR THIS. `FpsOverlayConfig` exposes `text_color`
/// and `text_config` and nothing else about presentation, and `customize_overlay`
/// rewrites exactly those two whenever the config changes. The shadow is a
/// separate component Ambition attaches, which is precisely why it needs a test:
/// nothing upstream would fail if it stopped being attached, and a counter that
/// is merely hard to read on pale rooms does not announce itself.
#[test]
fn the_fps_counter_has_a_shadow_to_read_against_pale_ground() {
    use bevy::dev_tools::fps_overlay::FPS_OVERLAY_ZINDEX;

    let mut app = rendered_app();
    settle(&mut app);

    let overlay_root = {
        let mut roots = app.world_mut().query::<(Entity, &GlobalZIndex)>();
        roots
            .iter(app.world())
            .find(|(_, z)| z.0 == FPS_OVERLAY_ZINDEX)
            .map(|(entity, _)| entity)
            .expect("the FPS overlay root is composed in every visible host")
    };
    let children: Vec<Entity> = app
        .world()
        .entity(overlay_root)
        .get::<Children>()
        .map(|children| children.iter().collect())
        .unwrap_or_default();

    // ⭐ THE PREMISE: there IS a text child to dress. Without this the assertion
    // below passes on an overlay that spawned nothing.
    let texts: Vec<Entity> = children
        .iter()
        .copied()
        .filter(|child| app.world().entity(*child).contains::<Text>())
        .collect();
    assert!(
        !texts.is_empty(),
        "the FPS overlay root has no text child, so there is nothing to shadow"
    );

    for text in texts {
        let shadow = app
            .world()
            .entity(text)
            .get::<TextShadow>()
            .copied()
            .expect("the counter's text carries a shadow");
        assert!(
            shadow.offset.x > 0.0 && shadow.offset.y > 0.0,
            "a zero offset hides the shadow directly behind the glyphs, which is \
             the one configuration that does nothing"
        );
        assert!(
            shadow.offset.x <= 2.0,
            "the shadow is offset {}px against a {}px glyph; past a couple of \
             pixels it reads as a second blurred counter, not an edge",
            shadow.offset.x,
            12.0,
        );
    }
}


/// ⛔⛔ THE TITLE SCREEN'S TAB STRIP IS WIRED FOR A POINTER — in the SHIPPED host.
///
/// Jon, 2026-09-06: "in the title screen there is no way for me to select the
/// settings menu. I can't click, tap, nothing." `install_bevy_ui_menu_tabs` had
/// exactly ONE caller in the workspace and it was the kaleidoscope menu, so the
/// system that turns a tab press into `MenuTabActivated` was never registered on
/// this screen: the renderer drew real `Button`s that nothing listened to.
///
/// ⚠ WHAT THIS TEST CANNOT DO, stated rather than faked: drive a real click.
/// Bevy's UI focus system RECOMPUTES `Interaction` every frame from live pointer
/// state, so a test that writes `Interaction::Pressed` has it overwritten with
/// `None` before any consumer sees it — measured here, and the reason an earlier
/// version of this test failed while the shipped chain was fine. A headless app
/// has no pointer, so the press edge itself is out of reach.
///
/// ⇒ SO IT ASSERTS THE THREE LINKS THAT ARE REACHABLE, which are exactly the
/// three that were broken or absent:
///   1. the strip is drawn as real `Button`s carrying `BevyUiMenuTab`;
///   2. the tab road is INSTALLED in this composition (`MenuTabActivated`
///      exists — the half that was missing);
///   3. the shell CONSUMES it: the message the renderer publishes moves the
///      strip. Link 3 is where `SelectTab` lives.
#[test]
fn the_shipped_title_screen_is_wired_for_a_pointer() {
    use ambition_platformer2d::game_shell::{LauncherTab, ShellLauncherState};

    let mut app = rendered_app();
    settle(&mut app);

    assert_eq!(
        app.world().resource::<ShellLauncherState>().tab,
        LauncherTab::Home,
        "premise: the title screen starts on the game list"
    );

    // 1. Real buttons, not decoration.
    let tabs: Vec<Entity> = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, (
                With<ambition_platformer2d::menu::render::bevy_ui::BevyUiMenuTab>,
                With<bevy::prelude::Button>,
            )>();
        q.iter(app.world()).collect()
    };
    assert!(
        tabs.len() >= 2,
        "the shipped title screen drew {} tab button(s); a strip with fewer than \
         two has nothing to navigate to",
        tabs.len()
    );

    // 2. The road is installed. This is the link that was missing entirely.
    assert!(
        app.world()
            .get_resource::<bevy::ecs::message::Messages<
                ambition_platformer2d::menu::MenuTabActivated,
            >>()
            .is_some(),
        "the shipped host does not install the tab pointer road, so every press \
         on a tab reaches no system at all"
    );

    // 3. The shell consumes what the renderer publishes.
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<
            ambition_platformer2d::menu::MenuTabActivated,
        >>()
        .write(ambition_platformer2d::menu::MenuTabActivated { index: 1 });
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<ShellLauncherState>().tab,
        LauncherTab::Settings,
        "the renderer's tab activation reached nothing in the shipped host, so \
         the Settings tab stays unreachable by pointer"
    );
}

/// ⛔⛔ THE TAB SWITCHED AND THE SCREEN DID NOT. Jon, three times on 2026-09-06:
/// *"there is no way to select the settings menu, I can't click, tap, nothing"*,
/// then *"Q and E do not change the visible menu"*, then *"I feel like it is a
/// visibility or update problem"* — which was exactly right.
///
/// ⇒ `ShellLauncherState.tab` flipped to `Settings` on EVERY road the whole time.
/// `render_basic_shell` early-returns when `shell_frame_key` is unchanged, and
/// that key held the title and the catalog entries but NOT the tab — so the key
/// was identical either side of a switch and the view kept drawing the page you
/// had left.
///
/// ⚠ AND THE TESTS THAT EXISTED ASSERTED THE STATE, which never broke.
/// `the_shipped_title_screen_is_wired_for_a_pointer` checks
/// `ShellLauncherState.tab == Settings` and passed throughout. This asserts what
/// the player can actually SEE: the tab strip's active flag and the words on the
/// screen.
#[test]
fn switching_the_tab_redraws_the_menu_the_player_sees() {
    use ambition_platformer2d::game_shell::{LauncherTab, ShellLauncherState};
    use ambition_platformer2d::menu::render::bevy_ui::BevyUiMenuTab;

    let mut app = rendered_app();
    settle(&mut app);

    let words = |app: &mut App| -> Vec<String> {
        let mut q = app.world_mut().query::<&bevy::prelude::Text>();
        let mut v: Vec<String> = q.iter(app.world()).map(|t| t.0.clone()).collect();
        v.sort();
        v
    };
    let active_tab = |app: &mut App| -> Option<usize> {
        let mut q = app.world_mut().query::<&BevyUiMenuTab>();
        q.iter(app.world()).find(|t| t.active).map(|t| t.index)
    };

    assert_eq!(
        app.world().resource::<ShellLauncherState>().tab,
        LauncherTab::Home,
        "premise: the title screen opens on the game list"
    );
    let before_words = words(&mut app);
    let before_tab = active_tab(&mut app);

    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<
            ambition_platformer2d::menu::MenuTabActivated,
        >>()
        .write(ambition_platformer2d::menu::MenuTabActivated { index: 1 });
    for _ in 0..8 {
        app.update();
    }

    assert_eq!(
        app.world().resource::<ShellLauncherState>().tab,
        LauncherTab::Settings,
        "premise: the shell consumed the activation"
    );
    assert_ne!(
        active_tab(&mut app),
        before_tab,
        "the tab strip still highlights the tab the player left, so the switch is \
         invisible even though the state changed"
    );
    assert_ne!(
        words(&mut app),
        before_words,
        "the launcher drew the same words after switching to Settings: the view \
         never rebuilt, which is what made the Settings tab unreachable by pointer, \
         by `E`/`Q` and by the bumpers at once"
    );
}

/// ⭐ AND IT MUST NOT REBUILD WHEN NOTHING HAPPENED — Jon, in the same breath as
/// the bug: *"we very likely don't want an architecture that causes menu rebuilds
/// all the time, that seems very inefficient."*
///
/// ⛔ THIS GUARDS THE WRONG FIX, which is the cheap one: dropping the key, or
/// keying on something that changes every frame, would make the test above pass
/// while respawning every node continuously. `shell_frame_key`'s own comment
/// records that `selected` was removed from it for exactly this reason — an arrow
/// press was despawning and respawning the whole launcher.
///
/// ⇒ The rule the key encodes: a field belongs in it when it changes WHICH NODES
/// SHOULD EXIST. A tab does; a cursor does not.
#[test]
fn an_idle_title_screen_does_not_rebuild_its_menu() {
    let mut app = rendered_app();
    settle(&mut app);

    let words = |app: &mut App| -> Vec<String> {
        let mut q = app.world_mut().query::<&bevy::prelude::Text>();
        let mut v: Vec<String> = q.iter(app.world()).map(|t| t.0.clone()).collect();
        v.sort();
        v
    };

    let mut rebuilds = 0;
    let mut last = words(&mut app);
    for _ in 0..60 {
        app.update();
        let now = words(&mut app);
        if now != last {
            rebuilds += 1;
            last = now;
        }
    }
    assert_eq!(
        rebuilds, 0,
        "the idle title screen rebuilt its menu {rebuilds} time(s) in 60 frames; \
         the frame key must name what a REBUILD is for, not what changes per frame"
    );
}

/// ⛔⛔ A ROW INDEX MEANS NOTHING WITHOUT ITS TAB. Jon, 2026-09-06, minutes after
/// the tab strip started redrawing: *"Double clicking master volume launches
/// sanic. Double clicking music volume launches maryo."*
///
/// ⇒ The settings rows carry `BasicLauncherAction(index)` — deliberately, "the
/// same contract the game rows use" — and `ShellLauncherCommand::Activate`
/// resolved that index against the GAME CATALOG unconditionally. Row 0 → game 0,
/// row 1 → game 1: the positional correspondence is the signature.
///
/// ⚠ THE VOLUME ASSERTION IS THE ONE THAT DISCRIMINATES HERE, and the route one
/// is defence rather than coverage — measured by poisoning the fix: reverting the
/// tab check reddens the volume arm, and the route arm stays green because this
/// composition's catalog has no launchable entry at index 1. ⇒ Kept anyway,
/// because the defect Jon hit IS a route and a future fixture with a fuller
/// catalog should not have to rediscover that the assertion belongs here. Saying
/// which arm carries the poison is the honest form.
#[test]
fn activating_a_settings_row_changes_the_setting_and_launches_nothing() {
    use ambition_platformer2d::game_shell::{
        LauncherTab, ShellLauncherCommand, ShellLauncherState,
    };
    use ambition_platformer2d::persistence::settings::UserSettings;

    let mut app = rendered_app();
    settle(&mut app);

    // Onto the settings tab by the road the tab strip uses.
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ShellLauncherCommand>>()
        .write(ShellLauncherCommand::SelectTab(1));
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<ShellLauncherState>().tab,
        LauncherTab::Settings,
        "premise: the launcher is showing the settings rows"
    );

    let route_before = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellRouter>()
        .active
        .as_ref()
        .map(|a| a.activation_id);
    // ⚠ SET A KNOWN MID VALUE FIRST. `nudge_master` clamps to `0.0..=1.0`, so a
    // shipped save already at maximum makes a correct step-up indistinguishable
    // from a dead control — which is exactly how this class of bug hides.
    app.world_mut()
        .resource_mut::<UserSettings>()
        .audio
        .master_volume = 0.5;
    let volume_before = app.world().resource::<UserSettings>().audio.master_volume;

    // ⚠ ROW 1, NOT ROW 0: `ShellAudioControl::ALL` opens with `Mute`, so index 0
    // is the mute toggle and index 1 is Master Volume. (That ordering is also why
    // mute was the one control that appeared to work while the tab was stuck —
    // it is the row a stray confirm lands on first.)
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ShellLauncherCommand>>()
        .write(ShellLauncherCommand::Activate(1));
    for _ in 0..6 {
        app.update();
    }

    let route_after = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellRouter>()
        .active
        .as_ref()
        .map(|a| a.activation_id);
    assert_eq!(
        route_after, route_before,
        "activating a SETTINGS row routed the shell somewhere: the row index was \
         resolved against the game catalog, so Master Volume launched a game"
    );
    assert_ne!(
        app.world().resource::<UserSettings>().audio.master_volume,
        volume_before,
        "activating Master Volume did not change it; confirm on an audio row is \
         the positive direction, the same convention the pause menu states"
    );
}

/// ⛔⛔ A SETTINGS ROW THAT SHOWS NO VALUE IS NOT A SETTINGS ROW. Jon, 2026-09-06:
/// *"video settings and audio settings seem not there or not hooked up."* They
/// were there. They drew "Master Volume" and no number, so there was nothing to
/// watch change — which is most of why the rows read as dead.
///
/// ⇒ TWO defects, one visible symptom. The `bevy_ui` backend DISCARDED the
/// control's `detail` (`detail: _`) while the kaleidoscope backend draws it, so
/// two renderers of one model disagreed about whether a control has a visible
/// value. And `shell_frame_key` did not name the audio values, so even once drawn
/// they would have frozen at whatever they were when the page was last built.
///
/// ⚠ THIS ASSERTS BOTH AT ONCE, deliberately: the percentages must be PRESENT and
/// the one the player adjusted must MOVE. Either fix alone leaves a row that lies.
#[test]
fn a_settings_row_shows_its_value_and_the_value_follows_the_setting() {
    use ambition_platformer2d::game_shell::ShellLauncherCommand;
    use ambition_platformer2d::persistence::settings::UserSettings;

    let mut app = rendered_app();
    settle(&mut app);
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ShellLauncherCommand>>()
        .write(ShellLauncherCommand::SelectTab(1));
    for _ in 0..6 {
        app.update();
    }

    let percents = |app: &mut App| -> Vec<String> {
        let mut q = app.world_mut().query::<&bevy::prelude::Text>();
        let mut v: Vec<String> = q
            .iter(app.world())
            .map(|t| t.0.clone())
            .filter(|s| s.ends_with('%'))
            .collect();
        v.sort();
        v
    };

    // A known mid value: `nudge_master` clamps at 1.0, so a saved maximum would
    // make a correct step-up indistinguishable from a dead control.
    app.world_mut()
        .resource_mut::<UserSettings>()
        .audio
        .master_volume = 0.5;
    for _ in 0..4 {
        app.update();
    }

    let before = percents(&mut app);
    assert!(
        !before.is_empty(),
        "the settings rows drew no value at all: a volume row with no number gives \
         the player nothing to watch change, which is how four live controls read \
         as dead"
    );

    // Row 1 is Master Volume (`ShellAudioControl::ALL` opens with `Mute`).
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<ShellLauncherCommand>>()
        .write(ShellLauncherCommand::Activate(1));
    for _ in 0..6 {
        app.update();
    }

    assert_ne!(
        percents(&mut app),
        before,
        "the setting moved and the drawn percentage did not: the rebuild key does \
         not name the values, so the row keeps showing whatever it was built with"
    );
}
