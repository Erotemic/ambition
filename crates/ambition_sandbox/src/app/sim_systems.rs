//! Bevy systems extracted from `sandbox_update`.
//!
//! Each function here is a real Bevy system with a narrow query/resource
//! signature. They live in the [`SandboxSet::CoreSimulation`] chain
//! configured by [`super::schedule::configure_sandbox_sets`], and their
//! order inside that chain is expressed by the tuple `.chain()` in
//! `add_simulation_plugins` rather than by `.after(name)` on each system.

use ambition_engine as ae;
use bevy::prelude::*;

use crate::audio::SfxMessage;
use crate::dev_tools::{self, EditableAbilitySet, EditableMovementTuning};
use crate::features::{
    self, DamageEvent as FeatureDamageEvent, FeatureEcsWorldOverlay, GameplayBanner,
    PlayerDamageEvent, PogoBounceEvent,
};
use crate::feel::SandboxFeelTuning;
use crate::fx::VfxMessage;
use crate::input::ControlFrame;
use crate::rooms::{LoadingZoneActivation, RoomSet, RoomTransitionRequested};
use crate::{
    CurrentPlayerAttack, GameWorld, MovingPlatformSet, PlayerDiedMessage, SandboxSimState,
    SafePositionContext,
};

/// Push live ability-flag and movement-tuning edits from the dev-tools
/// inspector resources onto the authoritative player. Runs every
/// frame, including paused / dialogue / cutscene, so the F3 inspector
/// keeps working when the sim is suspended — the previous procedural
/// `sandbox_update` called this *before* its mode-gate early-return,
/// so the same "always-on" behavior must be preserved now that
/// `sandbox_update` itself only runs in `GameMode::Playing`.
pub fn sync_live_player_dev_edits_system(
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    mut player_q: Query<
        &mut crate::player::PlayerMovementAuthority,
        With<crate::player::PlayerEntity>,
    >,
) {
    let Ok(mut authority) = player_q.single_mut() else {
        return;
    };
    dev_tools::sync_live_ability_edits(
        &mut authority.player,
        editable_abilities.as_engine(),
        editable_tuning.as_engine(),
    );
}

/// While gameplay is suspended (paused, dialogue, room transition,
/// cutscene), force `SandboxSimState::time_scale` to 0 so any
/// presentation system that scales an animation by `time_scale * dt`
/// freezes. The previous `mode_gate_phase` did the same thing at the
/// top of `sandbox_update`; now that `sandbox_update` is gated by
/// `run_if(gameplay_allowed)`, this needs to live in its own system
/// that runs only when gameplay is *not* allowed.
///
/// In gameplay mode `update_time_scale` (inside `sandbox_update`'s
/// `player_simulation_phase`) drives `time_scale` from hitstop /
/// slowmo / dev settings, so this system intentionally does nothing
/// when gameplay is allowed.
pub fn apply_suspended_time_scale_system(mut sim_state: ResMut<SandboxSimState>) {
    sim_state.time_scale = 0.0;
}

