//! The Bevy wiring: the touch HUD's spawn/despawn lifecycle and the collect
//! step that turns joystick + virtual-button UI state into the virtual
//! device's `MobileTouchState`.
//!
//! This is the crate's only ECS surface. `layout` computes where the controls
//! sit, `state` holds what they are doing, `virtual_device` exposes that
//! state to leafwing as bindable input kinds on the persistent participant,
//! and this module is what makes the controls exist in a running `App` and
//! collects them each frame. A touch device is a DEVICE, so everything here
//! belongs to the input layer.

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

/// Global z-band for the on-screen touch HUD (joystick + action /
/// menu buttons + bezel).
///
/// The touch HUD must render ABOVE every menu overlay AND win bevy_ui
/// picking against them, so the on-screen joystick keeps receiving
/// drags (which feed the participant's `MenuStick` binding) and the
/// action / Back buttons stay tappable while a menu is open.
///
/// Menu overlays sit at much lower stacking values: the OoT item grid
/// root uses local `ZIndex(62)`, the pause menu `ZIndex(50)`, the map
/// `ZIndex(60)`, and even the documented worst-case grid `GlobalZIndex(1000)`.
/// A `GlobalZIndex` establishes a global stacking context, so this single
/// high band lifts the whole HUD above any of them regardless of where the
/// menu roots live in the hierarchy. Picking in bevy_ui resolves
/// front-to-back by the same global stacking order, so a higher
/// `GlobalZIndex` here also means the HUD wins the pointer over a
/// full-screen menu scrim — the scrim no longer swallows HUD input.
pub const TOUCH_HUD_Z: i32 = 5000;

/// The joystick crate's own plugin and root marker, re-exported.
///
/// A composer that installs [`crate::placement::TouchPresentationPlugin`]
/// without the whole `TouchControlsPlugin` still needs the joystick itself for
/// the discovery step to have anything to find.
pub use virtual_joystick::{VirtualJoystickNode, VirtualJoystickPlugin};

/// Joystick id. The `virtual_joystick` plugin is generic over a
/// user-supplied id type; this enum picks Move (left stick) and
/// Aim (right stick).
#[derive(Default, Debug, Reflect, Hash, Clone, PartialEq, Eq)]
pub enum MobileStick {
    #[default]
    Move,
    Aim,
}

/// Live touch-input state. Updated each frame from the stick messages +
/// button state, then published as leafwing virtual-device controls into the
/// persistent participant's action state.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct MobileTouchState(pub TouchInputState);

/// Tracks the last non-control touch position used for menu drag
/// scrolling.
///
/// Bevy UI button `Interaction` covers taps on concrete rows.
/// This state is only for whole-panel gestures such as dragging
/// up/down to navigate a menu while another finger is still on
/// the movement stick. (Stick-driven menu navigation resolves through
/// the participant's `MenuStick` binding, not here.)
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct MenuTouchGestureState {
    pub(super) drag_scroll: DragScrollState,
}

/// Runtime VISIBILITY toggle for the on-screen touch UI. `true`
/// shows the stick + button HUD; `false` hides the overlay.
///
/// This controls ONLY the on-screen overlay's visibility; it does not
/// disable the virtual touch device. Touch enablement is owned by the plugin
/// itself (`TouchControlsPlugin`): the touch systems exist iff the plugin is
/// installed, so "rip touch out" = stop adding the plugin, not flip a boolean.
/// An untouched (even hidden) overlay publishes neutral virtual controls and
/// cannot stomp keyboard/gamepad input.
///
/// Flip it from the settings menu (the controls page "Touch Overlay"
/// row) or programmatically. No hotkey binding by design.
///
/// Default is `true` so the touch HUD shows immediately when the
/// `TouchControlsPlugin` is installed.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TouchControlsVisible(pub bool);

impl Default for TouchControlsVisible {
    fn default() -> Self {
        // The fold path is activity-gated; an idle touch HUD
        // doesn't stomp keyboard input.
        Self(true)
    }
}

/// Marker on every touch UI root (action cluster, menu row,
/// bezel) so the visibility-sync system can set `Visibility`
/// on all of them in one query.
#[derive(Component)]
pub struct MobileTouchUiRoot;

