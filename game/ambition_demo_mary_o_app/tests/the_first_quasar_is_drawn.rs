//! Her quasar is drawn from the first frame she has it.
//!
//! Mary-O is drawn from her parts, and her root sprite has no image. The
//! quasar shader reads her as ONE image, so the rigged driver composites her
//! while something declares her in `ComposedBodyDemand`. An effect that reads
//! a body as one image must declare that demand from the FACT that turns the
//! effect on. If it declares from state that it can build only after the body
//! is composited, the effect waits for a different reader to composite the
//! body first.
//!
//! Measured before the fix (2026-10-08): 36 frames into her first quasar she
//! had no overlay and was not composited. The overlay came at the first hit,
//! because a hit flash composites the body.
//!
//! These tests run the drawn composition (`visible`), with the real demand
//! set, driver and overlay set:
//!
//! ```text
//! cargo test -p ambition_demo_mary_o_app --features visible --test mary_o_it -- the_first_quasar_is_drawn
//! ```

#![cfg(feature = "visible")]

use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

use ambition_demo_mary_o::quasar_shader::MaryOQuasarMaterial;
use ambition_demo_mary_o_app::{build_windowed_demo_app_entering, RenderMode};
use ambition_platformer2d::actors::features::empowerment::Empowered;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::render::rendering::actors::rigged::RiggedPresentedBy;
use ambition_platformer2d::sprite_sheet::character::rigged::FrameInSprite;

/// A body composited for its first reader draws its cell on the second frame
/// (the page and its cameras arrive with the first frame's commands), and the
/// super-state reaches her health one tick after it is given. This bound is
/// far below the time to the first enemy of 1-1, so no hit flash helps.
const FRAMES_TO_SHOW: usize = 6;

