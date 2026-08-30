//! Sentry — a player-wielded **deployable turret**. `Attack` drops a stationary
//! sentry that auto-fires player-faction bolts at the nearest enemy in range on
//! a cadence, for a few seconds, then expires. It fills a gap in the kit: the
//! puppy-slug summon (`crate::abilities::thrown::puppy_slug_gun`) is *passive* (the slugs just
//! wander), and every other wielded ability is a one-shot the player aims — the
//! sentry is the first thing the player deploys that **autonomously attacks**.
//!
//! It fires through the same faction-aware projectile pool the volley uses
//! through the shared `ProjectileSpawnRequest` seam with the sentry as owner, so its bolts damage
//! enemies/bosses and ignore the player. Bosses carry `BodyKinematics`, but the
//! sentry targets by `CenteredAabb` + `ActorFaction::Enemy`, so it shoots mobs
//! (not bosses or the player). Pairs with the vortex: drop a sentry, vortex the
//! mob onto it.

use bevy::prelude::*;

use crate::features::{HeldItem};
use ambition_characters::control::ActorControl;
use ambition_combat::components::{ActorFaction, CenteredAabb};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::BodyMana;
use ambition_platformer2d_shared_tangle::lifecycle::{
    SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::sim_selection::winner_by;

/// Held-item id of the sentry gauntlet.
pub const SENTRY_ID: &str = "sentry";

/// Mana the sentry spends per deploy (out of 100).
const SENTRY_MANA_COST: f32 = 28.0;

/// How long (s) a deployed sentry lives.
const SENTRY_LIFETIME_S: f32 = 5.0;
/// Seconds between shots.
const SENTRY_FIRE_INTERVAL_S: f32 = 0.55;
/// Targeting range (px) — enemies beyond this are ignored.
const SENTRY_RANGE: f32 = 480.0;
const SENTRY_BOLT_SPEED: f32 = 430.0;
const SENTRY_BOLT_DAMAGE: i32 = 2;
const SENTRY_BOLT_LIFETIME: f32 = 1.4;
const SENTRY_BOLT_HALF: ae::Vec2 = ae::Vec2::new(7.0, 7.0);

/// A deployed sentry: lives at `pos`, fires when `fire_cooldown` hits zero.
#[derive(Component, Debug, Clone, Copy)]
pub struct Sentry {
    pub pos: ae::Vec2,
    pub remaining_s: f32,
    pub fire_cooldown: f32,
}

/// `Attack` while holding the sentry gauntlet drops a [`Sentry`] at the wielding
/// body's feet. Plain Attack only — `Shield + Attack` drops the item (the id is
/// `UseSystem`).
///
/// Body-generic: gated on the body's own resolved intent ([`ActorControl`], the
/// same frame an NPC brain writes) and iterating every wielder, so a
/// possessed/robot body holding the gauntlet deploys through this exact path.
/// `BodyMana` is the implicit gate (player-only today).
pub fn fire_sentry_system(
    mut wielders: Query<(
        Entity,
        &ActorControl,
        &BodyKinematics,
        &HeldItem,
        &mut BodyMana,
        Option<&SessionScopedEntity>,
        // The deployer's combat side, copied onto the turret. `Option` because a
        // body without one is a fixture, not something production seats.
        Option<&ActorFaction>,
        Option<&ambition_combat::targeting::MatchTeam>,
    )>,
    mut commands: Commands,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    for (wielder, control, kin, held, mut mana, owner, side, team) in &mut wielders {
        if !control.0.melee_pressed || control.0.shield_held {
            continue;
        }
        if held.spec.id != SENTRY_ID {
            continue;
        }
        if !mana.meter.try_spend(SENTRY_MANA_COST) {
            continue;
        }
        // G1: the turret INHERITS its summoner's presentation source, so the
        // shots it fires minutes later still sound like the character that placed
        // it — and still do after that character has left the field.
        let inherited = sfx.source_of(wielder);
        deploy_sentry(
            &mut commands,
            SessionSpawnScope::new(owner.map(|owner| owner.0)),
            kin.pos,
            side.copied().unwrap_or(ActorFaction::Player),
            team.cloned(),
            inherited,
        );
        sfx.write_for(
            wielder,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_ROCK_HIT,
                pos: kin.pos,
            },
        );
    }
}

