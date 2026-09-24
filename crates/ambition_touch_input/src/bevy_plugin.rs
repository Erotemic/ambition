//! Bevy wiring for touch input: the touch HUD's spawn and visibility
//! lifecycle, and the collect step that turns joystick and button UI state
//! into the virtual device's `MobileTouchState`.
//!
//! This is the crate's only ECS surface. `layout` computes where the controls
//! sit, `state` holds what they do, and `virtual_device` exposes that state to
//! leafwing as bindable input kinds on the persistent participant.

use std::borrow::Cow;

use bevy::input::mouse::MouseButton;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use virtual_joystick::*;

use super::layout::{
    movement_joystick_layout, touch_action_at_position, touch_action_layout, TouchActionButton,
    ACTION_BEZEL_H, ACTION_BEZEL_W, ACTION_CLUSTER_H, ACTION_CLUSTER_MARGIN, ACTION_CLUSTER_W,
    MENU_ROW_MARGIN, MENU_ROW_W,
};
use super::menu_bridge::fold_touch_gestures;
use super::state::TouchInputState;
use ambition_input::{ControlFrame, KeyboardPreset, Platformer2dInputActionMonolith};
use ambition_render::ui_fonts::{UiFontWeight, UiFonts};
use ambition_sim_view::{ControlContextKind, ControlPrompt, ControlSlot};
use ambition_ui_nav::DragScrollState;

/// Global z-band for the on-screen touch HUD (joystick, action and menu
/// buttons, bezel).
///
/// The HUD must render above every menu overlay and win bevy_ui picking over
/// them. Then the joystick keeps receiving drags (which feed the `MenuStick`
/// binding) and the action and Back buttons stay tappable while a menu is open.
/// Menu overlays use much lower values (item grid `ZIndex(62)`, pause
/// `ZIndex(50)`, map `ZIndex(60)`, worst-case grid `GlobalZIndex(1000)`).
/// `GlobalZIndex` sets a global stacking context, and picking uses the same
/// order, so a full-screen menu scrim cannot swallow HUD input.
pub const TOUCH_HUD_Z: i32 = 5000;

/// The joystick crate's plugin and root marker, re-exported.
///
/// A composer that installs only [`crate::placement::TouchPresentationPlugin`]
/// still needs the joystick so that the discovery step finds it.
pub use virtual_joystick::{VirtualJoystickNode, VirtualJoystickPlugin};

/// Joystick id for the generic `virtual_joystick` plugin: Move (left stick)
/// and Aim (right stick).
#[derive(Default, Debug, Reflect, Hash, Clone, PartialEq, Eq)]
pub enum MobileStick {
    #[default]
    Move,
    Aim,
}

/// Live touch-input state. Updated each frame from the stick messages and
/// button state, then published as leafwing virtual-device controls.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct MobileTouchState(pub TouchInputState);

/// Last non-control touch position, used for menu drag scrolling.
///
/// Button `Interaction` covers taps on rows. This state is only for
/// whole-panel gestures, such as a drag to scroll a menu while another finger
/// is on the stick. Stick menu navigation goes through the `MenuStick` binding.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct MenuTouchGestureState {
    pub(super) drag_scroll: DragScrollState,
}

/// Visibility toggle for the on-screen touch HUD. `true` shows it.
///
/// This does not disable the virtual touch device. Touch input exists if and
/// only if `TouchControlsPlugin` is installed. An untouched overlay, hidden or
/// not, publishes neutral controls and cannot override keyboard or gamepad
/// input. The settings menu ("Touch Overlay" row) sets it; there is no hotkey.
/// Defaults to `true`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TouchControlsVisible(pub bool);

impl Default for TouchControlsVisible {
    fn default() -> Self {
        // The fold path is activity-gated, so an idle HUD does not override
        // keyboard input.
        Self(true)
    }
}

/// Marker on every touch UI root, so the visibility sync sets `Visibility`
/// on all of them in one query.
#[derive(Component)]
pub struct MobileTouchUiRoot;

/// Which resolved control rectangle a root `Node` follows.
///
/// Every root is placed from [`TouchControlPlacement`], so the drawn control,
/// its touch region, and the reserved layout space are the same rectangle.
///
/// [`TouchControlPlacement`]: crate::placement::TouchControlPlacement
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchSurface {
    Movement,
    ActionBezel,
    ActionCluster,
    MenuRow,
}

