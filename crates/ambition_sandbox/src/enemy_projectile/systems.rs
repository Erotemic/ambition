//! Per-tick advance + collision for in-flight enemy projectiles.
//!
//! Phase 3c-iii: the in-flight bodies are ECS entities now
//! (`crate::enemy_projectile::entity`), mirroring the player pool. The spawn
//! consumer spawns one entity per `SpawnProjectile`; `update_enemy_projectiles`
//! is an ECS system that collects the entities, sorts by the shared
//! `ProjectileSeq`, and steps each (tick → faction routing → world collision →
//! keep/despawn) in that stable order — byte-identical to the old
//! `Vec`-iteration order.

use crate::engine_core as ae;
use crate::engine_core::AabbExt;
use bevy::prelude::*;

use super::entity::EnemyProjectile;
use crate::audio::SfxMessage;
use crate::features::{HitEvent, HitKnockback, HitMode, HitSource, HitTarget};
use crate::presentation::fx::VfxMessage;
use crate::projectile::{resolve_world_collision, WorldHitOutcome, WorldHitPolicy};
use crate::projectile::{
    ProjectileGameplay, ProjectileOwnerId, ProjectileSeq, ProjectileSeqCounter,
};
use crate::GameWorld;

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
                EnemyProjectile,
                Name::new("Enemy projectile (sim)"),
            ));
        }
    }
}

/// Speed multiplier applied to a parried shot as it reverses — a timed parry
/// sends the bolt back a little faster than it arrived (a satisfying deflect).
const PROJECTILE_REFLECT_SPEED_SCALE: f32 = 1.3;

/// Health a successful parry restores. A small reward for the skill-timed
/// deflect (and a reason to parry rather than dodge) — feel-tune.
const PARRY_HEAL: i32 = 1;

/// A timed-out or wall-killed **lasersword** detonates with a rendered
/// explosion (Jon's polish list: "when a laser sword times out or hits a wall it
/// should explode … use one of our rendered explosion sprites"). Laserswords are
/// tagged by their `lasersword:`-prefixed owner id. Returns `None` for any other
/// projectile so it keeps its plain despawn / `Impact` cue. VFX-only — it writes
/// a presentation message and never touches sim state, so replay is unaffected.
fn lasersword_detonation(
    owner: &crate::projectile::ProjectileOwnerId,
    pos: ae::Vec2,
) -> Option<VfxMessage> {
    // Same `"lasersword"` owner-id prefix the visuals layer keys its spinning-
    // sword sprite on (`enemy_projectile::visuals::LASERSWORD_OWNER_PREFIX`).
    owner
        .0
        .starts_with("lasersword")
        .then_some(VfxMessage::Explosion {
            pos,
            kind: crate::presentation::fx::ExplosionKind::ClassicBurst,
            scale: 0.7,
        })
}

