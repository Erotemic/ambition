use ambition_engine::AabbExt;

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

pub(super) fn sandbox_dt(hitstop_timer: f32, time_scale: f32, frame_dt: f32) -> f32 {
    if hitstop_timer > 0.0 {
        0.0
    } else {
        frame_dt * time_scale
    }
}


pub(super) fn reset_sandbox(
    world: &ae::World,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    player: &mut ae::Player,
    sim_state: &mut crate::SandboxSimState,
    attack: &mut Option<crate::PlayerAttackState>,
    anim: &mut crate::player::PlayerAnimState,
    combat: &mut crate::player::PlayerCombatState,
    interaction: &mut crate::player::PlayerInteractionState,
    blink_cam: &mut crate::player::PlayerBlinkCameraState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
) {
    let reset_from = player.pos;
    player.reset_to(world.spawn);
    player.refresh_movement_resources(tuning);
    player.mana.refill_full();
    sim_state.last_safe_player_pos = world.spawn;
    sim_state.time_scale = 1.0;
    sim_state.room_transition_cooldown = 0.0;
    *attack = None;
    anim.reset();
    combat.reset();
    combat.flash_timer = feel.reset_flash_time;
    interaction.reset();
    blink_cam.reset();
    let reset_to = player.pos;
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
    player: &mut ae::Player,
    dev_state: &mut crate::SandboxDevState,
    sim_state: &mut crate::SandboxSimState,
    moving_platforms: &mut Vec<crate::platforms::MovingPlatformState>,
    dialogue: &mut crate::dialog::DialogState,
    combat: &mut crate::player::PlayerCombatState,
    interaction: &mut crate::player::PlayerInteractionState,
    blink_cam: &mut crate::player::PlayerBlinkCameraState,
    world: &mut GameWorld,
    room_set: &mut rooms::RoomSet,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    transition: rooms::RoomTransition,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    assets: Option<&crate::game_assets::GameAssets>,
) {
    let old_velocity = player.vel;
    let abilities = player.abilities;
    let fly_enabled = player.fly_enabled;
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
    let arrival = rooms::validated_spawn(&world.0, transition.arrival, player.size);
    *player = ae::Player::new_with_abilities(arrival, abilities);
    player.refresh_movement_resources(tuning);
    player.fly_enabled = fly_enabled && player.abilities.fly;
    if edge_exit {
        player.vel = old_velocity;
    }
    blink_cam.blink_in_timer = 0.0;
    blink_cam.blink_camera_from = player.pos;
    blink_cam.blink_camera_to = player.pos;
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
    sim_state.last_safe_player_pos = player.pos;
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

    crate::rendering::spawn_parallax_layers(commands, &world.0, &spec.metadata, assets);
    spawn_room_visuals(
        commands,
        &world.0,
        &spec.loading_zones,
        physics_settings,
        assets,
    );
    platforms::spawn_moving_platforms(commands, &world.0, moving_platforms);
    sfx.push(SfxMessage::Reset { pos: player.pos });
    if edge_exit {
        // Edge exits should feel like contiguous room scrolling, not a death-like
        // teleport. Only show an arrival puff in the new room because `from` was
        // expressed in the previous room's coordinate space.
        vfx.push(VfxMessage::Burst {
            pos: player.pos,
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
            from: player.pos,
            to: player.pos,
        });
    }
}

