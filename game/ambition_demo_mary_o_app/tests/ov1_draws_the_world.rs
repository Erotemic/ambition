//! **Oracle-violation OV1, closed and gate-enforced.**
//!
//! > *"A demo cannot DRAW its own world."* — `docs/planning/tracks.md`, OV1
//!
//! Playbook exit 3 proved the demo shell ASSEMBLES from the engine's public
//! groups. It then drew nothing, because spawning a camera and the room's static
//! visuals was app-local code inside `ambition_app`. Drawing a room is not
//! content, so that was an engine gap, and `ambition_render`'s
//! `PlatformerPresentationPlugin` is now the engine's answer.
//!
//! This test observes the ENTITIES, not pixels, and it runs the FULL render graph
//! against no wgpu backend and no window (`RenderMode::Headless` — the standard
//! Bevy recipe). The camera exists, the blocks exist as `RoomVisual` entities, and
//! the sprite chain is scheduled. A real window changes one enum value.
//!
//! **The presentation face needs a renderer foundation**, which is why it is NOT
//! in `build_demo_app` (that one is `add_headless_foundation` + no rasterizer, for
//! the sim-only shell and `tests/exit_3.rs`). Discovering that was worth the split:
//! a demo that wants to draw says `--features visible`, and one that only wants to
//! step the sim pays for no renderer at all.

#![cfg(feature = "visible")]

use bevy::prelude::*;

use ambition::game_shell::{ShellCommand, ShellLauncherCommand, ShellRouter};
use ambition::platformer::lifecycle::{RoomVisual, SessionScopedEntity};

use ambition_demo_mary_o_app::{build_windowed_demo_app, RenderMode};

fn drawn_demo() -> App {
    build_windowed_demo_app(RenderMode::Headless)
}

fn settle(app: &mut App) {
    for _ in 0..5 {
        app.update();
    }
}

fn room_visual_count(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&RoomVisual>();
    query.iter(app.world()).count()
}

fn scoped_entity_count(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&SessionScopedEntity>();
    query.iter(app.world()).count()
}

fn unscoped_room_visual_count(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query::<(&RoomVisual, Option<&SessionScopedEntity>)>();
    query
        .iter(app.world())
        .filter(|(_, owner)| owner.is_none())
        .count()
}

fn ui_node_count(app: &mut App) -> usize {
    let mut query = app.world_mut().query::<&bevy::ui::Node>();
    query.iter(app.world()).count()
}

/// UI nodes the ENGINE'S presentation face brought in — every `bevy_ui` node
/// that is NOT part of a HUD this demo itself declared.
///
/// The distinction is the whole point of the guard after the declared-HUD seam
/// landed. "Zero UI nodes" used to be a fine proxy for "the engine dragged in no
/// game UI", because a demo could not have a HUD at all. Now it can, by
/// DECLARING one on its provider — so the guard has to name what it forbids
/// (engine-owned UI) instead of forbidding all UI and thereby forbidding the
/// demo's own feature.
fn engine_owned_ui_node_count(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<&bevy::ui::Node, bevy::prelude::Without<ambition::presentation::DeclaredHudRoot>>();
    query.iter(app.world()).count()
}

/// Nodes belonging to the HUD this demo declared.
fn declared_hud_node_count(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<&bevy::ui::Node, bevy::prelude::With<ambition::presentation::DeclaredHudRoot>>();
    query.iter(app.world()).count()
}

fn active_route(app: &App) -> Option<&str> {
    app.world()
        .resource::<ShellRouter>()
        .active
        .as_ref()
        .map(|active| active.route_id.as_str())
}

/// Mary-O's 1-1 authors ground segments, two one-way platforms, a stair pyramid,
/// and a goal pole. They must all appear as room visuals.
#[test]
fn the_demo_spawns_the_rooms_static_visuals() {
    let mut app = drawn_demo();
    settle(&mut app);

    let room_visuals = {
        let mut q = app
            .world_mut()
            .query::<&ambition::platformer::lifecycle::RoomVisual>();
        q.iter(app.world()).count()
    };
    assert!(
        room_visuals > 0,
        "the engine's presentation plugin must spawn the active room's static \
         visuals — this is what OV1 was: the code existed in `ambition_render`, \
         but no plugin called it, so every demo would have copied `ambition_app`'s"
    );
}

/// The camera the host's `camera_follow` drives. Before OV1 the app spawned it;
/// a demo that added the host group got follow logic pointed at nothing.
#[test]
fn the_demo_spawns_a_main_camera_and_publishes_it() {
    let mut app = drawn_demo();
    settle(&mut app);

    let cameras = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<ambition::platformer::camera_layers::MainCamera>>();
        q.iter(app.world()).count()
    };
    assert_eq!(cameras, 1, "exactly one main camera");

    assert!(
        app.world()
            .get_resource::<ambition::platformer::camera_layers::MainCameraEntity>()
            .is_some(),
        "`MainCameraEntity` must be published — the host's camera-follow and the \
         portal viewer both resolve the camera through it"
    );
}

