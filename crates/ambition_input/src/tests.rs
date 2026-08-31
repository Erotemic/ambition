//! Unit tests for the input crate (deadzone filtering, preset tables, analog→dir).

use super::*;
use crate::settings::ControlSettings;

#[test]
fn analog_drift_below_deadzone_zeros_movement() {
    // Simulated worn Xbox 360 controller with a small +Y bias.
    let (x, y) = ControlSettings::apply_deadzone(0.04, 0.06, 0.18);
    assert_eq!((x, y), (0.0, 0.0));
    // The same drift fed to analog_to_dir must not pick a direction.
    assert!(analog_to_dir(x, y, 0.5).is_none());
}

#[test]
fn keyboard_preset_presets_returns_four_unique_ids() {
    let presets = KeyboardPreset::presets();
    assert_eq!(presets.len(), 4);
    // Every preset id is unique.
    for (i, a) in presets.iter().enumerate() {
        for b in &presets[i + 1..] {
            assert_ne!(a.id, b.id);
        }
    }
}

#[test]
fn analog_to_dir_picks_dominant_axis() {
    assert_eq!(analog_to_dir(0.8, 0.1, 0.5), Some(MenuDir::Right));
    assert_eq!(analog_to_dir(-0.8, -0.1, 0.5), Some(MenuDir::Left));
    // +y is up in the leafwing convention used here.
    assert_eq!(analog_to_dir(0.1, 0.8, 0.5), Some(MenuDir::Up));
    assert_eq!(analog_to_dir(0.1, -0.8, 0.5), Some(MenuDir::Down));
}

#[test]
fn menu_state_emits_first_press_then_waits_for_initial_delay() {
    let mut state = MenuInputState::default();
    // First frame holding Down: emit immediately.
    let f = state.step(
        false,
        false,
        false,
        false,
        Some(MenuDir::Down),
        false,
        false,
        false,
        0.016,
        0.30,
        0.10,
    );
    assert!(f.down);
    // Continuing to hold for less than the initial delay must not
    // re-emit.
    let mut emits = 0;
    for _ in 0..5 {
        let f = state.step(
            false,
            false,
            false,
            false,
            Some(MenuDir::Down),
            false,
            false,
            false,
            0.016,
            0.30,
            0.10,
        );
        if f.down {
            emits += 1;
        }
    }
    assert_eq!(emits, 0, "should not repeat before initial delay elapses");
}

#[test]
fn menu_state_repeats_after_initial_delay() {
    let mut state = MenuInputState::default();
    // First push to start the hold.
    let _ = state.step(
        false,
        false,
        false,
        false,
        Some(MenuDir::Right),
        false,
        false,
        false,
        0.016,
        0.10,
        0.05,
    );
    let mut emits = 0;
    for _ in 0..40 {
        let f = state.step(
            false,
            false,
            false,
            false,
            Some(MenuDir::Right),
            false,
            false,
            false,
            0.016,
            0.10,
            0.05,
        );
        if f.right {
            emits += 1;
        }
    }
    assert!(emits >= 4, "expected several repeat ticks; got {emits}");
}

#[test]
fn cardinal_edges_pass_through_without_repeat_state() {
    let mut state = MenuInputState::default();
    // D-pad / arrow keys edge fires on one frame but does not start
    // an analog hold.
    let f = state.step(
        true, false, false, false, None, false, false, false, 0.016, 0.30, 0.10,
    );
    assert!(f.up);
    let f = state.step(
        false, false, false, false, None, false, false, false, 0.016, 0.30, 0.10,
    );
    assert!(!f.any_directional());
}

#[test]
fn menu_state_select_passes_through() {
    let mut state = MenuInputState::default();
    let f = state.step(
        false, false, false, false, None, true, false, false, 0.016, 0.30, 0.10,
    );
    assert!(f.select);
    assert!(!f.any_directional());
}

#[test]
fn menu_control_scroll_steps_round_and_clamp() {
    let frame = MenuControlFrame {
        scroll_y: -2.4,
        ..Default::default()
    };
    assert_eq!(frame.vertical_scroll_steps(), -2);
    let frame = MenuControlFrame {
        scroll_y: 42.0,
        ..Default::default()
    };
    assert_eq!(frame.vertical_scroll_steps(), 6);
}

