//! BOSS-CINEMATIC-COVERAGE, rendered: a boss that wakes in the shipped host
//! has its name card drawn on screen, not only published.
//!
//! `every_boss_shows_its_name` reads the card where the renderer reads it
//! (`BossRenderIndex`). This captures the frame: the rendered host starts in
//! a boss room, the boss is woken, and the pixels of the card's name, in its
//! colour, are counted inside the card's own rectangle, against the same
//! rectangle before the boss woke.

use ambition_platformer2d::boss_encounter::BossConfig;
use ambition_platformer2d::characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::game_shell::ShellCommand;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::render::capture::{CaptureProgress, CaptureSettings};
use ambition_platformer2d::render::rendering::boss_stage::BossNameCardPart;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};

/// Non-Wolfenstein bosses whose card is the encounter's own: the T-rex (its
/// conductor stages nothing) and the Clockwork Warden (no conductor).
const ROOMS: [&str; 2] = ["trex_arena", "basement_boss"];

fn boss_room(room: &str) -> App {
    let app = crate::shell_host_rendered::rendered_app_in_room(
        ambition_app::app::VisibleRenderMode::OffscreenGpu,
        Some(room),
    );
    let mut app = crate::local_play_modes::gpu_capture(app, &format!("ambition-boss-card-{room}.png"), UVec2::new(1280, 720));
    crate::shell_host_rendered::settle(&mut app);
    app.world_mut().write_message(ShellCommand::GoTo(ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into()));
    crate::shell_host_rendered::settle(&mut app);
    let world = app.world_mut();
    let mut players = world.query_filtered::<&mut BodyHealth, With<PrimaryPlayer>>();
    for mut health in players.iter_mut(world) {
        health.health.invulnerable.set(Invulnerability::SCRIPTED, true);
    }
    app
}

/// The fullest card any boss shows on this frame.
fn card(app: &App) -> f32 {
    app.world()
        .get_resource::<ambition_platformer2d::sim_view::BossRenderIndex>()
        .map(|index| index.iter().map(|(_, view)| view.stage.card).fold(0.0, f32::max))
        .unwrap_or(0.0)
}

/// Capture this frame and return it decoded.
fn captured(app: &mut App) -> Image {
    *app.world_mut().resource_mut::<CaptureProgress>() = CaptureProgress::default();
    crate::local_play_modes::capture_current_frame(app);
    let output = app.world().resource::<CaptureSettings>().output.clone();
    Image::from_buffer(
        &std::fs::read(output).expect("the capture was written"),
        bevy::image::ImageType::Extension("png"),
        bevy::image::CompressedImageFormats::empty(),
        true,
        default(),
        default(),
    )
    .expect("the capture decodes")
}

/// The physical rectangle (min, max) of the card's name text, when drawn.
fn name_rect(app: &mut App) -> Option<(UVec2, UVec2)> {
    let world = app.world_mut();
    let mut parts = world.query::<(&BossNameCardPart, &ComputedNode, &UiGlobalTransform, &InheritedVisibility)>();
    parts
        .iter(world)
        .find(|(part, node, _, visible)| **part == BossNameCardPart::Name && node.size().x > 0.0 && visible.get())
        .map(|(_, node, transform, _)| {
            let center = transform.translation;
            let half = node.size() * 0.5;
            ((center - half).max(Vec2::ZERO).as_uvec2(), (center + half).as_uvec2())
        })
}

/// Pixels inside `rect` in the name's colour (srgb 0.97, 0.93, 0.86).
fn name_coloured(image: &Image, (min, max): (UVec2, UVec2)) -> usize {
    let size = image.size();
    let mut count = 0;
    for y in min.y..max.y.min(size.y) {
        for x in min.x..max.x.min(size.x) {
            let color = image.get_color_at(x, y).expect("inside the image").to_srgba();
            if (color.red - 0.97).abs() < 0.06 && (color.green - 0.93).abs() < 0.06 && (color.blue - 0.86).abs() < 0.06 {
                count += 1;
            }
        }
    }
    count
}

#[test]
#[ignore = "needs a wgpu adapter; captures each boss's name card"]
fn a_woken_bosss_name_card_is_drawn_in_its_rectangle() {
    for room in ROOMS {
        let mut app = boss_room(room);
        assert!(card(&app) < 0.01, "{room}: precondition: no card is up before the boss wakes ({})", card(&app));
        let before = captured(&mut app);

        // Stand the player in front of the boss, as the headless witness does.
        let boss = {
            let world = app.world_mut();
            world
                .query::<(&BossConfig, &ae::BodyKinematics)>()
                .iter(world)
                .map(|(_, kin)| kin.pos)
                .next()
                .expect("the room places a boss")
        };
        let player = {
            let world = app.world_mut();
            world.query_filtered::<Entity, With<PrimaryPlayer>>().single(world).expect("one primary body")
        };
        crate::alices_battery_reaches_bob::put_body_at(&mut app, player, ae::Vec2::new(boss.x - 160.0, boss.y));
        let mut frames = 0;
        while card(&app) < 0.99 {
            frames += 1;
            assert!(frames < 60 * 7, "{room}: the boss's card did not come all the way up in seven seconds");
            app.update();
        }
        let rect = name_rect(&mut app).unwrap_or_else(|| panic!("{room}: the card is up and no name part is drawn"));
        let during = captured(&mut app);
        let (lit, control) = (name_coloured(&during, rect), name_coloured(&before, rect));
        println!("{room}: name rect {rect:?}: {lit} name-coloured pixels with the card, {control} before it");
        assert!(lit > 400, "{room}: the card's rectangle holds only {lit} pixels of the name's colour");
        assert!(control * 10 < lit, "control: before the wake the same rectangle held {control} (with the card: {lit})");
    }
}