/// Which resolved control rectangle a root `Node` follows.
///
/// The overlay no longer positions itself from window corners: every root
/// carries this and is placed from [`TouchControlPlacement`], so the drawn
/// control, its touch region, and the layout that reserved space for it are
/// one thing rather than three formulas that agree by luck.
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
/// Absolute pixels rather than anchors: the resolver has already accounted for
/// the device-safe area and the reserved surround, so re-deriving an inset here
/// would just be a second opinion that can disagree.
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
            // Nothing published a footprint for this surface: the controls are
            // HIDDEN, and hiding has to mean hidden.
            //
            //  this collapsed the node to a zero rect and called it hidden. A
            // zero-size node still LAYS OUT, and every child here is
            // `PositionType::Absolute` — so the joystick art, the U/D/L/R glyphs
            // and the action labels all kept drawing, at the collapsed node's
            // origin, which is the top-left corner of the screen.
            //
            // `Display::None` removes the subtree from layout entirely, which is
            // the thing the old comment believed it was doing.
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
        if (font.font_size - scaled).abs() > f32::EPSILON {
            font.font_size = scaled;
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

/// Installs the on-screen touch joystick + action-button overlay and the
/// systems that feed touch input into the shared `ControlFrame` /
/// `MenuControlFrame` seams.
///
/// Touch enablement is owned by THIS plugin, not a runtime boolean: the touch
/// controls exist iff the plugin is installed. To rip touch out later, remove
/// the single `add_plugins(TouchControlsPlugin)` line in the app build — no
/// setting to flip. The `touch_controls_visible` setting only hides/shows the
/// on-screen overlay (see [`TouchControlsVisible`]); it does not enable or
/// disable the touch input itself.
pub struct TouchControlsPlugin;

impl Plugin for TouchControlsPlugin {
    fn build(&self, app: &mut App) {
        //  this overlay spawns TEXT and pins `.after(UiFontsLoaded)`. An empty
        // set makes that pin vacuous with no warning, so the consumer installs
        // the plugin that fills it rather than assuming the composition did.
        ambition_render::ui_fonts::UiFontsPlugin::ensure_installed(app);
        use leafwing_input_manager::plugin::{CentralInputStorePlugin, InputManagerSystem};
        use leafwing_input_manager::prelude::updating::InputRegistration;
        use leafwing_input_manager::prelude::RegisterUserInput;
        use leafwing_input_manager::InputControlKind;

        // ── Touch as a VIRTUAL DEVICE ─────────────────────────────────────
        //
        // The touch overlay is a leafwing input SOURCE, not a second input
        // system: `MobileTouchState` is collected in PreUpdate (below), the
        // registered input kinds publish it into leafwing's central store,
        // and `bind_touch_virtual_inputs` binds them in the persistent
        // participant's `InputMap` — from there, touch resolves through
        // bindings and the active input context exactly like a keyboard or
        // gamepad. No system in this crate writes `ControlFrame` or the
        // `MenuControlFrame` buttons/stick directly (drag-scroll, a pointer
        // gesture, is the one deliberate exception in `menu_bridge`).
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
        // between them. One declaration, shared with every composer including
        // the assembled tests.
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
            // Collect the virtual-device state BEFORE leafwing unifies input
            // kinds this frame: button interactions (bevy_ui focus ran just
            // before) and the joystick messages land in `MobileTouchState`,
            // then the registered kinds publish it into the central store —
            // a touch press this frame is an ActionState press this frame.
            .add_systems(
                PreUpdate,
                (read_joystick_messages, update_buttons_from_interactions)
                    .chain()
                    .after(bevy::ui::UiSystems::Focus)
                    .before(InputManagerSystem::Unify),
            )
            // Bind (and after a preset swap, re-bind) the virtual device in
            // the participant's InputMap.
            .add_systems(
                Update,
                crate::virtual_device::bind_touch_virtual_inputs
                    .in_set(ambition_input::InputSet::ResolveActions),
            )
            .add_systems(
                Update,
                (
                    position_frame_axis_glyphs,
                    // The pointer-gesture lane: drag-scroll joins the menu
                    // frame after the participant populate rebuilt it, before
                    // the menu consumers read it.
                    fold_touch_gestures
                        .in_set(ambition_input::InputSet::Route)
                        .after(ambition_platformer2d_actor_monolith::schedule::MenuFramePopulate)
                        //  ONE pin, not one per reader. Naming each reader set
                        // is a pin that stops covering them the day a third
                        // reader is added and nothing says so.
                        .before(ambition_platformer2d_actor_monolith::schedule::MenuFrameConsume),
                )
                    .chain(),
            )
            // Contextual button-label sync, decomposed into narrow
            // systems. `update_button_verb_from_prompt` reads the
            // `ControlPrompt` read-model (the controlled subject's own
            // action names) and writes the per-button `ButtonVerb`;
            // `sync_touch_button_visibility_from_prompt` hides buttons
            // for slots the scheme lacks; `render_touch_button_text`
            // folds whatever ButtonVerb/Glyph/Pressed components exist
            // into the Text node. The split lets the glyph subtitle and
            // pressed-state highlight ride alongside without growing one
            // god-system.
            .add_systems(
                Update,
                (
                    // Labels now come from the CONTROLLED subject's action scheme
                    // via the `ControlPrompt` read-model (not the fixed
                    // smash-vocabulary affordance table), and buttons for slots
                    // the scheme lacks are hidden.
                    update_button_verb_from_prompt,
                    sync_touch_button_visibility_from_prompt,
                    // The STICK, on the same rule, after the root sync so it
                    // wins over the blanket setting mirror rather than racing it.
                    sync_touch_stick_visibility_from_context.after(sync_touch_ui_visibility),
                    // After `Route`, where `update_seat_active_devices` runs — so the glyph
                    // reflects THIS frame's device flip.
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
            // Mirror keyboard / gamepad axis input onto the joystick
            // knob's visual position, so the on-screen joystick doubles
            // as an input display for non-touch devices. Runs after
            // `virtual_joystick`'s own `update_ui` (in
            // `JoystickSystems::UpdateUI`) so it overrides the
            // centered rest position the crate writes when no
            // `touch_state` is active. A real mouse / touch drag still
            // wins because we early-out when `touch_state.is_some()`.
            .add_systems(
                PostUpdate,
                // Between the behavior stage (which hard-resets `base_offset`
                // to ZERO every frame for `JoystickFixed`) and `update_ui`
                // (which derives BOTH the base ring and the knob from it).
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
/// The root node is the gesture-exclusion region and is placed flush to the
/// screen corner by `apply_touch_control_placement`. The art must sit
/// `JOYSTICK_MARGIN` in from the corner instead, clear of the edge and its
/// side-swipe gestures — the same inset the U/R/L/D glyphs use.
///
/// `base_offset` is the one value to write: `virtual_joystick`'s `update_ui`
/// positions the base ring at it AND derives the knob from it
/// (`base_offset + base_half + knob_half + base_half * (delta - 1)`), so a
/// single assignment moves the whole drawn stick together, at rest and
/// mid-drag. Setting the child nodes directly would fight that system every
/// frame, and root `padding` cannot work here: Bevy honours padding only for
/// absolutely-positioned children with AUTO offsets, and both the base and the
/// knob are given explicit `left`/`top` by the crate.
///
/// `JoystickFixed` also derives its input center from the base rect
/// (`touch_state.current - joystick_base_rect.center()`), so moving the art
/// moves the stick's input center with it rather than leaving the touch
/// response offset from what is drawn.
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

/// Spawn the two on-screen joysticks (Move + Aim) using a
/// procedural circle texture so the mobile_touch path doesn't
/// require a Knob.png art asset to render. Mouse-drag works on
/// desktop because virtual_joystick routes mouse + touch through
/// the same Interaction-driven path.
pub fn spawn_touch_joysticks(mut cmd: Commands, mut images: ResMut<Assets<Image>>) {
    let knob = images.add(build_joystick_knob_image());
    let outline = images.add(build_joystick_outline_image());

    // Single Move stick on the left. A touch joystick and a set of touch buttons." The Aim stick
    // was dropped -- for blink-aim, the right-stick gamepad path stays canonical, and on touch the
    // action buttons cover Blink as a tap (a future polish could add a directional gesture).
    // Joystick footprint is scaled by `TOUCH_SCALE` from the original 120x120 / 56x56 layout to
    // match the shrunken action cluster. Placement (and therefore the menu drag-scroll exclusion)
    // comes from the resolved control regions once `tag_virtual_joystick_root` marks this root; the
    // constants here are only the authored full-size shape.
    let layout = movement_joystick_layout();
    create_joystick(
        &mut cmd,
        MobileStick::Move,
        knob,
        outline,
        // Keep the idle stick visible but quieter; active drags are still
        // readable because the knob moves, and the button cluster brightens
        // under the user's finger through normal Bevy interaction tinting.
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
        // JoystickFixed: knob returns to base center on release (vs JoystickFloating which
        // leaves the knob where the touch lifted).
        JoystickFixed,
        NoAction,
    );
    // No floating "Move" overlay above the stick — the knob's drag position is the directional
    // indicator on its own.
    //
    // Tag the joystick UI root with MobileTouchUiRoot so the
    // visibility-sync system hides it alongside the bezel and
    // button cluster when `TouchControlsVisible(false)`. The
    // virtual_joystick crate spawns its own root node above; we
    // can't easily pass our marker through `create_joystick`,
    // so we attach the marker via a deferred query in
    // `tag_virtual_joystick_root` (added to the plugin's
    // Update systems).
    let _ = &mut cmd; // suppress unused mut warning when no follow-up insert
}

/// A U/D/L/R glyph overlaid on the move joystick, marking one axis of the
/// controlled character's local reference frame. `local_axis` is the local unit
/// direction (down `(0,1)`, up `(0,-1)`, right `(1,0)`, left `(-1,0)`).
/// `position_frame_axis_glyphs` places each label at the raw joystick direction
/// that resolves to that local command under the active input mapping mode.
#[derive(Component, Clone, Copy)]
pub struct FrameAxisGlyph {
    pub local_axis: Vec2,
}

/// Spawn the four reference-frame glyphs as a non-interactive overlay sharing the
/// move joystick's footprint. Tagged `MobileTouchUiRoot` so it hides with the rest
/// of the touch HUD.
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
        // Share the movement stick's resolved rect rather than re-deriving a
        // corner inset here. `TouchSurface` is a PLACEMENT marker only (nothing
        // hit-tests on it), so this stays non-interactive; the glyphs then sit
        // in the same coordinate space as the stick art and both orbit
        // `art_center`, which is what keeps them concentric.
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

/// Place each glyph at the raw INPUT-frame direction that maps to its local
/// controlled-character command. Gameplay and the touch labels share the same
/// inverse mapping, so labels move only when the active mapping policy says that
/// a different raw joystick direction now means local U/D/L/R.
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
    // Root-local center of the DRAWN stick, not of the reserved footprint the
    // root node spans — the same anchor the base ring and knob are inset to.
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

/// Find any `VirtualJoystickNode` entity that doesn't yet have
/// our `MobileTouchUiRoot` marker and add it. Runs each Update;
/// idempotent thanks to the `Without<MobileTouchUiRoot>` filter.
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
        // Lift the joystick into the HUD z-band along with the marker.
        // Without this the `virtual_joystick` root sits at the default
        // z (0) and a full-screen menu overlay/scrim renders on top of it
        // AND eats its pointer events, so dragging the on-screen stick
        // produces no `VirtualJoystickMessage` while a menu is open and
        // the virtual device never sees a stick deflection. The high
        // `GlobalZIndex` is the fix that makes the joystick a real
        // menu-nav source over the grid AND the cube.
        cmd.entity(entity).insert((
            MobileTouchUiRoot,
            GlobalZIndex(TOUCH_HUD_Z),
            TouchSurface::Movement,
        ));
    }
}

/// Procedural 64x64 RGBA knob: solid white circle with a soft
/// anti-aliased rim. Uses the same shape as
/// `body_mode::build_morph_ball_image` but with a flat white
/// fill so the knob_color tint controls the appearance.
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

/// Procedural 96x96 RGBA outline: ring with anti-aliased inner and
/// outer edges. Used as the joystick's stationary background circle;
/// tinted via background_color in `create_joystick`.
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

/// Mirror `TouchControlsVisible` onto every `MobileTouchUiRoot`
/// node. Bevy `Visibility` propagates to children, so flipping
/// the root nodes hides every button + bezel + stick UI in one
/// pass.
pub fn sync_touch_ui_visibility(
    visible: Res<TouchControlsVisible>,
    mut query: Query<&mut Visibility, With<MobileTouchUiRoot>>,
) {
    // Deliberately NOT gated on `visible.is_changed()`. Roots appear at
    // runtime — the joystick root is discovered and tagged in `Discover` just
    // above — and a root that first exists on a frame the setting did not
    // change would otherwise never be synced at all, staying visible under a
    // `TouchControlsVisible(false)` session forever. The per-entity `!=` below
    // keeps change detection honest, and there are a handful of roots.
    let target = if visible.0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut vis in &mut query {
        *vis = target;
    }
}

/// Mirror `UserSettings.controls.touch_controls_visible` into the
/// `TouchControlsVisible` resource. Runs every Update so the
/// settings-menu toggle takes effect on the same frame it changes.
/// Both values default to `true` so the HUD is on by default and
/// the user can flip it off via the controls page.
pub fn sync_touch_visibility_from_settings(
    settings: Res<ambition_persistence::settings::UserSettings>,
    mut visible: ResMut<TouchControlsVisible>,
) {
    if visible.0 != settings.controls.touch_controls_visible {
        visible.0 = settings.controls.touch_controls_visible;
    }
}

/// Per-button held-last-frame mask. Used by
/// `update_buttons_from_interactions` to derive
/// `pressed_this_frame` / `released_this_frame` edges from the
/// raw `Interaction::Pressed` reading.
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

/// Spawn the touch button UI. Layout follows a controller mental
/// model: a lower-right diamond for primary face buttons plus a
/// small shoulder row above it. Labels describe gameplay intent
/// ("Interact", "Jump", "Fly") rather than keyboard keys, so the
/// same HUD makes sense on desktop mouse testing and on an
/// Android phone.
fn spawn_touch_buttons(mut cmd: Commands, ui_fonts: Option<Res<UiFonts>>) {
    let ui_fonts = ui_fonts.as_deref();
    // -- Mobile HUD bezel + controller-style gameplay action cluster --
    // Right-thumb controls, bottom-right:
    //
    //       Blink        Fly        Shot
    //
    //                Interact
    //        Attack              Burst
    //                  Jump
    //
    // The cluster uses a compact diagonal diamond. Its circular hit-test
    // below matches the visible circles, so diagonal square bounds may
    // overlap without making the controls ambiguous.
    // The raw touch hit-test below consumes `touch_action_layout()` so
    // multitouch stays aligned with the rendered overlay.
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
        // High global z-band so the HUD renders above every menu overlay
        // AND wins bevy_ui picking over a full-screen menu scrim.
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
        // Above any menu overlay so the action buttons stay tappable while
        // a menu is open. `GlobalZIndex` also wins picking over the scrim.
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

    // -- Menu-style buttons (top-right) --
    // Start opens/closes the pause menu. Reset doubles as menu Back while a
    // menu is open; label it explicitly so phone users have a native escape
    // affordance without needing a keyboard Escape key.
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
        // Above any menu overlay: the always-available "Back" lives here
        // and must reach `MenuControlFrame.back` even while a menu's
        // full-screen scrim is up. `GlobalZIndex` lifts it above the scrim
        // for both render order and picking.
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
            font_size,
            ..default()
        })
}

/// Build one absolutely-positioned gameplay-action button inside
/// the right thumb cluster. Absolute placement keeps the visible
/// controller diamond and raw-touch hit testing in lock-step.
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
            // Pressed-state flag (Phase 3) lives on the Button entity
            // so `sync_button_pressed_visual` can mutate
            // `BackgroundColor` on the same entity that carries the
            // pressed bit — avoiding a parent-walk through ChildOf.
            ButtonPressed(false),
            Name::new(format!("Touch{label}")),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                touch_text_font(ui_fonts, font_size),
                TextColor(Color::srgb(0.96, 0.97, 1.0)),
                // Center both lines (verb + glyph) horizontally
                // inside the circular button. Without this, the
                // multiline text rendered by `render_touch_button_text`
                // left-justifies and the glyph subtitle drifts to the
                // left edge of the circle while the verb stays in its
                // own line; the eye reads them as mis-aligned.
                TextLayout::new_with_justify(Justify::Center),
                // Marker so the rendering system can find this text
                // node and rewrite it. Carries the canonical action
                // identity; the ButtonVerb / ButtonGlyph components
                // layered on top are what's actually rewritten each
                // frame.
                TouchActionLabel(action),
                // Component-driven verb display. Updated by
                // `update_button_verb_from_prompt` from the
                // `ControlPrompt` read-model; rendered into `Text` by
                // `render_touch_button_text`. Splitting these concerns
                // means each per-frame derived value (verb, glyph) gets
                // its own narrow update system instead of one god-system.
                ButtonVerb::new(label),
                // Per-device glyph subtitle (Phase 2). Empty until
                // `update_button_glyph_from_active_input` writes the
                // first frame's value, so cold-start renders the verb
                // alone without a phantom "?" subtitle.
                ButtonGlyph(Cow::Borrowed("")),
            ));
        });
}