/// Place every touch root at its resolved rectangle, and scale the action
/// diamond's buttons with it.
///
/// Uses absolute pixels, not anchors: the resolver already includes the safe
/// area and reserved surround, so a second inset here could disagree.
pub fn apply_touch_control_placement(
    placement: Res<crate::placement::TouchControlPlacement>,
    mut surfaces: Query<(&TouchSurface, &mut Node)>,
    mut buttons: Query<(&TouchActionButton, &mut Node), Without<TouchSurface>>,
    mut labels: Query<(&mut TextFont, &TouchActionLabel)>,
) {
    for (surface, mut node) in &mut surfaces {
        let rect = match surface {
            TouchSurface::Movement => placement.movement,
            TouchSurface::ActionBezel => placement.action_bezel,
            TouchSurface::ActionCluster => placement.action_cluster,
            TouchSurface::MenuRow => placement.menu_row,
        };
        let Some(rect) = rect else {
            // No footprint published: the controls are hidden. Use
            // `Display::None`, not a zero rect. A zero-size node still lays
            // out, and its absolute children (stick art, glyphs, labels) would
            // draw at the screen's top-left corner.
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        set_node_rect(&mut node, rect.min, rect.size());
    }

    let scale = placement.action_scale;
    let layout = touch_action_layout();
    for (action, mut node) in &mut buttons {
        let Some(spec) = layout.iter().find(|spec| spec.action == *action) else {
            // Menu-row buttons are laid out by flex inside a fixed-size row.
            continue;
        };
        node.left = Val::Px(spec.left * scale);
        node.top = Val::Px(spec.top * scale);
        node.width = Val::Px(spec.size * scale);
        node.height = Val::Px(spec.size * scale);
        node.border_radius = BorderRadius::all(Val::Px(spec.size * 0.5 * scale));
    }
    for (mut font, label) in &mut labels {
        let Some(spec) = layout.iter().find(|spec| spec.action == label.0) else {
            continue;
        };
        let scaled = spec.font_size * scale;
        // Compare before write: a `Mut` deref marks the component changed.
        // This is the only writer of touch-label sizes, so every value is `Px`.
        if !matches!(font.font_size, FontSize::Px(px) if (px - scaled).abs() <= f32::EPSILON) {
            font.font_size = FontSize::Px(scaled);
        }
    }
}

fn set_node_rect(node: &mut Node, min: Vec2, size: Vec2) {
    node.position_type = PositionType::Absolute;
    node.left = Val::Px(min.x);
    node.top = Val::Px(min.y);
    node.right = Val::Auto;
    node.bottom = Val::Auto;
    node.width = Val::Px(size.x);
    node.height = Val::Px(size.y);
}

/// Installs the on-screen touch joystick and action-button overlay, and the
/// systems that feed touch input into the `ControlFrame` and
/// `MenuControlFrame` seams.
///
/// Touch input exists if and only if this plugin is installed. To remove
/// touch, remove the `add_plugins(TouchControlsPlugin)` line. The
/// `touch_controls_visible` setting only shows or hides the overlay (see
/// [`TouchControlsVisible`]).
pub struct TouchControlsPlugin;

impl Plugin for TouchControlsPlugin {
    fn build(&self, app: &mut App) {
        // This overlay spawns text and runs `.after(UiFontsLoaded)`. An empty
        // set makes that ordering do nothing, so install the plugin that fills it.
        ambition_render::ui_fonts::UiFontsPlugin::ensure_installed(app);
        use leafwing_input_manager::plugin::{CentralInputStorePlugin, InputManagerSystem};
        use leafwing_input_manager::prelude::updating::InputRegistration;
        use leafwing_input_manager::prelude::RegisterUserInput;
        use leafwing_input_manager::InputControlKind;

        // Touch is a virtual leafwing device, not a second input system.
        // `MobileTouchState` is collected in PreUpdate, the registered input
        // kinds publish it into leafwing's central store, and
        // `bind_touch_virtual_inputs` binds them in the participant's
        // `InputMap`. Then touch resolves like a keyboard or gamepad. No
        // system here writes `ControlFrame` or the `MenuControlFrame`
        // buttons or stick directly. Drag-scroll in `menu_bridge` is the one
        // exception.
        if !app.is_plugin_added::<CentralInputStorePlugin>() {
            app.add_plugins(CentralInputStorePlugin);
        }
        app.register_input_kind::<crate::virtual_device::TouchVirtualButton>(
            InputControlKind::Button,
        );
        app.register_input_kind::<crate::virtual_device::TouchStickDirection>(
            InputControlKind::Button,
        );
        app.register_input_kind::<crate::virtual_device::TouchVirtualStick>(
            InputControlKind::DualAxis,
        );
        app.register_buttonlike_input::<crate::virtual_device::TouchVirtualButton>();
        app.register_buttonlike_input::<crate::virtual_device::TouchStickDirection>();
        app.register_dual_axislike_input::<crate::virtual_device::TouchVirtualStick>();

        // Discover, requirements in, placement out, with the host's resolve
        // between them. Shared with every composer, including the tests.
        app.add_plugins(crate::placement::TouchPresentationPlugin);

        app.add_plugins(VirtualJoystickPlugin::<MobileStick>::default())
            .insert_resource(MobileTouchState::default())
            .insert_resource(MenuTouchGestureState::default())
            .insert_resource(TouchButtonEdges::default())
            .insert_resource(TouchControlsVisible::default())
            .add_systems(
                Startup,
                (
                    spawn_touch_buttons,
                    spawn_touch_joysticks,
                    spawn_frame_axis_glyphs,
                )
                    .after(ambition_render::ui_fonts::UiFontsLoaded),
            )
            // Collect virtual-device state before leafwing unifies input this
            // frame (after bevy_ui focus), so a touch press this frame is an
            // ActionState press this frame.
            .add_systems(
                PreUpdate,
                (read_joystick_messages, update_buttons_from_interactions)
                    .chain()
                    .after(bevy::ui::UiSystems::Focus)
                    .before(InputManagerSystem::Unify),
            )
            // Bind the virtual device in the participant's InputMap, and
            // re-bind after a preset swap.
            .add_systems(
                Update,
                crate::virtual_device::bind_touch_virtual_inputs
                    .in_set(ambition_input::InputSet::ResolveActions),
            )
            .add_systems(
                Update,
                (
                    position_frame_axis_glyphs,
                    // Drag-scroll joins the menu frame after the participant
                    // populate rebuilt it and before menu consumers read it.
                    fold_touch_gestures
                        .in_set(ambition_input::InputSet::Route)
                        .after(ambition_platformer2d_actor_monolith::schedule::MenuFramePopulate)
                        // One pin on the consume set, not one per reader, so a
                        // new reader is covered too.
                        .before(ambition_platformer2d_actor_monolith::schedule::MenuFrameConsume),
                )
                    .chain(),
            )
            // Button-label sync in narrow systems:
            // `update_button_verb_from_prompt` writes `ButtonVerb` from the
            // `ControlPrompt` read-model,
            // `sync_touch_button_visibility_from_prompt` hides buttons for
            // slots the scheme lacks, and `render_touch_button_text` folds
            // verb, glyph, and pressed state into the Text node.
            .add_systems(
                Update,
                (
                    // Labels come from the controlled subject's action scheme
                    // via `ControlPrompt`. Buttons for missing slots are hidden.
                    update_button_verb_from_prompt,
                    sync_touch_button_visibility_from_prompt,
                    // The stick uses the same rule. It runs after the root sync
                    // so it wins over the blanket setting.
                    sync_touch_stick_visibility_from_context.after(sync_touch_ui_visibility),
                    // After `Route` (where `update_seat_active_devices` runs),
                    // so the glyph shows this frame's device.
                    update_button_glyph_from_active_input.after(ambition_input::InputSet::Route),
                    update_button_pressed_from_actions
                        .after(ambition_sim_view::affordances::AffordancesSystemSet::Compute),
                    render_touch_button_text
                        .after(update_button_verb_from_prompt)
                        .after(update_button_glyph_from_active_input)
                        .after(update_button_pressed_from_actions),
                    sync_button_pressed_visual.after(update_button_pressed_from_actions),
                ),
            )
            // Mirror keyboard and gamepad axis input onto the joystick knob,
            // so the on-screen stick also displays non-touch input. Runs
            // after `JoystickSystems::UpdateUI` to override the centered rest
            // position. A real pointer drag wins (early-out on
            // `pointer_state.is_some()`).
            .add_systems(
                PostUpdate,
                // After the behavior stage (which resets `base_offset` to zero
                // for `JoystickFixed`) and before `update_ui` (which derives
                // the base ring and the knob from it).
                offset_joystick_art_within_footprint
                    .after(JoystickSystems::SendMessages)
                    .before(JoystickSystems::UpdateUI),
            )
            .add_systems(
                PostUpdate,
                drive_joystick_knob_from_axis.after(JoystickSystems::UpdateUI),
            );
    }
}

/// Inset the drawn stick within its reserved footprint.
///
/// The root node is the gesture-exclusion region, flush to the screen corner.
/// The art sits `JOYSTICK_MARGIN` in from the corner, clear of edge-swipe
/// gestures, with the same inset as the U/R/L/D glyphs.
///
/// Write only `base_offset`: `virtual_joystick`'s `update_ui` places the base
/// ring at it and derives the knob from it, so the whole stick moves together.
/// Setting the child nodes would fight that system. Root `padding` does not
/// work, because the crate gives the base and knob explicit `left`/`top`.
/// `JoystickFixed` also derives its input center from the base rect, so the
/// input center moves with the art.
fn offset_joystick_art_within_footprint(
    mut joysticks: Query<&mut virtual_joystick::VirtualJoystickState>,
) {
    let origin = movement_joystick_layout().art_origin();
    for mut state in &mut joysticks {
        if state.base_offset != origin {
            state.base_offset = origin;
        }
    }
}

/// Spawn the on-screen move joystick with procedural circle textures, so no
/// knob art asset is needed. Mouse drag works on desktop because
/// `virtual_joystick` routes mouse and touch through the same path.
pub fn spawn_touch_joysticks(mut cmd: Commands, mut images: ResMut<Assets<Image>>) {
    let knob = images.add(build_joystick_knob_image());
    let outline = images.add(build_joystick_outline_image());

    // One Move stick on the left. There is no Aim stick: blink-aim uses the
    // gamepad right stick, and on touch Blink is a button tap. Placement
    // (and the menu drag-scroll exclusion) comes from the resolved control
    // regions after `tag_virtual_joystick_root` marks this root; these values
    // are only the authored full-size shape.
    let layout = movement_joystick_layout();
    create_joystick(
        &mut cmd,
        MobileStick::Move,
        knob,
        outline,
        // Idle stick is visible but quiet. Drags stay readable because the
        // knob moves.
        Some(Color::srgba(0.95, 0.95, 0.95, 0.58)),
        Some(Color::srgba(0.20, 0.30, 0.45, 0.46)),
        Some(Color::srgba(0.10, 0.16, 0.24, 0.18)),
        Vec2::new(layout.knob_size, layout.knob_size),
        Vec2::new(layout.base_size, layout.base_size),
        Node {
            width: Val::Px(layout.base_size),
            height: Val::Px(layout.base_size),
            position_type: PositionType::Absolute,
            left: Val::Px(layout.margin),
            bottom: Val::Px(layout.margin),
            ..default()
        },
        // JoystickFixed: the knob returns to center on release.
        JoystickFixed,
        NoAction,
    );
    // No "Move" label: the knob position shows the direction.
    // `create_joystick` cannot take our marker, so `tag_virtual_joystick_root`
    // adds `MobileTouchUiRoot` to the root later.
    let _ = &mut cmd; // suppress unused mut warning when no follow-up insert
}

/// A U/D/L/R glyph on the move joystick, marking one axis of the controlled
/// character's local reference frame. `local_axis` is the local unit direction
/// (down `(0,1)`, up `(0,-1)`, right `(1,0)`, left `(-1,0)`).
/// `position_frame_axis_glyphs` places each label at the raw joystick
/// direction that resolves to that local command.
#[derive(Component, Clone, Copy)]
pub struct FrameAxisGlyph {
    pub local_axis: Vec2,
}

/// Spawn the four reference-frame glyphs as a non-interactive overlay on the
/// move joystick's footprint. Tagged `MobileTouchUiRoot` so it hides with the
/// HUD.
fn spawn_frame_axis_glyphs(mut cmd: Commands, ui_fonts: Option<Res<UiFonts>>) {
    let layout = movement_joystick_layout();
    let font = touch_text_font(ui_fonts.as_deref(), 22.0);
    cmd.spawn((
        Node {
            width: Val::Px(layout.exclusion_size),
            height: Val::Px(layout.exclusion_size),
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..default()
        },
        // Share the movement stick's resolved rect. `TouchSurface` is only a
        // placement marker (nothing hit-tests on it). The glyphs and stick art
        // both orbit `art_center`, so they stay concentric.
        TouchSurface::Movement,
        // The joystick underneath owns the touches.
        bevy::picking::Pickable::IGNORE,
        GlobalZIndex(TOUCH_HUD_Z + 1),
        MobileTouchUiRoot,
        Name::new("FrameAxisGlyphs"),
    ))
    .with_children(|root| {
        for (label, axis) in [
            ("U", Vec2::new(0.0, -1.0)),
            ("D", Vec2::new(0.0, 1.0)),
            ("L", Vec2::new(-1.0, 0.0)),
            ("R", Vec2::new(1.0, 0.0)),
        ] {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Text::new(label),
                font.clone(),
                TextColor(Color::srgba(0.80, 0.90, 1.0, 0.85)),
                bevy::picking::Pickable::IGNORE,
                FrameAxisGlyph { local_axis: axis },
            ));
        }
    });
}

/// Place each glyph at the raw input direction that maps to its local command.
/// Gameplay and the labels share the same inverse mapping, so labels move only
/// when the active mapping mode changes which raw direction means local U/D/L/R.
fn position_frame_axis_glyphs(
    gravity: Option<Res<ambition_platformer2d_shared_tangle::gravity::GravityField>>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut glyphs: Query<(&FrameAxisGlyph, &mut Node)>,
) {
    use ambition_geometry::{AccelerationFrame, InputFrameMode};
    let gdir =
        ambition_platformer2d_shared_tangle::gravity::gravity_dir_or_default(gravity.as_deref());
    let mode = user_settings
        .as_deref()
        .map_or(InputFrameMode::DEFAULT_MOVEMENT, |s| {
            s.gameplay.resolved_movement_frame_mode()
        });
    let frame = AccelerationFrame::new(gdir);
    let layout = movement_joystick_layout();
    // Root-local center of the drawn stick, not of the reserved footprint.
    let center = layout.art_center();
    let radius = layout.base_size * 0.36;
    for (glyph, mut node) in &mut glyphs {
        let on_input = frame
            .raw_axis_for_resolved_input(
                mode,
                ambition_geometry::LocalAxes::from_vec(glyph.local_axis),
            )
            .vec();
        node.left = Val::Px(center.x + on_input.x * radius - 7.0);
        node.top = Val::Px(center.y + on_input.y * radius - 13.0);
    }
}

