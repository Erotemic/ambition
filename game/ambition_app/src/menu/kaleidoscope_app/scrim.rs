//! The cube-menu scrim (dimming backdrop): its display camera, spawn, target,
//! and the open/close alpha fade.
//!
//! # A FULL-SCREEN SCRIM IS A DISPLAY FACT, NOT A VIEW FACT
//!
//! The scrim dims THE WORLD so the cube's text reads, and it must draw BEHIND
//! the order-8 cube. The default UI camera is the order-9 front HUD camera, so
//! it cannot simply inherit that; it needs a target with a lower order.
//!
//! - under any fixed-aspect presentation profile the gameplay camera already
//!   carries a `Camera::viewport` (`apply_gameplay_camera_viewport`), so the
//!   "full-screen" scrim covered the gameplay rectangle and left the surround
//!   bars undimmed;
//! - under a split layout there is one gameplay camera per local view, and the
//!   resource is whichever rig was inserted last — so the scrim would dim ONE
//!   PANE.
//!
//! So the scrim owns [`KaleidoscopeScrimCamera`]: a full-screen, viewport-free
//! `Camera2d` that sits one order behind the cube and renders no sprites at all.
//! It answers "the whole display" by construction, and it keeps answering it
//! however many gameplay views the composition grows.

use super::*;

/// The scrim's own UI camera — full-screen, one order behind the cube.
///
/// it deliberately carries `RenderLayers::none()`. Node→camera resolution
/// in `bevy_ui` is by `IsDefaultUiCamera` / `UiTargetCamera` and is independent
/// of sprite render layers, so the scrim still renders here while nothing in the
/// world can. That is the same trick the front HUD camera uses to avoid
/// re-drawing the world over the cube, one layer over.
#[derive(Component)]
pub(crate) struct KaleidoscopeScrimCamera;

/// Order fallback when the cube's config is not installed: one behind the cube's
/// own default (`KaleidoscopeMenuConfig::camera_order`, 8).
const SCRIM_CAMERA_ORDER_FALLBACK: isize = 7;

/// Spawn the scrim's display camera and the readability dim-scrim node
/// (full-screen, starts fully transparent).
///
/// The scrim is targeted at its own camera HERE rather than from an `Update`
/// guard, because the camera is spawned in the same call — there is no ordering
/// question left to defer. [`retarget_kaleidoscope_scrim`] stays as the repair
/// path for a scrim spawned by anything else.
///
/// [`fade_kaleidoscope_scrim`] drives the alpha.
pub(crate) fn spawn_kaleidoscope_scrim(
    mut commands: Commands,
    // the cube's order is a knob (`KaleidoscopeMenuConfig::camera_order`), and
    // "behind the cube" is the whole requirement — so the scrim's order is
    // DERIVED from it rather than hardcoded next to it and left to drift. The
    // config is inserted at plugin build time, so it is here by `Startup`;
    // `Option` keeps a fixture that skips the cube plugin from panicking.
    config: Option<Res<KaleidoscopeMenuConfig>>,
) {
    let order = config
        .map(|config| config.camera_order - 1)
        .unwrap_or(SCRIM_CAMERA_ORDER_FALLBACK);
    let camera = commands
        .spawn((
            KaleidoscopeScrimCamera,
            Camera2d,
            Camera {
                order,
                // Clearing here would wipe the world the scrim exists to dim.
                clear_color: ClearColorConfig::None,
                // never a viewport. The absence is the feature: an
                // unclipped camera is what makes this the DISPLAY rect rather
                // than a gameplay pane.
                ..default()
            },
            bevy::camera::visibility::RenderLayers::none(),
            Name::new("Cube scrim display camera"),
        ))
        .id();

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
        UiTargetCamera(camera),
        // Never eat clicks meant for the world/cube; purely a visual dimmer.
        GlobalZIndex(-1),
        Pickable::IGNORE,
    ));
}

