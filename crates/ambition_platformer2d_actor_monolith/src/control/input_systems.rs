//! Per-frame player input/timer systems.
//!
//! These publish the primary controller's slot gestures from the local device
//! and tick the home/player body's own reaction + presentation timers. They are
//! body-generic gameplay-sim logic (no render, no host-only types), so they live
//! beside the player state they mutate; the host schedule (`register_player_input_systems`)
//! owns their ordering + `run_if` gates and references these `pub fn`s.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_input::ControlFrame;

/// **The set [`input_timer_system`] runs in — this tick's input timers advance.**
///
/// A reset that must be seen by the timers (the app's player-reset input) lands
/// before it.
///
/// ⚠ ONE member, nested inside `PlayerInputSet::Device`. The parent also holds
/// the slot publish and the frame commit, both of which the reset must NOT
/// precede — it needs to beat the timer decrement, not the whole device phase.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputTimersAdvanced;

/// Tick per-frame gameplay timers and publish the primary controller's slot
/// gestures from the local device.
///
/// Two concerns, deliberately separated by ownership:
/// - **Home-body reaction timers** (`hitstun` / `hitstop` / `damage-invuln` /
///   `recoil`): the home/player body isn't in the actor tick, so it ticks its OWN
///   reaction timers here. This is the home body's own state, NOT authority over the
///   controlled subject — a possessed actor ticks its own timers in the actor path.
/// - **Slot gestures** (double-tap down/up): published from `Res<ControlFrame>` into
///   `SlotInteractionState` for the primary controller slot. Body mode / interaction
///   consume THAT (keyed by the controlled body's slot), never a per-body component.
///
/// The host registers this with `run_if(gameplay_allowed)` so it only runs in
/// `GameMode::Playing`. Writes `fast_fall_pressed` back to `Res<ControlFrame>`.
pub fn input_timer_system(
    time: Res<Time>,
    feel_tuning: Res<crate::time::feel::Platformer2dFeelTuningMonolith>,
    controlled: Option<Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>>,
    frames: Query<&crate::physics::ResolvedMotionFrame>,
    primary_q: Query<Entity, crate::actor::PrimaryPlayerOnly>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    mut sim_state: ResMut<crate::RoomTransitionCooldown>,
    mut control_frame: ResMut<ControlFrame>,
    mut slot_gestures: ResMut<crate::control::SlotInteractionState>,
    // Home/player bodies tick their OWN reaction timers here (they aren't in the
    // actor tick). Iterates every player body so a co-op / clone body ticks its own.
    mut home_feel_q: Query<
        &mut ambition_characters::actor::BodyCombat,
        With<crate::actor::PlayerEntity>,
    >,
) {
    let frame_dt = time.delta_secs();
    let feel = *feel_tuning;
    sim_state.remaining = (sim_state.remaining - frame_dt).max(0.0);
    // ⭐ **ONE decay, called — not a fourth spelling of it** (AC3.3). This was
    // five inline lines that decayed `landing_lag_timer` and forgot `hit_flash`,
    // while the shared `decay_reaction_timers` the actor and boss ticks call
    // decayed `hit_flash` and forgot `landing_lag_timer`. Two lists for one rule,
    // disagreeing in both directions.
    //
    // ⚠ **and picking up `hit_flash` here FIXES a body nobody was decaying.**
    // The blink used to decay in `cleanup_timers_system`, whose query is
    // `PrimaryPlayerOnly` — the HOME AVATAR. This query is `With<PlayerEntity>`,
    // every player body, so a co-op or clone body that took a hit no longer keeps
    // its damage blink lit forever.
    //
    // ⛔⛔ **THE RAW FRAME DELTA HERE IS DELIBERATE, AND MOVING IT TO THE SIM
    // CLOCK BREAKS SEVEN BOSS TESTS — measured 2026-08-18, D117.**
    //
    // The actor and boss ticks pass `world_time.sim_dt()`, and this site looks
    // like the odd one out. It is not. **Hitstop is a `sim_clock` requester** —
    // a connect asks `RequestedClockScale.sim_clock` down, and `scaled_dt =
    // raw_dt × time_scale` follows. So decaying `hitstop_timer` on `sim_dt()`
    // slows the timer that ENDS the freeze by the freeze itself, and the same
    // scale stretches the i-frame and hitstun windows measured against it.
    //
    // ⭐ i-frames are a promise to the PLAYER in real seconds — a bullet-time
    // moment must not hand out longer invulnerability — which is the same
    // reason the double-tap windows below are unscaled.
    //
    // ⚠ **what WAS wrong is the waiver, not the clock.** The `Res<Time>`
    // allowlist entry for this file claimed *"the reaction timers still compute
    // their own scaled dt manually"*, and no such scaling exists or should. A
    // waiver that describes a protection the code does not have is what stops
    // the next reader from checking — and it is why this was "fixed" once,
    // against seven passing tests, before the reason was written down.
    for mut combat in &mut home_feel_q {
        combat.decay_reaction_timers(frame_dt);
    }
    let interaction = slot_gestures.primary_mut();
    // Fast-fall = double-tap local-down for the controlled body. Raw cardinal
    // edges are resolved through the same input mapping policy as locomotion,
    // so ScreenDirected sideways gravity can map raw-right/raw-left into local
    // down/up without bespoke cases here.
    let gravity_dir = crate::control::controlled_frame_down(
        controlled.as_deref(),
        primary_q.single().ok(),
        &frames,
    );
    let movement_mode = user_settings
        .as_deref()
        .map_or(ae::InputFrameMode::DEFAULT_MOVEMENT, |s| {
            s.gameplay.resolved_movement_frame_mode()
        });
    let resolved = ae::AccelerationFrame::new(gravity_dir).resolve_control(
        movement_mode,
        ae::ScreenAxes::new(control_frame.axis_x, control_frame.axis_y),
    );
    let raw_edges = control_frame.raw_direction_edges();
    let descend_pressed = resolved.local_down_pressed(raw_edges);
    let ascend_pressed = resolved.local_up_pressed(raw_edges);
    let double_tap_down =
        interaction.register_down_tap(descend_pressed, frame_dt, feel.down_double_tap_window);
    control_frame.fast_fall_pressed = double_tap_down;
    if double_tap_down {
        interaction.double_tap_down_pending = true;
    }
    let door_double_tap_up =
        interaction.register_up_tap(ascend_pressed, frame_dt, feel.up_double_tap_window);
    if door_double_tap_up {
        interaction.double_tap_up_pending = true;
    }
}

