//! Co-op friendly fire in the assembled game: the player's setting is frozen
//! into the prepared session, reaches each live room's damage policy, and
//! decides whether a real contact between the two participants lands, for
//! melee, scripted strikes, detached attacks, splashes and bombs. The rule
//! itself (`damage_lands_between`) and launch allegiance are unit-tested in
//! `ambition_combat` (`targeting/tests.rs`, `allegiance.rs`).
//!
//! One session per setting: the setting is frozen when the session is
//! prepared, and every case under it runs in that one session, each on a
//! fresh health pool so one case's hit or invulnerability does not read as
//! the next one's.

use ambition_app::Platformer2dSimHarness;
use ambition_platformer2d::{combat, engine_core as ae};
use ambition_platformer2d::actor::{BodyHealth, Health};
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

/// A fresh 100-point pool and no reaction left from an earlier case (the
/// invulnerability window after a hit, hitstun), so the body is a combatant
/// that the next contact can hurt.
fn fresh_health(sim: &mut Platformer2dSimHarness, body: Entity) {
    sim.world_mut().entity_mut(body).insert((
        combat::components::ActiveCombatant,
        BodyHealth::new(Health::new(100)),
    ));
    if let Some(mut reaction) = sim.world_mut().get_mut::<ambition_platformer2d::actor::BodyCombat>(body) {
        reaction.reset();
    }
}

fn damage_taken(sim: &Platformer2dSimHarness, body: Entity) -> i32 {
    sim.world().get::<BodyHealth>(body).unwrap().damage_taken()
}

/// Peers whose co-op damage choices differ refuse to share a timeline before
/// they simulate. Each peer's choice is frozen into its prepared session (a
/// later change to the setting does not reach it) and into its prepared
/// identity, which is what the peers compare.
#[test]
fn peers_with_different_coop_damage_choices_refuse_before_simulation() {
    use ambition_platformer2d::rollback::{loopback_transports, start_peer_session, stop_session, PeerSessionSettings, RollbackFrameCount};
    let a = "127.0.0.1:7051".parse().unwrap();
    let b = "127.0.0.1:7052".parse().unwrap();
    let (to_b, to_a) = loopback_transports(a, b, 3);
    let mut peers = Vec::new();
    let mut identities = Vec::new();
    for (enabled, local, remote, transport) in [(true, 0, (1, b), to_b), (false, 1, (0, a), to_a)] {
        let options = fixed_60hz_room_options(ROOM)
            .with_sync_test_rollback_settings(4, 10).with_rollback_players(2);
        let mut sim = Platformer2dSimHarness::build(options, |app, options| {
            let mut settings = UserSettings::default();
            settings.gameplay.coop_friendly_fire = enabled;
            app.insert_resource(settings);
            ambition_app::rl_sim::ambition_sim_composition(app, options)
        }).unwrap();
        let rule = *session_world_component::<combat::targeting::ParticipantFriendlyFire>(sim.world())
            .expect("the prepared session installs its rule");
        assert_eq!(rule.enabled, enabled, "the prepared session's rule is the setting it was prepared with");
        identities.push(session_world_component::<ambition_platformer2d::runtime::PreparedContentIdentity>(sim.world())
            .unwrap().fingerprint);
        sim.world_mut().resource_mut::<UserSettings>().gameplay.coop_friendly_fire = !enabled;
        bob_beside_alice(&mut sim, ROOM, Some(PlayerSlot(1)));
        assert_eq!(
            session_world_component::<combat::targeting::ParticipantFriendlyFire>(sim.world()),
            Some(&rule),
            "a change to the setting after the session was prepared reached the running session"
        );
        let settings = PeerSessionSettings {
            players: 2, local: vec![local], remote: vec![remote],
            max_prediction_window: 8, input_delay: 0, desync_interval: 1,
            disconnect_timeout: std::time::Duration::from_secs(600),
        };
        stop_session(sim.world_mut());
        start_peer_session(sim.world_mut(), &settings, transport).unwrap();
        peers.push(sim);
    }
    assert_ne!(identities[0], identities[1], "different effective rules cannot agree as one content definition");
    for sim in &mut peers {
        let tick = sim.world().resource::<ambition_platformer2d::time::SimTick>().0;
        sim.app_mut().update();
        let reason = sim.rollback_health().expect_err("different effective rules cannot share a timeline");
        assert!(reason.contains("peer contract mismatch"), "{reason}");
        assert_eq!(sim.world().resource::<ambition_platformer2d::time::SimTick>().0, tick);
        assert_eq!(sim.world().resource::<RollbackFrameCount>().0, 0);
    }
}