/// Marker on the touch button's text node. Carries the
/// `TouchActionButton` identity so the verb-update system can map it
/// back to the correct control slot.
#[derive(Component)]
pub struct TouchActionLabel(pub TouchActionButton);

/// The verb-text to render under each touch button. Updated each
/// frame by [`update_button_verb_from_prompt`] from the
/// [`ambition_sim_view::ControlPrompt`] read-model. Held as
/// component data (not computed inline in the render system) so
/// independent concerns — verb, future glyph subtitle, future
/// pressed-state highlight — each own their own component + update
/// system and compose at render.
///
/// A button that is DRAWN with nothing to say therefore kept whatever it last said, from a context
/// that had ended.
///
/// That is not hypothetical. In a menu, Jump and Interact are drawn as the confirm pair, and they
/// take their verb from `prompt.menu_confirm` — which is published by
/// `install_menu_confirm_provider`, called from `ambition_app`'s kaleidoscope menu and NOWHERE
/// else.
///
/// As a struct, the fallback is never overwritten and the current verb is `Option`, so "the prompt
/// has nothing to say" resolves to the spawn label instead of to stale text.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ButtonVerb {
    /// Spawn-time fallback label. Only buttons without a contextual
    /// [`ControlSlot`] keep this text permanently; gameplay-slot labels come from
    /// the active prompt.
    fallback: &'static str,
    /// What the prompt says this frame, if anything. `String` (not
    /// `&'static str`) so authored `InteractVariant::Custom` prompts flow
    /// through unchanged.
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

