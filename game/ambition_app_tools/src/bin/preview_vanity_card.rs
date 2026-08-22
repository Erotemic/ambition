//! Looped preview of the programmatic startup vanity card.
//!
//! A visible change verified by a compile is a change nobody has seen. This
//! boots the REAL shell sequence — the same `ShellSequenceCatalog`, the same
//! registered segment kind, the same completion command the startup run-in
//! uses — around that one card, and routes back to itself when it finishes, so
//! the beat replays until the window is closed.
//!
//!     cargo run -p ambition_app_tools --bin preview_vanity_card
//!
//! no `--features visible`. That was true when this bin lived in
//! `ambition_app`, where `visible` is a default feature; `ambition_app_tools`
//! declares only `causal`, and cargo hard-errors on a feature a package does
//! not have — so the documented command could not run at all.
//!
//! Set `AMBITION_PREVIEW_SCALE_FACTOR=3` to preview at a phone's pixel density
//! rather than the desktop's, which is the axis a resolution proxy cannot reach.
//!
//! It composes NO gameplay: no session, no simulation, no launcher. If the card
//! draws here and not in the game, the difference is host composition, not the
//! card.

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
    // A phone differs from this desktop in DENSITY as well as size, and density
    // is the axis nothing else here can reach: `capture_scene` proxies a phone's
    // 640x360 at a scale factor of 1.0, so a card that is mis-scaled by exactly
    // the scale factor looks perfect in every desktop check and wrong on the
    // device. Set this to a phone's factor (2.0-3.5) to see what the phone sees.
    if let Ok(raw) = std::env::var(PREVIEW_SCALE_FACTOR_ENV) {
        match raw.trim().parse::<f32>() {
            Ok(factor) if factor > 0.0 => {
                info!("{PREVIEW_SCALE_FACTOR_ENV}={factor}: previewing at a phone's pixel density");
                window.resolution.set_scale_factor_override(Some(factor));
            }
            _ => panic!("{PREVIEW_SCALE_FACTOR_ENV} must be a positive number, got {raw:?}"),
        }
    }
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(window),
        ..default()
    }));
    app.add_plugins(MinimalShellPlugins);
    app.add_plugins(ambition_content::presentation::AmbitionPresentationPlugin);
    configure_preview_shell(&mut app);
    app.run();
}

fn configure_preview_shell(app: &mut App) {
    use ambition_platformer2d::game_shell::{
        ShellExperienceId, ShellSegmentRole, ShellSegmentSpec,
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
                segments: vec![ShellSegmentSpec::registered(
                    "preview_programmatic_vanity_card",
                    ShellSegmentRole::Vanity,
                    ambition_content::presentation::vanity_card_made_this_meme::MADE_THIS_MEME_CARD_SEGMENT_KIND,
                )],
            },
        );
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(PREVIEW_ROUTE, PREVIEW_ROUTE));
}
