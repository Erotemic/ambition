//! Mobile / touch input adapter for the Android demo path.
//!
//! Goal: a sideloadable Pixel-class APK where the sandbox is playable
//! with on-screen joysticks + controller-like touch buttons. The Leafwing keyboard/gamepad
//! pipeline is the canonical desktop input surface; this module
//! translates touch joystick + virtual buttons into the same
//! `ControlFrame` resource the simulator already consumes.
//!
//! Two layers:
//!
//! 1. **Pure helper (this module, always built)** —
//!    `fold_touch_into_control_frame` takes a `TouchInputState` plus
//!    a deadzone and returns a `ControlFrame`. Pure data, unit-tested,
//!    no Bevy / `virtual_joystick` dep. This is what RL agents,
//!    tests, and the Bevy systems all share.
//!
//! 2. **Bevy plugin (gated behind `mobile_touch`)** — wires
//!    `virtual_joystick` Move + Aim sticks plus a small button UI to
//!    the helper, then writes `ControlFrame`. Lives in
//!    `mobile_input::bevy::*`.
//!
//! See `TODO.md` → "Android demo touch controls" for the full plan.

use crate::input::ControlFrame;

/// Edge-vs-held button state. Two flags per button so the sim's
/// "pressed this frame" semantics survive the touch path. The Bevy
/// systems compute these by diffing per-frame against the last
/// frame's pressed mask.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TouchButton {
    /// True if the button is currently held.
    pub held: bool,
    /// True if the button was newly pressed this frame.
    pub pressed_this_frame: bool,
    /// True if the button was released this frame.
    pub released_this_frame: bool,
}

impl TouchButton {
    pub const fn off() -> Self {
        Self {
            held: false,
            pressed_this_frame: false,
            released_this_frame: false,
        }
    }

    pub const fn pressed_now() -> Self {
        Self {
            held: true,
            pressed_this_frame: true,
            released_this_frame: false,
        }
    }

    pub const fn held_continued() -> Self {
        Self {
            held: true,
            pressed_this_frame: false,
            released_this_frame: false,
        }
    }
}

/// One frame of mobile-touch input: two analog sticks (Move + Aim) plus
/// the gameplay-relevant action buttons. Mirrors the
/// `SandboxAction` set on the desktop side.
#[derive(Clone, Copy, Debug, Default)]
pub struct TouchInputState {
    /// Left stick raw value `[-1, 1]` (pre-deadzone).
    pub move_x: f32,
    pub move_y: f32,
    /// Edge flags: true on the frame the move stick crossed the
    /// up/down threshold (in either direction). The Bevy plugin
    /// computes these by diffing against the previous frame's
    /// `move_y`; tests / RL agents can set them directly. Auto-
    /// deriving from `move_y > 0.5` per frame is incorrect because
    /// `register_down_tap` would count every held frame as a
    /// fresh tap and trigger MorphBall on the second frame.
    pub move_y_just_crossed_up: bool,
    pub move_y_just_crossed_down: bool,
    /// Right stick raw value `[-1, 1]` (pre-deadzone).
    pub aim_x: f32,
    pub aim_y: f32,
    pub jump: TouchButton,
    pub attack: TouchButton,
    pub dash: TouchButton,
    pub blink: TouchButton,
    pub interact: TouchButton,
    pub projectile: TouchButton,
    pub fly_toggle: TouchButton,
    pub start: TouchButton,
    pub reset: TouchButton,
}

/// Apply a circular deadzone to an analog stick reading. Mirrors the
/// `ControlSettings::apply_deadzone` shape from the desktop input
/// pipeline so touch and stick feel identical at the seam.
pub fn apply_deadzone(x: f32, y: f32, deadzone: f32) -> (f32, f32) {
    let mag = (x * x + y * y).sqrt();
    if mag <= deadzone {
        return (0.0, 0.0);
    }
    // Re-scale so the post-deadzone magnitude reaches 1.0 at full
    // stick deflection rather than a clipped (1 - deadzone). Same
    // approach as the desktop deadzone helper.
    let scaled = (mag - deadzone) / (1.0 - deadzone).max(1e-6);
    let scaled = scaled.clamp(0.0, 1.0);
    let inv_mag = if mag > 1e-6 { 1.0 / mag } else { 0.0 };
    (x * inv_mag * scaled, y * inv_mag * scaled)
}

/// Fold a `TouchInputState` into the engine's `ControlFrame` shape.
///
/// `move_deadzone` and `aim_deadzone` are the per-stick deadzone
/// magnitudes; the desktop pipeline's `ControlSettings` holds the
/// canonical values, but the touch path can pick its own (touch
/// sticks usually have no drift, so a smaller deadzone like 0.05 is
/// enough). Pass 0.0 to disable.
///
/// Pure function — no Bevy / world / globals — so tests can pin every
/// edge case (sign convention, deadzone, button semantics) without
/// touching the rest of the engine.
pub fn fold_touch_into_control_frame(
    state: TouchInputState,
    move_deadzone: f32,
    aim_deadzone: f32,
) -> ControlFrame {
    let (move_x, move_y_raw) = apply_deadzone(state.move_x, state.move_y, move_deadzone);
    let (aim_x, aim_y_raw) = apply_deadzone(state.aim_x, state.aim_y, aim_deadzone);
    // The simulation's +Y is downward (screen-space). Touch joysticks
    // typically follow the same convention if mapped to "drag down =
    // axis_y > 0". Caller is responsible for matching that
    // convention before this function; we don't flip here.
    let move_y = move_y_raw;
    let aim_y = aim_y_raw;

    // Up / Down edge flags come from the caller explicitly (set on
    // the frame the move-Y axis crosses the threshold, cleared
    // next frame). Auto-deriving from "move_y > 0.5" every frame
    // breaks register_down_tap which counts each consecutive
    // true as a fresh tap and double-taps into MorphBall after one
    // held frame -- the same bug class as the AgentAction
    // converter; same fix.
    let up_pressed = state.move_y_just_crossed_up;
    let down_pressed = state.move_y_just_crossed_down;

    ControlFrame {
        axis_x: move_x,
        axis_y: move_y,
        jump_pressed: state.jump.pressed_this_frame,
        jump_held: state.jump.held,
        jump_released: state.jump.released_this_frame,
        dash_pressed: state.dash.pressed_this_frame,
        up_pressed,
        down_pressed,
        fast_fall_pressed: false,
        blink_pressed: state.blink.pressed_this_frame,
        blink_held: state.blink.held,
        blink_released: state.blink.released_this_frame,
        attack_pressed: state.attack.pressed_this_frame,
        pogo_pressed: false,
        fly_toggle_pressed: state.fly_toggle.pressed_this_frame,
        interact_pressed: state.interact.pressed_this_frame,
        reset_pressed: state.reset.pressed_this_frame,
        start_pressed: state.start.pressed_this_frame,
        projectile_pressed: state.projectile.pressed_this_frame,
        projectile_held: state.projectile.held,
        projectile_released: state.projectile.released_this_frame,
        aim_x,
        aim_y,
    }
}

