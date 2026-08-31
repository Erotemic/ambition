//! Device adapters that build the engine-owned `ControlFrame` resource.
//!
//! The pure, brain-facing [`ControlFrame`] vocabulary lives in
//! `ambition_platformer2d_core`; this module is the input adapter that translates
//! Leafwing `Platformer2dInputActionMonolith`s, control settings, and trigger hysteresis into that
//! frame. Headless/replay/netcode callers can construct `ControlFrame` directly
//! without depending on this crate.

#[cfg(feature = "input")]
use leafwing_input_manager::prelude::ActionState;

#[cfg(feature = "input")]
use ambition_platformer2d_core::ControlFrame;

#[cfg(feature = "input")]
use crate::actions::Platformer2dInputActionMonolith;

/// The largest movement magnitude the walk modifier permits.
///
/// ⛔⛔ IT MUST STAY BELOW `LocomotionTuning::run_commit_frac`, whose default is
/// [`ambition_platformer2d_core::movement::tuning::RUN_COMMIT_FRAC`] (0.55) —
/// that constant is what the simulation compares the magnitude against to decide
/// walk from run, so a cap at or above it would produce a "walk" that runs.
/// Stated as a named constant with the coupling written down rather than a 0.5
/// somebody has to rediscover.
///
/// ⚠ A body may author its OWN `run_commit_frac`. A kit that authors one above
/// this cap simply walks at this speed; a kit that authors one BELOW it would
/// run while the modifier is held, and that body wants its own answer rather
/// than a smaller global number.
pub const WALK_AXIS_CAP: f32 = 0.5;

/// Build a gameplay control frame, applying configurable deadzones,
/// trigger hysteresis, and the burst-input mode from
/// [`crate::settings::ControlSettings`].
///
/// `burst_state` is the persistent trigger edge tracker for the player; it must
/// outlive a single frame so the hysteretic press/release semantics work. The
/// function returns the next state so the caller can store it back into a Bevy
/// resource.
#[cfg(feature = "input")]

