//! The cube-menu scrim (dimming backdrop): spawn, camera retarget, and the
//! open/close alpha fade.
//!
//! Split out of the kaleidoscope menu host (2026-06-15).

use super::*;

/// Spawn the readability dim-scrim node (full-screen, starts fully transparent).
///
/// The scrim DIMS THE WORLD, so it must render BEHIND the order-8 cube. Since the
/// default UI camera is now the order-9 [`FrontHudCamera`] (which draws in front of
/// the cube), the scrim is explicitly retargeted onto the order-0 main camera via
/// [`retarget_kaleidoscope_scrim`] (the `MainCameraEntity` resource isn't guaranteed to
/// exist yet at this Startup point, so the target is attached from an Update guard).
/// [`fade_kaleidoscope_scrim`] drives its alpha.
pub(crate) fn spawn_kaleidoscope_scrim(mut commands: Commands) {
    commands.spawn((
        KaleidoscopeScrim,
        Name::new("Cube readability scrim"),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        // Never eat clicks meant for the world/cube; purely a visual dimmer.
        GlobalZIndex(-1),
        Pickable::IGNORE,
    ));
}

/// Retarget the dim-scrim onto the order-0 main camera so it renders BEHIND the cube.
///
/// The default UI camera is the order-9 front HUD camera (so the HUD draws in front
/// of the cube); without this retarget the scrim would inherit that default and dim
/// the cube itself. Runs once, as soon as both the scrim and the `MainCameraEntity`
/// resource exist (Startup ordering between them is not guaranteed, so this Update
/// guard does it on the first frame both are present). `Option<Res<_>>` keeps it
/// B0002-safe and never panics on an uninserted resource.
pub(crate) fn retarget_kaleidoscope_scrim(
    mut commands: Commands,
    main_camera: Option<Res<ambition_gameplay_core::session::camera_layers::MainCameraEntity>>,
    scrim: Query<Entity, (With<KaleidoscopeScrim>, Without<UiTargetCamera>)>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let Some(main_camera) = main_camera else {
        return;
    };
    let mut any = false;
    for entity in &scrim {
        commands
            .entity(entity)
            .insert(UiTargetCamera(main_camera.0));
        any = true;
    }
    if any {
        *done = true;
    }
}

/// Fade the dim-scrim's alpha with the cube's eased open `amount`, so the world
/// dims in/out exactly with the fold. Fully transparent when the cube is shut.
pub(crate) fn fade_kaleidoscope_scrim(
    open_state: Res<ambition_menu_kaleidoscope::KaleidoscopeOpenState>,
    mut scrim: Query<&mut BackgroundColor, With<KaleidoscopeScrim>>,
) {
    let alpha = open_state.amount.clamp(0.0, 1.0) * SCRIM_PEAK_ALPHA;
    for mut bg in &mut scrim {
        bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
    }
}
