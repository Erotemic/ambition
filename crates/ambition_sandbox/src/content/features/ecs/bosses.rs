//! Boss tick: encounter-phase forwarding, sandbox-aware collision, and
//! contact-damage publication to the player.

use super::*;

/// Tick ECS-authored bosses and publish player damage through Bevy messages.
pub fn update_ecs_bosses(
    world_time: Res<WorldTime>,
    world: Res<crate::GameWorld>,
    platform_set: Res<crate::MovingPlatformSet>,
    feel_tuning: Res<crate::time::feel::SandboxFeelTuning>,
    overlay: Res<FeatureEcsWorldOverlay>,
    encounter_registry: Res<crate::boss_encounter::BossEncounterRegistry>,
    mut enemy_projectiles: ResMut<crate::enemy_projectile::EnemyProjectileState>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::presentation::fx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut player_damage: MessageWriter<PlayerDamageEvent>,
    // Bosses target the primary player today. Real multiplayer
    // boss AI (per-player targeting, agro lists, phase transitions
    // that respond to multiple players) is a deeper redesign than
    // the iterate-all-players pattern used by hazards / projectiles
    // — see OVERNIGHT-TODO #17.8 "Generalize enemy targeting." The
    // `PrimaryPlayerOnly` filter documents the targeting decision
    // at the query rather than leaving it as an implicit
    // `single()` semantic.
    player_query: Query<
        (&crate::player::PlayerBody, &crate::player::PlayerCombatState),
        crate::player::PrimaryPlayerOnly,
    >,
    mut bosses: Query<
        (
            &mut FeatureAabb,
            &mut BossFeature,
            &mut BossPatternTimer,
            &mut BossPhase,
            &super::super::components::ActorTarget,
            Option<&mut crate::brain::Brain>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    // Sim clock: bosses must slow with bullet-time (ADR 0010); a
    // boss locked-on to the player should not get free hits when
    // the player triggers bullet-time mid-pattern.
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    let Ok((pb, combat)) = player_query.single() else {
        return;
    };
    let player_body = pb.aabb();
    let player_vulnerable =
        !pb.invincible && !pb.dodge_rolling && !pb.parrying && combat.vulnerable();
    for (mut aabb, mut feature, mut pattern_timer, mut phase, target, mut brain) in &mut bosses {
        let boss = &mut feature.boss;
        let target_pos = target.pos;
        // Shadow brain tick (parallel-shape) — populates the boss's
        // BossPattern brain state alongside the BossRuntime tick.
        // No consumer reads the brain output today; daytime work
        // migrates the boss-pattern state machine into the brain's
        // internal sub-state and flips the consumer.
        if let Some(brain) = brain.as_deref_mut() {
            let snap = crate::brain::BrainSnapshot {
                actor_pos: boss.pos,
                actor_vel: ae::Vec2::ZERO,
                actor_facing: 1.0,
                actor_on_ground: false,
                alive: boss.alive,
                target_pos,
                target_alive: true,
                sim_time: 0.0,
                dt,
                attack_cooldown_remaining: 0.0,
                attack_windup_remaining: 0.0,
                attack_active_remaining: 0.0,
                attack_recover_remaining: 0.0,
                stun_remaining: 0.0,
                wall_contact: None,
                player_input: None,
            };
            let mut shadow = ae::ActorControlFrame::neutral();
            brain.tick(&snap, &mut shadow);
            let _ = shadow;
        }
        // Forward this boss's current encounter phase into the runtime
        // so `Scripted` attack patterns can pick the right phase
        // timeline. Look up by the semantic encounter id derived from
        // the boss display name (matches the lazy-register path in
        // `boss_encounter::systems::update_boss_encounters`). If the
        // encounter hasn't been registered yet, leave the previous
        // phase value alone — defaults to `Dormant` from `new()`.
        let encounter_id = crate::boss_encounter::encounter_id_from_name(&boss.name);
        if let Some(state) = encounter_registry.get(&encounter_id) {
            if boss.encounter_phase != state.phase {
                boss.encounter_phase = state.phase;
                // Reset the scripted cursor on phase change so each phase's
                // timeline begins at step 0 rather than mid-step.
                boss.scripted_step_index = 0;
                boss.scripted_step_elapsed = 0.0;
            }
        }
        let mut outputs = crate::features::BossTickOutputs::default();
        boss.update(
            &feature_world,
            target_pos,
            feel_tuning.feature_combat_tuning(),
            &mut outputs,
            dt,
        );
        // Flush any spawn requests the strike emitted this tick
        // (today: GNU-ton's apple rain). Same shape as the enemy
        // projectile flush in `update_ecs_actors`.
        for spawn in outputs.projectile_spawns {
            enemy_projectiles.spawn(spawn);
        }
        aabb.center = boss.pos;
        aabb.half_size = boss.render_size() * 0.5;
        pattern_timer.0 = boss.pattern_timer;
        *phase = BossPhase::from_alive(boss.alive);
        if player_vulnerable && boss.alive {
            if let Some(damage) = boss.player_damage(player_body) {
                let pos = damage.impact_pos;
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
                player_damage.write(damage);
            }
        }
    }
}
