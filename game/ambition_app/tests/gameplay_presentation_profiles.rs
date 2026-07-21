//! Real-provider proof for gameplay presentation profiles.
//!
//! Design of record: `docs/planning/triage/gameplay-presentation-profiles.md`.
//!
//! The engine must not contain a branch that selects presentation by game name
//! (oracle 12), which means the only place the three motivating profiles can be
//! checked is where the real providers declare them. Each provider builds into
//! its own `App` — that also proves the declaration is App-local rather than a
//! process-global that leaks between games.
//!
//! Cheap on purpose: no window, no session, no gameplay. Building the provider
//! plugin is enough, because the declaration rides authoring rather than
//! runtime.

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::game_shell::MinimalShellPlugins;
use ambition::presentation::gameplay_presentation::{
    resolve_gameplay_presentation, AspectRatio, ControlFootprints, GameplayPresentationInput,
    GameplayPresentationProfileCatalog, GameplayPresentationProfiles, GameplayViewportPolicy,
    PresentationEnvironment, ScreenInsets, SubjectFramingPolicy,
};

fn provider_app(install: impl FnOnce(&mut App)) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalShellPlugins);
    app.add_plugins(ambition::load::AmbitionLoadPlugin);
    install(&mut app);
    app
}

fn declared(app: &App, route: &str) -> Option<GameplayPresentationProfiles> {
    app.world()
        .get_resource::<GameplayPresentationProfileCatalog>()
        .and_then(|catalog| catalog.get(route))
        .copied()
}

fn ambition_app() -> App {
    provider_app(|app| {
        app.add_plugins(ambition_content::AmbitionContentPlugin);
        app.add_plugins(ambition_content::provider::AmbitionExperiencePlugin::default());
    })
}

fn sanic_app() -> App {
    provider_app(|app| {
        app.add_plugins(ambition_demo_sanic::provider::SanicExperiencePlugin);
    })
}

fn mary_o_app() -> App {
    provider_app(|app| {
        app.add_plugins(ambition_demo_mary_o::provider::MaryOExperiencePlugin);
    })
}

/// Ambition flagship: normal camera on desktop, occlusion-aware soft framing
/// when touch is primary (oracle 6 + the mobile half of the motivation).
#[test]
fn the_flagship_declares_desktop_normal_and_touch_occlusion_aware() {
    let app = ambition_app();
    let profiles = declared(&app, ambition_content::provider::AMBITION_GAMEPLAY_ROUTE)
        .expect("the flagship declares presentation profiles");

    let desktop = profiles.for_environment(PresentationEnvironment::Desktop);
    assert_eq!(desktop.viewport, GameplayViewportPolicy::FullBleed);
    assert_eq!(
        desktop.framing,
        SubjectFramingPolicy::Normal,
        "desktop Ambition must keep the camera it already had",
    );

    let touch = profiles.for_environment(PresentationEnvironment::TouchPrimary);
    assert_eq!(touch.viewport, GameplayViewportPolicy::FullBleed);
    assert!(
        touch.framing.consumes_occlusions(),
        "touch-primary Ambition must keep the subject out of control regions",
    );
}

/// Sanic: soft velocity-aware framing on every platform (oracle 8).
#[test]
fn sanic_declares_soft_framing_everywhere() {
    let app = sanic_app();
    let profiles = declared(&app, ambition_demo_sanic::provider::SANIC_GAMEPLAY_ROUTE)
        .expect("Sanic declares presentation profiles");

    for environment in [
        PresentationEnvironment::Desktop,
        PresentationEnvironment::TouchPrimary,
        PresentationEnvironment::Handheld,
    ] {
        let profile = profiles.for_environment(environment);
        assert_eq!(profile.viewport, GameplayViewportPolicy::FullBleed);
        let framing = profile
            .framing
            .profile()
            .unwrap_or_else(|| panic!("Sanic must frame softly in {environment:?}"));
        assert!(
            framing.look_ahead_seconds > 0.0,
            "high-speed framing must lead the runner",
        );
    }
}