#[test]
fn coop_friendly_fire_reaches_both_body_consumers_through_real_melee_contacts() {
    for enabled in [false, true] {
        let mut sim = party(enabled);
        let room = bob_beside_alice(&mut sim, ROOM, Some(PlayerSlot(1)));
        let policy = {
            let world = sim.world_mut();
            world.query::<(&ambition_platformer2d::platformer::lifecycle::LiveRoomInstance, &combat::rules::RoomCombatTuning)>()
                .iter(world).find(|(live, _)| **live == room).unwrap().1.0.damage_policy()
        };
        assert_eq!(policy.participants, Some(enabled), "the room's damage policy carries the session's setting");
        for source_slot in [0, 1] {
            let source = driven(&mut sim, source_slot);
            let victim = driven(&mut sim, 1 - source_slot);
            // Bystander protection is separate from friendly fire.
            for body in [source, victim] {
                fresh_health(&mut sim, body);
            }
            let source_pos = sim.world().get::<ae::BodyKinematics>(source).unwrap().pos;
            sim.world_mut().get_mut::<ae::BodyKinematics>(victim).unwrap().pos = source_pos + ae::Vec2::new(24.0, 0.0);
            for _ in 0..2 { sim.step(base()); }
            let before = damage_taken(&sim, victim);
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
            let after = damage_taken(&sim, victim);
            assert_eq!(after > before, enabled, "source slot {source_slot}, friendly fire={enabled}: {before} -> {after}; contact={:?}, health={:?}",
                sim.world().get::<combat::strike::HitboxHits>(strike).map(|hits| hits.hit.contains(&victim)),
                sim.world().get::<BodyHealth>(victim));
            // Let the strike end before the next case swaps the roles.
            for _ in 0..8 { sim.step(base()); }
        }
    }
}