/// Place one turret. THE seam a sentry comes into the world through.
///
/// ⭐ ONE PLACE, so a test cannot assemble a turret production never builds —
/// and so the provenance its bolts are stamped from is decided once.
///
/// ⛔⛔ THE TURRET CARRIES ITS DEPLOYER'S COMBAT SIDE, FROZEN. Without it every
/// sentry bolt was harmless, and silently: a bolt's allegiance is stamped from
/// its `ProjectileOwner`, the owner is the turret, and the turret carried
/// `Sentry`, `Name` and a session scope and NO `ActorFaction`. Nothing stamped,
/// and `indiscriminate` is `allegiance.is_none() && owner.is_none()` — false for
/// a NAMED owner — so `can_hit` was false against every victim in the world. The
/// turret fired, the bolt flew, it overlapped its target, and nothing happened.
///
/// ⭐ FROZEN AT DEPLOY, NOT LOOKED UP AT FIRE TIME, for the reason the turret
/// exists: it deliberately outlives its deployer. A side re-derived from a body
/// that has left the field is no side at all, and the presentation source is
/// inherited here for exactly the same reason.
pub fn deploy_sentry(
    commands: &mut Commands,
    scope: SessionSpawnScope,
    pos: ae::Vec2,
    side: ActorFaction,
    team: Option<ambition_combat::targeting::MatchTeam>,
    inherited_presentation: Option<ambition_sfx::PresentationSourceId>,
) -> Entity {
    let mut turret = commands.spawn_session_scoped(
        scope,
        (
            Sentry {
                pos,
                remaining_s: SENTRY_LIFETIME_S,
                // A short arm delay before the first shot.
                fire_cooldown: 0.25,
            },
            Name::new("Sentry turret"),
            side,
        ),
    );
    if let Some(team) = team {
        turret.insert(team);
    }
    if let Some(source) = inherited_presentation {
        turret.insert(ambition_sfx::BodyPresentationSource(source));
    }
    turret.id()
}