/// What a menu's confirm button says when the menu did not name its own verb.
///
/// Generic on purpose: a specific word ("Play", "Advance", "Equip") is a game's
/// to publish through `ControlPrompt::menu_confirm`. This is only the guarantee
/// that the button never falls back to a GAMEPLAY verb, which is what it did
/// before — see `ButtonVerb`.
const DEFAULT_MENU_CONFIRM: &str = "Select";

/// The select-functional touch buttons: in a menu these fold into
/// `MenuControlFrame.select`, so they wear the menu's confirm verb.
fn is_menu_confirm_button(action: TouchActionButton) -> bool {
    matches!(
        action,
        TouchActionButton::Jump | TouchActionButton::Interact
    )
}

/// The dedicated menu-row buttons (Menu / Back) — always shown, never driven by
/// the gameplay scheme.
fn is_menu_button(action: TouchActionButton) -> bool {
    matches!(action, TouchActionButton::Start | TouchActionButton::Reset)
}

/// Per-frame: label each touch button from the [`ControlPrompt`] read-model.
///
/// - Gameplay: the CONTROLLED subject's own action names (possess a body →
///   the buttons rename); a slot the scheme lacks is left untouched (hidden by
///   [`sync_touch_button_visibility_from_prompt`]).
/// - Menu / Dialogue: the select-functional buttons (Jump / Interact) wear
///   the menu's confirm verb ("Select" / "Advance" / a specific item verb) so a
///   menu button never reads "Jump."
///
/// Reads the sim-published read-model, never the sim's live components.
pub fn update_button_verb_from_prompt(
    prompt: Res<ControlPrompt>,
    mut labels: Query<(&TouchActionLabel, &mut ButtonVerb)>,
) {
    for (TouchActionLabel(action), mut verb) in &mut labels {
        let next: Option<String> = match prompt.context {
            ControlContextKind::Gameplay => touch_button_slot(*action)
                .and_then(|slot| prompt.label_for(slot))
                .map(str::to_owned),
            ControlContextKind::Menu | ControlContextKind::Dialogue => {
                // The fallback belongs here rather than in each demo: what this
                // button does in a menu is CONFIRM, and that is true of every
                // composition that has a menu at all. A game with a better word
                // for it still says so through `menu_confirm`.
                is_menu_confirm_button(*action).then(|| {
                    prompt
                        .menu_confirm
                        .clone()
                        .unwrap_or_else(|| DEFAULT_MENU_CONFIRM.to_owned())
                })
            }
            ControlContextKind::Empty => None,
        };
        // Still change-detected, so `Changed<ButtonVerb>` stays honest for
        // `render_touch_button_text`.
        if verb.current != next {
            verb.current = next;
        }
    }
}