#[test]
fn a_damage_box_keeps_its_emission_room_and_session_after_its_source_leaves() {
    use bevy::ecs::system::RunSystemOnce;
    use ambition_platformer2d::characters::actor::ActorFaction;
    use ambition_platformer2d::platformer::lifecycle::{RoomScopedEntity, SessionScopedEntity};
    let mut sim = party(true);
    let room = bob_beside_alice(&mut sim, ROOM, Some(PlayerSlot(1)));
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "central_hub_complex"), "central_hub_complex");
    assert_eq!(crate::two_players_two_live_rooms::live_rooms(&mut sim).len(), 2);
    let alice = driven(&mut sim, 0);
    let bob = driven(&mut sim, 1);
    assert_ne!(sim.world().get::<InRoomInstance>(alice).unwrap().0, room);
    let session = *sim.world().get::<SessionScopedEntity>(bob).expect("the live body has a session");
    fresh_health(&mut sim, bob);
    for _ in 0..2 { sim.step(base()); }
    let home = sim.world().get::<ae::BodyKinematics>(bob).unwrap().pos;
    // One box per case, each at its own place in Bob's room, so each contact
    // is its own: (written at once or through the deferred writer, credited
    // to its emitter or to Alice in the other room).
    let cases = [(false, false), (false, true), (true, false), (true, true)];
    let mut attacks = Vec::new();
    for (index, (immediate, remote_credit)) in cases.into_iter().enumerate() {
        let center = home + ae::Vec2::new(-90.0 + 60.0 * index as f32, -80.0);
        let emitter = sim.world_mut().spawn((ActorFaction::Enemy, InRoomInstance(room), session)).id();
        let owner = if remote_credit { alice } else { emitter };
        let source = if remote_credit { combat::strike::HitSide::Player } else { combat::strike::HitSide::Enemy };
        let damage_box = combat::strike::DamageBox {
            cause: combat::events::HitSource::Melee,
            move_instance: None,
            half_extent: ae::Vec2::splat(24.0), shape: None, damage: 1,
            knockback: combat::strike::HitboxKnockback::FeelScale(0.0), lifetime_s: 60.0, name: Some("room provenance"),
        };
        if immediate {
            sim.world_mut().run_system_once(move |mut boxes: combat::damage_box::DamageBoxSpawner| {
                boxes.spawn(emitter, owner, source, center, damage_box.clone()).expect("the source is live");
            }).unwrap();
        } else {
            sim.world_mut().run_system_once(move |mut boxes: combat::damage_box::DamageBoxWriter| {
                assert!(boxes.write(emitter, owner, source, center, damage_box.clone()));
            }).unwrap();
        }
        sim.world_mut().despawn(emitter);
        sim.step(base());
        let attack = {
            let world = sim.world_mut();
            let attacks: Vec<_> = world.query::<(Entity, &combat::strike::Hitbox)>().iter(world)
                .filter(|(_, hit)| hit.owner == owner && hit.world_volume(None).unwrap().center() == center)
                .map(|(entity, _)| entity).collect();
            assert_eq!(attacks.len(), 1, "immediate={immediate}, remote_credit={remote_credit}: one box at its place");
            attacks[0]
        };
        let case = format!("immediate={immediate}, remote_credit={remote_credit}");
        assert_eq!(sim.world().get::<InRoomInstance>(attack).unwrap().0, room, "{case}: the box keeps its emission room");
        assert_eq!(sim.world().get::<SessionScopedEntity>(attack), Some(&session), "{case}: the box keeps its session");
        assert!(sim.world().get::<RoomScopedEntity>(attack).is_some(), "{case}: the box retires with its room");
        let allegiance = sim.world().get::<combat::allegiance::AttackAllegiance>(attack).unwrap();
        assert_eq!(allegiance.driver, remote_credit.then_some(DrivingParticipant(PlayerSlot(0))), "{case}: the box's credit");
        attacks.push((case, center, attack));
    }
    for (case, center, _) in &attacks {
        fresh_health(&mut sim, bob);
        sim.world_mut().get_mut::<ae::BodyKinematics>(bob).unwrap().pos = *center;
        for _ in 0..2 { sim.step(base()); }
        assert!(damage_taken(&sim, bob) > 0, "{case}: the box did not hurt Bob in its room");
    }
    assert_eq!(crate::two_players_two_live_rooms::live_rooms(&mut sim).len(), 2);
    let other_room = sim.world().get::<InRoomInstance>(alice).unwrap().0;
    let control_center = sim.world().get::<ae::BodyKinematics>(alice).unwrap().pos + ae::Vec2::new(0.0, -1000.0);
    let control = sim.world_mut().run_system_once(move |mut boxes: combat::damage_box::DamageBoxSpawner| {
        boxes.spawn(alice, alice, combat::strike::HitSide::Player, control_center,
            combat::strike::DamageBox {
                cause: combat::events::HitSource::Melee,
                move_instance: None,
                half_extent: ae::Vec2::splat(8.0), shape: None, damage: 1,
                knockback: combat::strike::HitboxKnockback::FeelScale(0.0), lifetime_s: 60.0, name: Some("other room control"),
            }).unwrap().id()
    }).unwrap();
    use ae::AabbExt as _;
    let door = crate::two_players_two_live_rooms::door_of(&mut sim, ROOM, "central_hub_complex").aabb.center();
    crate::two_players_two_live_rooms::put_bob_at(&mut sim, door);
    for _ in 0..120 {
        sim.drive_seat(1, ae::ControlFrame { interact_pressed: true, interact_held: true, ..Default::default() });
        sim.step(base());
        if sim.world().get::<InRoomInstance>(bob).unwrap().0 == other_room { break; }
    }
    sim.drive_seat(1, ae::ControlFrame::default());
    for _ in 0..30 { sim.step(base()); }
    assert_eq!(crate::two_players_two_live_rooms::live_rooms(&mut sim), vec![(other_room, "central_hub_complex".to_string())]);
    for (case, _, attack) in &attacks {
        assert!(sim.world().get_entity(*attack).is_err(), "{case}: a fixed area outlived its room");
    }
    assert!(sim.world().get_entity(control).is_ok(), "the other room lost its fixed area");
}

