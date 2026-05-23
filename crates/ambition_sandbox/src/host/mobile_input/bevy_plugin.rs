use std::borrow::Cow;

use bevy::input::mouse::MouseButton;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use virtual_joystick::*;

use super::layout::{
    touch_action_at_position, touch_action_layout, TouchActionButton, ACTION_BEZEL_H,
    ACTION_BEZEL_W, ACTION_CLUSTER_H, ACTION_CLUSTER_MARGIN, ACTION_CLUSTER_W, MENU_ROW_MARGIN,
    MENU_ROW_W,
};
use super::menu_bridge::{fold_to_control_frame, fold_to_menu_control_frame};
use super::state::TouchInputState;
use crate::input::{ControlFrame, KeyboardPreset, MenuInputState, SandboxAction};
use crate::ui_nav::DragScrollState;

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

/// Tracks the last non-control touch position used for menu drag
/// scrolling.
///
/// Bevy UI button `Interaction` covers taps on concrete rows.
/// This state is only for whole-panel gestures such as dragging
/// up/down to navigate a menu while another finger is still on
/// the movement stick.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct MenuTouchGestureState {
    pub(super) drag_scroll: DragScrollState,
    pub(super) stick_input: MenuInputState,
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
            )
            // Contextual button-label sync, decomposed into two
            // narrow systems. `update_button_verb_from_affordances`
            // reads `PlayerAffordances` (the per-frame "what would
            // each press do" table) and writes the per-button
            // `ButtonVerb`; `render_touch_button_text` folds whatever
            // ButtonVerb/Glyph/Pressed components exist into the
            // Text node. The split lets Phase 2 (glyph subtitle) and
            // Phase 3 (pressed-state highlight) ride alongside
            // without growing one god-system.
            .add_systems(
                Update,
                (
                    update_button_verb_from_affordances
                        .after(crate::player::affordances::AffordancesSystemSet::Compute),
                    update_button_glyph_from_active_input
                        .after(crate::player::affordances::AffordancesSystemSet::Compute),
                    update_button_pressed_from_actions
                        .after(crate::player::affordances::AffordancesSystemSet::Compute),
                    render_touch_button_text
                        .after(update_button_verb_from_affordances)
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
                drive_joystick_knob_from_axis.after(JoystickSystems::UpdateUI),
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
    // Joystick footprint is scaled by `TOUCH_SCALE` from the original
    // 120x120 / 56x56 layout to match the shrunken action cluster.
    // Keep the `JOYSTICK_*` constants in sync with the area exclusion
    // in `menu_bridge::touch_control_area_contains` (uses the same
    // base+margin geometry to skip the stick when matching menu
    // drag-scroll gestures).
    use super::layout::TOUCH_SCALE;
    let stick_base = 120.0 * TOUCH_SCALE;
    let stick_knob = 56.0 * TOUCH_SCALE;
    let stick_margin = 24.0 * TOUCH_SCALE;
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
        Vec2::new(stick_knob, stick_knob),
        Vec2::new(stick_base, stick_base),
        Node {
            width: Val::Px(stick_base),
            height: Val::Px(stick_base),
            position_type: PositionType::Absolute,
            left: Val::Px(stick_margin),
            bottom: Val::Px(stick_margin),
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
    // No floating "Move" overlay above the stick — the knob's drag
    // position is the directional indicator on its own. Per Jon
    // 2026-05-23: "the control stick itself on the touch screen
    // would be the thing that moves rather than having something
    // drawn above it." The action buttons still carry verb / glyph
    // labels because they don't visually change to show what they'd
    // do; the stick does.
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
    settings: Res<crate::persistence::settings::UserSettings>,
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
    dash: bool,
    blink: bool,
    interact: bool,
    projectile: bool,
    fly_toggle: bool,
    shield: bool,
    start: bool,
    reset: bool,
}

/// Spawn the touch button UI. Layout follows a controller mental
/// model: a lower-right diamond for primary face buttons plus a
/// small shoulder row above it. Labels describe gameplay intent
/// ("Interact", "Jump", "Fly") rather than keyboard keys, so the
/// same HUD makes sense on desktop mouse testing and on an
/// Android phone.
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
                TextFont {
                    font_size,
                    ..default()
                },
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
                // `update_button_verb_from_affordances` from the
                // global `PlayerAffordances` resource; rendered into
                // `Text` by `render_touch_button_text`. Splitting
                // these concerns means each per-frame derived value
                // (verb, glyph) gets its own narrow update system
                // instead of one god-system.
                ButtonVerb::Static(label),
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
/// back to the correct affordance.
#[derive(Component)]
pub struct TouchActionLabel(pub TouchActionButton);

/// The verb-text to render under each touch button. Updated each
/// frame by [`update_button_verb_from_affordances`] from the global
/// [`crate::player::affordances::PlayerAffordances`] table. Held as
/// component data (not computed inline in the render system) so
/// independent concerns — verb, future glyph subtitle, future
/// pressed-state highlight — each own their own component + update
/// system and compose at render.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub enum ButtonVerb {
    /// Pre-affordance label baked in at spawn — used until the
    /// affordance system populates the dynamic value, and as the
    /// stable fallback for buttons that don't carry a contextual
    /// meaning (FlyToggle / Start / Reset).
    Static(&'static str),
    /// Dynamic label written by the affordance update system.
    /// `String` (not `&'static str`) so authored `InteractVariant::Custom`
    /// prompts can flow through unchanged.
    Dynamic(String),
}

impl ButtonVerb {
    fn as_str(&self) -> &str {
        match self {
            ButtonVerb::Static(s) => s,
            ButtonVerb::Dynamic(s) => s.as_str(),
        }
    }
}

/// Per-frame: read [`PlayerAffordances`] and write each button's
/// [`ButtonVerb`].
///
/// One narrow system, one concern. The render system folds the verb
/// (and, in later phases, the glyph / pressed-state components) into
/// the actual `Text` node.
pub fn update_button_verb_from_affordances(
    affordances: Res<crate::player::affordances::PlayerAffordances>,
    mut labels: Query<(&TouchActionLabel, &mut ButtonVerb)>,
) {
    use crate::player::affordances::{InteractVariant, VariantLabel};
    for (TouchActionLabel(action), mut verb) in &mut labels {
        let next: Option<String> = match action {
            TouchActionButton::Jump => Some(affordances.jump.text().to_owned()),
            TouchActionButton::Attack => Some(affordances.attack.text().to_owned()),
            TouchActionButton::Dash => Some(affordances.dash.text().to_owned()),
            TouchActionButton::Shield => Some(affordances.shield.text().to_owned()),
            TouchActionButton::Interact => {
                // `display()` handles the `Custom(prompt)` case
                // transparently; for typed variants it returns the
                // canonical text from `VariantLabel`.
                Some(match &affordances.interact {
                    InteractVariant::Custom(prompt) => prompt.as_ref().to_owned(),
                    v => v.text().to_owned(),
                })
            }
            // Projectile is the contextual "special" — it picks up
            // the aim-driven variant (N-Special / S-Special / U-Special
            // / D-Special / Hadouken). Blink stays static because the
            // touch HUD has a separate dedicated Blink button whose
            // outcome doesn't vary with stick direction.
            TouchActionButton::Projectile => Some(affordances.special.text().to_owned()),
            TouchActionButton::Blink => None,
            // FlyToggle / Start / Reset don't have a contextual
            // meaning today — leave the static label alone.
            TouchActionButton::FlyToggle | TouchActionButton::Start | TouchActionButton::Reset => {
                None
            }
        };
        if let Some(next) = next {
            // Only flip when the string actually changes — keeps
            // Bevy's change detection bit honest so the render
            // system can filter on `Changed<ButtonVerb>` once
            // performance matters.
            let same = match &*verb {
                ButtonVerb::Static(s) => *s == next,
                ButtonVerb::Dynamic(s) => s == &next,
            };
            if !same {
                *verb = ButtonVerb::Dynamic(next);
            }
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

/// Per-device glyph subtitle (Phase 2). Updated each frame from
/// [`ActiveInputMethod`] + the active [`KeyboardPreset`].
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
/// `SandboxAction` is held this frame; consumed by
/// [`sync_button_pressed_visual`] to brighten the button background.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonPressed(pub bool);

/// Map a touch button to its canonical [`SandboxAction`]. Keeps the
/// glyph + pressed-state systems honest — both compute against the
/// same action mapping the touch fold uses for input contribution.
fn touch_action_to_sandbox_action(action: TouchActionButton) -> SandboxAction {
    match action {
        TouchActionButton::Jump => SandboxAction::Jump,
        TouchActionButton::Attack => SandboxAction::Attack,
        TouchActionButton::Dash => SandboxAction::Dash,
        TouchActionButton::Blink => SandboxAction::Blink,
        TouchActionButton::Interact => SandboxAction::Interact,
        TouchActionButton::Projectile => SandboxAction::Projectile,
        TouchActionButton::FlyToggle => SandboxAction::Utility,
        TouchActionButton::Shield => SandboxAction::QuickAction,
        TouchActionButton::Start => SandboxAction::Start,
        TouchActionButton::Reset => SandboxAction::Reset,
    }
}

/// Per-frame: write each button's glyph from the active input
/// device. Reads [`ActiveInputMethod`] + a default
/// [`KeyboardPreset`]; future hookup point for the player's chosen
/// preset.
pub fn update_button_glyph_from_active_input(
    active: Res<crate::player::affordances::ActiveInputMethod>,
    mut labels: Query<(&TouchActionLabel, &mut ButtonGlyph)>,
) {
    // TODO: when the sandbox exposes the player's selected
    // KeyboardPreset (currently swapped via `sync_preset_input_map`
    // but not surfaced as a Resource), read it here instead of
    // hardcoding the default Arrows+ZXC preset. Until then desktop
    // players who haven't rebound see Z/X/C glyphs, which match the
    // out-of-the-box bindings.
    let preset = KeyboardPreset::arrows_zxc();
    for (TouchActionLabel(touch_action), mut glyph) in &mut labels {
        let sa = touch_action_to_sandbox_action(*touch_action);
        let next = crate::player::affordances::glyph_for(sa, &preset, active.0);
        if glyph.0 != next {
            glyph.0 = next;
        }
    }
}

/// Per-frame: write each button's pressed flag from
/// `ActionState<SandboxAction>` on the primary player. Skips writing
/// when the value is unchanged so the visual-sync system can filter
/// on `Changed<ButtonPressed>`. Operates on the Button entity (which
/// carries both `TouchActionButton` and `ButtonPressed`), so no
/// parent walk is needed.
pub fn update_button_pressed_from_actions(
    actions_q: Query<
        &leafwing_input_manager::prelude::ActionState<SandboxAction>,
        With<crate::player::PrimaryPlayer>,
    >,
    mut buttons: Query<(&TouchActionButton, &mut ButtonPressed)>,
) {
    let Ok(actions) = actions_q.single() else {
        return;
    };
    for (touch_action, mut pressed) in &mut buttons {
        let sa = touch_action_to_sandbox_action(*touch_action);
        let held = actions.pressed(&sa);
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
fn spawn_menu_button(parent: &mut ChildSpawnerCommands, action: TouchActionButton, label: &str) {
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

    let make_btn = |held_now: bool, held_prev: bool| super::TouchButton {
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
    state.0.shield = make_btn(now.shield, edges.shield);
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
        TouchActionButton::Shield => edges.shield = true,
        TouchActionButton::Start => edges.start = true,
        TouchActionButton::Reset => edges.reset = true,
    }
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
/// Convention: `ControlFrame.axis_*` already follows the sim's
/// +Y-down convention, which matches Bevy UI's +Y-down `Node.top`
/// axis, so no Y inversion is needed here.
fn drive_joystick_knob_from_axis(
    control_frame: Res<ControlFrame>,
    joystick_q: Query<(&VirtualJoystickState, &Children), With<VirtualJoystickNode<MobileStick>>>,
    base_q: Query<&ComputedNode, With<VirtualJoystickUIBackground>>,
    mut knob_q: Query<(&mut Node, &ComputedNode), With<VirtualJoystickUIKnob>>,
) {
    for (state, children) in &joystick_q {
        // Real drag wins. The crate's `update_ui` already placed the
        // knob from `state.delta` based on the actual cursor.
        if state.touch_state.is_some() {
            continue;
        }
        let mut base_size: Option<Vec2> = None;
        let mut knob_entity: Option<Entity> = None;
        for child in children.iter() {
            if let Ok(base) = base_q.get(child) {
                base_size = Some(base.size());
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
        let knob_size = knob_computed.size();
        let base_half = base_size * 0.5;
        let knob_half = knob_size * 0.5;

        // Clamp the axis vector to the unit circle so diagonal inputs
        // ride the rim of the base ring instead of overshooting into
        // the corners (which would push the knob outside the visible
        // circle). Matches the crate's `joystick_delta` circular
        // clamp.
        let axis = Vec2::new(
            control_frame.axis_x.clamp(-1.0, 1.0),
            control_frame.axis_y.clamp(-1.0, 1.0),
        );
        let mag_sq = axis.length_squared();
        let axis = if mag_sq > 1.0 {
            axis / mag_sq.sqrt()
        } else {
            axis
        };

        // Same geometry as `virtual_joystick`'s update_ui for an
        // un-offset base + visual delta = axis (already in UI Y-down).
        // Reduces to: top-left = knob_half + base_half * axis.
        let target_left = knob_half.x + base_half.x * axis.x;
        let target_top = knob_half.y + base_half.y * axis.y;
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