/// Whether a touch button is active in the current prompt context — the SINGLE
/// source of truth consumed by BOTH on-screen visibility AND raw-touch hit
/// testing, so a hidden button can never still be tapped at its old location.
///
/// - Gameplay: available iff the controlled subject's scheme carries the
///   button's slot (Sanic — no Attack/Shot/Shield — cannot fire them by tapping
///   the invisible circle).
/// - Menu / Dialogue: only the select-functional Jump / Interact and the
///   Menu / Back row.
/// - Empty (no controllable subject / cold start): only the Menu / Back row
///   — every gameplay action is hidden AND untappable, never a stale default.
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

/// Is this button LIVE this frame — drawn AND touchable?
///
///  the drawn overlay and its touch targets used to answer this with different expressions,
/// and the difference was exactly one term. Visibility asked `(gameplay || always_available)
/// && touch_action_available(..)`; the touch mask asked `touch_action_available(..)` alone,
/// under a comment claiming "one availability source of truth".
///
/// So both halves call THIS, and a button cannot be drawn and untouchable or
/// touchable and undrawn.
///
/// `Start` and `Reset` are exempt from the gameplay term on purpose: they are
/// shell verbs — pause, restart — and hiding them is how a phone with no keyboard
/// loses its way out.
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
    //  the gameplay-ownership term applies only to a GAMEPLAY prompt, and
    // narrowing it to that is what gives dialogue its confirm button back.
    //
    // `publish_frontend_context_prompt` already resolves this correctly: when a non-gameplay
    // context owns the seat it rewrites the prompt to `ControlContextKind::Menu` with that
    // context's own submit label, and `touch_action_available` then admits only the menu confirm +
    // menu row.
    //
    //  what the term still buys is the STALE case: the prompt keeps its last
    // value when no seat resolves an owner, so a prompt still claiming Gameplay
    // while nobody owns gameplay must not show gameplay verbs. That is exactly
    // the condition below, and nothing wider.
    if !matches!(prompt.context, ControlContextKind::Gameplay) {
        return true;
    }
    gameplay_owns_input
}

/// Zero the held flag of any action not LIVE this frame, so an unavailable
/// action never registers touch — even through the raw fixed-layout hit test —
/// and a held action that becomes unavailable produces a clean release edge
/// (never a stuck hold or a dangling pending press).
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
    // Start / Reset are always available — no mask.
}

/// Per-frame: show exactly the buttons that are available this frame (see
/// [`touch_action_available`]). Shown buttons use `Visibility::Inherited` (never
/// `Visible`) so they still obey the overlay-wide [`TouchControlsVisible`] root
/// toggle.
/// The movement STICK is a verb nobody can press either.
///
/// `sync_touch_button_visibility_from_prompt` below hides the gameplay action buttons while a menu
/// owns input, on the argument that a verb nobody can press must not be on screen. The rendered
/// test that covers this queries `TouchActionButton` and the stick is not one, so nothing caught
/// it.
///
/// Gated on `TouchSurface::Movement` rather than on the shared
/// `MobileTouchUiRoot`, because the action bezel carries Start and Reset — the
/// shell-shaped verbs a phone with no keyboard needs to keep its way out — and
/// hiding their root would take those with it.
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
    // The stick STEERS A MENU too, and hiding it there cost the player their
    // only way to move a selection.
    //
    // `bind_touch_virtual_inputs` maps `TouchVirtualStick` to BOTH `Move` and `MenuStick`, and
    // the axis writer is ungated — so while a menu or dialogue owns the seat the stick is a
    // working navigation control, and hiding it on `gameplay_owned()` alone hid a control that
    // does something.
    //
    // What `Empty` means is that nothing published a cue and nothing claimed the seat, so no
    // surface is READING those frames — the stick would steer nothing. `surface_prompt` in
    // `ambition_sim_view::control_prompt` is where that judgement is made.
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

