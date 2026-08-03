//! Looped preview of the programmatic startup vanity card.
//!
//! A visible change verified by a compile is a change nobody has seen. This
//! boots the REAL shell sequence — the same `ShellSequenceCatalog`, the same
//! registered segment kind, the same completion command the startup run-in
//! uses — around that one card, and routes back to itself when it finishes, so
//! the beat replays until the window is closed.
//!
//!     cargo run -p ambition_app --bin preview_vanity_card --features visible
//!
//! It composes NO gameplay: no session, no simulation, no launcher. If the card
//! draws here and not in the game, the difference is host composition, not the
//! card.

use bevy::prelude::*;

use ambition_platformer2d::game_shell::{
    MinimalShellPlugins, ShellCompletionPolicy, ShellHostConfiguration, ShellHostSpec,
    ShellRouteCatalog, ShellRouteSpec, ShellSequenceCatalog, ShellSequenceSpec,
};

const PREVIEW_ROUTE: &str = "vanity_card_preview";
const PREVIEW_EXPERIENCE: &str = "vanity_card_preview";

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Ambition Vanity Card Preview".into(),
            resolution: (1280, 720).into(),
            ..default()
        }),
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
                    ambition_content::presentation::vanity_card::AMBITION_VANITY_CARD_SEGMENT_KIND,
                )],
            },
        );
    app.world_mut()
        .resource_mut::<ShellHostConfiguration>()
        .spec = Some(ShellHostSpec::new(PREVIEW_ROUTE, PREVIEW_ROUTE));
}
