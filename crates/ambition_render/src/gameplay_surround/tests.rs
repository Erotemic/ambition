//! The surround must cover exactly the display the gameplay camera does not.

use bevy::prelude::*;

use ambition_engine_core as ae;
use ambition_platformer_primitives::gameplay_presentation::{
    profiles, resolve_gameplay_presentation, ControlFootprints, GameplayPresentationInput,
    GameplayPresentationProfile, PresentationEnvironment, ScreenInsets,
};

use super::*;

fn app_with(display: ae::Vec2, profile: &GameplayPresentationProfile) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(GameplaySurroundPlugin);
    app.insert_resource(resolve_gameplay_presentation(GameplayPresentationInput {
        display_px: display,
        safe_area_insets: ScreenInsets::ZERO,
        profile,
        occlusions: &[],
        control_footprints: ControlFootprints::default(),
    }));
    // ONE update. The bars must be laid out at spawn, not on a following
    // frame: the frame a fixed-aspect game starts is the frame its pillarboxes
    // appear, and a one-frame gap is a flash of uncleared framebuffer.
    app.update();
    app
}

fn bar_rects(app: &mut App) -> Vec<(SurroundRegion, Rect)> {
    let mut out: Vec<(SurroundRegion, Rect)> = app
        .world_mut()
        .query::<(&GameplaySurroundBar, &Node)>()
        .iter(app.world())
        .filter_map(|(bar, node)| {
            let px = |value: Val| match value {
                Val::Px(px) => px,
                _ => 0.0,
            };
            let min = Vec2::new(px(node.left), px(node.top));
            let size = Vec2::new(px(node.width), px(node.height));
            (size.x > 0.5 && size.y > 0.5).then_some((bar.0, Rect::from_corners(min, min + size)))
        })
        .collect();
    out.sort_by_key(|(region, _)| *region);
    out
}

/// A fixed-aspect profile paints every pillarbox pixel, and paints nothing
/// inside the gameplay rectangle.
#[test]
fn the_surround_tiles_exactly_what_the_camera_does_not_draw() {
    let display = ae::Vec2::new(2400.0, 1080.0);
    let profile =
        *profiles::fixed_four_by_three().for_environment(PresentationEnvironment::Desktop);
    let mut app = app_with(display, &profile);

    let gameplay = app
        .world()
        .resource::<ResolvedGameplayPresentation>()
        .gameplay_rect;
    let gameplay_rect = Rect::from_corners(gameplay.min, gameplay.max);

    let bars = bar_rects(&mut app);
    assert_eq!(bars.len(), 2, "a 20:9 display pillarboxes: {bars:?}");

    let painted: f32 = bars
        .iter()
        .map(|(_, rect)| rect.width() * rect.height())
        .sum();
    let unpainted = display.x * display.y - gameplay_rect.width() * gameplay_rect.height();
    assert!(
        (painted - unpainted).abs() < 1.0,
        "painted {painted} but {unpainted} of the display is uncovered",
    );

    for (region, rect) in &bars {
        assert!(
            rect.intersect(gameplay_rect).is_empty(),
            "{region:?} bar {rect:?} covers gameplay {gameplay_rect:?}",
        );
    }
}

/// Full bleed leaves nothing unpainted, so the surround owns nothing and
/// despawns itself rather than lingering as a zero-size node tree.
#[test]
fn full_bleed_draws_no_surround() {
    let mut app = app_with(
        ae::Vec2::new(1920.0, 1080.0),
        &GameplayPresentationProfile::full_bleed(),
    );
    assert_eq!(
        app.world_mut()
            .query::<&GameplaySurroundRoot>()
            .iter(app.world())
            .count(),
        0,
    );
}

/// Switching from fixed aspect to full bleed at runtime tears the surround
/// down — a stale bar over live gameplay would be worse than never drawing one.
#[test]
fn leaving_a_fixed_aspect_profile_tears_the_surround_down() {
    let display = ae::Vec2::new(2400.0, 1080.0);
    let profile =
        *profiles::fixed_four_by_three().for_environment(PresentationEnvironment::Desktop);
    let mut app = app_with(display, &profile);
    assert!(!bar_rects(&mut app).is_empty());

    app.insert_resource(resolve_gameplay_presentation(GameplayPresentationInput {
        display_px: display,
        safe_area_insets: ScreenInsets::ZERO,
        profile: &GameplayPresentationProfile::full_bleed(),
        occlusions: &[],
        control_footprints: ControlFootprints::default(),
    }));
    app.update();

    assert_eq!(
        app.world_mut()
            .query::<&GameplaySurroundRoot>()
            .iter(app.world())
            .count(),
        0,
        "the surround must not survive a switch to full bleed",
    );
}
