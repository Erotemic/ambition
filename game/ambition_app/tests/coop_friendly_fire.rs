use ambition_app::Platformer2dSimHarness;
use ambition_platformer2d::{combat, engine_core as ae};
use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d::platformer::lifecycle::{session_world_component, InRoomInstance};
use ambition_platformer2d::persistence::settings::UserSettings;
use bevy::prelude::*;

use crate::common::{base, fixed_60hz_room_options};
use crate::two_players_two_live_rooms::{bob_beside_alice, ROOM};

fn party(enabled: bool) -> Platformer2dSimHarness {
    Platformer2dSimHarness::build(fixed_60hz_room_options(ROOM), |app, options| {
        let mut settings = UserSettings::default();
        settings.gameplay.coop_friendly_fire = enabled;
        app.insert_resource(settings);
        ambition_app::rl_sim::ambition_sim_composition(app, options)
    }).expect("the configured session prepares")
}

fn driven(sim: &mut Platformer2dSimHarness, slot: u8) -> Entity {
    let world = sim.world_mut();
    let bodies: Vec<_> = world.query::<(Entity, &DrivingParticipant)>().iter(world)
        .filter(|(_, driver)| driver.0 == PlayerSlot(slot)).map(|(entity, _)| entity).collect();
    assert_eq!(bodies.len(), 1, "one body drives slot {slot}");
    bodies[0]
}

#[test]
fn coop_friendly_fire_is_frozen_in_the_session_and_its_prepared_identity() {
    let mut identities = Vec::new();
    for enabled in [false, true] {
        let mut sim = party(enabled);
        let rule = *session_world_component::<combat::targeting::ParticipantFriendlyFire>(sim.world())
            .expect("the prepared session installs its rule");
        assert_eq!(rule.enabled, enabled);
        identities.push(session_world_component::<ambition_platformer2d::runtime::PreparedContentIdentity>(sim.world())
            .unwrap().fingerprint);
        sim.world_mut().resource_mut::<UserSettings>().gameplay.coop_friendly_fire = !enabled;
        for _ in 0..4 { sim.step(base()); }
        assert_eq!(session_world_component::<combat::targeting::ParticipantFriendlyFire>(sim.world()), Some(&rule));
    }
    assert_ne!(identities[0], identities[1], "different effective rules cannot agree as one content definition");
}

#[test]
fn coop_friendly_fire_reaches_both_body_consumers_through_real_melee_contacts() {
    for enabled in [false, true] {
        for source_slot in [0, 1] {
            let mut sim = party(enabled);
            let room = bob_beside_alice(&mut sim, ROOM, Some(PlayerSlot(1)));
            let source = driven(&mut sim, source_slot);
            let victim = driven(&mut sim, 1 - source_slot);
            // Bystander protection is separate from friendly fire.
            for body in [source, victim] {
                sim.world_mut().entity_mut(body).insert((
                    combat::components::ActiveCombatant,
                    ambition_platformer2d::actor::BodyHealth::new(ambition_platformer2d::actor::Health::new(100)),
                ));
            }
            let source_pos = sim.world().get::<ae::BodyKinematics>(source).unwrap().pos;
            sim.world_mut().get_mut::<ae::BodyKinematics>(victim).unwrap().pos = source_pos + ae::Vec2::new(24.0, 0.0);
            for _ in 0..2 { sim.step(base()); }
            let before = sim.world().get::<ambition_platformer2d::actor::BodyHealth>(victim).unwrap().damage_taken();
            let policy = {
                let world = sim.world_mut();
                world.query::<(&ambition_platformer2d::platformer::lifecycle::LiveRoomInstance, &combat::rules::RoomCombatTuning)>()
                    .iter(world).find(|(live, _)| **live == room).unwrap().1.0.damage_policy()
            };
            assert_eq!(policy.participants, Some(enabled));
            let strike = sim.world_mut().spawn((
                combat::strike::Hitbox {
                    owner: source, source: combat::strike::HitSide::Player,
                    anchor: combat::strike::HitboxAnchor::FollowOwner { local_offset: ae::Vec2::ZERO },
                    half_extent: ae::Vec2::new(96.0, 96.0), shape: None, facing: 1.0,
                    damage: 1, knockback: combat::strike::HitboxKnockback::FeelScale(0.0),
                    launch_dir: None, frame_down: ae::DEFAULT_GRAVITY_DIR,
                    reaction: None, strike_sfx: None, guard: Default::default(),
                },
                combat::strike::HitboxHits::default(),
                combat::strike::HitboxLifetime { remaining_s: 0.1 },
                InRoomInstance(room),
            )).id();
            for _ in 0..2 { sim.step(base()); }
            let after = sim.world().get::<ambition_platformer2d::actor::BodyHealth>(victim).unwrap().damage_taken();
            assert_eq!(after > before, enabled, "source slot {source_slot}, friendly fire={enabled}: {before} -> {after}; contact={:?}, health={:?}",
                sim.world().get::<combat::strike::HitboxHits>(strike).map(|hits| hits.hit.contains(&victim)),
                sim.world().get::<ambition_platformer2d::actor::BodyHealth>(victim));
        }
    }
}

