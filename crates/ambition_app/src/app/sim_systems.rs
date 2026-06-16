//! Core simulation Bevy systems.
//!
//! Each function is a narrow query/resource system registered in the
//! [`SandboxSet::CoreSimulation`] chain configured by
//! [`super::schedule::configure_sandbox_sets`]. Cross-set ordering lives in the
//! schedule; intra-set ordering is expressed by `.chain()` where registered.

use ambition_sandbox::engine_core as ae;
use bevy::prelude::*;

use ambition_render::fx::VfxMessage;
use ambition_sandbox::audio::SfxMessage;
use ambition_sandbox::dev::dev_tools::{self, EditableAbilitySet, EditableMovementTuning};
use ambition_sandbox::features::{
    self, FeatureEcsWorldOverlay, GameplayBanner, HitEvent as FeatureHitEvent,
};
use ambition_sandbox::input::ControlFrame;
use ambition_sandbox::rooms::{
    GatePortalRegistry, LoadingZoneActivation, RoomSet, RoomTransitionRequested,
};
use ambition_sandbox::time::feel::SandboxFeelTuning;
use ambition_sandbox::{
    GameWorld, MovingPlatformSet, PlayerDiedMessage, SafePositionContext, SandboxSimState,
};

/// Push live dev-tools ability/tuning edits onto the authoritative player.
/// Runs even while gameplay is suspended so the F3 inspector remains responsive.
pub fn sync_live_player_dev_edits_system(
    editable_tuning: Res<EditableMovementTuning>,
    editable_abilities: Res<EditableAbilitySet>,
    mut player_q: Query<
        (
            &mut ambition_sandbox::player::PlayerAbilities,
            &mut ambition_sandbox::player::PlayerFlightState,
            &mut ambition_sandbox::player::PlayerBlinkState,
            &mut ambition_sandbox::player::PlayerDashState,
            &mut ambition_sandbox::player::PlayerJumpState,
        ),
        With<ambition_sandbox::player::PlayerEntity>,
    >,
) {
    let Ok((mut abilities, mut flight, mut blink, mut dash, mut jump)) = player_q.single_mut()
    else {
        return;
    };
    dev_tools::sync_live_ability_edits_clusters(
        &mut abilities,
        &mut flight,
        &mut blink,
        &mut dash,
        &mut jump,
        editable_abilities.as_engine(),
        editable_tuning.as_engine(),
    );
}

/// While gameplay is suspended, force both live and requested sim-clock scale to
/// zero so presentation animations freeze and the smoother cannot ramp up next
/// frame. Gameplay mode leaves scale control to the normal time-control pipeline.
pub fn apply_suspended_time_scale_system(
    mut clock: ResMut<ambition_sandbox::time::clock_state::ClockState>,
    mut target: ResMut<ambition_sandbox::time::time_control::RequestedClockScale>,
) {
    clock.time_scale = 0.0;
    target.sim_clock = 0.0;
}