/// Add `MobileTouchUiRoot` to each `VirtualJoystickNode` that lacks it.
/// Idempotent through the `Without<MobileTouchUiRoot>` filter.
pub fn tag_virtual_joystick_root(
    mut cmd: Commands,
    query: Query<
        Entity,
        (
            With<VirtualJoystickNode<MobileStick>>,
            Without<MobileTouchUiRoot>,
        ),
    >,
) {
    for entity in &query {
        // Lift the joystick into the HUD z-band. At the default z, a menu
        // scrim draws over it and takes its pointer events, so the stick
        // cannot navigate menus.
        cmd.entity(entity).insert((
            MobileTouchUiRoot,
            GlobalZIndex(TOUCH_HUD_Z),
            TouchSurface::Movement,
        ));
    }
}

/// Procedural 64x64 RGBA knob: a white circle with an anti-aliased rim.
/// The fill is white so the `knob_color` tint sets the look.
fn build_joystick_knob_image() -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::image::Image as BevyImage;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let size = 64u32;
    let mut data = vec![0u8; (size * size * 4) as usize];
    let cx = (size as f32 - 1.0) * 0.5;
    let radius = size as f32 * 0.5;
    let edge = 1.5_f32;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cx;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = ((radius - dist) / edge).clamp(0.0, 1.0);
            let i = ((y * size + x) * 4) as usize;
            data[i] = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = (alpha * 255.0) as u8;
        }
    }
    BevyImage::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Procedural 96x96 RGBA ring with anti-aliased edges, for the joystick's
/// background circle. Tinted in `create_joystick`.
fn build_joystick_outline_image() -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::image::Image as BevyImage;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let size = 96u32;
    let mut data = vec![0u8; (size * size * 4) as usize];
    let cx = (size as f32 - 1.0) * 0.5;
    let outer = size as f32 * 0.5;
    let inner = outer - 8.0;
    let edge = 1.5_f32;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cx;
            let dist = (dx * dx + dy * dy).sqrt();
            let outer_a = ((outer - dist) / edge).clamp(0.0, 1.0);
            let inner_a = ((dist - inner) / edge).clamp(0.0, 1.0);
            let alpha = (outer_a * inner_a).clamp(0.0, 1.0);
            let i = ((y * size + x) * 4) as usize;
            data[i] = 255;
            data[i + 1] = 255;
            data[i + 2] = 255;
            data[i + 3] = (alpha * 255.0) as u8;
        }
    }
    BevyImage::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Mirror `TouchControlsVisible` onto every `MobileTouchUiRoot`. `Visibility`
/// propagates to children, so this hides the whole HUD.
pub fn sync_touch_ui_visibility(
    visible: Res<TouchControlsVisible>,
    mut query: Query<&mut Visibility, With<MobileTouchUiRoot>>,
) {
    // Not gated on `visible.is_changed()`: roots appear at runtime (the
    // joystick root is tagged in `Discover`), and a root that appears on a
    // frame with no setting change would never sync. There are few roots.
    let target = if visible.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut vis in &mut query {
        *vis = target;
    }
}

/// Mirror `UserSettings.controls.touch_controls_visible` into
/// `TouchControlsVisible` every Update, so the settings toggle applies on the
/// same frame. Both default to `true`.
pub fn sync_touch_visibility_from_settings(
    settings: Res<ambition_persistence::settings::UserSettings>,
    mut visible: ResMut<TouchControlsVisible>,
) {
    if visible.0 != settings.controls.touch_controls_visible {
        visible.0 = settings.controls.touch_controls_visible;
    }
}

/// Per-button held-last-frame mask. `update_buttons_from_interactions` uses it
/// to derive press and release edges from `Interaction::Pressed`.
#[derive(Resource, Default, Clone, Copy, Debug)]
struct TouchButtonEdges {
    jump: bool,
    attack: bool,
    special: bool,
    burst: bool,
    blink: bool,
    interact: bool,
    projectile: bool,
    fly_toggle: bool,
    shield: bool,
    grab: bool,
    modifier: bool,
    start: bool,
    reset: bool,
}

/// Spawn the touch button UI: a lower-right diamond for face buttons and a
/// small shoulder row above it. Labels name gameplay intent ("Interact",
/// "Jump", "Fly"), not keyboard keys.
fn spawn_touch_buttons(mut cmd: Commands, ui_fonts: Option<Res<UiFonts>>) {
    let ui_fonts = ui_fonts.as_deref();
    // Right-thumb controls, bottom-right:
    //
    //       Blink        Fly        Shot
    //
    //                Interact
    //        Attack              Burst
    //                  Jump
    //
    // The hit-test is circular and matches the drawn circles, so overlapping
    // square bounds are not ambiguous. It reads `touch_action_layout()`, so
    // multitouch stays aligned with the overlay.
    cmd.spawn((
        Node {
            width: Val::Px(ACTION_BEZEL_W),
            height: Val::Px(ACTION_BEZEL_H),
            position_type: PositionType::Absolute,
            right: Val::Px(0.0),
            bottom: Val::Px(0.0),
            border_radius: BorderRadius::all(Val::Px(34.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.18)),
        // HUD z-band: above menu overlays for render and picking.
        GlobalZIndex(TOUCH_HUD_Z),
        Name::new("MobileTouchActionBezel"),
        MobileTouchUiRoot,
        TouchSurface::ActionBezel,
    ));
    cmd.spawn((
        Node {
            width: Val::Px(ACTION_CLUSTER_W),
            height: Val::Px(ACTION_CLUSTER_H),
            position_type: PositionType::Absolute,
            right: Val::Px(ACTION_CLUSTER_MARGIN),
            bottom: Val::Px(ACTION_CLUSTER_MARGIN),
            ..default()
        },
        // HUD z-band: buttons stay tappable while a menu is open.
        GlobalZIndex(TOUCH_HUD_Z),
        Name::new("MobileTouchActionCluster"),
        MobileTouchUiRoot,
        TouchSurface::ActionCluster,
    ))
    .with_children(|parent| {
        for spec in touch_action_layout() {
            spawn_action_button_at(
                parent,
                spec.action,
                spec.label,
                spec.left,
                spec.top,
                spec.size,
                spec.font_size,
                ui_fonts,
            );
        }
    });

    // Menu buttons (top-right). Start opens and closes the pause menu. Reset
    // is also menu Back; the "Back" label gives phone users an escape without
    // a keyboard.
    cmd.spawn((
        Node {
            width: Val::Px(MENU_ROW_W),
            height: Val::Px(54.0),
            position_type: PositionType::Absolute,
            right: Val::Px(MENU_ROW_MARGIN),
            top: Val::Px(MENU_ROW_MARGIN),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::Center,
            ..default()
        },
        // HUD z-band: "Back" must reach `MenuControlFrame.back` over a
        // full-screen menu scrim.
        GlobalZIndex(TOUCH_HUD_Z),
        Name::new("MobileTouchMenuRow"),
        MobileTouchUiRoot,
        TouchSurface::MenuRow,
    ))
    .with_children(|parent| {
        for action in [TouchActionButton::Start, TouchActionButton::Reset] {
            let label = match action {
                TouchActionButton::Start => "Menu",
                TouchActionButton::Reset => "Back",
                _ => "?",
            };
            spawn_menu_button(parent, action, label, ui_fonts);
        }
    });
}

fn touch_text_font(ui_fonts: Option<&UiFonts>, font_size: f32) -> TextFont {
    ui_fonts
        .map(|fonts| fonts.text_font(font_size, UiFontWeight::Regular))
        .unwrap_or(TextFont {
            font_size: FontSize::Px(font_size),
            ..default()
        })
}

/// Build one absolutely-positioned action button in the right thumb cluster.
/// Absolute placement keeps the drawn diamond and the hit-test in step.
fn spawn_action_button_at(
    parent: &mut ChildSpawnerCommands,
    action: TouchActionButton,
    label: &'static str,
    left: f32,
    top: f32,
    size: f32,
    font_size: f32,
    ui_fonts: Option<&UiFonts>,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(top),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(Val::Px(size * 0.5)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.16, 0.19, 0.27, 0.38)),
            BorderColor::all(Color::srgba(0.68, 0.76, 0.92, 0.28)),
            action,
            // On the Button entity, so `sync_button_pressed_visual` sets
            // `BackgroundColor` without a parent walk.
            ButtonPressed(false),
            Name::new(format!("Touch{label}")),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                touch_text_font(ui_fonts, font_size),
                TextColor(Color::srgb(0.96, 0.97, 1.0)),
                // Center both lines (verb and glyph) in the circle.
                TextLayout::justify(Justify::Center),
                // Marker for the render system. The `ButtonVerb` and
                // `ButtonGlyph` components are what change each frame.
                TouchActionLabel(action),
                // Set by `update_button_verb_from_prompt`, rendered by
                // `render_touch_button_text`.
                ButtonVerb::new(label),
                // Empty until `update_button_glyph_from_active_input` runs,
                // so the first frame shows no "?" subtitle.
                ButtonGlyph(Cow::Borrowed("")),
            ));
        });
}

/// Marker on the touch button's text node. Carries the `TouchActionButton`
/// so the verb-update system can map it back to its control slot.
#[derive(Component)]
pub struct TouchActionLabel(pub TouchActionButton);

