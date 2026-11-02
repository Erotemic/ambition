//! The cube-menu scrim (dimming backdrop): its display camera, spawn, target,
//! and the open/close alpha fade.
//!
//! Each scrim node belongs to one menu. A shared display camera draws these
//! nodes behind the cube cameras. Node placement must follow the menu region.

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

/// Create a display camera and one scrim for each live menu.
pub(crate) fn spawn_kaleidoscope_scrim(
    mut commands: Commands,
    config: Option<Res<KaleidoscopeMenuConfig>>,
    menus: Query<Entity, With<ambition_platformer2d::inventory_ui::InventoryMenu>>,
    cameras: Query<Entity, With<KaleidoscopeScrimCamera>>,
    scrims: Query<&ambition_platformer2d::menu::MenuSurface, With<KaleidoscopeScrim>>,
) {
    let order = config
        .map(|config| config.camera_order - 1)
        .unwrap_or(SCRIM_CAMERA_ORDER_FALLBACK);
    let camera = if let Ok(camera) = cameras.single() {
        camera
    } else if cameras.is_empty() {
        commands
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
            .id()
    } else {
        return;
    };

    for menu in &menus {
        if scrims.iter().any(|owner| owner.0 == menu) {
            continue;
        }
        commands.spawn((
            ambition_platformer2d::menu::MenuSurface(menu),
            ambition_platformer2d::menu::render::MenuPresentationOf(menu),
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
}

/// Fade the dim-scrim's alpha with the cube's eased open `amount`, so the world
/// dims in/out exactly with the fold. Fully transparent when the cube is shut.
pub(crate) fn fade_kaleidoscope_scrim(
    open_states: Query<&ambition_menu_kaleidoscope::KaleidoscopeOpenState>,
    mut scrims: Query<
        (
            &ambition_platformer2d::menu::MenuSurface,
            &mut BackgroundColor,
        ),
        With<KaleidoscopeScrim>,
    >,
) {
    for (owner, mut background) in &mut scrims {
        let amount = open_states.get(owner.0).map_or(0.0, |state| state.amount);
        let want = Color::srgba(0.0, 0.0, 0.0, amount.clamp(0.0, 1.0) * SCRIM_PEAK_ALPHA);
        if background.0 != want {
            background.0 = want;
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

        crate::menu::test_support::spawn_inventory_menu(&mut app, Default::default());
        app.add_systems(Update, spawn_kaleidoscope_scrim);
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

    #[test]
    fn late_menus_get_scrims_and_retire_without_removing_the_display_camera() {
        use ambition_platformer2d::input::ParticipantId;
        use ambition_platformer2d::inventory_ui::InventoryMenu;
        use ambition_platformer2d::menu::MenuSurface;

        let mut app = App::new();
        app.add_systems(Update, spawn_kaleidoscope_scrim);
        app.update();
        let camera = app
            .world_mut()
            .query_filtered::<Entity, With<KaleidoscopeScrimCamera>>()
            .single(app.world())
            .unwrap();
        let first = app
            .world_mut()
            .spawn(InventoryMenu {
                owner: ParticipantId(0),
            })
            .id();
        app.update();
        let second = app
            .world_mut()
            .spawn(InventoryMenu {
                owner: ParticipantId(1),
            })
            .id();
        app.update();
        app.update();
        let mut query = app
            .world_mut()
            .query_filtered::<(Entity, &MenuSurface, &UiTargetCamera), With<KaleidoscopeScrim>>();
        let scrims: std::collections::HashMap<_, _> = query
            .iter(app.world())
            .map(|(entity, owner, target)| {
                assert_eq!(target.0, camera);
                (owner.0, entity)
            })
            .collect();
        assert_eq!(scrims.len(), 2);
        app.world_mut().despawn(second);
        assert!(app.world().get_entity(scrims[&second]).is_err());
        assert!(app.world().get_entity(scrims[&first]).is_ok());
        assert!(app.world().get_entity(camera).is_ok());
        app.update();
        assert_eq!(query.iter(app.world()).count(), 1);
    }
}
