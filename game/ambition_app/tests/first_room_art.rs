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

use ambition_app::app::{VisibleRenderMode, build_visible_app, shell_host};
use ambition_platformer2d::game_shell::ShellCommand;

fn step(app: &mut App) {
    app.update();
    std::thread::sleep(std::time::Duration::from_millis(8));
}

fn session_room_and_worn(app: &mut App) -> Option<(String, String)> {
    let world = app.world_mut();
    let mut rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let room = rooms
        .iter(world)
        .next()
        .map(|set| set.active_spec().id.clone())?;
    let mut worn = world.query_filtered::<
        &ambition_platformer2d::characters::actor::WornCharacter,
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

    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    for _ in 0..30 {
        step(&mut app);
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
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
    let pending: Vec<String> = sheet
        .pages
        .iter()
        .filter(|page| !server.is_loaded_with_dependencies(page.texture.id()))
        .map(|page| {
            format!(
                "{} ({:?})",
                page.texture
                    .path()
                    .map(|path| path.to_string())
                    .unwrap_or_default(),
                server.get_load_state(page.texture.id())
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