/// Repair path: give any untargeted scrim the scrim camera.
///
/// [`spawn_kaleidoscope_scrim`] already targets the node it spawns, so in the
/// shipped composition this matches nothing. It stays because the scrim is a
/// marker anything may spawn (fixtures do), and an untargeted full-screen node
/// silently inherits the order-9 front HUD camera — which dims the cube instead
/// of the world, the exact failure this module exists to prevent.
pub(crate) fn retarget_kaleidoscope_scrim(
    mut commands: Commands,
    scrim_camera: Query<Entity, With<KaleidoscopeScrimCamera>>,
    scrim: Query<Entity, (With<KaleidoscopeScrim>, Without<UiTargetCamera>)>,
) {
    if scrim.is_empty() {
        return;
    }
    let Ok(camera) = scrim_camera.single() else {
        // No camera, or somehow several: targeting an arbitrary one would be the
        // guess this module replaced.
        return;
    };
    for entity in &scrim {
        commands.entity(entity).insert(UiTargetCamera(camera));
    }
}

/// Fade the dim-scrim's alpha with the cube's eased open `amount`, so the world
/// dims in/out exactly with the fold. Fully transparent when the cube is shut.
pub(crate) fn fade_kaleidoscope_scrim(
    open_state: Res<ambition_menu_kaleidoscope::KaleidoscopeOpenState>,
    mut scrim: Query<&mut BackgroundColor, With<KaleidoscopeScrim>>,
) {
    let alpha = open_state.amount.clamp(0.0, 1.0) * SCRIM_PEAK_ALPHA;
    let want = Color::srgba(0.0, 0.0, 0.0, alpha);
    for mut bg in &mut scrim {
        // Compared before writing: `BackgroundColor` is change-detected, and the
        // fold sits settled for most of the frames this system runs — an
        // unconditional assignment dirties the UI node every one of them.
        if bg.0 != want {
            bg.0 = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE SCRIM TARGETS THE DISPLAY, NOT A GAMEPLAY PANE.
    ///
    /// The assertion is on the OUTPUT: whatever camera the scrim ends up
    /// targeting must be unclipped and must not be a gameplay rig.
    #[test]
    fn the_scrim_targets_an_unclipped_display_camera() {
        let mut app = App::new();
        let config = KaleidoscopeMenuConfig::default();
        let cube_order = config.camera_order;
        app.insert_resource(config);

        // The poison: a gameplay camera confined to one rectangle.
        let gameplay_camera = app
            .world_mut()
            .spawn((
                Camera2d,
                ambition_platformer2d::platformer::camera_layers::MainCamera,
                Camera {
                    viewport: Some(bevy::camera::Viewport {
                        physical_position: UVec2::new(0, 60),
                        physical_size: UVec2::new(640, 360),
                        ..default()
                    }),
                    ..default()
                },
            ))
            .id();

        app.add_systems(Startup, spawn_kaleidoscope_scrim);
        app.update();

        let scrim = {
            let mut q = app
                .world_mut()
                .query_filtered::<&UiTargetCamera, With<KaleidoscopeScrim>>();
            let found: Vec<Entity> = q.iter(app.world()).map(|target| target.0).collect();
            assert_eq!(
                found.len(),
                1,
                "expected exactly one targeted scrim node, found {}",
                found.len()
            );
            found[0]
        };

        assert_ne!(
            scrim, gameplay_camera,
            "the scrim targeted the gameplay camera, so a full-screen dimmer is laid \
             out against one viewport rectangle"
        );

        let target = app.world().entity(scrim);
        assert!(
            target.contains::<KaleidoscopeScrimCamera>(),
            "the scrim's target is not the display camera it owns"
        );
        let camera = target
            .get::<Camera>()
            .expect("the scrim's target must be a camera");
        assert!(
            camera.viewport.is_none(),
            "the scrim's target camera is clipped to a viewport, so the scrim cannot \
             cover the display"
        );
        assert!(
            camera.order < cube_order,
            "the scrim must render BEHIND the cube (order {} vs the cube's {cube_order}); \
             in front of it, it dims the menu it exists to make readable",
            camera.order
        );
    }
}
