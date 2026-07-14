//! Host-bound simulation systems that CANNOT move down to a library plugin.
//!
//! The body-generic input/timer/dev systems that used to live here folded into
//! their owning `ambition::actors` modules (C4): `input_timer_system`,
//! `interaction_input_system`, `cleanup_timers_system` → `ambition::actors::avatar`;
//! `sync_live_player_dev_edits_system` → `ambition::actors::dev`;
//! `apply_suspended_time_scale_system` → `ambition::actors::time::time_control`.
//! The host schedule (`super::plugins::register_player_input_systems`) still owns
//! their ordering + `run_if` gates and references those moved `pub fn`s.
//!
//! The two systems below stay in the app because they call the app-only
//! `super::world_flow::reset_sandbox` (a host/reset concern). They are
//! otherwise engine-shaped (the replay consumer drains the ENGINE's generic
//! `session::reset::RoomReplayRequested`; content emits it from the
//! `ContentDialogueFollowupSet` slot — the E5-finish de-weave), so they move
//! into [the windowed host] when the reset/world-flow concern moves with them.
//!
//! Each is a narrow query/resource system registered in the
//! [`SandboxSet::CoreSimulation`] chain configured by
//! [`super::schedule::configure_sandbox_sets`]. Cross-set ordering lives in the
//! schedule; intra-set ordering is expressed by `.chain()` where registered.

use ambition::engine_core as ae;
use bevy::prelude::*;

use ambition::actors::time::feel::SandboxFeelTuning;
use ambition::actors::time::time_control::ClockResetRequest;
use ambition::actors::SandboxSimState;
use ambition::combat::{ResetRoomFeaturesEvent, RoomResetReason};
use ambition::dev_tools::dev_tools::EditableMovementTuning;
use ambition::engine_core::RoomGeometry;
use ambition::input::ControlFrame;
use ambition::sfx::SfxWriter;
use ambition::vfx::VfxMessage;

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
    world: ambition::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock_resets: MessageWriter<ClockResetRequest>,
    mut reset_room_features: MessageWriter<ResetRoomFeaturesEvent>,
    mut sfx_writer: SfxWriter,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition::actors::features::MotionModel,
            &mut ambition::actors::actor::BodyAnimFacts,
            &mut ambition::characters::actor::BodyCombat,
            &mut ambition::actors::avatar::PlayerBlinkCameraState,
            &mut ambition::actors::actor::BodyMelee,
            &mut ambition::actors::avatar::PlayerSafetyState,
        ),
        ambition::actors::actor::PrimaryPlayerOnly,
    >,
    // Reset zeroes the local controller's slot gestures (reset/save identity is a
    // sanctioned PrimaryPlayer concern).
    mut slot_gestures: ResMut<ambition::actors::control::SlotInteractionState>,
) {
    if !control_frame.reset_pressed {
        return;
    }
    let Ok((
        mut cluster_item,
        mut motion_model,
        mut anim,
        mut combat,
        mut blink_cam,
        mut attack,
        mut safety,
    )) = player_q.single_mut()
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
        &mut motion_model,
        &mut clusters,
        &mut sim_state,
        &mut clock_resets,
        &mut safety,
        &mut attack.swing,
        &mut anim,
        &mut combat,
        slot_gestures.primary_mut(),
        &mut blink_cam,
        editable_tuning.as_engine(),
        *feel_tuning,
    );
    reset_room_features.write(ResetRoomFeaturesEvent {
        reason: RoomResetReason::Manual,
    });
}

/// Replay the ACTIVE room from a content-emitted request (the engine's
/// generic `RoomReplayRequested` — e.g. a "try again" dialogue beat).
///
/// This intentionally mirrors `apply_player_reset_input_system` instead of
/// driving `ControlFrame::reset_pressed`: the command can run while gameplay
/// input is suspended by dialogue, so relying on the input frame would make the
/// reset timing depend on UI/game-mode scheduling.
pub fn apply_room_replay_request_system(
    mut replay_requests: MessageReader<ambition::actors::session::reset::RoomReplayRequested>,
    world: ambition::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock_resets: MessageWriter<ClockResetRequest>,
    boss_registry: Res<ambition::actors::boss_encounter::BossEncounterRegistry>,
    mut save: Option<ResMut<ambition::persistence::save::SandboxSave>>,
    mut boss_music: Option<
        ambition::platformer::lifecycle::SessionWorldMut<
            ambition::encounter::EncounterMusicRequest,
        >,
    >,
    // Cut-rope boss placements in the room — R4 keys "cleared" by placement
    // (`config.id`), so the replay clears those keys (the respawned boss carries
    // the same LDtk id).
    cut_rope_bosses: Query<&ambition::actors::features::ecs::boss_clusters::BossConfig>,
    mut reset_room_features: MessageWriter<ResetRoomFeaturesEvent>,
    mut sfx_writer: SfxWriter,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition::actors::features::MotionModel,
            &mut ambition::actors::actor::BodyAnimFacts,
            &mut ambition::characters::actor::BodyCombat,
            &mut ambition::actors::avatar::PlayerBlinkCameraState,
            &mut ambition::actors::actor::BodyMelee,
            &mut ambition::actors::avatar::PlayerSafetyState,
        ),
        ambition::actors::actor::PrimaryPlayerOnly,
    >,
    mut slot_gestures: ResMut<ambition::actors::control::SlotInteractionState>,
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
        // `Single<&mut T>` derefs to `Mut<T>`, so `as_deref_mut` yields
        // `&mut Mut<T>`; peel the extra change-detection layer to `&mut T`.
        boss_music.as_deref_mut().map(|m| &mut **m),
        &cut_rope_placements,
    );

    let Ok((
        mut cluster_item,
        mut motion_model,
        mut anim,
        mut combat,
        mut blink_cam,
        mut attack,
        mut safety,
    )) = player_q.single_mut()
    else {
        reset_room_features.write(ResetRoomFeaturesEvent {
            reason: RoomResetReason::Manual,
        });
        return;
    };

    let mut clusters = cluster_item.as_clusters_mut();
    super::world_flow::reset_sandbox(
        &world.0,
        &mut sfx_writer,
        &mut vfx_writer,
        &mut motion_model,
        &mut clusters,
        &mut sim_state,
        &mut clock_resets,
        &mut safety,
        &mut attack.swing,
        &mut anim,
        &mut combat,
        slot_gestures.primary_mut(),
        &mut blink_cam,
        editable_tuning.as_engine(),
        *feel_tuning,
    );
    reset_room_features.write(ResetRoomFeaturesEvent {
        reason: RoomResetReason::Manual,
    });
}
