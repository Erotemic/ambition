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
/// Now it can, by DECLARING one on its provider — so the guard has to name what it forbids
/// (engine-owned UI) instead of forbidding all UI and thereby forbidding the demo's own feature.
fn engine_owned_ui_node_count(app: &mut App) -> usize {
    // ⛔⛔ `Without<DeclaredHudRoot>` EXCLUDED THE ROOT AND COUNTED ITS CHILDREN.
    // Measured 2026-09-03: this returned 40 against an expected 0, and all forty
    // were the demo's OWN declared HUD — five readouts × (panel, portrait,
    // stocks row, four stock pips, stock count). The root carries
    // `DeclaredHudRoot`; the nodes under it carry `DeclaredHudPortrait`,
    // `DeclaredHudStock`, `DeclaredHudStockCount` or nothing at all
    // (`hud/declared.rs:240-300`), so a root-only filter reads a demo's
    // permitted HUD as an engine violation.
    //
    // ⚠ The doctrine this guards EXPLICITLY ALLOWS what it was flagging: "A demo
    // that wants a HUD declares one — that is what `owns` means in the demos
    // doctrine." The engine was innocent and the filter was wrong.
    //
    // ⇒ Ownership is the SUBTREE, so ask by ancestry rather than by marker.
    let declared_roots: std::collections::HashSet<bevy::prelude::Entity> = {
        let mut roots = app
            .world_mut()
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<
                ambition_platformer2d::presentation::DeclaredHudRoot,
            >>();
        roots.iter(app.world()).collect()
    };
    let mut nodes = app
        .world_mut()
        .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<bevy::ui::Node>>();
    let candidates: Vec<bevy::prelude::Entity> = nodes.iter(app.world()).collect();
    candidates
        .into_iter()
        .filter(|entity| !under_a_declared_hud(app, *entity, &declared_roots))
        .count()
}

/// Is this node the declared HUD's, at any depth?
///
/// ⚠ Walks `ChildOf` to the top rather than checking the parent: the stock pips
/// are grandchildren (root → panel → stocks row → pip), so one level of
/// parent-checking would still have counted twenty of them.
fn under_a_declared_hud(
    app: &mut App,
    entity: bevy::prelude::Entity,
    roots: &std::collections::HashSet<bevy::prelude::Entity>,
) -> bool {
    let mut current = entity;
    loop {
        if roots.contains(&current) {
            return true;
        }
        match app.world().get::<bevy::prelude::ChildOf>(current) {
            Some(parent) => current = parent.parent(),
            None => return false,
        }
    }
}

/// Nodes belonging to the HUD this demo declared.
/// How many READOUTS the demo draws.
///
/// Counts `DeclaredHudSlot`, not `DeclaredHudRoot`. Both markers ride the nodes
/// this pass owns, but they answer different questions: `DeclaredHudRoot` is the
/// LIFETIME marker (the retire sweep keys on it, and a gauge bar carries it so
/// it dies with its slot), while a slot is one declared readout. Counting roots
/// made a demo that declared five readouts and published one gauge look like six
/// — the gauge is presentation OF a readout, not another one.
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

/// Mary-O's 1-1 authors ground segments, two one-way platforms, a stair pyramid,
/// and a goal pole. They must all appear as room visuals.
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