pub fn sync_touch_button_visibility_from_prompt(
    prompt: Res<ControlPrompt>,
    // Whether GAMEPLAY owns the participant's actions this frame.
    //
    // Optional because this overlay composes into apps that never install the
    // participant-context resolver; absent, the old prompt-only behaviour stands
    // rather than the overlay vanishing.
    active_context: Option<Res<ambition_input::SeatInputContexts>>,
    mut buttons: Query<(&TouchActionButton, &mut Visibility)>,
) {
    // A verb nobody can press must not be on screen.
    //
    // The game-select screen showed the gameplay Jump and Interact buttons over
    // itself while a stranger chose a game, with the rest of the cluster's
    // entities present but already hidden by the prompt. The prompt read-model
    // was not wrong; it was answering a different question. It describes what
    // the CONTROLLED SUBJECT can do, and keeps describing that perfectly well
    // while a menu owns input — "can anybody drive it right now" is a question
    // about the input context, not about the body.
    //
    // Start and Reset are exempt. They are shell-shaped verbs — pause, restart —
    // and hiding them is how a phone with no keyboard loses its way out.
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

/// Per-frame: fold each button's [`ButtonVerb`] + [`ButtonGlyph`]
/// into the actual `Text` widget. Verb on the first line, optional
/// glyph in parentheses on the second line.
///
/// Re-runs only when one of the input components changed (the `Or<>`
/// filter), so steady-state frames don't churn the `Text` change-
/// detection bit.
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

/// Per-device glyph subtitle. Updated each frame from the primary seat's
/// active device (`SeatActiveDevices`) + the active [`KeyboardPreset`].
///
/// Today the active preset is sourced from a default
/// (`KeyboardPreset::arrows_zxc()`) because the sandbox does not yet
/// expose the player's current preset as a resource. When that
/// plumbing lands the `preset` local below can read from a
/// `Res<ActiveKeyboardPreset>` (or similar) — the glyph adapter
/// itself is already preset-agnostic.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ButtonGlyph(pub Cow<'static, str>);

/// Pressed-state flag (Phase 3). Set true while the underlying
/// `Platformer2dInputActionMonolith` is held this frame; consumed by
/// [`sync_button_pressed_visual`] to brighten the button background.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonPressed(pub bool);

/// Map a touch button to its canonical gameplay [`Platformer2dInputActionMonolith`].
///
/// Two maps over one enum is the shape that produced the gamepad glyph table's "if those bindings
/// change the table here needs to follow", and a third reader — the on-screen prompt, asking what
/// the Attack SLOT is bound to — would have needed a third table.
///
/// It COMPOSES now: button → slot → action, through
/// `ambition_input::action_for_slot`. Start and Reset are the honest residue —
/// they are shell verbs with no ability slot at all, which is exactly what
/// `touch_button_slot` already returns `None` for.
///
/// Returns `None` rather than inventing a fallback action for an unclassifiable
/// button. Such a button receives neither glyph nor press.
fn touch_action_to_sandbox_action(
    action: TouchActionButton,
) -> Option<Platformer2dInputActionMonolith> {
    match action {
        TouchActionButton::Start => Some(Platformer2dInputActionMonolith::Start),
        TouchActionButton::Reset => Some(Platformer2dInputActionMonolith::Reset),
        gameplay => touch_button_slot(gameplay).and_then(ambition_input::action_for_slot),
    }
}

/// Per-frame: write each button's glyph from the active input
/// device. Reads the primary seat's device from `SeatActiveDevices` + the
/// player's selected [`KeyboardPreset`] (from settings), so HUD glyphs
/// follow a rebind instead of always showing the out-of-the-box Z/X/C keys.
pub fn update_button_glyph_from_active_input(
    devices: Res<ambition_input::SeatActiveDevices>,
    settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    seat_bindings: Option<Res<ambition_input::SeatBindings>>,
    mut labels: Query<(&TouchActionLabel, &mut ButtonGlyph)>,
) {
    // Resolve the player's chosen keyboard preset from settings; fall
    // back to the default Arrows+ZXC when settings aren't present
    // (e.g. a headless/host config that never inserts UserSettings).
    let preset = settings
        .map(|s| KeyboardPreset::by_index(s.controls.keyboard_preset_index))
        .unwrap_or_else(KeyboardPreset::arrows_zxc);
    // The overlay is one device on one screen: the local primary seat's
    // bindings. An absent resource means nothing projected them yet (a headless
    // host, or a frame before the first projection), and an empty binding set
    // renders empty glyphs rather than stale ones.
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

/// Per-frame: write each button's pressed flag from the PRIMARY
/// participant's `ActionState<Platformer2dInputActionMonolith>`. Touch is a bound virtual
/// device now, so the same `ActionState` covers a finger on the overlay, a
/// mouse click on it, AND the keyboard/gamepad — one source lights the
/// button for every device. Skips writing when the value is unchanged so
/// the visual-sync system can filter on `Changed<ButtonPressed>`. Operates
/// on the Button entity (which carries both `TouchActionButton` and
/// `ButtonPressed`), so no parent walk is needed.
///
/// Primary, not `single()`. The overlay is one device on one screen —
/// the machine's own — so it lights from the primary seat's actions, the
/// same selection [`update_button_glyph_from_active_input`] makes for the
/// glyphs. A couch seat's pad must not light the machine's screen; and the
/// old `single()` read went dead the moment a second couch participant
/// spawned (two matches → `Err` → every button unlit).
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
        // An unclassifiable button reads as NOT held, which is the truthful
        // answer: nothing is driving it.
        let held = touch_action_to_sandbox_action(*touch_action)
            .zip(actions)
            .is_some_and(|(sa, a)| a.pressed(&sa));
        if pressed.0 != held {
            pressed.0 = held;
        }
    }
}

/// Per-frame: when [`ButtonPressed`] flips, swap the button's
/// background color so the on-screen overlay doubles as a streamer-
/// style input display.
pub fn sync_button_pressed_visual(
    mut buttons: Query<(&ButtonPressed, &mut BackgroundColor), Changed<ButtonPressed>>,
) {
    for (pressed, mut bg) in &mut buttons {
        bg.0 = if pressed.0 {
            // Brighter, opaque when held — reads as "this is the
            // input I'm pressing right now."
            Color::srgba(0.42, 0.58, 0.95, 0.78)
        } else {
            // Match the default authored in `spawn_action_button_at`.
            Color::srgba(0.16, 0.19, 0.27, 0.38)
        };
    }
}

/// Build one menu-row button. Used for Menu / Back, which are
/// intermittent and live away from the gameplay action diamond.
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

/// Walk every `TouchActionButton` entity, read its `Interaction`,
/// and fold (held vs pressed/released edges) into
/// `MobileTouchState.<button>`. Edges are derived against the
/// previous frame's held mask in `TouchButtonEdges`.
fn update_buttons_from_interactions(
    query: Query<(&Interaction, &TouchActionButton), With<Button>>,
    touches: Res<Touches>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    placement: Res<crate::placement::TouchControlPlacement>,
    prompt: Res<ControlPrompt>,
    // The SAME term the visibility pass reads. Optional for the same reason it is
    // optional there: this overlay composes into apps that never install the
    // participant-context resolver, and absent, the prompt-only behaviour stands.
    active_context: Option<Res<ambition_input::SeatInputContexts>>,
    mut state: ResMut<MobileTouchState>,
    mut edges: ResMut<TouchButtonEdges>,
) {
    let gameplay = active_context
        .as_deref()
        .is_none_or(|seats| seats.primary().gameplay_owned());
    let mut now = TouchButtonEdges::default();

    // Desktop / editor path: Bevy UI interactions are enough for
    // mouse-driven button testing.
    for (interaction, action) in &query {
        let held = matches!(interaction, Interaction::Pressed);
        set_button_held(&mut now, *action, held);
    }

    // Android / real-touch path: Bevy's Button `Interaction` is not a reliable multitouch
    // source while another finger owns the virtual joystick. This lets the player keep the left
    // thumb on the move stick while tapping Jump / Attack / Burst with the right thumb. The
    // rectangles come from the RESOLVED placement, not from the window: a cluster reserved into
    // a surround column must be tappable where it is drawn. Deriving them here from window size
    // again is what let the drawn overlay and its touch targets drift apart.
    let cluster = placement.action_cluster;
    let menu_row = placement.menu_row;
    for touch in touches.iter() {
        if let Some(action) = touch_action_at_position(touch.position(), cluster, menu_row) {
            set_button_held(&mut now, action, true);
        }
    }

    // Desktop touch-HUD testing path: raw mouse hit testing mirrors the
    // Android raw-touch path, so the visible controller-like overlay can
    // be exercised even when another UI panel would otherwise consume
    // normal Bevy `Button` interaction.
    if mouse_buttons.pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor) = window.cursor_position() {
                if let Some(action) = touch_action_at_position(cursor, cluster, menu_row) {
                    set_button_held(&mut now, action, true);
                }
            }
        }
    }

    // `touch_action_live` is now literally the one source of truth — the visibility pass calls
    // the same function with the same two inputs, so the drawn overlay and its touch targets
    // cannot disagree.
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