/// Tick per-frame gameplay timers and detect double-tap gestures.
///
/// Registered with `run_if(gameplay_allowed)` so it only runs in
/// `GameMode::Playing`. Writes `fast_fall_pressed` back to
/// `Res<ControlFrame>` so `sandbox_update` sees the updated flag.
/// Sets `PlayerInteractionState::double_tap_up_pending` so the
/// subsequent interaction phase inside `sandbox_update` can activate
/// doors/NPCs.
pub fn input_timer_system(
    time: Res<Time>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut sim_state: ResMut<crate::SandboxSimState>,
    mut control_frame: ResMut<ControlFrame>,
    mut player_q: Query<
        (
            &mut crate::player::PlayerCombatState,
            &mut crate::player::PlayerInteractionState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    let frame_dt = time.delta_secs();
    let feel = *feel_tuning;
    let Ok((mut combat, mut interaction)) = player_q.single_mut() else {
        return;
    };
    sim_state.room_transition_cooldown = (sim_state.room_transition_cooldown - frame_dt).max(0.0);
    combat.damage_invuln_timer = (combat.damage_invuln_timer - frame_dt).max(0.0);
    combat.hitstun_timer = (combat.hitstun_timer - frame_dt).max(0.0);
    let double_tap_down = interaction.register_down_tap(
        control_frame.down_pressed,
        frame_dt,
        feel.down_double_tap_window,
    );
    control_frame.fast_fall_pressed = double_tap_down;
    if double_tap_down {
        interaction.double_tap_down_pending = true;
    }
    let door_double_tap_up = interaction.register_up_tap(
        control_frame.up_pressed,
        frame_dt,
        feel.up_double_tap_window,
    );
    if door_double_tap_up {
        interaction.double_tap_up_pending = true;
    }
    combat.hitstop_timer = (combat.hitstop_timer - frame_dt).max(0.0);
}

/// Fold the explicit `Interact` action together with the
/// `double_tap_up_pending` gesture, gate the result on hit-stun, and
/// advance the per-frame interact buffer on
/// [`crate::player::PlayerInteractionState`].
///
/// Replaces the historical inline `interaction_input_phase` that ran
/// inside `sandbox_update`. Downstream code (notably
/// `detect_room_transition_system`) no longer reads the buffered
/// signal off `ControlFrame`; it reads `PlayerInteractionState::
/// buffered()` directly off the component.
///
/// Gated by `gameplay_allowed`: the buffer must not tick down while
/// paused / in dialogue / mid-cutscene — the previous procedural
/// version was protected by the now-extracted mode-gate early-return.
///
/// Ordering: must run after `input_timer_system` (which decrements
/// `combat.hitstun_timer` and sets `double_tap_up_pending` from
/// `register_up_tap`) and before `detect_room_transition_system`
/// (which consumes the buffered signal post-`sandbox_update`).
pub fn interaction_input_system(
    time: Res<Time>,
    feel_tuning: Res<SandboxFeelTuning>,
    control_frame: Res<ControlFrame>,
    mut player_q: Query<
        (
            &crate::player::PlayerCombatState,
            &mut crate::player::PlayerInteractionState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    let frame_dt = time.delta_secs();
    let feel = *feel_tuning;
    let Ok((combat, mut interaction)) = player_q.single_mut() else {
        return;
    };
    let door_double_tap_up = std::mem::take(&mut interaction.double_tap_up_pending);
    let raw_interact_pressed = if combat.hitstun_timer > 0.0 {
        false
    } else {
        control_frame.interact_pressed || door_double_tap_up
    };
    let _live = interaction.buffered_interact(
        raw_interact_pressed,
        frame_dt,
        feel.interaction_buffer_time,
    );
}

/// Detect a player-pressed reset (the Reset button / `controls.reset_pressed`)
/// and execute the full sandbox reset before the rest of the gameplay
/// chain runs.
///
/// Replaces the inline input-driven half of `reset_phase`. The
/// engine-driven half (a reset surfaced by `update_player_control_with_tuning`
/// or `update_player_simulation_with_tuning` returning `events.reset = true`)
/// still runs inline inside `sandbox_update`'s `player_control_phase`
/// / `player_simulation_phase` — those paths know the engine has
/// already mutated the player and need to finish the sandbox-side
/// cleanup in the same call.
///
/// This system clears `ControlFrame::reset_pressed` after handling it so
/// the engine path inside `update_player_control_with_tuning` does not
/// re-trigger a reset on the same frame. Writes sfx/vfx directly to
/// `MessageWriter`s via local Vec buffers (the engine helper
/// `reset_sandbox` still uses Vec push semantics).
///
/// Gated by `gameplay_allowed`: paused / dialogue modes don't process
/// reset input.
pub fn apply_player_reset_input_system(
    mut control_frame: ResMut<ControlFrame>,
    world: Res<GameWorld>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut sim_state: ResMut<SandboxSimState>,
    mut attack_state: ResMut<CurrentPlayerAttack>,
    mut reset_room_features: MessageWriter<features::ResetRoomFeaturesEvent>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut player_q: Query<
        (
            &mut crate::player::PlayerMovementAuthority,
            &mut crate::player::PlayerAnimState,
            &mut crate::player::PlayerCombatState,
            &mut crate::player::PlayerInteractionState,
            &mut crate::player::PlayerBlinkCameraState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    if !control_frame.reset_pressed {
        return;
    }
    let Ok((mut authority, mut anim, mut combat, mut interaction, mut blink_cam)) =
        player_q.single_mut()
    else {
        return;
    };
    // Clear the press immediately so the inline engine update in
    // `player_control_phase` doesn't trigger a redundant `player.reset_to`
    // followed by another sandbox-side reset later this frame.
    control_frame.reset_pressed = false;

    super::world_flow::reset_sandbox(
        &world.0,
        &mut sfx_writer,
        &mut vfx_writer,
        &mut authority.player,
        &mut sim_state,
        &mut attack_state.0,
        &mut anim,
        &mut combat,
        &mut interaction,
        &mut blink_cam,
        editable_tuning.as_engine(),
        *feel_tuning,
    );
    reset_room_features.write(features::ResetRoomFeaturesEvent);
}

/// Detect a loading-zone overlap and emit a [`RoomTransitionRequested`]
/// message. The actual room load (despawn old, spawn new, reset player
/// to spawn point) happens in `apply_room_transition_system`, which
/// runs immediately after this system in the `CoreSimulation` chain.
///
/// Replaces the inline `room_transition_phase` that used to live inside
/// `sandbox_update` and `PhaseOutcome::Return`-skip `attack_phase`. The
/// extracted ordering — `sandbox_update` → `detect_room_transition_system`
/// → `apply_room_transition_system` — means `attack_phase` now always
/// runs even on a transition frame; this is a tiny semantic change
/// (the in-flight attack hitbox in the old room is wasted) but the
/// replay-fixture regression test confirms player position determinism
/// is preserved because attacks do not push the player.
///
/// Gated by `gameplay_allowed`: transitions must not fire while paused
/// or in dialogue. `apply_room_transition_system` itself is unconditional
/// because it reads its own message queue and is a no-op when empty.
pub fn detect_room_transition_system(
    room_set: Res<RoomSet>,
    sim_state: Res<SandboxSimState>,
    mut transition_writer: MessageWriter<RoomTransitionRequested>,
    mut player_q: Query<
        (
            &crate::player::PlayerMovementAuthority,
            &mut crate::player::PlayerInteractionState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    if sim_state.room_transition_cooldown > 0.0 {
        return;
    }
    let Ok((authority, mut interaction)) = player_q.single_mut() else {
        return;
    };
    let Some(zone) = room_set.transition_for_player(&authority.player, interaction.buffered())
    else {
        return;
    };
    let zone_sfx = match zone.zone.activation {
        LoadingZoneActivation::Door => Some(ambition_sfx::ids::WORLD_DOOR_OPEN),
        LoadingZoneActivation::EdgeExit => Some(ambition_sfx::ids::WORLD_PORTAL_ENTER),
    };
    // Clear the interact buffer so the same press doesn't re-trigger
    // a transition next frame before `load_room` resets it.
    interaction.clear();
    transition_writer.write(RoomTransitionRequested::new(zone, zone_sfx));
}

/// Drive the player's slash / pogo attack lifecycle: start a new
/// swing on rising-edge input (gated by hit-stun), then advance any
/// in-flight attack — applying hits, debris, and recoil through the
/// damage / pogo / sfx / vfx message channels.
///
/// Replaces the inline `attack_phase` that used to run last inside
/// `sandbox_update`. Runs after `detect_room_transition_system` in the
/// `CoreSimulation` chain so its sequencing relative to room transitions
/// matches the prior ordering (detect first, then attack, then apply).
///
/// The two engine-side helpers (`start_attack`, `advance_attack`) still
/// accept `&mut Vec<…>` collectors for sfx and vfx. The extracted system
/// drains those local Vecs to the real `MessageWriter`s at the bottom,
/// which is the same pattern the procedural `FrameFeedback` used to
/// implement — but the channels are no longer threaded through the
/// `sandbox_update` orchestrator.
pub fn attack_advance_system(
    time: Res<Time>,
    world: Res<GameWorld>,
    moving_platforms: Res<MovingPlatformSet>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    control_frame: Res<ControlFrame>,
    feature_ecs_overlay: Res<FeatureEcsWorldOverlay>,
    mut attack_state: ResMut<CurrentPlayerAttack>,
    mut player_q: Query<
        (
            &mut crate::player::PlayerMovementAuthority,
            &mut crate::player::PlayerAnimState,
            &mut crate::player::PlayerCombatState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    mut damage_events: MessageWriter<FeatureDamageEvent>,
    mut pogo_bounces: MessageWriter<PogoBounceEvent>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
) {
    let Ok((mut authority, mut anim, mut combat)) = player_q.single_mut() else {
        return;
    };
    let player = &mut authority.player;
    let controls = *control_frame;
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;
    let frame_dt = time.delta_secs();

    if combat.hitstun_timer <= 0.0 && (controls.attack_pressed || controls.pogo_pressed) {
        super::world_flow::start_attack(
            &mut sfx_writer,
            &mut vfx_writer,
            player,
            &mut attack_state.0,
            &mut anim,
            controls,
        );
    }
    super::world_flow::advance_attack(
        &mut sfx_writer,
        &mut vfx_writer,
        &world.0,
        &moving_platforms.0,
        player,
        &mut attack_state.0,
        &mut anim,
        &mut combat,
        tuning,
        feel,
        frame_dt,
        &feature_ecs_overlay,
        &mut damage_events,
        &mut pogo_bounces,
    );
}

/// Resolve this tick's `PlayerDamageEvent`s + remember the last
/// safe-spawn position. Replaces the inline `damage_heal_dialogue_phase`
/// that used to run inside `sandbox_update`.
///
/// Reads `MessageReader<PlayerDamageEvent>` (no intermediate Vec
/// needed), routes the first event through `handle_player_damage_events`
/// — which can knock back, hitstun, hazard-respawn, or fully kill the
/// player — and writes resulting sfx / vfx / died messages directly to
/// their `MessageWriter`s. Then runs `remember_safe_player_position`
/// to update `sim_state.last_safe_player_pos` when the player wasn't
/// damaged this frame, isn't blinking, isn't in hitstun, and isn't
/// mid-room-transition.
///
/// Ordering: must run after `sandbox_update` (whose
/// `player_simulation_phase` is the canonical producer of player state
/// for this frame) and before `attack_advance_system` /
/// `detect_room_transition_system` (which both read post-damage player
/// state). Gated by `gameplay_allowed`.
pub fn apply_player_damage_system(
    world: Res<GameWorld>,
    moving_platforms: Res<MovingPlatformSet>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    user_settings: Res<crate::settings::UserSettings>,
    feature_ecs_overlay: Res<FeatureEcsWorldOverlay>,
    mut sim_state: ResMut<SandboxSimState>,
    mut banner: ResMut<GameplayBanner>,
    mut damage_events: MessageReader<PlayerDamageEvent>,
    mut died_writer: MessageWriter<PlayerDiedMessage>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut player_q: Query<
        (
            &mut crate::player::PlayerMovementAuthority,
            Option<&mut crate::player::PlayerHealth>,
            &mut crate::player::PlayerAnimState,
            &mut crate::player::PlayerCombatState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    let Ok((mut authority, player_health, mut anim, mut combat)) = player_q.single_mut() else {
        return;
    };
    let player = &mut authority.player;
    let player_damage_events: Vec<PlayerDamageEvent> = damage_events.read().copied().collect();
    let feature_damaged_player = !player_damage_events.is_empty();

    let assist_factor = match user_settings.gameplay.assist {
        crate::settings::AssistMode::Off => 1.0,
        crate::settings::AssistMode::On => 0.5,
    };
    let difficulty_multiplier = user_settings.gameplay.difficulty.damage_taken_multiplier()
        * user_settings.gameplay.player_damage_multiplier
        * assist_factor;
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;

    super::world_flow::handle_player_damage_events(
        &world.0,
        &mut sfx_writer,
        &mut vfx_writer,
        &mut died_writer,
        player,
        &mut sim_state,
        &mut banner,
        player_health.map(|h| h.into_inner()),
        &player_damage_events,
        tuning,
        feel,
        difficulty_multiplier,
        &mut anim,
        &mut combat,
    );

    let safe_world = features::world_with_sandbox_solids(
        &world.0,
        &moving_platforms.0,
        &feature_ecs_overlay,
    );
    let ctx = SafePositionContext {
        damaged_this_frame: feature_damaged_player,
        in_hitstun: combat.hitstun_timer > 0.0,
        feature_requested_reset: false,
        blink_grace_active: player.blink_grace_timer > 0.0,
        room_transitioning: sim_state.room_transition_cooldown > 0.0,
    };
    crate::remember_safe_player_position(&mut sim_state, player, &safe_world, ctx);
}

/// Decay presentation-only animation and flash timers.
///
/// Runs every frame (including paused/dialogue) so visual flash and
/// animation pose timers wind down continuously, not just during
/// gameplay. Owns: real-time decay of `flash_timer`, `preset_flash`,
/// `slash_anim_timer`, `blink_in_timer`, `camera_snap_timer`. New
/// presentation-flash timers belong here; gameplay timers belong in
/// `input_timer_system`.
pub fn cleanup_timers_system(
    time: Res<Time>,
    mut dev_state: ResMut<crate::SandboxDevState>,
    mut player_q: Query<
        (
            &crate::player::PlayerMovementAuthority,
            &mut crate::player::PlayerAnimState,
            &mut crate::player::PlayerCombatState,
            &mut crate::player::PlayerBlinkCameraState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    let frame_dt = time.delta_secs();
    let Ok((authority, mut anim, mut combat, mut blink_cam)) = player_q.single_mut() else {
        return;
    };
    let player = &authority.player;
    combat.flash_timer = (combat.flash_timer - frame_dt).max(0.0);
    dev_state.preset_flash = (dev_state.preset_flash - frame_dt).max(0.0);
    anim.slash_anim_timer = (anim.slash_anim_timer - frame_dt).max(0.0);
    blink_cam.blink_in_timer = (blink_cam.blink_in_timer - frame_dt).max(0.0);
    blink_cam.camera_snap_timer = (blink_cam.camera_snap_timer - frame_dt).max(0.0);
    update_anim_signal_timers(player, &mut anim, frame_dt);
}

/// Drive the presentation-only landing + dash-startup timers and capture
/// the per-frame state needed for edge detection.
///
/// The sprite picker (`pick_player_anim`) reads these from the
/// `PlayerAnimState` component. Detection lives here so all presentation
/// timers decay in one phase and so the "previous frame" snapshot is
/// the one immediately before the next gameplay tick.
fn update_anim_signal_timers(
    player: &ae::Player,
    anim: &mut crate::player::PlayerAnimState,
    frame_dt: f32,
) {
    // Hard-landing threshold: pre-touchdown downward speed (px/s) above
    // which we play `LandHard` instead of `LandRecovery`. Tuned by the
    // sandbox's terminal-fall feel; raise if normal jump landings start
    // reading as hard impacts.
    const HARD_LAND_SPEED: f32 = 520.0;
    // Time the landing pose holds after touchdown.
    const LAND_HARD_HOLD_SECS: f32 = 0.34;
    const LAND_SOFT_HOLD_SECS: f32 = 0.16;
    // Brief pre-roll for the dash startup pose. Falls below the dash's
    // own duration so the streaking dash row still gets airtime.
    const DASH_STARTUP_SECS: f32 = 0.05;

    let on_ground = player.on_ground;
    let dash_timer = player.dash_timer;

    // Landing edge: airborne last frame, grounded this frame.
    if on_ground && !anim.anim_prev_on_ground {
        let impact_speed = anim.anim_prev_vel_y;
        let hard = impact_speed >= HARD_LAND_SPEED;
        anim.land_anim_hard = hard;
        anim.land_anim_timer = if hard {
            LAND_HARD_HOLD_SECS
        } else {
            LAND_SOFT_HOLD_SECS
        };
    } else if !on_ground {
        // Stay airborne: the landing pose only plays on the ground.
        anim.land_anim_timer = 0.0;
    } else {
        anim.land_anim_timer = (anim.land_anim_timer - frame_dt).max(0.0);
    }

    // Dash rising edge: previous frame had no dash, this frame has one.
    if dash_timer > 0.0 && anim.anim_prev_dash_timer <= 0.0 {
        anim.dash_startup_timer = DASH_STARTUP_SECS;
    } else {
        anim.dash_startup_timer = (anim.dash_startup_timer - frame_dt).max(0.0);
    }

    // Snapshot for the next frame. Sample vel.y BEFORE any further
    // physics so the landing detector sees the pre-touchdown speed
    // (engine zeroes vertical velocity on contact); this system runs
    // at the end of the gameplay loop, so the player state here is
    // already post-integration but still reflects the speed that produced
    // this frame's `on_ground`.
    anim.anim_prev_on_ground = on_ground;
    anim.anim_prev_vel_y = player.vel.y;
    anim.anim_prev_dash_timer = dash_timer;
}
