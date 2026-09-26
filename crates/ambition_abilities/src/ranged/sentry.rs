//! Sentry: a player-wielded deployable turret. `Attack` drops a stationary
//! sentry that fires bolts at the nearest enemy in range on a cadence for a
//! few seconds, then expires. It is the first deployable that attacks on its
//! own; the puppy-slug summon (`crate::thrown::puppy_slug_gun`) only wanders,
//! and other abilities are aimed one-shots.
//!
//! It fires through the shared `ProjectileSpawnRequest` seam with the sentry
//! as owner, so its bolts carry the deployer's side. It targets by
//! `CenteredAabb` and `ActorFaction::Enemy`, so it shoots mobs, not bosses or
//! the player. Pairs with the vortex: drop a sentry, pull the mob onto it.

use bevy::prelude::*;

use ambition_combat::held_items::HeldItem;
use ambition_characters::control::ActorControl;
use ambition_combat::components::{ActorFaction, CenteredAabb};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::BodyKinematics;
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
/// `pub` so the kernel's end-to-end bolt damage test (which chains two kernel
/// projectile systems) can name this value.
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

/// `Attack` while holding the sentry gauntlet drops a [`Sentry`] at the
/// wielder's feet. Plain Attack only; `Shield + Attack` drops the item (the id
/// is `UseSystem`).
///
/// Body-generic: gated on the body's resolved intent ([`ActorControl`], the
/// same frame an NPC brain writes) for every wielder, so a possessed or robot
/// body deploys through this path. Mana is the gate; a body has it only when
/// its experience declares the pool.
pub fn fire_sentry_system(
    mut wielders: Query<(
        Entity,
        &ActorControl,
        &BodyKinematics,
        &HeldItem,
        Option<&mut ambition_platformer2d_core::resources::ActorResources>,
        Option<&SessionScopedEntity>,
        // The deployer's combat side, copied onto the turret. `Option` because
        // only fixtures lack one.
        Option<&ActorFaction>,
        // The driver too: possession keeps a possessed NPC's faction as
        // `Enemy` and moves its side through the driving relationship
        // (`targeting::effective_faction`). The authored faction alone would
        // make a player's sentry shoot the player.
        Option<&ambition_characters::control::DrivingParticipant>,
        Option<&ambition_combat::targeting::MatchTeam>,
        // The deployer's identity and mint stream. Fixtures carry neither.
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
        // Refuse before spending (ADR 0030): a dynamic entity that cannot name
        // its spawner does not spawn, and its bolts, which mint under the
        // turret, would be skipped by `mint_spawned_sim_ids`. The refusal must
        // stay above `try_spend`, or it takes mana and deploys nothing.
        let (Some(deployer), Some(counter)) = (deployer_id, deployer_counter.as_mut()) else {
            warn!(
                "a sentry deploy was refused: the deployer carries no SimId or no \
                 SimIdCounter, so the turret could not be named and its bolts \
                 could not mint under it"
            );
            continue;
        };
        // The turret is a dynamically spawned sim entity, and its bolts mint
        // under it.
        let id = Some(ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(
            deployer,
            counter.next(),
        ));
        if !crate::mana::spend(mana.as_deref_mut(), SENTRY_MANA_COST) {
            continue;
        }
        // G1: the turret inherits its summoner's presentation source, so its
        // shots sound like the placing character, even after it leaves.
        let inherited = sfx.source_of(wielder);
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

/// Place one turret. The only way a sentry enters the world, so tests build
/// the same turret production does, and bolt provenance is decided once.
///
/// The turret carries its deployer's combat side. A bolt's allegiance is
/// stamped from its `ProjectileOwner` (the turret); with no `ActorFaction` on
/// the turret, `can_hit` is false against every victim and the bolts do
/// nothing.
///
/// It also carries an identity: a bolt's `SimId` is
/// `SimId::spawned(owner, ..)` with the turret as owner, so an unnamed turret
/// makes bolts `mint_spawned_sim_ids` skips.
///
/// Both are frozen at deploy, not looked up at fire time, because the turret
/// outlives its deployer. The presentation source is inherited for the same
/// reason.
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
/// The outer loop order is a gameplay decision. Two turrets firing on one
/// tick write two `ProjectileSpawnRequest`s, and the materializer assigns the
/// global `ProjectileSeq` in request order, which decides each bolt's
/// identity. So turrets are ordered by their own state, then identity, like
/// [`update_vortex_wells`]. Position and the two timers fully decide a
/// turret's action, so two that tie emit identical requests.
pub fn update_sentries(
    world_time: Res<ambition_time::WorldTime>,
    mut commands: Commands,
    mut sentries: Query<(Entity, &mut Sentry)>,
    // Tie-break authority for the outer loop, read separately so a turret
    // with no id still fires.
    ids: Query<&SimId>,
    enemies: Query<
        (
            &CenteredAabb,
            &ActorFaction,
            Option<&ambition_characters::actor::BodyHealth>,
            // A body out of play, or behind the playable plane, is not a target.
            (
                bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
                Option<&ambition_platformer2d_core::DepthPlane>,
            ),
            // Tie-break authority: two equidistant enemies are common, and
            // query order must not decide.
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
            // Whether a participant drives this body, which decides its
            // effective side. See the filter below.
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
        // Nearest enemy, with a named tie-break. `min_by` on distance alone
        // keeps the first minimum, so query order would pick between
        // equidistant enemies, and that changes who dies.
        let target = winner_by(
            enemies
                .iter()
                // A dead enemy is an intangible corpse; skip it.
                // Use the effective faction, not the authored one: a possessed
                // NPC keeps `ActorFaction::Enemy` and fights as a Player
                // through its driver. Same answer as the strike resolver.
                // Not widened to `can_damage`: which classes a sentry engages
                // (Enemy, not Npc/Boss/Neutral) is a separate design question.
                .filter(|(_, f, health, (out_of_play, plane), _, driver)| {
                    ambition_combat::targeting::effective_faction(**f, *driver)
                        == ActorFaction::Enemy
                        && !ambition_combat::util::body_is_untouchable(*health, *out_of_play, *plane)
                })
                .filter(|(aabb, _, _, _, _, _)| aabb.center.distance(sentry.pos) <= SENTRY_RANGE),
            |(aabb, _, _, _, _, _)| aabb.center.distance_squared(sentry.pos),
            |(_, _, _, _, id, _)| *id,
        )
        .map(|(aabb, _, _, _, _, _)| aabb.center);
        let Some(target) = target else {
            // No target: idle, with the cadence ready to fire as soon as an
            // enemy arrives.
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
        // The turret fires, with the source it inherited at spawn.
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

    /// ADR 0030: an unnameable turret does not deploy, and the refusal costs
    /// no mana.
    ///
    /// A deployer without an identity would place a turret nothing can name,
    /// and `mint_spawned_sim_ids` would skip its bolts. `try_spend` is in the
    /// same loop body, so this test checks the meter as well as the turret
    /// count; it fails if the refusal moves below the spend.
    #[test]
    fn a_deployer_with_no_identity_deploys_no_turret_and_keeps_its_mana() {
        let mut app = test_app();
        let deployer = spawn_primary_player_holding(&mut app, SENTRY_ID);
        // Remove the fixture body's `SimId`: ADR 0030 says this body must be
        // refused.
        app.world_mut()
            .entity_mut(deployer)
            .remove::<ambition_platformer2d_shared_tangle::sim_id::SimId>();

        let before = crate::test_support::mana(&app, deployer);
        // The deployer must afford the sentry, or the mana gate would explain
        // "no turret".
        assert!(
            before >= SENTRY_MANA_COST,
            "the fixture cannot afford a sentry ({before} < {SENTRY_MANA_COST}), so \
             a refusal and an empty meter are indistinguishable here"
        );

        app.world_mut()
            .get_mut::<ActorControl>(deployer)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();

        let mut turrets = app.world_mut().query::<&Sentry>();
        assert_eq!(
            turrets.iter(app.world()).count(),
            0,
            "a deployer with no SimId deployed a turret anyway, so the unnameable \
             road is still reachable"
        );
        let after = crate::test_support::mana(&app, deployer);
        assert_eq!(
            after, before,
            "the refusal charged the deployer for a turret it did not get — the \
             `let ... else` is below `try_spend` instead of above it"
        );
    }

    /// Control for the test above: the same fixture with its identity deploys.
    /// Otherwise "no turret" could come from any other gate.
    #[test]
    fn the_same_deployer_with_its_identity_does_deploy() {
        let mut app = test_app();
        let deployer = spawn_primary_player_holding(&mut app, SENTRY_ID);
        let before = crate::test_support::mana(&app, deployer);
        app.world_mut()
            .get_mut::<ActorControl>(deployer)
            .unwrap()
            .0
            .melee_pressed = true;
        app.update();

        let mut turrets = app.world_mut().query::<&Sentry>();
        assert_eq!(turrets.iter(app.world()).count(), 1, "one turret deployed");
        let after = crate::test_support::mana(&app, deployer);
        assert!(
            after < before,
            "a deploy that happened did not spend mana, so the meter is not the \
             witness the refusal arm thinks it is"
        );
    }

    /// Possession moves the allegiance, not the faction.
    ///
    /// A possessed NPC keeps `ActorFaction::Enemy`; `targeting::effective_faction`
    /// gives its side. A turret deployed by a player driving an enemy body must
    /// have the Player side and must not target the player's body.
    #[test]
    fn a_turret_deployed_through_a_possessed_body_fights_on_the_players_side() {
        use ambition_characters::control::DrivingParticipant;
        use ambition_characters::control::PlayerSlot;

        let mut app = test_app();
        // The body the player drives: authored Enemy, effectively Player.
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
        // The only candidate in range: authored Enemy, but driven by a
        // participant, so effectively Player.
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
        // A dead enemy is an intangible corpse: the sentry must not target it
        // (dead enemies linger with a bbox). Removing the `body_is_untouchable`
        // skip in `update_sentries` makes this fail.
        let mut app = test_app();
        let player = spawn_primary_player_holding(&mut app, SENTRY_ID);
        // A dead enemy (0 HP) in range of the deploy point (100,100).
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
/// Ordering proof for sentry bolts. The damage test moved to the kernel
/// because it chains two kernel projectile systems; see
/// `ambition_platformer2d_actor_monolith::projectile::sentry_bolt_damage_tests`.
/// The old module name is kept so its history stays findable.
mod damage_tests {
    use super::*;

    /// Which turret fires first decides which bolt gets which identity. Two
    /// ready turrets each write a `ProjectileSpawnRequest`; the materializer
    /// assigns the global `ProjectileSeq` in request order, and
    /// `mint_spawned_sim_ids` names bolts by `(owner, seq)`. This test reverses
    /// only the deploy order and compares the whole request sequence.
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
                // An enemy beside each turret, so both fire on the same tick.
                // Beside, not on: a zero aim is skipped by the fire path.
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
            // Past the shared 0.25s arm delay, so both are ready together.
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
