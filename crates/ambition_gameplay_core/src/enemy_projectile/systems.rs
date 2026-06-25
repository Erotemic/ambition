//! Enemy-projectile spawn executor. `apply_projectile_effects` materializes
//! one entity per `crate::effects::Effect::Projectiles` request (enemy / boss
//! volleys). The per-tick advance + world collision is NOT here: it shares the
//! unified, faction-routed `crate::projectile::step_projectiles` with the
//! player pool. In-flight bodies are ECS entities (mirroring the player pool),
//! sorted by the shared `ProjectileSeq` for deterministic order.

use bevy::prelude::*;

use super::entity::EnemyProjectile;
use crate::projectile::{ProjectileOwnerId, ProjectileSeqCounter};

/// Materialize enemy-pool projectiles from [`crate::effects::Effect::Projectiles`]
/// requests — one projectile ENTITY per shot. Scheduled BEFORE
/// `update_enemy_projectiles` so a body spawned this tick advances one step this
/// frame. Non-projectile effects (DamageBox / Summon) are handled by
/// `crate::effects::apply_effects`; this executor lives next to the projectile
/// substrate so the shared [`ProjectileSeq`] is assigned in emission order (its
/// sort then reproduces the historical push order).
///
/// Each entity carries the SHARED [`crate::player::BodyKinematics`] body + the
/// [`ProjectileGameplay`] marker/state + the [`ProjectileOwnerId`] string + the
/// monotonic [`ProjectileSeq`].
pub fn apply_projectile_effects(
    mut commands: Commands,
    mut seq: ResMut<ProjectileSeqCounter>,
    mut requests: MessageReader<crate::effects::EffectRequest>,
) {
    for req in requests.read() {
        let crate::effects::Effect::Projectiles { faction, shots } = &req.effect else {
            continue;
        };
        for shot in shots {
            let owner_id = shot.owner_id.clone();
            let projectile =
                crate::enemy_projectile::EnemyProjectileState::build(shot.clone(), *faction);
            commands.spawn((
                projectile.body.kin,
                projectile.body.game,
                seq.next(),
                ProjectileOwnerId(owner_id),
                crate::projectile::LiveProjectile,
                EnemyProjectile,
                Name::new("Enemy projectile (sim)"),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::SfxMessage;
    use ambition_engine_core as ae;
    use crate::features::{HitEvent, HitSource};
    use ambition_vfx::vfx::VfxMessage;

    use crate::enemy_projectile::test_support::{enemy_projectile_bodies, spawn_enemy_projectile};
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
    /// wielded ranged boss attack (`crate::abilities::ranged::volley`). The enemy-faction path is
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
        app.add_message::<crate::player::PlayerHealRequested>();
        app.init_resource::<ProjectileSeqCounter>();
        app.init_resource::<CapturedHits>();
        app.init_resource::<crate::features::FeatureEcsWorldOverlay>();
        app.init_resource::<crate::trace::GameplayTraceBuffer>();
        app.add_systems(
            Update,
            (crate::projectile::step_projectiles, capture_hits).chain(),
        );

        let enemy_pos = ae::Vec2::new(300.0, 100.0);
        app.world_mut().spawn((
            crate::features::FeatureSimEntity,
            crate::features::FeatureId::new("test_enemy"),
            crate::features::CenteredAabb::new(enemy_pos, ae::Vec2::new(16.0, 24.0)),
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
        spawn_enemy_projectile(
            &mut app,
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
            enemy_projectile_bodies(&mut app).is_empty(),
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
            BodyKinematics, PlayerBaseSize, PlayerCombatState, PlayerDodgeState, PlayerEntity,
            PlayerOffense, PlayerShieldState,
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
        app.add_message::<crate::player::PlayerHealRequested>();
        app.init_resource::<ProjectileSeqCounter>();
        app.init_resource::<crate::features::FeatureEcsWorldOverlay>();
        app.init_resource::<crate::trace::GameplayTraceBuffer>();
        app.add_systems(Update, crate::projectile::step_projectiles);

        let player_pos = ae::Vec2::new(200.0, 200.0);
        app.world_mut().spawn((
            PlayerEntity,
            BodyKinematics {
                pos: player_pos,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            PlayerBaseSize {
                base_size: ae::Vec2::new(24.0, 40.0),
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
        spawn_enemy_projectile(
            &mut app,
            EnemyProjectileSpawn {
                origin: player_pos,
                dir: incoming.normalize(),
                speed: 300.0,
                damage: 2,
                max_lifetime: 2.0,
                half_extent: ae::Vec2::new(8.0, 8.0),
                owner_id: "boss_bolt".into(),
                gravity: 0.0,
            },
            ProjectileFaction::Enemy,
        );

        app.update();

        let bodies = enemy_projectile_bodies(&mut app);
        assert_eq!(bodies.len(), 1, "the parried bolt stays in flight");
        let body = &bodies[0].body;
        assert_eq!(
            body.game.faction,
            crate::projectile::ProjectileFaction::Player,
            "parry flips the bolt to the player's faction"
        );
        assert!(
            body.kin.vel.x > 0.0,
            "reversed: it now travels back toward the enemy (was -x)"
        );
        assert!(
            body.kin.vel.length() > 300.0,
            "reflected with a speed boost, was 300 now {}",
            body.kin.vel.length()
        );
    }
}
