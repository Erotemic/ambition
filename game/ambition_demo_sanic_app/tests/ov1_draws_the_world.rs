//! Oracle-violation OV1, closed and gate-enforced.
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
//! The presentation face needs a renderer foundation, which is why it is NOT
//! in `build_demo_app` (that one is `add_headless_foundation` + no rasterizer, for
//! the sim-only shell and `tests/exit_3.rs`). Discovering that was worth the split:
//! a demo that wants to draw says `--features visible`, and one that only wants to
//! step the sim pays for no renderer at all.

#![cfg(feature = "visible")]

use bevy::prelude::*;

use ambition_platformer2d::game_shell::{ShellCommand, ShellLauncherCommand, ShellRouter};
use ambition_platformer2d::platformer::lifecycle::{RoomVisual, SessionScopedEntity};

use ambition_demo_sanic_app::{build_windowed_demo_app, RenderMode};

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
/// Now it can, by DECLARING one on its provider — so the guard has to name what it forbids
/// (engine-owned UI) instead of forbidding all UI and thereby forbidding the demo's own feature.
fn engine_owned_ui_node_count(app: &mut App) -> usize {
    engine_owned_ui_nodes(app).len()
}

/// The same population, NAMED — because "16" does not say whose they are.
///
/// ⛔ Added 2026-09-04 after the feature union reported 16 here with no way to
/// tell a HUD from a pause menu from a dev overlay. A count is a finding you
/// cannot act on; the names say which composition dragged them in.
fn engine_owned_ui_nodes(app: &mut App) -> Vec<String> {
    let mut node_query = app
        .world_mut()
        .query_filtered::<(bevy::prelude::Entity, Option<&bevy::prelude::Name>), bevy::prelude::With<bevy::ui::Node>>();
    let all: Vec<(bevy::prelude::Entity, String)> = node_query
        .iter(app.world())
        .map(|(entity, name)| {
            (
                entity,
                name.map(|name| name.to_string())
                    .unwrap_or_else(|| format!("<unnamed {entity}>")),
            )
        })
        .collect();
    all.into_iter()
        .filter(|(entity, _)| {
            !ambition_platformer2d::presentation::declared_hud_owns(app.world(), *entity)
        })
        .map(|(_, name)| name)
        .collect()
}

/// Nodes belonging to the HUD this demo declared.
/// How many READOUTS the demo draws.
///
/// Counts `DeclaredHudSlot`, not `DeclaredHudRoot`. Both ride the nodes the declared-HUD pass owns
/// and they answer different questions: `DeclaredHudRoot` is the LIFETIME marker (the retire sweep
/// keys on it, and a gauge bar carries one so it dies with its slot), while a slot is one declared
/// readout. Counting roots made a demo that declared two readouts and published gauges look like
/// four — a gauge is presentation OF a readout, not another one.
fn declared_hud_node_count(app: &mut App) -> usize {
    let mut query = app
        .world_mut()
        .query_filtered::<&bevy::ui::Node, bevy::prelude::With<ambition_platformer2d::presentation::DeclaredHudSlot>>();
    query.iter(app.world()).count()
}

fn active_route(app: &App) -> Option<&str> {
    app.world()
        .resource::<ShellRouter>()
        .active
        .as_ref()
        .map(|active| active.route_id.as_str())
}