#[test]
fn scripted_strikes_use_body_contacts_for_either_participant() {
    use ambition_app::AgentAction;
    for enabled in [false, true] {
        let mut sim = party(enabled);
        crate::a_second_seat_joins_the_session::join(&mut sim, 4).expect("the second seat joins");
        sim.drive_seat(1, ae::ControlFrame::default());
        let spec = ambition_platformer2d::characters::brain::held_item_by_id("blink").unwrap();
        for body in [driven(&mut sim, 0), driven(&mut sim, 1)] {
            sim.world_mut().entity_mut(body).insert(combat::held_items::HeldItem::new(spec.clone()));
        }
        // Each participant blinks into the other in one room; with friendly
        // fire on, Bob then blinks again with Alice in another room.
        let mut cases = vec![(0, false), (1, false)];
        if enabled {
            cases.push((1, true));
        }
        for (source_slot, separate_rooms) in cases {
            if separate_rooms {
                // Past the earlier blink's cooldown.
                for _ in 0..120 { sim.step(base()); }
                assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "central_hub_complex"), "central_hub_complex");
            }
            let source = driven(&mut sim, source_slot);
            let victim = driven(&mut sim, 1 - source_slot);
            assert_eq!(sim.world().get::<InRoomInstance>(source) == sim.world().get::<InRoomInstance>(victim), !separate_rooms);
            let lands = enabled && !separate_rooms;
            for (body, x) in [(source, 400.0), (victim, 550.0)] {
                fresh_health(&mut sim, body);
                crate::a_second_seat_joins_the_session::put_body_at(&mut sim, body, ae::Vec2::new(x, 100.0));
            }
            for _ in 0..2 { sim.step(base()); }
            let before = damage_taken(&sim, victim);
            let from = sim.world().get::<ae::BodyKinematics>(source).unwrap().pos;
            if source_slot == 1 {
                sim.drive_seat(1, ae::ControlFrame { axis_x: 1.0, attack_pressed: true, ..Default::default() });
                sim.step(base());
                sim.drive_seat(1, ae::ControlFrame::default());
            } else {
                sim.step(AgentAction { move_x: 1.0, attack: true, ..base() });
            }
            let case = format!("slot {source_slot}, friendly fire={enabled}, separate rooms={separate_rooms}");
            let to = sim.world().get::<ae::BodyKinematics>(source).unwrap().pos;
            assert!(to.x - from.x > 100.0, "{case}: the source used the real blink module: {from:?} -> {to:?}");
            let events = sim.world().resource::<Messages<combat::events::HitEvent>>();
            let strikes: Vec<_> = events.get_cursor().read(events)
                .filter(|event| event.attacker == Some(source)).cloned().collect();
            assert_eq!(strikes.iter().any(|event| event.target == combat::events::HitTarget::Body(victim)), lands,
                "{case}: {strikes:?}");
            for _ in 0..3 { sim.step(base()); }
            let after = damage_taken(&sim, victim);
            assert_eq!(after > before, lands, "{case}: {before} -> {after}");
            assert!(!sim.world_mut().query::<&combat::strike::Hitbox>().iter(sim.world())
                .any(|hitbox| hitbox.owner == source), "{case}: a one-tick strike remained live");
        }
    }
}

