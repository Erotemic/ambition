//! Sentry — a player-wielded **deployable turret**. `Attack` drops a stationary
//! sentry that auto-fires player-faction bolts at the nearest enemy in range on
//! a cadence, for a few seconds, then expires. It fills a gap in the kit: the
//! puppy-slug summon (`crate::thrown::puppy_slug_gun`) is *passive* (the slugs just
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

use ambition_combat::held_items::HeldItem;
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
use ambition_platformer2d_shared_tangle::sim_id::SimId;
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
/// ⚠ `pub` for one reason: the bolt's end-to-end damage proof lives in the
/// KERNEL (it chains two kernel projectile systems), and a test that cannot
/// name the number it is asserting would be asserting a literal.
pub const SENTRY_BOLT_DAMAGE: i32 = 2;
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
        // ⛔⛔ AND THE DRIVER, because the AUTHORED faction is not the side this
        // body is fighting on. Possession deliberately leaves a possessed NPC's
        // faction as `Enemy` and moves its allegiance through the driving
        // relationship instead (`targeting::effective_faction`) — so freezing
        // the authored value onto the turret gave a player's sentry an ENEMY
        // side, and it then shot at the player who placed it.
        Option<&ambition_characters::control::DrivingParticipant>,
        Option<&ambition_combat::targeting::MatchTeam>,
        // The deployer's identity and its own mint stream. `Option` because a
        // fixture body carries neither.
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
        Option<&mut ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
    )>,
    mut commands: Commands,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    for (
        wielder,
        control,
        kin,
        held,
        mut mana,
        owner,
        side,
        driver,
        team,
        deployer_id,
        mut deployer_counter,
    ) in &mut wielders
    {
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
        // `SimId::spawned(deployer, counter.next())` — the turret is a
        // dynamically-spawned sim entity, and its bolts mint under IT.
        let id = match (deployer_id, deployer_counter.as_mut()) {
            (Some(deployer), Some(counter)) => {
                Some(ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(
                    deployer,
                    counter.next(),
                ))
            }
            _ => None,
        };
        deploy_sentry(
            &mut commands,
            SessionSpawnScope::new(owner.map(|owner| owner.0)),
            kin.pos,
            ambition_combat::targeting::effective_faction(
                side.copied().unwrap_or(ActorFaction::Player),
                driver,
            ),
            team.cloned(),
            inherited,
            id,
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
/// ⛔⛔ AND SO IS ITS IDENTITY, for the same reason and a second one: a bolt's
/// `SimId` is minted as `SimId::spawned(owner, ..)` where the owner is the
/// TURRET, so an unnamed turret produced bolts `mint_spawned_sim_ids` skips
/// entirely — the chain broke one link above where anybody was looking.
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
    id: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
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
    if let Some(id) = id {
        turret.insert(id);
    }
    turret.id()
}

/// Tick every sentry: age it out, and when its cadence is ready, fire one
/// player-faction bolt at the nearest Enemy-faction actor within range. Runs on
/// `scaled_dt` (bullet-time slows the turret with everything else).
///
/// ⛔⛔ THE OUTER LOOP IS A GAMEPLAY DECISION TOO, and it was Bevy query order.
/// Two turrets firing on one tick write two `ProjectileSpawnRequest`s, and the
/// materializer hands out the GLOBAL `ProjectileSeq` in request order — so which
/// turret's bolt gets the lower sequence, and therefore which identity each bolt
/// mints, depended on archetype order. The nearest-target tie-break inside the
/// loop was repaired; the loop AROUND it was not.
///
/// ⭐ ORDERED BY THE TURRET'S OWN STATE, THEN ITS IDENTITY — the same rule
/// [`update_vortex_wells`] uses. Position and the two timers determine what a
/// turret does this tick completely, so two that tie on them emit identical
/// requests and may be visited either way round.
pub fn update_sentries(
    world_time: Res<ambition_time::WorldTime>,
    mut commands: Commands,
    mut sentries: Query<(Entity, &mut Sentry)>,
    // The final tie-break's authority for the OUTER loop, read separately so a
    // turret with no id still fires — it just cannot break a tie WITH one.
    ids: Query<&SimId>,
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
            // Whether a participant is driving this body, which is what decides
            // its EFFECTIVE side. See the filter below.
            Option<&ambition_characters::control::DrivingParticipant>,
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
    let mut order: Vec<(ae::Vec2, f32, f32, Option<SimId>, Entity)> = sentries
        .iter()
        .map(|(entity, sentry)| {
            (
                sentry.pos,
                sentry.remaining_s,
                sentry.fire_cooldown,
                ids.get(entity).ok().cloned(),
                entity,
            )
        })
        .collect();
    order.sort_by(|a, b| {
        a.0.x
            .total_cmp(&b.0.x)
            .then_with(|| a.0.y.total_cmp(&b.0.y))
            .then_with(|| a.1.total_cmp(&b.1))
            .then_with(|| a.2.total_cmp(&b.2))
            .then_with(|| a.3.cmp(&b.3))
    });
    for (_, _, _, _, entity) in order {
        let Ok((entity, mut sentry)) = sentries.get_mut(entity) else {
            continue;
        };
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
                // ⛔⛔ THE EFFECTIVE FACTION, NOT THE AUTHORED ONE. A possessed
                // NPC keeps `ActorFaction::Enemy` on purpose and fights as a
                // Player through its driving relationship, so a raw `== Enemy`
                // test had a turret firing on the body the player is currently
                // driving. `effective_faction` is the same answer the strike
                // resolver has been giving for a while — this asks it too.
                //
                // ⚠ DELIBERATELY NOT WIDENED TO `can_damage`. Which CLASSES a
                // sentry engages (Enemy, and not Npc/Boss/Neutral) is a design
                // question this repair does not answer; it only stops the
                // allegiance being read from the wrong field.
                .filter(|(_, f, health, out_of_play, _, driver)| {
                    ambition_combat::targeting::effective_faction(**f, *driver)
                        == ActorFaction::Enemy
                        && !ambition_combat::util::body_is_untouchable(*health, *out_of_play)
                })
                .filter(|(aabb, _, _, _, _, _)| aabb.center.distance(sentry.pos) <= SENTRY_RANGE),
            |(aabb, _, _, _, _, _)| aabb.center.distance_squared(sentry.pos),
            |(_, _, _, _, id, _)| *id,
        )
        .map(|(aabb, _, _, _, _, _)| aabb.center);
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
                splash_half_extent: 0.0,
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
    use crate::test_support::spawn_primary_player_holding;
    use crate::test_support::live_projectile_bodies;
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

    /// ⛔⛔ POSSESSION MOVES THE ALLEGIANCE, NOT THE FACTION.
    ///
    /// A possessed NPC keeps `ActorFaction::Enemy` deliberately — the whole
    /// point of `targeting::effective_faction` is that possession needs no
    /// faction overwrite/restore path. Both halves of the sentry read the
    /// authored field instead, so a player driving an enemy body deployed a
    /// turret whose frozen side was ENEMY, and that turret then had the player's
    /// own body as a valid target.
    #[test]
    fn a_turret_deployed_through_a_possessed_body_fights_on_the_players_side() {
        use ambition_characters::control::DrivingParticipant;
        use ambition_characters::control::PlayerSlot;

        let mut app = test_app();
        // The body the player is DRIVING: authored Enemy, effectively Player.
        let possessed = spawn_primary_player_holding(&mut app, SENTRY_ID);
        app.world_mut()
            .entity_mut(possessed)
            .insert((ActorFaction::Enemy, DrivingParticipant(PlayerSlot(0))));
        // A real enemy, nobody driving it, in range of the deploy point.
        app.world_mut().spawn((
            FeatureSimEntity,
            CenteredAabb::new(ae::Vec2::new(300.0, 100.0), ae::Vec2::new(24.0, 40.0)),
            ActorFaction::Enemy,
        ));

        app.world_mut()
            .get_mut::<ActorControl>(possessed)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();
        app.world_mut()
            .get_mut::<ActorControl>(possessed)
            .unwrap()
            .0
            .melee_pressed = false;

        let side = {
            let world = app.world_mut();
            let mut turrets = world.query_filtered::<&ActorFaction, With<Sentry>>();
            turrets.iter(world).next().copied()
        };
        assert_eq!(
            side,
            Some(ActorFaction::Player),
            "the turret froze the possessed body's AUTHORED faction, so a player's \
             sentry came out on the enemy side and will shoot the player"
        );
    }

    /// And the same fact on the target end: the body a player is driving must
    /// not be shot by a player's own turret, however its authored faction reads.
    #[test]
    fn a_turret_does_not_fire_on_the_body_a_player_is_driving() {
        use ambition_characters::control::DrivingParticipant;
        use ambition_characters::control::PlayerSlot;

        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, SENTRY_ID);
        // The ONLY candidate in range: authored Enemy, but driven by a
        // participant, so its effective side is Player.
        app.world_mut().spawn((
            FeatureSimEntity,
            CenteredAabb::new(ae::Vec2::new(300.0, 100.0), ae::Vec2::new(24.0, 40.0)),
            ActorFaction::Enemy,
            DrivingParticipant(PlayerSlot(1)),
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
            "the turret fired on a body a second participant is driving — its \
             authored `Enemy` is not the side it is fighting on"
        );
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
/// ⚠ NAMED `damage_tests` AND NO LONGER ABOUT DAMAGE. Its damage arm —
/// `a_sentry_bolt_damages_the_enemy_it_was_fired_at` — moved to the kernel in
/// the abilities carve, because it chains two KERNEL projectile systems and a
/// test needing two crates belongs where both are visible. What is left is the
/// ORDERING proof, which needs neither. Kept under the old name so the git
/// history of the pair stays findable; see
/// `ambition_platformer2d_actor_monolith::projectile::sentry_bolt_damage_tests`.
mod damage_tests {
    use super::*;

    /// ⛔⛔ WHICH TURRET FIRES FIRST DECIDED WHICH BOLT GOT WHICH IDENTITY. Two
    /// turrets ready on the same tick each write a `ProjectileSpawnRequest`, and
    /// the materializer hands out the GLOBAL `ProjectileSeq` in request order —
    /// so archetype order chose the sequence, and `mint_spawned_sim_ids` sorts by
    /// `(owner, seq)` to name the bolts. The nearest-target tie-break INSIDE the
    /// loop was repaired last pass; the loop around it was still query-ordered.
    ///
    /// This arm reverses only the deploy order and compares the whole request
    /// sequence the tick produced.
    #[test]
    fn two_turrets_firing_on_one_tick_write_their_requests_in_the_same_order() {
        fn request_origins(order: [ae::Vec2; 2]) -> Vec<ae::Vec2> {
            #[derive(Resource, Default)]
            struct Captured(Vec<ae::Vec2>);
            fn capture(
                mut reader: MessageReader<ProjectileSpawnRequest>,
                mut out: ResMut<Captured>,
            ) {
                out.0.extend(reader.read().map(|r| r.projectile.body.pos()));
            }

            let mut app = App::new();
            app.add_message::<ProjectileSpawnRequest>();
            app.add_message::<ambition_sfx::OwnedSfxMessage>();
            app.init_resource::<Captured>();
            app.insert_resource(ambition_time::WorldTime {
                raw_dt: 1.0 / 60.0,
                scaled_dt: 1.0 / 60.0,
            });
            app.add_systems(Update, (update_sentries, capture).chain());
            for (n, pos) in order.iter().enumerate() {
                // An enemy just beside each turret, so both acquire a target and
                // both fire on the same tick. Beside, not ON: a target at the
                // turret's own position gives a zero aim, which the fire path
                // skips.
                app.world_mut().spawn((
                    FeatureSimEntity,
                    CenteredAabb {
                        center: *pos + ae::Vec2::new(50.0, 0.0),
                        half_size: ae::Vec2::splat(12.0),
                    },
                    ActorFaction::Enemy,
                ));
                let mut commands = app.world_mut().commands();
                deploy_sentry(
                    &mut commands,
                    ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED,
                    *pos,
                    ActorFaction::Player,
                    None,
                    None,
                    Some(SimId::spawned(&SimId::player_slot(0), n as u64)),
                );
                app.world_mut().flush();
            }
            // Past the 0.25s arm delay both share, so both are ready together.
            for _ in 0..17 {
                app.update();
            }
            std::mem::take(&mut app.world_mut().resource_mut::<Captured>().0)
        }

        let a = ae::Vec2::new(100.0, 100.0);
        let b = ae::Vec2::new(900.0, 100.0);
        let forwards = request_origins([a, b]);
        let backwards = request_origins([b, a]);
        assert!(
            forwards.len() >= 2,
            "both turrets fired at least once, got {forwards:?}"
        );
        assert_eq!(
            forwards, backwards,
            "reversing the order two turrets were deployed in changed the order \
             their spawn requests were written, so the global ProjectileSeq — and \
             every bolt identity minted from it — lands on the other bolt"
        );
    }

}
