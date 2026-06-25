//! The per-frame boss tick: encounter-phase sync + brain ticking + the main
//! ECS boss update (`update_ecs_bosses`).

use super::super::*;

use ambition_characters::brain::{
    action_set::ActionRequest, boss_pattern::tick_boss_pattern, ActorActionMessage, ActorControl,
    BossAttackState, BossPatternContext, Brain, StateMachineCfg,
};
use ambition_engine_core::AabbExt;
use crate::features::{boss_attack_damage, boss_special_for_profile, BossVolumeContext};
use bevy::prelude::MessageWriter;

/// Sync each boss's `encounter_phase` mirror from the entity-local
/// [`BossPhaseState`] copy (`BossStatus.encounter`). The mirror is a convenience
/// field the brain (`BossPatternContext`) reads; the `BossStatus.encounter`
/// phase machine — ticked by `update_boss_encounters` — is the source of truth.
/// Keyed per-entity by construction, so two of the same archetype sync
/// independent phases.
///
/// Runs before [`tick_boss_brains_system`] so the brain sees this frame's phase.
pub fn sync_boss_encounter_phase(
    mut bosses: Query<super::super::boss_clusters::BossClusterQueryData, With<FeatureSimEntity>>,
    mut last_logged: bevy::ecs::system::Local<
        std::collections::HashMap<String, crate::boss_encounter::BossEncounterPhase>,
    >,
) {
    for mut feature in &mut bosses {
        let boss_id = feature.config.id.clone();
        let behavior_id = feature.config.behavior.id.clone();
        // Phase comes from the entity-local copy, keyed per-entity by
        // construction, so two of the same archetype sync independent phases.
        let new_phase = feature.status.encounter.as_ref().map(|p| p.phase);
        // Log phase transitions per boss so we can see in the logs
        // when (or if) Dormant → Intro → Phase1 actually fires.
        let prev = last_logged.get(&boss_id).copied();
        if new_phase != prev {
            match new_phase {
                Some(phase) => {
                    bevy::log::info!(
                        target: "ambition::boss_encounter",
                        "sync_phase: boss={} (behavior.id={}) phase {:?} → {:?}",
                        boss_id,
                        behavior_id,
                        prev,
                        phase,
                    );
                    last_logged.insert(boss_id.clone(), phase);
                }
                None => {
                    bevy::log::warn!(
                        target: "ambition::boss_encounter",
                        "sync_phase: boss={} behavior.id={} has no entity-local encounter state (boss.encounter_phase stays {:?})",
                        boss_id,
                        behavior_id,
                        feature.status.encounter_phase,
                    );
                    last_logged.insert(boss_id.clone(), feature.status.encounter_phase);
                }
            }
        }
        if let Some(phase) = new_phase {
            feature.status.encounter_phase = phase;
        }
    }
}

/// Tick every boss's `BossPattern` brain: advance the cursor, emit
/// `ActorControlFrame` intent (movement + melee/special edges), and
/// update the `BossAttackState` component. `BossAttackState` is the
/// single source of truth for boss attack state — the volume / damage /
/// debug-overlay paths all query it.
pub fn tick_boss_brains_system(
    world_time: Res<WorldTime>,
    world: Res<crate::RoomGeometry>,
    platform_set: Res<crate::MovingPlatformSet>,
    overlay: Res<FeatureEcsWorldOverlay>,
    mut bosses: Query<
        (
            bevy::ecs::entity::Entity,
            super::super::boss_clusters::BossClusterRef,
            &mut Brain,
            &mut ActorControl,
            &mut BossAttackState,
            &super::super::super::components::ActorTarget,
        ),
        With<FeatureSimEntity>,
    >,
    mut action_messages: MessageWriter<ActorActionMessage>,
) {
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    for (entity, feature, mut brain, mut control, mut attack_state, target) in &mut bosses {
        let boss = feature.as_boss_ref();
        if !boss.status.alive {
            // Dead boss: zero out frame + attack state so any
            // downstream consumer sees a coherent "no intent".
            control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
            attack_state.clear();
            continue;
        }

        let StateMachineCfg::BossPattern { cfg, state } = pattern_brain_mut(&mut brain) else {
            // Boss has a non-BossPattern brain (test fixture). Leave
            // ActorControl + BossAttackState neutral so a future
            // brain swap doesn't leak stale intent.
            control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
            attack_state.clear();
            continue;
        };

        let front_wall_clearance = boss_front_wall_clearance(
            &feature_world,
            &boss,
            target.pos,
            cfg.macro_tuning.front_wall_standoff,
        );
        let ctx = BossPatternContext {
            encounter_phase: boss.status.encounter_phase,
            actor_pos: boss.kin.pos,
            target_pos: target.pos,
            world_size: world.0.size,
            front_wall_clearance,
            dt,
        };
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        tick_boss_pattern(cfg, state, &ctx, &mut frame, &mut attack_state);

        // Boss-side Special direct-write: the Gradient Sentinel has
        // four distinct specials (MemorizedVolley / PitTrap /
        // RotatingCross / MinionCascade) which doesn't fit
        // `ActionSet`'s single special slot. Rather than grow the
        // ActionSet schema or the ActorControlFrame, the boss tick
        // writes `ActorActionMessage::Special { spec }` directly
        // using `boss_special_for_profile` to look up the spec from
        // the live `BossAttackState.active_profile`. The boss's
        // `ActionSet.special` is set to `None` for multi-special
        // bosses (see `spawn_boss`) so the generic
        // `emit_brain_action_messages` resolver doesn't fire a
        // duplicate. GNU-ton's apple rain takes the same path so all
        // boss specials share one wiring.
        if frame.special_pressed {
            if let Some(profile) = attack_state.active_profile.as_ref() {
                if let Some(spec) = boss_special_for_profile(profile) {
                    action_messages.write(ActorActionMessage {
                        actor: entity,
                        request: ActionRequest::Special { spec },
                    });
                }
            }
        }
        control.0 = frame;
    }
}