/// A participant's attacks that outlive their link to him (a damage box, a
/// shot, a splash) keep the participants' rule they were launched under: the
/// allegiance is captured at launch, so losing control of the source, or the
/// source itself, changes nothing. Bob's body plays an enemy, so only his
/// launch allegiance makes him a participant.
#[test]
fn detached_attacks_and_splashes_keep_the_participants_rule_after_control_or_owner_loss() {
    use ambition_platformer2d::characters::actor::ActorFaction;
    use ambition_platformer2d::projectiles as shots;
    use bevy::ecs::system::RunSystemOnce;
    /// A shot of `source`'s that does not move by itself.
    fn shot(source: Entity, origin: ae::Vec2, half: f32, splash: f32, world_hit: shots::WorldHitPolicy) -> shots::ProjectileSpawnRequest {
        shots::ProjectileSpawnRequest::named(
            source,
            shots::InFlightProjectile { body: shots::ProjectileBody::from_spec(shots::ProjectileSpec {
                origin, direction: ae::Vec2::X, damage: 1,
                speed: 0.0, max_lifetime: 10.0, half_extent: ae::Vec2::splat(half),
                gravity: 0.0, bounces: 0, world_hit,
                charge_tier: 0, boomerang_return_s: None, splash_half_extent: splash,
            }) },
            shots::ProjectileKind::Fireball, shots::ProjectileStart::StepNextTick,
        )
    }
    fn shot_of(sim: &mut Platformer2dSimHarness, source: Entity, origin: ae::Vec2) -> Entity {
        let world = sim.world_mut();
        let matches: Vec<_> = world.query::<(Entity, &shots::ProjectileOwner, &ae::BodyKinematics)>().iter(world)
            .filter(|(_, owner, kin)| owner.0 == source && kin.pos.distance(origin) < 1.0)
            .map(|(entity, ..)| entity).collect();
        assert_eq!(matches.len(), 1, "one shot of the source at {origin:?}");
        matches[0]
    }
    for enabled in [false, true] {
        let mut sim = party(enabled);
        bob_beside_alice(&mut sim, ROOM, Some(PlayerSlot(1)));
        let alice = driven(&mut sim, 0);
        let bob = driven(&mut sim, 1);
        sim.world_mut().entity_mut(bob).insert(ActorFaction::Enemy);
        fresh_health(&mut sim, alice);
        for _ in 0..2 { sim.step(base()); }
        let home = sim.world().get::<ae::BodyKinematics>(alice).unwrap().pos;
        // Bob, still driven, launches one attack of each kind per ending: a
        // box and a shot that hold their place, and a shot that splashes
        // where it lands.
        let place = |index: usize| home + ae::Vec2::new(-120.0 + 80.0 * index as f32, -80.0);
        let mut launched = Vec::new();
        for (index, kind) in ["box", "shot"].into_iter().enumerate() {
            let center = place(index);
            let attack = if kind == "box" {
                sim.world_mut().run_system_once(move |mut boxes: combat::damage_box::DamageBoxSpawner| {
                    boxes.spawn(bob, bob, combat::strike::HitSide::Player, center,
                        combat::strike::DamageBox {
                            cause: combat::events::HitSource::Melee,
                            move_instance: None,
                            half_extent: ae::Vec2::splat(24.0), shape: None, damage: 1,
                            knockback: combat::strike::HitboxKnockback::FeelScale(0.0), lifetime_s: 10.0, name: Some("owned area"),
                        }).expect("the live source emits a box").id()
                }).unwrap()
            } else {
                sim.world_mut().write_message(shot(bob, center, 24.0, 0.0, shots::WorldHitPolicy::Bouncing));
                sim.step(base());
                shot_of(&mut sim, bob, center)
            };
            launched.push((kind, center, attack));
        }
        let splash_origin = home + ae::Vec2::new(60.0, -80.0);
        sim.world_mut().write_message(shot(bob, splash_origin, 4.0, 128.0, shots::WorldHitPolicy::ExpireOnContact));
        sim.step(base());
        let splash = shot_of(&mut sim, bob, splash_origin);
        for (kind, _, attack) in launched.iter().copied().chain([("splash", splash_origin, splash)]) {
            assert_eq!(
                sim.world().get::<combat::allegiance::AttackAllegiance>(attack).expect("capture precedes owner loss").driver,
                Some(DrivingParticipant(PlayerSlot(1))),
                "the {kind}'s launch allegiance names Bob's seat"
            );
        }
        sim.world_mut().entity_mut(splash).insert(shots::FiredByMoveInstance(11));

        // Bob loses control of his body: the box and the shot still land by
        // the rule.
        sim.world_mut().entity_mut(bob).remove::<DrivingParticipant>();
        for (kind, center, _) in &launched {
            fresh_health(&mut sim, alice);
            sim.world_mut().get_mut::<ae::BodyKinematics>(alice).unwrap().pos = *center;
            for _ in 0..2 { sim.step(base()); }
            assert_eq!(damage_taken(&sim, alice) > 0, enabled, "after control loss, the {kind}, friendly fire={enabled}");
        }

        // Then Bob's body leaves the world: the splash still lands by the
        // rule, credited to the shot's move.
        sim.world_mut().despawn(bob);
        fresh_health(&mut sim, alice);
        sim.world_mut().get_mut::<ae::BodyKinematics>(alice).unwrap().pos = home;
        for _ in 0..2 { sim.step(base()); }
        let mut cursor = sim.world().resource::<Messages<combat::HitEvent>>().get_cursor_current();
        sim.world_mut().get_mut::<ae::BodyKinematics>(splash).unwrap().vel = ae::Vec2::new(0.0, 12000.0);
        for _ in 0..2 { sim.step(base()); }
        assert!(sim.world().get_entity(splash).is_err(), "the shot must hit the floor and emit its splash");
        let messages = sim.world().resource::<Messages<combat::HitEvent>>();
        let hits: Vec<_> = cursor.read(messages).filter(|hit|
            hit.attacker == Some(bob) && hit.target == combat::events::HitTarget::Body(alice)
        ).collect();
        assert_eq!(damage_taken(&sim, alice) > 0, enabled, "after owner loss, the splash, friendly fire={enabled}");
        assert_eq!(!hits.is_empty(), enabled, "after owner loss, the splash's hit events, friendly fire={enabled}");
        for hit in hits {
            assert_eq!(hit.source, combat::events::HitSource::Projectile);
            assert_eq!(hit.attacker_move_instance, Some(11));
        }
    }
}