/// The demo's speedway authors exactly one solid block (`speedway_floor`) plus a
/// rideable chain. The floor is what must appear.
#[test]
fn the_demo_spawns_the_rooms_static_visuals() {
    let mut app = drawn_demo();
    settle(&mut app);

    let room_visuals = {
        let mut q = app
            .world_mut()
            .query::<&ambition_platformer2d::platformer::lifecycle::RoomVisual>();
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
fn the_demo_loads_shared_assets_and_draws_landmarks_and_the_loop() {
    let mut app = drawn_demo();
    settle(&mut app);

    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>()
            .is_some(),
        "the standalone demo must use the shared GameAssets loader, not an app-local sprite path"
    );

    let names: Vec<String> = {
        let mut q = app.world_mut().query::<&Name>();
        q.iter(app.world())
            .map(|name| name.as_str().to_owned())
            .collect()
    };
    // The LDtk-authored course's visible landmarks. The named ENTITY blocks
    // (monitors, rebound pads) keep their identities; the IntGrid-lowered
    // terrain draws under the loader's generic kind names ("ldtk solid" /
    // "ldtk one-way" / "ldtk hazard") — authored names are erased by the
    // IntGrid lowering (code smell #15), so kind presence is what's provable.
    for landmark in [
        "Block: monitor_speed",
        "Block: ReboundPad",
        "Block: ldtk solid",
        "Block: ldtk one-way",
        "Block: ldtk hazard",
    ] {
        assert!(
            names.iter().any(|name| name == landmark),
            "missing visible speedway landmark {landmark:?}"
        );
    }

    let floor_is_tiled = {
        let mut q = app.world_mut().query::<(&Name, &Sprite)>();
        q.iter(app.world()).any(|(name, sprite)| {
            name.as_str() == "Block: ldtk solid"
                && matches!(
                    &sprite.image_mode,
                    bevy::sprite::SpriteImageMode::Tiled { .. }
                )
        })
    };
    assert!(
        floor_is_tiled,
        "the speedway ground slabs must use Ambition's tiled ground sprite path"
    );
    let loop_segments = names
        .iter()
        .filter(|name| name.starts_with("Surface: sanic_loop segment "))
        .count();
    assert_eq!(
        loop_segments,
        ambition_demo_sanic::LOOP_RAMP_SEGMENTS
            + ambition_demo_sanic::LOOP_SEGMENTS
            + ambition_demo_sanic::LOOP_RUNOUT_SEGMENTS,
        "every collision segment in the smooth ramp+loop+runout route must have a visible strip"
    );
}

#[test]
fn the_demo_spawns_a_renderable_player_sprite() {
    let mut app = drawn_demo();
    settle(&mut app);

    let visible_players = {
        let mut q = app.world_mut().query_filtered::<(
            &ambition_platformer2d::render::rendering::PlayerSpriteCharacter,
            &Sprite,
        ), With<ambition_platformer2d::platformer::lifecycle::PlayerVisual>>(
        );
        q.iter(app.world())
            .filter(|(bound, _)| bound.id == "sanic")
            .count()
    };
    assert_eq!(
        visible_players, 1,
        "the generic character binder must attach the Sanic identity to the player sprite; \
         it uses the published sheet when present and the marked fallback otherwise"
    );
}

#[test]
fn changing_the_worn_form_rebinds_the_existing_super_sanic_sheet_path() {
    let mut app = drawn_demo();
    settle(&mut app);

    {
        let mut q = app.world_mut().query_filtered::<
            &mut ambition_platformer2d::characters::actor::WornCharacter,
            With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
        >();
        let mut worn = q
            .iter_mut(app.world_mut())
            .next()
            .expect("the visible demo spawned its canonical player");
        *worn = ambition_platformer2d::characters::actor::WornCharacter::new(
            ambition_demo_sanic::SUPER_SANIC_CHARACTER_ID,
        );
    }
    settle(&mut app);

    let rebound = {
        let mut q = app.world_mut().query_filtered::<
            &ambition_platformer2d::render::rendering::PlayerSpriteCharacter,
            With<ambition_platformer2d::platformer::lifecycle::PlayerVisual>,
        >();
        q.iter(app.world())
            .any(|bound| bound.id == ambition_demo_sanic::SUPER_SANIC_CHARACTER_ID)
    };
    assert!(
        rebound,
        "the generic binder must rebind the same player to the Super Sanic catalog row"
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
            .query_filtered::<Entity, With<ambition_platformer2d::platformer::camera_layers::MainCamera>>();
        q.iter(app.world()).count()
    };
    assert_eq!(cameras, 1, "exactly one main camera");

    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::platformer::camera_layers::MainCameraEntity>()
            .is_some(),
        "`MainCameraEntity` must be published — the host's camera-follow and the \
         portal viewer both resolve the camera through it"
    );
}