pub(crate) fn boss_front_wall_clearance(
    world: &ae::World,
    boss: &super::super::boss_clusters::BossRef<'_>,
    target_pos: ae::Vec2,
    standoff: f32,
) -> Option<f32> {
    if standoff <= 0.0 {
        return None;
    }
    let dx = target_pos.x - boss.kin.pos.x;
    if dx.abs() <= 1.0 {
        return None;
    }
    let dir_x = dx.signum();
    let probe_distance = dx.abs().max(standoff + 1.0).min(1_024.0);
    let body = boss.aabb();
    horizontal_front_wall_clearance(world, body, dir_x, probe_distance)
}

pub(crate) fn horizontal_front_wall_clearance(
    world: &ae::World,
    body: ae::Aabb,
    dir_x: f32,
    probe_distance: f32,
) -> Option<f32> {
    if dir_x.abs() <= f32::EPSILON || probe_distance <= 0.0 {
        return None;
    }
    let dir_x = dir_x.signum();
    // Probe the vertical lane the boss body would actually sweep through.
    // Use only a small skin instead of a large percentage inset: low side
    // walls should still stop the behemoth, but a floor tile that merely
    // touches the boss's feet (or overlaps by a pixel due to integration
    // tolerance) must not be misclassified as a front wall.
    let vertical_skin = 4.0_f32.min(body.height() * 0.10);
    let lane_top = body.top() + vertical_skin;
    let lane_bottom = body.bottom() - vertical_skin;
    let (lane_top, lane_bottom) = if lane_top < lane_bottom {
        (lane_top, lane_bottom)
    } else {
        let center_y = body.center().y;
        (
            center_y - body.height() * 0.25,
            center_y + body.height() * 0.25,
        )
    };

    let mut best: Option<f32> = None;
    for block in &world.blocks {
        if !matches!(
            block.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
        ) {
            continue;
        }
        let vertical_overlap =
            lane_bottom.min(block.aabb.bottom()) - lane_top.max(block.aabb.top());
        if vertical_overlap <= 1.0 {
            continue;
        }
        let clearance = if dir_x > 0.0 {
            block.aabb.left() - body.right()
        } else {
            body.left() - block.aabb.right()
        };
        if clearance < -1.0 || clearance > probe_distance {
            continue;
        }
        let clearance = clearance.max(0.0);
        if best.is_none_or(|b| clearance < b) {
            best = Some(clearance);
        }
    }
    best
}

/// Helper: dig out the `&mut StateMachineCfg` from a `Brain`.
/// Bosses never spawn with `Brain::Player`; the `unreachable!` arm
/// is a safety net for that invariant.
fn pattern_brain_mut(brain: &mut Brain) -> &mut StateMachineCfg {
    match brain {
        Brain::StateMachine(cfg) => cfg,
        Brain::Player(_) => unreachable!("Boss entities are never spawned with Brain::Player"),
    }
}

