//! A theme the player walks away from leaves `Assets<Image>`, not just the set.
//!
//! ⛔⛔ WHY THIS IS AN APP TEST AND NOT A CRATE TEST. The crate-level guards in
//! `ambition_sprite_sheet` prove `retain_themes` drops handles. Dropping a handle
//! is NECESSARY AND NOT SUFFICIENT: Bevy frees an image when its LAST handle
//! drops, so anything else holding a clone — a spawned `ParallaxLayerVisual`, a
//! manifest, a prefetch — keeps the pixels resident no matter what the set says.
//! Only a real composition can answer whether the memory actually went.
//!
//! ⛔ AND THE OBVIOUS ROUTE MEASURES NOTHING. The hall door is the walk every
//! other asset test uses, and it crosses NO theme boundary: `central_hub_main`
//! is `biome: hub` -> `Hub`, and `hall_of_characters` is `biome: hall`, which is
//! not a `ParallaxTheme::from_key` key, so it falls through to
//! `visual_theme: default` -> `Hub` as well. A retire assertion on that walk
//! would pass while doing nothing. This test uses `tech_bros_door` instead,
//! measured from `sandbox.ldtk`:
//!
//! ```text
//! central_hub_main    Hub       + 21 neighbours -> KEEP 6 themes (24 layers)
//! tech_bros_basement  Basement  + 2 neighbours  -> KEEP 2 themes (8 layers)
//!                                                 => 4 themes / 16 layers retire
//! ```
//!
//! ⛔⛔ `#[ignore]`, AND RUN ALONE BY A SCRIPT, for the reason
//! `hall_redecode_census.rs` states: `ambition_app` has ONE `[[test]]` target, so
//! every file under `tests/` is a module of `app_it` sharing a process, and cargo
//! runs them as parallel threads. A sibling booting its own app would populate
//! `Assets<Image>` underneath this one's assertions. Driven by
//! `scripts/measure_parallax_retire.sh` with an exact filter.

use bevy::prelude::*;

use ambition_app::app::{build_visible_app, shell_host, VisibleRenderMode};
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::sprite_sheet::game_assets::{
    GameAssets, ParallaxLayerAsset, ParallaxTheme,
};

const DEPARTING_DOOR: &str = "tech_bros_door";

fn step(app: &mut App) {
    app.update();
    std::thread::sleep(std::time::Duration::from_millis(8));
}

fn settle(app: &mut App, frames: usize) {
    for _ in 0..frames {
        step(app);
    }
}

fn resident_themes(app: &App) -> Vec<ParallaxTheme> {
    app.world()
        .resource::<GameAssets>()
        .parallax_layers
        .resident_themes()
}

/// Every `AssetId` the given themes' layers currently hold, so the test can ask
/// `Assets<Image>` about them after the handles are gone.
fn layer_asset_ids(app: &App, themes: &[ParallaxTheme]) -> Vec<(ParallaxTheme, AssetId<Image>)> {
    let assets = app.world().resource::<GameAssets>();
    let mut ids = Vec::new();
    for &theme in themes {
        for &layer in ParallaxLayerAsset::ALL {
            if let Some(handle) = assets.parallax_layers.get(theme, layer) {
                ids.push((theme, handle.id()));
            }
        }
    }
    ids
}