/// Tick per-frame gameplay timers and detect double-tap gestures.
///
/// Registered with `run_if(gameplay_allowed)` so it only runs in
/// `GameMode::Playing`. Writes `fast_fall_pressed` back to
/// `Res<ControlFrame>` so the player tick sees the updated flag.
/// Sets `PlayerInteractionState::double_tap_up_pending` so the
/// subsequent interaction phase inside the player tick can activate
/// doors/NPCs.
pub fn input_timer_system(
    time: Res<Time>,
    feel_tuning: Res<SandboxFeelTuning>,
    gravity_field: Option<Res<ambition_sandbox::physics::GravityField>>,
    mut sim_state: ResMut<ambition_sandbox::SandboxSimState>,
    mut control_frame: ResMut<ControlFrame>,
    mut player_q: Query<
        (
            &mut ambition_sandbox::player::PlayerCombatState,
            &mut ambition_sandbox::player::PlayerInteractionState,
        ),
        With<ambition_sandbox::player::PlayerEntity>,
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
    // Fast-fall = double-tap toward gravity. The descend EDGE is the screen-down
    // press normally, but the screen-up press under inverted gravity (past ±90°),
    // so the gesture flips with gravity like the other gates.
    let gravity_up = gravity_field.as_deref().is_some_and(|g| g.dir.y < 0.0);
    let descend_pressed = if gravity_up {
        control_frame.up_pressed
    } else {
        control_frame.down_pressed
    };
    let double_tap_down =
        interaction.register_down_tap(descend_pressed, frame_dt, feel.down_double_tap_window);
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
/// [`ambition_sandbox::player::PlayerInteractionState`].
///
/// Downstream consumers read the buffered signal from
/// `PlayerInteractionState::buffered()`. Gated by `gameplay_allowed` so the
/// buffer does not tick down while paused, in dialogue, or mid-cutscene.
///
/// Ordering: must run after `input_timer_system` (which decrements
/// `combat.hitstun_timer` and sets `double_tap_up_pending` from
/// `register_up_tap`) and before `detect_room_transition_system`
/// (which consumes the buffered signal post-player-tick).
pub fn interaction_input_system(
    time: Res<Time>,
    feel_tuning: Res<SandboxFeelTuning>,
    control_frame: Res<ControlFrame>,
    mut player_q: Query<
        (
            &ambition_sandbox::player::PlayerCombatState,
            &mut ambition_sandbox::player::PlayerInteractionState,
        ),
        With<ambition_sandbox::player::PlayerEntity>,
    >,
) {
    let frame_dt = time.delta_secs();
    let feel = *feel_tuning;
    let Ok((combat, mut interaction)) = player_q.single_mut() else {
        return;
    };
    let door_double_tap_up = std::mem::take(&mut interaction.double_tap_up_pending);
    // Reads `Res<ControlFrame>` directly rather than `PlayerInputFrame`
    // because this system runs mid-input-chain — `input_timer_system`
    // writes `fast_fall_pressed` to the resource and the per-player
    // `sync_local_player_input_frame` mirror only fires at the END of
    // the chain. Switching to `PlayerInputFrame` here would read the
    // previous frame's snapshot.
    let raw_interact_pressed = if combat.hitstun_timer > 0.0 {
        false
    } else {
        control_frame.interact_pressed || door_double_tap_up
    };
    let _live =
        interaction.buffered_interact(raw_interact_pressed, frame_dt, feel.interaction_buffer_time);
}

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
    world: Res<GameWorld>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock: ResMut<ambition_sandbox::time::clock_state::ClockState>,
    mut reset_room_features: MessageWriter<features::ResetRoomFeaturesEvent>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            &mut ambition_sandbox::player::PlayerAnimState,
            &mut ambition_sandbox::player::PlayerCombatState,
            &mut ambition_sandbox::player::PlayerInteractionState,
            &mut ambition_sandbox::player::PlayerBlinkCameraState,
            &mut ambition_sandbox::player::ActivePlayerAttack,
            &mut ambition_sandbox::player::PlayerSafetyState,
        ),
        With<ambition_sandbox::player::PlayerEntity>,
    >,
) {
    if !control_frame.reset_pressed {
        return;
    }
    let Ok((
        mut cluster_item,
        mut anim,
        mut combat,
        mut interaction,
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
        &mut clusters,
        &mut sim_state,
        &mut clock,
        &mut safety,
        &mut attack.0,
        &mut anim,
        &mut combat,
        &mut interaction,
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
    world: Res<GameWorld>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock: ResMut<ambition_sandbox::time::clock_state::ClockState>,
    mut boss_registry: ResMut<ambition_sandbox::boss_encounter::BossEncounterRegistry>,
    mut save: Option<ResMut<ambition_sandbox::persistence::save::SandboxSave>>,
    mut boss_music: Option<ResMut<ambition_sandbox::encounter::BossEncounterMusicRequest>>,
    mut reset_room_features: MessageWriter<features::ResetRoomFeaturesEvent>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            &mut ambition_sandbox::player::PlayerAnimState,
            &mut ambition_sandbox::player::PlayerCombatState,
            &mut ambition_sandbox::player::PlayerInteractionState,
            &mut ambition_sandbox::player::PlayerBlinkCameraState,
            &mut ambition_sandbox::player::ActivePlayerAttack,
            &mut ambition_sandbox::player::PlayerSafetyState,
        ),
        With<ambition_sandbox::player::PlayerEntity>,
    >,
) {
    if replay_requests.read().count() == 0 {
        return;
    }
    ambition_content::bosses::reset_cut_rope_boss_attempt(
        &mut *boss_registry,
        save.as_deref_mut(),
        boss_music.as_deref_mut(),
    );

    let Ok((
        mut cluster_item,
        mut anim,
        mut combat,
        mut interaction,
        mut blink_cam,
        mut attack,
        mut safety,
    )) = player_q.single_mut()
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
        &mut attack.0,
        &mut anim,
        &mut combat,
        &mut interaction,
        &mut blink_cam,
        editable_tuning.as_engine(),
        *feel_tuning,
    );
    reset_room_features.write(features::ResetRoomFeaturesEvent {
        reason: features::RoomResetReason::Manual,
    });
}