/// The verb text for a touch button. Set each frame by
/// [`update_button_verb_from_prompt`] from the
/// [`ambition_sim_view::ControlPrompt`] read-model.
///
/// The fallback is never overwritten, and the current verb is an `Option`.
/// When the prompt has nothing to say, the button shows the spawn label, not
/// stale text from an ended context. Example: in a menu, Jump and Interact
/// take `prompt.menu_confirm`, which only `install_menu_confirm_provider`
/// publishes.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
// Required here, not on `ButtonPressed`: the prompt system queries
// `(&TouchActionLabel, &mut ButtonVerb, &mut ButtonReady)`, and test fixtures
// spawn a label and a verb without press state.
#[require(ButtonReady)]
pub struct ButtonVerb {
    /// Spawn-time fallback label. Only buttons without a [`ControlSlot`] keep
    /// it permanently.
    fallback: &'static str,
    /// What the prompt says this frame, if anything. A `String` so authored
    /// `InteractVariant::Custom` prompts pass through.
    current: Option<String>,
}

impl ButtonVerb {
    /// A button that says `fallback` until a prompt gives it something better.
    pub fn new(fallback: &'static str) -> Self {
        Self {
            fallback,
            current: None,
        }
    }

    fn as_str(&self) -> &str {
        self.current.as_deref().unwrap_or(self.fallback)
    }
}

/// The control SLOT a touch button labels, or `None` for the menu/system
/// buttons (Start / Reset) that carry no gameplay action.
fn touch_button_slot(action: TouchActionButton) -> Option<ControlSlot> {
    Some(match action {
        TouchActionButton::Jump => ControlSlot::Jump,
        TouchActionButton::Attack => ControlSlot::Attack,
        TouchActionButton::Special => ControlSlot::Special,
        TouchActionButton::Burst => ControlSlot::Burst,
        TouchActionButton::Blink => ControlSlot::Blink,
        TouchActionButton::Interact => ControlSlot::Interact,
        TouchActionButton::Projectile => ControlSlot::Projectile,
        TouchActionButton::FlyToggle => ControlSlot::Utility,
        TouchActionButton::Shield => ControlSlot::Shield,
        TouchActionButton::Grab => ControlSlot::Grab,
        TouchActionButton::Modifier => ControlSlot::Modifier,
        TouchActionButton::Start | TouchActionButton::Reset => return None,
    })
}

/// Confirm-button text when the menu names no verb.
///
/// A specific word ("Play", "Equip") is for a game to publish through
/// `ControlPrompt::menu_confirm`. This only makes sure the button never shows
/// a gameplay verb.
const DEFAULT_MENU_CONFIRM: &str = "Select";

/// The select-functional touch buttons: in a menu these fold into
/// `MenuControlFrame.select`, so they wear the menu's confirm verb.
fn is_menu_confirm_button(action: TouchActionButton) -> bool {
    matches!(
        action,
        TouchActionButton::Jump | TouchActionButton::Interact
    )
}

/// The menu-row buttons (Menu / Back). Always shown; the gameplay scheme does
/// not drive them.
fn is_menu_button(action: TouchActionButton) -> bool {
    matches!(action, TouchActionButton::Start | TouchActionButton::Reset)
}

/// Per frame: label each touch button from the [`ControlPrompt`] read-model.
///
/// - Gameplay: the controlled subject's own action names. A slot the scheme
///   lacks is left as is (hidden by
///   [`sync_touch_button_visibility_from_prompt`]).
/// - Menu / Dialogue: Jump and Interact show the menu's confirm verb, so a
///   menu button never reads "Jump".
///
/// Reads the sim-published read-model, never the sim's live components.
pub fn update_button_verb_from_prompt(
    prompt: Res<ControlPrompt>,
    mut labels: Query<(&TouchActionLabel, &mut ButtonVerb, &mut ButtonReady)>,
) {
    for (TouchActionLabel(action), mut verb, mut ready) in &mut labels {
        // Readiness uses the same slot lookup. Change-detected, so an unchanged
        // button does not trigger a restyle.
        let next_ready = touch_button_slot(*action).is_none_or(|slot| prompt.ready_for(slot));
        if ready.0 != next_ready {
            ready.0 = next_ready;
        }
        let next: Option<String> = match prompt.context {
            ControlContextKind::Gameplay => touch_button_slot(*action)
                .and_then(|slot| prompt.label_for(slot))
                .map(str::to_owned),
            ControlContextKind::Menu | ControlContextKind::Dialogue => {
                // In a menu these buttons confirm, in every composition. A
                // game with a better word publishes it through `menu_confirm`.
                is_menu_confirm_button(*action).then(|| {
                    prompt
                        .menu_confirm
                        .clone()
                        .unwrap_or_else(|| DEFAULT_MENU_CONFIRM.to_owned())
                })
            }
            ControlContextKind::Empty => None,
        };
        // Change-detected, so `Changed<ButtonVerb>` stays accurate for
        // `render_touch_button_text`.
        if verb.current != next {
            verb.current = next;
        }
    }
}

/// Whether a touch button is active in the current prompt context.
///
/// - Gameplay: if and only if the controlled subject's scheme has the
///   button's slot (Sanic has no Attack, so its hidden circle cannot fire).
/// - Menu / Dialogue: only Jump / Interact and the Menu / Back row.
/// - Empty (no controllable subject, cold start): only the Menu / Back row.
pub fn touch_action_available(action: TouchActionButton, prompt: &ControlPrompt) -> bool {
    match prompt.context {
        ControlContextKind::Gameplay => match touch_button_slot(action) {
            Some(slot) => prompt.label_for(slot).is_some(),
            None => true, // Start / Reset carry no gameplay slot; always available
        },
        ControlContextKind::Menu | ControlContextKind::Dialogue => {
            is_menu_confirm_button(action) || is_menu_button(action)
        }
        ControlContextKind::Empty => is_menu_button(action),
    }
}

/// Whether this button is live this frame: drawn and touchable.
///
/// Both the visibility sync and the touch mask call this, so a button cannot
/// be drawn but untouchable, or touchable but hidden. `Start` and `Reset` are
/// always live: they are pause and restart, and a phone without a keyboard
/// needs them to get out.
pub fn touch_action_live(
    action: TouchActionButton,
    prompt: &ControlPrompt,
    gameplay_owns_input: bool,
) -> bool {
    if !touch_action_available(action, prompt) {
        return false;
    }
    let always_available = matches!(action, TouchActionButton::Start | TouchActionButton::Reset);
    if always_available {
        return true;
    }
    // The gameplay-ownership term applies only to a gameplay prompt.
    // `publish_frontend_context_prompt` already rewrites the prompt to `Menu`
    // when a non-gameplay context owns the seat. The term covers the stale
    // case: the prompt keeps its last value when no seat resolves an owner, so
    // a prompt that still says Gameplay while nobody owns gameplay must not
    // show gameplay verbs.
    if !matches!(prompt.context, ControlContextKind::Gameplay) {
        return true;
    }
    gameplay_owns_input
}

/// Clear the held flag of any action that is not live this frame. The raw
/// hit test then cannot fire an unavailable action, and a held action that
/// becomes unavailable gets a clean release edge.
fn mask_unavailable(now: &mut TouchButtonEdges, prompt: &ControlPrompt, gameplay: bool) {
    let avail = |a| touch_action_live(a, prompt, gameplay);
    now.jump &= avail(TouchActionButton::Jump);
    now.attack &= avail(TouchActionButton::Attack);
    now.special &= avail(TouchActionButton::Special);
    now.burst &= avail(TouchActionButton::Burst);
    now.blink &= avail(TouchActionButton::Blink);
    now.interact &= avail(TouchActionButton::Interact);
    now.projectile &= avail(TouchActionButton::Projectile);
    now.fly_toggle &= avail(TouchActionButton::FlyToggle);
    now.shield &= avail(TouchActionButton::Shield);
    now.grab &= avail(TouchActionButton::Grab);
    now.modifier &= avail(TouchActionButton::Modifier);
    // Start / Reset are always available: no mask.
}

