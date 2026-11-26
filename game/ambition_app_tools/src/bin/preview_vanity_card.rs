//! Looped preview of the programmatic startup vanity card using the real shell
//! sequence and completion path.
//!
//!     cargo run -p ambition_app_tools --features game --bin preview_vanity_card
//!
//! Set `AMBITION_PREVIEW_SCALE_FACTOR=3` to emulate a phone-scale pixel density.
//! The preview composes shell presentation only: no session, simulation, or launcher.

use ambition_content::presentation::vanity_card_made_this_meme::{
    MADE_THIS_MEME_CARD_SEGMENT_KIND, MadeThisMemeCardPlugin, made_this_meme_card_duration,
};
use ambition_platformer2d::asset_manager::{
    AssetProfile,
    bevy_integration::AmbitionAssetCatalog,
    library_source::{LibraryAssets, register_sources},
    platformer_assets::Platformer2dAssetCatalog,
};
use bevy::prelude::*;

use ambition_platformer2d::game_shell::{
    MinimalShellPlugins, ShellCompletionPolicy, ShellHostConfiguration, ShellHostSpec,
    ShellRouteCatalog, ShellRouteSpec, ShellSequenceCatalog, ShellSequenceSpec,
};

/// Overrides the window's pixel density, so a desktop can stand in for a phone.
const PREVIEW_SCALE_FACTOR_ENV: &str = "AMBITION_PREVIEW_SCALE_FACTOR";

const PREVIEW_ROUTE: &str = "vanity_card_preview";
const PREVIEW_EXPERIENCE: &str = "vanity_card_preview";

fn main() {
    let mut app = App::new();
    let mut window = Window {
        title: "Ambition Vanity Card Preview".into(),
        resolution: (1280, 720).into(),
        ..default()
    };
    // A phone differs in density as well as size. `capture_scene` proxies a
    // phone's 640x360 at scale factor 1.0, so a card mis-scaled by exactly
    // the scale factor looks right on desktop and wrong on the device. Set
    // this to a phone's factor (2.0-3.5) to see what the phone sees.
    if let Ok(raw) = std::env::var(PREVIEW_SCALE_FACTOR_ENV) {
        match raw.trim().parse::<f32>() {
            Ok(factor) if factor > 0.0 => {
                info!("{PREVIEW_SCALE_FACTOR_ENV}={factor}: previewing at a phone's pixel density");
                window.resolution.set_scale_factor_override(Some(factor));
            }
            _ => panic!("{PREVIEW_SCALE_FACTOR_ENV} must be a positive number, got {raw:?}"),
        }
    }
    register_sources(&mut app, "assets");
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(window),
        ..default()
    }));
    app.add_plugins(MinimalShellPlugins);
    ambition_app::app::host_pack::HOST_PACK.import_libraries(app.world_mut());
    let snapshot = app.world().resource::<LibraryAssets>().snapshot();
    let manifest = snapshot
        .libraries
        .asset_manifest()
        .expect("Read the asset catalog");
    app.insert_resource(
        Platformer2dAssetCatalog::new(
            AmbitionAssetCatalog::new(manifest),
            AssetProfile::DesktopDevLoose,
        )
        .with_snapshot(&snapshot),
    );
    ambition_platformer2d::render::ui_fonts::UiFontsPlugin::ensure_installed(&mut app);
    app.add_plugins(MadeThisMemeCardPlugin);
    app.add_systems(Startup, |mut commands: Commands| {
        commands.spawn(Camera2d);
    });
    configure_preview_shell(&mut app);
    app.run();
}

fn configure_preview_shell(app: &mut App) {
    use ambition_platformer2d::game_shell::{
        ShellExperienceId, ShellSegmentPolicy, ShellSegmentRole, ShellSegmentSpec,
    };

    // `on_complete: GoTo(itself)` is what makes this a loop, and it is the
    // ordinary route policy rather than a preview-only replay mechanism.
    app.world_mut()
        .resource_mut::<ShellRouteCatalog>()
        .register(
            ShellRouteSpec::new(PREVIEW_ROUTE, PREVIEW_EXPERIENCE)
                .on_complete(ShellCompletionPolicy::GoTo(PREVIEW_ROUTE.into())),
        );
    app.world_mut()
        .resource_mut::<ShellSequenceCatalog>()
        .register(
            ShellExperienceId::new(PREVIEW_EXPERIENCE),
            ShellSequenceSpec {
                segments: vec![
                    ShellSegmentSpec::registered(
                        "preview_programmatic_vanity_card",
                        ShellSegmentRole::Vanity,
                        MADE_THIS_MEME_CARD_SEGMENT_KIND,
                    )
                    .with_policy(ShellSegmentPolicy {
                        auto_advance_after: Some(made_this_meme_card_duration()),
                        ..default()
                    }),
                ],
            },
        );
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(PREVIEW_ROUTE, PREVIEW_ROUTE));
}
