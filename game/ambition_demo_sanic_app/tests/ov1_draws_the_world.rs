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

use ambition_demo_sanic_app::{build_windowed_demo_app, RenderMode};

fn drawn_demo() -> App {
    build_windowed_demo_app(RenderMode::Headless)
}

/// The demo's speedway authors exactly one solid block (`speedway_floor`) plus a
/// rideable chain. The floor is what must appear.
#[test]
fn the_demo_spawns_the_rooms_static_visuals() {
    let mut app = drawn_demo();
    app.update(); // Startup: the presentation plugin's set runs here.

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

#[test]
fn the_demo_spawns_a_renderable_player_sprite() {
    let mut app = drawn_demo();
    app.update();

    let visible_players = {
        let mut q = app.world_mut().query_filtered::<
            Entity,
            (
                With<ambition::platformer::lifecycle::PlayerVisual>,
                With<Sprite>,
            ),
        >();
        q.iter(app.world()).count()
    };
    assert_eq!(
        visible_players, 1,
        "the generic presentation face must attach a fallback Sprite to the
         simulation-owned PlayerVisual; otherwise demos draw the room but no player"
    );
}

/// The camera the host's `camera_follow` drives. Before OV1 the app spawned it;
/// a demo that added the host group got follow logic pointed at nothing.
#[test]
fn the_demo_spawns_a_main_camera_and_publishes_it() {
    let mut app = drawn_demo();
    app.update();

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
/// in fact ENGINE state (`load_room_geometry` writes it, and `ambition_runtime`
/// re-exports it as a host seam). A demo carrying it is correct. A demo carrying
/// Ambition's HUD is not, and a UI node is what a HUD is made of.
#[test]
fn the_presentation_plugin_adds_no_hud_and_no_menu() {
    let mut app = drawn_demo();
    app.update();

    let ui_nodes = {
        let mut q = app.world_mut().query::<&bevy::ui::Node>();
        q.iter(app.world()).count()
    };
    assert_eq!(
        ui_nodes, 0,
        "the engine's presentation face draws the WORLD. Ambition's HUD, its \
         pause menu, and its dev overlays are the game's, assembled app-side. A \
         demo that wants a HUD builds its own — that is what `owns` means in the \
         demos doctrine."
    );

    // ...and the engine's own visual-quality budget IS part of the face, because
    // `spawn_room_visuals` reads it to pick sprite variants.
    assert!(app
        .world()
        .get_resource::<ambition::render::quality::ResolvedVisualQuality>()
        .is_some(),);
}
