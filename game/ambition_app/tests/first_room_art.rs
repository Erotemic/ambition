//! The shell route's FIRST room has its art before it activates.
//!
//! On the host (`desktop-timeline-run-20260902T015511Z` / `015909Z` /
//! `020529Z`) the player's own 7.6 MP sheet decoded 0.15 s AFTER every first
//! `room-loaded`, as a 67-79 ms frame with the player drawn as a rectangle
//! until it landed: the route's preparation barrier validated everything and
//! decoded nothing. `prepare-first-room-art` is the work item that holds the
//! barrier — and the load foreground — until the start room's cast and the
//! starting character are decoded.

use bevy::prelude::*;

use ambition_app::app::{VisibleRenderMode, build_visible_app_with};
use ambition_platformer2d::game_shell::ShellCommand;

fn step(app: &mut App) {
    app.update();
    std::thread::sleep(std::time::Duration::from_millis(8));
}

fn session_room_and_worn(app: &mut App) -> Option<(String, String)> {
    let world = app.world_mut();
    let live_definition = ambition_platformer2d::world::rooms::sole_live_room_definition(world)?;
    let mut rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let room = rooms
        .iter(world)
        .next()
        .map(|set| set.spec(live_definition).id.clone())?;
    let mut worn = world.query_filtered::<
        &ambition_platformer2d::characters::actor::CharacterForm,
        With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
    >();
    let worn = worn.iter(world).next()?.0.as_str().to_string();
    Some((room, worn))
}

/// At the frame the session exists, the starting character's sheet is
/// realized AND every page of it is loaded: nothing about the player is left
/// to decode after the reveal. Red with the starting character dropped from
/// the work item's demand (the sheet is then `Loading` at activation).
#[test]
fn the_starting_characters_sheet_is_decoded_before_the_route_activates() {
    use ambition_platformer2d::sprite_sheet::game_assets::GameAssets;

    let mut app = build_visible_app_with(VisibleRenderMode::NoWindow, true, crate::common::in_the_sandbox_hub);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    for _ in 0..30 {
        step(&mut app);
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into(),
    ));

    // The first frame a session world exists is the activation frame; what is
    // true of the art THEN is what the player sees at the reveal.
    let mut activated = None;
    for frame in 0..1200 {
        step(&mut app);
        if let Some(found) = session_room_and_worn(&mut app) {
            activated = Some((frame, found));
            break;
        }
    }
    let (frame, (room, worn)) = activated.expect("the Ambition route never activated");
    assert!(!worn.is_empty(), "the primary avatar wears no character");

    let assets = app.world().resource::<GameAssets>();
    let server = app.world().resource::<AssetServer>();
    let sheet = assets.characters.sheet(&worn).unwrap_or_else(|| {
        panic!(
            "frame {frame}: the route activated in '{room}' with the worn character \
             '{worn}' not realized — its sheet is decoded after the reveal"
        )
    });
    // What the body draws from: its baked pages, or its part pages when it is
    // drawn from parts alone (`CharacterSpriteAsset::presentation_images`).
    let images: Vec<_> = if sheet.parts_only() {
        sheet.presentation_images()
    } else {
        sheet.pages.iter().map(|page| &page.texture).collect()
    };
    let pending: Vec<String> = images
        .into_iter()
        .filter(|texture| !server.is_loaded_with_dependencies(texture.id()))
        .map(|texture| {
            format!(
                "{} ({:?})",
                texture.path().map(|path| path.to_string()).unwrap_or_default(),
                server.get_load_state(texture.id())
            )
        })
        .collect();
    assert!(
        pending.is_empty(),
        "frame {frame}: the route activated in '{room}' with {} page(s) of the worn \
         character '{worn}' still loading: {pending:?} — that decode lands after the \
         reveal, in the open",
        pending.len()
    );
    println!(
        "[first-room-art] activated at frame {frame} in '{room}' with '{worn}' realized \
         and {} page(s) loaded",
        sheet.pages.len()
    );
}

#[cfg(feature = "audio")]
#[test]
fn a_dormant_wolfenstein_has_his_entrance_decoded_before_he_claims_music() {
    use ambition_platformer2d::audio::{catalog::PreparedMusicCatalog, library::KiraAudioSource};
    use ambition_platformer2d::boss_encounter::{BossConfig, BossEncounter, BossEncounterPhase};
    use ambition_platformer2d::platformer::lifecycle::session_world_component;

    let mut app = crate::shell_host_rendered::rendered_app_in_room(
        VisibleRenderMode::NoWindow,
        Some("castle_godwin_throne"),
    );
    for _ in 0..30 {
        step(&mut app);
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    let mut decoded = false;
    for _ in 0..1200 {
        step(&mut app);
        let sleeping = app
            .world_mut()
            .query::<(&BossConfig, &BossEncounter)>()
            .iter(app.world())
            .find(|(config, _)| config.behavior.id == "wolfenstein_boss")
            .map(|(_, state)| state.encounter_phase());
        let Some(phase) = sleeping else { continue };
        assert_eq!(phase, BossEncounterPhase::Dormant);
        let Some(music) = session_world_component::<PreparedMusicCatalog>(app.world()) else {
            continue;
        };
        let cues = music
            .cues
            .as_ref()
            .expect("the session has its prepared cues");
        let binding = cues.binding_for_boss("wolfenstein_boss").unwrap();
        let cue = cues.cue(&binding.cue_id).unwrap();
        let server = app.world().resource::<AssetServer>();
        let assets = app.world().resource::<Assets<KiraAudioSource>>();
        decoded = ["Intro", "Phase1"].into_iter().all(|phase| {
            let state = cue.state(binding.state_in(phase).unwrap()).unwrap();
            let section = cue.section(&state.section_id).unwrap();
            !section.sources.is_empty()
                && section.sources.iter().all(|source| {
                    server
                        .get_handle::<KiraAudioSource>(source.path.clone())
                        .is_some_and(|handle| assets.contains(handle.id()))
                })
        });
        if decoded {
            break;
        }
    }
    assert!(
        decoded,
        "the dormant boss's entrance and immediate continuation did not decode"
    );
    let sources =
        session_world_component::<ambition_platformer2d::encounter::EncounterMusicRequest>(
            app.world(),
        )
        .unwrap();
    assert!(!sources.candidates().any(|source| source.source.kind()
        == ambition_platformer2d::boss_encounter::BOSS_MUSIC_SOURCE_KIND));
}
