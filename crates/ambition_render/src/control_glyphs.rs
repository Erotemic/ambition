//! Load published control pictures through the selected sprite content.

use ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog;
use ambition_sprite_sheet::SpriteContent;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// The render-owned image inputs for a control presenter.
#[derive(SystemParam)]
pub struct ControlGlyphImages<'w> {
    catalog: Option<Res<'w, Platformer2dAssetCatalog>>,
    content: Option<Res<'w, SpriteContent>>,
    server: Option<Res<'w, AssetServer>>,
}

impl ControlGlyphImages<'_> {
    pub fn is_changed(&self) -> bool {
        self.catalog
            .as_ref()
            .is_some_and(|catalog| catalog.is_changed())
            || self
                .content
                .as_ref()
                .is_some_and(|content| content.is_changed())
    }

    /// Missing art leaves the caller's control label visible.
    pub fn load(&self, path: &str) -> Option<Handle<Image>> {
        let path = self.catalog.as_ref()?.try_image_path_for_load(path)?;
        Some(ambition_sprite_sheet::game_assets::load_sheet_image(
            self.content.as_deref(),
            self.server.as_deref()?,
            "control glyph",
            path,
        ))
    }

    /// Use the load state because the renderer can release the CPU image.
    pub fn is_ready(&self, image: &Handle<Image>) -> bool {
        self.server
            .as_ref()
            .is_some_and(|server| server.is_loaded_with_dependencies(image.id()))
    }
}

/// What presses a drawn control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GlyphPressSource {
    /// A key on any keyboard.
    Key(KeyCode),
    /// A button on any connected pad.
    Button(GamepadButton),
    /// The surface that drew it writes [`GlyphPress::held`] itself (a touch
    /// button knows its own hold).
    #[default]
    Owner,
}

/// A drawn control that presses with its control: while held, the picture
/// sinks, shrinks and darkens; on release it springs back. Any surface that
/// draws a control's picture adds this beside it; [`GlyphPressPlugin`] does
/// the rest. A UI picture moves by its `UiTransform`, so layout never sees
/// it; a world picture only scales, because its placement writes its
/// translation every frame.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct GlyphPress {
    pub source: GlyphPressSource,
    /// Held now. Written from the device for a key or button source.
    pub held: bool,
    /// 0 at rest, 1 fully pressed.
    depth: f32,
}

impl GlyphPress {
    pub fn key(key: KeyCode) -> Self {
        Self { source: GlyphPressSource::Key(key), ..default() }
    }
    pub fn button(button: GamepadButton) -> Self {
        Self { source: GlyphPressSource::Button(button), ..default() }
    }
    /// Pressed by its surface, which writes [`Self::held`].
    pub fn owned() -> Self {
        Self::default()
    }
    pub fn depth(&self) -> f32 {
        self.depth
    }
}

/// How a pressed picture looks at full depth.
const PRESSED_SCALE: f32 = 0.86;
const PRESSED_DROP_PX: f32 = 3.0;
const PRESSED_SHADE: f32 = 0.72;
/// Per-second rates: the press lands in a frame or two, the release eases out.
const PRESS_RATE: f32 = 40.0;
const RELEASE_RATE: f32 = 14.0;

/// Read each device-sourced glyph's hold.
pub fn hold_glyphs_from_devices(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    pads: Query<&Gamepad>,
    mut glyphs: Query<&mut GlyphPress>,
) {
    for mut glyph in &mut glyphs {
        let held = match glyph.source {
            GlyphPressSource::Key(key) => keys.as_ref().is_some_and(|keys| keys.pressed(key)),
            GlyphPressSource::Button(button) => pads.iter().any(|pad| pad.pressed(button)),
            GlyphPressSource::Owner => continue,
        };
        if glyph.held != held {
            glyph.held = held;
        }
    }
}

