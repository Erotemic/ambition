use super::state::apply_deadzone;
#[cfg(feature = "mobile_touch")]
use super::state::TouchButton;

// ─── The virtual-device path: touch resolves through participant bindings ───
//
// These exercise the REAL leafwing pipeline: `MobileTouchState` (the collected
// device state) → the registered input kinds → the participant's `InputMap`
// bindings → `ActionState<SandboxAction>`. No fold, no special cases — the
// same resolution a keyboard or gamepad gets.

#[cfg(feature = "mobile_touch")]
mod virtual_device_tests {
    use super::super::bevy_plugin::MobileTouchState;
    use super::super::virtual_device::{
        bind_touch_virtual_inputs, TouchStickDirection, TouchVirtualButton, TouchVirtualStick,
    };
    use super::TouchButton;
    use ambition_input::{InputParticipant, ParticipantContexts, SandboxAction};
    use bevy::prelude::*;
    use leafwing_input_manager::plugin::{CentralInputStorePlugin, InputManagerPlugin};
    use leafwing_input_manager::prelude::updating::InputRegistration;
    use leafwing_input_manager::prelude::{ActionState, InputMap};
    use leafwing_input_manager::InputControlKind;

    /// A minimal app with the REAL leafwing pipeline + the participant and
    /// the touch virtual device bound in its `InputMap`.
    fn app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.add_plugins(bevy::input::InputPlugin);
        app.add_plugins(InputManagerPlugin::<SandboxAction>::default());
        if !app.is_plugin_added::<CentralInputStorePlugin>() {
            app.add_plugins(CentralInputStorePlugin);
        }
        app.register_input_kind::<TouchVirtualButton>(InputControlKind::Button);
        app.register_input_kind::<TouchStickDirection>(InputControlKind::Button);
        app.register_input_kind::<TouchVirtualStick>(InputControlKind::DualAxis);
        app.init_resource::<MobileTouchState>();
        app.add_systems(Update, bind_touch_virtual_inputs);
        let participant = app
            .world_mut()
            .spawn((
                InputParticipant::primary(),
                ParticipantContexts::default(),
                ActionState::<SandboxAction>::default(),
                InputMap::<SandboxAction>::default(),
            ))
            .id();
        // First frame binds the virtual device into the fresh InputMap.
        app.update();
        (app, participant)
    }

    fn actions<'a>(app: &'a App, participant: Entity) -> &'a ActionState<SandboxAction> {
        app.world()
            .get::<ActionState<SandboxAction>>(participant)
            .unwrap()
    }

    fn hold(app: &mut App, set: impl Fn(&mut MobileTouchState)) {
        let mut state = app.world_mut().resource_mut::<MobileTouchState>();
        set(&mut state);
    }

    #[test]
    fn a_touch_button_press_resolves_to_its_bound_actions() {
        let (mut app, participant) = app();
        // The Jump button feeds BOTH the gameplay verb and the menu confirm —
        // a DECLARED double-binding, not a hidden branch.
        hold(&mut app, |s| s.0.jump = TouchButton::pressed_now());
        app.update();
        let a = actions(&app, participant);
        assert!(a.pressed(&SandboxAction::Jump), "touch Jump -> Jump");
        assert!(
            a.pressed(&SandboxAction::MenuSelect),
            "touch Jump also -> MenuSelect (declared menu-confirm binding)"
        );
        assert!(
            a.just_pressed(&SandboxAction::Jump),
            "first frame is an edge"
        );

        // Held on the next frame: still pressed, no new edge.
        hold(&mut app, |s| s.0.jump = TouchButton::held_continued());
        app.update();
        let a = actions(&app, participant);
        assert!(a.pressed(&SandboxAction::Jump));
        assert!(
            !a.just_pressed(&SandboxAction::Jump),
            "a held touch button is held, not a fresh press every frame"
        );

        // Release: the edge reaches the action.
        hold(&mut app, |s| s.0.jump = TouchButton::off());
        app.update();
        let a = actions(&app, participant);
        assert!(a.just_released(&SandboxAction::Jump));
    }

    #[test]
    fn back_and_shoulder_buttons_resolve_to_their_declared_actions() {
        let (mut app, participant) = app();
        hold(&mut app, |s| {
            s.0.reset = TouchButton::pressed_now();
            s.0.fly_toggle = TouchButton::pressed_now();
            s.0.shield = TouchButton::pressed_now();
            s.0.special = TouchButton::pressed_now();
        });
        app.update();
        let a = actions(&app, participant);
        assert!(a.pressed(&SandboxAction::Reset), "Reset -> Reset");
        assert!(
            a.pressed(&SandboxAction::MenuBack),
            "Reset doubles as menu Back (declared binding)"
        );
        assert!(a.pressed(&SandboxAction::Utility), "Fly -> Utility");
        assert!(
            a.pressed(&SandboxAction::QuickAction),
            "Shield -> QuickAction"
        );
        assert!(
            a.pressed(&SandboxAction::Special),
            "the dedicated Special button reaches the Special slot (gate 5)"
        );
    }

    #[test]
    fn the_stick_feeds_move_and_menustick_in_leafwing_convention() {
        let (mut app, participant) = app();
        // Drag the on-screen stick fully DOWN (touch state is +Y-down).
        hold(&mut app, |s| s.0.move_y = 1.0);
        app.update();
        let a = actions(&app, participant);
        let move_pair = a.clamped_axis_pair(&SandboxAction::Move);
        let menu_pair = a.clamped_axis_pair(&SandboxAction::MenuStick);
        assert!(
            move_pair.y < -0.9,
            "down-drag publishes -Y in leafwing's +Y-up convention (got {move_pair:?}); \
             the gameplay reader flips it to the sim's +Y-down exactly as for a gamepad"
        );
        assert!(
            menu_pair.y < -0.9,
            "the SAME stick feeds MenuStick, so menus navigate from it (got {menu_pair:?})"
        );
    }

    #[test]
    fn stick_directions_fire_discrete_edges_not_repeats() {
        let (mut app, participant) = app();
        hold(&mut app, |s| s.0.move_y = 1.0);
        app.update();
        assert!(
            actions(&app, participant).just_pressed(&SandboxAction::MoveDown),
            "crossing the direction threshold is a MoveDown press edge"
        );
        // Held past the threshold: no fresh edge — the double-tap-down
        // detectors must not see a held stick as repeated taps.
        app.update();
        let a = actions(&app, participant);
        assert!(a.pressed(&SandboxAction::MoveDown));
        assert!(
            !a.just_pressed(&SandboxAction::MoveDown),
            "a held direction is held, not a fresh tap every frame"
        );
    }

    #[test]
    fn a_preset_swap_keeps_the_virtual_device_bound() {
        let (mut app, participant) = app();
        // A preset swap REPLACES the whole InputMap (sync_preset_input_map).
        *app.world_mut()
            .get_mut::<InputMap<SandboxAction>>(participant)
            .unwrap() = ambition_input::KeyboardPreset::by_index(1).input_map();
        app.update(); // re-bind runs on Changed<InputMap>
        hold(&mut app, |s| s.0.jump = TouchButton::pressed_now());
        app.update();
        assert!(
            actions(&app, participant).pressed(&SandboxAction::Jump),
            "touch stays bound after the preset swap replaced the map"
        );
    }
}