/// Per frame: show the movement stick only when it steers something.
///
/// Gated on `TouchSurface::Movement`, not the shared `MobileTouchUiRoot`: the
/// action bezel carries Start and Reset, and hiding that root would hide them.
pub fn sync_touch_stick_visibility_from_context(
    active_context: Option<Res<ambition_input::SeatInputContexts>>,
    prompt: Res<ControlPrompt>,
    visible: Res<TouchControlsVisible>,
    mut sticks: Query<(&TouchSurface, &mut Visibility)>,
) {
    // The touch overlay is one device on one screen: the local primary seat.
    let gameplay = active_context
        .as_deref()
        .is_none_or(|seats| seats.primary().gameplay_owned());
    // The stick also steers menus: `bind_touch_virtual_inputs` maps
    // `TouchVirtualStick` to both `Move` and `MenuStick`. So keep it visible in
    // Menu and Dialogue. `Empty` means no surface reads those frames (see
    // `surface_prompt` in `ambition_sim_view::control_prompt`), so hide it.
    let steers_something = gameplay
        || matches!(
            prompt.context,
            ControlContextKind::Menu | ControlContextKind::Dialogue
        );
    let target = if visible.0 && steers_something {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (surface, mut vis) in &mut sticks {
        if !matches!(surface, TouchSurface::Movement) {
            continue;
        }
        if *vis != target {
            *vis = target;
        }
    }
}

/// Per frame: show exactly the buttons that are live (see
/// [`touch_action_live`]). Shown buttons use `Visibility::Inherited`, not
/// `Visible`, so they still obey the [`TouchControlsVisible`] root toggle.
pub fn sync_touch_button_visibility_from_prompt(
    prompt: Res<ControlPrompt>,
    // Whether gameplay owns the participant's actions this frame. Optional:
    // apps without the participant-context resolver get prompt-only behavior.
    active_context: Option<Res<ambition_input::SeatInputContexts>>,
    mut buttons: Query<(&TouchActionButton, &mut Visibility)>,
) {
    // A verb nobody can press must not be on screen. The prompt describes what
    // the controlled subject can do, even while a menu owns input; whether
    // anyone can drive it now is a question for the input context.
    // Start and Reset are exempt (see `touch_action_live`).
    // The touch overlay is one device on one screen: the local primary seat.
    let gameplay = active_context
        .as_deref()
        .is_none_or(|seats| seats.primary().gameplay_owned());

    for (action, mut vis) in &mut buttons {
        let target = if touch_action_live(*action, &prompt, gameplay) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != target {
            *vis = target;
        }
    }
}

/// Per frame: fold each button's [`ButtonVerb`] and [`ButtonGlyph`] into its
/// `Text`: the verb on line one, the glyph in parentheses on line two.
/// Runs only on change, so steady frames do not mark `Text` changed.
pub fn render_touch_button_text(
    mut q: Query<
        (&ButtonVerb, &ButtonGlyph, &mut Text),
        Or<(Changed<ButtonVerb>, Changed<ButtonGlyph>)>,
    >,
) {
    for (verb, glyph, mut text) in &mut q {
        let verb_str = verb.as_str();
        let glyph_str = glyph.0.as_ref();
        let desired = if glyph_str.is_empty() {
            verb_str.to_owned()
        } else {
            format!("{verb_str}\n({glyph_str})")
        };
        if text.0 != desired {
            text.0 = desired;
        }
    }
}

/// Per-device glyph subtitle, from the primary seat's active device
/// (`SeatActiveDevices`) and the selected [`KeyboardPreset`].
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ButtonGlyph(pub Cow<'static, str>);

/// Set while the button's `Platformer2dInputActionMonolith` is held this
/// frame. [`sync_button_pressed_visual`] brightens the button from it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonPressed(pub bool);

/// Whether this button's action can fire now.
///
/// Read from the prompt; never timed here. The `ControlPrompt` entry carries
/// the sim's answer (the body's fire-rate floor), so a dimmed button and a
/// refused press always agree. `true` for buttons with no cooldown and for a
/// slot the prompt does not carry (the label shows that case).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonReady(pub bool);

impl Default for ButtonReady {
    fn default() -> Self {
        Self(true)
    }
}

/// Map a touch button to its gameplay [`Platformer2dInputActionMonolith`].
///
/// Composes button → slot → action through `ambition_input::action_for_slot`,
/// so there is no second table to keep in sync. Start and Reset have no slot
/// and map directly. Returns `None` for a button with no action; it gets no
/// glyph and no press.
fn touch_action_to_sandbox_action(
    action: TouchActionButton,
) -> Option<Platformer2dInputActionMonolith> {
    match action {
        TouchActionButton::Start => Some(Platformer2dInputActionMonolith::Start),
        TouchActionButton::Reset => Some(Platformer2dInputActionMonolith::Reset),
        gameplay => touch_button_slot(gameplay).and_then(ambition_input::action_for_slot),
    }
}

/// Per frame: write each button's glyph from the primary seat's active device
/// and the player's selected [`KeyboardPreset`], so glyphs follow a rebind.
pub fn update_button_glyph_from_active_input(
    devices: Res<ambition_input::SeatActiveDevices>,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    seat_bindings: Option<Res<ambition_input::SeatBindings>>,
    mut labels: Query<(&TouchActionLabel, &mut ButtonGlyph)>,
) {
    // Default to Arrows+ZXC when there are no settings (headless host).
    let preset = settings
        .map(|s| KeyboardPreset::by_index(s.controls.keyboard_preset_index))
        .unwrap_or_else(KeyboardPreset::arrows_zxc);
    // The primary seat's bindings. If nothing projected them yet, an empty set
    // gives empty glyphs, not stale ones.
    let empty = ambition_input::ActionBindings::default();
    let bound = seat_bindings.as_deref().map_or(&empty, |seats| {
        seats.for_seat(ambition_input::ParticipantId::PRIMARY.slot())
    });
    for (TouchActionLabel(touch_action), mut glyph) in &mut labels {
        let Some(sa) = touch_action_to_sandbox_action(*touch_action) else {
            continue;
        };
        let next = ambition_input::glyph_for(
            sa,
            &preset,
            bound,
            devices.for_seat(ambition_input::ParticipantId::PRIMARY.slot()),
        );
        if glyph.0 != next {
            glyph.0 = next;
        }
    }
}

/// Per frame: write each button's pressed flag from the primary participant's
/// `ActionState<Platformer2dInputActionMonolith>`. Touch is a bound virtual
/// device, so this one source lights the button for touch, mouse, keyboard,
/// and gamepad. Writes only on change, for `Changed<ButtonPressed>`.
///
/// Uses the primary seat, not `single()`: the overlay is the machine's own
/// screen, like [`update_button_glyph_from_active_input`]. A couch seat's pad
/// must not light it, and `single()` fails when a second participant spawns.
pub fn update_button_pressed_from_actions(
    actions_q: Query<(
        &ambition_input::InputParticipant,
        &leafwing_input_manager::prelude::ActionState<Platformer2dInputActionMonolith>,
    )>,
    mut buttons: Query<(&TouchActionButton, &mut ButtonPressed)>,
) {
    let actions = actions_q
        .iter()
        .find(|(participant, _)| participant.id == ambition_input::ParticipantId::PRIMARY)
        .map(|(_, actions)| actions);
    for (touch_action, mut pressed) in &mut buttons {
        // A button with no action reads as not held.
        let held = touch_action_to_sandbox_action(*touch_action)
            .zip(actions)
            .is_some_and(|(sa, a)| a.pressed(&sa));
        if pressed.0 != held {
            pressed.0 = held;
        }
    }
}

/// Per frame: when [`ButtonPressed`] or [`ButtonReady`] changes, set the
/// button's background, so the overlay also works as an input display.
pub fn sync_button_pressed_visual(
    mut buttons: Query<
        (&ButtonPressed, &ButtonReady, &mut BackgroundColor),
        Or<(Changed<ButtonPressed>, Changed<ButtonReady>)>,
    >,
) {
    for (pressed, ready, mut bg) in &mut buttons {
        // One writer for the color, so readiness is folded in here. Two
        // systems writing `BackgroundColor` would race on schedule order.
        bg.0 = match (pressed.0, ready.0) {
            // Held: brighter and more opaque.
            (true, _) => Color::srgba(0.42, 0.58, 0.95, 0.78),
            // Match the default authored in `spawn_action_button_at`.
            (false, true) => Color::srgba(0.16, 0.19, 0.27, 0.38),
            // Recharging: dimmer, never hidden, so the player can still see
            // where the shot is.
            (false, false) => Color::srgba(0.10, 0.11, 0.15, 0.22),
        };
    }
}

/// Build one menu-row button (Menu / Back), away from the action diamond.
fn spawn_menu_button(
    parent: &mut ChildSpawnerCommands,
    action: TouchActionButton,
    label: &str,
    ui_fonts: Option<&UiFonts>,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(88.0),
                height: Val::Px(44.0),
                margin: UiRect::all(Val::Px(4.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.20, 0.16, 0.22, 0.60)),
            action,
            Name::new(format!("Touch{label}")),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                touch_text_font(ui_fonts, 15.0),
                TextColor(Color::srgb(0.94, 0.90, 0.96)),
            ));
        });
}

/// Read each `TouchActionButton`'s `Interaction` and fold held state and
/// press/release edges into `MobileTouchState`. Edges compare against the
/// previous frame's mask in `TouchButtonEdges`.
fn update_buttons_from_interactions(
    query: Query<(&Interaction, &TouchActionButton), With<Button>>,
    touches: Res<Touches>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    placement: Res<crate::placement::TouchControlPlacement>,
    prompt: Res<ControlPrompt>,
    // The same term the visibility pass reads, optional for the same reason.
    active_context: Option<Res<ambition_input::SeatInputContexts>>,
    mut state: ResMut<MobileTouchState>,
    mut edges: ResMut<TouchButtonEdges>,
) {
    let gameplay = active_context
        .as_deref()
        .is_none_or(|seats| seats.primary().gameplay_owned());
    let mut now = TouchButtonEdges::default();

    // Desktop path: bevy_ui interactions are enough for mouse testing.
    for (interaction, action) in &query {
        let held = matches!(interaction, Interaction::Pressed);
        set_button_held(&mut now, *action, held);
    }

    // Real-touch path: `Interaction` is not reliable for multitouch while
    // another finger holds the joystick. Hit-test raw touches so the left
    // thumb can stay on the stick. The rectangles come from the resolved
    // placement, not the window, so touch targets match what is drawn.
    let cluster = placement.action_cluster;
    let menu_row = placement.menu_row;
    for touch in touches.iter() {
        if let Some(action) = touch_action_at_position(touch.position(), cluster, menu_row) {
            set_button_held(&mut now, action, true);
        }
    }

    // Desktop raw mouse hit test, like the touch path, so the overlay works
    // even when another UI panel takes the `Button` interaction.
    if mouse_buttons.pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                if let Some(action) = touch_action_at_position(cursor, cluster, menu_row) {
                    set_button_held(&mut now, action, true);
                }
            }
        }
    }

    // Same `touch_action_live` inputs as the visibility pass, so drawn buttons
    // and touch targets agree.
    mask_unavailable(&mut now, &prompt, gameplay);

    let make_btn = |held_now: bool, held_prev: bool| super::TouchButton {
        held: held_now,
        pressed_this_frame: held_now && !held_prev,
        released_this_frame: !held_now && held_prev,
    };
    state.0.jump = make_btn(now.jump, edges.jump);
    state.0.attack = make_btn(now.attack, edges.attack);
    state.0.special = make_btn(now.special, edges.special);
    state.0.burst = make_btn(now.burst, edges.burst);
    state.0.blink = make_btn(now.blink, edges.blink);
    state.0.interact = make_btn(now.interact, edges.interact);
    state.0.projectile = make_btn(now.projectile, edges.projectile);
    state.0.fly_toggle = make_btn(now.fly_toggle, edges.fly_toggle);
    state.0.shield = make_btn(now.shield, edges.shield);
    state.0.grab = make_btn(now.grab, edges.grab);
    state.0.modifier = make_btn(now.modifier, edges.modifier);
    state.0.start = make_btn(now.start, edges.start);
    state.0.reset = make_btn(now.reset, edges.reset);
    *edges = now;
}

