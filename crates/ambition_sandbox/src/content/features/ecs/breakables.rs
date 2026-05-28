//! Per-frame tick for breakable feature entities: respawn countdown
//! and the stand-to-break collapse trigger.

use super::*;
use crate::content::features::util::player_is_standing_on;
use crate::content::features::BREAK_ON_STAND_SECONDS;

/// Tick ECS-owned breakable timers and stand-to-break triggers.
pub fn update_ecs_breakables(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    player_body_q: Query<&crate::player::PlayerKinematics, With<crate::player::PlayerEntity>>,
    mut banner: ResMut<GameplayBanner>,
    mut breakables: Query<
        (
            Entity,
            &FeatureName,
            &FeatureAabb,
            &mut BreakableFeature,
            Option<&mut RespawnTimer>,
            Option<&mut StandTimer>,
        ),
        With<FeatureSimEntity>,
    >,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
) {
    // Sim clock: breakable respawn / stand-to-break should freeze in
    // bullet-time alongside the player and enemies (ADR 0010).
    let dt = world_time.sim_dt();
    for (entity, name, aabb, mut feature, respawn_timer, stand_timer) in &mut breakables {
        if feature.broken() {
            if let Some(mut timer) = respawn_timer {
                timer.0 = (timer.0 - dt).max(0.0);
                if timer.0 <= 0.0 {
                    feature.breakable.state = ae::BreakableState::Intact;
                    feature.breakable.health.reset();
                    commands.entity(entity).remove::<RespawnTimer>();
                    banner.show(format!("{} respawned", name.0.as_str()), 2.6);
                    vfx.write(VfxMessage::Burst {
                        pos: aabb.center,
                        count: 16,
                        speed: 230.0,
                        color: [0.84, 0.95, 1.0, 0.82],
                        kind: ParticleKind::Spark,
                    });
                }
            }
            continue;
        }

        let breaks_on_stand = feature.breakable.collision.blocks_movement()
            && feature.breakable.trigger.allows_stand();
        let Some(mut stand_timer) = stand_timer else {
            continue;
        };
        // Iterate every player so any player standing on a
        // collision-blocking breakable triggers the collapse. Single-
        // player behavior preserved because there's one entity in the
        // iterator today. OVERNIGHT-TODO #17.8 (iterate-all-players
        // "no targeting" B-bucket).
        let any_player_standing = breaks_on_stand
            && player_body_q
                .iter()
                .any(|kin| player_is_standing_on(kin.aabb(), aabb.aabb()));
        if any_player_standing {
            stand_timer.0 += dt;
            if stand_timer.0 >= BREAK_ON_STAND_SECONDS {
                let damage = feature.breakable.health.current.max(1);
                let broke = feature.breakable.apply_damage(damage);
                if broke {
                    begin_ecs_breakable_respawn(&mut commands, entity, &feature.breakable);
                    stand_timer.0 = 0.0;
                    banner.show(format!("{} collapsed under weight", name.0.as_str()), 2.6);
                    emit_breakable_destroyed(aabb.center, &mut sfx, &mut vfx, &mut debris);
                }
            }
        } else {
            stand_timer.0 = (stand_timer.0 - dt * 2.0).max(0.0);
        }
    }
}
