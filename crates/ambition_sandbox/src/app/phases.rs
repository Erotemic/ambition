#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::input_systems::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::update::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;

/// Phase 1 — dialogue / pause / non-gameplay early returns.
///
/// Owns: zeroing `time_scale`, decaying `flash_timer` + `preset_flash` in
/// modes that intentionally suspend gameplay.
///
/// Should not own: gameplay input edits, movement, combat, or room
/// transitions. New "in dialogue / paused / cutscene" timer decay
/// belongs here; new gameplay logic does not.
pub(super) fn mode_gate_phase(
    mode: &GameMode,
    runtime: &mut SandboxRuntime,
    frame_dt: f32,
) -> PhaseOutcome {
    if matches!(mode, GameMode::Dialogue) {
        runtime.time_scale = 0.0;
        runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
        runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
        return PhaseOutcome::Return;
    }
    if !mode.allows_gameplay() {
        // Pause, dialogue, and transition modes intentionally do not consume
        // gameplay inputs or advance simulation timers. Developer hotkeys
        // and HUD sync remain responsive because those systems are outside
        // this early return.
        runtime.time_scale = 0.0;
        runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
        runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
        return PhaseOutcome::Return;
    }
    PhaseOutcome::Continue
}

/// Phase 2 — gameplay timer decay + semantic input tweaks.
///
/// Owns: per-frame decay of `room_transition_cooldown`,
/// `damage_invuln_timer`, `hitstun_timer`, `hitstop_timer`; rewriting
/// `controls.fast_fall_pressed` from a down double-tap; producing the
/// `door_double_tap_up` signal returned to the caller.
///
/// Should not own: movement, combat, feature runtime updates. New
/// gameplay-only timers and new input-edge gestures belong here. Returns
/// the door / NPC double-tap-up signal so `interaction_input_phase` can
/// fold it in alongside the explicit `Interact` action.
pub(super) fn input_timer_phase(
    controls: &mut ControlFrame,
    runtime: &mut SandboxRuntime,
    feel: SandboxFeelTuning,
    frame_dt: f32,
) -> bool {
    runtime.room_transition_cooldown = (runtime.room_transition_cooldown - frame_dt).max(0.0);
    runtime.damage_invuln_timer = (runtime.damage_invuln_timer - frame_dt).max(0.0);
    runtime.hitstun_timer = (runtime.hitstun_timer - frame_dt).max(0.0);
    let double_tap_down =
        runtime.register_down_tap(controls.down_pressed, frame_dt, feel.down_double_tap_window);
    controls.fast_fall_pressed = double_tap_down;
    // Re-route the double-tap-down edge through SandboxRuntime so the
    // body-mode driver in the progression chain (after sandbox_update)
    // can read it. The local `controls` mutation here doesn't reach
    // post-update systems because sandbox_update consumes a copy of the
    // resource; engine-side fast-fall is consumed inline so the local
    // copy is fine for that, but morph-ball entry needs the edge to
    // survive past `sandbox_update`'s scope.
    if double_tap_down {
        runtime.double_tap_down_pending = true;
    }
    let door_double_tap_up =
        runtime.register_up_tap(controls.up_pressed, frame_dt, feel.up_double_tap_window);
    runtime.hitstop_timer = (runtime.hitstop_timer - frame_dt).max(0.0);
    door_double_tap_up
}

/// Phase 3 — explicit reset input.
///
/// Owns: routing the `reset_pressed` button through `reset_sandbox`. New
/// "the player asked for a reset / restart" branches belong here; engine
/// or feature-driven resets stay in `player_control_phase`,
/// `player_simulation_phase`, or `damage_heal_dialogue_phase`.
pub(super) fn reset_phase(
    controls: &ControlFrame,
    world: &ae::World,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    feature_ecs_queues: &mut features::FeatureEcsQueues,
) -> PhaseOutcome {
    if controls.reset_pressed {
        reset_sandbox(
            world,
            &mut feedback.sfx,
            &mut feedback.vfx,
            runtime,
            tuning,
            feel,
        );
        feature_ecs_queues.reset_room_features = true;
        return PhaseOutcome::Return;
    }
    PhaseOutcome::Continue
}