/// Read one action's held flag from the edge mask; the inverse of
/// [`set_button_held`]. Lets tests ask whether an action is touchable without
/// a second copy of the mapping. Test-only, because production only writes the
/// mask.
#[cfg(test)]
fn held_of(edges: &TouchButtonEdges, action: TouchActionButton) -> bool {
    match action {
        TouchActionButton::Jump => edges.jump,
        TouchActionButton::Attack => edges.attack,
        TouchActionButton::Special => edges.special,
        TouchActionButton::Burst => edges.burst,
        TouchActionButton::Blink => edges.blink,
        TouchActionButton::Interact => edges.interact,
        TouchActionButton::Projectile => edges.projectile,
        TouchActionButton::FlyToggle => edges.fly_toggle,
        TouchActionButton::Shield => edges.shield,
        TouchActionButton::Grab => edges.grab,
        TouchActionButton::Modifier => edges.modifier,
        TouchActionButton::Start => edges.start,
        TouchActionButton::Reset => edges.reset,
    }
}

fn set_button_held(edges: &mut TouchButtonEdges, action: TouchActionButton, held: bool) {
    if !held {
        return;
    }
    match action {
        TouchActionButton::Jump => edges.jump = true,
        TouchActionButton::Attack => edges.attack = true,
        TouchActionButton::Special => edges.special = true,
        TouchActionButton::Burst => edges.burst = true,
        TouchActionButton::Blink => edges.blink = true,
        TouchActionButton::Interact => edges.interact = true,
        TouchActionButton::Projectile => edges.projectile = true,
        TouchActionButton::FlyToggle => edges.fly_toggle = true,
        TouchActionButton::Shield => edges.shield = true,
        TouchActionButton::Grab => edges.grab = true,
        TouchActionButton::Modifier => edges.modifier = true,
        TouchActionButton::Start => edges.start = true,
        TouchActionButton::Reset => edges.reset = true,
    }
}

/// Whether `drive_joystick_knob_from_axis` overrides the knob with the
/// gameplay move axis this frame.
///
/// The override shows keyboard and gamepad input on the knob. While a menu or
/// the launcher owns input, the gameplay `ControlFrame` is neutral, so the
/// override would snap the knob to center during a menu drag. Keyed on the
/// [`ControlPrompt`] context (the same contract as the button labels), not on
/// `GameMode` or actor presence.
pub fn axis_override_drives_knob(context: ControlContextKind) -> bool {
    // Menu / Dialogue / Empty read the stick through the menu seam.
    matches!(context, ControlContextKind::Gameplay)
}

/// Mirror keyboard and gamepad axis input onto the joystick knob, so the HUD
/// also displays non-touch input.
///
/// During a real drag (`pointer_state.is_some()`), `virtual_joystick`'s
/// `update_ui` drives the knob. Otherwise, replace the centered rest position
/// with an offset from `ControlFrame.axis_x` / `axis_y`, using the crate's
/// circle-bounded math. Skipped outside gameplay (see
/// [`axis_override_drives_knob`]) so the knob follows a menu drag.
///
/// `ControlFrame.axis_*` is +Y-down, like Bevy UI `Node.top`, so there is no
/// Y inversion.
fn drive_joystick_knob_from_axis(
    prompt: Res<ControlPrompt>,
    control_frame: Res<ControlFrame>,
    joystick_q: Query<(&VirtualJoystickState, &Children), With<VirtualJoystickNode<MobileStick>>>,
    base_q: Query<&ComputedNode, With<VirtualJoystickUIBackground>>,
    mut knob_q: Query<(&mut Node, &ComputedNode), With<VirtualJoystickUIKnob>>,
) {
    // Outside gameplay the axis is about 0; let `update_ui` follow the drag.
    if !axis_override_drives_knob(prompt.context) {
        return;
    }
    // Axes inside ±1e-3 are no input: snap the knob to center even with a
    // `pointer_state`. On Android the crate can keep a stale `pointer_state`
    // after release, which pinned the knob off-center. The stick-active gate
    // in the menu_bridge fold keeps this dead-band out of gameplay.
    const NEUTRAL_EPS: f32 = 1.0e-3;

    for (state, children) in &joystick_q {
        let axis_raw = Vec2::new(
            control_frame.axis_x.clamp(-1.0, 1.0),
            control_frame.axis_y.clamp(-1.0, 1.0),
        );
        let neutral = axis_raw.x.abs() < NEUTRAL_EPS && axis_raw.y.abs() < NEUTRAL_EPS;

        // A real drag wins while the axis moves; `update_ui` already placed
        // the knob. A neutral axis still overrides (see `NEUTRAL_EPS`).
        if state.pointer_state.is_some() && !neutral {
            continue;
        }
        let mut base_size: Option<Vec2> = None;
        let mut knob_entity: Option<Entity> = None;
        for child in children.iter() {
            if let Ok(base) = base_q.get(child) {
                // Convert to logical pixels to match the `Val::Px` we write.
                // `ComputedNode::size()` is physical, and Android scale
                // factors (2.5–3×) would push the knob off the ring. Same as
                // the crate's `virtual_joystick::systems::node_rect`.
                base_size = Some(base.size() * base.inverse_scale_factor);
            }
            if knob_q.contains(child) {
                knob_entity = Some(child);
            }
        }
        let (Some(base_size), Some(knob_entity)) = (base_size, knob_entity) else {
            continue;
        };
        let Ok((mut knob_node, knob_computed)) = knob_q.get_mut(knob_entity) else {
            continue;
        };
        let knob_size = knob_computed.size() * knob_computed.inverse_scale_factor;
        let base_half = base_size * 0.5;
        let knob_half = knob_size * 0.5;

        // Clamp to the unit circle so diagonals stay on the ring. Matches
        // the crate's `joystick_delta` clamp.
        let mag_sq = axis_raw.length_squared();
        let axis = if mag_sq > 1.0 {
            axis_raw / mag_sq.sqrt()
        } else {
            axis_raw
        };

        // Center the knob on the base center, then offset by the axis times
        // the travel radius (`base_half - knob_half`, so full deflection stays
        // inside the ring). `left`/`top` address the top-left corner, so
        // subtract `knob_half`. Use the same `art_origin` as
        // `offset_joystick_art_within_footprint`, or the knob jumps to the
        // footprint corner when the axis goes neutral.
        let art_origin = movement_joystick_layout().art_origin();
        let travel = base_half - knob_half;
        let center_left = art_origin.x + base_half.x - knob_half.x;
        let center_top = art_origin.y + base_half.y - knob_half.y;
        let target_left = center_left + travel.x * axis.x;
        let target_top = center_top + travel.y * axis.y;
        let new_left = Val::Px(target_left);
        let new_top = Val::Px(target_top);
        // Write only on change, so idle frames do not mark the node changed.
        if knob_node.left != new_left {
            knob_node.left = new_left;
        }
        if knob_node.top != new_top {
            knob_node.top = new_top;
        }
        if knob_node.position_type != PositionType::Absolute {
            knob_node.position_type = PositionType::Absolute;
        }
    }
}

/// Read every `VirtualJoystickMessage<MobileStick>` this frame into
/// `MobileTouchState`, keeping the latest reading per stick.
fn read_joystick_messages(
    mut reader: MessageReader<VirtualJoystickMessage<MobileStick>>,
    mut state: ResMut<MobileTouchState>,
) {
    for msg in reader.read() {
        // `axis()` is the delta in -1..=1 per axis. Do not use `value()`: it
        // is the raw pointer pixel position. `snap_axis()` gives only -1/0/+1
        // past a 0.5 deadzone, which loses analog feel; the engine applies its
        // own deadzone.
        //
        // Cardinal press edges are not derived here. The
        // `TouchStickDirection` buttons publish held state and leafwing
        // derives the edge, as for a gamepad stick, so double-tap detectors
        // see real taps.
        let axis = msg.axis();
        match msg.id() {
            MobileStick::Move => {
                state.0.move_x = axis.x;
                // Bevy UI Y is up; the sim's +Y is down. Flip so a drag down
                // gives axis_y > 0.
                state.0.move_y = -axis.y;
            }
            MobileStick::Aim => {
                state.0.aim_x = axis.x;
                state.0.aim_y = -axis.y;
            }
        }
    }
}

#[cfg(test)]
mod prompt_tests {
    use super::*;
    use ambition_sim_view::PromptEntry;

    fn prompt(context: ControlContextKind, entries: Vec<(ControlSlot, &str)>) -> ControlPrompt {
        ControlPrompt {
            context,
            entries: entries
                .into_iter()
                .map(|(slot, label)| PromptEntry {
                    slot,
                    label: label.to_owned(),
                    visual: None,
                    // Label fixtures only; bindings have a separate test.
                    binding: None,
                    // Readiness is `ButtonReady`'s concern, not the label's.
                    ready: true,
                })
                .collect(),
            menu_confirm: None,
        }
    }

