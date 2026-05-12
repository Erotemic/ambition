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
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::update::*;
#[allow(unused_imports)]
use super::*;

pub(super) fn sandbox_dt(runtime: &SandboxRuntime, frame_dt: f32) -> f32 {
    if runtime.hitstop_timer > 0.0 {
        0.0
    } else {
        frame_dt * runtime.time_scale
    }
}

// `move_toward` has moved to `crate::lib` (`ambition_sandbox`) so the
// `SandboxRuntime` impl can use it; it is re-imported via the wildcard above.

pub(super) fn reset_sandbox(
    world: &ae::World,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    let reset_from = runtime.player.pos;
    runtime.reset(world, tuning);
    runtime.flash_timer = feel.reset_flash_time;
    let reset_to = runtime.player.pos;
    sfx.push(SfxMessage::Reset { pos: reset_to });
    vfx.push(VfxMessage::ResetEffects {
        from: reset_from,
        to: reset_to,
    });
}

pub(super) fn load_room(
    commands: &mut Commands,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    world: &mut GameWorld,
    room_set: &mut rooms::RoomSet,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    transition: rooms::RoomTransition,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    assets: Option<&crate::game_assets::GameAssets>,
) {
    let old_velocity = runtime.player.vel;
    let abilities = runtime.player.abilities;
    let fly_enabled = runtime.player.fly_enabled;
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
    let arrival = rooms::validated_spawn(&world.0, transition.arrival, runtime.player.size);
    runtime.player = ae::Player::new_with_abilities(arrival, abilities);
    runtime.player.refresh_movement_resources(tuning);
    runtime.player.fly_enabled = fly_enabled && runtime.player.abilities.fly;
    if edge_exit {
        runtime.player.vel = old_velocity;
    }
    runtime.flash_timer = if edge_exit {
        feel.edge_transition_flash
    } else {
        feel.door_transition_flash
    };
    runtime.hitstop_timer = 0.0;
    runtime.damage_invuln_timer = 0.0;
    runtime.hitstun_timer = 0.0;
    runtime.last_safe_player_pos = runtime.player.pos;
    runtime.time_scale = 1.0;
    runtime.down_tap_timer = 0.0;
    runtime.moving_platform = spec
        .moving_platform
        .unwrap_or_else(|| platforms::MovingPlatformState::time_reference(&world.0));
    runtime.features = features::FeatureRuntime::from_world(&world.0);
    runtime.dialogue.close();
    // This guard prevents immediate backtracking when arriving inside/near a
    // paired zone. It should not feel like frozen input, so keep it short and
    // rely on validated arrivals to do most of the safety work.
    runtime.room_transition_cooldown = if edge_exit {
        feel.edge_transition_cooldown
    } else {
        feel.door_transition_cooldown
    };
    runtime.preset_flash = 1.0;

    spawn_room_visuals(
        commands,
        &world.0,
        &spec.loading_zones,
        &spec.metadata,
        physics_settings,
        assets,
    );
    platforms::spawn_moving_platform(commands, &world.0, runtime.moving_platform);
    sfx.push(SfxMessage::Reset {
        pos: runtime.player.pos,
    });
    if edge_exit {
        // Edge exits should feel like contiguous room scrolling, not a death-like
        // teleport. Only show an arrival puff in the new room because `from` was
        // expressed in the previous room's coordinate space.
        vfx.push(VfxMessage::Burst {
            pos: runtime.player.pos,
            count: 18,
            speed: 260.0,
            color: [0.35, 0.95, 1.0, 0.75],
            kind: ParticleKind::Dust,
        });
    } else {
        // Door transitions are discrete interactions, so a teleport-like effect
        // is acceptable; use the destination for both endpoints to avoid mixing
        // coordinate systems from two rooms.
        vfx.push(VfxMessage::ResetEffects {
            from: runtime.player.pos,
            to: runtime.player.pos,
        });
    }
}