/// Phase 4 — control-clock half of the two-clock player update.
///
/// Owns: hitstun-filtered control snapshot, real-time `frame_dt`
/// `update_player_control_with_tuning` call, pogo-bounce → feature-event
/// routing, `handle_player_events` for the control-clock pass.
///
/// Should not own: gravity/platform/AI ticks (those run on `sim_dt` in
/// `player_simulation_phase`). New responsive-input mechanics that need
/// real time (jump buffers, blink aim, dash chains) belong here. Returns
/// `Return` if the engine asked for a sandbox reset.
pub(super) fn player_control_phase(
    controls: ControlFrame,
    world: &ae::World,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    feature_ecs_queues: &mut features::FeatureEcsQueues,
) -> PhaseOutcome {
    // Two-clock update:
    // - control_dt is real time for responsive inputs and precision-blink aim;
    // - sim_dt is scaled game time for gravity, platforms, enemies, particles.
    let filtered = controls_for_hitstun(controls, feel, runtime.hitstun_timer);
    let input = filtered.engine_input(frame_dt);
    let control_world =
        features::world_with_sandbox_solids(world, &runtime.moving_platforms, &runtime.features, feature_ecs_overlay);
    let control_events = ae::update_player_control_with_tuning(
        &control_world,
        &mut runtime.player,
        input,
        frame_dt,
        tuning,
    );
    if control_events.reset {
        reset_sandbox(
            world,
            &mut feedback.sfx,
            &mut feedback.vfx,
            runtime,
            tuning,
            feel,
        );
        feature_ecs_queues.reset_room_features = true;
        return PhaseOutcome::Return;
    }
    // Damage breakable pogo orbs the player just bounced off. The
    // engine reports orb AABBs; the sandbox matches them against
    // breakables flagged `pogo_refresh` and routes hit/break events
    // through the standard feature pipeline.
    for &orb_aabb in &control_events.pogo_hits {
        feature_ecs_queues.pogo_bounces.push((orb_aabb, 1));
    }
    handle_player_events(
        &mut feedback.sfx,
        &mut feedback.vfx,
        runtime,
        control_events,
        None,
    );
    PhaseOutcome::Continue
}

/// Phase 5 — sim-clock half of the two-clock player update.
///
/// Owns: `update_time_scale` (hitstop / bullet-time / slowmo ramp),
/// scaled `sim_dt`, moving-platform tick + ride-along, sandbox-side
/// solid rebuild, `update_player_simulation_with_tuning`, landing-dust
/// feedback through `handle_player_events`.
///
/// Should not own: feature-runtime ticks or interact-buffering. New
/// game-time-affected motion (gravity tweaks, platform AI, knockback
/// resolution) belongs here. Returns `Return` if simulation asked for a
/// sandbox reset.
pub(super) fn player_simulation_phase(
    controls: ControlFrame,
    world: &ae::World,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    feature_ecs_queues: &mut features::FeatureEcsQueues,
) -> PhaseOutcome {
    let filtered = controls_for_hitstun(controls, feel, runtime.hitstun_timer);
    let input = filtered.engine_input(frame_dt);

    runtime.update_time_scale(frame_dt, feel);
    let sim_dt = sandbox_dt(runtime, frame_dt);

    let player_for_platforms = &runtime.player;
    let mut riding_platform = None;
    for (index, platform) in runtime.moving_platforms.iter_mut().enumerate() {
        let delta = platform.update(sim_dt);
        if riding_platform.is_none() && platform.is_riding(player_for_platforms) {
            riding_platform = Some((index, delta, platform.pos, platform.direction()));
        }
    }
    let riding_now = riding_platform.is_some();
    let was_riding_platform = runtime.player.was_riding_platform;
    if riding_now != was_riding_platform {
        // Diagnostic: log riding-state transitions. Useful for chasing the
        // "intermittent glitchy platform behavior" repro (TODO S). With
        // multiple authored platforms, the first current rider is reported.
        if let Some((platform_index, _, platform_pos, platform_dir)) = riding_platform {
            debug!(
                target: "ambition::platform",
                riding = true,
                platform_index,
                player_pos = ?runtime.player.pos,
                player_vel = ?runtime.player.vel,
                on_ground = runtime.player.on_ground,
                platform_pos = ?platform_pos,
                platform_dir,
                "moving-platform riding transition"
            );
        } else {
            debug!(
                target: "ambition::platform",
                riding = false,
                player_pos = ?runtime.player.pos,
                player_vel = ?runtime.player.vel,
                on_ground = runtime.player.on_ground,
                "moving-platform riding transition"
            );
        }
    }
    runtime.player.was_riding_platform = riding_now;
    if let Some((_, platform_delta, _, _)) = riding_platform {
        runtime.player.pos += platform_delta;
    }
    let collision_world =
        features::world_with_sandbox_solids(world, &runtime.moving_platforms, &runtime.features, feature_ecs_overlay);

    let was_grounded = runtime.player.on_ground;
    let sim_events = ae::update_player_simulation_with_tuning(
        &collision_world,
        &mut runtime.player,
        input,
        sim_dt,
        tuning,
    );
    if sim_events.reset {
        reset_sandbox(
            world,
            &mut feedback.sfx,
            &mut feedback.vfx,
            runtime,
            tuning,
            feel,
        );
        feature_ecs_queues.reset_room_features = true;
        return PhaseOutcome::Return;
    }
    handle_player_events(
        &mut feedback.sfx,
        &mut feedback.vfx,
        runtime,
        sim_events,
        Some(was_grounded),
    );
    PhaseOutcome::Continue
}

