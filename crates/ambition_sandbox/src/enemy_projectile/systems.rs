//! Per-tick advance + collision for in-flight enemy projectiles.

use crate::engine_core as ae;
use crate::engine_core::AabbExt;
use bevy::prelude::*;

use super::state::EnemyProjectileState;
use crate::audio::SfxMessage;
use crate::features::{HitEvent, HitKnockback, HitMode, HitSource, HitTarget};
use crate::presentation::fx::VfxMessage;
use crate::projectile::{resolve_world_collision, WorldHitOutcome, WorldHitPolicy};
use crate::GameWorld;

pub fn update_enemy_projectiles(
    world_time: Res<crate::WorldTime>,
    world: Res<GameWorld>,
    mut state: ResMut<EnemyProjectileState>,
    player_body_q: Query<
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
    mut hit_events: MessageWriter<HitEvent>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    let dt = world_time.sim_dt();
    let mut keep = Vec::with_capacity(state.bodies.len());
    let bodies = std::mem::take(&mut state.bodies);

    for mut shot in bodies {
        let alive = shot.body.tick(dt);
        if !alive {
            continue;
        }

        // Player damage check (gate on vulnerability + alive). Iterates
        // every player so a future co-op build hits the player who
        // walked into the volley, not implicitly the primary player.
        // The first vulnerable, overlapping player wins; single-player
        // behavior is preserved because there's exactly one entity in
        // the query today. OVERNIGHT-TODO #17.8 (B-bucket
        // iterate-all-players for projectile/hazard hits).
        let mut hit_any_player = false;
        for (player_entity, kin, offense, dodge, shield, combat) in &player_body_q {
            let dodge_rolling = dodge.roll_timer > 0.0;
            let vulnerable = !offense.invincible
                && !dodge_rolling
                && !shield.parrying()
                && combat.vulnerable();
            if !vulnerable || !shot.body.aabb().strict_intersects(kin.aabb()) {
                continue;
            }
            let knock_dir = (kin.pos.x - shot.body.pos.x).signum();
            let knock_dir = if knock_dir.abs() < 0.001 {
                1.0
            } else {
                knock_dir
            };
            let impact_pos = ae::Vec2::new(
                (kin.pos.x + shot.body.pos.x) * 0.5,
                (kin.pos.y + shot.body.pos.y) * 0.5,
            );
            hit_events.write(HitEvent {
                volume: shot.body.aabb(),
                damage: shot.body.damage.max(1),
                source: HitSource::EnemyProjectile,
                // Enemy projectiles iterate every player; the first
                // vulnerable overlapping player wins this volley.
                // Stamp the target so the player-damage reader lands
                // the hit on the right player rather than the primary
                // by default.
                target: HitTarget::Player(player_entity),
                mode: HitMode::Knockback,
                knockback: Some(HitKnockback {
                    dir: knock_dir,
                    strength: 0.85,
                    source_pos: shot.body.pos,
                    impact_pos,
                }),
                ignored_targets: Vec::new(),
            });
            sfx.write(SfxMessage::Hit { pos: shot.body.pos });
            vfx.write(VfxMessage::Impact { pos: shot.body.pos });
            hit_any_player = true;
            break;
        }
        if hit_any_player {
            continue;
        }

        // World collision: dispatch through the shared resolver with
        // the enemy faction's "expire on any contact" policy. One-way
        // platforms are treated as solid for enemy shots so they
        // don't sail through floors and confuse the spatial read
        // (OVERNIGHT-TODO #17.7).
        match resolve_world_collision(
            &mut shot.body,
            &world.0,
            WorldHitPolicy::EnemyExpireOnAnyContact,
        ) {
            WorldHitOutcome::Expired { pos } => {
                vfx.write(VfxMessage::Impact { pos });
                continue;
            }
            WorldHitOutcome::Bounced { .. } | WorldHitOutcome::Continue => {}
        }

        keep.push(shot);
    }

    state.bodies = keep;
}