#[test]
fn menu_state_back_passes_through() {
    let mut state = MenuInputState::default();
    let f = state.step(
        false, false, false, false, None, false, true, false, 0.016, 0.30, 0.10,
    );
    assert!(f.back);
}

/// ⭐⭐ A KEYBOARD FIGHTER CAN WALK.
///
/// ⛔⛔ THE SIMULATION READS THE STICK'S MAGNITUDE AS THE GAIT — below
/// `run_commit_frac` is a walk, at or above it is a run — and a DIGITAL source
/// can only ever say 1.0. So a keyboard or D-pad fighter could not walk at all:
/// no walk approach, no walk-to-tilt spacing, and `BodyMotionFacts::running`
/// permanently true, which answers every grounded Attack press with the dash
/// attack. The parity inventory's row points at this file rather than at
/// locomotion, and the gait itself is measured correct by
/// `a_light_tilt_walks_and_a_full_one_runs`.
///
/// ⭐ A CAP, NOT A SCALE. A player already tilting an analog stick to a walk
/// must not be punished for also asking to walk.
#[cfg(feature = "input")]
mod the_walk_modifier {
    use crate::actions::Platformer2dInputActionMonolith as Action;
    use crate::control::{read_gameplay_control_frame_with_settings, WALK_AXIS_CAP};
    use crate::settings::{ControlFilters, GameplayEdgeState};
    use leafwing_input_manager::prelude::ActionState;

    /// An action state holding `Move` at `axis` with Walk up or down.
    fn holding(axis: bevy::math::Vec2, walk: bool) -> ActionState<Action> {
        let mut state = ActionState::<Action>::default();
        state.set_axis_pair(&Action::Move, axis);
        if walk {
            state.press(&Action::Walk);
        }
        state
    }

    fn frame_of(axis: bevy::math::Vec2, walk: bool) -> ambition_platformer2d_core::ControlFrame {
        read_gameplay_control_frame_with_settings(
            &holding(axis, walk),
            ControlFilters::from_settings(&crate::settings::ControlSettings::default()),
            GameplayEdgeState::default(),
        )
        .0
    }

    /// ⛔ THE PREMISE. A digital hold really is full deflection, which is the
    /// whole reason the action has to exist.
    #[test]
    fn a_digital_hold_is_full_deflection_without_the_walk_key() {
        let frame = frame_of(bevy::math::Vec2::new(1.0, 0.0), false);
        assert!(
            (frame.axis_x - 1.0).abs() < 1e-6,
            "a key held is 1.0; got {}",
            frame.axis_x
        );
    }

    #[test]
    fn holding_walk_caps_a_digital_hold_into_the_walk_band() {
        let frame = frame_of(bevy::math::Vec2::new(1.0, 0.0), true);
        assert!(
            (frame.axis_x - WALK_AXIS_CAP).abs() < 1e-6,
            "a walking key hold must come out at the cap; got {}",
            frame.axis_x
        );
        // ⛔ THE COUPLING, ASSERTED. The cap is only a walk because it is below
        // the gait threshold; a change to either that crossed the other would
        // ship a "walk" that runs.
        assert!(
            frame.axis_x < ambition_platformer2d_core::movement::RUN_COMMIT_FRAC,
            "the cap must stay below the gait threshold or the `walk` runs: {} vs {}",
            frame.axis_x,
            ambition_platformer2d_core::movement::RUN_COMMIT_FRAC
        );
    }

    /// ⭐ IT IS A CAP. An analog stick already inside the walk band is left
    /// alone — asking to walk while walking must not slow you further.
    #[test]
    fn an_analog_tilt_already_inside_the_band_is_untouched() {
        // ⚠ COMPARED AGAINST THE SAME INPUT UNHELD, not against the raw number:
        // the deadzone rescales a small tilt before the cap ever sees it, so a
        // raw 0.3 is already ~0.15 by then. The claim is that holding walk
        // changes NOTHING here, and only the pair can say that.
        let held = frame_of(bevy::math::Vec2::new(0.3, 0.0), true);
        let free = frame_of(bevy::math::Vec2::new(0.3, 0.0), false);
        assert!(
            free.axis_x < WALK_AXIS_CAP,
            "the premise: 0.3 must already be inside the walk band, else this \
             measures the cap rather than its absence; got {}",
            free.axis_x
        );
        assert!(
            (held.axis_x - free.axis_x).abs() < 1e-6,
            "asking to walk while already walking slowed the body: {} -> {}",
            free.axis_x,
            held.axis_x
        );
    }

