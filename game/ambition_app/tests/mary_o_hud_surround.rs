//! Does the real Mary O route actually put the real HUD in its surround?
//!
//! Design of record: `docs/planning/triage/gameplay-presentation-profiles.md`.
//!
//! `ambition_render::hud`'s own tests hand `place_player_hud` a resolved
//! presentation and a synthetic `PlayerHudRoot`. That proves the placement
//! arithmetic and nothing about the game: whether the Mary O route really
//! resolves a 4:3 viewport, whether the real HUD really spawns under it, and
//! whether the rectangle `bevy_ui` finally lays out really clears the gameplay
//! rect are all assumed.
//!
//! This assembles the actual chain instead:
//!
//! ```text
//! Mary O route selected
//!   -> provider session active
//!     -> fixed 4:3 presentation resolved by the real host
//!       -> a controlled actor exists
//!         -> the real spawn_player_hud runs
//!           -> place_player_hud selects the left surround
//!             -> the laid-out HUD rect is outside gameplay_rect
//! ```
//!
//! Nothing here inserts a `ResolvedGameplayPresentation`.

use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::transform::TransformPlugin;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::MinimalPlugins;

use ambition::engine_core as ae;
use ambition::game_shell::{ActiveGameplaySession, ShellCommand};
use ambition::platformer::gameplay_presentation::{
    GameplayViewportPolicy, ResolvedGameplayPresentation, ScreenRect, SurroundRegion,
};
use ambition::render::hud::{
    place_player_hud, spawn_player_hud, PlayerHudRoot, HUD_MARGIN, OVERLAY_ANCHOR,
};
use ambition_app::app::shell_host;

/// A 20:9 phone-shaped display. A 4:3 gameplay rectangle is 1440 wide here, so
/// each side surround is 480px — comfortably more than the HUD needs.
const WIDE: ae::Vec2 = ae::Vec2::new(2400.0, 1080.0);
/// Barely wider than 4:3: the gameplay rect takes 1365 of 1400, leaving ~17px
/// columns that no HUD can use.
const NARROW: ae::Vec2 = ae::Vec2::new(1400.0, 1024.0);

/// The real shell host, plus the visible-host presentation cluster and the two
/// real HUD systems the flagship app registers.
///
/// Deliberately not all of `add_presentation_plugins`: menus, fonts and
/// materials have nothing to do with this question, and pulling them in would
/// make a HUD-placement failure look like an asset failure. Everything on the
/// path being tested is the production article.
fn mary_o_hud_app(display: ae::Vec2) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.init_asset::<bevy::image::TextureAtlasLayout>();
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.add_plugins(bevy::text::TextPlugin);
    // `bevy_ui`'s viewport picking hard-requires the picking resources.
    app.add_plugins((
        bevy::input::InputPlugin,
        bevy::picking::PickingPlugin,
        bevy::picking::InteractionPlugin,
    ));
    // The real layout pass, so the final HUD rectangle is one taffy computed.
    app.add_plugins(bevy::ui::UiPlugin);

    app.init_state::<ambition::platformer::schedule::GameMode>();
    app.insert_resource(shell_host::AmbitionShellHosted);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    shell_host::compose_ambition_shell_host(&mut app);

    // The visible-host presentation cluster: this is what resolves Mary O's
    // declared 4:3 profile against the window.
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);

    // The two real HUD systems, in the order and ordering the flagship uses.
    app.add_systems(
        Update,
        (
            spawn_player_hud,
            place_player_hud
                .after(ambition::platformer::gameplay_presentation::GameplayPresentationSet),
        )
            .chain(),
    );

    app.world_mut().spawn((window_at(display), PrimaryWindow));
    app
}

fn window_at(display: ae::Vec2) -> Window {
    let mut resolution = WindowResolution::new(display.x as u32, display.y as u32);
    resolution.set_scale_factor(1.0);
    resolution.set(display.x, display.y);
    Window {
        resolution,
        ..default()
    }
}

/// Drive the shell into the real Mary O gameplay route and let the session,
/// the world and the player settle.
fn enter_mary_o(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
    app.world_mut()
        .write_message(ShellCommand::GoTo(ambition::game_shell::ShellRouteId::new(
            ambition_demo_mary_o::provider::MARY_O_GAMEPLAY_ROUTE,
        )));
    for _ in 0..24 {
        app.update();
    }
    assert!(
        app.world().resource::<ActiveGameplaySession>().0.is_some(),
        "the Mary O gameplay session must be active",
    );
}

fn resolved(app: &App) -> &ResolvedGameplayPresentation {
    app.world().resource::<ResolvedGameplayPresentation>()
}