pub(super) fn handle_player_events(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    player: &ae::Player,
    combat: &mut crate::player::PlayerCombatState,
    blink_cam: &mut crate::player::PlayerBlinkCameraState,
    events: ae::FrameEvents,
    was_grounded: Option<bool>,
) {
    let pos = player.pos;
    for op in &events.operations {
        match op {
            ae::MovementOp::Jump | ae::MovementOp::WallJump => {
                sfx.push(SfxMessage::Jump { pos });
                vfx.push(VfxMessage::Dust {
                    pos: player.pos,
                    facing: player.facing,
                });
            }
            ae::MovementOp::DoubleJump => {
                sfx.push(SfxMessage::DoubleJump { pos });
                vfx.push(VfxMessage::Burst {
                    pos: player.pos,
                    count: 14,
                    speed: 210.0,
                    color: [0.70, 1.0, 0.86, 0.82],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Dash | ae::MovementOp::DoubleDash => {
                sfx.push(SfxMessage::Dash { pos });
                vfx.push(VfxMessage::Burst {
                    pos: player.pos,
                    count: 10,
                    speed: 330.0,
                    color: [1.0, 0.86, 0.38, 0.90],
                    kind: ParticleKind::Spark,
                });
            }
            ae::MovementOp::DodgeRoll => {
                sfx.push(SfxMessage::Dash { pos });
                vfx.push(VfxMessage::Burst {
                    pos: player.pos,
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
                vfx.push(VfxMessage::Burst {
                    pos: player.pos,
                    count: 12,
                    speed: 180.0,
                    color: [0.45, 0.82, 1.0, 0.72],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::Pogo | ae::MovementOp::Rebound => {
                sfx.push(SfxMessage::Pogo { pos });
            }
            ae::MovementOp::SwimStroke => {
                sfx.push(SfxMessage::Jump { pos });
                vfx.push(VfxMessage::Burst {
                    pos,
                    count: 8,
                    speed: 150.0,
                    color: [0.50, 0.85, 1.0, 0.70],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::LedgeGrab => {
                vfx.push(VfxMessage::Dust {
                    pos: player.pos,
                    facing: player.facing,
                });
            }
            ae::MovementOp::LedgeJump => {
                sfx.push(SfxMessage::Jump { pos });
                vfx.push(VfxMessage::Burst {
                    pos: player.pos,
                    count: 8,
                    speed: 180.0,
                    color: [0.70, 1.0, 0.86, 0.82],
                    kind: ParticleKind::Dust,
                });
            }
            ae::MovementOp::ShieldUp => {
                // Reuse the quick blink tone as a placeholder until a
                // dedicated Shield SoundCue is added to the sfxbank.
                sfx.push(SfxMessage::Blink {
                    pos,
                    precision: false,
                });
                vfx.push(VfxMessage::Burst {
                    pos: player.pos,
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
                sfx.push(SfxMessage::Reset { pos });
            }
        }
    }
    for blink in &events.blinks {
        sfx.push(SfxMessage::Blink {
            pos: blink.from,
            precision: blink.precision,
        });
        blink_cam.blink_in_duration = crate::BLINK_IN_ANIM_TIME;
        blink_cam.blink_in_timer = blink_cam.blink_in_duration;
        blink_cam.blink_camera_from = blink.from;
        blink_cam.blink_camera_to = blink.to;
        vfx.push(VfxMessage::BlinkEffects {
            from: blink.from,
            to: blink.to,
            precision: blink.precision,
        });
    }
    if events.hazard || !events.operations.is_empty() {
        combat.flash_timer = 0.12;
    }
    if let Some(was_grounded) = was_grounded {
        if !was_grounded && player.on_ground {
            vfx.push(VfxMessage::Dust {
                pos: player.pos + ae::Vec2::new(0.0, player.size.y * 0.5),
                facing: player.facing,
            });
        }
    }
}

pub(super) fn death_respawn_player(
    world: &ae::World,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    died: &mut Vec<PlayerDiedMessage>,
    player: &mut ae::Player,
    sim_state: &mut crate::SandboxSimState,
    banner: &mut features::GameplayBanner,
    mut player_health: Option<&mut crate::player::PlayerHealth>,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
    anim: &mut crate::player::PlayerAnimState,
    combat: &mut crate::player::PlayerCombatState,
) {
    let to = world.spawn;
    player.reset_to(world.spawn);
    player.refresh_movement_resources(tuning);
    player.mana.refill_full();
    sim_state.last_safe_player_pos = world.spawn;
    sim_state.time_scale = 1.0;
    sim_state.room_transition_cooldown = 0.0;
    anim.reset();
    combat.reset();
    if let Some(health) = player_health.as_deref_mut() {
        health.reset();
    }
    combat.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    combat.flash_timer = feel.reset_flash_time.max(0.35);
    banner.show("PLAYER DOWN: respawned at room start with full HP", 2.4);
    sfx.push(SfxMessage::Death { pos: from });
    vfx.push(VfxMessage::ResetEffects { from, to });
    died.push(PlayerDiedMessage { pos: from });
}

pub(super) fn handle_player_damage_events(
    world: &ae::World,
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    died: &mut Vec<PlayerDiedMessage>,
    player: &mut ae::Player,
    sim_state: &mut crate::SandboxSimState,
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
    if player.invincible {
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
            player,
            sim_state,
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
            safe_respawn_player(sfx, vfx, player, sim_state, combat, tuning, feel, damage.impact_pos);
        }
        features::PlayerDamageMode::Knockback => {
            apply_player_knockback(sfx, vfx, player, combat, tuning, feel, damage);
        }
    }
}

pub(super) fn safe_respawn_player(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    player: &mut ae::Player,
    sim_state: &mut crate::SandboxSimState,
    combat: &mut crate::player::PlayerCombatState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    from: ae::Vec2,
) {
    let to = sim_state.last_safe_player_pos;
    player.reset_to(to);
    player.refresh_movement_resources(tuning);
    combat.damage_invuln_timer = feel.hazard_respawn_invulnerability_time;
    combat.hitstun_timer = 0.0;
    combat.hitstop_timer = 0.0;
    combat.flash_timer = feel.reset_flash_time;
    sim_state.time_scale = 1.0;
    sfx.push(SfxMessage::Reset { pos: to });
    vfx.push(VfxMessage::ResetEffects { from, to });
}

pub(super) fn apply_player_knockback(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    player: &mut ae::Player,
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
        -player.facing
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
    player.vel.x = dir * knock_x * strength;
    player.vel.y = -knock_y * strength;
    player.refresh_movement_resources(tuning);
    combat.hitstun_timer = if boss_hit {
        feel.boss_hitstun_time
    } else {
        feel.enemy_hitstun_time
    } * strength.max(0.35);
    combat.damage_invuln_timer = feel.knockback_invulnerability_time;
    combat.hitstop_timer = feel.player_damage_hitstop_time;
    combat.flash_timer = 0.20;
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

pub(super) fn start_attack(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    player: &mut ae::Player,
    attack: &mut Option<crate::PlayerAttackState>,
    anim: &mut crate::player::PlayerAnimState,
    controls: ControlFrame,
) {
    if !player.abilities.attack || attack.is_some() {
        return;
    }
    let intent = ae::resolve_attack_intent(
        player,
        controls.axis_x,
        controls.axis_y,
        controls.pogo_pressed,
    );
    let spec = ae::attack_spec(player, intent);

    // Directional attacks get small self-motion so the hitbox feels connected
    // to the controller. Keep these impulses modest; the engine control path
    // still owns the canonical slash/pogo op + recoil bookkeeping.
    player.vel += spec.self_impulse;
    if matches!(intent, ae::AttackIntent::AirUp | ae::AttackIntent::Up)
        && player.vel.y > -40.0
    {
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
        && intent == ae::AttackIntent::AirDown
        && player.vel.y >= 0.0
        && player.vel.y < 80.0
    {
        player.vel.y = 80.0;
    }

    let player_pos = player.pos;
    sfx.push(SfxMessage::Slash { pos: player_pos });
    anim.slash_anim_timer = spec.total_seconds().max(0.20);
    *attack = Some(crate::PlayerAttackState::new(spec));
    vfx.push(VfxMessage::SlashPreview {
        hitbox: ae::attack_hitbox(player, spec),
    });
}

pub(super) fn advance_attack(
    sfx: &mut Vec<SfxMessage>,
    vfx: &mut Vec<VfxMessage>,
    world: &ae::World,
    moving_platforms: &[crate::platforms::MovingPlatformState],
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

    if phase == ae::AttackPhase::Active {
        let attack = ae::attack_hitbox(player, attack_state.spec);
        let first_active_frame = !attack_state.active_started;
        if first_active_frame {
            attack_state.active_started = true;
            vfx.push(VfxMessage::SlashPreview { hitbox: attack });
        }

        let player_pos = player.pos;
        let mut pogo_landed = false;
        if player.abilities.pogo && attack_state.spec.can_pogo && !attack_state.pogo_applied {
            let attack_world = features::world_with_sandbox_solids(
                world,
                moving_platforms,
                feature_ecs_overlay,
            );
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
                sfx.push(SfxMessage::Pogo { pos: player_pos });
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
        // Damage is resolved by the ECS damage queue after `sandbox_update`.
        // Keep this phase responsible only for spawning the one-frame hitbox
        // and for immediate pogo/world-contact feedback.
        let landed = false;
        let killed = false;

        if landed || pogo_landed {
            if landed {
                sfx.push(SfxMessage::Hit { pos: player_pos });
            }
            combat.hitstop_timer = feel.attack_hitstop_time;
            combat.flash_timer = 0.16;
        }
        if killed {
            sfx.push(SfxMessage::Death { pos: player_pos });
        }
        if landed
            && player.abilities.pogo
            && attack_state.spec.can_pogo
            && !attack_state.pogo_applied
        {
            player.vel.y = -tuning.pogo_speed;
            player.refresh_movement_resources(tuning);
            attack_state.pogo_applied = true;
            sfx.push(SfxMessage::Pogo { pos: player_pos });
        }
    }

    if attack_state.done() {
        anim.slash_anim_timer = 0.0;
    } else {
        *attack = Some(attack_state);
    }
}