/// Phase 6 — interact / double-tap-up + buffering.
///
/// Owns: hitstun gating of interaction, folding the explicit `Interact`
/// action together with the `door_double_tap_up` signal from
/// `input_timer_phase`, writing the buffered result back into
/// `controls.interact_pressed` via `runtime.buffered_interact`.
///
/// Should not own: actually triggering doors, NPCs, chests, or pickups —
/// `feature_runtime_phase` and `room_transition_phase` consume the
/// buffered signal. Up is too valuable for platforming/flight/aiming to
/// double as a one-tap door or NPC trigger, so doors/NPCs/chests accept
/// either the dedicated `Interact` action or a deliberate double-tap-up
/// gesture.
pub(super) fn interaction_input_phase(
    controls: &mut ControlFrame,
    runtime: &mut SandboxRuntime,
    feel: SandboxFeelTuning,
    door_double_tap_up: bool,
    frame_dt: f32,
) {
    let raw_interact_pressed = if runtime.hitstun_timer > 0.0 {
        false
    } else {
        controls.interact_pressed || door_double_tap_up
    };
    controls.interact_pressed =
        runtime.buffered_interact(raw_interact_pressed, frame_dt, feel.interaction_buffer_time);
}

/// Phase 7 — feature runtime tick.
///
/// Owns: per-frame `runtime.features.update` call for legacy hazards, enemies,
/// bosses, NPCs, switches, and dynamic compatibility chests; routing the
/// resulting audio/vfx/debris cues through `handle_feature_events`. Static
/// pickups, chests, and breakables now run in ECS systems after this phase.
///
/// Should not own: applying the resulting damage / heals / dialogue /
/// reset flags — those are intentionally split into
/// `damage_heal_dialogue_phase` so the side-effect surface is grep-able
/// in one place. Returns the raw `FeatureEvents` so the next phase can
/// consume them.
pub(super) fn feature_runtime_phase(
    controls: &ControlFrame,
    world: &ae::World,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
) -> features::FeatureEvents {
    let feature_dt = sandbox_dt(runtime, frame_dt);
    let feature_world =
        features::world_with_sandbox_solids(world, &runtime.moving_platforms, &runtime.features, feature_ecs_overlay);
    let feature_player = runtime.player.clone();
    // Invincibility short-circuits at the emit site too: otherwise
    // standing in a hazard while the F3 toggle is on would re-emit a
    // damage event (and its impact / message side effects) every frame
    // — the handler drops the event, but the impacts still spawn
    // particles and SFX.
    let player_vulnerable = !runtime.player.invincible && runtime.damage_invuln_timer <= 0.0;
    let feature_events = runtime.features.update(
        &feature_world,
        &feature_player,
        controls.interact_pressed,
        player_vulnerable,
        feel.feature_combat_tuning(),
        feature_dt,
    );
    handle_feature_events(
        &mut feedback.sfx,
        &mut feedback.vfx,
        &mut feedback.debris,
        &feature_events,
        runtime.player.pos,
    );
    feature_events
}

