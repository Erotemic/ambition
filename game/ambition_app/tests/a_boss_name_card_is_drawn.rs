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
        crate::alices_reading_reaches_bob::put_body_at(&mut app, player, ae::Vec2::new(boss.x - 160.0, boss.y));
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

/// Every drawn name part's physical rectangle (min, max), left to right.
fn name_rects(app: &mut App) -> Vec<(UVec2, UVec2)> {
    let world = app.world_mut();
    let mut parts = world.query::<(&BossNameCardPart, &ComputedNode, &UiGlobalTransform, &InheritedVisibility)>();
    let mut rects: Vec<(UVec2, UVec2)> = parts
        .iter(world)
        .filter(|(part, node, _, visible)| **part == BossNameCardPart::Name && node.size().x > 0.0 && visible.get())
        .map(|(_, node, transform, _)| {
            let center = transform.translation;
            let half = node.size() * 0.5;
            ((center - half).max(Vec2::ZERO).as_uvec2(), (center + half).as_uvec2())
        })
        .collect();
    rects.sort_by_key(|(min, _)| (min.x, min.y));
    rects
}

/// Each boss's card, by the boss's id.
fn cards_by_boss(app: &App) -> std::collections::BTreeMap<String, f32> {
    app.world()
        .get_resource::<ambition_platformer2d::sim_view::BossRenderIndex>()
        .map(|index| index.iter().map(|(boss, view)| (boss.to_owned(), view.stage.card)).collect())
        .unwrap_or_default()
}

/// Two local views in two boss rooms: Alice before the T-rex, Bob before the
/// Clockwork Warden. Each view draws its own boss's card, in its own half,
/// and neither rectangle holds the name once the cards are put away.
#[test]
#[ignore = "needs a wgpu adapter; captures two views' name cards in two boss rooms"]
fn two_views_in_two_boss_rooms_each_draw_their_bosss_card() {
    use ambition_platformer2d::platformer::lifecycle::InRoomInstance;
    let host = crate::local_play_modes::gpu_capture(
        crate::shell_host_rendered::rendered_app_in_room(ambition_app::app::VisibleRenderMode::OffscreenGpu, Some(ROOMS[0])),
        "ambition-boss-card-two-views.png",
        UVec2::new(1280, 720),
    );
    let (mut app, _pads) = crate::local_play_modes::split_session(host);
    let (bodies, rooms) = crate::local_play_modes::separate_participant_to(&mut app, 1, ROOMS[1]);
    {
        let world = app.world_mut();
        for body in bodies {
            world.get_mut::<BodyHealth>(body).expect("a body").health.invulnerable.set(Invulnerability::SCRIPTED, true);
        }
    }

    for (body, room) in bodies.into_iter().zip(rooms) {
        let boss = {
            let world = app.world_mut();
            world
                .query::<(&BossConfig, &ae::BodyKinematics, &InRoomInstance)>()
                .iter(world)
                .find(|(_, _, at)| **at == room)
                .map(|(_, kin, _)| kin.pos)
                .unwrap_or_else(|| panic!("the room {room:?} places a boss"))
        };
        crate::alices_reading_reaches_bob::put_body_at(&mut app, body, ae::Vec2::new(boss.x - 160.0, boss.y));
    }
    let mut frames = 0;
    loop {
        let cards = cards_by_boss(&app);
        if cards.len() >= 2 && cards.values().all(|card| *card >= 0.99) {
            break;
        }
        frames += 1;
        assert!(frames < 60 * 7, "both cards did not come all the way up in seven seconds: {cards:?}");
        app.update();
    }
    let rects = name_rects(&mut app);
    assert_eq!(rects.len(), 2, "two views, two drawn names: {rects:?}");
    assert!(rects[0].1.x <= 640 && rects[1].0.x >= 640, "each name is drawn in its own view's half: {rects:?}");
    let during = captured(&mut app);
    let output = app.world().resource::<CaptureSettings>().output.clone();
    std::fs::copy(&output, output.with_file_name("ambition-boss-card-two-views-up.png")).expect("the capture is kept for review");
    // The control: the same rectangles once both cards are put away. The
    // bosses wake while the split session is set up, so there is no frame
    // before the wake to compare with.
    let mut frames = 0;
    while cards_by_boss(&app).values().any(|card| *card > 0.01) {
        frames += 1;
        assert!(frames < 60 * 20, "the cards were not put away in twenty seconds: {:?}", cards_by_boss(&app));
        app.update();
    }
    let after = captured(&mut app);
    for rect in rects {
        let (lit, control) = (name_coloured(&during, rect), name_coloured(&after, rect));
        println!("two views: name rect {rect:?}: {lit} name-coloured pixels with the card, {control} after it");
        assert!(lit > 400, "the card's rectangle {rect:?} holds only {lit} pixels of the name's colour");
        assert!(control * 10 < lit, "control: with the card put away the same rectangle held {control} (with it: {lit})");
    }
}