    fn menu_prompt(confirm: &str) -> ControlPrompt {
        ControlPrompt {
            context: ControlContextKind::Menu,
            entries: Vec::new(),
            menu_confirm: Some(confirm.to_owned()),
        }
    }

    #[test]
    fn attack_button_relabels_from_the_prompt() {
        let mut app = App::new();
        app.insert_resource(prompt(
            ControlContextKind::Gameplay,
            vec![(ControlSlot::Attack, "Cleave")],
        ));
        app.add_systems(Update, update_button_verb_from_prompt);
        let text = app
            .world_mut()
            .spawn((
                TouchActionLabel(TouchActionButton::Attack),
                ButtonVerb::new("Atk"),
            ))
            .id();
        app.update();

        let verb = app.world().entity(text).get::<ButtonVerb>().unwrap();
        assert_eq!(verb.as_str(), "Cleave");
    }

    /// The Utility button shows the subject's own word for that slot, and
    /// falls back to the spawn label only when the subject has none. The verb
    /// is a word no game uses, so a hardcoded real label fails.
    #[test]
    fn the_utility_button_wears_whatever_the_subject_calls_that_slot() {
        fn verb_of_fly_button(prompt: ControlPrompt) -> String {
            let mut app = App::new();
            app.insert_resource(prompt);
            app.add_systems(Update, update_button_verb_from_prompt);
            let button = app
                .world_mut()
                .spawn((
                    TouchActionLabel(TouchActionButton::FlyToggle),
                    ButtonVerb::new("Fly"),
                ))
                .id();
            app.update();
            app.world()
                .entity(button)
                .get::<ButtonVerb>()
                .unwrap()
                .as_str()
                .to_owned()
        }

        assert_eq!(
            verb_of_fly_button(prompt(
                ControlContextKind::Gameplay,
                vec![(ControlSlot::Utility, "Ensporulate")],
            )),
            "Ensporulate",
            "a subject that names its Utility action puts that word on the button"
        );
        assert_eq!(
            verb_of_fly_button(prompt(
                ControlContextKind::Gameplay,
                vec![(ControlSlot::Jump, "Jump")],
            )),
            "Fly",
            "and ONLY a subject with nothing on Utility leaves the spawn label"
        );
    }

    #[test]
    fn button_for_a_slot_the_scheme_lacks_is_hidden() {
        let mut app = App::new();
        // Gameplay prompt with ONLY a Jump action (a movement-only body).
        app.insert_resource(prompt(
            ControlContextKind::Gameplay,
            vec![(ControlSlot::Jump, "Jump")],
        ));
        app.add_systems(Update, sync_touch_button_visibility_from_prompt);
        let jump = app
            .world_mut()
            .spawn((TouchActionButton::Jump, Visibility::Inherited))
            .id();
        let attack = app
            .world_mut()
            .spawn((TouchActionButton::Attack, Visibility::Inherited))
            .id();
        // The menu button must never be hidden by the gameplay scheme.
        let start = app
            .world_mut()
            .spawn((TouchActionButton::Start, Visibility::Inherited))
            .id();
        app.update();

        let vis = |e: Entity| *app.world().entity(e).get::<Visibility>().unwrap();
        assert_eq!(vis(jump), Visibility::Inherited, "present slot stays shown");
        assert_eq!(vis(attack), Visibility::Hidden, "absent slot is hidden");
        assert_eq!(vis(start), Visibility::Inherited, "menu button untouched");
    }

    #[test]
    fn menu_relabels_select_buttons_and_hides_gameplay_buttons() {
        // In a menu the select-functional Jump reads the confirm verb (never
        // "Jump"), gameplay-only Attack is hidden, and the Back button stays.
        let mut app = App::new();
        app.insert_resource(menu_prompt("Equip"));
        app.add_systems(
            Update,
            (
                update_button_verb_from_prompt,
                sync_touch_button_visibility_from_prompt,
            ),
        );
        let jump = app
            .world_mut()
            .spawn((
                TouchActionButton::Jump,
                Visibility::Inherited,
                TouchActionLabel(TouchActionButton::Jump),
                ButtonVerb::new("Jump"),
            ))
            .id();
        let attack = app
            .world_mut()
            .spawn((TouchActionButton::Attack, Visibility::Inherited))
            .id();
        let back = app
            .world_mut()
            .spawn((TouchActionButton::Reset, Visibility::Inherited))
            .id();
        app.update();

        let ent = |e: Entity| app.world().entity(e);
        assert_eq!(
            ent(jump).get::<ButtonVerb>().unwrap().as_str(),
            "Equip",
            "select button wears the menu confirm verb"
        );
        assert_eq!(
            *ent(jump).get::<Visibility>().unwrap(),
            Visibility::Inherited,
            "select button stays shown"
        );
        assert_eq!(
            *ent(attack).get::<Visibility>().unwrap(),
            Visibility::Hidden,
            "gameplay-only button hidden in a menu"
        );
        assert_eq!(
            *ent(back).get::<Visibility>().unwrap(),
            Visibility::Inherited,
            "Back button stays"
        );
    }

    #[test]
    fn availability_predicate_covers_all_contexts() {
        // Gameplay: available iff the scheme carries the slot.
        let g = prompt(
            ControlContextKind::Gameplay,
            vec![(ControlSlot::Jump, "Jump")],
        );
        assert!(touch_action_available(TouchActionButton::Jump, &g));
        assert!(!touch_action_available(TouchActionButton::Attack, &g));
        assert!(touch_action_available(TouchActionButton::Start, &g)); // menu row always

        // The Special button follows the scheme's Special slot.
        assert_eq!(
            touch_button_slot(TouchActionButton::Special),
            Some(ControlSlot::Special)
        );
        assert!(
            !touch_action_available(TouchActionButton::Special, &g),
            "no Special slot in this scheme -> hidden + untappable"
        );
        let g_special = prompt(
            ControlContextKind::Gameplay,
            vec![
                (ControlSlot::Jump, "Jump"),
                (ControlSlot::Special, "Bubble Shield"),
            ],
        );
        assert!(
            touch_action_available(TouchActionButton::Special, &g_special),
            "a special-bearing scheme makes the Special button available"
        );

        // Menu: only select-functional + menu row.
        let m = menu_prompt("Select");
        assert!(touch_action_available(TouchActionButton::Jump, &m));
        assert!(touch_action_available(TouchActionButton::Reset, &m));
        assert!(!touch_action_available(TouchActionButton::Attack, &m));

        // Empty: gameplay actions hidden, only the menu row survives.
        let e = ControlPrompt::default(); // context = Empty
        assert!(!touch_action_available(TouchActionButton::Jump, &e));
        assert!(!touch_action_available(TouchActionButton::Attack, &e));
        assert!(touch_action_available(TouchActionButton::Start, &e));
    }

    /// Every button a dialogue shows reads what it does. Runs the real
    /// systems and checks the rendered `Text` of live buttons only; a stale
    /// label on a hidden button is not visible.
    #[test]
    fn every_button_a_dialogue_shows_reads_what_it_does() {
        let confirm = "Advance";
        let mut app = App::new();
        app.insert_resource(menu_prompt(confirm));
        app.add_systems(
            Update,
            (update_button_verb_from_prompt, render_touch_button_text).chain(),
        );
        for (action, gameplay_label) in [
            (TouchActionButton::Jump, "Jump"),
            (TouchActionButton::Interact, "Talk"),
            (TouchActionButton::Attack, "Atk"),
            (TouchActionButton::Start, "Menu"),
            (TouchActionButton::Reset, "Back"),
        ] {
            app.world_mut().spawn((
                TouchActionLabel(action),
                action,
                ButtonVerb::new(gameplay_label),
                ButtonGlyph(Cow::Borrowed("")),
                Text::new(gameplay_label),
            ));
        }
        app.update();

        let prompt_value = app.world().resource::<ControlPrompt>().clone();
        let mut shown = app.world_mut().query::<(&TouchActionButton, &Text)>();
        let mut seen = 0;
        for (action, text) in shown.iter(app.world()) {
            if !touch_action_live(*action, &prompt_value, false) {
                continue; // hidden: its label is not on screen to be wrong
            }
            seen += 1;
            let body = text.0.lines().next().unwrap_or_default().to_owned();
            let expected = match action {
                TouchActionButton::Jump | TouchActionButton::Interact => confirm,
                TouchActionButton::Start => "Menu",
                TouchActionButton::Reset => "Back",
                other => panic!("{other:?} should not be visible in a dialogue"),
            };
            assert_eq!(
                body, expected,
                "{action:?} is on screen during a dialogue reading {body:?}, but \
                 it does {expected:?} — a confirm button still wearing its \
                 gameplay verb is a control that lies about itself"
            );
        }
        assert!(
            seen >= 3,
            "the dialogue must SHOW something to confirm and something to go \
             back with; only {seen} button(s) were live, which is the state that \
             left a phone player with no way out but the corner back button"
        );
    }

    /// Every gameplay slot exposed by the touch overlay must have a virtual-device
    /// binding. Shell-only buttons such as Start and Reset may be bound without a
    /// gameplay slot.
    #[test]
    fn every_button_the_overlay_can_draw_can_also_be_pressed() {
        let bound: std::collections::HashSet<TouchActionButton> =
            crate::virtual_device::touch_bindings()
                .into_iter()
                .map(|(_, button)| button.0)
                .collect();
        for action in crate::virtual_device::ALL_TOUCH_BUTTONS {
            if touch_button_slot(action).is_none() {
                continue; // no gameplay slot: never labelled from a scheme
            }
            assert!(
                bound.contains(&action),
                "{action:?} carries a ControlSlot, so the overlay draws it, \
                 labels it from the subject's scheme and hit-tests it — but \
                 `touch_bindings` sends nothing when it is pressed. A button \
                 that lies about being a control is worse than a missing one"
            );
        }
    }