#[test]
fn detached_attacks_keep_the_participants_rule_after_control_or_owner_loss() {
    use ambition_platformer2d::characters::actor::ActorFaction;
    use ambition_platformer2d::projectiles as shots;
    for enabled in [false, true] {
        for projectile in [false, true] {
            for remove_owner in [false, true] {
                let mut sim = party(enabled);
                let room = bob_beside_alice(&mut sim, ROOM, Some(PlayerSlot(1)));
                let source = driven(&mut sim, 1);
                let victim = driven(&mut sim, 0);
                sim.world_mut().entity_mut(source).insert(ActorFaction::Enemy);
                sim.world_mut().entity_mut(victim).insert((
                    combat::components::ActiveCombatant,
                    ambition_platformer2d::actor::BodyHealth::new(ambition_platformer2d::actor::Health::new(100)),
                ));
                let center = sim.world().get::<ae::BodyKinematics>(victim).unwrap().pos + ae::Vec2::new(0.0, -80.0);
                let area = if projectile {
                    sim.world_mut().write_message(shots::ProjectileSpawnRequest::named(
                        source,
                        shots::InFlightProjectile { body: shots::ProjectileBody::from_spec(shots::ProjectileSpec {
                            origin: center, direction: ae::Vec2::new(1.0, 0.0), damage: 1,
                            speed: 0.0, max_lifetime: 1.0, half_extent: ae::Vec2::splat(24.0),
                            gravity: 0.0, bounces: 0, world_hit: shots::WorldHitPolicy::Bouncing,
                            charge_tier: 0, boomerang_return_s: None, splash_half_extent: 0.0,
                        }) },
                        shots::ProjectileKind::Fireball, shots::ProjectileStart::StepNextTick,
                    ));
                    None
                } else {
                    Some(sim.world_mut().spawn((
                        combat::strike::Hitbox {
                            owner: source, source: combat::strike::HitSide::Player,
                            anchor: combat::strike::HitboxAnchor::World { center },
                            half_extent: ae::Vec2::splat(24.0), shape: None, facing: 1.0,
                            damage: 1, knockback: combat::strike::HitboxKnockback::FeelScale(0.0),
                            launch_dir: None, frame_down: ae::DEFAULT_GRAVITY_DIR,
                            reaction: None, strike_sfx: None, guard: Default::default(),
                        },
                        combat::strike::HitboxHits::default(),
                        combat::strike::HitboxLifetime { remaining_s: 1.0 }, InRoomInstance(room),
                    )).id())
                };
                sim.step(base());
                let attack = area.unwrap_or_else(|| {
                    let world = sim.world_mut();
                    let matches: Vec<_> = world.query::<(Entity, &shots::ProjectileOwner)>().iter(world)
                        .filter(|(_, owner)| owner.0 == source).map(|(entity, _)| entity).collect();
                    assert_eq!(matches.len(), 1);
                    matches[0]
                });
                let capture = sim.world().get::<combat::allegiance::AttackAllegiance>(attack).expect("capture precedes owner loss");
                assert_eq!(capture.driver, Some(DrivingParticipant(PlayerSlot(1))));
                if remove_owner { sim.world_mut().despawn(source); }
                else { sim.world_mut().entity_mut(source).remove::<DrivingParticipant>(); }
                sim.world_mut().get_mut::<ae::BodyKinematics>(victim).unwrap().pos = center;
                let before = sim.world().get::<ambition_platformer2d::actor::BodyHealth>(victim).unwrap().damage_taken();
                for _ in 0..2 { sim.step(base()); }
                let after = sim.world().get::<ambition_platformer2d::actor::BodyHealth>(victim).unwrap().damage_taken();
                assert_eq!(after > before, enabled, "projectile={projectile}, remove_owner={remove_owner}, enabled={enabled}: {before} -> {after}");
            }
        }
    }
}

#[test]
fn peers_with_different_coop_damage_choices_refuse_before_simulation() {
    use ambition_platformer2d::rollback::{loopback_transports, start_peer_session, stop_session, PeerSessionSettings, RollbackFrameCount};
    let a = "127.0.0.1:7051".parse().unwrap();
    let b = "127.0.0.1:7052".parse().unwrap();
    let (to_b, to_a) = loopback_transports(a, b, 3);
    let mut peers = Vec::new();
    for (enabled, local, remote, transport) in [(true, 0, (1, b), to_b), (false, 1, (0, a), to_a)] {
        let options = fixed_60hz_room_options(ROOM)
            .with_sync_test_rollback_settings(4, 10).with_rollback_players(2);
        let mut sim = Platformer2dSimHarness::build(options, |app, options| {
            let mut settings = UserSettings::default();
            settings.gameplay.coop_friendly_fire = enabled;
            app.insert_resource(settings);
            ambition_app::rl_sim::ambition_sim_composition(app, options)
        }).unwrap();
        bob_beside_alice(&mut sim, ROOM, Some(PlayerSlot(1)));
        let settings = PeerSessionSettings {
            players: 2, local: vec![local], remote: vec![remote],
            max_prediction_window: 8, input_delay: 0, desync_interval: 1,
            disconnect_timeout: std::time::Duration::from_secs(600),
        };
        stop_session(sim.world_mut());
        start_peer_session(sim.world_mut(), &settings, transport).unwrap();
        peers.push(sim);
    }
    for sim in &mut peers {
        let tick = sim.world().resource::<ambition_platformer2d::time::SimTick>().0;
        sim.app_mut().update();
        let reason = sim.rollback_health().expect_err("different effective rules cannot share a timeline");
        assert!(reason.contains("peer contract mismatch"), "{reason}");
        assert_eq!(sim.world().resource::<ambition_platformer2d::time::SimTick>().0, tick);
        assert_eq!(sim.world().resource::<RollbackFrameCount>().0, 0);
    }
}