/// Tick every sentry: age it out, and when its cadence is ready, fire one
/// player-faction bolt at the nearest Enemy-faction actor within range. Runs on
/// `scaled_dt` (bullet-time slows the turret with everything else).
pub fn update_sentries(
    world_time: Res<ambition_time::WorldTime>,
    mut commands: Commands,
    mut sentries: Query<(Entity, &mut Sentry)>,
    enemies: Query<
        (
            &CenteredAabb,
            &ActorFaction,
            Option<&ambition_characters::actor::BodyHealth>,
            // The world's hands are off this body — it is not a target either.
            bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
            // The tie-break's authority, read here rather than through a second
            // lookup: two equidistant enemies is an ordinary arrangement, not a
            // corner case, and query order must not be what decides it.
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
        ),
        With<FeatureSimEntity>,
    >,
    mut projectiles: MessageWriter<ProjectileSpawnRequest>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    let dt = world_time.scaled_dt;
    if dt <= 0.0 {
        return;
    }
    for (entity, mut sentry) in &mut sentries {
        sentry.remaining_s -= dt;
        if sentry.remaining_s <= 0.0 {
            if let Ok(mut ec) = commands.get_entity(entity) {
                ec.despawn();
            }
            continue;
        }
        sentry.fire_cooldown -= dt;
        if sentry.fire_cooldown > 0.0 {
            continue;
        }
        // ⛔ NEAREST ENEMY — AND A NAMED TIE-BREAK. This was `min_by` on
        // distance alone, which keeps the FIRST minimum, so two equidistant
        // enemies were resolved by Bevy query order. Two badniks abreast of a
        // turret is not a corner case, and which one eats the bolt changes who
        // dies and when.
        let target = winner_by(
            enemies
                .iter()
                // Structural tangibility gate: a dead enemy is an
                // intangible corpse — the sentry does not target it.
                .filter(|(_, f, health, out_of_play, _)| {
                    **f == ActorFaction::Enemy
                        && !ambition_combat::util::body_is_untouchable(*health, *out_of_play)
                })
                .filter(|(aabb, _, _, _, _)| aabb.center.distance(sentry.pos) <= SENTRY_RANGE),
            |(aabb, _, _, _, _)| aabb.center.distance_squared(sentry.pos),
            |(_, _, _, _, id)| *id,
        )
        .map(|(aabb, _, _, _, _)| aabb.center);
        let Some(target) = target else {
            // No target — idle (keep the cadence ready so it fires the instant
            // an enemy wanders in).
            sentry.fire_cooldown = 0.0;
            continue;
        };
        let dir = (target - sentry.pos).normalize_or_zero();
        if dir == ae::Vec2::ZERO {
            continue;
        }
        projectiles.write(ProjectileSpawnRequest::open(
            entity,
            ProjectileSpawn {
                origin: sentry.pos,
                dir,
                speed: SENTRY_BOLT_SPEED,
                damage: SENTRY_BOLT_DAMAGE,
                max_lifetime: SENTRY_BOLT_LIFETIME,
                half_extent: SENTRY_BOLT_HALF,
                gravity: 0.0,
                visual_id: String::new(),
                // Straight volley: this ability authors no bounce.
                bounces: 0,
                bounce_on_world_contact: false,
                boomerang_return_s: None,
            },
            ProjectileStart::StepThisTick,
        ));
        sentry.fire_cooldown = SENTRY_FIRE_INTERVAL_S;
        // The TURRET fires, and it inherited its summoner's source at spawn.
        sfx.write_for(
            entity,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_ROCK_HIT,
                pos: sentry.pos,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::test_support::spawn_primary_player_holding;
    use crate::enemy_projectile::test_support::live_projectile_bodies;
    use ambition_projectiles::ProjectileSeqCounter;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<ambition_projectiles::ProjectileSpawnRequest>();
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 0.1,
            scaled_dt: 0.1,
        });
        app.init_resource::<ProjectileSeqCounter>();
        // update_sentries emits ProjectileSpawnRequest; the projectile-domain
        // materializer spawns the entity (chained after).
        app.add_systems(
            Update,
            (
                fire_sentry_system,
                update_sentries,
                ambition_projectiles::materialize_projectiles_for_this_tick,
            )
                .chain(),
        );
        app
    }

    #[test]
    fn deployed_sentry_fires_a_player_bolt_at_a_nearby_enemy() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, SENTRY_ID);
        // An enemy within range of where the sentry will deploy (100,100).
        app.world_mut().spawn((
            FeatureSimEntity,
            CenteredAabb::new(ae::Vec2::new(300.0, 100.0), ae::Vec2::new(24.0, 40.0)),
            ActorFaction::Enemy,
        ));
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update(); // deploy (arm delay 0.25; dt 0.1 → not yet firing)
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = false;
        // Tick until past the arm delay + a fire interval.
        for _ in 0..10 {
            app.update();
        }
        let bodies = live_projectile_bodies(&mut app);
        assert!(
            !bodies.is_empty(),
            "the sentry should have fired at the enemy"
        );
    }

    #[test]
    fn sentry_does_not_fire_at_a_dead_enemy() {
        // A dead enemy is an intangible corpse: the sentry must not target it.
        // (Enemies die and linger with a bbox, so this is reachable.) Poison:
        // drop the `body_is_corpse` skip in `update_sentries` and the sentry
        // fires bolts at the corpse.
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, SENTRY_ID);
        // A DEAD enemy (0 HP) within range of where the sentry deploys (100,100).
        app.world_mut().spawn((
            FeatureSimEntity,
            CenteredAabb::new(ae::Vec2::new(300.0, 100.0), ae::Vec2::new(24.0, 40.0)),
            ActorFaction::Enemy,
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health {
                current: 0,
                max: 3,
                invulnerable: Default::default(),
            }),
        ));
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = false;
        for _ in 0..10 {
            app.update();
        }
        assert!(
            live_projectile_bodies(&mut app).is_empty(),
            "the sentry must not fire at a dead enemy corpse"
        );
    }

    #[test]
    fn sentry_with_no_enemy_in_range_does_not_fire_and_expires() {
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, SENTRY_ID);
        // Enemy far outside SENTRY_RANGE.
        app.world_mut().spawn((
            FeatureSimEntity,
            CenteredAabb::new(ae::Vec2::new(2000.0, 100.0), ae::Vec2::new(24.0, 40.0)),
            ActorFaction::Enemy,
        ));
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        app.world_mut()
            .get_mut::<ActorControl>(player)
            .unwrap()
            .0
            .melee_pressed = false;
        for _ in 0..5 {
            app.update();
        }
        assert!(
            live_projectile_bodies(&mut app).is_empty(),
            "no target in range → no shots"
        );
        // Age out (lifetime 5s at 0.1/tick → 50 ticks).
        for _ in 0..55 {
            app.update();
        }
        let count = app.world_mut().query::<&Sentry>().iter(app.world()).count();
        assert_eq!(count, 0, "the sentry expires and despawns");
    }
}

#[cfg(test)]
mod damage_tests {
    use super::*;
    use ambition_combat::events::{HitEvent, HitSource};
    use ambition_vfx::vfx::VfxMessage;
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct CapturedHits(Vec<HitEvent>);

    fn capture_hits(mut reader: MessageReader<HitEvent>, mut cap: ResMut<CapturedHits>) {
        for e in reader.read() {
            cap.0.push(e.clone());
        }
    }