/// The presentation plugin is generic: it draws a platformer, not Ambition. If a
/// future change drags the game's HUD or menus into it, a demo's dependency wall
/// grows silently. This is the guard.
///
/// A demo carrying it is correct. A demo carrying Ambition's HUD is not, and a UI node is what
/// a HUD is made of.
#[test]
fn the_presentation_plugin_adds_no_hud_and_no_menu() {
    let mut app = drawn_demo();
    settle(&mut app);

    let engine_owned = engine_owned_ui_nodes(&mut app);
    assert_eq!(
        engine_owned.len(),
        0,
        "the engine's presentation face draws the WORLD. Ambition's HUD, its \
         pause menu, and its dev overlays are the game's, assembled app-side. A \
         demo that wants a HUD declares one — that is what `owns` means in the \
         demos doctrine. Present: {engine_owned:?}"
    );

    // The other direction, and it is not optional: filtering the count above by
    // a demo-owned marker would let an engine node hide simply by acquiring
    // that marker. Pinning the demo's own HUD to EXACTLY what it declared means
    // neither side can drift without a failure — too few and the demo's feature
    // regressed, too many and something else is wearing its marker.
    assert_eq!(
        declared_hud_node_count(&mut app),
        2,
        "this demo declares 2 HUD readout(s) — the ring counter and the\n         end-of-act results card — and must draw exactly that many"
    );

    // ...and the engine's own visual-quality budget IS part of the face, because
    // `spawn_room_visuals` reads it to pick sprite variants.
    assert!(app
        .world()
        .get_resource::<ambition_platformer2d::render::quality::ResolvedVisualQuality>()
        .is_some(),);
}

#[test]
fn visible_sanic_presentation_retires_and_relaunches_with_the_session() {
    let mut app = drawn_demo();
    settle(&mut app);

    assert_eq!(active_route(&app), Some("sanic_gameplay"));
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
    assert_eq!(active_route(&app), Some("sanic_launcher"));
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
    assert_eq!(active_route(&app), Some("sanic_gameplay"));
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
        2,
        "the declared HUD is session-scoped and must rebuild on relaunch"
    );
}

/// Rings must render via the ANIMATED pickup path (a `sanic_ring_prop` sheet),
/// not the static coin fallback. Reproduces the runtime binding headlessly: the
/// animated path names its entities "Pickup sprite: ring"; the static fallback
/// names them "Room entity: ring".
#[test]
fn rings_render_with_the_animated_sheet_not_static_coins() {
    let mut app = drawn_demo();
    settle(&mut app);
    let names: Vec<String> = {
        let mut q = app.world_mut().query::<&Name>();
        q.iter(app.world())
            .map(|name| name.as_str().to_owned())
            .collect()
    };
    let animated = names
        .iter()
        .filter(|n| n.as_str() == "Pickup sprite: ring")
        .count();
    let static_coins = names
        .iter()
        .filter(|n| n.as_str() == "Room entity: ring")
        .count();
    assert_eq!(
        static_coins, 0,
        "no ring may fall back to the static coin — found {static_coins} coins, {animated} animated"
    );
    assert!(
        animated >= 30,
        "every ring must bind the animated sanic_ring_prop sheet — found {animated}"
    );
}

/// The readout carries a live value, not just a node. A HUD that spawns the
/// right number of empty text nodes looks identical to a working one in a node
/// count, so this reads the text back.
///
/// It also pins the direction the seam exists for: the ENGINE never learns what
/// a ring is. "RINGS" is here because `publish_sanic_ring_readout` writes it
/// from `PlayerHudFacts.balance` — the shared economy's wallet, credited by the
/// ordinary `currency` pickup path that Sanic's 35 authored rings already use.
#[test]
fn the_declared_hud_shows_the_games_own_words_and_a_live_value() {
    let mut app = drawn_demo();
    settle(&mut app);

    let texts: Vec<String> = {
        let mut query = app
            .world_mut()
            .query_filtered::<&bevy::prelude::Text, bevy::prelude::With<ambition_platformer2d::presentation::DeclaredHudRoot>>();
        query.iter(app.world()).map(|text| text.0.clone()).collect()
    };
    assert_eq!(texts.len(), 2, "both declared slots draw a text node");
    // The results card is deliberately BLANK while the act is running — it is
    // published only on a clear — so the ring counter is the one with text.
    let rings = texts
        .iter()
        .find(|t| t.starts_with("RINGS "))
        .unwrap_or_else(|| panic!("the ring readout is on screen; got {texts:?}"));
    assert!(
        texts.iter().any(|t| t.is_empty()),
        "the results card stays empty until the act is cleared: {texts:?}"
    );
    assert!(
        rings.starts_with("RINGS "),
        "the demo's own label reaches the screen; the engine supplies no \
         vocabulary. got {:?}",
        texts[0]
    );
    assert!(
        texts[0].trim_start_matches("RINGS ").parse::<i32>().is_ok(),
        "the readout carries the live wallet balance, not a placeholder: {:?}",
        texts[0]
    );
}

