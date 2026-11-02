use super::*;
use bevy::text::{RemSize, TextLayoutInfo};
use bevy::ui::ComputedUiRenderTargetInfo;

/// The authored maximum size. The visible text can shrink to its layout box.
#[derive(Component, Clone, Copy)]
pub(super) struct MenuTextFit {
    maximum: FontSize,
    fit_height: bool,
}

impl MenuTextFit {
    pub(super) fn line(maximum: FontSize) -> Self {
        Self {
            maximum,
            fit_height: false,
        }
    }
    pub(super) fn box_size(maximum: FontSize) -> Self {
        Self {
            maximum,
            fit_height: true,
        }
    }
}

pub(super) fn apply_dynamic_text(
    mut texts: Query<
        (&crate::MenuDynamicTextContent, &mut Text),
        Changed<crate::MenuDynamicTextContent>,
    >,
) {
    for (content, mut text) in &mut texts {
        if text.0 != content.0 {
            text.0.clone_from(&content.0);
        }
    }
}

/// Use Bevy's measured width. Keep the authored size when the text fits.
pub(super) fn fit_menu_text(
    rem: Res<RemSize>,
    boxes: Query<Ref<ComputedNode>>,
    mut texts: Query<(
        &ChildOf,
        Ref<TextLayoutInfo>,
        Ref<ComputedUiRenderTargetInfo>,
        Ref<MenuTextFit>,
        &mut TextFont,
    )>,
) {
    for (parent, layout, target, fit, mut font) in &mut texts {
        let Ok(node) = boxes.get(parent.parent()) else {
            continue;
        };
        if !node.is_changed()
            && !layout.is_changed()
            && !target.is_changed()
            && !fit.is_changed()
            && !rem.is_changed()
        {
            continue;
        }
        let available = node.content_box().size().x * node.inverse_scale_factor();
        let measured = layout.size.x;
        if available <= 0.0 || measured <= 0.0 {
            continue;
        }
        let maximum = fit.maximum.eval(target.logical_size(), rem.0);
        let current = font.font_size.eval(target.logical_size(), rem.0);
        let mut ratio = available / measured;
        if fit.fit_height && layout.size.y > 0.0 {
            let height = node.content_box().size().y * node.inverse_scale_factor();
            if height <= 0.0 {
                continue;
            }
            ratio = ratio.min(height / layout.size.y);
        }
        let fitted = current * ratio;
        let wanted = if fitted >= maximum {
            fit.maximum
        } else {
            FontSize::Px(fitted.floor().max(1.0))
        };
        if font.font_size != wanted {
            font.font_size = wanted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MenuColor, MenuDynamicText, MenuDynamicTextContent, MenuPageModel, MenuTextAlign};
    use bevy::camera::{ComputedCameraValues, RenderTargetInfo};

    #[test]
    fn text_fits_its_menu_region_and_recovers_without_changing_another_menu() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::image::ImagePlugin::default(),
            bevy::input::InputPlugin,
            bevy::picking::PickingPlugin,
            bevy::picking::InteractionPlugin,
            bevy::picking::input::PointerInputPlugin,
            bevy::window::WindowPlugin::default(),
            bevy::text::TextPlugin,
            bevy::ui::UiPlugin::default(),
            BevyUiMenuRestylePlugin,
        ));
        app.init_asset::<bevy::image::TextureAtlasLayout>();
        app.world_mut().spawn((
            Camera2d,
            IsDefaultUiCamera,
            Camera {
                computed: ComputedCameraValues {
                    target_info: Some(RenderTargetInfo {
                        physical_size: UVec2::new(1280, 720),
                        scale_factor: 1.0,
                    }),
                    ..default()
                },
                ..default()
            },
        ));
        let menus = [0, 1].map(|_| {
            app.world_mut()
                .spawn(crate::MenuInteraction::<()>::default())
                .id()
        });
        let long = "SYSTEM - REBIND CONTROLS HEADING 1/32";
        let mut page = MenuPageModel::<(), ()>::new((), "System", MenuColor::BLACK);
        page.text(
            50.0,
            12.0,
            5.0,
            long,
            MenuTextAlign::Center,
            MenuColor::WHITE,
        );
        page.control(
            crate::MenuRect::new(12.0, 25.0, 44.0, 8.0),
            crate::MenuControlKind::Item,
            "Contextual\nBlink",
            Some("equipped".into()),
            false,
            false,
            Some(()),
        );
        page.dynamic_text(1, 75.0, 70.0, 4.0, MenuTextAlign::Center, MenuColor::WHITE);
        let tabs = [BevyUiMenuTabSpec::new((), "A long menu tab")];
        let roots = menus.map(|menu| {
            let mut commands = app.world_mut().commands();
            let root = spawn_bevy_ui_menu(
                &mut commands,
                &BevyUiMenuView {
                    menu,
                    tabs: &tabs,
                    active_tab: 0,
                    page: &page,
                    focused: None,
                    focused_tab: None,
                },
            );
            commands.entity(root).insert(Node {
                position_type: PositionType::Absolute,
                width: Val::Px(640.0),
                height: Val::Px(720.0),
                ..default()
            });
            root
        });
        for _ in 0..12 {
            app.update();
        }
        let mut query = app
            .world_mut()
            .query::<(Entity, &crate::MenuSurface, &Text)>();
        let headings = menus.map(|menu| {
            query
                .iter(app.world())
                .find(|(_, owner, text)| owner.0 == menu && text.0 == long)
                .unwrap()
                .0
        });
        let fits = |app: &App, text: Entity| {
            let parent = app.world().get::<ChildOf>(text).unwrap().parent();
            let node = app.world().get::<ComputedNode>(parent).unwrap();
            let layout = app.world().get::<TextLayoutInfo>(text).unwrap();
            assert!(
                layout.size.x > 0.0,
                "the real text pipeline must measure the heading"
            );
            assert!(
                layout.size.x <= node.content_box().size().x * node.inverse_scale_factor() + 0.01,
                "the text must fit in its own layout box"
            );
        };
        for text in headings {
            fits(&app, text);
        }
        let mut captions = app.world_mut().query::<(Entity, &MenuTextFit, &Text)>();
        for (entity, fit, text) in captions.iter(app.world()) {
            if text.0.is_empty() {
                continue;
            }
            fits(&app, entity);
            if fit.fit_height {
                let parent = app.world().get::<ChildOf>(entity).unwrap().parent();
                let bounds = app.world().get::<ComputedNode>(parent).unwrap();
                let layout = app.world().get::<TextLayoutInfo>(entity).unwrap();
                assert!(
                    layout.size.y
                        <= bounds.content_box().size().y * bounds.inverse_scale_factor() + 0.01
                );
            }
        }

        let first_size = app.world().get::<TextFont>(headings[0]).unwrap().font_size;
        let other_tick = app
            .world()
            .entity(headings[1])
            .get_ref::<TextFont>()
            .unwrap()
            .last_changed();
        app.world_mut().get_mut::<Node>(roots[0]).unwrap().width = Val::Px(320.0);
        for _ in 0..8 {
            app.update();
        }
        fits(&app, headings[0]);
        assert_ne!(
            app.world().get::<TextFont>(headings[0]).unwrap().font_size,
            first_size
        );
        assert_eq!(
            app.world()
                .entity(headings[1])
                .get_ref::<TextFont>()
                .unwrap()
                .last_changed(),
            other_tick
        );
        let settled_tick = app
            .world()
            .entity(headings[0])
            .get_ref::<TextFont>()
            .unwrap()
            .last_changed();
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world()
                .entity(headings[0])
                .get_ref::<TextFont>()
                .unwrap()
                .last_changed(),
            settled_tick
        );
        app.world_mut().get_mut::<Node>(roots[0]).unwrap().width = Val::Px(1280.0);
        for _ in 0..8 {
            app.update();
        }
        fits(&app, headings[0]);
        assert_eq!(
            app.world().get::<TextFont>(headings[0]).unwrap().font_size,
            FontSize::Vh(5.0)
        );

        let dynamic = app
            .world_mut()
            .query::<(Entity, &crate::MenuSurface, &MenuDynamicText)>()
            .iter(app.world())
            .find(|(_, owner, _)| owner.0 == menus[0])
            .unwrap()
            .0;
        app.world_mut()
            .get_mut::<MenuDynamicTextContent>(dynamic)
            .unwrap()
            .0 = long.into();
        for _ in 0..8 {
            app.update();
        }
        assert_eq!(app.world().get::<Text>(dynamic).unwrap().0, long);
        fits(&app, dynamic);
        let line = app.world().get::<ChildOf>(dynamic).unwrap().parent();
        let body = app.world().get::<ChildOf>(line).unwrap().parent();
        let line_center = app
            .world()
            .get::<UiGlobalTransform>(line)
            .unwrap()
            .translation;
        let body_center = app
            .world()
            .get::<UiGlobalTransform>(body)
            .unwrap()
            .translation;
        let body_width = app.world().get::<ComputedNode>(body).unwrap().size().x;
        assert!((line_center.x - body_center.x - body_width * 0.25).abs() < 1.0);
        let text_center = app
            .world()
            .get::<UiGlobalTransform>(dynamic)
            .unwrap()
            .translation;
        assert!((text_center.x - line_center.x).abs() < 1.0);
    }
}