/// Detect a loading-zone overlap and emit a [`RoomTransitionRequested`]
/// message. The actual room load (despawn old, spawn new, reset player
/// to spawn point) happens in `apply_room_transition_system`, which
/// runs immediately after this system in the `CoreSimulation` chain.
///
/// Ordering is player tick → detect transition → apply transition. Attacks may
/// still advance on a transition frame, but replay fixtures confirm player-position
/// determinism because attacks do not push the player.
///
/// Gated by `gameplay_allowed`: transitions must not fire while paused
/// or in dialogue. `apply_room_transition_system` itself is unconditional
/// because it reads its own message queue and is a no-op when empty.
pub fn detect_room_transition_system(
    room_set: Res<RoomSet>,
    sim_state: Res<SandboxSimState>,
    portals: Res<GatePortalRegistry>,
    mut transition_writer: MessageWriter<RoomTransitionRequested>,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            &mut ambition_sandbox::player::PlayerInteractionState,
        ),
        With<ambition_sandbox::player::PlayerEntity>,
    >,
) {
    if sim_state.room_transition_cooldown > 0.0 {
        return;
    }
    let Ok((mut cluster_item, mut interaction)) = player_q.single_mut() else {
        return;
    };
    let clusters = cluster_item.as_clusters_mut();
    let Some(zone) =
        room_set.transition_for_player(clusters.kinematics.aabb(), interaction.buffered())
    else {
        return;
    };
    // Portal check: if this zone is registered as a portal, the
    // portal's own phase must be `On` for traversal to be allowed.
    // The switch only commands the boot/shutdown sequence — the
    // portal itself runs the state machine. Non-portal zones pass
    // through unchanged.
    if portals.is_portal(&zone.zone.id) && !portals.allows_traversal(&zone.zone.id) {
        return;
    }
    let zone_sfx = match zone.zone.activation {
        LoadingZoneActivation::Door => Some(ambition_sfx::ids::WORLD_DOOR_OPEN),
        // Walk-through zones (mid-room portals and side-edge exits)
        // both use the portal-enter sfx — the door-open sound only
        // fits the discrete interact door beat.
        LoadingZoneActivation::EdgeExit | LoadingZoneActivation::Walk => {
            Some(ambition_sfx::ids::WORLD_PORTAL_ENTER)
        }
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
/// Runs after transition detection so ordering remains detect → attack → apply.
/// Engine helpers still collect sfx/vfx in local Vecs; this system drains them to
/// the real message writers.
pub fn attack_advance_system(
    time: Res<Time>,
    world: Res<GameWorld>,
    moving_platforms: Res<MovingPlatformSet>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    feature_ecs_overlay: Res<FeatureEcsWorldOverlay>,
    gravity_field: Option<Res<ambition_sandbox::physics::GravityField>>,
    mut player_q: Query<
        (
            Entity,
            ae::PlayerClusterQueryData,
            &mut ambition_sandbox::player::PlayerAnimState,
            &mut ambition_sandbox::player::PlayerCombatState,
            &mut ambition_sandbox::player::ActivePlayerAttack,
            &ambition_sandbox::brain::ActorControl,
            Option<&ambition_sandbox::features::HeldItem>,
        ),
        With<ambition_sandbox::player::PlayerEntity>,
    >,
    mut brain_actions: MessageReader<ambition_sandbox::brain::ActorActionMessage>,
    mut hit_events: MessageWriter<FeatureHitEvent>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
) {
    let Ok((
        player_entity,
        mut cluster_item,
        mut anim,
        mut combat,
        mut attack,
        actor_control,
        held_item,
    )) = player_q.single_mut()
    else {
        return;
    };
    // Only an actually-held weapon (axe etc.) re-tunes the swing; the default
    // ActionSet melee keeps the directional attack_spec_from_view feel.
    let held_melee = held_item.and_then(|item| item.spec.melee);
    // The brain-driver system populated this `ActorControl` for the
    // current player upstream (PlayerInput set). Every combat verb
    // start_attack needs (pogo, axes for attack-intent resolution)
    // lives on the brain-driven frame — the raw `PlayerInputFrame`
    // is no longer read in this system.
    let actor_frame = actor_control.0;
    let mut tuning = editable_tuning.as_engine();
    // Sync gravity into the tuning so the pogo bounce (and any other
    // gravity-relative impulse this system applies) launches OPPOSITE the live
    // gravity, not a hardcoded world-up. Without this the attack-path pogo used
    // the default `(0,1)` down and bounced the wrong way under inverted gravity.
    let gdir = gravity_field
        .as_deref()
        .map_or(ae::Vec2::new(0.0, 1.0), |g| g.dir);
    ambition_sandbox::physics::apply_gravity_dir(&mut tuning, gdir);
    let feel = *feel_tuning;
    let frame_dt = time.delta_secs();

    let mut clusters = cluster_item.as_clusters_mut();
    // Melee comes through the ActionSet-resolved brain message; pogo is a
    // player-specific intent mirrored onto `ActorControlFrame`.
    let melee_requested = brain_actions
        .read()
        .any(|msg| msg.actor == player_entity && msg.is_melee());
    if combat.hitstun_timer <= 0.0 && (melee_requested || actor_frame.pogo_pressed) {
        super::world_flow::start_attack(
            &mut sfx_writer,
            &mut vfx_writer,
            &mut clusters,
            &mut attack.0,
            &mut anim,
            actor_frame,
            held_melee,
            tuning.gravity_dir,
        );
    }
    super::world_flow::advance_attack(
        player_entity,
        &mut sfx_writer,
        &mut vfx_writer,
        &world.0,
        &moving_platforms.0,
        &mut clusters,
        &mut attack.0,
        &mut anim,
        &mut combat,
        tuning,
        feel,
        frame_dt,
        &feature_ecs_overlay,
        &mut hit_events,
    );
}

/// Resolve this tick's victim-side `HitEvent`s and remember the last safe-spawn
/// position.
///
/// Reads `MessageReader<HitEvent>` and filters to victim-side
/// sources (hazard / enemy / boss); attacker-side hits (player slash,
/// player projectile, pogo) are consumed by
/// `apply_feature_hit_events` separately. Routes the first event
/// through `handle_player_damage_events` — which can knock back,
/// hitstun, hazard-respawn, or fully kill the player — and writes
/// resulting sfx / vfx / died messages directly to their
/// `MessageWriter`s. Then runs `remember_safe_player_position` to
/// update `sim_state.last_safe_player_pos` when the player wasn't
/// damaged this frame, isn't blinking, isn't in hitstun, and isn't
/// mid-room-transition.
///
/// Ordering: must run after the player tick (whose
/// `player_simulation_phase` is the canonical producer of player state
/// for this frame) and before `attack_advance_system` /
/// `detect_room_transition_system` (which both read post-damage player
/// state). Gated by `gameplay_allowed`.
pub fn apply_player_hit_events(
    world: Res<GameWorld>,
    control_frame: Res<ControlFrame>,
    moving_platforms: Res<MovingPlatformSet>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    user_settings: Res<ambition_sandbox::persistence::settings::UserSettings>,
    feature_ecs_overlay: Res<FeatureEcsWorldOverlay>,
    mut sim_state: ResMut<SandboxSimState>,
    mut clock: ResMut<ambition_sandbox::time::clock_state::ClockState>,
    mut banner: ResMut<GameplayBanner>,
    mut hit_events: MessageReader<FeatureHitEvent>,
    mut died_writer: MessageWriter<PlayerDiedMessage>,
    mut sfx_writer: MessageWriter<SfxMessage>,
    mut vfx_writer: MessageWriter<VfxMessage>,
    primary_q: Query<
        Entity,
        (
            With<ambition_sandbox::player::PlayerEntity>,
            With<ambition_sandbox::player::PrimaryPlayer>,
        ),
    >,
    mut player_q: Query<
        (
            Entity,
            ae::PlayerClusterQueryData,
            Option<&mut ambition_sandbox::player::PlayerHealth>,
            &mut ambition_sandbox::player::PlayerAnimState,
            &mut ambition_sandbox::player::PlayerCombatState,
            &mut ambition_sandbox::player::PlayerSafetyState,
        ),
        With<ambition_sandbox::player::PlayerEntity>,
    >,
) {
    let primary = primary_q.single().ok();
    // Drain only victim-side hits — attacker-side hits flow to
    // `apply_feature_hit_events`. The two consumers read the same
    // `HitEvent` channel from independent `MessageReader` positions
    // so both see every event but each filters by source-direction.
    let events: Vec<FeatureHitEvent> = hit_events
        .read()
        .filter(|e| !e.source.is_attacker_side())
        .cloned()
        .collect();

    let assist_factor = match user_settings.gameplay.assist {
        ambition_sandbox::persistence::settings::AssistMode::Off => 1.0,
        ambition_sandbox::persistence::settings::AssistMode::On => 0.5,
    };
    let difficulty_multiplier = user_settings.gameplay.difficulty.damage_taken_multiplier()
        * user_settings.gameplay.player_damage_multiplier
        * assist_factor;
    let tuning = editable_tuning.as_engine();
    let feel = *feel_tuning;
    let safe_world =
        features::world_with_sandbox_solids(&world.0, &moving_platforms.0, &feature_ecs_overlay);

    // Resolve every event to a concrete target entity once: events
    // with `HitTarget::Player(e)` route to that player; events with
    // `HitTarget::Volume` (legacy "iterates-and-takes-primary") fall
    // back to the primary player. Events that never resolve (no
    // primary, e.g. headless pre-spawn) are silently dropped.
    let resolved: Vec<(Entity, FeatureHitEvent)> = events
        .into_iter()
        .filter_map(|e| {
            let target = match e.target {
                features::HitTarget::Player(entity) => Some(entity),
                features::HitTarget::Volume => primary,
                features::HitTarget::OrbMatch => None,
            };
            target.map(|t| (t, e))
        })
        .collect();

    for (player_entity, mut cluster_item, player_health, mut anim, mut combat, mut safety) in
        &mut player_q
    {
        let target_events: Vec<FeatureHitEvent> = resolved
            .iter()
            .filter(|(t, _)| *t == player_entity)
            .map(|(_, e)| e.clone())
            .collect();
        let damaged_this_frame = !target_events.is_empty();

        let mut clusters = cluster_item.as_clusters_mut();
        super::world_flow::handle_player_damage_events(
            &world.0,
            control_frame.shield_held,
            &mut sfx_writer,
            &mut vfx_writer,
            &mut died_writer,
            &mut clusters,
            &mut sim_state,
            &mut clock,
            &mut safety,
            &mut banner,
            player_health.map(|h| h.into_inner()),
            &target_events,
            tuning,
            feel,
            difficulty_multiplier,
            &mut anim,
            &mut combat,
        );

        let ctx = SafePositionContext {
            damaged_this_frame,
            in_hitstun: combat.hitstun_timer > 0.0,
            feature_requested_reset: false,
            blink_grace_active: clusters.blink.grace_timer > 0.0,
            room_transitioning: sim_state.room_transition_cooldown > 0.0,
        };
        ambition_sandbox::remember_safe_player_position(&mut safety, &clusters, &safe_world, ctx);
    }
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
    mut dev_state: ResMut<ambition_sandbox::SandboxDevState>,
    mut player_q: Query<
        (
            &ambition_sandbox::player::BodyKinematics,
            &ambition_sandbox::player::PlayerGroundState,
            &ambition_sandbox::player::PlayerDashState,
            &mut ambition_sandbox::player::PlayerAnimState,
            &mut ambition_sandbox::player::PlayerCombatState,
            &mut ambition_sandbox::player::PlayerBlinkCameraState,
        ),
        With<ambition_sandbox::player::PlayerEntity>,
    >,
) {
    let frame_dt = time.delta_secs();
    let Ok((kinematics, ground, dash, mut anim, mut combat, mut blink_cam)) = player_q.single_mut()
    else {
        return;
    };
    combat.flash_timer = (combat.flash_timer - frame_dt).max(0.0);
    dev_state.preset_flash = (dev_state.preset_flash - frame_dt).max(0.0);
    anim.slash_anim_timer = (anim.slash_anim_timer - frame_dt).max(0.0);
    anim.shoot_anim_timer = (anim.shoot_anim_timer - frame_dt).max(0.0);
    anim.wall_jump_anim_timer = (anim.wall_jump_anim_timer - frame_dt).max(0.0);
    anim.interact_anim_timer = (anim.interact_anim_timer - frame_dt).max(0.0);
    blink_cam.blink_in_timer = (blink_cam.blink_in_timer - frame_dt).max(0.0);
    blink_cam.camera_snap_timer = (blink_cam.camera_snap_timer - frame_dt).max(0.0);
    update_anim_signal_timers(
        ground.on_ground,
        kinematics.vel.y,
        dash.timer,
        &mut anim,
        frame_dt,
    );
}

/// Drive the presentation-only landing + dash-startup timers and capture
/// the per-frame state needed for edge detection.
///
/// The sprite picker (`pick_player_anim`) reads these from the
/// `PlayerAnimState` component. Detection lives here so all presentation
/// timers decay in one phase and so the "previous frame" snapshot is
/// the one immediately before the next gameplay tick.
fn update_anim_signal_timers(
    on_ground: bool,
    vel_y: f32,
    dash_timer: f32,
    anim: &mut ambition_sandbox::player::PlayerAnimState,
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
    anim.anim_prev_vel_y = vel_y;
    anim.anim_prev_dash_timer = dash_timer;
}

#[cfg(test)]
mod suspended_time_tests {
    use super::*;
    use ambition_sandbox::game_mode::{gameplay_suspended, GameMode};
    use ambition_sandbox::time::time_control::RequestedClockScale;
    use ambition_sandbox::WorldTime;
    use bevy::state::app::StatesPlugin;

    /// Regression: when gameplay is suspended (pause / dialogue /
    /// cutscene / room transition), `apply_suspended_time_scale_system`
    /// must zero both `SandboxSimState::time_scale` AND
    /// `RequestedClockScale::sim_clock` BEFORE `refresh_world_time`
    /// snapshots them — otherwise `WorldTime::scaled_dt` stays
    /// non-zero on the first suspended frame and any presentation
    /// system multiplying by it ticks one extra frame after pause
    /// lands.
    #[test]
    fn suspended_frame_zeros_world_time_scaled_dt() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.insert_state(GameMode::Paused);
        app.insert_resource(ambition_sandbox::time::clock_state::ClockState { time_scale: 1.0 });
        app.insert_resource(RequestedClockScale {
            sim_clock: 1.0,
            ..Default::default()
        });
        app.insert_resource(WorldTime {
            raw_dt: 0.016,
            scaled_dt: 0.016,
        });
        app.insert_resource(Time::<()>::default());

        // Mirror the new ordering from `register_player_input_systems`:
        // suspended-zero FIRST, then refresh.
        app.add_systems(
            Update,
            (
                apply_suspended_time_scale_system.run_if(gameplay_suspended),
                ambition_sandbox::refresh_world_time,
            )
                .chain(),
        );

        // Pump one wall-clock tick so refresh_world_time has a real dt.
        let frame = std::time::Duration::from_millis(16);
        app.world_mut().resource_mut::<Time>().advance_by(frame);
        app.update();

        let clock = app
            .world()
            .resource::<ambition_sandbox::time::clock_state::ClockState>();
        let target = app.world().resource::<RequestedClockScale>();
        let wt = app.world().resource::<WorldTime>();
        assert_eq!(
            clock.time_scale, 0.0,
            "suspended frame must zero ClockState.time_scale"
        );
        assert_eq!(
            target.sim_clock, 0.0,
            "suspended frame must zero RequestedClockScale.sim_clock"
        );
        assert_eq!(
            wt.scaled_dt, 0.0,
            "suspended frame must zero WorldTime.scaled_dt (refresh_world_time \
             must see the zeroed time_scale, not last frame's 1.0)"
        );
        // wall_dt keeps ticking through pause — that's the contract.
        assert!(
            (wt.wall_dt() - 0.016).abs() < 1e-6,
            "wall clock must keep ticking through pause"
        );
    }

    /// Gameplay-allowed frames take the regular emit → apply → smooth
    /// path; the suspended fallback is short-circuited by `run_if`.
    /// `refresh_world_time` then sees `sim_state.time_scale = 1.0`
    /// (the default) and reports a non-zero `scaled_dt`.
    #[test]
    fn gameplay_frame_preserves_world_time_scaled_dt() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.insert_state(GameMode::Playing);
        app.insert_resource(ambition_sandbox::time::clock_state::ClockState::default());
        app.insert_resource(RequestedClockScale::default());
        app.insert_resource(WorldTime::default());
        app.insert_resource(Time::<()>::default());

        app.add_systems(
            Update,
            (
                apply_suspended_time_scale_system.run_if(gameplay_suspended),
                ambition_sandbox::refresh_world_time,
            )
                .chain(),
        );

        let frame = std::time::Duration::from_millis(16);
        app.world_mut().resource_mut::<Time>().advance_by(frame);
        app.update();

        let wt = app.world().resource::<WorldTime>();
        assert!(
            wt.scaled_dt > 0.0,
            "gameplay frame must produce a non-zero scaled_dt; got {}",
            wt.scaled_dt
        );
    }
}