/// Bevy plugin wiring `virtual_joystick` to the `ControlFrame` seam.
/// Gated behind the `mobile_touch` feature so desktop / gamepad /
/// headless / RL builds don't pull in `virtual_joystick` and don't
/// register the touch systems.
///
/// Today the plugin only wires the two analog sticks (Move + Aim);
/// touch buttons for Jump / Attack / Dash / Blink / Interact /
/// Projectile / Start / Reset are documented as a follow-up. RL
/// agents and tests can still produce a `TouchInputState` directly
/// and call `fold_touch_into_control_frame` from any code path.
#[cfg(feature = "mobile_touch")]
pub mod bevy_plugin {
    use super::{fold_touch_into_control_frame, TouchButton, TouchInputState};
    use crate::input::{ControlFrame, MenuControlFrame, MenuInputState};
    use bevy::input::mouse::MouseButton;
    use bevy::input::touch::Touches;
    use bevy::prelude::*;
    use bevy::window::PrimaryWindow;
    use virtual_joystick::*;

    /// Joystick id. The `virtual_joystick` plugin is generic over a
    /// user-supplied id type; this enum picks Move (left stick) and
    /// Aim (right stick).
    #[derive(Default, Debug, Reflect, Hash, Clone, PartialEq, Eq)]
    pub enum MobileStick {
        #[default]
        Move,
        Aim,
    }

    /// Live touch-input state. Updated each frame from the stick
    /// messages + button state. The folder system reads this and
    /// writes the canonical `ControlFrame`.
    #[derive(Resource, Default, Clone, Copy, Debug)]
    pub struct MobileTouchState(pub TouchInputState);

    /// Tracks the last non-control touch position used for menu drag scrolling.
    ///
    /// Bevy UI button `Interaction` covers taps on concrete rows. This state is
    /// only for whole-panel gestures such as dragging up/down to navigate a
    /// menu while another finger is still on the movement stick.
    #[derive(Resource, Default, Clone, Copy, Debug)]
    pub struct MenuTouchGestureState {
        last_pos: Option<Vec2>,
        drag_scroll_accum: f32,
        stick_input: MenuInputState,
    }

    /// Runtime visibility toggle for the touch UI. `true` shows the
    /// stick + button HUD; `false` hides the elements and zeroes
    /// the touch input contribution to ControlFrame so neither
    /// path stomps the desktop input.
    ///
    /// Per Jon's "we also need a toggle for touch controls, so we
    /// can disable them in the desktop version of the game, but
    /// also still test them there." This is a Bevy resource —
    /// flip it from the pause menu / settings menu (preferred,
    /// see TODO row) or programmatically from code. No hotkey
    /// binding by design: Jon also asked us to "move all of these
    /// options into settings" so the canonical non-hotkey place
    /// for the toggle is the settings menu, not an F-key.
    ///
    /// Default is `true` so the touch HUD shows immediately when
    /// the game launches with `--features mobile_touch`.
    #[derive(Resource, Clone, Copy, Debug)]
    pub struct TouchControlsVisible(pub bool);