#[allow(clippy::too_many_arguments)]
pub fn update_enemy_projectiles(
    mut commands: Commands,
    world_time: Res<crate::WorldTime>,
    world: Res<GameWorld>,
    gravity: crate::physics::GravityCtx,
    // In-flight enemy projectiles are ECS entities now (Phase 3c-iii). The step
    // loop below collects them, sorts by `ProjectileSeq` (Bevy iteration order
    // is unspecified; seq reproduces the old Vec push order), then steps each.
    //
    // B0001 disjointness: this is the only `&mut BodyKinematics` query in the
    // system, but it must be provably disjoint from the player / actor / boss
    // body queries below that also touch `BodyKinematics`. Projectiles are
    // neither a player nor a feature-sim entity, so `Without<PlayerEntity>` +
    // `Without<FeatureSimEntity>` proves it (mirrors the player pool's fix).
    mut projectiles: Query<
        (
            Entity,
            &mut crate::player::BodyKinematics,
            &mut ProjectileGameplay,
            &ProjectileSeq,
            &crate::projectile::ProjectileOwnerId,
        ),
        (
            With<EnemyProjectile>,
            Without<crate::player::PlayerEntity>,
            Without<crate::features::FeatureSimEntity>,
        ),
    >,
    player_body_q: Query<
        (
            Entity,
            &crate::player::BodyKinematics,
            &crate::player::PlayerOffense,
            &crate::player::PlayerDodgeState,
            &crate::player::PlayerShieldState,
            &crate::player::PlayerCombatState,
        ),
        // `Without<EnemyProjectile>` keeps this read-only player body query
        // provably disjoint from the mutable projectile query above (both touch
        // `BodyKinematics`; B0001).
        (With<crate::player::PlayerEntity>, Without<EnemyProjectile>),
    >,
    mut hit_events: MessageWriter<HitEvent>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    // A successful parry heals the player a little (a reason to parry, not dodge).
    mut heals: MessageWriter<crate::player::PlayerHealRequested>,
    // Enemy/boss targets for PLAYER-faction shots (a wielded ranged boss
    // attack, `crate::abilities::ranged::volley`). Same shapes the held-projectile + feature-damage
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
    // Placed portals — an enemy shot crossing an aperture transits the pair
    // (rotated momentum) instead of detonating on the portal wall.
    portals: Query<&crate::portal::PlacedPortal>,
) {
    let dt = world_time.sim_dt();
    let portal_list: Vec<crate::portal::PlacedPortal> = portals.iter().copied().collect();

    // Collect the in-flight enemy projectile entities and sort by spawn
    // sequence. The old code iterated `state.bodies` in Vec push order (oldest
    // first); `ProjectileSeq` is the monotonic spawn id, so sorting by it
    // reproduces that order deterministically regardless of Bevy's archetype
    // iteration order — the determinism judge for `scripted_gameplay` + the
    // enemy projectile suites.
    let mut ordered: Vec<(Entity, ProjectileSeq)> = projectiles
        .iter()
        .map(|(entity, _, _, seq, _)| (entity, *seq))
        .collect();
    ordered.sort_by_key(|(_, seq)| *seq);

    for (proj_entity, _) in ordered {
        // Re-fetch mutably by entity (the collect above borrowed `&`).
        let Ok((_, mut kin, mut game, _, owner)) = projectiles.get_mut(proj_entity) else {
            continue;
        };

        // Localized gravity: resolve from the shot's own position.
        let gravity_sign = gravity.sign_at(kin.pos);
        let alive = game.tick(&mut kin, dt, gravity_sign);
        if !alive {
            // A timed-out lasersword detonates (Jon's polish list); other shots
            // just wink out as before. VFX + SFX are presentation, so replay is
            // unaffected.
            if let Some(boom) = lasersword_detonation(owner, kin.pos) {
                vfx.write(boom);
                sfx.write(SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_EXPLOSION,
                    pos: kin.pos,
                });
            }
            commands.entity(proj_entity).despawn();
            continue;
        }

        // Portal transit (both factions): if this shot crossed a portal aperture
        // this tick, map it through the pair and skip this tick's collision /
        // damage so it threads the portal instead of detonating on the wall
        // (Jon: "fireballs should transit portals without exploding").
        if !portal_list.is_empty()
            && crate::projectile::try_projectile_portal_transit(&mut kin, &portal_list)
        {
            continue;
        }

        // Faction routing: a PLAYER-faction shot (a wielded ranged boss attack)
        // damages the enemies/bosses it overlaps and expires on contact —
        // mirroring `item_pickup::held_projectile_step`, reusing the shared
        // overlap helpers so the hit-check matches what `apply_feature_hit_events`
        // applies (incl. multi-part boss hurtboxes). Enemy-faction shots fall to
        // the existing player-damage path below, byte-identical.
        if game.faction == crate::projectile::ProjectileFaction::Player {
            let hit_event = HitEvent {
                volume: kin.aabb(),
                damage: game.damage.max(1),
                source: HitSource::PlayerProjectile { kind: game.kind },
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
                sfx.write(SfxMessage::Hit { pos: kin.pos });
                vfx.write(VfxMessage::Impact { pos: kin.pos });
                commands.entity(proj_entity).despawn();
                continue;
            }
            // No feature hit this tick — fall through to world collision (shared).
            match resolve_world_collision(
                &mut kin,
                &mut game,
                &world.0,
                WorldHitPolicy::EnemyExpireOnAnyContact,
            ) {
                WorldHitOutcome::Expired { pos } => {
                    // A lasersword detonates on the wall (boom + blast sound);
                    // everything else gives the plain impact spark.
                    match lasersword_detonation(owner, pos) {
                        Some(boom) => {
                            vfx.write(boom);
                            sfx.write(SfxMessage::Play {
                                id: ambition_sfx::ids::WORLD_EXPLOSION,
                                pos,
                            });
                        }
                        None => {
                            vfx.write(VfxMessage::Impact { pos });
                        }
                    }
                    commands.entity(proj_entity).despawn();
                }
                WorldHitOutcome::Bounced { .. } | WorldHitOutcome::Continue => {}
            }
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
        for (player_entity, player_kin, offense, dodge, shield, combat) in &player_body_q {
            if !kin.aabb().strict_intersects(player_kin.aabb()) {
                continue;
            }
            // PARRY: a timed shield reflects the shot — flip it to the player's
            // faction and reverse (+boost) its velocity, so the faction-aware
            // routing above now sends it back into the enemies/bosses. Deflect
            // the boss's own attack at it. Checked before the vulnerability gate
            // because parrying is exactly the "not vulnerable, act instead" case.
            if shield.parrying() {
                game.faction = crate::projectile::ProjectileFaction::Player;
                kin.vel = -kin.vel * PROJECTILE_REFLECT_SPEED_SCALE;
                sfx.write(SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_ROCK_HIT,
                    pos: kin.pos,
                });
                vfx.write(VfxMessage::Impact { pos: kin.pos });
                // Reward the timed deflect with a little health.
                heals.write(crate::player::PlayerHealRequested::new(PARRY_HEAL));
                reflected = true;
                break;
            }
            let dodge_rolling = dodge.roll_timer > 0.0;
            let vulnerable = !offense.invincible && !dodge_rolling && combat.vulnerable();
            if !vulnerable {
                continue;
            }
            let knock_dir = (player_kin.pos.x - kin.pos.x).signum();
            let knock_dir = if knock_dir.abs() < 0.001 {
                1.0
            } else {
                knock_dir
            };
            let impact_pos = ae::Vec2::new(
                (player_kin.pos.x + kin.pos.x) * 0.5,
                (player_kin.pos.y + kin.pos.y) * 0.5,
            );
            hit_events.write(HitEvent {
                volume: kin.aabb(),
                damage: game.damage.max(1),
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
                    source_pos: kin.pos,
                    impact_pos,
                }),
                ignored_targets: Vec::new(),
            });
            sfx.write(SfxMessage::Hit { pos: kin.pos });
            vfx.write(VfxMessage::Impact { pos: kin.pos });
            hit_any_player = true;
            break;
        }
        // A parried shot survives as a player-faction bolt — keep it in flight so
        // next tick's player-faction routing lands it on the enemies.
        if reflected {
            continue;
        }
        if hit_any_player {
            commands.entity(proj_entity).despawn();
            continue;
        }

        // World collision: dispatch through the shared resolver with
        // the enemy faction's "expire on any contact" policy. One-way
        // platforms are treated as solid for enemy shots so they
        // don't sail through floors and confuse the spatial read
        // (OVERNIGHT-TODO #17.7).
        match resolve_world_collision(
            &mut kin,
            &mut game,
            &world.0,
            WorldHitPolicy::EnemyExpireOnAnyContact,
        ) {
            WorldHitOutcome::Expired { pos } => {
                vfx.write(VfxMessage::Impact { pos });
                commands.entity(proj_entity).despawn();
            }
            WorldHitOutcome::Bounced { .. } | WorldHitOutcome::Continue => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lasersword_detonates_on_death_other_shots_do_not() {
        use crate::projectile::ProjectileOwnerId;
        let pos = ae::Vec2::new(10.0, 20.0);
        // A lasersword (its `lasersword:`-prefixed owner) detonates with a
        // rendered explosion at its position — Jon's polish-list request.
        let sword = ProjectileOwnerId("lasersword:pirate_3".to_string());
        let boom = lasersword_detonation(&sword, pos);
        assert!(
            matches!(boom, Some(VfxMessage::Explosion { pos: p, .. }) if p == pos),
            "a lasersword should detonate at its position, got {boom:?}",
        );
        // Any other enemy shot (apple, bolt) keeps its plain despawn/impact.
        let apple = ProjectileOwnerId("gnu_ton_apple:gnu_ton".to_string());
        assert!(lasersword_detonation(&apple, pos).is_none());
    }
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
        app.add_systems(Update, update_enemy_projectiles);

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
