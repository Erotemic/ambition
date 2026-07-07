use super::{fade_kaleidoscope_materials, KaleidoscopeFade, KaleidoscopeOpenState};
use bevy::prelude::*;

/// Fix 3 contract: a SOLID (untextured) plane is `Opaque` at FULL alpha at ALL
/// open amounts — it never fades. Drawing solids Opaque the whole time (not just
/// when settled) keeps the per-face depth bands resolved by the GPU depth test, so
/// coplanar panels/lines never z-fight as the cube folds open/closed (the
/// open/close flicker). The fold geometry + the scrim carry the transition.
#[test]
fn amount_drives_material_alpha() {
    let mut app = App::new();
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<StandardMaterial>();
    app.init_resource::<KaleidoscopeOpenState>();
    app.add_systems(Update, fade_kaleidoscope_materials);

    let handle = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 1.0),
            ..default()
        });
    let id = handle.id();
    app.world_mut()
        .spawn((KaleidoscopeFade { base_alpha: 0.8 }, MeshMaterial3d(handle)));

    let mat = |app: &App| {
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .get(id)
            .unwrap()
            .clone()
    };

    // Mid-fold: a solid stays Opaque at full base alpha (does NOT fade).
    app.world_mut()
        .resource_mut::<KaleidoscopeOpenState>()
        .amount = 0.5;
    app.update();
    let m = mat(&app);
    assert!(
        (m.base_color.alpha() - 0.8).abs() < 1e-4,
        "solid keeps base alpha mid-fold"
    );
    assert_eq!(
        m.alpha_mode,
        AlphaMode::Opaque,
        "solid is Opaque at every amount (no z-fight during the fold)"
    );

    // Fully open: still Opaque at base_alpha.
    app.world_mut()
        .resource_mut::<KaleidoscopeOpenState>()
        .amount = 1.0;
    app.update();
    let m = mat(&app);
    assert!((m.base_color.alpha() - 0.8).abs() < 1e-4, "open alpha");
    assert_eq!(m.alpha_mode, AlphaMode::Opaque, "open solid is Opaque");

    // Folded shut: a solid is STILL Opaque at base alpha (it pops with the fold
    // geometry + scrim rather than cross-fading).
    app.world_mut()
        .resource_mut::<KaleidoscopeOpenState>()
        .amount = 0.0;
    app.update();
    let m = mat(&app);
    assert!(
        (m.base_color.alpha() - 0.8).abs() < 1e-4,
        "solid keeps base alpha when shut = {}",
        m.base_color.alpha()
    );
    assert_eq!(m.alpha_mode, AlphaMode::Opaque, "shut solid is Opaque");
}

/// A TEXTURED plane (text glyph atlas / item icon — any material with a
/// `base_color_texture`) must STAY `Blend` when the menu is fully open, even
/// though solid planes go `Opaque`. Drawing a mostly-transparent texture Opaque
/// renders its transparent texels as the base-colour box — the "text is just
/// squares" / "icons look weird" regression. Pins the per-element split.
#[test]
fn textured_planes_stay_blend_when_open() {
    let mut app = App::new();
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<StandardMaterial>();
    app.init_resource::<KaleidoscopeOpenState>();
    app.add_systems(Update, fade_kaleidoscope_materials);

    let handle = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(Handle::<Image>::default()),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
    let id = handle.id();
    app.world_mut()
        .spawn((KaleidoscopeFade { base_alpha: 1.0 }, MeshMaterial3d(handle)));

    let mat = |app: &App| {
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .get(id)
            .unwrap()
            .clone()
    };

    // Fully open: Blend, alpha = base_alpha.
    app.world_mut()
        .resource_mut::<KaleidoscopeOpenState>()
        .amount = 1.0;
    app.update();
    let m = mat(&app);
    assert_eq!(
            m.alpha_mode,
            AlphaMode::Blend,
            "textured plane must stay Blend when open (Opaque would draw transparent texels as squares)"
        );
    assert!(
        (m.base_color.alpha() - 1.0).abs() < 1e-4,
        "open textured alpha"
    );

    // Fix 3: a textured plane cross-fades — its alpha tracks `amount` (so text /
    // icons fade in/out with the fold even though solids do not).
    app.world_mut()
        .resource_mut::<KaleidoscopeOpenState>()
        .amount = 0.5;
    app.update();
    let m = mat(&app);
    assert_eq!(
        m.alpha_mode,
        AlphaMode::Blend,
        "textured stays Blend mid-fold"
    );
    assert!(
        (m.base_color.alpha() - 0.5).abs() < 1e-4,
        "textured alpha tracks amount = {}",
        m.base_color.alpha()
    );
}
