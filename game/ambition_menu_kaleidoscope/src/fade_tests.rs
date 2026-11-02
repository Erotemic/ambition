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
    let menu = app.world_mut().spawn(KaleidoscopeOpenState::default()).id();
    app.world_mut()
        .entity_mut(menu)
        .insert(ambition_menu::MenuSurface(menu));
    app.add_systems(Update, fade_kaleidoscope_materials);

    let handle = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 1.0),
            ..default()
        });
    let id = handle.id();
    app.world_mut().spawn((
        ChildOf(menu),
        KaleidoscopeFade { base_alpha: 0.8 },
        MeshMaterial3d(handle),
    ));

    let mat = |app: &App| {
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .get(id)
            .unwrap()
            .clone()
    };

    // Mid-fold: a solid stays Opaque at full base alpha (does NOT fade).
    app.world_mut()
        .get_mut::<KaleidoscopeOpenState>(menu)
        .unwrap()
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
        .get_mut::<KaleidoscopeOpenState>(menu)
        .unwrap()
        .amount = 1.0;
    app.update();
    let m = mat(&app);
    assert!((m.base_color.alpha() - 0.8).abs() < 1e-4, "open alpha");
    assert_eq!(m.alpha_mode, AlphaMode::Opaque, "open solid is Opaque");

    // Folded shut: a solid is STILL Opaque at base alpha (it pops with the fold
    // geometry + scrim rather than cross-fading).
    app.world_mut()
        .get_mut::<KaleidoscopeOpenState>(menu)
        .unwrap()
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

/// A TEXTURED plane (text glyph atlas / item icon — any material with a `base_color_texture`) must
/// STAY `Blend` when the menu is fully open, even though solid planes go `Opaque`. Pins the
/// per-element split.
#[test]
fn textured_planes_stay_blend_when_open() {
    let mut app = App::new();
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<StandardMaterial>();
    let menu = app.world_mut().spawn(KaleidoscopeOpenState::default()).id();
    app.world_mut()
        .entity_mut(menu)
        .insert(ambition_menu::MenuSurface(menu));
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
    app.world_mut().spawn((
        ChildOf(menu),
        KaleidoscopeFade { base_alpha: 1.0 },
        MeshMaterial3d(handle),
    ));

    let mat = |app: &App| {
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .get(id)
            .unwrap()
            .clone()
    };

    // Fully open: Blend, alpha = base_alpha.
    app.world_mut()
        .get_mut::<KaleidoscopeOpenState>(menu)
        .unwrap()
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
        .get_mut::<KaleidoscopeOpenState>(menu)
        .unwrap()
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

#[test]
fn each_menu_fades_only_its_own_descendants() {
    let mut app = App::new();
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<StandardMaterial>();
    app.add_systems(Update, fade_kaleidoscope_materials);
    let menus = [0.25, 0.75].map(|amount| {
        let menu = app
            .world_mut()
            .spawn(KaleidoscopeOpenState {
                amount,
                target: amount,
            })
            .id();
        app.world_mut()
            .entity_mut(menu)
            .insert(ambition_menu::MenuSurface(menu));
        menu
    });
    let handles = menus.map(|menu| {
        let intermediate = app.world_mut().spawn(ChildOf(menu)).id();
        let material = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(Handle::<Image>::default()),
                alpha_mode: AlphaMode::Blend,
                ..default()
            });
        app.world_mut().spawn((
            ChildOf(intermediate),
            KaleidoscopeFade { base_alpha: 1.0 },
            MeshMaterial3d(material.clone()),
        ));
        material
    });
    app.update();
    let alphas = |app: &App| {
        handles.each_ref().map(|handle| {
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(handle)
                .unwrap()
                .base_color
                .alpha()
        })
    };
    assert_eq!(alphas(&app), [0.25, 0.75]);
    app.world_mut()
        .get_mut::<KaleidoscopeOpenState>(menus[0])
        .unwrap()
        .amount = 0.5;
    app.update();
    assert_eq!(alphas(&app), [0.5, 0.75]);
    app.world_mut()
        .get_mut::<KaleidoscopeOpenState>(menus[1])
        .unwrap()
        .amount = 0.0;
    app.update();
    assert_eq!(alphas(&app), [0.5, 0.0]);
}

#[test]
fn a_settled_ring_does_not_stop_another_menus_fold() {
    use super::{animate_cube_ring, CubeFace, KaleidoscopeMenuConfig, MenuRing};
    let mut app = App::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_millis(16));
    app.insert_resource(time);
    app.insert_resource(KaleidoscopeMenuConfig::default());
    app.add_systems(Update, animate_cube_ring::<(), ()>);
    let menus = [1.0, 0.0].map(|amount| {
        app.world_mut()
            .spawn((
                MenuRing,
                ambition_menu::ActiveMenuPages::<(), ()>::default(),
                Transform::default(),
                KaleidoscopeOpenState {
                    amount,
                    target: 1.0,
                },
            ))
            .id()
    });
    let faces = menus.map(|menu| {
        app.world_mut()
            .spawn((
                ChildOf(menu),
                Transform::default(),
                CubeFace {
                    index: 0,
                    angle: 0.0,
                    base_translation: Vec3::ZERO,
                    base_rotation: Quat::IDENTITY,
                    base_scale: Vec3::ONE,
                    half_height: 1.0,
                },
            ))
            .id()
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<KaleidoscopeOpenState>(menus[0])
            .unwrap()
            .amount,
        1.0
    );
    let opening = app
        .world()
        .get::<KaleidoscopeOpenState>(menus[1])
        .unwrap()
        .amount;
    assert!(opening > 0.0 && opening < 1.0);
    assert_eq!(
        *app.world().get::<Transform>(faces[0]).unwrap(),
        Transform::default()
    );
    assert_ne!(
        *app.world().get::<Transform>(faces[1]).unwrap(),
        Transform::default()
    );
    app.world_mut()
        .get_mut::<KaleidoscopeOpenState>(menus[0])
        .unwrap()
        .target = 0.0;
    app.update();
    assert!(
        app.world()
            .get::<KaleidoscopeOpenState>(menus[0])
            .unwrap()
            .amount
            < 1.0
    );
    assert!(
        app.world()
            .get::<KaleidoscopeOpenState>(menus[1])
            .unwrap()
            .amount
            > opening
    );
}