/// Fold the explicit `Interact` action together with the
/// `double_tap_up_pending` gesture, gate the result on the CONTROLLED body's
/// hit-stun, and advance the per-frame interact buffer on the primary
/// controller's slot (`SlotInteractionState`).
///
/// Downstream consumers read the buffered signal from
/// `SlotInteractionState::primary().buffered()` (or the controlled body's slot).
/// The host gates this on `gameplay_allowed` so the buffer does not tick down
/// while paused, in dialogue, or mid-cutscene.
///
/// Ordering: must run after `input_timer_system` (which decrements the controlled
/// body's `combat.hitstun_timer` and sets `double_tap_up_pending` from
/// `register_up_tap`) and before `detect_room_transition_system` (which consumes
/// the buffered signal post-player-tick).
/// **The set [`interaction_input_system`] runs in — the interact buffer is armed.**
///
/// The portal input-warp window opens after this: a warp must not rewrite the
/// frame before the interact press has been buffered for the slot, or the press
/// is attributed to the post-warp state. `portal_schedule` said that by naming
/// this function from another crate.
///
/// ⚠ ONE member. The neighbours in `PlayerInputSet::Device` are the timer
/// decrement before it and the frame commit after it — the two things this sits
/// BETWEEN — so a wider set would erase exactly the window the consumer wants.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InteractionInputBuffered;

pub fn interaction_input_system(
    time: Res<Time>,
    feel_tuning: Res<crate::time::feel::Platformer2dFeelTuningMonolith>,
    control_frame: Res<ControlFrame>,
    frames: Query<&crate::physics::ResolvedMotionFrame>,
    user_settings: Option<Res<ambition_persistence::settings::UserSettings>>,
    controlled: Option<Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>>,
    mut slot_gestures: ResMut<crate::control::SlotInteractionState>,
    // Hit-stun gate reads the CONTROLLED body's reaction state — the body actually
    // being driven, home avatar or possessed actor.
    combat_q: Query<&ambition_characters::actor::BodyCombat>,
    primary_q: Query<
        Entity,
        (
            With<crate::actor::PlayerEntity>,
            With<crate::actor::PrimaryPlayer>,
        ),
    >,
) {
    let frame_dt = time.delta_secs();
    let feel = *feel_tuning;
    let subject = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary_q.single().ok());
    let hitstun = subject
        .and_then(|subject| combat_q.get(subject).ok())
        .map_or(0.0, |combat| combat.hitstun_timer);
    let interaction = slot_gestures.primary_mut();
    let door_double_tap_up = std::mem::take(&mut interaction.double_tap_up_pending);
    // Down + Interact is the possession gesture (`abilities::traversal::possession`),
    // so a held-Down interact is CLAIMED by possession and must NOT also trigger a
    // normal interaction (open a door / start an NPC dialog) — otherwise the press
    // that begins a possession hold also opens whatever's adjacent. Suppress the
    // interact EDGE while Down is held, using the SAME gravity-resolved "down" the
    // possession trigger uses so they agree under any gravity. The double-tap-UP
    // door request is an Up gesture, so it is never suppressed.
    // `subject` already resolved controlled-or-primary above.
    let gravity_dir = crate::control::controlled_frame_down(None, subject, &frames);
    let movement_mode = user_settings
        .as_deref()
        .map_or(ae::InputFrameMode::DEFAULT_MOVEMENT, |s| {
            s.gameplay.resolved_movement_frame_mode()
        });
    let down_held = crate::abilities::traversal::possession::holding_descend(
        control_frame.axis_x,
        control_frame.axis_y,
        gravity_dir,
        movement_mode,
    );
    // Reads `Res<ControlFrame>` directly (local input publication is exactly
    // where raw device state is allowed): this system runs mid-input-chain and
    // publishes the device's interact into the slot buffer. The hit-stun gate uses
    // the CONTROLLED body's `hitstun`, resolved above.
    let raw_interact_pressed = if hitstun > 0.0 {
        false
    } else {
        (control_frame.interact_pressed && !down_held) || door_double_tap_up
    };
    let _live =
        interaction.buffered_interact(raw_interact_pressed, frame_dt, feel.interaction_buffer_time);
}