/// The presentation plugin is generic: it draws a platformer, not Ambition. If a
/// future change drags the game's HUD or menus into it, a demo's dependency wall
/// grows silently. This is the guard.
///
/// It asserts on `bevy_ui` nodes rather than on named resources, because the
/// naming test I first wrote was WRONG: `SandboxDevState` looked app-local and is
/// in fact ENGINE state (`commit_room_transition_geometry` writes it, and `ambition_runtime`
/// re-exports it as a host seam). A demo carrying it is correct. A demo carrying
/// Ambition's HUD is not, and a UI node is what a HUD is made of.
#[test]
fn the_presentation_plugin_adds_no_hud_and_no_menu() {
    let mut app = drawn_demo();
    settle(&mut app);

    assert_eq!(
        engine_owned_ui_node_count(&mut app),
        0,
        "the engine's presentation face draws the WORLD. Ambition's HUD, its \
         pause menu, and its dev overlays are the game's, assembled app-side. A \
         demo that wants a HUD declares one — that is what `owns` means in the \
         demos doctrine."
    );

    // The other direction, and it is not optional: filtering the count above by
    // a demo-owned marker would let an engine node hide simply by acquiring
    // that marker. Pinning the demo's own HUD to EXACTLY what it declared means
    // neither side can drift without a failure — too few and the demo's feature
    // regressed, too many and something else is wearing its marker.
    assert_eq!(
        declared_hud_node_count(&mut app),
        5,
        "this demo declares 5 HUD readout(s) — score, coins, time, lives, and the\n         transient card — and must draw exactly that many"
    );

    // ...and the engine's own visual-quality budget IS part of the face, because
    // `spawn_room_visuals` reads it to pick sprite variants.
    assert!(app
        .world()
        .get_resource::<ambition::render::quality::ResolvedVisualQuality>()
        .is_some(),);
}

/// **Standalone points its AssetServer at the engine's sprite tree, not the
/// empty cwd `assets/`.** The bug: the windowed demo took Bevy's default
/// cwd-relative `"assets"` file root, which has no `sprites/` tree, so every
/// character silently fell back to a bare box while the hosted app (which sets
/// the actors-assets root) drew fine — a standalone/hosted divergence. The demo
/// now sets the SAME shared engine root the hosted app uses; assert that root is
/// the on-disk sprite tree, so a `load("sprites/…png")` can resolve. (The async
/// load itself is not driven to completion by a manually-stepped headless app, so
/// this asserts the configured root rather than polling a load state.)
#[test]
fn the_windowed_demo_asset_root_is_the_engine_sprite_tree() {
    let root = ambition::asset_manager::actors_desktop_asset_root();
    // Shipped/override builds fall back to the relative "assets"; this dev-checkout
    // test proves the resolved root actually holds the character sheets.
    if root != "assets" {
        let sheet = std::path::Path::new(&root)
            .join("sprites")
            .join("super_mary_o_spritesheet.png");
        assert!(
            sheet.is_file(),
            "the windowed demo's asset root {root} must contain the character \
             sheets so standalone renders real sprites, not bare boxes"
        );
    }
}

#[test]
fn visible_mary_o_presentation_retires_and_relaunches_with_the_session() {
    let mut app = drawn_demo();
    settle(&mut app);

    assert_eq!(active_route(&app), Some("mary_o_gameplay"));
    let first_visual_count = room_visual_count(&mut app);
    assert!(
        first_visual_count > 0,
        "gameplay must materialize room presentation"
    );
    assert!(
        scoped_entity_count(&mut app) >= first_visual_count,
        "the active session must own at least the complete room presentation"
    );
    assert_eq!(
        unscoped_room_visual_count(&mut app),
        0,
        "every room visual must carry the exact session owner"
    );

    app.world_mut().write_message(ShellCommand::QuitToHome);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("mary_o_launcher"));
    assert_eq!(
        room_visual_count(&mut app),
        0,
        "room presentation retires at home"
    );
    assert_eq!(
        scoped_entity_count(&mut app),
        0,
        "the visible host leaves no activation-owned entity at home"
    );
    assert!(
        ui_node_count(&mut app) > 0,
        "the minimal host launcher must become the visible frontend at home"
    );

    app.world_mut()
        .write_message(ShellLauncherCommand::LaunchSelected);
    settle(&mut app);
    assert_eq!(active_route(&app), Some("mary_o_gameplay"));
    assert!(
        room_visual_count(&mut app) > 0,
        "relaunch rebuilds presentation"
    );
    assert_eq!(
        unscoped_room_visual_count(&mut app),
        0,
        "relaunched room visuals remain activation-owned"
    );
    assert_eq!(
        engine_owned_ui_node_count(&mut app),
        0,
        "the launcher presentation retires when gameplay resumes"
    );
    // ...and the demo's own HUD comes back WITH the session. It is
    // session-scoped, so a relaunch that rebuilt the world but not the readouts
    // would leave a game running with no HUD — silently, since nothing else
    // counts these nodes.
    assert_eq!(
        declared_hud_node_count(&mut app),
        5,
        "the declared HUD is session-scoped and must rebuild on relaunch"
    );
}