pub(super) fn handle_player_events(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    events: ae::FrameEvents,
    was_grounded: Option<bool>,
) {
    let pos = runtime.player.pos;
    for op in &events.operations {
        match op {
            ae::MovementOp::Jump | ae::MovementOp::WallJump => {
                sfx.push(SfxMessage::Jump { pos });
                vfx.push(VfxMessage::Dust {
                    pos: runtime.player.pos,
                    facing: runtime.player.facing,
                });
            }
            ae::MovementOp::DoubleJump => {
                sfx.push(SfxMessage::DoubleJump { pos });
                vfx.push(VfxMessage::Burst {
                    pos: runtime.player.pos,
                    count: 14,
                    speed: 210.0,
                    color: [0.70, 1.0, 0.86, 0.82],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Dash | ae::MovementOp::DoubleDash => {
                sfx.push(SfxMessage::Dash { pos });
                vfx.push(VfxMessage::Burst {
                    pos: runtime.player.pos,
                    count: 10,
                    speed: 330.0,
                    color: [1.0, 0.86, 0.38, 0.90],
                    kind: ParticleKind::Spark,
                });
            }
            ae::MovementOp::Blink | ae::MovementOp::PrecisionBlink => {
                // Blink visuals use the explicit `events.blinks` endpoint data below.
            }
            ae::MovementOp::FlyToggle => {
                vfx.push(VfxMessage::Burst {
                    pos: runtime.player.pos,
                    count: 12,
                    speed: 180.0,
                    color: [0.45, 0.82, 1.0, 0.72],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Pogo | ae::MovementOp::Rebound => {
                sfx.push(SfxMessage::Pogo { pos });
            }
            ae::MovementOp::WallCling | ae::MovementOp::WallClimb | ae::MovementOp::Slash => {}
            ae::MovementOp::Reset => {
                sfx.push(SfxMessage::Reset { pos });
            }
        }
    }
    for blink in &events.blinks {
        sfx.push(SfxMessage::Blink {
            pos: blink.from,
            precision: blink.precision,
        });
        vfx.push(VfxMessage::BlinkEffects {
            from: blink.from,
            to: blink.to,
            precision: blink.precision,
        });
    }
    if events.hazard || !events.operations.is_empty() {
        runtime.flash_timer = 0.12;
    }
    if let Some(was_grounded) = was_grounded {
        if !was_grounded && runtime.player.on_ground {
            vfx.push(VfxMessage::Dust {
                pos: runtime.player.pos + ae::Vec2::new(0.0, runtime.player.size.y * 0.5),
                facing: runtime.player.facing,
            });
        }
    }
}

pub(super) fn handle_feature_events(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    debris: &mut Vec<DebrisBurstMessage>,
    events: &features::FeatureEvents,
    player_pos: ae::Vec2,
) {
    if events.reset_player {
        sfx.push(SfxMessage::Reset { pos: player_pos });
    }
    for physics_burst in &events.physics_bursts {
        let cue = match physics_burst.cue {
            features::FeaturePhysicsCue::Breakable => physics::PhysicsDebrisCue::Breakable,
            features::FeaturePhysicsCue::EnemyRagdoll => physics::PhysicsDebrisCue::EnemyRagdoll,
            features::FeaturePhysicsCue::BossRagdoll => physics::PhysicsDebrisCue::BossRagdoll,
        };
        debris.push(DebrisBurstMessage {
            pos: physics_burst.pos,
            cue,
        });
    }
    for &pos in &events.impacts {
        vfx.push(VfxMessage::Impact { pos });
        vfx.push(VfxMessage::Burst {
            pos,
            count: 14,
            speed: 300.0,
            color: [1.0, 0.34, 0.28, 0.88],
            kind: ParticleKind::Shard,
        });
        debris.push(DebrisBurstMessage {
            pos,
            cue: physics::PhysicsDebrisCue::Impact,
        });
    }
    for &pos in &events.bursts {
        vfx.push(VfxMessage::Burst {
            pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
    }
    for &pos in &events.chests_opened {
        sfx.push(SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_TREASURE_CHEST_OPEN,
            pos,
        });
    }
    for (kind, pos) in &events.pickups_collected {
        let id = match kind {
            ae::PickupKind::Health { .. } => ambition_sfx::ids::WORLD_HEALTH_COLLECT,
            ae::PickupKind::Currency { .. } => ambition_sfx::ids::WORLD_COIN_PICKUP,
            // Ability / StoryFlag / Custom — fall back to the generic
            // pickup SFX until those gain dedicated sounds.
            _ => ambition_sfx::ids::WORLD_PICKUP_GENERIC,
        };
        sfx.push(SfxMessage::Play { id, pos: *pos });
    }
    for &pos in &events.breakables_destroyed {
        sfx.push(SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_CRATE_BREAK,
            pos,
        });
    }
    for (_payload, pos) in events.switch_activations() {
        sfx.push(SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_SWITCH_TOGGLE,
            pos,
        });
    }
    // Generic SFX-only effects queued by sim-side code via
    // `events.play_sfx(id, pos)`. Used for things that have no other
    // consumer (player damage, hazard contact, etc.); anything that
    // also drives VFX/persistence/quests stays on a typed event vec.
    for (id, pos) in events.sfx_plays() {
        sfx.push(SfxMessage::Play { id, pos });
    }
}

pub(super) fn handle_player_heal_events(
    runtime: &mut SandboxRuntime,
    events: &features::FeatureEvents,
) {
    if events.player_heal > 0 {
        runtime.player_health.heal(events.player_heal);
    }
}

pub(super) fn death_respawn_player(
    world: &ae::World,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    died: &mut Vec<PlayerDiedMessage>,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
) {
    let to = world.spawn;
    runtime.reset(world, tuning);
    runtime.player_health.reset();
    runtime.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    runtime.flash_timer = feel.reset_flash_time.max(0.35);
    runtime.features.banner = "PLAYER DOWN: respawned at room start with full HP".to_string();
    runtime.features.banner_timer = 2.4;
    sfx.push(SfxMessage::Death { pos: from });
    vfx.push(VfxMessage::ResetEffects { from, to });
    died.push(PlayerDiedMessage { pos: from });
}

pub(super) fn handle_player_damage_events(
    world: &ae::World,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    died: &mut Vec<PlayerDiedMessage>,
    runtime: &mut SandboxRuntime,
    events: &features::FeatureEvents,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    difficulty_multiplier: f32,
) {
    let Some(mut damage) = events.player_damage.first().copied() else {
        return;
    };
    // Invincibility (debug toggle): drop the damage event entirely
    // before any state mutates so testing systems that consume HP
    // (boss phases, encounter pacing, music) can run uninterrupted.
    if runtime.player.invincible {
        return;
    }
    // Difficulty / assist scaling. Easy halves incoming damage, hard
    // doubles it; the menu setting also exposes a fine-grained
    // gameplay damage multiplier. The minimum is one HP so a damage
    // event always lands somewhere.
    let scaled = ((damage.amount as f32) * difficulty_multiplier).round() as i32;
    damage.amount = scaled.max(1);
    if runtime.player_health.damage(damage.amount) {
        death_respawn_player(
            world,
            sfx,
            vfx,
            died,
            runtime,
            tuning,
            feel,
            damage.impact_pos,
        );
        return;
    }
    match damage.mode {
        features::PlayerDamageMode::SafeRespawn => {
            safe_respawn_player(sfx, vfx, runtime, tuning, feel, damage.impact_pos);
        }
        features::PlayerDamageMode::Knockback => {
            apply_player_knockback(sfx, vfx, runtime, tuning, feel, damage);
        }
    }
}

pub(super) fn safe_respawn_player(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
) {
    let to = runtime.last_safe_player_pos;
    runtime.player.reset_to(to);
    runtime.player.refresh_movement_resources(tuning);
    runtime.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    runtime.hitstun_timer = 0.0;
    runtime.hitstop_timer = 0.0;
    runtime.flash_timer = feel.reset_flash_time;
    runtime.time_scale = 1.0;
    sfx.push(SfxMessage::Reset { pos: to });
    vfx.push(VfxMessage::ResetEffects { from, to });
}

pub(super) fn apply_player_knockback(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    runtime: &mut SandboxRuntime,
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
        -runtime.player.facing
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
    runtime.player.vel.x = dir * knock_x * strength;
    runtime.player.vel.y = -knock_y * strength;
    runtime.player.refresh_movement_resources(tuning);
    runtime.hitstun_timer = if boss_hit {
        feel.boss_hitstun_time
    } else {
        feel.enemy_hitstun_time
    } * strength.max(0.35);
    runtime.damage_invuln_timer = feel.knockback_invulnerability_time;
    runtime.hitstop_timer = feel.player_damage_hitstop_time;
    runtime.flash_timer = 0.20;
    sfx.push(SfxMessage::Hit {
        pos: damage.impact_pos,
    });
    vfx.push(VfxMessage::Impact {
        pos: damage.impact_pos,
    });
}

pub(super) fn controls_for_hitstun(
    mut controls: ControlFrame,
    feel: SandboxFeelTuning,
    hitstun_timer: f32,
) -> ControlFrame {
    if hitstun_timer <= 0.0 {
        return controls;
    }
    let scale = feel.hitstun_control_scale.clamp(0.0, 1.0);
    controls.axis_x *= scale;
    controls.axis_y *= scale;
    controls.jump_pressed = false;
    controls.dash_pressed = false;
    controls.fast_fall_pressed = false;
    controls.blink_pressed = false;
    controls.blink_held = false;
    controls.blink_released = false;
    controls.attack_pressed = false;
    controls.pogo_pressed = false;
    controls.fly_toggle_pressed = false;
    controls.interact_pressed = false;
    controls
}

pub(super) fn process_attack(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    debris: &mut Vec<DebrisBurstMessage>,
    runtime: &mut SandboxRuntime,
    controls: ControlFrame,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    if !runtime.player.abilities.attack {
        return;
    }
    let player_pos = runtime.player.pos;
    sfx.push(SfxMessage::Slash { pos: player_pos });
    // Roughly the slash sheet's eight 75ms frames; the animation system
    // freezes on the last frame once `clip_held` is set, so this only
    // needs to cover the typical clip duration.
    runtime.slash_anim_timer = 0.60;
    let attack = ae::slash_hitbox(&runtime.player, controls.axis_y, controls.pogo_pressed);
    vfx.push(VfxMessage::SlashPreview { hitbox: attack });
    let mut landed = false;
    let mut killed = false;
    let player_facing = runtime.player.facing;
    let slash_damage = runtime.player.damage_multiplier.max(1);
    let feature_events =
        runtime
            .features
            .apply_player_attack(attack, slash_damage, player_facing * 300.0);
    landed |= !feature_events.impacts.is_empty();
    killed |= feature_events
        .messages
        .iter()
        .any(|message| message.contains("defeated"));
    handle_feature_events(sfx, vfx, debris, &feature_events, player_pos);

    if landed {
        sfx.push(SfxMessage::Hit { pos: player_pos });
        runtime.hitstop_timer = feel.attack_hitstop_time;
        runtime.flash_timer = 0.16;
    }
    if killed {
        sfx.push(SfxMessage::Death { pos: player_pos });
    }
    if landed && runtime.player.abilities.pogo && (controls.pogo_pressed || controls.axis_y > 0.25)
    {
        runtime.player.vel.y = -tuning.pogo_speed;
        runtime.player.refresh_movement_resources(tuning);
        sfx.push(SfxMessage::Pogo { pos: player_pos });
    }
}