    /// A menu that names no confirm verb still labels its buttons correctly.
    ///
    /// Only `ambition_app`'s kaleidoscope menu calls
    /// `install_menu_confirm_provider`, so in every demo (such as Sanic)
    /// `menu_confirm` is `None`. The sibling test passes a verb, so it cannot
    /// detect stale gameplay verbs; this test drives `menu_confirm: None`.
    #[test]
    fn a_menu_with_no_authored_verb_still_never_reads_a_gameplay_one() {
        let gameplay_verbs = ["Spin Dash", "Jump", "Talk", "Atk"];
        let mut app = App::new();
        app.insert_resource(ControlPrompt {
            context: ControlContextKind::Menu,
            entries: Vec::new(),
            menu_confirm: None, // ← the whole point
        });
        app.add_systems(
            Update,
            (update_button_verb_from_prompt, render_touch_button_text).chain(),
        );
        for (action, gameplay_label) in [
            (TouchActionButton::Jump, "Spin Dash"),
            (TouchActionButton::Interact, "Talk"),
            (TouchActionButton::Attack, "Atk"),
            (TouchActionButton::Start, "Menu"),
            (TouchActionButton::Reset, "Back"),
        ] {
            app.world_mut().spawn((
                TouchActionLabel(action),
                action,
                ButtonVerb::new(gameplay_label),
                ButtonGlyph(Cow::Borrowed("")),
                Text::new(gameplay_label),
            ));
        }
        app.update();

        let prompt_value = app.world().resource::<ControlPrompt>().clone();
        let mut shown = app.world_mut().query::<(&TouchActionButton, &Text)>();
        let mut confirms = 0;
        for (action, text) in shown.iter(app.world()) {
            if !touch_action_live(*action, &prompt_value, false) {
                continue;
            }
            let body = text.0.lines().next().unwrap_or_default().to_owned();
            if is_menu_confirm_button(*action) {
                confirms += 1;
                assert_eq!(
                    body, DEFAULT_MENU_CONFIRM,
                    "{action:?} confirms this menu but reads {body:?}"
                );
            }
            assert!(
                !gameplay_verbs.contains(&body.as_str()),
                "{action:?} is on screen in a MENU reading {body:?}, a verb from \
                 the gameplay the player just left — the control lies about what \
                 pressing it does"
            );
        }
        assert!(
            confirms >= 1,
            "a menu with no authored verb still has to offer a way to confirm; \
             {confirms} confirm button(s) were live"
        );
    }

    /// The move stick is shown wherever it steers something, including a menu
    /// or a dialogue. `bind_touch_virtual_inputs` maps `TouchVirtualStick` to
    /// both `Move` and `MenuStick`, and a hidden node takes no drags.
    #[test]
    fn the_move_stick_is_shown_wherever_it_steers_something() {
        fn stick_visibility(prompt_value: ControlPrompt) -> Visibility {
            let mut app = App::new();
            app.insert_resource(prompt_value);
            // Nobody owns gameplay — the dialogue/menu case.
            app.init_resource::<ambition_input::SeatInputContexts>();
            app.insert_resource(TouchControlsVisible(true));
            app.add_systems(Update, sync_touch_stick_visibility_from_context);
            let stick = app
                .world_mut()
                .spawn((TouchSurface::Movement, Visibility::Hidden))
                .id();
            app.update();
            *app.world()
                .get::<Visibility>(stick)
                .expect("the stick exists")
        }

        assert_eq!(
            stick_visibility(menu_prompt("Select")),
            Visibility::Inherited,
            "a menu or dialogue owns the seat and `MenuStick` is bound to this \
             stick, so it steers the selection and must be on screen"
        );
        assert_eq!(
            stick_visibility(ControlPrompt::default()),
            Visibility::Hidden,
            "the Empty context routes neither binding, and the standing rule is \
             that a control nobody can use must not be on screen"
        );
    }

    /// A dialogue's confirm button is shown and live. When a non-gameplay
    /// context owns the seat, `publish_frontend_context_prompt` rewrites the
    /// prompt to `Menu`, which selects Jump/Interact as confirm. Requiring
    /// `gameplay_owned()` there would hide them.
    #[test]
    fn a_dialogue_confirm_button_is_live_even_though_gameplay_does_not_own_input() {
        let m = menu_prompt("Select");
        assert!(
            touch_action_live(TouchActionButton::Jump, &m, false),
            "a menu/dialogue prompt nominates Jump as its confirm button, so it \
             must be drawn AND tappable while that context owns the seat"
        );
        assert!(
            !touch_action_live(TouchActionButton::Attack, &m, false),
            "Attack is not a menu verb — the prompt term still hides it"
        );
    }

    /// A stale gameplay prompt while nobody owns gameplay shows nothing. The
    /// prompt keeps its last value when no seat resolves an owner; this is the
    /// one case the ownership term covers.
    #[test]
    fn a_stale_gameplay_prompt_shows_nothing_while_nobody_owns_gameplay() {
        let g = prompt(
            ControlContextKind::Gameplay,
            vec![(ControlSlot::Jump, "Jump")],
        );
        assert!(
            !touch_action_live(TouchActionButton::Jump, &g, false),
            "a prompt still claiming Gameplay while nobody owns gameplay must \
             not offer gameplay verbs — the launcher's capturing claim over the \
             title screen is exactly this"
        );
        assert!(
            touch_action_live(TouchActionButton::Start, &g, false),
            "Start is a shell verb and stays live; hiding it is how a phone with \
             no keyboard loses its way out"
        );
    }

    /// Drawn and touchable agree for every combination. Checked as a property,
    /// so a second, different expression in either path fails.
    #[test]
    fn what_is_drawn_is_exactly_what_is_touchable() {
        let prompts = [
            prompt(
                ControlContextKind::Gameplay,
                vec![(ControlSlot::Jump, "Jump"), (ControlSlot::Attack, "Hit")],
            ),
            menu_prompt("Select"),
            ControlPrompt::default(),
        ];
        let actions = [
            TouchActionButton::Jump,
            TouchActionButton::Attack,
            TouchActionButton::Special,
            TouchActionButton::Burst,
            TouchActionButton::Interact,
            TouchActionButton::Start,
            TouchActionButton::Reset,
        ];
        for p in &prompts {
            for gameplay in [false, true] {
                for a in actions {
                    let drawn = touch_action_live(a, p, gameplay);
                    let mut edges = TouchButtonEdges::default();
                    set_button_held(&mut edges, a, true);
                    mask_unavailable(&mut edges, p, gameplay);
                    let touchable = held_of(&edges, a);
                    assert_eq!(
                        drawn, touchable,
                        "{a:?} is drawn={drawn} but touchable={touchable} \
                         (context {:?}, gameplay {gameplay}) — a button that is \
                         one and not the other is an invisible control or a dead \
                         visible one",
                        p.context,
                    );
                }
            }
        }
    }

    #[test]
    fn hidden_action_is_not_tappable_end_to_end() {
        // Drive the interaction system with a "pressed" Attack button the scheme lacks.
        let mut app = App::new();
        app.insert_resource(prompt(
            ControlContextKind::Gameplay,
            vec![(ControlSlot::Jump, "Jump")], // Attack absent from the scheme
        ));
        app.init_resource::<Touches>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.init_resource::<MobileTouchState>();
        app.init_resource::<TouchButtonEdges>();
        app.init_resource::<crate::placement::TouchControlPlacement>();
        app.add_systems(Update, update_buttons_from_interactions);
        app.world_mut()
            .spawn((Button, Interaction::Pressed, TouchActionButton::Attack));
        app.world_mut()
            .spawn((Button, Interaction::Pressed, TouchActionButton::Jump));
        app.update();

        let state = &app.world().resource::<MobileTouchState>().0;
        assert!(!state.attack.held, "hidden Attack must not register a hold");
        assert!(
            !state.attack.pressed_this_frame,
            "hidden Attack must not emit a press edge"
        );
        assert!(state.jump.held, "an available button still registers");
    }

    /// "Hidden" has to mean removed from layout, not resized to nothing.
    #[test]
    fn an_unplaced_touch_surface_leaves_the_layout_entirely() {
        let mut app = App::new();
        app.init_resource::<crate::placement::TouchControlPlacement>();
        app.add_systems(Update, apply_touch_control_placement);
        let surface = app
            .world_mut()
            .spawn((TouchSurface::Movement, Node::default()))
            .id();

        // Nothing published a footprint: the default placement has no rects.
        app.update();
        assert_eq!(
            app.world().get::<Node>(surface).unwrap().display,
            Display::None,
            "an unplaced surface was merely collapsed, so its absolutely-positioned \
             children still draw at the screen origin"
        );

        // And a surface that IS placed comes back.
        app.world_mut()
            .resource_mut::<crate::placement::TouchControlPlacement>()
            .movement = Some(
            ambition_platformer2d_shared_tangle::gameplay_presentation::ScreenRect::from_min_size(
                Vec2::new(40.0, 300.0),
                Vec2::new(160.0, 160.0),
            ),
        );
        app.update();
        let node = app.world().get::<Node>(surface).unwrap().clone();
        assert_eq!(
            node.display,
            Display::Flex,
            "hiding must be reversible: a surface that gains a footprint has to \
             come back, or the controls vanish for the rest of the session"
        );
        assert_eq!(node.left, Val::Px(40.0));
        assert_eq!(node.top, Val::Px(300.0));
    }
}
