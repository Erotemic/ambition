use crate::engine_core::AabbExt;

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
use super::phases::*;
#[allow(unused_imports)]
use super::player_tick::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::*;

pub(super) fn sandbox_dt(hitstop_timer: f32, time_scale: f32, frame_dt: f32) -> f32 {
    if hitstop_timer > 0.0 {
        0.0
    } else {
        frame_dt * time_scale
    }
}

pub(super) fn reset_sandbox(
    world: &ae::World,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    sim_state: &mut crate::SandboxSimState,
    safety: &mut crate::player::PlayerSafetyState,
    attack: &mut Option<crate::PlayerAttackState>,
    anim: &mut crate::player::PlayerAnimState,
    combat: &mut crate::player::PlayerCombatState,
    interaction: &mut crate::player::PlayerInteractionState,
    blink_cam: &mut crate::player::PlayerBlinkCameraState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    let reset_from = clusters.kinematics.pos;
    ae::reset_player_clusters(clusters, world.spawn);
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning,
    );
    clusters.mana.meter.refill_full();
    safety.last_safe_pos = world.spawn;
    sim_state.time_scale = 1.0;
    sim_state.room_transition_cooldown = 0.0;
    *attack = None;
    anim.reset();
    combat.reset();
    combat.flash_timer = feel.reset_flash_time;
    interaction.reset();
    blink_cam.reset();
    let reset_to = clusters.kinematics.pos;
    sfx.write(SfxMessage::Reset { pos: reset_to });
    vfx.write(VfxMessage::ResetEffects {
        from: reset_from,
        to: reset_to,
    });
}

pub(super) fn load_room(
    commands: &mut Commands,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    dev_state: &mut crate::SandboxDevState,
    sim_state: &mut crate::SandboxSimState,
    safety: &mut crate::player::PlayerSafetyState,
    moving_platforms: &mut Vec<crate::world::platforms::MovingPlatformState>,
    dialogue: &mut crate::dialog::DialogState,
    combat: &mut crate::player::PlayerCombatState,
    interaction: &mut crate::player::PlayerInteractionState,
    blink_cam: &mut crate::player::PlayerBlinkCameraState,
    world: &mut GameWorld,
    room_set: &mut rooms::RoomSet,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    transition: rooms::RoomTransition,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    assets: Option<&crate::assets::game_assets::GameAssets>,
) {
    let old_velocity = clusters.kinematics.vel;
    let fly_enabled = clusters.flight.fly_enabled;
    let player_size = clusters.kinematics.size;
    let edge_exit = matches!(
        transition.zone.activation,
        rooms::LoadingZoneActivation::EdgeExit
    );

    for (entity, physics_entity) in room_visuals.iter() {
        if physics_entity.is_some() {
            physics::retire_physics_entity(commands, entity);
        } else {
            commands.entity(entity).despawn();
        }
    }
    let spec = room_set.set_active(transition.target_room).clone();
    world.0 = spec.world.clone();

    // Room transitions are not player deaths/resets. Rebuild transient room
    // state, but preserve ability progression and, for edge exits, preserve
    // velocity so side-to-side room changes feel continuous. Door transitions
    // intentionally zero velocity because they are discrete interactions.
    let arrival = rooms::validated_spawn(&world.0, transition.arrival, player_size);
    ae::reset_player_clusters(clusters, arrival);
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning,
    );
    clusters.flight.fly_enabled = fly_enabled && clusters.abilities.abilities.fly;
    if edge_exit {
        clusters.kinematics.vel = old_velocity;
    }
    blink_cam.blink_in_timer = 0.0;
    blink_cam.blink_camera_from = clusters.kinematics.pos;
    blink_cam.blink_camera_to = clusters.kinematics.pos;
    blink_cam.camera_snap_timer = if edge_exit {
        0.0
    } else {
        crate::ROOM_DOOR_CAMERA_SNAP_TIME
    };
    combat.flash_timer = if edge_exit {
        feel.edge_transition_flash
    } else {
        feel.door_transition_flash
    };
    combat.hitstop_timer = 0.0;
    combat.damage_invuln_timer = 0.0;
    combat.hitstun_timer = 0.0;
    safety.last_safe_pos = clusters.kinematics.pos;
    sim_state.time_scale = 1.0;
    interaction.down_tap_timer = 0.0;
    *moving_platforms = platforms::moving_platforms_for_room(&spec);
    features::spawn_room_feature_entities(commands, &spec);
    dialogue.close();
    // This guard prevents immediate backtracking when arriving inside/near a
    // paired zone. It should not feel like frozen input, so keep it short and
    // rely on validated arrivals to do most of the safety work.
    sim_state.room_transition_cooldown = if edge_exit {
        feel.edge_transition_cooldown
    } else {
        feel.door_transition_cooldown
    };
    dev_state.preset_flash = 1.0;

    crate::presentation::rendering::spawn_parallax_layers(
        commands,
        &world.0,
        &spec.metadata,
        assets,
    );
    spawn_room_visuals(commands, &spec, physics_settings, assets);
    platforms::spawn_moving_platforms(commands, &world.0, moving_platforms);
    let arrival_pos = clusters.kinematics.pos;
    sfx.write(SfxMessage::Reset { pos: arrival_pos });
    if edge_exit {
        // Edge exits should feel like contiguous room scrolling, not a death-like
        // teleport. Only show an arrival puff in the new room because `from` was
        // expressed in the previous room's coordinate space.
        vfx.write(VfxMessage::Burst {
            pos: arrival_pos,
            count: 18,
            speed: 260.0,
            color: [0.35, 0.95, 1.0, 0.75],
            kind: ParticleKind::Dust,
        });
    } else {
        // Door transitions are discrete interactions, so a teleport-like effect
        // is acceptable; use the destination for both endpoints to avoid mixing
        // coordinate systems from two rooms.
        vfx.write(VfxMessage::ResetEffects {
            from: arrival_pos,
            to: arrival_pos,
        });
    }
}