#[test]
fn inventory_bombs_hurt_either_participant_only_in_the_blast_room() {
    use ambition_platformer2d::{abilities::ranged::bomb::BombFuse, held_items::{GroundItem, Release, ReleasedAs}};
    for enabled in [false, true] {
        let mut sim = party(enabled);
        bob_beside_alice(&mut sim, ROOM, Some(PlayerSlot(1)));
        let alice = driven(&mut sim, 0);
        let start = sim.world().get::<ae::BodyKinematics>(alice).unwrap().pos;
        // Then, with friendly fire off, Alice in another room.
        let rooms: &[bool] = if enabled { &[false] } else { &[false, true] };
        for &separate_rooms in rooms {
            if separate_rooms {
                // Back where she stood, the blast's knockback spent, Alice
                // walks out.
                let alice = driven(&mut sim, 0);
                fresh_health(&mut sim, alice);
                crate::a_second_seat_joins_the_session::put_body_at(&mut sim, alice, start);
                for _ in 0..30 { sim.step(base()); }
                assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "central_hub_complex"), "central_hub_complex");
            }
            let alice = driven(&mut sim, 0);
            let bob = driven(&mut sim, 1);
            for body in [alice, bob] {
                fresh_health(&mut sim, body);
                crate::a_second_seat_joins_the_session::put_body_at(&mut sim, body, ae::Vec2::new(400.0, 100.0));
            }
            for _ in 0..2 { sim.step(base()); }
            let room = *sim.world().get::<InRoomInstance>(bob).unwrap();
            let pos = sim.world().get::<ae::BodyKinematics>(bob).unwrap().pos;
            let before = [alice, bob].map(|body| damage_taken(&sim, body));
            let bomb = sim.world_mut().spawn((
                GroundItem::released(
                    ambition_platformer2d::characters::brain::held_item_by_id("bomb").unwrap(),
                    pos, ae::Vec2::ZERO, ae::Vec2::splat(14.0),
                ),
                ReleasedAs(Release::Throw), BombFuse { timer: 0.001 }, room,
            )).id();
            for _ in 0..2 { sim.step(base()); }
            assert!(sim.world().get_entity(bomb).is_err(), "the bomb detonated");
            let messages = sim.world().resource::<Messages<combat::HitEvent>>();
            let mut cursor = messages.get_cursor();
            let hits: Vec<_> = cursor.read(messages).filter(|hit| hit.attacker == Some(bomb)).collect();
            for (index, body) in [alice, bob].into_iter().enumerate() {
                let lands = body == bob || !separate_rooms;
                let after = damage_taken(&sim, body);
                assert_eq!(after > before[index], lands, "seat {index}, friendly fire={enabled}, separate rooms={separate_rooms}: {} -> {after}", before[index]);
                assert_eq!(hits.iter().any(|hit| hit.target == combat::events::HitTarget::Body(body)), lands);
            }
            assert!(hits.iter().all(|hit| hit.source == combat::events::HitSource::Hazard));
            assert!(!sim.world_mut().query::<&combat::strike::Hitbox>().iter(sim.world()).any(|hit| hit.owner == bomb),
                "the blast resolves once after its bomb leaves");
        }
    }
}