// ─── The gesture lane + presenter policy ────────────────────────────────────

/// The gesture system marks Touch as the active input source on genuine
/// activity — the symmetric counterpart of the keyboard/mouse/gamepad
/// detector — without stomping the marker while the overlay is idle.
#[cfg(feature = "mobile_touch")]
#[test]
fn genuine_touch_activity_marks_touch_active() {
    use super::bevy_plugin::{MenuTouchGestureState, MobileTouchState};
    use super::menu_bridge::fold_touch_gestures;
    use ambition_input::{ActiveInputKind, MenuControlFrame};
    use bevy::input::touch::Touches;
    use bevy::input::ButtonInput;
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<Touches>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_resource::<MobileTouchState>();
    app.init_resource::<MenuTouchGestureState>();
    app.init_resource::<super::placement::TouchControlPlacement>();
    app.init_resource::<MenuControlFrame>();
    app.init_resource::<ambition_persistence::settings::UserSettings>();
    app.insert_resource(ActiveInputKind::Keyboard);
    app.add_systems(Update, fold_touch_gestures);

    // Idle overlay: the marker is untouched.
    app.update();
    assert_eq!(
        *app.world().resource::<ActiveInputKind>(),
        ActiveInputKind::Keyboard
    );

    // A genuine stick drag flips it to Touch.
    app.world_mut().resource_mut::<MobileTouchState>().0.move_y = 1.0;
    app.update();
    assert_eq!(
        *app.world().resource::<ActiveInputKind>(),
        ActiveInputKind::Touch,
        "using the touch joystick marks Touch as the active input source"
    );
}

#[cfg(feature = "mobile_touch")]
#[test]
fn axis_override_drives_knob_only_while_gameplay_owns_the_controls() {
    // While a menu/launcher owns the controls the gameplay axis is ~0, so
    // the knob-drive override must NOT run — otherwise it snaps the knob to
    // center even as the player drags it to navigate. Keyed on the resolved
    // prompt context (the action/cue contract), never on GameMode.
    use super::bevy_plugin::axis_override_drives_knob;
    use ambition_sim_view::ControlContextKind;

    assert!(
        axis_override_drives_knob(ControlContextKind::Gameplay),
        "gameplay: knob should mirror the move axis"
    );
    for context in [
        ControlContextKind::Menu,
        ControlContextKind::Dialogue,
        ControlContextKind::Empty,
    ] {
        assert!(
            !axis_override_drives_knob(context),
            "{context:?}: knob follows the live drag, not the zeroed axis"
        );
    }
}