/// Level 1-1 of the drawn demo, with her standing in it.
///
/// `RenderMode::Headless` has no render app, so `Material2dPlugin` is not
/// installed and the quasar's material collection is declared here. The
/// overlay systems run only where that collection exists.
fn drawn_level() -> App {
    let mut app = build_windowed_demo_app_entering(
        RenderMode::Headless,
        ambition_demo_mary_o::MARY_O_GAMEPLAY_ROUTE,
        ambition_demo_mary_o::LEVEL_1_1_ROOM_ID,
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    app.init_asset::<MaryOQuasarMaterial>();
    for _ in 0..240 {
        app.update();
    }
    app
}

fn player(app: &mut App) -> Entity {
    let world = app.world_mut();
    let mut players = world.query_filtered::<Entity, With<PrimaryPlayer>>();
    players.iter(world).next().expect("a player")
}

/// Whether the driver composites her now: it states her frame in the cell.
fn is_composited(app: &App, player: Entity) -> bool {
    app.world().get::<FrameInSprite>(player).is_some()
}

/// The image of each quasar overlay that is shown.
fn shown_overlays(app: &mut App) -> Vec<Handle<Image>> {
    let world = app.world_mut();
    let mut overlays = world.query::<(&MeshMaterial2d<MaryOQuasarMaterial>, &Visibility)>();
    let shown: Vec<_> = overlays
        .iter(world)
        .filter(|(_, visibility)| **visibility == Visibility::Visible)
        .map(|(material, _)| material.0.clone())
        .collect();
    let materials = world.resource::<Assets<MaryOQuasarMaterial>>();
    shown
        .iter()
        .map(|material| materials.get(material).expect("the overlay's material").color_texture.clone())
        .collect()
}

fn give_the_quasar(app: &mut App, player: Entity, seconds: f32) {
    app.world_mut().entity_mut(player).insert(Empowered::for_seconds(
        ambition_demo_mary_o::star::COSMIC_QUASAR_SUPER_STATE,
        seconds,
    ));
}

/// Step until `done` holds; the number of frames, or `None` past `most`.
fn frames_until(app: &mut App, most: usize, mut done: impl FnMut(&mut App) -> bool) -> Option<usize> {
    for frame in 1..=most {
        app.update();
        if done(app) {
            return Some(frame);
        }
    }
    None
}

/// The overlay is shown over her composited image.
fn assert_the_quasar_reads_her_image(app: &mut App, player: Entity) {
    let overlays = shown_overlays(app);
    assert_eq!(overlays.len(), 1, "one quasar overlay is shown");
    assert!(is_composited(app, player), "the quasar is shown over a body that is not composited");
    let root = app.world().get::<Sprite>(player).expect("her root sprite").image.clone();
    assert_eq!(overlays[0], root, "the overlay samples a different image than her root draws");
    assert!(
        app.world().resource::<Assets<Image>>().get(&root).is_some(),
        "the image the overlay samples does not exist"
    );
}

/// She is given her first quasar while nothing else reads her as one image.
#[test]
fn her_first_quasar_is_drawn_at_once() {
    let mut app = drawn_level();
    let player = player(&mut app);
    assert!(
        app.world().get::<RiggedPresentedBy>(player).is_some(),
        "premise: she is drawn from her parts"
    );
    assert!(!is_composited(&app, player), "premise: nothing reads her as one image");
    assert!(shown_overlays(&mut app).is_empty(), "premise: no quasar is shown");

    give_the_quasar(&mut app, player, 999.0);
    let frames = frames_until(&mut app, FRAMES_TO_SHOW, |app| !shown_overlays(app).is_empty());
    assert!(
        frames.is_some(),
        "{FRAMES_TO_SHOW} frames into her first quasar no overlay is shown (composited: {})",
        is_composited(&app, player)
    );
    assert_the_quasar_reads_her_image(&mut app, player);
}

/// The quasar ends, she goes back to her parts, and the next quasar is drawn
/// at once also: the effect does not depend on what the first one left.
#[test]
fn a_second_quasar_is_drawn_after_she_went_back_to_her_parts() {
    let mut app = drawn_level();
    let player = player(&mut app);
    give_the_quasar(&mut app, player, 0.5);
    assert!(
        frames_until(&mut app, FRAMES_TO_SHOW, |app| !shown_overlays(app).is_empty()).is_some(),
        "premise: the first quasar is shown"
    );
    let back = frames_until(&mut app, 240, |app| shown_overlays(app).is_empty() && !is_composited(app, player));
    assert!(back.is_some(), "premise: the quasar ended and she is drawn from her parts again");

    give_the_quasar(&mut app, player, 999.0);
    let frames = frames_until(&mut app, FRAMES_TO_SHOW, |app| !shown_overlays(app).is_empty());
    assert!(frames.is_some(), "{FRAMES_TO_SHOW} frames into her second quasar no overlay is shown");
    assert_the_quasar_reads_her_image(&mut app, player);
}

/// A pipe hides her quasar with her (Jon, 2026-10-08: the rainbow was drawn
/// over the pipe she went down).
///
/// A pipe is drawn in front of the cast, so a body inside it is hidden. What
/// is drawn ON a body (the quasar's overlay, a flash) is above the body, and
/// it must stay under the pipe's plane (`BODY_DEPTH_BAND`).
///
/// Measured before: the overlay and the pipes of 1-1 were both drawn at
/// depth 21.0.
#[test]
fn a_pipe_hides_her_quasar_with_her() {
    use ambition_platformer2d::render::rendering::PropVisual;

    let mut app = drawn_level();
    let player = player(&mut app);
    give_the_quasar(&mut app, player, 999.0);
    assert!(
        frames_until(&mut app, FRAMES_TO_SHOW, |app| !shown_overlays(app).is_empty()).is_some(),
        "premise: the quasar is shown"
    );
    let world = app.world_mut();
    let body_z = world.get::<Transform>(player).expect("her transform").translation.z;
    let mut overlays = world.query::<(&MeshMaterial2d<MaryOQuasarMaterial>, &Transform, &Visibility)>();
    let overlay_z = overlays
        .iter(world)
        .find(|(.., visibility)| **visibility == Visibility::Visible)
        .map(|(_, transform, _)| transform.translation.z)
        .expect("the shown overlay");
    let mut props = world.query::<(&PropVisual, &Transform)>();
    let pipes: Vec<f32> = props
        .iter(world)
        .filter(|(prop, _)| prop.draw.occludes_bodies())
        .map(|(_, transform)| transform.translation.z)
        .collect();
    assert!(pipes.len() >= 2, "premise: 1-1 draws its pipes ({})", pipes.len());
    assert!(overlay_z > body_z, "premise: the quasar is drawn over her ({overlay_z} against {body_z})");
    for pipe_z in pipes {
        assert!(
            overlay_z < pipe_z,
            "her quasar is drawn at depth {overlay_z} and a pipe at {pipe_z}: the pipe does not hide it"
        );
    }
}