#[test]
fn an_environmental_strike_can_hurt_its_credit_body_in_either_seat() {
    use combat::strike::{Hitbox, HitboxAnchor, HitboxHits, HitboxLifetime, HitboxKnockback, HitSide};
    let mut sim = party(false);
    let room = bob_beside_alice(&mut sim, ROOM, Some(PlayerSlot(1)));
    for source_slot in [0, 1] {
        for source in [HitSide::Environment, HitSide::Player] {
            let owner = driven(&mut sim, source_slot);
            let other = driven(&mut sim, 1 - source_slot);
            for body in [owner, other] {
                fresh_health(&mut sim, body);
                crate::a_second_seat_joins_the_session::put_body_at(&mut sim, body, ae::Vec2::new(400.0, 100.0));
            }
            for _ in 0..2 { sim.step(base()); }
            let pos = sim.world().get::<ae::BodyKinematics>(owner).unwrap().pos;
            let before = [owner, other].map(|body| damage_taken(&sim, body));
            sim.world_mut().spawn((
                Hitbox {
                    owner, source, anchor: HitboxAnchor::World { center: pos },
                    half_extent: ae::Vec2::splat(80.0), shape: None, facing: 1.0, damage: 4,
                    knockback: HitboxKnockback::FeelScale(0.0), launch_dir: None,
                    frame_down: ae::DEFAULT_GRAVITY_DIR, reaction: None, strike_sfx: None, guard: Default::default(),
                },
                combat::events::HitSource::Hazard, HitboxHits::default(),
                HitboxLifetime { remaining_s: 0.0 }, InRoomInstance(room),
            ));
            for _ in 0..2 { sim.step(base()); }
            for (index, body) in [owner, other].into_iter().enumerate() {
                let after = damage_taken(&sim, body);
                assert_eq!(after > before[index], source == HitSide::Environment,
                    "source={source:?}, owner seat={source_slot}, victim={index}: {} -> {after}", before[index]);
            }
        }
    }
}