/// Phase 8 — apply heals/damage, dialogue start, feature-driven reset.
///
/// Owns: `handle_player_heal_events`, `handle_player_damage_events`,
/// `remember_safe_player_position` when the player wasn't damaged this
/// frame, clearing the interact buffer when a feature consumed it,
/// starting `GameMode::Dialogue` on a feature-issued dialogue request,
/// routing feature-driven reset through `reset_sandbox`.
///
/// Should not own: the feature tick itself (that's
/// `feature_runtime_phase`) or attack / room-transition routing. Returns
/// `Return` if dialogue started or the feature requested a sandbox
/// reset.
pub(super) fn damage_heal_dialogue_phase(
    world: &ae::World,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    feature_events: &features::FeatureEvents,
    next_mode: &mut NextState<GameMode>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    difficulty_multiplier: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    feature_ecs_queues: &mut features::FeatureEcsQueues,
) -> PhaseOutcome {
    let feature_damaged_player = !feature_events.player_damage.is_empty();
    let feature_interaction_consumed = feature_events.consumed_interaction;
    handle_player_heal_events(runtime, feature_events);
    handle_player_damage_events(
        world,
        &mut feedback.sfx,
        &mut feedback.vfx,
        &mut feedback.died,
        runtime,
        feature_events,
        tuning,
        feel,
        difficulty_multiplier,
    );
    {
        let safe_world = features::world_with_sandbox_solids(
            world,
            &runtime.moving_platforms,
            &runtime.features,
            feature_ecs_overlay,
        );
        let ctx = crate::SafePositionContext {
            damaged_this_frame: feature_damaged_player,
            in_hitstun: runtime.hitstun_timer > 0.0,
            feature_requested_reset: feature_events.reset_player,
            blink_grace_active: runtime.player.blink_grace_timer > 0.0,
            room_transitioning: runtime.room_transition_cooldown > 0.0,
        };
        runtime.remember_safe_player_position(&safe_world, ctx);
    }
    if feature_interaction_consumed {
        runtime.clear_interact_buffer();
    }
    if let Some(request) = &feature_events.dialogue_request {
        runtime
            .dialogue
            .start(&request.dialogue_id, &request.npc_name);
        runtime.clear_interact_buffer();
        runtime.hitstop_timer = 0.0;
        next_mode.set(GameMode::Dialogue);
        return PhaseOutcome::Return;
    }
    if feature_events.reset_player {
        reset_sandbox(
            world,
            &mut feedback.sfx,
            &mut feedback.vfx,
            runtime,
            tuning,
            feel,
        );
        feature_ecs_queues.reset_room_features = true;
        return PhaseOutcome::Return;
    }
    PhaseOutcome::Continue
}

/// Phase 9 — loading-zone transition + `load_room`.
///
/// Owns: cooldown gate, `room_set.transition_for_player` query against
/// the buffered interact signal, clearing the interact buffer on a
/// matched transition, calling `load_room` for the actual swap.
///
/// Should not own: which buttons trigger a transition (that's
/// `interaction_input_phase`) or per-zone content rebuild (that's
/// `load_room`). Returns `Return` if a transition fired this frame.
pub(super) fn room_transition_phase(
    commands: &mut Commands,
    controls: &ControlFrame,
    world: &mut GameWorld,
    room_set: &mut rooms::RoomSet,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    game_assets: Option<&crate::game_assets::GameAssets>,
) -> PhaseOutcome {
    if runtime.room_transition_cooldown > 0.0 {
        return PhaseOutcome::Continue;
    }
    let Some(zone) = room_set.transition_for_player(&runtime.player, controls.interact_pressed)
    else {
        return PhaseOutcome::Continue;
    };
    // Door zones get a `world.door.open` cue at the player's current
    // position (the zone's own AABB is the threshold, but we want the
    // sound at the listener). EdgeExits get `world.portal.enter` —
    // they're conceptually a screen-edge teleport, distinct from
    // walking through a door. Authored content can override later
    // with a `LoadingZoneActivation` variant if heavy doors / save
    // teleports want their own clips.
    let player_pos = runtime.player.pos;
    let zone_sfx = match zone.zone.activation {
        rooms::LoadingZoneActivation::Door => Some(ambition_sfx::ids::WORLD_DOOR_OPEN),
        rooms::LoadingZoneActivation::EdgeExit => Some(ambition_sfx::ids::WORLD_PORTAL_ENTER),
    };
    if let Some(id) = zone_sfx {
        feedback.sfx.push(SfxMessage::Play {
            id,
            pos: player_pos,
        });
    }
    runtime.clear_interact_buffer();
    load_room(
        commands,
        &mut feedback.sfx,
        &mut feedback.vfx,
        runtime,
        world,
        room_set,
        room_visuals,
        zone,
        tuning,
        feel,
        physics_settings,
        game_assets,
    );
    PhaseOutcome::Return
}

