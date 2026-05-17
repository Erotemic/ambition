//! Per-tick advance + collision for in-flight enemy projectiles.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::prelude::*;

use super::state::EnemyProjectileState;
use crate::audio::SfxMessage;
use crate::features::{PlayerDamageEvent, PlayerDamageMode, PlayerDamageSource};
use crate::fx::VfxMessage;
use crate::GameWorld;

pub fn update_enemy_projectiles(
    world_time: Res<crate::WorldTime>,
    world: Res<GameWorld>,
    mut state: ResMut<EnemyProjectileState>,
    player_body_q: Query<
        (
            &crate::player::PlayerBody,
            &crate::player::PlayerCombatState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    mut player_damage: MessageWriter<PlayerDamageEvent>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    let dt = world_time.sim_dt();
    let mut keep = Vec::with_capacity(state.bodies.len());
    let bodies = std::mem::take(&mut state.bodies);

    let player_state = player_body_q.single().ok();

    for mut shot in bodies {
        let alive = shot.body.tick(dt);
        if !alive {
            continue;
        }

        // Player damage check (gate on vulnerability + alive).
        if let Some((pb, combat)) = player_state {
            let vulnerable =
                !pb.invincible && !pb.dodge_rolling && !pb.parrying && combat.vulnerable();
            if vulnerable && shot.body.aabb().strict_intersects(pb.aabb()) {
                let knock_dir = (pb.pos.x - shot.body.pos.x).signum();
                let knock_dir = if knock_dir.abs() < 0.001 {
                    1.0
                } else {
                    knock_dir
                };
                let impact_pos = ae::Vec2::new(
                    (pb.pos.x + shot.body.pos.x) * 0.5,
                    (pb.pos.y + shot.body.pos.y) * 0.5,
                );
                player_damage.write(PlayerDamageEvent {
                    mode: PlayerDamageMode::Knockback,
                    source: PlayerDamageSource::EnemyProjectile,
                    source_pos: shot.body.pos,
                    impact_pos,
                    knockback_dir: knock_dir,
                    strength: 0.85,
                    amount: shot.body.damage.max(1),
                });
                sfx.write(SfxMessage::Hit { pos: shot.body.pos });
                vfx.write(VfxMessage::Impact { pos: shot.body.pos });
                continue;
            }
        }

        // World collision: expire on solid contact. (One-way platforms
        // are treated as solid for enemy shots so they don't sail
        // through floors and confuse the spatial read.)
        let aabb = shot.body.aabb();
        let solid_hit = world.0.blocks.iter().any(|block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. } | ae::BlockKind::OneWay
            ) && block.aabb.strict_intersects(aabb)
        });
        if solid_hit {
            vfx.write(VfxMessage::Impact { pos: shot.body.pos });
            continue;
        }

        keep.push(shot);
    }

    state.bodies = keep;
}