#[test]
// ⛔ `--heavy` RE-ENABLES THIS AND SHOULD NOT. `./run_tests.sh --heavy` runs
// `cargo test --workspace --include-ignored`, which cannot distinguish "ignored
// because slow" from "ignored because invalid unless alone" — and this is the
// second kind. Run under `--heavy` its assertions may be satisfied by a
// sibling's `Assets<Image>`, so the dangerous outcome is a GREEN result that
// measured nothing. Recorded as member #12 in
// `docs/recipes/checks-that-did-not-run.md`.
#[ignore = "boots a real app and reads Assets<Image>: valid only when run alone, via scripts/measure_parallax_retire.sh"]
fn a_theme_the_player_walked_away_from_leaves_assets_image() {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, true);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    settle(&mut app, 240);

    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle(&mut app, 600);

    // ⛔ PREMISE ONE: the hub prefetched more than its own theme. If this is 1,
    // the prefetch did not run and there is nothing for a retire to remove --
    // the run would pass while measuring nothing, which is the failure mode this
    // whole family of tests exists to avoid.
    let before = resident_themes(&app);
    assert!(
        before.len() >= 3,
        "the hub holds only {:?}; the neighbour prefetch never loaded its \
         neighbours' themes, so this run cannot observe a retire",
        before
    );

    // ⛔ CAPTURED NOW, WHILE THE HANDLES STILL EXIST. After the retire the set
    // cannot tell us which `AssetId`s to ask `Assets<Image>` about, because the
    // handles that named them are exactly what was dropped.
    let before_ids = layer_asset_ids(&app, &before);

    let (target_room, arrival) = {
        let mut query = app
            .world_mut()
            .query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query.iter(app.world()).next().expect("a session room set");
        let zone = room_set
            .active_loading_zones()
            .iter()
            .find(|zone| zone.id == DEPARTING_DOOR || zone.name == DEPARTING_DOOR)
            .unwrap_or_else(|| {
                panic!(
                    "the active room '{}' has no `{DEPARTING_DOOR}`, so this test \
                     is not crossing a theme boundary",
                    room_set.active_spec().id
                )
            })
            .clone();
        let transition = room_set
            .transition_for_player(
                zone.aabb,
                ambition_platformer2d::engine_core::Vec2::ZERO,
                true,
            )
            .expect("the tech-bros door resolves to a transition");
        (
            room_set.rooms[transition.target_room].id.clone(),
            transition.arrival,
        )
    };

    let subject = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<
            &ambition_platformer2d::platformer::sim_id::SimId,
            With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        q.iter(world)
            .next()
            .expect("the hub has a primary avatar to send through its door")
            .clone()
    };

    let _ = app.world_mut()
        .resource_mut::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
        .record(
            0,
            ambition_platformer2d::actors::session::lifecycle_commit::LifecycleIntent::Transition(
                ambition_platformer2d::actors::session::lifecycle_commit::RoomTransitionIntent {
                    subject,
                    target_room,
                    arrival,
                    edge_exit: false,
                    zone_sfx: None,
                },
            ),
        );

    settle(&mut app, 900);

    let after = resident_themes(&app);
    let departed: Vec<_> = before
        .iter()
        .copied()
        .filter(|theme| !after.contains(theme))
        .collect();

    // ⛔ PREMISE TWO: something actually retired. Without this a policy that
    // never fires reads identical to one that fires correctly.
    assert!(
        !departed.is_empty(),
        "walking from the hub to the basement retired NOTHING -- before {:?}, \
         after {:?}. Either the retire system did not run or every theme the hub \
         held is still a neighbour of the destination",
        before,
        after
    );

    // The handles are gone from the set.
    for &theme in &departed {
        for &layer in ParallaxLayerAsset::ALL {
            assert!(
                app.world()
                    .resource::<GameAssets>()
                    .parallax_layers
                    .get(theme, layer)
                    .is_none(),
                "{theme:?}/{layer:?} still answers `get` after the theme retired",
            );
        }
    }

    // ⭐ AND THE PIXELS ARE GONE, which is the half a handle count cannot prove.
    // Bevy drops the asset when the last handle goes, on a later frame than the
    // drop -- hence the settle above and the re-check here.
    let departed_ids: Vec<_> = before_ids
        .iter()
        .filter(|(theme, _)| departed.contains(theme))
        .copied()
        .collect();
    assert!(
        !departed_ids.is_empty(),
        "no `AssetId` was captured for the retired themes {departed:?}, so the \
         `Assets<Image>` half of this test would assert over an empty set",
    );

    let images = app.world().resource::<Assets<Image>>();
    let still_resident: Vec<_> = departed_ids
        .iter()
        .filter(|(_, id)| images.get(*id).is_some())
        .map(|(theme, _)| *theme)
        .collect();
    assert!(
        still_resident.is_empty(),
        "the set dropped its handles but `Assets<Image>` still holds layers for \
         {still_resident:?}. Something else owns a clone -- a spawned \
         `ParallaxLayerVisual`, a manifest, or a prefetch entry -- so the memory \
         did not actually go and the retire is cosmetic",
    );

    println!(
        "[parallax-retire] before {:?} ({} layers) -> after {:?} ({} layers); retired {:?}",
        before,
        before.len() * ParallaxLayerAsset::ALL.len(),
        after,
        app.world().resource::<GameAssets>().parallax_layers.len(),
        departed,
    );
}
