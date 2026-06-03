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

/// Speed multiplier applied to a parried shot as it reverses — a timed parry
/// sends the bolt back a little faster than it arrived (a satisfying deflect).
const PROJECTILE_REFLECT_SPEED_SCALE: f32 = 1.3;

pub fn update_enemy_projectiles(
    world_time: Res<crate::WorldTime>,
    world: Res<GameWorld>,
    gravity: crate::physics::GravityCtx,
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
    // Enemy/boss targets for PLAYER-faction shots (a wielded ranged boss
    // attack, `crate::volley`). Same shapes the held-projectile + feature-damage
    // overlap helpers use. Enemy-faction shots ignore these and hit the player.
    ecs_actors: Query<
        (
            &crate::features::FeatureId,
            &crate::features::FeatureAabb,
            &crate::features::ActorDisposition,
            &crate::features::ActorCombatState,
        ),
        (
            With<crate::features::FeatureSimEntity>,
            Without<crate::features::BossConfig>,
        ),
    >,
    ecs_bosses: Query<
        (
            &crate::features::FeatureId,
            &crate::features::FeatureAabb,
            crate::features::BossClusterRef,
            &crate::brain::BossAttackState,
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        With<crate::features::FeatureSimEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    let mut keep = Vec::with_capacity(state.bodies.len());
    let bodies = std::mem::take(&mut state.bodies);

    for mut shot in bodies {
        // Localized gravity: resolve from the shot's own position.
        let gravity_sign = gravity.sign_at(shot.body.pos);
        let alive = shot.body.tick(dt, gravity_sign);
        if !alive {
            continue;
        }

        // Faction routing: a PLAYER-faction shot (a wielded ranged boss attack)
        // damages the enemies/bosses it overlaps and expires on contact —
        // mirroring `item_pickup::held_projectile_step`, reusing the shared
        // overlap helpers so the hit-check matches what `apply_feature_hit_events`
        // applies (incl. multi-part boss hurtboxes). Enemy-faction shots fall to
        // the existing player-damage path below, byte-identical.
        if shot.body.faction == crate::projectile::ProjectileFaction::Player {
            let hit_event = HitEvent {
                volume: shot.body.aabb(),
                damage: shot.body.damage.max(1),
                source: HitSource::PlayerProjectile {
                    kind: shot.body.kind,
                },
                attacker: None,
                target: HitTarget::Volume,
                mode: HitMode::Knockback,
                knockback: None,
                ignored_targets: Vec::new(),
            };
            if crate::features::ecs_hit_event_hits_actor(&hit_event, &ecs_actors)
                || crate::features::ecs_hit_event_hits_boss(&hit_event, &ecs_bosses)
            {
                hit_events.write(hit_event);
                sfx.write(SfxMessage::Hit { pos: shot.body.pos });
                vfx.write(VfxMessage::Impact { pos: shot.body.pos });
                continue;
            }
            // No feature hit this tick — fall through to world collision (shared).
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
        let mut reflected = false;
        for (player_entity, kin, offense, dodge, shield, combat) in &player_body_q {
            if !shot.body.aabb().strict_intersects(kin.aabb()) {
                continue;
            }
            // PARRY: a timed shield reflects the shot — flip it to the player's
            // faction and reverse (+boost) its velocity, so the faction-aware
            // routing above now sends it back into the enemies/bosses. Deflect
            // the boss's own attack at it. Checked before the vulnerability gate
            // because parrying is exactly the "not vulnerable, act instead" case.
            if shield.parrying() {
                shot.body.faction = crate::projectile::ProjectileFaction::Player;
                shot.body.vel = -shot.body.vel * PROJECTILE_REFLECT_SPEED_SCALE;
                sfx.write(SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_ROCK_HIT,
                    pos: shot.body.pos,
                });
                vfx.write(VfxMessage::Impact { pos: shot.body.pos });
                reflected = true;
                break;
            }
            let dodge_rolling = dodge.roll_timer > 0.0;
            let vulnerable = !offense.invincible && !dodge_rolling && combat.vulnerable();
            if !vulnerable {
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
                attacker: None,
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
        // A parried shot survives as a player-faction bolt — keep it in flight so
        // next tick's player-faction routing lands it on the enemies.
        if reflected {
            keep.push(shot);
            continue;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy_projectile::EnemyProjectileSpawn;
    use crate::projectile::ProjectileFaction;

    #[derive(Resource, Default)]
    struct CapturedHits(Vec<HitEvent>);

    fn capture_hits(mut reader: MessageReader<HitEvent>, mut cap: ResMut<CapturedHits>) {
        for e in reader.read() {
            cap.0.push(e.clone());
        }
    }

    /// The faction-aware routing keystone: a **Player**-faction shot in the
    /// shared pool damages the enemy it overlaps (a PlayerProjectile hit, NOT an
    /// EnemyProjectile one) and expires on contact — the substrate for the
    /// wielded ranged boss attack (`crate::volley`). The enemy-faction path is
    /// unchanged (covered by the existing boss-special consumer tests).
    #[test]
    fn player_faction_shot_damages_an_overlapping_enemy_and_expires() {
        let mut app = App::new();
        app.insert_resource(crate::GameWorld(ae::World::new(
            "phys",
            ae::Vec2::new(800.0, 800.0),
            ae::Vec2::new(400.0, 400.0),
            vec![ae::Block::solid(
                "floor",
                ae::Vec2::new(0.0, 780.0),
                ae::Vec2::new(800.0, 20.0),
            )],
        )));
        app.insert_resource(crate::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.add_message::<HitEvent>();
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.init_resource::<EnemyProjectileState>();
        app.init_resource::<CapturedHits>();
        app.add_systems(Update, (update_enemy_projectiles, capture_hits).chain());

        let enemy_pos = ae::Vec2::new(300.0, 100.0);
        app.world_mut().spawn((
            crate::features::FeatureSimEntity,
            crate::features::FeatureId::new("test_enemy"),
            crate::features::FeatureAabb::new(enemy_pos, ae::Vec2::new(16.0, 24.0)),
            crate::features::ActorDisposition::Hostile,
            crate::features::ActorCombatState {
                alive: true,
                hit_flash: 0.0,
                strike_count: 0,
                attack_windup_timer: 0.0,
                attack_timer: 0.0,
                training_dummy: false,
            },
        ));
        // A player-faction shot already overlapping the enemy.
        app.world_mut()
            .resource_mut::<EnemyProjectileState>()
            .spawn_with_faction(
                EnemyProjectileSpawn {
                    origin: enemy_pos,
                    dir: ae::Vec2::new(1.0, 0.0),
                    speed: 200.0,
                    damage: 3,
                    max_lifetime: 2.0,
                    half_extent: ae::Vec2::new(8.0, 8.0),
                    owner_id: "player_volley".into(),
                    gravity: 0.0,
                },
                ProjectileFaction::Player,
            );

        app.update();

        let cap = app.world().resource::<CapturedHits>();
        assert!(
            cap.0
                .iter()
                .any(|e| matches!(e.source, HitSource::PlayerProjectile { .. })),
            "the player-faction shot lands a PlayerProjectile hit on the enemy"
        );
        assert!(
            !cap.0
                .iter()
                .any(|e| matches!(e.source, HitSource::EnemyProjectile)),
            "it must NOT register as an enemy projectile (would hit the player)"
        );
        assert!(
            app.world().resource::<EnemyProjectileState>().bodies.is_empty(),
            "the shot expires on contact with the enemy"
        );
    }

    /// Parry-reflect: an enemy shot overlapping a **parrying** player flips to
    /// the player's faction and reverses (+boosts) its velocity, so the same
    /// faction-aware routing now sends it back at the enemies — deflect the
    /// boss's attack at it.
    #[test]
    fn a_parried_enemy_shot_flips_to_player_faction_and_reverses() {
        use crate::player::{
            PlayerCombatState, PlayerDodgeState, PlayerEntity, PlayerKinematics, PlayerOffense,
            PlayerShieldState,
        };
        let mut app = App::new();
        app.insert_resource(crate::GameWorld(ae::World::new(
            "phys",
            ae::Vec2::new(800.0, 800.0),
            ae::Vec2::new(400.0, 400.0),
            vec![],
        )));
        app.insert_resource(crate::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.add_message::<HitEvent>();
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.init_resource::<EnemyProjectileState>();
        app.add_systems(Update, update_enemy_projectiles);

        let player_pos = ae::Vec2::new(200.0, 200.0);
        app.world_mut().spawn((
            PlayerEntity,
            PlayerKinematics {
                pos: player_pos,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                base_size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            PlayerOffense::default(),
            PlayerDodgeState::default(),
            // Parry window OPEN.
            PlayerShieldState {
                active: true,
                parry_window_timer: 0.2,
            },
            PlayerCombatState::default(),
        ));
        // An enemy bolt overlapping the player, travelling left (toward where it
        // came from — at the player).
        let incoming = ae::Vec2::new(-300.0, 0.0);
        app.world_mut()
            .resource_mut::<EnemyProjectileState>()
            .spawn(EnemyProjectileSpawn {
                origin: player_pos,
                dir: incoming.normalize(),
                speed: 300.0,
                damage: 2,
                max_lifetime: 2.0,
                half_extent: ae::Vec2::new(8.0, 8.0),
                owner_id: "boss_bolt".into(),
                gravity: 0.0,
            });

        app.update();

        let state = app.world().resource::<EnemyProjectileState>();
        assert_eq!(state.bodies.len(), 1, "the parried bolt stays in flight");
        let body = &state.bodies[0].body;
        assert_eq!(
            body.faction,
            crate::projectile::ProjectileFaction::Player,
            "parry flips the bolt to the player's faction"
        );
        assert!(body.vel.x > 0.0, "reversed: it now travels back toward the enemy (was -x)");
        assert!(
            body.vel.length() > 300.0,
            "reflected with a speed boost, was 300 now {}",
            body.vel.length()
        );
    }
}
