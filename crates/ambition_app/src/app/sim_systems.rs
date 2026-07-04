//! Host-bound simulation systems that CANNOT move down to a library plugin.
//!
//! The body-generic input/timer/dev systems that used to live here folded into
//! their owning `ambition_gameplay_core` modules (C4): `input_timer_system`,
//! `interaction_input_system`, `cleanup_timers_system` → `gameplay_core::player`;
//! `sync_live_player_dev_edits_system` → `gameplay_core::dev`;
//! `apply_suspended_time_scale_system` → `gameplay_core::time::time_control`.
//! The host schedule (`super::plugins::register_player_input_systems`) still owns
//! their ordering + `run_if` gates and references those moved `pub fn`s.
//!
//! The two systems below stay in the app because they call the app-only
//! `super::world_flow::reset_sandbox` (a host/reset concern) AND write
//! `ambition_render::fx::VfxMessage` — and `ambition_gameplay_core` has no
//! `ambition_render` dependency, so they cannot move to a library plugin. The
//! cut-rope replay system is NAMED content (`ambition_content::bosses`); moving it
//! content-side needs the rooms world-hook seam (JD4, fable-reserved), so it stays
//! here for now.
//!
//! Each is a narrow query/resource system registered in the
//! [`SandboxSet::CoreSimulation`] chain configured by
//! [`super::schedule::configure_sandbox_sets`]. Cross-set ordering lives in the
//! schedule; intra-set ordering is expressed by `.chain()` where registered.

use ambition_engine_core as ae;
use bevy::prelude::*;

use ambition_engine_core::RoomGeometry;
use ambition_gameplay_core::dev::dev_tools::EditableMovementTuning;
use ambition_gameplay_core::features;
use ambition_gameplay_core::time::feel::SandboxFeelTuning;
use ambition_gameplay_core::SandboxSimState;
use ambition_input::ControlFrame;
use ambition_render::fx::VfxMessage;
use ambition_sfx::SfxMessage;

/// Detect a player-pressed reset (the Reset button / `controls.reset_pressed`)
/// and execute the full sandbox reset before the rest of the gameplay
/// chain runs.
///
/// Handles input-driven resets before the rest of gameplay. Engine-driven resets
/// still finish in their player-control/simulation call sites because those paths
/// have already mutated the player and must complete cleanup immediately.
///
/// This system clears `ControlFrame::reset_pressed` after handling it
/// so the engine path inside `update_player_control_with_clusters`
/// does not re-trigger a reset on the same frame. Writes sfx/vfx directly to
/// `MessageWriter`s via local Vec buffers (the engine helper
/// `reset_sandbox` still uses Vec push semantics).
///
/// Gated by `gameplay_allowed`: paused / dialogue modes don't process
/// reset input.
pub fn apply_player_reset_input_system(
    mut control_frame: ResMut<ControlFrame>,
    world: Res<RoomGeometry>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock: ResMut<ambition_time::ClockState>,
    mut reset_room_features: MessageWriter<features::ResetRoomFeaturesEvent>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_gameplay_core::player::BodyAnimFacts,
            &mut ambition_characters::actor::BodyCombat,
            &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
            &mut ambition_gameplay_core::player::BodyMelee,
            &mut ambition_gameplay_core::player::PlayerSafetyState,
        ),
        ambition_gameplay_core::actor::PrimaryPlayerOnly,
    >,
    // Reset zeroes the local controller's slot gestures (reset/save identity is a
    // sanctioned PrimaryPlayer concern).
    mut slot_gestures: ResMut<ambition_gameplay_core::player::SlotInteractionState>,
) {
    if !control_frame.reset_pressed {
        return;
    }
    let Ok((mut cluster_item, mut anim, mut combat, mut blink_cam, mut attack, mut safety)) =
        player_q.single_mut()
    else {
        return;
    };
    // Clear the press immediately so the inline engine update in
    // `player_control_phase` doesn't trigger a redundant `player.reset_to`
    // followed by another sandbox-side reset later this frame.
    control_frame.reset_pressed = false;

    let mut clusters = cluster_item.as_clusters_mut();
    super::world_flow::reset_sandbox(
        &world.0,
        &mut sfx_writer,
        &mut vfx_writer,
        &mut clusters,
        &mut sim_state,
        &mut clock,
        &mut safety,
        &mut attack.swing,
        &mut anim,
        &mut combat,
        slot_gestures.primary_mut(),
        &mut blink_cam,
        editable_tuning.as_engine(),
        *feel_tuning,
    );
    reset_room_features.write(features::ResetRoomFeaturesEvent {
        reason: features::RoomResetReason::Manual,
    });
}