/// The world-label placement pass is part of the GENERIC face, not of
/// Ambition's composition.
///
/// `spawn_room_visuals` — engine code, called by the plugin under test — spawns
/// `WorldLabel` components for authored signage and fixture plates. For one
/// batch the systems that give those components meaning (placement, the subject
/// fade, the typeface) were installed only by
/// `ActorNameplatePresentationPlugin`, which lives in `ambition_app`'s plugin
/// list and in no demo. So AC12/AC20's "one ranked placement pass over every
/// world-space text label" was a policy of the full Ambition app, and this demo,
/// Sanic and the external consumer all still drew labels at raw anchors in
/// Bevy's fallback font.
///
/// Asserted by the POLICY, not by the plugin list: two labels from two families
/// are spawned on top of each other, and the pass must separate them. A test
/// that looked for the plugin would go green on a plugin that had stopped
/// working — presence is not behaviour.
#[test]
fn the_generic_presentation_face_places_world_labels() {
    use ambition_platformer2d::render::rendering::{WorldLabel, WorldLabelFamily};

    let mut app = drawn_demo();
    settle(&mut app);

    let anchor = Vec3::new(96.0, 96.0, 40.0);
    let spawn_label = |app: &mut App, id: &str, family: WorldLabelFamily| {
        app.world_mut()
            .spawn((
                Text2d::new("PLACEMENT PROBE"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Transform::from_translation(anchor),
                WorldLabel::new(id, family, anchor),
            ))
            .id()
    };
    let sign = spawn_label(&mut app, "probe:sign", WorldLabelFamily::Signage);
    let plate = spawn_label(&mut app, "probe:plate", WorldLabelFamily::Actor);
    settle(&mut app);

    let at = |app: &App, entity: Entity| app.world().get::<Transform>(entity).unwrap().translation;
    assert_eq!(
        at(&app, sign),
        anchor,
        "authored signage is the family that never yields — if this moved, the \
         ranking is upside down rather than absent"
    );
    assert_ne!(
        at(&app, plate),
        anchor,
        "two world labels from two families were left drawn through each other: \
         the placement pass is not scheduled in this composition, which is the \
         whole of the AC12 defect and not a demo-only cosmetic"
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
        .get_resource::<ambition_platformer2d::render::quality::ResolvedVisualQuality>()
        .is_some(),);
}

#[test]
fn the_windowed_demo_asset_root_is_the_engine_sprite_tree() {
    let root = ambition_platformer2d::asset_manager::actors_desktop_asset_root();
    // Shipped/override builds fall back to the relative "assets"; this dev-checkout
    // test proves the resolved root actually holds the character sheets.
    if root != "assets" {
        let sheet = std::path::Path::new(&root)
            .join("sprites")
            .join("mary_o_v2_spritesheet.png");
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

/// A `VfxMessage` this demo writes must be DRAWN by this demo.
///
/// This demo composes `PlatformerEnginePlugins` + `PlatformerHostPlugins` and no `ambition_app`, so
/// the message went into a queue with no reader.
///
/// Exactly the OV1 shape the rest of this file guards, one message channel over:
/// the code existed in `ambition_render`, and no plugin a demo installs called
/// it. `ambition_platformer2d_host::HostVfxPresentationPlugin` is the answer.
///
/// asserted on the ENTITY, not on the plugin list. A test that looked for the
/// plugin would stay green on a plugin that had stopped working, and the same
/// assertion covers the brick-break `VfxMessage::Burst` and every impact spark,
/// which travel the same subscriber.
#[test]
fn a_vfx_message_this_demo_writes_is_drawn_by_this_demo() {
    use ambition_platformer2d::render::fx::ParticleVisual;
    use ambition_platformer2d::vfx::VfxMessage;

    let mut app = drawn_demo();
    settle(&mut app);

    let particle_count = |app: &mut App| {
        let mut q = app.world_mut().query::<&ParticleVisual>();
        q.iter(app.world()).count()
    };
    let before = particle_count(&mut app);

    app.world_mut().write_message(VfxMessage::CoinPop {
        pos: ambition_platformer2d::engine_core::Vec2::new(64.0, 64.0),
    });
    settle(&mut app);

    // ⚠ THE DIAGNOSTIC IS PART OF THE ASSERTION, added 2026-09-04 after this
    // failed under the workspace feature union and passed at this crate's own
    // feature set — so "it drew nothing" had to say whether the SUBSCRIBER is
    // absent or present-and-silent. Those want opposite fixes.
    // `HostVfxPresentationPlugin` gates its whole chain on `session_world_exists`,
    // so the first thing to know is whether this composition HAS one.
    let session_roots = {
        let mut q = app
            .world_mut()
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<
                ambition_platformer2d::platformer::lifecycle::SessionRoot,
            >>();
        q.iter(app.world()).count()
    };
    let after = particle_count(&mut app);
    assert!(
        after > before,
        "this composition wrote a `VfxMessage::CoinPop` and drew nothing \
         (particles {before} -> {after}); \
         session roots present: {session_roots}. \
         `HostVfxPresentationPlugin` gates its whole chain on \
         `session_world_exists`, so ZERO roots means the subscriber is \
         installed and skipped rather than absent — a different defect from \
         the original coin-pop report."
    );
}
