//! Enemy-projectile spawn executor. `apply_projectile_effects` materializes
//! one entity per `crate::effects::Effect::Projectiles` request (enemy / boss
//! volleys). The per-tick advance + world collision is NOT here: it shares the
//! unified, faction-routed `crate::projectile::step_projectiles` with the
//! player pool. In-flight bodies are ECS entities (mirroring the player pool),
//! sorted by the shared `ProjectileSeq` for deterministic order.

use bevy::prelude::*;

use super::entity::EnemyProjectile;
use crate::projectile::{ProjectileOwner, ProjectileOwnerId, ProjectileSeqCounter};

/// Materialize enemy-pool projectiles from [`crate::effects::Effect::Projectiles`]
/// requests — one projectile ENTITY per shot. Scheduled BEFORE
/// `update_enemy_projectiles` so a body spawned this tick advances one step this
/// frame. Non-projectile effects (DamageBox / Summon) are handled by
/// `crate::effects::apply_effects`; this executor lives next to the projectile
/// substrate so the shared [`ProjectileSeq`] is assigned in emission order (its
/// sort then reproduces the historical push order).
///
/// Each entity carries the SHARED [`crate::actor::BodyKinematics`] body + the
/// [`ProjectileGameplay`] marker/state + the [`ProjectileOwnerId`] string + the
/// monotonic [`ProjectileSeq`]. When the request names a real firing actor
/// (`req.owner != Entity::PLACEHOLDER`) the entity also carries
/// [`ProjectileOwner`] so a kill attributes back to that actor — the
/// `step_projectiles` player-faction branch reads it for `HitEvent::attacker`
/// (player-wielded volley/meteor, a possessed actor's player-faction shots).
/// The enemy-faction branch attributes `None` independently, so an unowned or
/// enemy shot is unaffected.
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
            // Decode the opaque visual tag the firing site stamped into the
            // shot's visual identity. The render layer reads THIS component —
            // not the owner-id string — to pick the projectile's art.
            let visual_kind = crate::projectile::ProjectileVisualKind::from_tag(shot.visual_tag);
            let projectile =
                crate::enemy_projectile::EnemyProjectileState::build(shot.clone(), *faction);
            let mut entity = commands.spawn((
                projectile.body.kin,
                projectile.body.game,
                seq.next(),
                ProjectileOwnerId(owner_id),
                visual_kind,
                crate::projectile::LiveProjectile,
                EnemyProjectile,
                Name::new("Enemy projectile (sim)"),
            ));
            if req.owner != Entity::PLACEHOLDER {
                entity.insert(ProjectileOwner(req.owner));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::SfxMessage;
    use crate::features::{HitEvent, HitSource};
    use ambition_engine_core as ae;
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
        app.insert_resource(crate::RoomGeometry(ae::World::new(
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
            crate::features::BodyCombat {
                alive: true,
                hit_flash: 0.0,
                strike_count: 0,
                attack_windup_timer: 0.0,
                attack_timer: 0.0,
                training_dummy: false,
                ..Default::default()
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
                visual_tag: 0,
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

    // ── S3e: relational actor-vs-actor projectiles ──────────────────────────

    /// Build a headless app wired for `step_projectiles` with the given relations.
    fn arena_projectile_app(relations: crate::features::FactionRelations) -> App {
        let mut app = App::new();
        app.insert_resource(crate::RoomGeometry(ae::World::new(
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
        app.init_resource::<CapturedHits>();
        app.init_resource::<crate::features::FeatureEcsWorldOverlay>();
        app.init_resource::<crate::trace::GameplayTraceBuffer>();
        app.insert_resource(relations);
        app.add_systems(
            Update,
            (crate::projectile::step_projectiles, capture_hits).chain(),
        );
        app
    }

    fn spawn_boss_actor(app: &mut App, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                crate::features::FeatureSimEntity,
                crate::features::FeatureId::new("arena_robot"),
                crate::features::CenteredAabb::new(pos, ae::Vec2::new(16.0, 24.0)),
                crate::features::ActorFaction::Boss,
            ))
            .id()
    }

    fn spawn_overlapping_enemy_glider(app: &mut App, pos: ae::Vec2) {
        spawn_enemy_projectile(
            app,
            EnemyProjectileSpawn {
                origin: pos,
                dir: ae::Vec2::new(1.0, 0.0),
                speed: 200.0,
                damage: 3,
                max_lifetime: 2.0,
                half_extent: ae::Vec2::new(8.0, 8.0),
                owner_id: "pca_glider".into(),
                gravity: 0.0,
                visual_tag: 0,
            },
            ProjectileFaction::Enemy,
        );
    }

    /// An Enemy-faction shot (the PCA's glider) damages a Boss-faction body when
    /// the relations matrix marks them hostile — the projectile half of the
    /// non-player-centric arena. Pre-resolved to that exact actor.
    #[test]
    fn enemy_glider_damages_a_relationally_hostile_actor() {
        let mut relations = crate::features::FactionRelations::default();
        relations.set_mutual_hostile(
            crate::features::ActorFaction::Enemy,
            crate::features::ActorFaction::Boss,
            true,
        );
        let mut app = arena_projectile_app(relations);
        let pos = ae::Vec2::new(300.0, 100.0);
        let boss_actor = spawn_boss_actor(&mut app, pos);
        spawn_overlapping_enemy_glider(&mut app, pos);
        app.update();
        let cap = app.world().resource::<CapturedHits>();
        assert!(
            cap.0
                .iter()
                .any(|e| matches!(e.source, HitSource::EnemyProjectile)
                    && e.target == crate::features::HitTarget::Actor(boss_actor)),
            "the enemy glider lands a pre-resolved hit on the hostile Boss actor"
        );
    }

    /// Damage is PHYSICAL, not relational: with default relations (no targeting
    /// hostility set), an Enemy glider STILL damages a DIFFERENT-faction (Boss)
    /// actor it overlaps. Targeting is the relational concern; a shot that LANDS
    /// hurts any non-ally. (Friendly fire is off by default, so a same-faction
    /// body would be spared.)
    #[test]
    fn enemy_glider_damages_a_different_faction_actor_physically() {
        let mut app = arena_projectile_app(crate::features::FactionRelations::default());
        let pos = ae::Vec2::new(300.0, 100.0);
        let boss_actor = spawn_boss_actor(&mut app, pos);
        spawn_overlapping_enemy_glider(&mut app, pos);
        app.update();
        let cap = app.world().resource::<CapturedHits>();
        assert!(
            cap.0
                .iter()
                .any(|e| matches!(e.source, HitSource::EnemyProjectile)
                    && e.target == crate::features::HitTarget::Actor(boss_actor)),
            "a different-faction actor is hit regardless of relations (physical damage)"
        );
    }

    /// Parry-reflect: an enemy shot overlapping a **parrying** player flips to
    /// the player's faction and reverses (+boosts) its velocity, so the same
    /// faction-aware routing now sends it back at the enemies — deflect the
    /// boss's attack at it.
    #[test]
    fn a_parried_enemy_shot_flips_to_player_faction_and_reverses() {
        use crate::actor::BodyCombat;
        use crate::actor::BodyKinematics;
        use crate::actor::PlayerEntity;
        use crate::actor::{BodyBaseSize, BodyDodgeState, BodyOffense, BodyShieldState};
        let mut app = App::new();
        app.insert_resource(crate::RoomGeometry(ae::World::new(
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
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                crate::combat::components::ActorFaction::Player,
                BodyKinematics {
                    pos: player_pos,
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                BodyBaseSize {
                    base_size: ae::Vec2::new(24.0, 40.0),
                },
                BodyOffense::default(),
                BodyDodgeState::default(),
                // Parry window OPEN.
                BodyShieldState {
                    active: true,
                    parry_window_timer: 0.2,
                },
                BodyCombat::default(),
            ))
            .id();
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
                visual_tag: 0,
            },
            ProjectileFaction::Enemy,
        );

        app.update();

        let bodies = enemy_projectile_bodies(&mut app);
        assert_eq!(bodies.len(), 1, "the parried bolt stays in flight");
        let body = &bodies[0].body;
        // Parry RE-OWNS the bolt to the player (so its firer faction is Player next
        // tick → it routes as the player's own shot, back at the enemies) — it does
        // NOT mutate a faction label.
        let owner = app
            .world_mut()
            .query::<&crate::projectile::ProjectileOwner>()
            .iter(app.world())
            .next()
            .map(|o| o.0);
        assert_eq!(
            owner,
            Some(player),
            "parry re-owns the bolt to the player"
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

    /// Task B: an enemy shot spawned through the executor with a real firing
    /// actor carries `ProjectileOwner`, so the hit it lands on the player
    /// attributes back to that actor (`HitEvent::attacker`), instead of the
    /// historical `None`. Drives the full `EffectRequest` → executor →
    /// `step_projectiles` path so the stamping + the enemy-branch read are both
    /// exercised.
    #[test]
    fn an_owned_enemy_shot_attributes_its_player_hit_to_the_firing_actor() {
        use crate::actor::BodyCombat;
        use crate::actor::BodyKinematics;
        use crate::actor::PlayerEntity;
        use crate::actor::{BodyBaseSize, BodyDodgeState, BodyOffense, BodyShieldState};
        let mut app = App::new();
        app.insert_resource(crate::RoomGeometry(ae::World::new(
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
        app.add_message::<crate::effects::EffectRequest>();
        app.add_message::<crate::player::PlayerHealRequested>();
        app.init_resource::<ProjectileSeqCounter>();
        app.init_resource::<CapturedHits>();
        app.init_resource::<crate::features::FeatureEcsWorldOverlay>();
        app.init_resource::<crate::trace::GameplayTraceBuffer>();
        app.add_systems(
            Update,
            (
                apply_projectile_effects,
                crate::projectile::step_projectiles,
                capture_hits,
            )
                .chain(),
        );

        // Stand-in for the firing boss/enemy entity.
        let attacker = app.world_mut().spawn_empty().id();

        // A vulnerable player (no parry / dodge / invuln) at the shot's origin.
        let player_pos = ae::Vec2::new(200.0, 200.0);
        app.world_mut().spawn((
            PlayerEntity,
            BodyKinematics {
                pos: player_pos,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: ae::Vec2::new(24.0, 40.0),
            },
            BodyOffense::default(),
            BodyDodgeState::default(),
            BodyShieldState {
                active: false,
                parry_window_timer: 0.0,
            },
            BodyCombat::default(),
        ));

        // Fire an enemy-faction shot owned by `attacker`, overlapping the player.
        app.world_mut()
            .write_message(crate::effects::EffectRequest {
                owner: attacker,
                effect: crate::effects::Effect::Projectiles {
                    faction: ProjectileFaction::Enemy,
                    shots: vec![EnemyProjectileSpawn {
                        origin: player_pos,
                        dir: ae::Vec2::new(1.0, 0.0),
                        speed: 100.0,
                        damage: 2,
                        max_lifetime: 2.0,
                        half_extent: ae::Vec2::new(8.0, 8.0),
                        owner_id: "boss_bolt".into(),
                        gravity: 0.0,
                        visual_tag: 0,
                    }],
                },
            });

        app.update();

        let cap = app.world().resource::<CapturedHits>();
        let player_hit = cap
            .0
            .iter()
            .find(|e| matches!(e.source, HitSource::EnemyProjectile))
            .expect("the enemy shot lands an EnemyProjectile hit on the player");
        assert_eq!(
            player_hit.attacker,
            Some(attacker),
            "the hit attributes back to the firing actor, not None"
        );
    }

    /// The spawn executor decodes the shot's opaque `visual_tag` into a
    /// `ProjectileVisualKind` component — the render layer's single art-selection
    /// input, set without reading `owner_id`.
    #[test]
    fn spawn_executor_attaches_visual_kind_from_tag() {
        use crate::projectile::ProjectileVisualKind;
        let mut app = App::new();
        app.add_message::<crate::effects::EffectRequest>();
        app.init_resource::<ProjectileSeqCounter>();
        app.add_systems(Update, apply_projectile_effects);
        app.world_mut()
            .write_message(crate::effects::EffectRequest {
                owner: Entity::PLACEHOLDER,
                effect: crate::effects::Effect::Projectiles {
                    faction: ProjectileFaction::Enemy,
                    shots: vec![EnemyProjectileSpawn {
                        origin: ae::Vec2::ZERO,
                        dir: ae::Vec2::new(1.0, 0.0),
                        speed: 100.0,
                        damage: 1,
                        max_lifetime: 1.0,
                        half_extent: ae::Vec2::new(8.0, 8.0),
                        owner_id: "pca".into(),
                        gravity: 0.0,
                        visual_tag: ProjectileVisualKind::Glider.to_tag(),
                    }],
                },
            });
        app.update();
        let mut q = app.world_mut().query::<&ProjectileVisualKind>();
        let kinds: Vec<_> = q.iter(app.world()).copied().collect();
        assert_eq!(
            kinds,
            vec![ProjectileVisualKind::Glider],
            "the Glider tag must materialize as a Glider visual-kind component"
        );
    }
}
