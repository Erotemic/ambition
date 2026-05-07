//! Mobile / touch input adapter for the Android demo path.
//!
//! Goal: a sideloadable Pixel-class APK where the sandbox is playable
//! with on-screen joysticks + buttons. The Leafwing keyboard/gamepad
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
    use crate::input::ControlFrame;
    use bevy::prelude::*;
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
                .insert_resource(TouchButtonEdges::default())
                .insert_resource(TouchControlsVisible::default())
                .add_systems(Startup, (spawn_touch_buttons, spawn_touch_joysticks))
                .add_systems(
                    Update,
                    (
                        tag_virtual_joystick_root,
                        sync_touch_ui_visibility,
                        read_joystick_messages,
                        update_buttons_from_interactions,
                        fold_to_control_frame,
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
    fn spawn_touch_joysticks(
        mut cmd: Commands,
        mut images: ResMut<Assets<Image>>,
    ) {
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
            Some(Color::srgba(0.95, 0.95, 0.95, 0.9)),
            Some(Color::srgba(0.20, 0.30, 0.45, 0.8)),
            Some(Color::srgba(0.10, 0.16, 0.24, 0.30)),
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
        start: bool,
        reset: bool,
    }

    /// Spawn the touch button UI. Layout follows Jon's 2026-05-07
    /// guidance:
    ///
    /// - Gameplay-action buttons (Jump / Attack / Dash / Blink / E /
    ///   Proj) live on a right-anchored 2-column grid in the bottom
    ///   right CORNER, NOT spanning the main screen. This keeps the
    ///   gameplay viewport unobstructed.
    /// - Menu-style buttons (Start / Reset) live on a top-anchored
    ///   row that's allowed to sit "on the main screen" since menus
    ///   are intermittent.
    ///
    /// Each button has a visible Text label (Jump / Atk / Dash / etc.)
    /// so the tap targets are differentiated rather than identical
    /// gray squares.
    fn spawn_touch_buttons(mut cmd: Commands) {
        // -- Mobile HUD bezel + gameplay action buttons --
        // Per Jon's "pad the left and right parts of the screen ...
        // could be the start of a mobile hud" note, frame the touch
        // cluster with a slightly darker translucent backdrop so the
        // buttons read as a HUD strip rather than floating sprites
        // over gameplay. Some overlap with gameplay view is OK; the
        // bezel just signals "this region is HUD".
        cmd.spawn((
            Node {
                width: Val::Px(216.0),
                height: Val::Px(152.0),
                position_type: PositionType::Absolute,
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.45)),
            Name::new("MobileTouchActionBezel"),
            MobileTouchUiRoot,
        ));
        cmd.spawn((
            Node {
                width: Val::Px(192.0),
                height: Val::Px(128.0),
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                bottom: Val::Px(12.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::FlexEnd,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            Name::new("MobileTouchActionCluster"),
            MobileTouchUiRoot,
        ))
        .with_children(|parent| {
            for action in [
                TouchActionButton::Blink,
                TouchActionButton::Projectile,
                TouchActionButton::Dash,
                TouchActionButton::Attack,
                TouchActionButton::Interact,
                TouchActionButton::Jump,
            ] {
                let label = match action {
                    TouchActionButton::Jump => "Jump",
                    TouchActionButton::Attack => "Atk",
                    TouchActionButton::Dash => "Dash",
                    TouchActionButton::Blink => "Blink",
                    TouchActionButton::Interact => "E",
                    TouchActionButton::Projectile => "Proj",
                    _ => "?",
                };
                spawn_action_button(parent, action, label);
            }
        });

        // -- Menu-style buttons (top-right, smaller) --
        cmd.spawn((
            Node {
                width: Val::Px(168.0),
                height: Val::Px(48.0),
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(12.0),
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
                    TouchActionButton::Start => "Pause",
                    TouchActionButton::Reset => "Reset",
                    _ => "?",
                };
                spawn_menu_button(parent, action, label);
            }
        });
    }

    /// Build one gameplay-action button: 72x72 with a visible text
    /// label. Margin is small so the cluster fits in the corner.
    fn spawn_action_button(parent: &mut ChildSpawnerCommands, action: TouchActionButton, label: &str) {
        parent
            .spawn((
                Button,
                Node {
                    width: Val::Px(56.0),
                    height: Val::Px(56.0),
                    margin: UiRect::all(Val::Px(4.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.18, 0.22, 0.30, 0.55)),
                action,
                Name::new(format!("Touch{label}")),
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.95, 0.95)),
                ));
            });
    }

    /// Build one menu-row button: 64x40, smaller text. Used for
    /// Start / Reset which are intermittent and OK to sit at the top
    /// of the gameplay viewport.
    fn spawn_menu_button(parent: &mut ChildSpawnerCommands, action: TouchActionButton, label: &str) {
        parent
            .spawn((
                Button,
                Node {
                    width: Val::Px(72.0),
                    height: Val::Px(40.0),
                    margin: UiRect::all(Val::Px(4.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.20, 0.16, 0.22, 0.65)),
                action,
                Name::new(format!("Touch{label}")),
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.92, 0.88, 0.92)),
                ));
            });
    }

    /// Walk every `TouchActionButton` entity, read its `Interaction`,
    /// and fold (held vs pressed/released edges) into
    /// `MobileTouchState.<button>`. Edges are derived against the
    /// previous frame's held mask in `TouchButtonEdges`.
    fn update_buttons_from_interactions(
        query: Query<(&Interaction, &TouchActionButton), With<Button>>,
        mut state: ResMut<MobileTouchState>,
        mut edges: ResMut<TouchButtonEdges>,
    ) {
        let mut now = TouchButtonEdges::default();
        for (interaction, action) in &query {
            let held = matches!(interaction, Interaction::Pressed);
            match action {
                TouchActionButton::Jump => now.jump |= held,
                TouchActionButton::Attack => now.attack |= held,
                TouchActionButton::Dash => now.dash |= held,
                TouchActionButton::Blink => now.blink |= held,
                TouchActionButton::Interact => now.interact |= held,
                TouchActionButton::Projectile => now.projectile |= held,
                TouchActionButton::Start => now.start |= held,
                TouchActionButton::Reset => now.reset |= held,
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
        state.0.start = make_btn(now.start, edges.start);
        state.0.reset = make_btn(now.reset, edges.reset);
        *edges = now;
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
            let axis = msg.snap_axis(None);
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

    /// Merge the latest `MobileTouchState` into `ControlFrame`. The
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
    /// When the touch UI is hidden or inactive, the merge is a
    /// no-op so the keyboard-derived frame passes through unchanged.
    fn fold_to_control_frame(
        state: Res<MobileTouchState>,
        visible: Res<TouchControlsVisible>,
        mut frame: ResMut<ControlFrame>,
    ) {
        if !visible.0 {
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
        let touch_move_mag =
            (touch_frame.axis_x * touch_frame.axis_x + touch_frame.axis_y * touch_frame.axis_y)
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
    pub use super::{fold_touch_into_control_frame as _fold_for_doc, TouchButton as _btn_for_doc, TouchInputState as _state_for_doc};
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
}