    /// ⭐ AND IT PRESERVES DIRECTION. A diagonal walk is still a diagonal.
    #[test]
    fn the_cap_preserves_direction() {
        let frame = frame_of(bevy::math::Vec2::new(1.0, 1.0), true);
        let magnitude = (frame.axis_x * frame.axis_x + frame.axis_y * frame.axis_y).sqrt();
        assert!(
            (magnitude - WALK_AXIS_CAP).abs() < 1e-5,
            "a capped diagonal must have the cap's magnitude; got {magnitude}"
        );
        assert!(
            (frame.axis_x.abs() - frame.axis_y.abs()).abs() < 1e-5,
            "a 45° hold must stay at 45°: {} vs {}",
            frame.axis_x,
            frame.axis_y
        );
    }
}

/// The right stick as an attack stick — the device half of parity inventory §9.
#[cfg(feature = "input")]
mod the_attack_stick {
    use crate::actions::Platformer2dInputActionMonolith as Action;
    use crate::control::read_gameplay_control_frame_with_settings;
    use crate::settings::{
        ControlFilters, ControlSettings, GameplayEdgeState, RightStickMode,
        AIM_STICK_ATTACK_THRESHOLD,
    };
    use ambition_platformer2d_core::AttackStrengthHint;
    use leafwing_input_manager::prelude::ActionState;

    fn filters(mode: RightStickMode) -> ControlFilters {
        let mut settings = ControlSettings::default();
        settings.right_stick_mode = mode;
        ControlFilters::from_settings(&settings)
    }

    fn aiming(aim: bevy::math::Vec2) -> ActionState<Action> {
        let mut state = ActionState::<Action>::default();
        state.set_axis_pair(&Action::AimStick, aim);
        state
    }

    /// Push the stick to `aim` for two frames, carrying the edge state, and
    /// return both frames — the flick and the frame after it.
    fn push(
        mode: RightStickMode,
        aim: bevy::math::Vec2,
    ) -> (
        ambition_platformer2d_core::ControlFrame,
        ambition_platformer2d_core::ControlFrame,
    ) {
        let (first, edges) = read_gameplay_control_frame_with_settings(
            &aiming(aim),
            filters(mode),
            GameplayEdgeState::default(),
        );
        let (second, _) =
            read_gameplay_control_frame_with_settings(&aiming(aim), filters(mode), edges);
        (first, second)
    }

    /// ⭐⭐ A TILT STICK THROWS A TILT AT FULL DEFLECTION, which is the thing the
    /// one-way `attack_strong_hint` bool made impossible: the deflection armed a
    /// flick, the flick matched the direction, and the interpreter returned
    /// `Smash` however the device asked.
    #[test]
    fn a_tilt_stick_flick_is_a_tilt_press_even_at_full_deflection() {
        let (flick, _) = push(RightStickMode::TiltAttack, bevy::math::Vec2::new(1.0, 0.0));
        assert!(flick.attack_pressed, "the flick did not press attack");
        assert_eq!(flick.attack_strength_hint, AttackStrengthHint::Tilt);
        assert!(
            flick.attack_from_aim_stick,
            "the press did not say it was aimed by the right stick, so the attack \
             would come out in whatever direction the player was running"
        );
    }

    /// …and a smash stick forces the other direction, from the same push.
    #[test]
    fn a_smash_stick_flick_is_a_smash_press() {
        let (flick, _) = push(RightStickMode::SmashAttack, bevy::math::Vec2::new(1.0, 0.0));
        assert!(flick.attack_pressed);
        assert_eq!(flick.attack_strength_hint, AttackStrengthHint::Smash);
    }

