use super::{setup_cube, KaleidoscopeMenuConfig, KaleidoscopePauseCamera};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;

/// Trimming Bevy's defaults left Camera3d on TonyMcMapface without its LUT, producing Bevy's
/// magenta diagnostic output. The cube therefore opts out explicitly: no hidden LUT dependency
/// or LUT-backed transform.
#[test]
fn kaleidoscope_camera_uses_the_cheap_unlit_render_contract() {
    let mut app = App::new();
    app.insert_resource(KaleidoscopeMenuConfig::default());
    app.world_mut().spawn(ambition_menu::ActiveMenuPages::<(), ()>::default());
    app.add_systems(Startup, setup_cube::<(), ()>);
    app.update();

    let world = app.world_mut();
    let mut cameras =
        world.query_filtered::<(&Tonemapping, &Msaa), With<KaleidoscopePauseCamera>>();
    let (tonemapping, msaa) = cameras
        .single(world)
        .expect("setup_cube should spawn exactly one kaleidoscope pause camera");

    assert_eq!(
        *tonemapping,
        Tonemapping::None,
        "the unlit kaleidoscope UI must not inherit Camera3d's LUT-backed tonemapper",
    );
    assert_eq!(
        *msaa,
        Msaa::Sample4,
        "the failed runtime MSAA experiment must not silently rewrite the established camera contract",
    );
}

#[test]
fn setup_creates_a_camera_for_each_published_menu() {
    let mut app = App::new();
    app.insert_resource(KaleidoscopeMenuConfig::default());
    let menus = [0, 1].map(|_| {
        app.world_mut()
            .spawn(ambition_menu::ActiveMenuPages::<(), ()>::default())
            .id()
    });
    app.add_systems(Startup, setup_cube::<(), ()>);
    app.update();
    let mut cameras = app
        .world_mut()
        .query_filtered::<&ambition_menu::MenuSurface, With<KaleidoscopePauseCamera>>();
    let owners: std::collections::HashSet<_> =
        cameras.iter(app.world()).map(|surface| surface.0).collect();
    assert_eq!(owners, menus.into_iter().collect());
    for menu in menus {
        assert!(app
            .world()
            .get::<super::KaleidoscopeOpenState>(menu)
            .is_some());
    }
}