/// Replay the cut-rope boss room from a Yarn/dialogue command.
///
/// This intentionally mirrors `apply_player_reset_input_system` instead of
/// driving `ControlFrame::reset_pressed`: the command can run while gameplay
/// input is suspended by dialogue, so relying on the input frame would make the
/// reset timing depend on UI/game-mode scheduling.
pub fn apply_cut_rope_room_replay_request_system(
    mut replay_requests: MessageReader<ambition_content::bosses::CutRopeRoomReplayRequested>,
    world: Res<RoomGeometry>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock: ResMut<ambition_time::ClockState>,
    boss_registry: Res<ambition_gameplay_core::boss_encounter::BossEncounterRegistry>,
    mut save: Option<ResMut<ambition_gameplay_core::persistence::save::SandboxSave>>,
    mut boss_music: Option<ResMut<ambition_gameplay_core::encounter::BossEncounterMusicRequest>>,
    // Cut-rope boss placements in the room — R4 keys "cleared" by placement
    // (`config.id`), so the replay clears those keys (the respawned boss carries
    // the same LDtk id).
    cut_rope_bosses: Query<&ambition_gameplay_core::combat::boss_clusters::BossConfig>,
    mut reset_room_features: MessageWriter<features::ResetRoomFeaturesEvent>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_gameplay_core::player::BodyAnimFacts,
            &mut ambition_characters::actor::BodyCombat,
            &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
            &mut ambition_gameplay_core::player::BodyMelee,
            &mut ambition_gameplay_core::player::PlayerSafetyState,
        ),
        ambition_gameplay_core::actor::PrimaryPlayerOnly,
    >,
    mut slot_gestures: ResMut<ambition_gameplay_core::player::SlotInteractionState>,
) {
    if replay_requests.read().count() == 0 {
        return;
    }
    let cut_rope_placements: Vec<String> = cut_rope_bosses
        .iter()
        .filter(|config| ambition_content::bosses::is_cut_rope_boss(&config.behavior.id))
        .map(|config| config.id.clone())
        .collect();
    ambition_content::bosses::reset_cut_rope_boss_attempt(
        &boss_registry,
        save.as_deref_mut(),
        boss_music.as_deref_mut(),
        &cut_rope_placements,
    );

    let Ok((mut cluster_item, mut anim, mut combat, mut blink_cam, mut attack, mut safety)) =
        player_q.single_mut()
    else {
        reset_room_features.write(features::ResetRoomFeaturesEvent {
            reason: features::RoomResetReason::Manual,
        });
        return;
    };

    let mut clusters = cluster_item.as_clusters_mut();
    super::world_flow::reset_sandbox(
        &world.0,
        &mut sfx_writer,
        &mut vfx_writer,
        &mut clusters,
        &mut sim_state,
        &mut clock,
        &mut safety,
        &mut attack.swing,
        &mut anim,
        &mut combat,
        slot_gestures.primary_mut(),
        &mut blink_cam,
        editable_tuning.as_engine(),
        *feel_tuning,
    );
    reset_room_features.write(features::ResetRoomFeaturesEvent {
        reason: features::RoomResetReason::Manual,
    });
}
