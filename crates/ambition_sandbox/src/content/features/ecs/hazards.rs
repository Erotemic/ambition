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
    mut player_damage: MessageWriter<PlayerDamageEvent>,
    player: Query<
        (
            &crate::player::PlayerBody,
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
        for (pb, combat) in &player {
            let player_vulnerable =
                !pb.invincible && !pb.dodge_rolling && !pb.parrying && combat.vulnerable();
            if !player_vulnerable || !hazard.aabb().strict_intersects(pb.aabb()) {
                continue;
            }
            let pos = pb.pos;
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
            player_damage.write(PlayerDamageEvent {
                mode: hazard.mode,
                source: PlayerDamageSource::Hazard,
                source_pos: hazard.pos,
                impact_pos: pos,
                knockback_dir,
                strength: 1.0,
                amount: hazard.volume.damage.amount.max(1),
            });
        }
    }
}