/// Read one action's held flag back out of the edge mask.
///
/// The inverse of [`set_button_held`], and it exists so a test can ask "is this
/// action touchable" without restating the match — a second copy of that mapping
/// is how the two halves this file just unified got out of step in the first
/// place.
///
///  `cfg(test)`: the production path only ever WRITES the mask, so leaving this
/// ungated is a `dead_code` warning — and `no warnings (cargo check
/// --all-targets)` is one of the suite's four jobs, so it would have been a red
/// I armed myself.
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

/// Decide whether `drive_joystick_knob_from_axis` should override the
/// knob position with the gameplay MOVE axis this frame.
///
/// The override mirrors the gameplay axis onto the knob so the stick
/// doubles as an input display for keyboard / gamepad. But while a menu
/// or the launcher owns input, the gameplay `ControlFrame` is neutral (the
/// context/mode routing suppresses it), so applying the override would
/// snap the knob back to center even as the player drags it to navigate
/// the menu.
///
/// Keyed on the [`ControlPrompt`]'s resolved context — the same
/// action/cue contract that drives the button labels — never on
/// `GameMode` or actor presence: the presenter asks "does gameplay own
/// the controls right now", not "which mode is the game in".
pub fn axis_override_drives_knob(context: ControlContextKind) -> bool {
    // Only mirror the gameplay axis onto the knob while gameplay owns
    // the controls. Menu / Dialogue / Empty (launcher, startup) all
    // consume the stick through the menu seam instead.
    matches!(context, ControlContextKind::Gameplay)
}