/// Integrate ECS-authored bosses + publish damage. The brain
/// (`tick_boss_brains_system`) owns intent and has already written
/// `ActorControl` + `BossAttackState` by the time this system runs.
///
/// This system:
/// 1. Integrates the boss body using `ActorControl::0.desired_vel`.
/// 2. Syncs presentation mirrors (`CenteredAabb`, `BossPatternTimer`,
///    `BossPhase`).
/// 3. Publishes attack + body-contact damage via the pure
///    `boss_attack_damage` helper, which reads `BossAttackState`
///    directly (no runtime mirror fields involved).
pub fn update_ecs_bosses(
    world_time: Res<WorldTime>,
    world: Res<crate::RoomGeometry>,
    platform_set: Res<crate::MovingPlatformSet>,
    overlay: Res<FeatureEcsWorldOverlay>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut hit_events: MessageWriter<HitEvent>,
    // Per-boss target via `ActorTarget` (populated by `select_actor_targets`):
    // read each boss's targeted player by Entity from the all-players query.
    // Single-player behavior is preserved because there's only one player today;
    // real multiplayer boss AI (per-player agro lists, phase transitions that
    // respond to multiple players) is a deeper redesign left for later.
    player_query: Query<
        (
            &crate::player::BodyKinematics,
            &crate::player::PlayerOffense,
            &crate::player::PlayerDodgeState,
            &crate::player::PlayerShieldState,
            &crate::player::PlayerCombatState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    mut bosses: Query<
        (
            Entity,
            &mut CenteredAabb,
            super::super::boss_clusters::BossClusterQueryData,
            &mut BossPatternTimer,
            &mut BossDeathAnimation,
            &mut BossPhase,
            &ActorControl,
            &BossAttackState,
            &Brain,
            &super::super::super::components::ActorTarget,
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        // The player carries the unified `BodyKinematics`, and `player_query`
        // above reads it; exclude the player here so this `&mut BodyKinematics`
        // boss query is provably disjoint from it.
        (With<FeatureSimEntity>, Without<crate::player::PlayerEntity>),
    >,
    // Per-position gravity, so a grounded boss's footprint AABB orients to its
    // reference frame (matches the rotated sprite + the actor/player footprints).
    gravity: crate::physics::GravityCtx,
) {
    // Sim clock: bosses must slow with bullet-time (ADR 0010); a
    // boss locked-on to the player should not get free hits when
    // the player triggers bullet-time mid-pattern.
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    for (
        boss_entity,
        mut aabb,
        mut feature,
        mut pattern_timer,
        mut death_anim,
        mut phase,
        control,
        attack_state,
        brain,
        actor_target,
        animation_frame,
    ) in &mut bosses
    {
        // Resolve this boss's targeted player. If the target's
        // entity has despawned or no players exist, skip the body-
        // contact check — body still integrates so the boss keeps
        // animating its pattern. `target_entity` is threaded onto
        // the emitted `HitEvent::target` so the player-side reader
        // lands boss-attack damage on this specific player.
        let target_entity = actor_target.entity;
        let target_player = target_entity.and_then(|e| player_query.get(e).ok());
        // Integration: take the brain-emitted desired_vel and let
        // `step_kinematic` translate it into a collision-resolved
        // position change. The brain decided what we want; the
        // runtime decides what's actually possible.
        if control.0.facing.abs() > 0.001 {
            feature.kin.facing = control.0.facing.signum();
        }
        feature
            .as_boss_mut()
            .integrate_body(&feature_world, control.0.velocity_target, dt);
        aabb.center = feature.kin.pos;
        // Orient the footprint to the boss's reference frame so the box matches
        // the gravity-righted sprite. `to_world_half` swaps width<->height only
        // under sideways gravity / a wall — vertical gravity is unchanged, so
        // replay stays byte-identical.
        let boss_frame =
            ambition_engine_core::AccelerationFrame::new(gravity.dir_at(feature.kin.pos));
        aabb.half_size = boss_frame.to_world_half(feature.as_boss_ref().render_size() * 0.5);
        // Mirror the brain's pattern_timer (now living in
        // `BossPatternState`) into the presentation-side
        // `BossPatternTimer` component for sprite-animation
        // consumers. Defaults to 0 when the boss has a non-BossPattern
        // brain (test fixtures).
        pattern_timer.0 = match brain {
            Brain::StateMachine(StateMachineCfg::BossPattern { state, .. }) => state.pattern_timer,
            _ => 0.0,
        };
        if feature.status.alive {
            death_anim.clear();
        } else if phase.is_active() && death_anim.remaining_s <= 0.0 {
            death_anim.start();
        } else {
            death_anim.tick(dt);
        }
        *phase = BossPhase::from_alive(feature.status.alive);
        let (Some(target_entity), Some((kin, offense, dodge, shield, combat))) =
            (target_entity, target_player)
        else {
            continue;
        };
        let player_body = kin.aabb();
        let dodge_rolling = dodge.roll_timer > 0.0;
        let player_vulnerable =
            !offense.invincible && !dodge_rolling && !shield.parrying() && combat.vulnerable();
        if player_vulnerable && feature.status.alive {
            let ctx = BossVolumeContext::from_ref(feature.as_boss_ref(), attack_state)
                .with_animation_frame(animation_frame);
            if let Some(damage) = boss_attack_damage(&ctx, boss_entity, target_entity, player_body) {
                let pos = damage
                    .knockback
                    .as_ref()
                    .map(|k| k.impact_pos)
                    .unwrap_or_else(|| damage.volume.center());
                sfx.write(crate::audio::SfxMessage::Play {
                    id: ambition_sfx::ids::PLAYER_DAMAGE,
                    pos,
                });
                vfx.write(VfxMessage::Impact { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 14,
                    speed: 300.0,
                    color: [1.0, 0.34, 0.28, 0.88],
                    kind: ParticleKind::Shard,
                });
                debris.write(DebrisBurstMessage {
                    pos,
                    cue: PhysicsDebrisCue::Impact,
                });
                hit_events.write(damage);
            }
        }
    }
}