    /// ⛔⛔ THE FLICK RECENTERS BEFORE THE SIM TICK, AND THE DIRECTION MUST
    /// SURVIVE IT — the production path, adapter → latch, not the adapter alone.
    ///
    /// Every other C-stick test here reads ONE device frame, where the aim level
    /// still holds the flick. That is the moment the bug cannot appear. A real
    /// flick is fast: the stick is often back at rest by the next device sample,
    /// while the sim tick has not run yet. The press, the strength and
    /// `attack_from_aim_stick` are EDGES and survive; `aim_x`/`aim_y` are LEVELS
    /// and do not — so the direction had to become part of the press.
    ///
    /// ⭐ THE LEFT STICK IS HELD THE OTHER WAY, because that is what makes the
    /// failure visible as a WRONG direction rather than a missing one:
    /// `player::attack_axis` falls back to the movement axis when the attack aim
    /// is zero, so the pre-fix behaviour was an attack thrown at the player's
    /// back.
    #[test]
    fn a_flick_that_recenters_before_the_tick_still_attacks_where_it_pointed() {
        use ambition_platformer2d_core::ControlFrameLatch;

        let mut running_left = aiming(bevy::math::Vec2::new(1.0, 0.0));
        running_left.set_axis_pair(&Action::Move, bevy::math::Vec2::new(-1.0, 0.0));
        let (flick, edges) = read_gameplay_control_frame_with_settings(
            &running_left,
            filters(RightStickMode::TiltAttack),
            GameplayEdgeState::default(),
        );

        // The very next device sample: right stick back at rest, still running left.
        let mut centered = aiming(bevy::math::Vec2::ZERO);
        centered.set_axis_pair(&Action::Move, bevy::math::Vec2::new(-1.0, 0.0));
        let (recentered, _) = read_gameplay_control_frame_with_settings(
            &centered,
            filters(RightStickMode::TiltAttack),
            edges,
        );

        let mut latch = ControlFrameLatch::default();
        latch.accumulate(flick);
        latch.accumulate(recentered);
        let tick = latch.take();

        assert!(tick.attack_pressed, "the sub-tick flick lost its press");
        assert!(tick.attack_from_aim_stick);
        assert_eq!(tick.attack_strength_hint, AttackStrengthHint::Tilt);
        assert!(
            tick.aim_x.abs() < 0.01,
            "premise check: the aim LEVEL must be back at rest, or this arm is \
             measuring the easy case the other tests already cover"
        );
        assert!(
            tick.attack_aim_x > 0.5,
            "the flick pointed RIGHT and the tick must still know it; the aim \
             level is at rest and the movement axis points LEFT, so without the \
             press carrying its own direction this attack comes out backwards"
        );
    }

    /// ⛔ THE HYSTERESIS. A stick held out is one attack, not one per frame —
    /// the same rule the burst trigger has, and without it leaning on the stick
    /// is a machine gun.
    #[test]
    fn a_held_stick_presses_once() {
        let (flick, held) = push(RightStickMode::TiltAttack, bevy::math::Vec2::new(1.0, 0.0));
        assert!(flick.attack_pressed);
        assert!(
            !held.attack_pressed,
            "the stick re-fired while it was still held out"
        );
    }

    /// ⛔ AND THE DEFAULT MODE IS UNTOUCHED. The right stick aims; it does not
    /// attack, at any deflection.
    #[test]
    fn the_aim_mode_never_presses_attack() {
        let (flick, held) = push(RightStickMode::Aim, bevy::math::Vec2::new(1.0, 0.0));
        for frame in [flick, held] {
            assert!(
                !frame.attack_pressed,
                "the right stick pressed attack while it was set to AIM"
            );
            assert_eq!(frame.attack_strength_hint, AttackStrengthHint::Auto);
            assert!(!frame.attack_from_aim_stick);
        }
    }

    /// ⛔ A NUDGE IS NOT A FLICK. Between the deadzone and the threshold the
    /// stick says nothing, so resting a thumb on it does not attack.
    #[test]
    fn a_deflection_below_the_threshold_is_not_a_press() {
        let nudge = AIM_STICK_ATTACK_THRESHOLD * 0.5;
        let (flick, _) = push(
            RightStickMode::TiltAttack,
            bevy::math::Vec2::new(nudge, 0.0),
        );
        assert!(
            !flick.attack_pressed,
            "a {nudge} deflection threw an attack; `AIM_STICK_ATTACK_THRESHOLD` \
             is what makes the gesture deliberate"
        );
    }
}