pub fn read_gameplay_control_frame_with_settings(
    actions: &ActionState<Platformer2dInputActionMonolith>,
    controls: crate::settings::ControlFilters,
    edges: crate::settings::GameplayEdgeState,
) -> (ControlFrame, crate::settings::GameplayEdgeState) {
    let raw_move = actions.clamped_axis_pair(&Platformer2dInputActionMonolith::Move);
    // Deadzone first, so analog drift does not pollute the magnitude the
    // simulation reads as a gait: the stick's MAGNITUDE is the walk/run
    // distinction (`run * max_run_speed`, cut by `run_commit_frac`), so drift
    // here is a body that walks on its own.
    //
    let (deadzoned_x, deadzoned_y) = crate::settings::ControlSettings::apply_deadzone(
        raw_move.x,
        raw_move.y,
        controls.left_stick_deadzone,
    );
    let axis = bevy::math::Vec2::new(deadzoned_x, deadzoned_y);

    // The modifier slot is reported RAW — held level and press edge — and the adapter assigns it no
    // meaning. Now the state travels intact to the simulation and a body's own rules decide what
    // sustaining it does.
    let modifier_held = actions.pressed(&Platformer2dInputActionMonolith::Modifier);
    let modifier_pressed = actions.just_pressed(&Platformer2dInputActionMonolith::Modifier);
    // ⭐⭐ THE WALK. The simulation reads the stick's MAGNITUDE as the gait —
    // below `run_commit_frac` is a walk, at or above it is a run — and a DIGITAL
    // source can only ever say 1.0. So a keyboard or D-pad fighter could not
    // walk at all: no walk approach, no walk-to-tilt spacing, and
    // `BodyMotionFacts::running` permanently true, which answers every grounded
    // Attack press with the dash attack.
    //
    // ⭐ A CAP, NOT A SCALE, and the difference matters on an analog stick. A
    // scale would take a player already tilting 0.3 down to 0.15 — punishing
    // them for asking to walk while already walking. Capping says what the
    // player means: *do not exceed a walk*. It is therefore source-independent,
    // which is what the composite `Move` binding needs: a gamepad binds BOTH the
    // D-pad and the left stick to it, and a rule that read differently on each
    // would drift the two apart.
    //
    // ⛔ THE LOCOMOTION HALF IS NOT TOUCHED, deliberately: the parity inventory's
    // row says the gap is INPUT and that the gait itself works
    // (`a_light_tilt_walks_and_a_full_one_runs` measures it). This is the one
    // place a key becomes an axis.
    // ⛔ THE `Walk` ACTION, NOT `Modifier`. The modifier slot is already claimed
    // — Mary-O reads `modifier_held` as her RUN — so capping on it here would
    // make her run key slow her down. See `Platformer2dInputActionMonolith::Walk`.
    let walk_held = actions.pressed(&Platformer2dInputActionMonolith::Walk);
    let axis = if walk_held {
        let magnitude = axis.length();
        if magnitude > WALK_AXIS_CAP {
            axis * (WALK_AXIS_CAP / magnitude)
        } else {
            axis
        }
    } else {
        axis
    };
    let left_pressed = actions.just_pressed(&Platformer2dInputActionMonolith::MoveLeft);
    let right_pressed = actions.just_pressed(&Platformer2dInputActionMonolith::MoveRight);
    let up_pressed = actions.just_pressed(&Platformer2dInputActionMonolith::MoveUp);
    let down_pressed = actions.just_pressed(&Platformer2dInputActionMonolith::MoveDown);

    // BURST-press hysteresis: read the analog right trigger value plus the binary
    // RT2 button as the "press level". The settings-defined press / release
    // thresholds collapse trigger jitter into a single edge.
    //
    // the device-side names moved to BURST too, and the WIRE did not. A
    // stored remap is keyed by the action's `Debug` spelling, so `"Dash"` is
    // carried to `"Burst"` by `settings::ControlSettings::migrate_renamed_actions`;
    // the persisted `dash_input_mode` key is PINNED with `#[serde(rename)]`.
    let raw_trigger = actions
        .value(&Platformer2dInputActionMonolith::BurstAnalog)
        .clamp(0.0, 1.0);
    let burst_button_value = if actions.pressed(&Platformer2dInputActionMonolith::Burst) {
        1.0
    } else {
        0.0
    };
    let trigger_value = raw_trigger.max(burst_button_value);
    let (next_burst_state, trigger_edge_pressed) = crate::settings::update_trigger_edge(
        edges.burst,
        trigger_value,
        controls.trigger_release_threshold,
        controls.trigger_press_threshold,
    );
    let burst_pressed = match controls.burst_input_mode {
        crate::settings::BurstInputMode::Trigger => trigger_edge_pressed,
        // Button mode: ignore trigger hysteresis, only the configured Burst
        // button counts (e.g. RB on a 360 pad).
        crate::settings::BurstInputMode::Button => {
            actions.just_pressed(&Platformer2dInputActionMonolith::Burst)
        }
        crate::settings::BurstInputMode::Both => {
            trigger_edge_pressed || actions.just_pressed(&Platformer2dInputActionMonolith::Burst)
        }
    };

    // This is the fix for old-controller drift pushing the blink target upward.
    let raw_aim = actions.clamped_axis_pair(&Platformer2dInputActionMonolith::AimStick);
    let (aim_x_raw, aim_y_raw) = crate::settings::ControlSettings::apply_deadzone(
        raw_aim.x,
        raw_aim.y,
        controls.right_stick_deadzone,
    );
    let aim_y = if controls.invert_aim_y {
        -aim_y_raw
    } else {
        aim_y_raw
    };

    // ⭐⭐ THE C-STICK. In an attack mode the right stick's deflection is a
    // PRESS, not an aim: crossing `AIM_STICK_ATTACK_THRESHOLD` from rest throws
    // the authored strength in that direction, and the same hysteresis the
    // burst trigger uses stops a stick held out from re-firing every frame.
    //
    // ⛔ THE DIRECTION STILL RIDES `aim_x`/`aim_y`, deliberately: the brain
    // already resolves that pair into the body's local frame for the projectile
    // aim, so a C-stick attack needs no second resolution and cannot disagree
    // with one.
    //
    // ⚠ AND THE STICK STOPS AIMING, which is the whole reason this is a MODE. A
    // player whose right stick throws attacks is not aiming a blink with it.
    let (next_aim_stick_state, aim_stick_flicked) = crate::settings::update_trigger_edge(
        edges.aim_stick,
        bevy::math::Vec2::new(aim_x_raw, aim_y_raw).length(),
        controls.right_stick_deadzone,
        crate::settings::AIM_STICK_ATTACK_THRESHOLD,
    );
    let stick_attack = controls
        .right_stick_mode
        .attack_strength()
        .filter(|_| aim_stick_flicked);

    let frame = ControlFrame {
        axis_x: axis.x,
        // Ambition's simulation uses screen-space world coordinates: +Y is
        // downward. Leafwing's virtual D-pads use the usual +Y-up convention.
        axis_y: -axis.y,
        jump_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Jump),
        jump_held: actions.pressed(&Platformer2dInputActionMonolith::Jump),
        jump_released: actions.just_released(&Platformer2dInputActionMonolith::Jump),
        burst_pressed,
        left_pressed,
        right_pressed,
        up_pressed,
        down_pressed,
        fast_fall_pressed: false,
        blink_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Blink),
        blink_held: actions.pressed(&Platformer2dInputActionMonolith::Blink),
        blink_released: actions.just_released(&Platformer2dInputActionMonolith::Blink),
        special_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Special),
        special_held: actions.pressed(&Platformer2dInputActionMonolith::Special),
        attack_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Attack)
            || stick_attack.is_some(),
        attack_held: actions.pressed(&Platformer2dInputActionMonolith::Attack),
        attack_released: actions.just_released(&Platformer2dInputActionMonolith::Attack),
        // ⚠ ONLY `Smash` IS BOUND TODAY. The hint is three-valued so a
        // right-stick tilt mode can force `Tilt` at full deflection (parity
        // inventory §9); no device produces that yet, and an unbound hint is
        // `Auto` — the interpreter reading the stick, which is what every
        // ordinary attack button asks for.
        // ⭐ THE STICK OUTRANKS THE BUTTON when both speak on one frame, because
        // the stick's whole purpose is to force a strength the interpreter would
        // not have chosen. A `StrongAttack` press with no stick flick still means
        // `Smash`, and everything else is `Auto` — the interpreter reading the
        // stick, which is what an ordinary attack button asks for.
        attack_strength_hint: stick_attack.unwrap_or(
            if actions.pressed(&Platformer2dInputActionMonolith::StrongAttack) {
                ambition_platformer2d_core::AttackStrengthHint::Smash
            } else {
                ambition_platformer2d_core::AttackStrengthHint::Auto
            },
        ),
        attack_from_aim_stick: stick_attack.is_some(),
        pogo_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Pogo),
        fly_toggle_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Utility),
        interact_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Interact),
        interact_held: actions.pressed(&Platformer2dInputActionMonolith::Interact),
        reset_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Reset),
        start_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Start),
        projectile_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Projectile),
        projectile_held: actions.pressed(&Platformer2dInputActionMonolith::Projectile),
        projectile_released: actions.just_released(&Platformer2dInputActionMonolith::Projectile),
        shield_held: actions.pressed(&Platformer2dInputActionMonolith::Shield),
        // `just_pressed`, not `pressed`: the grab is an ATTEMPT, and the
        // authored grab move owns how long that attempt stays active. Reading
        // the level here would re-attempt every tick a player leans on the
        // button, which deletes the cost of whiffing one.
        grab_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Grab),
        // A taunt is one press, one performance — the same reason grab reads the
        // edge rather than the level.
        taunt_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Taunt),
        modifier_held,
        modifier_pressed,
        aim_x: aim_x_raw,
        // Match the sim's +Y-down convention.
        aim_y: -aim_y,
    };
    (
        frame,
        crate::settings::GameplayEdgeState {
            burst: next_burst_state,
            aim_stick: next_aim_stick_state,
        },
    )
}

/// Convenience for tests/headless-visible paths: gameplay frame with default
/// control settings and a fresh trigger state.
#[cfg(feature = "input")]
pub fn read_gameplay_control_frame(
    actions: &ActionState<Platformer2dInputActionMonolith>,
) -> ControlFrame {
    let defaults = crate::settings::ControlSettings::default();
    let (frame, _) = read_gameplay_control_frame_with_settings(
        actions,
        crate::settings::ControlFilters::from_settings(&defaults),
        crate::settings::GameplayEdgeState::default(),
    );
    frame
}

/// Read only the gameplay-side state that should still flow during pause/menu
/// mode. Today that's just `start_pressed` (which the pause toggle reads) —
/// every other gameplay action is suppressed.
#[cfg(feature = "input")]
pub fn read_menu_control_frame(
    actions: &ActionState<Platformer2dInputActionMonolith>,
) -> ControlFrame {
    ControlFrame {
        start_pressed: actions.just_pressed(&Platformer2dInputActionMonolith::Start),
        ..ControlFrame::default()
    }
}