/// Ease each glyph toward its hold and draw the depth. Real time, so a
/// picture presses under the pause menu too.
#[allow(clippy::type_complexity)]
pub fn animate_glyph_presses(
    // Absent in a composition with no clock; then nothing animates.
    time: Option<Res<Time<Real>>>,
    mut glyphs: Query<(
        &mut GlyphPress,
        Option<&mut UiTransform>,
        Option<&mut ImageNode>,
        Option<&mut Transform>,
        Has<Node>,
    )>,
) {
    let Some(time) = time else { return };
    let dt = time.delta_secs();
    for (mut glyph, ui, image, transform, is_ui) in &mut glyphs {
        let target = if glyph.held { 1.0 } else { 0.0 };
        if glyph.depth == target {
            continue;
        }
        let rate = if glyph.held { PRESS_RATE } else { RELEASE_RATE };
        let mut depth = glyph.depth + (target - glyph.depth) * (1.0 - (-rate * dt).exp());
        if (depth - target).abs() < 0.01 {
            depth = target;
        }
        glyph.depth = depth;
        let scale = 1.0 + (PRESSED_SCALE - 1.0) * depth;
        if is_ui {
            if let Some(mut ui) = ui {
                ui.scale = Vec2::splat(scale);
                ui.translation = Val2::px(0.0, PRESSED_DROP_PX * depth);
            }
            if let Some(mut image) = image {
                let shade = 1.0 + (PRESSED_SHADE - 1.0) * depth;
                let alpha = image.color.alpha();
                image.color = Color::srgba(shade, shade, shade, alpha);
            }
        } else if let Some(mut transform) = transform {
            transform.scale = Vec3::new(scale, scale, 1.0);
        }
    }
}

/// Installs [`GlyphPress`]. Idempotent: every surface that draws pictures may
/// add it.
pub struct GlyphPressPlugin;

impl Plugin for GlyphPressPlugin {
    fn is_unique(&self) -> bool {
        false
    }

    fn build(&self, app: &mut App) {
        #[derive(Resource)]
        struct Installed;
        if app.world().contains_resource::<Installed>() {
            return;
        }
        app.insert_resource(Installed).add_systems(
            Update,
            (hold_glyphs_from_devices, animate_glyph_presses)
                .chain()
                .after(crate::rendering::label_layout::WorldLabelLayoutSet),
        );
    }
}

impl GlyphPress {
    /// The press of a bound control's picture; `None` for a control no device
    /// reports (a chord).
    pub fn of(control: &ambition_input::PhysicalControl) -> Option<Self> {
        match control {
            ambition_input::PhysicalControl::Key(key) => Some(Self::key(*key)),
            ambition_input::PhysicalControl::Button(button) => Some(Self::button(*button)),
            ambition_input::PhysicalControl::Axis(_) | ambition_input::PhysicalControl::Touch(_) | ambition_input::PhysicalControl::TouchDirection(_) | ambition_input::PhysicalControl::Other(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A held key presses its picture down and in; released, it springs back
    /// to rest exactly.
    #[test]
    fn a_held_control_presses_its_picture_and_the_release_restores_it() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(1.0 / 60.0)))
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(GlyphPressPlugin);
        let glyph = app.world_mut().spawn((Node::default(), ImageNode::default(), GlyphPress::key(KeyCode::KeyZ))).id();
        let ui = |app: &App| *app.world().get::<UiTransform>(glyph).unwrap();
        app.update();
        assert_eq!(ui(&app), UiTransform::IDENTITY);

        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyZ);
        for _ in 0..6 {
            app.update();
        }
        let pressed = ui(&app);
        assert!(pressed.scale.x < 0.9 && pressed.translation.y != Val::ZERO, "{pressed:?}");
        assert!(app.world().get::<ImageNode>(glyph).unwrap().color.to_srgba().red < 0.8);

        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyZ);
        for _ in 0..60 {
            app.update();
        }
        assert_eq!(ui(&app).scale, Vec2::ONE);
        assert_eq!(app.world().get::<GlyphPress>(glyph).unwrap().depth(), 0.0);
    }
}