    impl Default for TouchControlsVisible {
        fn default() -> Self {
            // Default to TRUE so the touch HUD is visible on all
            // platforms when `mobile_touch` is enabled. Per Jon's
            // 2026-05-07 feedback, the HUD should be visible by
            // default on desktop too so it can be tested with a
            // mouse. Flipping to false will be the user's choice
            // via the settings menu (TODO).
            //
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

    pub struct MobileTouchPlugin;

    impl Plugin for MobileTouchPlugin {
        fn build(&self, app: &mut App) {
            app.add_plugins(VirtualJoystickPlugin::<MobileStick>::default())
                .insert_resource(MobileTouchState::default())
                .insert_resource(MenuTouchGestureState::default())
                .insert_resource(TouchButtonEdges::default())
                .insert_resource(TouchControlsVisible::default())
                .add_systems(Startup, (spawn_touch_buttons, spawn_touch_joysticks))
                .add_systems(
                    Update,
                    (
                        tag_virtual_joystick_root,
                        sync_touch_visibility_from_settings,
                        sync_touch_ui_visibility,
                        read_joystick_messages,
                        update_buttons_from_interactions,
                        fold_to_menu_control_frame
                            .after(crate::app::populate_menu_control_frame_from_actions)
                            .before(crate::app::apply_menu_frame_to_cutscene_request)
                            .before(crate::pause_menu::pause_menu_toggle),
                        fold_to_control_frame
                            // Touch fold MUST run AFTER the keyboard
                            // fold (`populate_control_frame_from_actions`)
                            // so the OR-merge sees the keyboard's
                            // contribution to ControlFrame instead of
                            // racing with it. Without this ordering,
                            // populate_control_frame_from_actions can
                            // run AFTER fold_to_control_frame, which
                            // resets ControlFrame to defaults / leafwing's
                            // values and stomps the touch button merge.
                            .after(crate::app::populate_control_frame_from_actions)
                            // ALSO run before `sandbox_update` so the
                            // merged ControlFrame is visible to the sim
                            // on the same frame. Without this, Bevy is
                            // free to schedule fold after sandbox_update,
                            // and one-frame `pressed` edges (Jump /
                            // Attack / Dash / Blink / Interact / Reset /
                            // Start) never reach the engine -- they vanish
                            // when populate resets ControlFrame the next
                            // frame. Held axes have the same issue:
                            // sandbox_update sees axis_x = 0 because the
                            // touch fold hasn't written yet. Projectile
                            // happened to work only because `held` and
                            // `released` persist across frames in the
                            // touch state, masking the ordering bug.
                            .before(crate::app::sandbox_update)
                            // ALSO run before pause_menu_toggle so the
                            // touch Start press is in ControlFrame before
                            // pause_menu_toggle reads it. The pause /
                            // inventory / navigate chain in app.rs is
                            // ordered after populate_control_frame_from_actions,
                            // and our fold runs after populate; this
                            // .before(pause_menu_toggle) wins the tie
                            // so fold also runs before pause.
                            .before(crate::pause_menu::pause_menu_toggle),
                    )
                        .chain(),
                );
        }
    }

    /// Spawn the two on-screen joysticks (Move + Aim) using a
    /// procedural circle texture so the mobile_touch path doesn't
    /// require a Knob.png art asset to render. Mouse-drag works on
    /// desktop because virtual_joystick routes mouse + touch through
    /// the same Interaction-driven path.
    ///
    /// Per Jon's "make mobile_touch overlay intentionally testable
    /// with a mouse on desktop builds. ... mouse is a single-pointer
    /// debug path, not a replacement for real multitouch testing."
    fn spawn_touch_joysticks(mut cmd: Commands, mut images: ResMut<Assets<Image>>) {
        let knob = images.add(build_joystick_knob_image());
        let outline = images.add(build_joystick_outline_image());

        // Single Move stick on the left. Per Jon's 2026-05-07
        // feedback "We only need one for this game. A touch joystick
        // and a set of touch buttons." The Aim stick was dropped --
        // for blink-aim, the right-stick gamepad path stays
        // canonical, and on touch the action buttons cover Blink as
        // a tap (a future polish could add a directional gesture).
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
            Vec2::new(56.0, 56.0),
            Vec2::new(120.0, 120.0),
            Node {
                width: Val::Px(120.0),
                height: Val::Px(120.0),
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                bottom: Val::Px(24.0),
                ..default()
            },
            // JoystickFixed: knob returns to base center on release
            // (vs JoystickFloating which leaves the knob where the
            // touch lifted). Fixed mode is what the example uses and
            // produces predictable axis values for desktop mouse
            // testing.
            JoystickFixed,
            NoAction,
        );
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

    /// Find any `VirtualJoystickNode` entity that doesn't yet have
    /// our `MobileTouchUiRoot` marker and add it. Runs each Update;
    /// idempotent thanks to the `Without<MobileTouchUiRoot>` filter.
    fn tag_virtual_joystick_root(
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
            cmd.entity(entity).insert(MobileTouchUiRoot);
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

    /// Procedural 96x96 RGBA outline: ring with anti-aliased inner
    /// + outer edges. Used as the joystick's stationary background
    /// circle; tinted via background_color in `create_joystick`.
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
    fn sync_touch_ui_visibility(
        visible: Res<TouchControlsVisible>,
        mut query: Query<&mut Visibility, With<MobileTouchUiRoot>>,
    ) {
        if !visible.is_changed() {
            return;
        }
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
    fn sync_touch_visibility_from_settings(
        settings: Res<crate::settings::UserSettings>,
        mut visible: ResMut<TouchControlsVisible>,
    ) {
        if visible.0 != settings.controls.touch_controls_visible {
            visible.0 = settings.controls.touch_controls_visible;
        }
    }

    /// Marker + identity for touch action buttons. Each `TouchActionButton`
    /// entity is a Bevy `Button` whose `Interaction` state is folded into
    /// the matching `TouchInputState` field each frame.
    #[derive(Component, Clone, Copy, Debug)]
    pub enum TouchActionButton {
        Jump,
        Attack,
        Dash,
        Blink,
        Interact,
        Projectile,
        FlyToggle,
        Start,
        Reset,
    }

    /// Per-button held-last-frame mask. Used by
    /// `update_buttons_from_interactions` to derive
    /// `pressed_this_frame` / `released_this_frame` edges from the
    /// raw `Interaction::Pressed` reading.
    #[derive(Resource, Default, Clone, Copy, Debug)]
    struct TouchButtonEdges {
        jump: bool,
        attack: bool,
        dash: bool,
        blink: bool,
        interact: bool,
        projectile: bool,
        fly_toggle: bool,
        start: bool,
        reset: bool,
    }

    const ACTION_CLUSTER_MARGIN: f32 = 10.0;
    const ACTION_BEZEL_PAD: f32 = 8.0;
    const ACTION_CLUSTER_W: f32 = 310.0;
    const ACTION_CLUSTER_H: f32 = 312.0;
    const ACTION_BEZEL_W: f32 = ACTION_CLUSTER_W + ACTION_BEZEL_PAD * 2.0;
    const ACTION_BEZEL_H: f32 = ACTION_CLUSTER_H + ACTION_BEZEL_PAD * 2.0;
    const MENU_ROW_MARGIN: f32 = 12.0;
    const MENU_ROW_W: f32 = 198.0;
    const MENU_W: f32 = 88.0;
    const MENU_H: f32 = 44.0;
    const MENU_CELL: f32 = 96.0; // 88px button + 4px margin each side

    #[derive(Clone, Copy, Debug)]
    pub(super) struct TouchActionSpec {
        pub(super) action: TouchActionButton,
        pub(super) label: &'static str,
        pub(super) left: f32,
        pub(super) top: f32,
        pub(super) size: f32,
        pub(super) font_size: f32,
    }

    /// Canonical lower-right action layout used by both the rendered UI and
    /// raw multitouch hit testing. Keep all positions here so spacing fixes
    /// cannot drift between the visible overlay and the Android touch path.
    pub(super) fn touch_action_layout() -> [TouchActionSpec; 7] {
        [
            TouchActionSpec {
                action: TouchActionButton::Blink,
                label: "Blink",
                left: 18.0,
                top: 10.0,
                size: 64.0,
                font_size: 13.0,
            },
            TouchActionSpec {
                action: TouchActionButton::FlyToggle,
                label: "Fly",
                left: 123.0,
                top: 2.0,
                size: 68.0,
                font_size: 14.0,
            },
            TouchActionSpec {
                action: TouchActionButton::Projectile,
                label: "Shot",
                left: 228.0,
                top: 10.0,
                size: 64.0,
                font_size: 13.0,
            },
            TouchActionSpec {
                action: TouchActionButton::Interact,
                label: "Interact",
                left: 116.0,
                top: 76.0,
                size: 76.0,
                font_size: 14.0,
            },
            TouchActionSpec {
                action: TouchActionButton::Attack,
                label: "Attack",
                left: 48.0,
                top: 148.0,
                size: 78.0,
                font_size: 14.0,
            },
            TouchActionSpec {
                action: TouchActionButton::Dash,
                label: "Dash",
                left: 184.0,
                top: 148.0,
                size: 78.0,
                font_size: 14.0,
            },
            TouchActionSpec {
                action: TouchActionButton::Jump,
                label: "Jump",
                left: 115.0,
                top: 218.0,
                size: 80.0,
                font_size: 15.0,
            },
        ]
    }

    pub(super) fn touch_action_cluster_origin(window_size: Vec2) -> Vec2 {
        Vec2::new(
            window_size.x - ACTION_CLUSTER_MARGIN - ACTION_CLUSTER_W,
            window_size.y - ACTION_CLUSTER_MARGIN - ACTION_CLUSTER_H,
        )
    }

    /// Spawn the touch button UI. Layout follows a controller mental model:
    /// a lower-right diamond for primary face buttons plus a small shoulder row
    /// above it. Labels describe gameplay intent ("Interact", "Jump", "Fly")
    /// rather than keyboard keys, so the same HUD makes sense on desktop mouse
    /// testing and on an Android phone.
    fn spawn_touch_buttons(mut cmd: Commands) {
        // -- Mobile HUD bezel + controller-style gameplay action cluster --
        // Right-thumb controls, bottom-right:
        //
        //       Blink        Fly        Shot
        //
        //                Interact
        //        Attack              Dash
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
            Name::new("MobileTouchActionBezel"),
            MobileTouchUiRoot,
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
            Name::new("MobileTouchActionCluster"),
            MobileTouchUiRoot,
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
            Name::new("MobileTouchMenuRow"),
            MobileTouchUiRoot,
        ))
        .with_children(|parent| {
            for action in [TouchActionButton::Start, TouchActionButton::Reset] {
                let label = match action {
                    TouchActionButton::Start => "Menu",
                    TouchActionButton::Reset => "Back",
                    _ => "?",
                };
                spawn_menu_button(parent, action, label);
            }
        });
    }

    /// Build one absolutely-positioned gameplay-action button inside the right
    /// thumb cluster. Absolute placement keeps the visible controller diamond and
    /// raw-touch hit testing in lock-step.
    fn spawn_action_button_at(
        parent: &mut ChildSpawnerCommands,
        action: TouchActionButton,
        label: &str,
        left: f32,
        top: f32,
        size: f32,
        font_size: f32,
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
                Name::new(format!("Touch{label}")),
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new(label),
                    TextFont {
                        font_size,
                        ..default()
                    },
                    TextColor(Color::srgb(0.96, 0.97, 1.0)),
                ));
            });
    }

    /// Build one menu-row button. Used for Menu / Back, which are intermittent
    /// and live away from the gameplay action diamond.
    fn spawn_menu_button(
        parent: &mut ChildSpawnerCommands,
        action: TouchActionButton,
        label: &str,
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
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
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
        mut state: ResMut<MobileTouchState>,
        mut edges: ResMut<TouchButtonEdges>,
    ) {
        let mut now = TouchButtonEdges::default();

        // Desktop / editor path: Bevy UI interactions are enough for
        // mouse-driven button testing.
        for (interaction, action) in &query {
            let held = matches!(interaction, Interaction::Pressed);
            set_button_held(&mut now, *action, held);
        }

        // Android / real-touch path: Bevy's Button `Interaction` is
        // not a reliable multitouch source while another finger owns
        // the virtual joystick. Read raw active touches and hit-test
        // against the same fixed button layout instead. This lets the
        // player keep the left thumb on the move stick while tapping
        // Jump / Attack / Dash with the right thumb.
        let window_size = windows
            .single()
            .ok()
            .map(|w| Vec2::new(w.width(), w.height()));
        if let Some(window_size) = window_size {
            for touch in touches.iter() {
                if let Some(action) = touch_action_at_position(touch.position(), window_size) {
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
                        if let Some(action) = touch_action_at_position(cursor, window_size) {
                            set_button_held(&mut now, action, true);
                        }
                    }
                }
            }
        }

        let make_btn = |held_now: bool, held_prev: bool| TouchButton {
            held: held_now,
            pressed_this_frame: held_now && !held_prev,
            released_this_frame: !held_now && held_prev,
        };
        state.0.jump = make_btn(now.jump, edges.jump);
        state.0.attack = make_btn(now.attack, edges.attack);
        state.0.dash = make_btn(now.dash, edges.dash);
        state.0.blink = make_btn(now.blink, edges.blink);
        state.0.interact = make_btn(now.interact, edges.interact);
        state.0.projectile = make_btn(now.projectile, edges.projectile);
        state.0.fly_toggle = make_btn(now.fly_toggle, edges.fly_toggle);
        state.0.start = make_btn(now.start, edges.start);
        state.0.reset = make_btn(now.reset, edges.reset);
        *edges = now;
    }

    fn set_button_held(edges: &mut TouchButtonEdges, action: TouchActionButton, held: bool) {
        if !held {
            return;
        }
        match action {
            TouchActionButton::Jump => edges.jump = true,
            TouchActionButton::Attack => edges.attack = true,
            TouchActionButton::Dash => edges.dash = true,
            TouchActionButton::Blink => edges.blink = true,
            TouchActionButton::Interact => edges.interact = true,
            TouchActionButton::Projectile => edges.projectile = true,
            TouchActionButton::FlyToggle => edges.fly_toggle = true,
            TouchActionButton::Start => edges.start = true,
            TouchActionButton::Reset => edges.reset = true,
        }
    }

    pub(super) fn touch_action_at_position(
        pos: Vec2,
        window_size: Vec2,
    ) -> Option<TouchActionButton> {
        // Touch positions use the same top-left-origin logical coordinate
        // space as Bevy window cursor positions. Gameplay action buttons are
        // visible circles, so hit-test them as circles too: diagonal square
        // bounds are allowed to overlap when the circles themselves do not.
        let cluster_origin = touch_action_cluster_origin(window_size);
        for spec in touch_action_layout() {
            let center = Vec2::new(
                cluster_origin.x + spec.left + spec.size * 0.5,
                cluster_origin.y + spec.top + spec.size * 0.5,
            );
            if pos.distance(center) <= spec.size * 0.5 {
                return Some(spec.action);
            }
        }

        // Menu row: right=MENU_ROW_MARGIN, top=MENU_ROW_MARGIN, Menu / Back.
        let menu_left = window_size.x - MENU_ROW_MARGIN - MENU_ROW_W;
        let menu_top = MENU_ROW_MARGIN;
        for (action, col) in [
            (TouchActionButton::Start, 0usize),
            (TouchActionButton::Reset, 1),
        ] {
            let left = menu_left + col as f32 * MENU_CELL + 4.0;
            let top = menu_top + 4.0;
            if pos.x >= left && pos.x <= left + MENU_W && pos.y >= top && pos.y <= top + MENU_H {
                return Some(action);
            }
        }

        None
    }

    /// Read every `VirtualJoystickMessage<MobileStick>` published this
    /// frame and update the `MobileTouchState`. The plugin emits a
    /// stream of axis updates per touch; we keep the latest reading
    /// per stick.
    fn read_joystick_messages(
        mut reader: MessageReader<VirtualJoystickMessage<MobileStick>>,
        mut state: ResMut<MobileTouchState>,
        mut prev_move_y: Local<f32>,
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
        // Compute Up/Down edge crossings from move_y diff. The pure
        // folder reads these; setting them only on the threshold-
        // crossing frame keeps the double-tap-down detector honest
        // (held Down doesn't repeatedly fire MorphBall).
        const THRESHOLD: f32 = 0.5;
        let crossed_up = *prev_move_y >= -THRESHOLD && state.0.move_y < -THRESHOLD;
        let crossed_down = *prev_move_y <= THRESHOLD && state.0.move_y > THRESHOLD;
        state.0.move_y_just_crossed_up = crossed_up;
        state.0.move_y_just_crossed_down = crossed_down;
        *prev_move_y = state.0.move_y;
    }

    /// Merge the latest `MobileTouchState` into gameplay `ControlFrame`. The
    /// desktop input pipeline (Leafwing) writes its own version of
    /// the frame upstream; this system MERGES rather than replaces:
    ///
    /// - **Movement axis** is mutually exclusive between keyboard
    ///   and touch. If the touch stick is past its deadzone, touch
    ///   wins (keyboard's axis is overwritten). Otherwise the
    ///   keyboard contribution is preserved. This matches Jon's
    ///   "disable the touch dpad when I'm using the keyboard arrows,
    ///   and disable the keyboard arrows when I'm using the touch
    ///   dpad" intent.
    /// - **Action buttons** OR-merge. A held touch button OR a held
    ///   keyboard button counts as held. Edge flags are similarly
    ///   merged so a touch tap + keyboard tap on the same frame
    ///   both register. Per Jon's "the held/release buttons for
    ///   actions I think should be independent."
    ///
    /// When the touch UI is hidden, inactive, or the game is in a UI mode, the
    /// merge is a no-op so the keyboard-derived/suppressed frame passes through
    /// unchanged. UI modes consume touch stick/button intent via
    /// `fold_to_menu_control_frame` instead.
    fn fold_to_control_frame(
        mode: Res<State<crate::game_mode::GameMode>>,
        state: Res<MobileTouchState>,
        visible: Res<TouchControlsVisible>,
        mut frame: ResMut<ControlFrame>,
    ) {
        if !visible.0 {
            return;
        }
        if !mode.get().allows_gameplay() {
            return;
        }
        if !touch_state_is_active(&state.0) {
            return;
        }
        const MOVE_DEADZONE: f32 = 0.05;
        const AIM_DEADZONE: f32 = 0.10;
        let touch_frame = fold_touch_into_control_frame(state.0, MOVE_DEADZONE, AIM_DEADZONE);
        // Mutually-exclusive axis: touch wins iff its post-deadzone
        // magnitude beats threshold 0.05. Otherwise leave keyboard
        // axis alone.
        let touch_move_mag = (touch_frame.axis_x * touch_frame.axis_x
            + touch_frame.axis_y * touch_frame.axis_y)
            .sqrt();
        if touch_move_mag > 0.05 {
            frame.axis_x = touch_frame.axis_x;
            frame.axis_y = touch_frame.axis_y;
            // Also forward the up/down edge flags from touch, since
            // an axis source switch can be the gesture that fires
            // a Door tap or ladder entry.
            frame.up_pressed = frame.up_pressed || touch_frame.up_pressed;
            frame.down_pressed = frame.down_pressed || touch_frame.down_pressed;
        }
        let touch_aim_mag =
            (touch_frame.aim_x * touch_frame.aim_x + touch_frame.aim_y * touch_frame.aim_y).sqrt();
        if touch_aim_mag > 0.10 {
            frame.aim_x = touch_frame.aim_x;
            frame.aim_y = touch_frame.aim_y;
        }
        // OR-merge action buttons. A keyboard JUMP plus a touch
        // JUMP on the same frame should still register as a single
        // press.
        frame.jump_pressed |= touch_frame.jump_pressed;
        frame.jump_held |= touch_frame.jump_held;
        frame.jump_released |= touch_frame.jump_released;
        frame.dash_pressed |= touch_frame.dash_pressed;
        frame.attack_pressed |= touch_frame.attack_pressed;
        frame.blink_pressed |= touch_frame.blink_pressed;
        frame.blink_held |= touch_frame.blink_held;
        frame.blink_released |= touch_frame.blink_released;
        frame.interact_pressed |= touch_frame.interact_pressed;
        frame.projectile_pressed |= touch_frame.projectile_pressed;
        frame.projectile_held |= touch_frame.projectile_held;
        frame.projectile_released |= touch_frame.projectile_released;
        frame.fly_toggle_pressed |= touch_frame.fly_toggle_pressed;
        frame.reset_pressed |= touch_frame.reset_pressed;
        frame.start_pressed |= touch_frame.start_pressed;
        frame.pogo_pressed |= touch_frame.pogo_pressed;
    }

    /// Merge touch buttons, the touch stick in UI modes, and non-control drag
    /// gestures into the semantic menu frame.
    ///
    /// This is intentionally separate from `fold_to_control_frame`: gameplay axes
    /// and UI gestures have different consumers. The touch Start button toggles
    /// pause, Reset acts as Back, Jump/Interact can confirm, and the move stick
    /// becomes the same repeated up/down/left/right intent as keyboard arrows
    /// while a dialog or pause menu is active. One-finger drags outside the fixed
    /// touch-control regions still map to menu scroll/navigation, and the same
    /// drag path accepts a pressed left mouse button for desktop testing.
    fn fold_to_menu_control_frame(
        time: Res<Time>,
        mode: Res<State<crate::game_mode::GameMode>>,
        state: Res<MobileTouchState>,
        visible: Res<TouchControlsVisible>,
        touches: Res<Touches>,
        mouse_buttons: Res<ButtonInput<MouseButton>>,
        windows: Query<&Window, With<PrimaryWindow>>,
        user_settings: Res<crate::settings::UserSettings>,
        mut gesture: ResMut<MenuTouchGestureState>,
        mut frame: ResMut<MenuControlFrame>,
    ) {
        if !visible.0 {
            gesture.last_pos = None;
            gesture.drag_scroll_accum = 0.0;
            gesture.stick_input = MenuInputState::default();
            return;
        }

        let touch = state.0;
        frame.start |= touch.start.pressed_this_frame;
        frame.back |= touch.reset.pressed_this_frame;
        frame.back_held |= touch.reset.held;
        frame.select |= touch.jump.pressed_this_frame || touch.interact.pressed_this_frame;
        frame.select_held |= touch.jump.held || touch.interact.held;

        let menu_mode = matches!(
            mode.get(),
            crate::game_mode::GameMode::Dialogue | crate::game_mode::GameMode::Paused
        );
        if menu_mode {
            let analog_dir = touch_move_to_menu_dir(
                touch,
                user_settings.controls.left_stick_deadzone,
            );
            let input = gesture.stick_input.step(
                false,
                false,
                false,
                false,
                analog_dir,
                false,
                false,
                false,
                time.delta_secs(),
                user_settings.controls.menu_repeat_initial_delay,
                user_settings.controls.menu_repeat_interval,
            );
            let stick_frame = MenuControlFrame::from_menu_input(input);
            frame.up |= stick_frame.up;
            frame.down |= stick_frame.down;
            frame.left |= stick_frame.left;
            frame.right |= stick_frame.right;
        } else {
            gesture.stick_input = MenuInputState::default();
        }

        let Ok(window) = windows.single() else {
            gesture.last_pos = None;
            gesture.drag_scroll_accum = 0.0;
            return;
        };
        let window_size = Vec2::new(window.width(), window.height());

        let touch_pos = touches
            .iter()
            .map(|touch| touch.position())
            .find(|pos| !touch_control_area_contains(*pos, window_size));
        let mouse_pos = if mouse_buttons.pressed(MouseButton::Left) {
            window
                .cursor_position()
                .filter(|pos| !touch_control_area_contains(*pos, window_size))
        } else {
            None
        };
        let menu_pos = touch_pos.or(mouse_pos);

        if let Some(pos) = menu_pos {
            if let Some(last) = gesture.last_pos {
                let dy = pos.y - last.y;
                // Bevy touch/cursor positions are top-left-origin. A phone-style
                // swipe up (negative dy) should move the highlighted row down,
                // matching normal phone scroll semantics.
                if dy.abs() >= 3.0 {
                    gesture.drag_scroll_accum += dy / 30.0;
                    let whole_steps = gesture.drag_scroll_accum.trunc().clamp(-5.0, 5.0);
                    if whole_steps != 0.0 {
                        frame.scroll_y += whole_steps;
                        gesture.drag_scroll_accum -= whole_steps;
                    }
                }
            }
            gesture.last_pos = Some(pos);
        } else {
            gesture.last_pos = None;
            gesture.drag_scroll_accum = 0.0;
        }
    }

    pub(super) fn touch_move_to_menu_dir(
        touch: TouchInputState,
        deadzone: f32,
    ) -> Option<crate::input::MenuDir> {
        let (x, y_down) = crate::settings::ControlSettings::apply_deadzone(
            touch.move_x,
            touch.move_y,
            deadzone,
        );
        // Touch/gameplay stores +Y as down, while the menu analog helper expects
        // +Y as up to match gamepad/keyboard menu convention. Flip here so
        // dragging the visible joystick down selects the next dialog option.
        crate::input::analog_to_dir(x, -y_down, 0.5)
    }

    fn touch_control_area_contains(pos: Vec2, window_size: Vec2) -> bool {
        if touch_action_at_position(pos, window_size).is_some() {
            return true;
        }
        // Approximate virtual joystick footprint in the lower-left corner. The
        // exact nodes are owned by `virtual_joystick`, so a geometric exclusion is
        // the least-coupled way to avoid treating movement-stick drags as menu
        // scroll gestures.
        pos.x <= 300.0 && pos.y >= window_size.y - 300.0
    }

    /// True if any touch input field has a non-default value. Used
    /// to gate the fold so an empty touch state doesn't stomp the
    /// keyboard-derived ControlFrame every frame.
    ///
    /// Includes `released_this_frame` flags: without them, the
    /// frame after a button release would skip the fold and the
    /// release edge would never reach the simulator. Concrete
    /// repro: tapping Projectile with a mouse charged the fireball
    /// (frame N: pressed) but never released it (frame N+1: held=
    /// false, pressed=false, released=true → activity gate skipped
    /// the fold without this clause).
    fn touch_state_is_active(state: &TouchInputState) -> bool {
        let stick_active = state.move_x.abs() > 1e-3
            || state.move_y.abs() > 1e-3
            || state.aim_x.abs() > 1e-3
            || state.aim_y.abs() > 1e-3;
        let any_button = state.jump.held
            || state.attack.held
            || state.dash.held
            || state.blink.held
            || state.interact.held
            || state.projectile.held
            || state.fly_toggle.held
            || state.start.held
            || state.reset.held;
        let any_edge = state.jump.pressed_this_frame
            || state.attack.pressed_this_frame
            || state.dash.pressed_this_frame
            || state.blink.pressed_this_frame
            || state.interact.pressed_this_frame
            || state.projectile.pressed_this_frame
            || state.fly_toggle.pressed_this_frame
            || state.start.pressed_this_frame
            || state.reset.pressed_this_frame
            || state.move_y_just_crossed_up
            || state.move_y_just_crossed_down;
        let any_release = state.jump.released_this_frame
            || state.attack.released_this_frame
            || state.dash.released_this_frame
            || state.blink.released_this_frame
            || state.interact.released_this_frame
            || state.projectile.released_this_frame
            || state.fly_toggle.released_this_frame
            || state.start.released_this_frame
            || state.reset.released_this_frame;
        stick_active || any_button || any_edge || any_release
    }

    // Re-export the helper so `MobileTouchPlugin` is a one-import seam.
    pub use super::{
        fold_touch_into_control_frame as _fold_for_doc, TouchButton as _btn_for_doc,
        TouchInputState as _state_for_doc,
    };
    // Suppress dead-code warnings for the re-export aliases.
    #[allow(dead_code)]
    fn _re_exports_used() {
        let _ = _fold_for_doc;
        let _ = _state_for_doc::default();
        let _ = _btn_for_doc::off();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_kills_sub_threshold_input() {
        let (x, y) = apply_deadzone(0.05, 0.05, 0.10);
        assert_eq!((x, y), (0.0, 0.0));
    }

    #[test]
    fn deadzone_preserves_above_threshold_direction() {
        // Stick pushed all the way right (1.0, 0.0), 0.10 deadzone:
        // post-deadzone should still be effectively (1.0, 0.0).
        let (x, y) = apply_deadzone(1.0, 0.0, 0.10);
        assert!((x - 1.0).abs() < 1e-3, "x should reach 1.0; got {x}");
        assert_eq!(y, 0.0);
    }

    #[test]
    fn deadzone_zero_passes_through() {
        let (x, y) = apply_deadzone(0.5, -0.3, 0.0);
        assert_eq!(x, 0.5);
        assert_eq!(y, -0.3);
    }

    #[test]
    fn fold_zero_state_produces_neutral_control_frame() {
        let frame = fold_touch_into_control_frame(TouchInputState::default(), 0.05, 0.05);
        assert_eq!(frame.axis_x, 0.0);
        assert_eq!(frame.axis_y, 0.0);
        assert!(!frame.jump_pressed);
        assert!(!frame.jump_held);
        assert!(!frame.up_pressed);
        assert!(!frame.down_pressed);
    }

    #[test]
    fn fold_sets_jump_flags_from_button_state() {
        let mut state = TouchInputState::default();
        state.jump = TouchButton::pressed_now();
        let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
        assert!(frame.jump_pressed);
        assert!(frame.jump_held);
        assert!(!frame.jump_released);
    }

    #[test]
    fn fold_translates_aim_stick() {
        let mut state = TouchInputState::default();
        state.aim_x = 0.8;
        state.aim_y = -0.5;
        let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
        // After deadzone (0.05) + scaling: still strongly positive x,
        // negative y. Don't pin exact values; pin sign + magnitude.
        assert!(frame.aim_x > 0.5);
        assert!(frame.aim_y < -0.3);
    }

    #[test]
    fn fold_propagates_explicit_up_pressed_edge() {
        // The Bevy plugin computes edge crossings from previous-
        // frame `move_y`; the pure folder consumes the explicit
        // edge flags rather than auto-deriving from `move_y > 0.5`
        // (which would re-trigger every frame and fire MorphBall
        // through the double-tap-down detector).
        let mut state = TouchInputState::default();
        state.move_y = -1.0;
        state.move_y_just_crossed_up = true;
        let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
        assert!(frame.up_pressed);
        assert!(!frame.down_pressed);
    }

    #[test]
    fn fold_propagates_explicit_down_pressed_edge() {
        let mut state = TouchInputState::default();
        state.move_y = 1.0;
        state.move_y_just_crossed_down = true;
        let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
        assert!(frame.down_pressed);
        assert!(!frame.up_pressed);
    }

    #[test]
    fn fold_held_down_without_edge_flag_does_not_fire_down_pressed() {
        // Pin the bug fix: holding move_y=1.0 every frame WITHOUT
        // setting the edge flag should NOT fire down_pressed. This
        // is the "held Down" case that previously oscillated body_mode
        // through the double-tap-down detector.
        let mut state = TouchInputState::default();
        state.move_y = 1.0;
        state.move_y_just_crossed_down = false;
        let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
        assert!(!frame.down_pressed);
        assert!(!frame.up_pressed);
    }

    #[test]
    fn fold_propagates_all_action_buttons() {
        // Every action button: pressed-this-frame should map through.
        let mut state = TouchInputState::default();
        state.attack = TouchButton::pressed_now();
        state.dash = TouchButton::pressed_now();
        state.blink = TouchButton::pressed_now();
        state.interact = TouchButton::pressed_now();
        state.projectile = TouchButton::pressed_now();
        state.fly_toggle = TouchButton::pressed_now();
        state.start = TouchButton::pressed_now();
        state.reset = TouchButton::pressed_now();
        let frame = fold_touch_into_control_frame(state, 0.05, 0.05);
        assert!(frame.attack_pressed);
        assert!(frame.dash_pressed);
        assert!(frame.blink_pressed);
        assert!(frame.interact_pressed);
        assert!(frame.projectile_pressed);
        assert!(frame.fly_toggle_pressed);
        assert!(frame.start_pressed);
        assert!(frame.reset_pressed);
    }

    #[cfg(feature = "mobile_touch")]
    #[test]
    fn touch_move_to_menu_dir_flips_touch_y_for_menu_navigation() {
        use crate::input::MenuDir;
        use crate::mobile_input::bevy_plugin::touch_move_to_menu_dir;

        let mut state = TouchInputState::default();
        state.move_y = 1.0;
        assert_eq!(touch_move_to_menu_dir(state, 0.05), Some(MenuDir::Down));

        state.move_y = -1.0;
        assert_eq!(touch_move_to_menu_dir(state, 0.05), Some(MenuDir::Up));
    }

    #[cfg(feature = "mobile_touch")]
    #[test]
    fn touch_move_to_menu_dir_applies_deadzone() {
        use crate::mobile_input::bevy_plugin::touch_move_to_menu_dir;

        let mut state = TouchInputState::default();
        state.move_y = 0.10;
        assert_eq!(touch_move_to_menu_dir(state, 0.25), None);
    }

    #[cfg(feature = "mobile_touch")]
    #[test]
    fn touch_action_hit_test_includes_fly_button() {
        use crate::mobile_input::bevy_plugin::{
            touch_action_at_position, touch_action_cluster_origin, touch_action_layout,
            TouchActionButton,
        };

        let window_size = bevy::prelude::Vec2::new(1080.0, 2340.0);
        let fly = touch_action_layout()
            .into_iter()
            .find(|spec| matches!(spec.action, TouchActionButton::FlyToggle))
            .expect("Fly button remains in the touch action layout");
        // Center of the visible Fly shoulder button in the lower-right cluster.
        let cluster_origin = touch_action_cluster_origin(window_size);
        let pos = bevy::prelude::Vec2::new(
            cluster_origin.x + fly.left + fly.size * 0.5,
            cluster_origin.y + fly.top + fly.size * 0.5,
        );
        assert!(matches!(
            touch_action_at_position(pos, window_size),
            Some(TouchActionButton::FlyToggle)
        ));
    }

    #[cfg(feature = "mobile_touch")]
    #[test]
    fn touch_action_layout_keeps_visible_circles_apart() {
        use crate::mobile_input::bevy_plugin::touch_action_layout;

        const MIN_VISUAL_GAP: f32 = 4.0;
        let layout = touch_action_layout();
        for (i, a) in layout.iter().enumerate() {
            let ac = bevy::prelude::Vec2::new(a.left + a.size * 0.5, a.top + a.size * 0.5);
            for b in layout.iter().skip(i + 1) {
                let bc = bevy::prelude::Vec2::new(b.left + b.size * 0.5, b.top + b.size * 0.5);
                let gap = ac.distance(bc) - (a.size + b.size) * 0.5;
                assert!(
                    gap >= MIN_VISUAL_GAP,
                    "touch circles should have at least {MIN_VISUAL_GAP}px gap: {} and {} only have {gap:.1}px",
                    a.label, b.label
                );
            }
        }
    }

    #[cfg(feature = "mobile_touch")]
    #[test]
    fn touch_action_hit_test_uses_visible_circle_not_square_bounds() {
        use crate::mobile_input::bevy_plugin::{
            touch_action_at_position, touch_action_cluster_origin, touch_action_layout,
            TouchActionButton,
        };

        let window_size = bevy::prelude::Vec2::new(1280.0, 720.0);
        let layout = touch_action_layout();
        let attack = layout
            .iter()
            .find(|spec| matches!(spec.action, TouchActionButton::Attack))
            .expect("Attack remains in the touch action layout");
        let jump = layout
            .iter()
            .find(|spec| matches!(spec.action, TouchActionButton::Jump))
            .expect("Jump remains in the touch action layout");
        assert!(
            attack.top + attack.size > jump.top,
            "diagonal square bounds should be allowed to overlap vertically"
        );

        let origin = touch_action_cluster_origin(window_size);
        let square_only = bevy::prelude::Vec2::new(
            origin.x + attack.left + attack.size - 2.0,
            origin.y + jump.top + 2.0,
        );
        assert_eq!(touch_action_at_position(square_only, window_size), None);
    }
}
