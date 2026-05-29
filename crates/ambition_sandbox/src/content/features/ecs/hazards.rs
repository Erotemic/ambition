//! Hazard tick: patrol motion, contact damage, and the impact SFX/VFX
//! published to the presentation/audio buses.

use super::*;
use crate::content::features::util::hazard_sfx_id;

/// Tick ECS-authored hazards and publish player damage through Bevy messages.
pub fn update_ecs_hazards(
    world_time: Res<WorldTime>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::presentation::fx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut hit_events: MessageWriter<HitEvent>,
    player: Query<
        (
            Entity,
            &crate::player::PlayerKinematics,
            &crate::player::PlayerOffense,
            &crate::player::PlayerDodgeState,
            &crate::player::PlayerShieldState,
            &crate::player::PlayerCombatState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    mut hazards: Query<
        (&FeatureName, &mut FeatureAabb, &mut HazardFeature),
        With<FeatureSimEntity>,
    >,
) {
    // Sim clock: patrolling damage volumes must slow in bullet-time
    // so the player can route around them. ADR 0010.
    let dt = world_time.sim_dt();
    if player.is_empty() {
        // No players yet (pre-spawn); tick hazard motion but skip the
        // damage check so the patrol path still advances.
        for (_name, mut aabb, mut feature) in &mut hazards {
            let hazard = &mut feature.hazard;
            hazard.update(dt);
            aabb.center = hazard.pos;
            aabb.half_size = hazard.size * 0.5;
        }
        return;
    }
    for (_name, mut aabb, mut feature) in &mut hazards {
        let hazard = &mut feature.hazard;
        hazard.update(dt);
        aabb.center = hazard.pos;
        aabb.half_size = hazard.size * 0.5;
        if !hazard.active() {
            continue;
        }
        // Iterate every player so each overlapping player takes damage
        // independently — a future co-op build wants hazards to bite
        // every player in the volume, not implicitly the primary one.
        // OVERNIGHT-TODO #17.8 (B-bucket iterate-all-players for
        // hazard hits). Single-player behavior preserved because the
        // iterator has exactly one entity today.
        for (player_entity, kin, offense, dodge, shield, combat) in &player {
            let dodge_rolling = dodge.roll_timer > 0.0;
            let player_vulnerable = !offense.invincible
                && !dodge_rolling
                && !shield.parrying()
                && combat.vulnerable();
            if !player_vulnerable || !hazard.aabb().strict_intersects(kin.aabb()) {
                continue;
            }
            let pos = kin.pos;
            let knockback_dir = (pos.x - hazard.pos.x).signum();
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
            sfx.write(crate::audio::SfxMessage::Play {
                id: hazard_sfx_id(&hazard.name),
                pos,
            });
            hit_events.write(HitEvent {
                volume: hazard.aabb(),
                damage: hazard.volume.damage.amount.max(1),
                source: HitSource::Hazard,
                // Hazards iterate every overlapping player; tag the
                // event with the player who actually overlapped so
                // the reader lands the hit on the right one.
                target: HitTarget::Player(player_entity),
                mode: hazard.mode,
                knockback: Some(HitKnockback {
                    dir: knockback_dir,
                    strength: 1.0,
                    source_pos: hazard.pos,
                    impact_pos: pos,
                }),
                ignored_targets: Vec::new(),
            });
        }
    }
}