/// The rectangle `bevy_ui` actually laid the HUD root out at, in logical px.
fn laid_out_hud(app: &mut App) -> ScreenRect {
    let mut query = app.world_mut().query_filtered::<(
        &bevy::ui::ComputedNode,
        &bevy::ui::UiGlobalTransform,
    ), With<PlayerHudRoot>>();
    let (computed, transform) = query
        .iter(app.world())
        .next()
        .expect("the real player HUD is spawned");
    let scale = computed.inverse_scale_factor();
    let size = computed.size() * scale;
    let center = transform.translation * scale;
    ScreenRect::from_min_size(center - size * 0.5, size)
}

fn hud_anchor(app: &mut App) -> ae::Vec2 {
    let mut query = app
        .world_mut()
        .query_filtered::<&Node, With<PlayerHudRoot>>();
    let node = query
        .iter(app.world())
        .next()
        .expect("the real player HUD is spawned");
    let px = |value| match value {
        Val::Px(px) => px,
        other => panic!("expected Px, got {other:?}"),
    };
    ae::Vec2::new(px(node.left), px(node.top))
}

/// The whole vertical slice, on the route that motivated it.
#[test]
fn the_mary_o_route_puts_the_real_hud_in_the_reserved_surround() {
    let mut app = mary_o_hud_app(WIDE);
    enter_mary_o(&mut app);

    // The route really did resolve a fixed 4:3 viewport — nothing inserted it.
    let layout = resolved(&app);
    assert!(
        matches!(layout.viewport, GameplayViewportPolicy::FixedAspect { .. }),
        "the Mary O route must resolve a fixed-aspect viewport, got {:?}",
        layout.viewport,
    );
    let gameplay = layout.gameplay_rect;
    assert!(
        (gameplay.width() / gameplay.height() - 4.0 / 3.0).abs() < 0.01,
        "the resolved gameplay rect must be 4:3, got {gameplay:?}",
    );
    assert!(
        layout.prefers_surround_hud(),
        "and the profile must want its HUD in the surround",
    );

    let region = resolved(&app)
        .hud_region(SurroundRegion::Left)
        .expect("a 4:3 viewport on 20:9 leaves a left HUD region");

    // The HUD occupies the region it asked for. This is the DISCRIMINATING
    // assertion: on a display this widely pillarboxed the overlay anchor also
    // happens to miss the gameplay rect, so only the anchor itself
    // distinguishes "placed in the resolved region" from "never moved".
    let anchor = hud_anchor(&mut app);
    assert_eq!(
        anchor,
        region.min + ae::Vec2::splat(HUD_MARGIN),
        "the HUD must be anchored in the resolved left surround region \
         {region:?}, not left at its overlay anchor {OVERLAY_ANCHOR:?}",
    );

    // ...and the rectangle taffy finally lays out agrees, which the render
    // crate's own unit test cannot show.
    let drawn = laid_out_hud(&mut app);
    assert!(
        drawn.width() > 1.0 && drawn.height() > 1.0,
        "the HUD must have a real laid-out size, got {drawn:?}",
    );
    assert_eq!(
        drawn.min, anchor,
        "the laid-out rect must start at the anchor the placement wrote",
    );
    assert!(
        drawn.min.x >= region.min.x - 0.5 && drawn.max.x <= region.max.x + 0.5,
        "and stay inside the left surround region {region:?}, got {drawn:?}",
    );
    // The user-visible consequence. Implied on this display rather than
    // proven by it, but it is the point of the whole feature.
    assert!(
        !drawn.overlaps(gameplay),
        "the laid-out HUD {drawn:?} must be clear of the 4:3 gameplay rect \
         {gameplay:?}",
    );
}

/// On a display with no usable surround the HUD stays on screen and simply
/// overlays, at a real size — never squeezed, never collapsed.
#[test]
fn a_display_without_usable_surround_keeps_the_hud_overlaying() {
    let mut app = mary_o_hud_app(NARROW);
    enter_mary_o(&mut app);

    let layout = resolved(&app);
    let gameplay = layout.gameplay_rect;
    let region = layout.hud_region(SurroundRegion::Left);
    assert!(
        region.is_none_or(|rect| rect.width() < 100.0),
        "the fixture must leave no usable left surround, got {region:?}",
    );

    let drawn = laid_out_hud(&mut app);
    assert!(
        drawn.width() > 1.0 && drawn.height() > 1.0,
        "the HUD must still be laid out at a real size, got {drawn:?}",
    );
    assert!(
        drawn.overlaps(gameplay),
        "with nowhere else to go it overlays gameplay: {drawn:?} vs {gameplay:?}",
    );

    // The overlay anchor by name, not a squeezed sliver of a 17px column.
    assert_eq!(
        hud_anchor(&mut app),
        OVERLAY_ANCHOR,
        "with no usable surround the HUD must fall back to its overlay anchor",
    );
}