    /// ⭐⭐ A DEPLOYED SENTRY MUST ACTUALLY DAMAGE THE ENEMY IT SHOOTS.
    ///
    /// ⛔⛔ ASSERTING THAT A BOLT APPEARS IS NOT THIS TEST. The turret fired,
    /// the projectile materialized, it flew, it overlapped its target — and it
    /// could not damage anything, because a shot's combat side is stamped from
    /// its OWNER entity and the owner here is the turret, which carries
    /// `Sentry`, `Name` and a session scope and no `ActorFaction` at all.
    /// `indiscriminate` is `allegiance.is_none() && owner.is_none()`, so a
    /// named owner with no faction is the one combination that can hit nobody.
    #[test]
    fn a_sentry_bolt_damages_the_enemy_it_was_fired_at() {
        let mut app = App::new();
        app.insert_resource(ambition_boss_encounter::test_boss_catalog().clone());
        app.init_resource::<ambition_projectiles::ProjectileVisualCatalog>();
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            ambition_platformer2d_core::RoomGeometry(ae::World::new(
                "sentry range",
                ae::Vec2::new(2000.0, 800.0),
                ae::Vec2::new(400.0, 400.0),
                Vec::new(),
            )),
        );
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.add_message::<HitEvent>();
        app.add_message::<ambition_sfx::OwnedSfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_message::<ProjectileSpawnRequest>();
        app.add_message::<crate::avatar::PlayerHealRequested>();
        app.init_resource::<ambition_projectiles::ProjectileSeqCounter>();
        app.init_resource::<CapturedHits>();
        app.init_resource::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>();
        app.init_resource::<ambition_gameplay_trace::GameplayTraceBuffer>();
        app.add_systems(
            Update,
            (
                update_sentries,
                ambition_projectiles::materialize_projectiles_for_this_tick,
                crate::projectile::stamp_new_projectile_allegiance,
                crate::projectile::step_projectiles,
                capture_hits,
            )
                .chain(),
        );

        let enemy_pos = ae::Vec2::new(360.0, 400.0);
        let enemy = app
            .world_mut()
            .spawn((
                ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity,
                ambition_combat::components::FeatureId::new("sentry_target"),
                ambition_combat::components::CenteredAabb::new(
                    enemy_pos,
                    ae::Vec2::new(16.0, 24.0),
                ),
                ambition_combat::components::ActorDisposition::Hostile,
                ActorFaction::Enemy,
                ambition_characters::actor::BodyCombat {
                    hit_flash: 0.0,
                    training_dummy: false,
                    ..Default::default()
                },
                ambition_characters::actor::BodyHealth::new(
                    ambition_characters::actor::Health::new(20),
                ),
            ))
            .id();

        // The turret, deployed by a Player-faction wielder, a short way from its
        // target and armed to fire on the first tick.
        let wielder = app.world_mut().spawn(ActorFaction::Player).id();
        // ⛔ SPAWNED THROUGH THE PRODUCTION SEAM, not by hand: a fixture that
        // assembles its own turret can give it a faction production never
        // grants, and then this test passes about a body that does not ship.
        let side = *app
            .world()
            .get::<ActorFaction>(wielder)
            .expect("the fixture wielder states a side");
        {
            let mut commands = app.world_mut().commands();
            deploy_sentry(
                &mut commands,
                ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                ae::Vec2::new(300.0, 400.0),
                side,
                None,
                None,
            );
        }
        app.world_mut().flush();

        // Long enough for the bolt to cross the 60px gap at its authored speed.
        for _ in 0..90 {
            app.update();
        }

        let hits: Vec<_> = app
            .world()
            .resource::<CapturedHits>()
            .0
            .iter()
            .filter(|e| matches!(e.source, HitSource::Projectile))
            .collect();
        assert!(
            !hits.is_empty(),
            "the sentry's bolt reached its target and dealt no damage — a shot \
             whose owner carries no faction stamps no allegiance, and \
             `indiscriminate` is false for a NAMED owner, so `can_hit` is false \
             against every victim in the world"
        );

        assert!(
            hits.iter()
                .all(|e| e.target == ambition_combat::events::HitTarget::Body(enemy)),
            "the bolt must NAME the body it struck, got {:?}",
            hits.iter().map(|e| &e.target).collect::<Vec<_>>(),
        );
        assert!(
            hits.iter().any(|e| e.damage == SENTRY_BOLT_DAMAGE),
            "the bolt lands its authored {SENTRY_BOLT_DAMAGE} damage, got {:?}",
            hits.iter().map(|e| e.damage).collect::<Vec<_>>(),
        );
        // ⚠ THE VERDICT UNDER TEST IS `can_hit`, and it is complete here: a
        // `HitEvent` naming this victim with this damage is exactly what the
        // faction routing decides. `apply_feature_hit_events` — which turns that
        // into `BodyHealth` — is a separate system sitting AT Bevy's
        // system-param ceiling and is covered where it lives; wiring its dozen
        // resources in here would make this a test about fixture assembly.
    }
}