/// Decay presentation-only animation and flash timers.
///
/// Runs every frame (including paused/dialogue) so visual flash and
/// animation pose timers wind down continuously, not just during
/// gameplay. Owns: real-time decay of `hit_flash`, `preset_flash`,
/// `slash_anim_timer`, `blink_in_timer`, `camera_snap_timer`. New
/// presentation-flash timers belong here; gameplay timers belong in
/// `input_timer_system`.
pub fn cleanup_timers_system(
    time: Res<Time>,
    mut dev_state: ResMut<ambition_dev_tools::DeveloperRuntimeState>,
    mut player_q: Query<
        (
            &ae::BodyMotionFacts,
            &mut crate::actor::BodyAnimFacts,
            &mut ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState,
        ),
        crate::actor::PrimaryPlayerOnly,
    >,
) {
    let frame_dt = time.delta_secs();
    let Ok((motion_facts, mut anim, mut blink_cam)) = player_q.single_mut() else {
        return;
    };
    // ⛔ `hit_flash` is NOT decayed here any more (AC3.3). It is a body-generic
    // reaction timer and it decays with the rest of them in `input_timer_system`,
    // which iterates every `PlayerEntity` rather than the home avatar alone —
    // this system's query could not see a second player body at all.
    dev_state.preset_flash = (dev_state.preset_flash - frame_dt).max(0.0);
    // Player-specific presentation timers (the blink-camera lerp) decay here; the
    // body-generic anim OVERLAYS advance through the shared helper the actor tick
    // also runs (fable review §A9).
    blink_cam.blink_in_timer = (blink_cam.blink_in_timer - frame_dt).max(0.0);
    blink_cam.camera_snap_timer = (blink_cam.camera_snap_timer - frame_dt).max(0.0);
    crate::features::advance_body_anim_overlays(motion_facts.dashing, &mut anim, frame_dt);
}

#[cfg(test)]
mod interaction_suppression_tests {
    use super::*;
    use crate::actor::{PlayerEntity, PrimaryPlayer};
    use crate::control::SlotInteractionState;
    use crate::time::feel::Platformer2dFeelTuningMonolith;
    use ambition_characters::actor::BodyCombat;

    /// Build a minimal app with `interaction_input_system` and one primary
    /// player, set the control frame, run a frame, and report whether the
    /// primary controller's slot interaction buffer went live.
    fn buffered_after(interact: bool, axis_y: f32) -> bool {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.insert_resource(Platformer2dFeelTuningMonolith::default());
        app.init_resource::<SlotInteractionState>();
        app.insert_resource(ControlFrame {
            interact_pressed: interact,
            axis_y,
            ..Default::default()
        });
        app.world_mut()
            .spawn((PlayerEntity, PrimaryPlayer, BodyCombat::default()));
        app.add_systems(Update, interaction_input_system);
        app.update();
        app.world()
            .resource::<SlotInteractionState>()
            .primary()
            .buffered()
    }

    /// A plain Interact (no Down) registers a normal interaction.
    #[test]
    fn plain_interact_registers() {
        assert!(
            buffered_after(true, 0.0),
            "Interact with no Down must trigger a normal interaction"
        );
    }

    /// Down + Interact is the possession gesture and must NOT register a normal
    /// interaction (the in-game bug Jon hit: starting a possession hold next to
    /// an NPC opened its dialog). The Down-held interact edge is suppressed.
    #[test]
    fn down_interact_is_claimed_by_possession_not_a_normal_interact() {
        assert!(
            !buffered_after(true, 1.0),
            "Down+Interact must be claimed by possession, not open a door/NPC"
        );
    }

    /// Sanity: no interact press → nothing buffered, with or without Down.
    #[test]
    fn no_press_buffers_nothing() {
        assert!(!buffered_after(false, 0.0));
        assert!(!buffered_after(false, 1.0));
    }
}
