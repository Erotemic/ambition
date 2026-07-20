use super::{setup_cube, KaleidoscopeMenuConfig, KaleidoscopePauseCamera};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;

/// Regression for 1fef3879 and the follow-up performance investigation.
///
/// Trimming Bevy's defaults left Camera3d on TonyMcMapface without its LUT,
/// producing Bevy's magenta diagnostic output. Restoring the LUT fixed color but
/// added a measurable full-screen cost to this otherwise-unlit UI scene. The cube
/// therefore opts out explicitly: no hidden LUT dependency or LUT-backed transform.
#[test]
fn kaleidoscope_camera_uses_the_cheap_unlit_render_contract() {
    let mut app = App::new();
    app.insert_resource(KaleidoscopeMenuConfig::default());
    app.add_systems(Startup, setup_cube);
    app.update();

    let world = app.world_mut();
    let mut cameras = world.query_filtered::<
        (&Tonemapping, &Msaa),
        With<KaleidoscopePauseCamera>,
    >();
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