// ─── Pure state helpers ─────────────────────────────────────────────────────

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

// ─── HUD layout invariants ──────────────────────────────────────────────────

#[cfg(feature = "mobile_touch")]
#[test]
fn touch_hud_z_is_above_every_menu_overlay() {
    // The HUD's `GlobalZIndex` band must sit ABOVE every menu overlay so it
    // renders on top AND wins bevy_ui picking (so the joystick keeps
    // receiving drags and the Back button stays tappable while a menu's
    // full-screen scrim is up). Assert the ordering against the concrete
    // overlay z values used in the menu modules.
    use super::bevy_plugin::TOUCH_HUD_Z;

    // Local `ZIndex` values authored on the menu roots:
    const PAUSE_MENU_Z: i32 = 50;
    const MAP_Z: i32 = 60;
    const GRID_MENU_Z: i32 = 62;
    // Documented worst-case the prompt calls out for the grid root.
    const GRID_GLOBAL_Z_WORST_CASE: i32 = 1000;

    assert!(TOUCH_HUD_Z > PAUSE_MENU_Z);
    assert!(TOUCH_HUD_Z > MAP_Z);
    assert!(TOUCH_HUD_Z > GRID_MENU_Z);
    assert!(
        TOUCH_HUD_Z > GRID_GLOBAL_Z_WORST_CASE,
        "HUD must out-rank even a GlobalZIndex(1000) menu root"
    );
}

#[cfg(feature = "mobile_touch")]
#[test]
fn touch_action_hit_test_includes_fly_button() {
    use super::layout::{
        touch_action_at_position, touch_action_circle, touch_action_layout, TouchActionButton,
        ACTION_CLUSTER_H, ACTION_CLUSTER_W,
    };
    use ambition_platformer_primitives::gameplay_presentation::ScreenRect;

    // A cluster resolved somewhere arbitrary: the hit test follows the
    // PLACEMENT, so a window-relative fixture would be testing the wrong thing.
    let cluster = ScreenRect::from_min_size(
        bevy::prelude::Vec2::new(820.0, 2020.0),
        bevy::prelude::Vec2::new(ACTION_CLUSTER_W, ACTION_CLUSTER_H),
    );
    let fly = touch_action_layout()
        .into_iter()
        .find(|spec| matches!(spec.action, TouchActionButton::FlyToggle))
        .expect("Fly button remains in the touch action layout");
    let (pos, _) = touch_action_circle(fly, cluster);
    assert!(matches!(
        touch_action_at_position(pos, Some(cluster), None),
        Some(TouchActionButton::FlyToggle)
    ));
}

#[cfg(feature = "mobile_touch")]
#[test]
fn touch_action_layout_keeps_visible_circles_apart() {
    use super::layout::touch_action_layout;

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
fn the_drawn_stick_and_the_urld_glyphs_share_one_center() {
    // The stick art and the U/R/L/D glyphs are drawn by two different systems
    // into the same root rect. They used to derive their positions
    // independently — the glyph root re-derived a corner inset from
    // JOYSTICK_MARGIN while the joystick root took the resolver's rect, flush
    // to the screen corner — so the stick sat up-and-LEFT of the glyphs it is
    // supposed to sit inside. Both now key off `art_center`.
    use super::layout::movement_joystick_layout;
    use bevy::math::Vec2;

    let layout = movement_joystick_layout();

    // Where the base ring lands: `offset_joystick_art_within_footprint` writes
    // `art_origin` to `base_offset`, and the crate draws the ring there.
    let ring_center = layout.art_origin() + Vec2::splat(layout.base_size * 0.5);
    // Where the knob rests: `drive_joystick_knob_from_axis` at a neutral axis.
    let knob_half = layout.knob_size * 0.5;
    let base_half = layout.base_size * 0.5;
    let knob_top_left = layout.art_origin() + Vec2::splat(base_half) - Vec2::splat(knob_half);
    let knob_center = knob_top_left + Vec2::splat(knob_half);
    // Where the glyphs orbit: `position_frame_axis_glyphs`.
    let glyph_center = layout.art_center();

    assert_eq!(
        ring_center, glyph_center,
        "base ring center must coincide with the glyph cluster center",
    );
    assert_eq!(
        knob_center, glyph_center,
        "resting knob center must coincide with the glyph cluster center",
    );

    // And the art must stay clear of the screen edge: the reserved footprint is
    // flush to the corner, so the drawn ring's own inset IS the edge buffer
    // that keeps the thumb off the side-swipe gesture zone.
    assert_eq!(
        layout.art_origin().x,
        layout.margin,
        "art must sit JOYSTICK_MARGIN in from the footprint's left edge",
    );
    let bottom_gap = layout.exclusion_size - (layout.art_origin().y + layout.base_size);
    assert!(
        (bottom_gap - layout.margin).abs() < 1e-3,
        "art must sit JOYSTICK_MARGIN ({}) up from the footprint's bottom edge; got {bottom_gap}",
        layout.margin,
    );
}