/// Mary-O: a fixed 4:3 gameplay viewport on every platform, surround reserved.
#[test]
fn mary_o_declares_a_fixed_four_by_three_viewport() {
    let app = mary_o_app();
    let profiles = declared(&app, ambition_demo_mary_o::provider::MARY_O_GAMEPLAY_ROUTE)
        .expect("Mary-O declares presentation profiles");

    for environment in [
        PresentationEnvironment::Desktop,
        PresentationEnvironment::TouchPrimary,
        PresentationEnvironment::Handheld,
    ] {
        let profile = profiles.for_environment(environment);
        let GameplayViewportPolicy::FixedAspect { aspect, .. } = profile.viewport else {
            panic!("Mary-O must be fixed-aspect in {environment:?}");
        };
        assert!((aspect.ratio() - AspectRatio::FOUR_THREE.ratio()).abs() < 1e-6);
    }
}

/// The declaration is OPTIONAL. Pocket is the fourth-provider architecture
/// proof, and it must keep composing without saying anything about
/// presentation — otherwise this became a tax on every future game.
#[test]
fn a_provider_may_decline_to_declare_presentation() {
    let app = provider_app(|app| {
        app.add_plugins(ambition_demo_pocket::PocketExperiencePlugin);
    });
    let declared_any = app
        .world()
        .get_resource::<GameplayPresentationProfileCatalog>()
        .is_some_and(|catalog| catalog.routes().next().is_some());
    assert!(
        !declared_any,
        "Pocket declares nothing, so no catalog entry should exist",
    );
}

/// Each provider's declaration is App-LOCAL: building Sanic must not teach an
/// Ambition app about Sanic's profile. A process-global here would make the
/// multi-game host's framing depend on link order.
#[test]
fn declarations_do_not_leak_between_apps() {
    let ambition = ambition_app();
    let sanic = sanic_app();

    assert!(declared(
        &ambition,
        ambition_demo_sanic::provider::SANIC_GAMEPLAY_ROUTE
    )
    .is_none());
    assert!(declared(&sanic, ambition_content::provider::AMBITION_GAMEPLAY_ROUTE).is_none());
}

/// The declarations must produce genuinely DIFFERENT layouts on one display —
/// three presets that resolved identically would be decoration, not policy.
#[test]
fn the_three_declarations_resolve_to_different_layouts() {
    let display = ae::Vec2::new(2400.0, 1080.0);
    let resolve = |profiles: GameplayPresentationProfiles, environment| {
        resolve_gameplay_presentation(GameplayPresentationInput {
            display_px: display,
            safe_area_insets: ScreenInsets::ZERO,
            profile: profiles.for_environment(environment),
            occlusions: &[],
            control_footprints: ControlFootprints::default(),
        })
    };

    let ambition = declared(
        &ambition_app(),
        ambition_content::provider::AMBITION_GAMEPLAY_ROUTE,
    )
    .expect("flagship profiles");
    let sanic = declared(
        &sanic_app(),
        ambition_demo_sanic::provider::SANIC_GAMEPLAY_ROUTE,
    )
    .expect("Sanic profiles");
    let mary_o = declared(
        &mary_o_app(),
        ambition_demo_mary_o::provider::MARY_O_GAMEPLAY_ROUTE,
    )
    .expect("Mary-O profiles");

    let ambition = resolve(ambition, PresentationEnvironment::Desktop);
    let sanic = resolve(sanic, PresentationEnvironment::Desktop);
    let mary_o = resolve(mary_o, PresentationEnvironment::Desktop);

    // Ambition and Sanic both fill the display; only Mary-O pillarboxes.
    assert_eq!(ambition.gameplay_rect, ambition.display_rect);
    assert_eq!(sanic.gameplay_rect, sanic.display_rect);
    assert!(mary_o.gameplay_rect.width() < mary_o.display_rect.width());
    assert!(
        mary_o.has_surround() && !mary_o.letterbox_rects().is_empty(),
        "a pillarboxed profile owes the display a painted surround",
    );

    // ...and Ambition and Sanic differ from each other in FRAMING, not viewport.
    assert!(ambition.soft_framing.is_none());
    assert!(sanic.soft_framing.is_some());
    assert_ne!(ambition.subject_safe_rect, sanic.subject_safe_rect);
}