/// Phase 10 — slash / pogo attack triggering.
///
/// Owns: hitstun gate, attack/pogo button check, dispatching to
/// `start_attack` / `advance_attack` (which emit sfx/vfx/debris and run
/// feature-side hit application during active frames).
///
/// Should not own: damage application semantics — those live in
/// `advance_attack` and the engine. New attack archetypes should add
/// branches here only when the trigger condition differs.
pub(super) fn attack_phase(
    controls: &ControlFrame,
    world: &ae::World,
    runtime: &mut SandboxRuntime,
    feedback: &mut FrameFeedback,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    feature_ecs_queues: &mut features::FeatureEcsQueues,
) {
    if runtime.hitstun_timer <= 0.0 && (controls.attack_pressed || controls.pogo_pressed) {
        start_attack(&mut feedback.sfx, &mut feedback.vfx, runtime, *controls);
    }
    advance_attack(
        &mut feedback.sfx,
        &mut feedback.vfx,
        world,
        runtime,
        tuning,
        feel,
        frame_dt,
        feature_ecs_overlay,
        feature_ecs_queues,
    );
}

/// Phase 11 — flash / preset / slash animation timer decay.
///
/// Owns: real-time decay of `flash_timer`, `preset_flash`,
/// `slash_anim_timer`. New presentation-flash timers belong here;
/// gameplay timers belong in `input_timer_phase`.
pub(super) fn cleanup_timers_phase(runtime: &mut SandboxRuntime, frame_dt: f32) {
    runtime.flash_timer = (runtime.flash_timer - frame_dt).max(0.0);
    runtime.preset_flash = (runtime.preset_flash - frame_dt).max(0.0);
    runtime.slash_anim_timer = (runtime.slash_anim_timer - frame_dt).max(0.0);
    runtime.blink_in_timer = (runtime.blink_in_timer - frame_dt).max(0.0);
    runtime.camera_snap_timer = (runtime.camera_snap_timer - frame_dt).max(0.0);
    update_anim_signal_timers(runtime, frame_dt);
}

/// Drive the presentation-only landing + dash-startup timers and capture
/// the per-frame state needed for edge detection.
///
/// The sprite picker (`pick_player_anim`) reads these directly. Detection
/// lives here so all presentation timers decay in one phase and so the
/// "previous frame" snapshot is the one immediately before the next
/// gameplay tick.
fn update_anim_signal_timers(runtime: &mut SandboxRuntime, frame_dt: f32) {
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

    let on_ground = runtime.player.on_ground;
    let dash_timer = runtime.player.dash_timer;

    // Landing edge: airborne last frame, grounded this frame.
    if on_ground && !runtime.anim_prev_on_ground {
        let impact_speed = runtime.anim_prev_vel_y;
        let hard = impact_speed >= HARD_LAND_SPEED;
        runtime.land_anim_hard = hard;
        runtime.land_anim_timer = if hard {
            LAND_HARD_HOLD_SECS
        } else {
            LAND_SOFT_HOLD_SECS
        };
    } else if !on_ground {
        // Stay airborne: the landing pose only plays on the ground.
        runtime.land_anim_timer = 0.0;
    } else {
        runtime.land_anim_timer = (runtime.land_anim_timer - frame_dt).max(0.0);
    }

    // Dash rising edge: previous frame had no dash, this frame has one.
    if dash_timer > 0.0 && runtime.anim_prev_dash_timer <= 0.0 {
        runtime.dash_startup_timer = DASH_STARTUP_SECS;
    } else {
        runtime.dash_startup_timer = (runtime.dash_startup_timer - frame_dt).max(0.0);
    }

    // Snapshot for the next frame. Sample vel.y BEFORE any further
    // physics so the landing detector sees the pre-touchdown speed
    // (engine zeroes vertical velocity on contact); cleanup_timers_phase
    // runs at the end of the gameplay loop, so the player state here is
    // already post-integration but still reflects the speed that produced
    // this frame's `on_ground`.
    runtime.anim_prev_on_ground = on_ground;
    runtime.anim_prev_vel_y = runtime.player.vel.y;
    runtime.anim_prev_dash_timer = dash_timer;
}