/// Bevy system: reads `RoomTransitionRequested` messages written by
/// `detect_room_transition_system` and applies the room load.
///
/// Runs immediately after the player tick in the `CoreSimulation` chain
/// so the player position, world, and room_set are updated before any
/// other post-sim systems run in the same frame.
pub fn apply_room_transition_system(
    mut commands: Commands,
    mut requests: MessageReader<rooms::RoomTransitionRequested>,
    mut event_writers: SandboxEventWriters,
    mut player_q: Query<
        (
            ae::PlayerClusterQueryData,
            &mut crate::player::PlayerCombatState,
            &mut crate::player::PlayerInteractionState,
            &mut crate::player::PlayerBlinkCameraState,
            &mut crate::player::PlayerSafetyState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    mut world: ResMut<GameWorld>,
    mut room_set: ResMut<rooms::RoomSet>,
    mut dev_state: ResMut<crate::SandboxDevState>,
    mut sim_state: ResMut<crate::SandboxSimState>,
    mut moving_platforms: ResMut<crate::MovingPlatformSet>,
    mut dialogue: ResMut<crate::dialog::DialogState>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    editable_tuning: Res<EditableMovementTuning>,
    feel_tuning: Res<SandboxFeelTuning>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    game_assets: Option<Res<crate::assets::game_assets::GameAssets>>,
    mut combat_reset: super::feedback::CombatRoomReset,
) {
    for request in requests.read() {
        let Ok((mut cluster_item, mut combat, mut interaction, mut blink_cam, mut safety)) =
            player_q.single_mut()
        else {
            continue;
        };
        // Any enemy volleys still in flight from the previous room
        // would otherwise sail across the seam and hit the player
        // mid-transition. The slot board is per-target and the live
        // actor list is about to be torn down + rebuilt, so drop
        // every reservation now and let the next tick rebuild.
        combat_reset.clear_carryover();
        let mut clusters = cluster_item.as_clusters_mut();
        // Play the zone-entry SFX at the pre-load player position so it sounds
        // like it originates from the door/edge the player walked through.
        let player_pos_before = clusters.kinematics.pos;
        if let Some(sfx_id) = request.zone_sfx {
            event_writers.sfx.write(SfxMessage::Play {
                id: sfx_id,
                pos: player_pos_before,
            });
        }
        let target_room = request.transition.target_room;
        load_room(
            &mut commands,
            &mut event_writers.sfx,
            &mut event_writers.vfx,
            &mut clusters,
            &mut dev_state,
            &mut sim_state,
            &mut safety,
            &mut moving_platforms.0,
            &mut dialogue,
            &mut combat,
            &mut interaction,
            &mut blink_cam,
            &mut world,
            &mut room_set,
            &room_visuals,
            request.transition.clone(),
            editable_tuning.as_engine(),
            *feel_tuning,
            *physics_settings,
            game_assets.as_deref(),
        );
        log_room_transition_landing(
            target_room,
            &room_set,
            clusters.kinematics.pos,
            clusters.kinematics.size,
            &world.0,
            &combat_reset.feature_overlay,
        );
    }
}

/// One-line diagnostic emitted on every room transition. Goal: when
/// "player fell through the floor in <room>" reports come in we have
/// the signals on disk / in the browser console to tell apart the
/// usual suspects:
///
/// - `world_blocks` == 0 → `to_room_set()` didn't populate this room's
///   `world.blocks` (LDtk load / merge issue).
/// - `overlay_blocks` == 0 in a room whose floor is breakable / actor
///   / boss → ECS feature spawn raced the post-transition sim tick.
/// - `gap_below_feet` large or `none` → `validated_spawn` placed the
///   player above the floor (`world.0`-only collision check missed the
///   overlay floor) and gravity is about to pull them through.
///
/// Cheap: runs once per RoomTransitionRequested, iterates blocks once
/// to find the highest top-below-feet, no per-frame cost. Filter the
/// browser console / log file with target `ambition::room_transition`.
fn log_room_transition_landing(
    target_room: usize,
    room_set: &rooms::RoomSet,
    pos: ae::Vec2,
    size: ae::Vec2,
    world: &ae::World,
    feature_overlay: &crate::features::FeatureEcsWorldOverlay,
) {
    let target_id = room_set
        .rooms
        .get(target_room)
        .map(|spec| spec.id.clone())
        .unwrap_or_else(|| format!("<index {target_room}>"));
    let feet_y = pos.y + size.y * 0.5;
    let body = ae::Aabb::new(pos, size * 0.5);
    let overlapping_world = world
        .blocks
        .iter()
        .filter(|b| b.aabb.strict_intersects(body))
        .count();
    let overlapping_overlay = feature_overlay
        .blocks
        .iter()
        .filter(|b| b.aabb.strict_intersects(body))
        .count();
    let gap = ground_gap_below_feet(feet_y, &body, world, feature_overlay);
    let gap_desc = match gap {
        Some((distance, source)) => format!("{distance:.1}px ({source})"),
        None => "none within 256px".to_string(),
    };
    bevy::log::info!(
        target: "ambition::room_transition",
        "room transition: target={target_id} player_pos=({:.1},{:.1}) \
         world_blocks={} overlay_blocks={} gap_below_feet={gap_desc} \
         body_overlaps[world={overlapping_world}, overlay={overlapping_overlay}]",
        pos.x,
        pos.y,
        world.blocks.len(),
        feature_overlay.blocks.len(),
    );
}

/// Probe straight down from the player's feet for the nearest block
/// top (within 256 px). Returns `(distance, source)` where `source` is
/// `"world"`, `"overlay"`, or `"both"`. `None` means nothing — the
/// player is over a pit (real bug) or `to_room_set()` / overlay
/// rebuild hasn't materialised the floor yet (the race we're hunting).
fn ground_gap_below_feet(
    feet_y: f32,
    body: &ae::Aabb,
    world: &ae::World,
    feature_overlay: &crate::features::FeatureEcsWorldOverlay,
) -> Option<(f32, &'static str)> {
    const MAX_PROBE_PX: f32 = 256.0;
    let probe = |blocks: &[ae::Block]| {
        let mut best: Option<f32> = None;
        for block in blocks {
            // X must overlap the player body.
            if block.aabb.right() <= body.left() || block.aabb.left() >= body.right() {
                continue;
            }
            // Only consider blocks whose top is below feet.
            let top = block.aabb.top();
            if top < feet_y {
                continue;
            }
            let gap = top - feet_y;
            if gap > MAX_PROBE_PX {
                continue;
            }
            best = Some(best.map_or(gap, |b| b.min(gap)));
        }
        best
    };
    let world_gap = probe(&world.blocks);
    let overlay_gap = probe(&feature_overlay.blocks);
    match (world_gap, overlay_gap) {
        (Some(a), Some(b)) if (a - b).abs() < 0.5 => Some((a.min(b), "both")),
        (Some(a), Some(b)) if a <= b => Some((a, "world")),
        (Some(_), Some(b)) => Some((b, "overlay")),
        (Some(a), None) => Some((a, "world")),
        (None, Some(b)) => Some((b, "overlay")),
        (None, None) => None,
    }
}

pub(super) fn handle_player_events(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &ae::PlayerClustersMut<'_>,
    combat: &mut crate::player::PlayerCombatState,
    blink_cam: &mut crate::player::PlayerBlinkCameraState,
    events: ae::FrameEvents,
    was_grounded: Option<bool>,
) {
    let pos = clusters.kinematics.pos;
    let facing = clusters.kinematics.facing;
    let size = clusters.kinematics.size;
    let on_ground = clusters.ground.on_ground;
    for op in &events.operations {
        match op {
            ae::MovementOp::Jump | ae::MovementOp::WallJump => {
                sfx.write(SfxMessage::Jump { pos });
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::DoubleJump => {
                sfx.write(SfxMessage::DoubleJump { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 14,
                    speed: 210.0,
                    color: [0.70, 1.0, 0.86, 0.82],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Dash | ae::MovementOp::DoubleDash => {
                sfx.write(SfxMessage::Dash { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 10,
                    speed: 330.0,
                    color: [1.0, 0.86, 0.38, 0.90],
                    kind: ParticleKind::Spark,
                });
            }
            ae::MovementOp::DodgeRoll => {
                sfx.write(SfxMessage::Dash { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 8,
                    speed: 240.0,
                    color: [0.60, 1.0, 0.70, 0.80],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Blink | ae::MovementOp::PrecisionBlink => {
                // Blink visuals use the explicit `events.blinks` endpoint data below.
            }
            ae::MovementOp::FlyToggle => {
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 12,
                    speed: 180.0,
                    color: [0.45, 0.82, 1.0, 0.72],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Pogo | ae::MovementOp::Rebound => {
                sfx.write(SfxMessage::Pogo { pos });
            }
            ae::MovementOp::SwimStroke => {
                sfx.write(SfxMessage::Jump { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 8,
                    speed: 150.0,
                    color: [0.50, 0.85, 1.0, 0.70],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::LedgeGrab => {
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::LedgeJump => {
                sfx.write(SfxMessage::Jump { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 8,
                    speed: 180.0,
                    color: [0.70, 1.0, 0.86, 0.82],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::LedgeRoll => {
                // Reuse the dash sfx — the ledge roll IS a dodge-roll
                // semantically (invuln rolling motion). Adds a small
                // dust burst at the platform lip for visual feedback.
                sfx.write(SfxMessage::Dash { pos });
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::LedgeGetupAttack => {
                // The engine pairs this op with MovementOp::Slash on
                // the same frame, so the slash SFX/VFX (and the
                // attack hitbox) fire through the normal slash path.
                // Here we only add the lift-up dust so the swing
                // reads as "coming off the ledge," not "in mid-air."
                // TODO: when a dedicated getup-attack sprite lands,
                // route a distinct VFX/SFX here too.
                vfx.write(VfxMessage::Dust { pos, facing });
            }
            ae::MovementOp::ShieldUp => {
                // Reuse the quick blink tone as a placeholder until a
                // dedicated Shield SoundCue is added to the sfxbank.
                sfx.write(SfxMessage::Blink {
                    pos,
                    precision: false,
                });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 12,
                    speed: 120.0,
                    color: [0.50, 0.80, 1.0, 0.70],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::LedgeClimbStart
            | ae::MovementOp::LedgeClimbFinish
            | ae::MovementOp::LedgeDrop
            | ae::MovementOp::WallCling
            | ae::MovementOp::WallClimb
            | ae::MovementOp::Slash => {}
            ae::MovementOp::Reset => {
                sfx.write(SfxMessage::Reset { pos });
            }
        }
    }
    for blink in &events.blinks {
        sfx.write(SfxMessage::Blink {
            pos: blink.from,
            precision: blink.precision,
        });
        blink_cam.blink_in_duration = crate::BLINK_IN_ANIM_TIME;
        blink_cam.blink_in_timer = blink_cam.blink_in_duration;
        blink_cam.blink_camera_from = blink.from;
        blink_cam.blink_camera_to = blink.to;
        vfx.write(VfxMessage::BlinkEffects {
            from: blink.from,
            to: blink.to,
            precision: blink.precision,
        });
    }
    if events.hazard || !events.operations.is_empty() {
        combat.flash_timer = 0.12;
    }
    if let Some(was_grounded) = was_grounded {
        if !was_grounded && on_ground {
            vfx.write(VfxMessage::Dust {
                pos: pos + ae::Vec2::new(0.0, size.y * 0.5),
                facing,
            });
        }
    }
}

pub(super) fn death_respawn_player(
    world: &ae::World,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    died: &mut MessageWriter<PlayerDiedMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    sim_state: &mut crate::SandboxSimState,
    safety: &mut crate::player::PlayerSafetyState,
    banner: &mut features::GameplayBanner,
    player_health: Option<&mut crate::player::PlayerHealth>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
    anim: &mut crate::player::PlayerAnimState,
    combat: &mut crate::player::PlayerCombatState,
) {
    let to = world.spawn;
    ae::reset_player_clusters(clusters, world.spawn);
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning,
    );
    clusters.mana.meter.refill_full();
    safety.last_safe_pos = world.spawn;
    sim_state.time_scale = 1.0;
    sim_state.room_transition_cooldown = 0.0;
    anim.reset();
    combat.reset();
    if let Some(health) = player_health {
        health.reset();
    }
    combat.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    combat.flash_timer = feel.reset_flash_time.max(0.35);
    banner.show("PLAYER DOWN: respawned at room start with full HP", 2.4);
    sfx.write(SfxMessage::Death { pos: from });
    vfx.write(VfxMessage::ResetEffects { from, to });
    died.write(PlayerDiedMessage { pos: from });
}

pub(super) fn handle_player_damage_events(
    world: &ae::World,
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    died: &mut MessageWriter<PlayerDiedMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    sim_state: &mut crate::SandboxSimState,
    safety: &mut crate::player::PlayerSafetyState,
    banner: &mut features::GameplayBanner,
    mut player_health: Option<&mut crate::player::PlayerHealth>,
    damage_events: &[features::PlayerDamageEvent],
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    difficulty_multiplier: f32,
    anim: &mut crate::player::PlayerAnimState,
    combat: &mut crate::player::PlayerCombatState,
) {
    let Some(mut damage) = damage_events.first().copied() else {
        return;
    };
    // Invincibility (debug toggle): drop the damage event entirely
    // before any state mutates so testing systems that consume HP
    // (boss phases, encounter pacing, music) can run uninterrupted.
    if clusters.offense.invincible {
        return;
    }
    // Difficulty / assist scaling. Easy halves incoming damage, hard
    // doubles it; the menu setting also exposes a fine-grained
    // gameplay damage multiplier. The minimum is one HP so a damage
    // event always lands somewhere.
    let scaled = ((damage.amount as f32) * difficulty_multiplier).round() as i32;
    damage.amount = scaled.max(1);
    let died_from_damage = if let Some(health) = player_health.as_deref_mut() {
        health.damage(damage.amount)
    } else {
        false
    };
    if died_from_damage {
        death_respawn_player(
            world,
            sfx,
            vfx,
            died,
            clusters,
            sim_state,
            safety,
            banner,
            player_health,
            tuning,
            feel,
            damage.impact_pos,
            anim,
            combat,
        );
        return;
    }
    match damage.mode {
        features::PlayerDamageMode::SafeRespawn => {
            safe_respawn_player(
                sfx,
                vfx,
                clusters,
                sim_state,
                safety,
                combat,
                tuning,
                feel,
                damage.impact_pos,
            );
        }
        features::PlayerDamageMode::Knockback => {
            apply_player_knockback(sfx, vfx, clusters, combat, tuning, feel, damage);
        }
    }
}

pub(super) fn safe_respawn_player(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    sim_state: &mut crate::SandboxSimState,
    safety: &crate::player::PlayerSafetyState,
    combat: &mut crate::player::PlayerCombatState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
) {
    let to = safety.last_safe_pos;
    ae::reset_player_clusters(clusters, to);
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning,
    );
    combat.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    combat.hitstun_timer = 0.0;
    combat.hitstop_timer = 0.0;
    combat.flash_timer = feel.reset_flash_time;
    sim_state.time_scale = 1.0;
    sfx.write(SfxMessage::Reset { pos: to });
    vfx.write(VfxMessage::ResetEffects { from, to });
}

pub(super) fn apply_player_knockback(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    clusters: &mut ae::PlayerClustersMut<'_>,
    combat: &mut crate::player::PlayerCombatState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    damage: features::PlayerDamageEvent,
) {
    let _source_pos_for_future_directional_rules = damage.source_pos;
    let boss_hit = matches!(
        damage.source,
        features::PlayerDamageSource::BossBody | features::PlayerDamageSource::BossAttack
    );
    let dir = if damage.knockback_dir.abs() <= 0.001 {
        -clusters.kinematics.facing
    } else {
        damage.knockback_dir.signum()
    };
    let strength = damage.strength.max(0.0);
    let knock_x = if boss_hit {
        feel.boss_knockback_x
    } else {
        feel.enemy_knockback_x
    };
    let knock_y = if boss_hit {
        feel.boss_knockback_y
    } else {
        feel.enemy_knockback_y
    };
    clusters.kinematics.vel.x = dir * knock_x * strength;
    clusters.kinematics.vel.y = -knock_y * strength;
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning,
    );
    combat.hitstun_timer = if boss_hit {
        feel.boss_hitstun_time
    } else {
        feel.enemy_hitstun_time
    } * strength.max(0.35);
    combat.damage_invuln_timer = feel.knockback_invulnerability_time;
    combat.hitstop_timer = feel.player_damage_hitstop_time;
    combat.flash_timer = 0.20;
    sfx.write(SfxMessage::Hit {
        pos: damage.impact_pos,
    });
    vfx.write(VfxMessage::Impact {
        pos: damage.impact_pos,
    });
}

/// Build the engine's `InputState` purely from `ActorControl` —
/// the player's brain output is the single source of truth for
/// every input verb the simulation consumes. The polarity flip is
/// now complete: raw `ControlFrame` is no longer consulted inside
/// the player simulation phases.
///
/// `drop_through_pressed` is derived from the standard `axis_y +
/// jump_pressed` gesture (same logic that lived in
/// `ControlFrame::engine_input` pre-migration), so consumers don't
/// have to special-case it.
///
/// The hitstun gate is applied to the FINAL `InputState` so every
/// verb is zeroed uniformly.
pub(super) fn engine_input_from_actor_control(
    actor: crate::actor_control::ActorControlFrame,
    feel: SandboxFeelTuning,
    hitstun_timer: f32,
    control_dt: f32,
) -> ae::InputState {
    // Same drop-through gesture the legacy `ControlFrame::engine_input`
    // computed (down held + jump just-pressed). Lives here because
    // it's a gesture, not a primitive verb.
    let drop_through_pressed = actor.desired_vel.y > 0.35 && actor.jump_pressed;
    let mut input = ae::InputState {
        axis_x: actor.desired_vel.x,
        axis_y: actor.desired_vel.y,
        jump_pressed: actor.jump_pressed,
        jump_held: actor.jump_held,
        jump_released: actor.jump_released,
        dash_pressed: actor.dash_pressed,
        fly_toggle_pressed: actor.fly_toggle_pressed,
        blink_pressed: actor.blink_pressed,
        blink_held: actor.blink_held,
        blink_released: actor.blink_released,
        fast_fall_pressed: actor.fast_fall_pressed,
        drop_through_pressed,
        attack_pressed: actor.melee_pressed,
        pogo_pressed: actor.pogo_pressed,
        interact_pressed: actor.interact_pressed,
        reset_pressed: false,
        shield_held: actor.shield_held,
        control_dt,
    };
    if hitstun_timer > 0.0 {
        let scale = feel.hitstun_control_scale.clamp(0.0, 1.0);
        input.axis_x *= scale;
        input.axis_y *= scale;
        input.jump_pressed = false;
        input.dash_pressed = false;
        input.fast_fall_pressed = false;
        input.blink_pressed = false;
        input.blink_held = false;
        input.blink_released = false;
        input.attack_pressed = false;
        input.pogo_pressed = false;
        input.fly_toggle_pressed = false;
        input.interact_pressed = false;
    }
    input
}

pub(super) fn start_attack(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    player: &mut ae::Player,
    attack: &mut Option<crate::PlayerAttackState>,
    anim: &mut crate::player::PlayerAnimState,
    controls: ControlFrame,
) {
    if !player.abilities.attack || attack.is_some() {
        return;
    }
    let intent = crate::combat::resolve_attack_intent(
        player,
        controls.axis_x,
        controls.axis_y,
        controls.pogo_pressed,
    );
    let spec = crate::combat::attack_spec(player, intent);

    // Directional attacks get small self-motion so the hitbox feels connected
    // to the controller. Keep these impulses modest; the engine control path
    // still owns the canonical slash/pogo op + recoil bookkeeping.
    player.vel += spec.self_impulse;
    if matches!(intent, crate::combat::AttackIntent::AirUp | crate::combat::AttackIntent::Up) && player.vel.y > -40.0 {
        player.vel.y = -40.0;
    }
    // Force downward commitment ONLY for the aerial down spike. The
    // grounded `Down` is now a kneeling forward poke (Marth-style
    // down-tilt) — it's rooted to the floor, so injecting downward
    // velocity here would punch the player into the ground / through
    // one-way platforms and make the attack feel like a glitched
    // pogo. AirDown still wants the commit, but not when the control
    // phase already applied an upward pogo bounce earlier this frame.
    // That same-frame ordering matters for 1hp breakable pogo orbs:
    // the bounce is real even when the orb shatters immediately, so
    // slash startup must not overwrite the negative Y velocity.
    if !controls.pogo_pressed
        && intent == crate::combat::AttackIntent::AirDown
        && player.vel.y >= 0.0
        && player.vel.y < 80.0
    {
        player.vel.y = 80.0;
    }

    let player_pos = player.pos;
    sfx.write(SfxMessage::Slash { pos: player_pos });
    anim.slash_anim_timer = spec.total_seconds().max(0.20);
    *attack = Some(crate::PlayerAttackState::new(spec));
    vfx.write(VfxMessage::SlashPreview {
        hitbox: crate::combat::attack_hitbox(player, spec),
    });
}

pub(super) fn advance_attack(
    sfx: &mut MessageWriter<SfxMessage>,
    vfx: &mut MessageWriter<VfxMessage>,
    world: &ae::World,
    moving_platforms: &[crate::world::platforms::MovingPlatformState],
    player: &mut ae::Player,
    attack: &mut Option<crate::PlayerAttackState>,
    anim: &mut crate::player::PlayerAnimState,
    combat: &mut crate::player::PlayerCombatState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    damage_events: &mut MessageWriter<features::DamageEvent>,
    pogo_bounces: &mut MessageWriter<features::PogoBounceEvent>,
) {
    let Some(mut attack_state) = attack.take() else {
        return;
    };

    attack_state.elapsed += frame_dt.max(0.0);
    let Some(phase) = attack_state.phase() else {
        anim.slash_anim_timer = 0.0;
        return;
    };

    if phase == crate::combat::AttackPhase::Active {
        let attack = crate::combat::attack_hitbox(player, attack_state.spec);
        let first_active_frame = !attack_state.active_started;
        if first_active_frame {
            attack_state.active_started = true;
            vfx.write(VfxMessage::SlashPreview { hitbox: attack });
        }

        let player_pos = player.pos;
        let mut pogo_landed = false;
        if player.abilities.pogo && attack_state.spec.can_pogo && !attack_state.pogo_applied {
            let attack_world =
                features::world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);
            if let Some(orb_aabb) = attack_world.blocks.iter().find_map(|block| {
                let valid_target = matches!(
                    block.kind,
                    ae::BlockKind::PogoOrb
                        | ae::BlockKind::Solid
                        | ae::BlockKind::OneWay
                        | ae::BlockKind::BlinkWall { .. }
                        | ae::BlockKind::Rebound { .. }
                );
                (valid_target && attack.strict_intersects(block.aabb)).then_some(block.aabb)
            }) {
                player.vel.y = -tuning.pogo_speed;
                player.refresh_movement_resources(tuning);
                player.on_ground = false;
                attack_state.pogo_applied = true;
                pogo_landed = true;
                sfx.write(SfxMessage::Pogo { pos: player_pos });
                pogo_bounces.write(features::PogoBounceEvent::new(orb_aabb, 1));
            }
        }
        let slash_damage = player.damage_multiplier.max(1);
        let knock_x = if attack_state.spec.knockback.x.abs() > 0.0 {
            attack_state.spec.knockback.x
        } else {
            player.facing * 300.0
        };
        if first_active_frame {
            damage_events.write(features::DamageEvent {
                volume: attack,
                damage: slash_damage,
                source: features::DamageSource::PlayerSlash { knock_x },
                ignored_targets: attack_state.hit_targets.clone(),
            });
        }
        // Damage is resolved by the ECS damage queue after the player tick.
        // Keep this phase responsible only for spawning the one-frame hitbox
        // and for immediate pogo/world-contact feedback.
        let landed = false;
        let killed = false;

        if landed || pogo_landed {
            if landed {
                sfx.write(SfxMessage::Hit { pos: player_pos });
            }
            combat.hitstop_timer = feel.attack_hitstop_time;
            combat.flash_timer = 0.16;
        }
        if killed {
            sfx.write(SfxMessage::Death { pos: player_pos });
        }
        if landed
            && player.abilities.pogo
            && attack_state.spec.can_pogo
            && !attack_state.pogo_applied
        {
            player.vel.y = -tuning.pogo_speed;
            player.refresh_movement_resources(tuning);
            attack_state.pogo_applied = true;
            sfx.write(SfxMessage::Pogo { pos: player_pos });
        }
    }

    if attack_state.done() {
        anim.slash_anim_timer = 0.0;
    } else {
        *attack = Some(attack_state);
    }
}