/// Mirror keyboard / gamepad axis input onto the on-screen joystick
/// knob's visual position, so the touch HUD doubles as an input
/// display for non-touch devices.
///
/// When a real drag is in progress (`state.touch_state.is_some()`),
/// this system bails out and lets `virtual_joystick`'s built-in
/// `update_ui` drive the knob from the actual touch / mouse cursor
/// — the drag is the authoritative source. Otherwise, we override
/// the centered rest position the crate wrote with a knob offset
/// derived from `ControlFrame.axis_x` / `axis_y`, using the same
/// circle-bounded math the crate's `update_ui` uses.
///
/// While a MENU is open (`!allows_gameplay()`), the whole override is
/// skipped (see [`axis_override_drives_knob`]): touch is routed to the
/// menu frame, so the gameplay axis is ~0 and overriding would snap
/// the knob to center. Skipping lets `virtual_joystick`'s `update_ui`
/// keep the knob on the live drag so it follows the finger as the
/// player navigates the menu.
///
/// Convention: `ControlFrame.axis_*` already follows the sim's
/// +Y-down convention, which matches Bevy UI's +Y-down `Node.top`
/// axis, so no Y inversion is needed here.
fn drive_joystick_knob_from_axis(
    prompt: Res<ControlPrompt>,
    control_frame: Res<ControlFrame>,
    joystick_q: Query<(&VirtualJoystickState, &Children), With<VirtualJoystickNode<MobileStick>>>,
    base_q: Query<&ComputedNode, With<VirtualJoystickUIBackground>>,
    mut knob_q: Query<(&mut Node, &ComputedNode), With<VirtualJoystickUIKnob>>,
) {
    // While a menu owns the controls the gameplay axis is ~0. Skip the
    // override entirely so `virtual_joystick`'s `update_ui` keeps the knob
    // on the live drag and it follows the finger during menu navigation.
    if !axis_override_drives_knob(prompt.context) {
        return;
    }
    // Treat axes inside ±1e-3 as "no input." Below this the knob must
    // snap to the base's center regardless of any active or stale
    // `state.touch_state`: on Android the crate occasionally holds a
    // non-`None` touch_state after release, which left the knob pinned
    // bottom-right of the base ring even with zero stick input. The
    // stick-active gate in the menu_bridge fold already prevents this
    // tiny dead-band from contributing to gameplay.
    const NEUTRAL_EPS: f32 = 1.0e-3;

    for (state, children) in &joystick_q {
        let axis_raw = Vec2::new(
            control_frame.axis_x.clamp(-1.0, 1.0),
            control_frame.axis_y.clamp(-1.0, 1.0),
        );
        let neutral = axis_raw.x.abs() < NEUTRAL_EPS && axis_raw.y.abs() < NEUTRAL_EPS;

        // Real drag wins -- but only while the axis is actually moving.
        // The crate's `update_ui` already placed the knob from
        // `state.delta` based on the live cursor, so we don't fight it
        // there. A neutral axis means we DO need to override (see the
        // NEUTRAL_EPS comment above).
        if state.touch_state.is_some() && !neutral {
            continue;
        }
        let mut base_size: Option<Vec2> = None;
        let mut knob_entity: Option<Entity> = None;
        for child in children.iter() {
            if let Ok(base) = base_q.get(child) {
                // Multiply by `inverse_scale_factor` so we read sizes in
                // *logical* pixels, matching the `Val::Px` units we
                // write back to `Node.left` / `Node.top`. On Android
                // the window scale factor is typically 2.5–3×, so the
                // raw `ComputedNode::size()` (which is *physical*
                // pixels) overshoots by that factor and parks the knob
                // bottom-right of the base ring. Mirrors the crate's
                // own `update_ui` (see virtual_joystick::systems::
                // node_rect, which scales the same way before doing
                // the same math).
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

        // Clamp to the unit circle so diagonal inputs ride the rim of
        // the base ring instead of overshooting into the corners (which
        // would push the knob outside the visible circle). Matches the
        // crate's `joystick_delta` circular clamp.
        let mag_sq = axis_raw.length_squared();
        let axis = if mag_sq > 1.0 {
            axis_raw / mag_sq.sqrt()
        } else {
            axis_raw
        };

        // Anchor the knob's *center* on the base's center, then offset
        // by the axis vector scaled to the knob's travel radius
        // (`base_half - knob_half`, so a full deflection keeps the knob
        // fully inside the ring). `Node.left`/`Node.top` address the
        // knob's top-left corner, so subtract `knob_half` to land its
        // center at the target. Prior formula assumed `knob_size ==
        // base_size / 2` and left the knob ~4 px off on desktop
        // (cosmetically fine there) and visibly down-right on Android
        // where DPI scaling magnified the error.
        // Same `art_origin` the base ring is inset by (see
        // `offset_joystick_art_within_footprint`) — this system overrides the
        // crate's rest placement, so it has to agree with it or the knob
        // snaps back to the footprint corner the moment the axis goes neutral.
        let art_origin = movement_joystick_layout().art_origin();
        let travel = base_half - knob_half;
        let center_left = art_origin.x + base_half.x - knob_half.x;
        let center_top = art_origin.y + base_half.y - knob_half.y;
        let target_left = center_left + travel.x * axis.x;
        let target_top = center_top + travel.y * axis.y;
        let new_left = Val::Px(target_left);
        let new_top = Val::Px(target_top);
        // Avoid thrashing Bevy's change-detection bit on idle frames
        // where the axis hasn't moved.
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

/// Read every `VirtualJoystickMessage<MobileStick>` published this
/// frame and update the `MobileTouchState`. The plugin emits a
/// stream of axis updates per touch; we keep the latest reading
/// per stick.
fn read_joystick_messages(
    mut reader: MessageReader<VirtualJoystickMessage<MobileStick>>,
    mut state: ResMut<MobileTouchState>,
) {
    for msg in reader.read() {
        // `axis()` returns the joystick delta in -1..=1 per axis
        // (this is what we want as a stick reading). `value()`
        // looks superficially right but actually returns the raw
        // mouse/touch *pixel position*, so reading it produced
        // huge always-positive numbers that the downstream
        // deadzone normalized to roughly (+0.13, +0.99) regardless
        // of drag direction -- "joystick only moves right slowly".
        // `snap_axis()` is also available but emits discrete
        // -1/0/+1 past a 0.5 deadzone, killing analog feel; we
        // prefer raw axis + the engine's own deadzone.
        //
        // Cardinal press EDGES are not derived here: the
        // `TouchStickDirection` virtual buttons publish the held
        // threshold state and leafwing derives the edge from the
        // transition, exactly as for a gamepad stick direction —
        // so double-tap detectors see honest taps, never a held
        // direction repeated every frame.
        let axis = msg.axis();
        match msg.id() {
            MobileStick::Move => {
                state.0.move_x = axis.x;
                // Bevy's UI Y increases UPWARD; the simulator's +Y
                // is downward. Flip so the touch stick matches the
                // desktop convention (drag down -> axis_y > 0).
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
                    // These fixtures are about the touch overlay's LABELS; the
                    // physical binding is `SeatBindings`' answer and a separate
                    // test covers it.
                    binding: None,
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

    /// The Utility button is named by the subject, not by the engine — and it
    /// falls back only when the subject says nothing.
    ///
    /// Both directions in one test, because the pair IS the invariant and the
    /// half that was believed (permanently "Fly") is the half that was false.
    ///
    /// Poisoned on purpose: the verb is a word no game in this repo uses, so a
    /// resolution that hardcoded any real label — "Fly", "Transform", Sanic's
    /// anything — fails here. What is pinned is that the WORD TRAVELS, for any
    /// game, not that some particular game's word arrives.
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

        // Gate 5: the Special button follows the scheme's Special slot — shown +
        // tappable only for a special-bearing body, hidden + untappable otherwise.
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

    /// Every button a dialogue SHOWS reads what it actually does.
    ///
    /// Driven through the real systems — `update_button_verb_from_prompt` then
    /// `render_touch_button_text` — so it asserts the rendered `Text`, not an
    /// intermediate. And it asserts over the buttons that are LIVE, because a
    /// stale label on a hidden button is invisible while a stale label on a
    /// shown one is the complaint.
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

    /// A menu that names no confirm verb still labels its buttons honestly —
    /// the Sanic case, and the one that actually bit.
    ///
    /// *"I know in sanic the button text doesn't match what the
    /// controls really are."* `menu_confirm` is published by
    /// `install_menu_confirm_provider`, which only `ambition_app`'s kaleidoscope
    /// menu calls — so in every demo it is `None`, the old `.flatten()` wrote
    /// nothing, and the confirm pair kept the verbs of the gameplay the player
    /// had just left.
    ///
    ///  the sibling test above passes a verb and would pass against the broken
    /// code, because supplying one is exactly what hides this. The distinguishing
    /// input is `menu_confirm: None`, so that is what this drives.
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

    /// The move stick is shown wherever it STEERS something, which includes
    /// a menu or a dialogue — not only gameplay.
    ///
    /// `bind_touch_virtual_inputs` maps `TouchVirtualStick` to both `Move` and
    /// `MenuStick`, so while a menu owns the seat the stick is a working
    /// navigation control. Hiding it on `gameplay_owned()` alone left a phone
    /// player with no way to move a dialogue selection — and a hidden node takes
    /// no drags, so it was genuinely dead rather than merely invisible.
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

    /// A dialogue's confirm button is SHOWN and LIVE — the report's third
    /// failure, and the one the ownership term caused.
    ///
    /// When a non-gameplay context owns the seat, `publish_frontend_context_prompt`
    /// rewrites the prompt to `Menu` with that context's submit label, so the
    /// prompt has already nominated Jump/Interact as the way to confirm. ANDing
    /// `gameplay_owned()` over that hid exactly those buttons, which is why the
    /// right-hand cluster vanished during dialogue on a phone and left no
    /// reliable way to confirm a choice.
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

    /// A STALE gameplay prompt while nobody owns gameplay shows nothing —
    /// the fix, preserved and now stated as its own case.
    ///
    /// The prompt keeps its last value when no seat resolves an owner, so this
    /// is the one condition the ownership term still buys. Narrowing it to a
    /// gameplay-context prompt is what let the dialogue case above work without
    /// giving this one back.
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

    /// Drawn and touchable are the same question, over every combination.
    ///
    /// It is asserted as a PROPERTY rather than by checking that both call one function, because
    /// what must not come back is a second expression — and a second expression would pass a "do
    /// they call the same fn" test by simply not calling it.
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