/// The speedway's BACKDROP moves. (72h S12)
///
/// OV1's original question was whether a demo can draw its world at all. This is
/// the same question one frame later: the backdrop spawned, and then
/// `sync_parallax_layers` — the system that slides each layer against the camera
/// at its own rate — was registered by `game/ambition_app` ALONE. So this demo,
/// whose entire subject is a camera travelling fast along a speedway, drew its
/// sky pinned to the world origin and let it slide out of frame.
///
/// Nothing about that reads as a missing system: the art is correct, in the
/// wrong place, and only once you move. Sanic is the demo where it is most
/// visible and the one that had no guard for it.
#[test]
fn the_speedway_backdrop_follows_the_camera() {
    use ambition_platformer2d::view::ParallaxLayerVisual;

    let mut app = drawn_demo();
    // Without it this test passed alone and FAILED inside the full suite binary, which is the
    // flake shape this run already paid for once today in `shell_host_lifecycle`.
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));

    // Long enough for the session to activate and the asset bind to land: the
    // layers cannot exist before `GameAssets` carries this room's theme.
    let mut spawned = false;
    for _ in 0..600 {
        app.update();
        let mut query = app.world_mut().query::<&ParallaxLayerVisual>();
        if query.iter(app.world()).next().is_some() {
            spawned = true;
            break;
        }
    }
    assert!(
        spawned,
        "the speedway never spawned a parallax layer, so the motion assertion \
         below would compare two empty lists. Either the theme's art is absent \
         from the asset catalog or nothing loaded it for this room — both are \
         real answers and neither is 'the backdrop moves'."
    );

    let positions = |app: &mut App| -> Vec<f32> {
        let mut query = app
            .world_mut()
            .query_filtered::<&Transform, With<ParallaxLayerVisual>>();
        query
            .iter(app.world())
            .map(|transform| transform.translation.x)
            .collect()
    };
    let before = positions(&mut app);

    // Hold right. The body accelerates down the speedway, the camera follows it,
    // and each layer follows the camera at its own factor.
    //
    // SAMPLE THE WHOLE RUN, not the endpoints. This test compared `before`
    // against `after` alone and failed for four months' worth of wrong reasons —
    // it was read as a missing `sync_parallax_layers`, then as an unregistered
    // `camera_follow`, then as a broken camera clamp, then as Sanic being slow.
    // He is not slow: he reaches 1216 px/s and x = 1533 by frame 120, and the
    // layers move ~1000px with him. The speedway LOOPS. By frame 240 he is
    // back at x ≈ 260, the camera is back on its left clamp (centre x = 400,
    // exactly `view_width / 2` in a 6400-wide room), and every layer is back
    // where it started — correctly, to the float.
    //
    // So the endpoints are the one pair of samples that cannot see this working.
    // Track the largest deviation across the run instead: a backdrop that follows
    // must be somewhere else at SOME point, and that is true whether the course
    // loops, doubles back, or runs straight.
    let mut max_deviation = 0.0f32;
    for _ in 0..240 {
        ambition_platformer2d::sim::drive_control_frame(
            app.world_mut(),
            ambition_platformer2d::sim::ControlFrame {
                axis_x: 1.0,
                ..Default::default()
            },
        );
        app.update();
        let now = positions(&mut app);
        if now.len() == before.len() {
            for (start, current) in before.iter().zip(&now) {
                max_deviation = max_deviation.max((current - start).abs());
            }
        }
    }

    let after = positions(&mut app);
    assert_eq!(
        before.len(),
        after.len(),
        "the layer set changed while running, so the comparison is not about \
         motion"
    );
    assert!(
        max_deviation > 1.0,
        "over 240 frames of holding right, no parallax layer ever moved more \
         than {max_deviation} px from where it started. The sky is pinned to the \
         world while the camera runs down the speedway. ⚠ before blaming a \
         system: check that the SUBJECT moved and that the camera left its \
         clamp — this assertion has been misread as four different bugs, and \
         every one of them was innocent."
    );
}
