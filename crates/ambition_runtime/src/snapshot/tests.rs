//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

#[test]
fn the_hasher_is_process_stable_and_order_sensitive() {
    let mut a = StateHasher::default();
    a.write_str("x");
    a.write_u64(7);
    let mut b = StateHasher::default();
    b.write_u64(7);
    b.write_str("x");
    assert_ne!(a.finish(), b.finish(), "order matters");

    // The literal FNV-1a offset basis, so a refactor of the constants is loud.
    assert_eq!(StateHasher::default().finish(), 0xcbf2_9ce4_8422_2325);
}

/// Strings are length-delimited, so `"ab" + "c"` and `"a" + "bc"` differ. A
/// hash that confused two entity ids would report sync across a real desync.
#[test]
fn string_writes_are_length_delimited() {
    let mut a = StateHasher::default();
    a.write_str("ab");
    a.write_str("c");
    let mut b = StateHasher::default();
    b.write_str("a");
    b.write_str("bc");
    assert_ne!(a.finish(), b.finish());
}

/// Every NaN hashes alike (two sims that both blew up agree), but `-0.0` and
/// `0.0` do not: in a physics sim a body resting at `-0.0` velocity has been
/// pushed, and that is a state difference worth catching.
#[test]
fn nan_is_canonical_but_negative_zero_is_not() {
    let hash = |v: f32| {
        let mut h = StateHasher::default();
        h.write_f32(v);
        h.finish()
    };
    assert_eq!(hash(f32::NAN), hash(-f32::NAN));
    assert_eq!(hash(f32::NAN), hash(0.0 / 0.0));
    assert_ne!(hash(0.0), hash(-0.0));
}

/// **The reason `hash_entities_by_key` exists.** Bevy's query order follows
/// archetype layout; two sims can walk the same entities in different orders.
/// A hash that noticed would cry desync on every run.
#[test]
fn entity_rows_hash_the_same_however_the_query_walked_them() {
    let rows = |order: [usize; 3]| {
        let all = [
            ("b".to_string(), vec![2u8]),
            ("a".to_string(), vec![1u8]),
            ("c".to_string(), vec![3u8]),
        ];
        let mut h = StateHasher::default();
        hash_entities_by_key(
            h_mut(&mut h),
            order.iter().map(|i| all[*i].clone()).collect(),
        );
        h.finish()
    };
    fn h_mut(h: &mut StateHasher) -> &mut StateHasher {
        h
    }
    assert_eq!(rows([0, 1, 2]), rows([2, 1, 0]));
    assert_eq!(rows([0, 1, 2]), rows([1, 2, 0]));
}

/// ...but the row COUNT is hashed, so an entity that failed to spawn in one
/// sim is a divergence rather than a shrug.
#[test]
fn a_missing_entity_changes_the_hash() {
    let mut a = StateHasher::default();
    hash_entities_by_key(&mut a, vec![("x".into(), vec![1]), ("y".into(), vec![2])]);
    let mut b = StateHasher::default();
    hash_entities_by_key(&mut b, vec![("x".into(), vec![1])]);
    assert_ne!(a.finish(), b.finish());
}

#[test]
fn identical_streams_are_in_sync() {
    let r = compare_hash_streams(&[1, 2, 3], &[1, 2, 3]);
    assert!(r.in_sync());
    assert_eq!(r.ticks_compared, 3);
}

#[test]
fn the_report_names_the_first_divergent_tick_and_not_the_last() {
    let r = compare_hash_streams(&[1, 2, 9, 9], &[1, 2, 3, 4]);
    assert_eq!(r.first_divergence_tick, Some(2));
}

/// A sim that stopped early did not agree; it stopped.
#[test]
fn a_short_stream_diverges_at_its_own_end() {
    let r = compare_hash_streams(&[1, 2], &[1, 2, 3]);
    assert_eq!(r.first_divergence_tick, Some(2));
    assert!(!r.in_sync());
}

#[test]
fn a_registry_hashes_its_entry_names_so_two_registries_never_agree_by_luck() {
    let world = World::new();
    let mut a = SnapshotRegistry::default();
    a.register_diagnostic("alpha", |_, h| h.write_u64(1));
    let mut b = SnapshotRegistry::default();
    b.register_diagnostic("beta", |_, h| h.write_u64(1));
    assert_ne!(a.hash_world(&world), b.hash_world(&world));
    assert_eq!(a.len(), 1);
    assert_eq!(a.names().collect::<Vec<_>>(), ["alpha"]);
}

#[test]
fn per_entry_hashes_localize_a_divergence() {
    let world = World::new();
    let mut reg = SnapshotRegistry::default();
    reg.register_diagnostic("a", |_, h| h.write_u64(1));
    reg.register_diagnostic("b", |_, h| h.write_u64(2));
    let by_entry = reg.hash_by_entry(&world);
    // Two registered diagnostics, plus the active-room (finding 2) and identity-roster
    // (finding 1) pseudo-entries.
    assert_eq!(by_entry.len(), 4);
    assert_eq!(by_entry[0].0, "a");
    assert_ne!(by_entry[0].1, by_entry[1].1);
    assert_eq!(by_entry[2].0, SnapshotRegistry::ACTIVE_ROOM_ENTRY);
    assert_eq!(by_entry[3].0, SnapshotRegistry::ROSTER_ENTRY);
}

// ── N3.1: take / restore ─────────────────────────────────────────────────

use ambition_characters::actor::{BodyHealth, Health};
use bevy::math::Vec2;

/// A component nothing registers and nothing declares derived. It stands in for
/// every un-migrated piece of sim state: a brain, a cooldown, a portal's transit
/// latch. `restore` destroys it, and the report must SAY so.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
struct UnregisteredThing(u32);

/// A component `restore` is allowed to destroy, because the system that
/// maintains it rebuilds it every tick.
#[derive(Component)]
struct DerivedThing;

/// A component that is half authored content and half mutable cursor — the
/// `ActorMotionPath` shape, in miniature.
#[derive(Component, Debug, PartialEq)]
struct Patrol {
    /// Authored. Never in a blob.
    waypoints: Vec<f32>,
    /// Mutable. The only thing a rollback touches.
    segment: u32,
}

impl SnapshotCursor for Patrol {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        put_u32(out, self.segment);
    }
    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        self.segment = r.u32()?;
        Some(())
    }
}

/// A resource that is half authored geometry, half mutable assignment — the
/// `CombatSlotsRes` shape, in miniature. Its cursor carries only the mutable half.
#[derive(bevy::ecs::resource::Resource, Debug, PartialEq)]
struct TestBoard {
    /// Authored. Never in the cursor.
    slots: u32,
    /// Mutable. The only thing the cursor rewinds.
    assigned: u32,
}

impl SnapshotCursor for TestBoard {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        put_u32(out, self.assigned);
    }
    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        self.assigned = r.u32()?;
        Some(())
    }
}

fn kin(pos: Vec2, vel: Vec2) -> BodyKinematics {
    BodyKinematics {
        pos,
        vel,
        size: Vec2::new(16.0, 32.0),
        facing: 1.0,
    }
}

fn engine_registry() -> SnapshotRegistry {
    let mut reg = SnapshotRegistry::default();
    register_engine_sim_state(&mut reg);
    reg
}

fn sim_world() -> World {
    let mut world = World::new();
    world.insert_resource(ambition_time::SimTick(11));
    world.insert_resource(ambition_time::WorldTime {
        raw_dt: 1.0 / 60.0,
        scaled_dt: 1.0 / 60.0,
    });
    world.spawn((
        SimId::placement("boss-1"),
        SimIdCounter(3),
        kin(Vec2::new(10.0, -4.5), Vec2::new(0.0, 120.0)),
        BodyHealth::new(Health {
            current: 40,
            max: 100,
            invulnerable: false,
        }),
    ));
    world.spawn((
        SimId::player_slot(0),
        SimIdCounter(0),
        kin(Vec2::new(-3.0, 0.0), Vec2::ZERO),
    ));
    world
}

/// **Every codec round-trips.** A field an encoder writes and a decoder skips is
/// a restore that rewinds to a *different* world — quietly, and only in the
/// field nobody encoded. `Reader::finish` refuses leftover bytes, so this also
/// catches a decoder that reads too little.
///
/// The property is `encode ∘ decode ∘ encode == encode`, asserted on BYTES
/// rather than on values: a decoder that drops a field re-encodes a default in
/// its place, and not every sim type wants a `PartialEq` it does not otherwise
/// need just so a test can look at it.
#[test]
fn every_engine_codec_round_trips_exactly() {
    fn round_trip<T: SnapshotState>(v: T) {
        let bytes = encode_one(&v);
        let back = decode_one::<T>(&bytes).expect("decodes");
        assert_eq!(
            encode_one(&back),
            bytes,
            "{} lost a field",
            std::any::type_name::<T>()
        );
    }
    round_trip(ambition_time::SimTick(9_000_000_001));
    round_trip(ambition_time::WorldTime {
        raw_dt: 0.016,
        scaled_dt: -0.0,
    });
    round_trip(kin(Vec2::new(1.5, -2.25), Vec2::new(-0.0, 7.0)));
    round_trip(BodyHealth::new(Health {
        current: -3,
        max: 250,
        invulnerable: true,
    }));
    round_trip(SimIdCounter(u64::MAX));
    let mut ability_pattern = ambition_engine_core::AbilitySet::basic();
    ability_pattern.move_horizontal = false;
    ability_pattern.jump = true;
    ability_pattern.variable_jump = false;
    ability_pattern.double_jump = true;
    ability_pattern.fast_fall = false;
    ability_pattern.wall_jump = true;
    ability_pattern.wall_cling = false;
    ability_pattern.wall_climb = true;
    ability_pattern.dash = false;
    ability_pattern.double_dash = true;
    ability_pattern.fly = false;
    ability_pattern.blink = true;
    ability_pattern.precision_blink = false;
    ability_pattern.blink_through_soft_walls = true;
    ability_pattern.blink_through_hard_walls = false;
    ability_pattern.attack = true;
    ability_pattern.pogo = false;
    ability_pattern.directional_primary = true;
    ability_pattern.directional_special = false;
    ability_pattern.rebound = true;
    ability_pattern.reset = false;
    ability_pattern.ledge_grab = true;
    ability_pattern.swim = false;
    ability_pattern.glide = true;
    ability_pattern.dodge = false;
    ability_pattern.shield = true;
    let ability_bytes = encode_one(&ability_pattern);
    let ability_back =
        decode_one::<ambition_engine_core::AbilitySet>(&ability_bytes).expect("AbilitySet decodes");
    assert_eq!(ability_back, ability_pattern, "every ability flag survives");
    round_trip(bc::BodyAbilities::new(ability_pattern));

    // The body-state clusters. `snapshot_pod!` writes these codecs from a field
    // list, so the risk is a field OMITTED from the list, not a field mistyped —
    // and an omitted field is exactly what `encode ∘ decode ∘ encode` cannot see.
    // `every_registered_component_survives_a_world_round_trip` below is the one
    // that catches it, by comparing hashes of a world rather than of a value.
    round_trip(bc::BodyGroundState { on_ground: true });
    round_trip(bc::BodyWallState {
        on_wall: true,
        wall_normal_x: -1.0,
    });
    round_trip(bc::BodyJumpState {
        air_jumps_available: 2,
        ladder_jump_boost: 1.5,
        ladder_drop_through_timer: 0.0,
        ladder_drop_through_hold_lock: true,
    });
    round_trip(bc::BodyDashState {
        charges_available: 255,
        cooldown: 0.3,
    });
    round_trip(bc::BodyFlightState {
        fly_enabled: true,
        carried_run: -12.0,
    });
    round_trip(bc::BodyBlinkState { cooldown: 1.0 });
    round_trip(bc::BodyDodgeState { cooldown: 0.9 });
    round_trip(bc::BodyShieldState {
        active: true,
        parry_window_timer: 0.08,
    });
    round_trip(bc::BodyOffense {
        damage_multiplier: -2,
        invincible: true,
    });
    round_trip(bc::BodyLifetime {
        time_alive: 99.5,
        resets: u32::MAX,
        max_speed: 1200.0,
    });
    round_trip(bc::BodyActionBuffer {
        attack: 0.3,
        pogo: 0.4,
        projectile: 0.5,
    });
    // The axis policy's PRIVATE maneuver state rides inside the MotionModel
    // codec (ADR 0024 O4): every field — timers, buffers, blink telegraph,
    // the ledge hang state machine — must survive the round trip.
    {
        use ambition_engine_core::ledge_grab::{LedgeContact, LedgeGrabState};
        use ambition_engine_core::{AxisManeuverState, AxisSweptMotion, MotionModel};
        let mut grab = LedgeGrabState::hanging(LedgeContact {
            wall_normal_x: -1.0,
            anchor: Vec2::new(86.0, 110.0),
            climb_target: Vec2::new(115.0, 77.0),
        });
        grab.elapsed = 0.3;
        grab.climbing = true;
        grab.momentum_at_grab = Vec2::new(400.0, -50.0);
        round_trip(MotionModel::AxisSwept(AxisSweptMotion {
            params: Default::default(),
            state: AxisManeuverState {
                coyote_timer: 0.1,
                drop_through_timer: -0.0,
                rebound_cooldown: 3.0,
                wall_clinging: true,
                wall_climbing: false,
                pre_wall_vel: Vec2::new(1.0, 2.0),
                pre_wall_vel_age: 0.5,
                buffer_jump: 0.1,
                buffer_dash: 0.2,
                buffer_blink: 0.6,
                dash_timer: 0.2,
                blink_hold_active: true,
                blink_hold_timer: 0.4,
                blink_aiming: true,
                blink_aim_offset: Vec2::new(-3.0, 4.0),
                blink_grace_timer: 0.05,
                dodge_roll_timer: 0.1,
                ledge_grab: Some(grab),
                gliding: true,
                fast_falling: false,
                flight_phase: 6.28,
            },
        }));
    }
    round_trip(bc::BodyBaseSize {
        base_size: Vec2::new(16.0, 32.0),
    });
    round_trip(bc::SweepSample {
        prev: Vec2::new(1.0, 2.0),
        curr: Vec2::new(3.0, 4.0),
        vel: Vec2::new(5.0, 6.0),
        half: Vec2::new(7.0, 8.0),
    });
    round_trip(ambition_characters::actor::pose::ActorPose {
        center: Vec2::new(1.0, 2.0),
        feet: Vec2::new(1.0, 18.0),
        facing: -1.0,
    });
    round_trip(ambition_platformer_primitives::orientation::ActorRoll { angle: 1.57 });
    round_trip(ambition_combat::components::ActorCooldowns {
        attack_cooldown: 0.4,
        respawn_timer: 2.0,
    });
    round_trip(ambition_engine_core::geometry::CenteredAabb {
        center: Vec2::new(5.0, 6.0),
        half_size: Vec2::new(8.0, 16.0),
    });
    {
        use ambition_actors::features::ecs::perception::{Perception, PerceptionMemory};
        use ambition_characters::actor::ActorFaction;
        use ambition_characters::brain::boss_pattern::{
            BossAttackIntent, BossAttackProfile, BossAttackState, TelegraphSpec,
        };
        use ambition_characters::perception::{RememberedActor, WorldMemory};

        round_trip(BossAttackProfile::Strike("floor_slam".into()));
        round_trip(BossAttackProfile::Special("overfit_volley".into()));
        round_trip(BossAttackIntent {
            telegraph_profile: Some(BossAttackProfile::Strike("side_sweep".into())),
            active_profile: None,
        });
        round_trip(BossAttackState {
            telegraph_profile: None,
            telegraph_remaining: 0.4,
            telegraph_elapsed: 0.1,
            telegraph_spec: Some(TelegraphSpec {
                pose: Some("wind_up".into()),
                cue: None,
                vfx: Some("sparks".into()),
            }),
            active_profile: Some(BossAttackProfile::Special("apple_rain".into())),
            active_remaining: 1.25,
            active_elapsed: -0.0,
        });
        round_trip(Perception::Omniscient);
        round_trip(Perception::Sighted {
            viewport_half: Vec2::new(320.0, 180.0),
        });
        round_trip(PerceptionMemory(WorldMemory::from_snapshot([
            (
                "zeta".to_string(),
                RememberedActor {
                    pos: Vec2::new(1.0, 2.0),
                    vel: Vec2::new(-3.0, 0.0),
                    faction: ActorFaction::Player,
                    hostile_to_self: true,
                    last_seen: 9.5,
                    confidence: 0.75,
                },
            ),
            (
                "alpha".to_string(),
                RememberedActor {
                    pos: Vec2::ZERO,
                    vel: Vec2::ZERO,
                    faction: ActorFaction::Neutral,
                    hostile_to_self: false,
                    last_seen: 0.0,
                    confidence: 1.0,
                },
            ),
        ])));
    }

    round_trip(bc::BodyMana {
        meter: ambition_engine_core::player_state::ResourceMeter {
            current: 12.0,
            max: 50.0,
            regen_rate: 1.0,
            decay_rate: -0.0,
        },
    });
}

/// **The test `encode ∘ decode ∘ encode` cannot be: a field left out of the
/// codec entirely.**
///
/// A codec that never touches `coyote_timer` round-trips its own bytes perfectly
/// and loses the timer. So: put a world in a known state, snapshot it, wreck
/// EVERY registered component, restore, and demand the world hash come back. The
/// hash reads the components through the same codecs — so this catches a field
/// dropped from `snapshot_pod!`'s list only if the hash sees it too, which is
/// the honest limit of "one serialization, two consumers".
///
/// The unlosable half is the field that MOVES something: a dropped `coyote_timer`
/// changes what the next jump does, and
/// `a_restored_sim_replays_the_future_it_was_rewound_from` in `ambition_app` is
/// the test that runs the sim forward and notices.
#[test]
fn every_registered_component_survives_a_world_round_trip() {
    let reg = engine_registry();
    let mut world = sim_world();
    let id = *live_ids(&mut world).get("placement:boss-1").unwrap();
    // The grace timers / active-dash countdown live INSIDE the model variant
    // now (ADR 0024 O4); the clusters keep the contact fact and the resources.
    let mut model = ambition_engine_core::MotionModel::default();
    if let ambition_engine_core::MotionModel::AxisSwept(axis) = &mut model {
        axis.state.coyote_timer = 0.125;
        axis.state.drop_through_timer = 0.25;
        axis.state.rebound_cooldown = 0.5;
        axis.state.dash_timer = 0.75;
    }
    world.entity_mut(id).insert((
        bc::BodyGroundState { on_ground: true },
        bc::BodyDashState {
            charges_available: 3,
            cooldown: 1.5,
        },
        bc::BodyAbilities::new(ambition_engine_core::AbilitySet::sandbox_all()),
        model,
    ));
    let before = reg.hash_world(&world);
    let snap = take(&world, &reg);

    world.entity_mut(id).insert((
        bc::BodyGroundState::default(),
        bc::BodyDashState::default(),
        bc::BodyAbilities::new(ambition_engine_core::AbilitySet::basic()),
        ambition_engine_core::MotionModel::default(),
    ));
    assert_ne!(reg.hash_world(&world), before);

    restore(&mut world, &snap, &reg).unwrap();
    assert_eq!(reg.hash_world(&world), before);
    let ambition_engine_core::MotionModel::AxisSwept(axis) = world
        .entity(id)
        .get::<ambition_engine_core::MotionModel>()
        .unwrap()
    else {
        panic!("restore must bring back the axis policy");
    };
    assert_eq!(axis.state.coyote_timer, 0.125, "the timer came back");
    assert_eq!(
        world
            .entity(id)
            .get::<bc::BodyAbilities>()
            .unwrap()
            .abilities,
        ambition_engine_core::AbilitySet::sandbox_all(),
        "the host-kit derivation input came back"
    );
}

/// Restoring a host-code persona restores BOTH the identity and the ability set
/// it derives from, then the ordinary Changed<> system reconstructs the same kit.
/// This is the regression the generic codec round-trip cannot express: changing
/// abilities after capture must not influence the restored protagonist profile.
#[test]
fn restoring_worn_host_code_rebuilds_from_the_snapshotted_abilities() {
    use ambition_actors::combat::moveset::ActorMoveset;
    use ambition_characters::actor::WornCharacter;
    use ambition_characters::brain::{ActionSet, MeleeActionSpec, RangedActionSpec};
    use bevy::prelude::*;

    ambition_actors::character_roster::install_character_catalog(include_str!(
        "../../../../game/ambition_content/assets/data/character_catalog.ron"
    ));
    ambition_actors::character_roster::install_default_character_id("player");

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // The App-local catalog resource the worn-character system reads (the
    // process-global install above only stages the RON for this plugin).
    app.add_plugins(ambition_actors::character_roster::character_roster_plugin());
    app.add_systems(
        Update,
        ambition_actors::avatar::apply_worn_character_gameplay,
    );
    let entity = app
        .world_mut()
        .spawn((
            SimId::player_slot(0),
            WornCharacter::new("player"),
            Name::new("unset"),
            ActionSet::default(),
            ActorMoveset(Default::default()),
            kin(Vec2::ZERO, Vec2::ZERO),
            // A worn body is a FULL body: the movement clusters + one explicit
            // policy from spawn; the worn-character system refreshes movement
            // identity through the one transition seam.
            ambition_actors::actor::AncillaryMovementBundle::from_scratch(
                ambition_engine_core::BodyClusterScratch::new_with_abilities(
                    Vec2::ZERO,
                    ambition_engine_core::AbilitySet::sandbox_all(),
                ),
            ),
            ambition_engine_core::MotionModel::default(),
        ))
        .id();
    app.update();

    let mut reg = SnapshotRegistry::default();
    reg.register_component::<WornCharacter>("worn_character");
    reg.register_component::<bc::BodyAbilities>("body_abilities");
    let snap = take(app.world(), &reg);

    let mut reduced = ambition_engine_core::AbilitySet::basic();
    reduced.attack = false;
    reduced.shield = false;
    app.world_mut().entity_mut(entity).insert((
        WornCharacter::new("npc_pirate_admiral"),
        bc::BodyAbilities::new(reduced),
    ));
    app.update();
    assert!(matches!(
        app.world().get::<ActionSet>(entity).unwrap().ranged,
        Some(RangedActionSpec::Pistol { .. })
    ));

    restore(app.world_mut(), &snap, &reg).expect("restore succeeds");
    app.update();

    let restored_abilities = app
        .world()
        .get::<bc::BodyAbilities>(entity)
        .unwrap()
        .abilities;
    assert_eq!(
        restored_abilities,
        ambition_engine_core::AbilitySet::sandbox_all()
    );
    let restored = app.world().get::<ActionSet>(entity).unwrap();
    assert!(matches!(restored.melee, Some(MeleeActionSpec::Swipe(_))));
    assert!(matches!(
        restored.ranged,
        Some(RangedActionSpec::Bolt { .. })
    ));
}

/// A truncated blob decodes to `None` rather than to a plausible lie.
#[test]
fn a_short_blob_is_rejected_rather_than_guessed() {
    let bytes = encode_one(&kin(Vec2::ONE, Vec2::ZERO));
    assert!(decode_one::<BodyKinematics>(&bytes[..bytes.len() - 1]).is_none());
    let mut too_long = bytes.clone();
    too_long.push(0);
    assert!(
        decode_one::<BodyKinematics>(&too_long).is_none(),
        "leftover bytes mean the decoder disagreed with the encoder"
    );
}

/// **The oracle for the whole slice.** Take, wreck the world, restore, and the
/// registered state hashes to what it hashed before. This is the property N0.4
/// and FB6 both actually need, and it is one assertion.
#[test]
fn a_restored_world_hashes_exactly_as_the_taken_one_did() {
    let reg = engine_registry();
    let mut world = sim_world();
    let before = reg.hash_world(&world);
    let snap = take(&world, &reg);

    // Advance the sim, badly: move a body, hurt it, kill the other, spawn a
    // third, wind the clock.
    let boss = world
        .try_query_filtered::<Entity, With<SimId>>()
        .unwrap()
        .iter(&world)
        .next()
        .unwrap();
    world
        .entity_mut(boss)
        .insert(kin(Vec2::splat(999.0), Vec2::ZERO));
    world.insert_resource(ambition_time::SimTick(50));
    world.spawn((
        SimId::spawned(&SimId::player_slot(0), 1),
        kin(Vec2::ZERO, Vec2::ZERO),
    ));

    assert_ne!(reg.hash_world(&world), before, "the wreck must be visible");
    let report = restore(&mut world, &snap, &reg).unwrap();
    assert_eq!(reg.hash_world(&world), before, "restore did not restore");
    assert_eq!(report.patched, 2, "both snapshot entities were still there");
    assert_eq!(report.despawned, 1, "the body spawned after the snapshot");
    assert_eq!(report.respawned, 0);
}

/// An entity spawned after the snapshot ceases to exist; one despawned since is
/// recreated. Both fall out of "the snapshot is the truth", not out of a diff.
#[test]
fn restore_forgets_the_future_and_remembers_the_dead() {
    let reg = engine_registry();
    let mut world = sim_world();
    let snap = take(&world, &reg);
    assert_eq!(snap.sim_ids(), ["placement:boss-1", "slot:0"]);

    let doomed: Vec<Entity> = world
        .try_query_filtered::<Entity, With<SimId>>()
        .unwrap()
        .iter(&world)
        .collect();
    world.despawn(doomed[0]);
    world.spawn((SimId::placement("ghost"), kin(Vec2::ZERO, Vec2::ZERO)));

    let report = restore(&mut world, &snap, &reg).unwrap();
    assert_eq!(report.despawned, 1, "the ghost");
    assert_eq!(report.respawned, 1, "the one we killed");
    assert_eq!(report.patched, 1, "the survivor was patched, not rebuilt");

    let ids: Vec<String> = world
        .try_query::<&SimId>()
        .unwrap()
        .iter(&world)
        .map(|id| id.as_str().to_string())
        .collect();
    assert!(
        ids.contains(&"placement:boss-1".to_string()),
        "the dead came back"
    );
    assert!(
        !ids.contains(&"ghost".to_string()),
        "the future was forgotten"
    );
}

/// **Identity is unique, and restore refuses a world where it is not** (audit H2).
///
/// Two live entities carrying one `SimId` make every by-id lookup pick one at
/// random — the silent corruption the old "later wins" map delegated upstream.
/// `duplicate_live_ids` names the collision, and `restore` refuses (panics in every
/// build) rather than patch an arbitrary one. Poison test for the identity
/// invariant, in the same commit as the enforcement (poison-test atomicity rule).
#[test]
fn restore_refuses_a_world_with_two_entities_of_one_identity() {
    let reg = engine_registry();
    let mut world = sim_world();
    let snap = take(&world, &reg);

    // A SECOND entity claims an id that already exists.
    world.spawn((SimId::placement("boss-1"), kin(Vec2::ZERO, Vec2::ZERO)));

    // The detector names the collision precisely...
    assert_eq!(
        duplicate_live_ids(&mut world),
        vec![("placement:boss-1".to_string(), 2)],
        "the identity-roster check must name the duplicated id and its count"
    );

    // ...and restore refuses rather than corrupt. Suppress the backtrace: this
    // panic is expected, and an alarming trace on a passing test is noise.
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        restore(&mut world, &snap, &reg)
    }));
    std::panic::set_hook(prev);
    assert!(
        refused.is_err(),
        "restore must refuse a duplicated identity, not silently patch one of the two"
    );
}

/// The snapshot's own roster must be unambiguous: `duplicate_ids` surfaces the
/// duplicates `sim_ids` dedups away, so restore can refuse a malformed snapshot — even
/// a collision that shares NO registered component row, which a per-entry scan misses
/// (re-audit finding 3).
#[test]
fn a_snapshot_roster_surfaces_its_duplicate_ids() {
    let reg = engine_registry();
    let world = sim_world();
    let good = take(&world, &reg);
    assert!(
        good.duplicate_ids().is_empty(),
        "a snapshot of a unique-identity world has no duplicate rows"
    );

    // Forge the collision a per-component scan is blind to: one id twice in the full
    // roster, sharing no component row. The roster sees identity regardless of which
    // (or how few) components an entity carried.
    let mut malformed = good.clone();
    malformed.roster.push("placement:ghost-dup".to_string());
    malformed.roster.push("placement:ghost-dup".to_string());
    malformed.roster.sort();
    assert_eq!(
        malformed.duplicate_ids(),
        vec!["placement:ghost-dup".to_string()],
        "duplicate_ids must catch a collision that carries no shared component row"
    );
}

/// **A zero-component `SimId` entity is snapshot state the roster makes authoritative**
/// (re-audit finding 1).
///
/// It appears in no component entry, so a restore driven by the old component-derived id
/// set was blind to it: it despawned the entity if it survived, and dropped it if it had
/// died. Driven by the roster, restore preserves it (a), reconstructs it (b), despawns a
/// future one (c) — and the state hash now sees it at all.
#[test]
fn a_zero_component_sim_id_entity_is_covered_by_the_roster() {
    let reg = engine_registry();
    let mut world = sim_world();

    let before = reg.hash_world(&world);
    // An entity with an identity and NOTHING the registry knows — the case a
    // per-component id set cannot see.
    world.spawn(SimId::placement("ghost"));
    assert_ne!(
        reg.hash_world(&world),
        before,
        "the state hash is blind to a zero-component identity — finding 1's roster term \
         is missing"
    );

    // (a) It SURVIVES a restore that snapshotted it: not mistaken for a future birth.
    let with_ghost = take(&world, &reg);
    assert!(
        with_ghost.sim_ids().contains(&"placement:ghost"),
        "the snapshot's authoritative id set must include the zero-component entity"
    );
    restore(&mut world, &with_ghost, &reg).unwrap();
    assert!(
        live_ids(&mut world).contains_key("placement:ghost"),
        "restore despawned a zero-component survivor it had snapshotted"
    );

    // (b) It is RECONSTRUCTED when it died inside the window (bare — no room authors it).
    let ghost = *live_ids(&mut world).get("placement:ghost").unwrap();
    world.despawn(ghost);
    let report = restore(&mut world, &with_ghost, &reg).unwrap();
    assert!(
        live_ids(&mut world).contains_key("placement:ghost"),
        "restore did not reconstruct a zero-component entity that died inside the window"
    );
    assert_eq!(report.respawned, 1, "the reconstruction must be reported");

    // (c) A live one the snapshot never knew is despawned as a future birth.
    let without_ghost = take(&sim_world(), &reg);
    restore(&mut world, &without_ghost, &reg).unwrap();
    assert!(
        !live_ids(&mut world).contains_key("placement:ghost"),
        "restore kept a zero-component entity that was born after the snapshot"
    );
}

/// **Restore validates the snapshot roster independently of the live world** (re-audit
/// findings 2 + 3). `take` enforces uniqueness at capture, but a snapshot arriving over
/// the N3.3 wire was never take-validated. Restore refuses with a RETURNED
/// `MalformedSnapshot` (corrupt input) — not a panic (reserved for a live-identity bug) —
/// rather than pick one of the colliding rows, and its dup detection no longer trusts the
/// caller to have sorted first.
#[test]
fn restore_refuses_a_snapshot_whose_roster_is_ambiguous() {
    let reg = engine_registry();
    let mut world = sim_world();

    // `duplicate_ids` no longer trusts the caller to have sorted: the audit's split
    // duplicate in an UNSORTED roster (`["dup", "other", "dup"]`) is still detected
    // (re-audit finding 2), where an adjacent-only scan of the stored order would miss it.
    let mut probe = take(&world, &reg);
    probe.roster = vec![
        "placement:ghost-dup".into(),
        "placement:other".into(),
        "placement:ghost-dup".into(),
    ];
    assert_eq!(
        probe.duplicate_ids(),
        vec!["placement:ghost-dup".to_string()],
        "duplicate_ids must detect a non-adjacent collision in an unsorted roster"
    );

    // The live world is clean; only the (deserialized) SNAPSHOT is corrupt. Restore
    // refuses it as a returned error, having touched nothing.
    let mut snap = take(&world, &reg);
    snap.roster.push("placement:ghost-dup".into());
    snap.roster.push("placement:ghost-dup".into());
    snap.roster.sort();
    match restore(&mut world, &snap, &reg) {
        Err(RestoreError::MalformedSnapshot { .. }) => {}
        other => panic!("restore accepted an ambiguous roster instead of refusing: {other:?}"),
    }
}

/// **A snapshot entry whose blob kind disagrees with the registry is refused**
/// (re-audit finding 2). A component blob arriving under a resource entry — the sort of
/// corruption a wire format admits — used to be silently SKIPPED by restore's
/// `find(..).kind` match. `validate_snapshot` catches it before any mutation.
#[test]
fn restore_refuses_a_snapshot_with_a_kind_mismatched_entry() {
    let reg = engine_registry();
    let mut world = sim_world();
    let mut snap = take(&world, &reg);

    // Find a resource entry and corrupt its blob into a component blob.
    let (name, slot) = snap
        .entries
        .iter_mut()
        .find_map(|(n, b)| matches!(b, EntryBlob::Resource(_)).then_some((*n, b)))
        .expect("the engine registry has at least one resource entry");
    *slot = EntryBlob::Component(Vec::new());
    let _ = name;

    match restore(&mut world, &snap, &reg) {
        Err(RestoreError::MalformedSnapshot { .. }) => {}
        other => {
            panic!("restore accepted a kind-mismatched entry instead of refusing: {other:?}")
        }
    }
}

/// **A reordered snapshot is refused** (third-pass re-audit). `restore` iterates
/// `snapshot.entries` directly, so a permuted deserialized snapshot is operationally
/// significant (a resolved codec could resolve before a registered dependency is applied).
/// `validate_snapshot` now requires the exact registry order.
#[test]
fn a_reordered_snapshot_is_rejected() {
    let reg = engine_registry();
    let mut world = sim_world();
    let mut snap = take(&world, &reg);
    assert!(snap.entries.len() >= 2, "need two entries to reorder");
    snap.entries.swap(0, 1);
    match restore(&mut world, &snap, &reg) {
        Err(RestoreError::MalformedSnapshot { .. }) => {}
        other => panic!("restore accepted a reordered snapshot instead of refusing: {other:?}"),
    }
}

/// **A resource-cursor absence blob rejects trailing bytes and a non-canonical tag**
/// (third-pass re-audit). The `false` (absent) path used to remove the resource and report
/// success WITHOUT exhausting the reader, and `Reader::bool` accepted any nonzero byte as
/// `true`. Both are corruption a decoder must refuse.
#[test]
fn a_resource_cursor_absence_blob_rejects_trailing_bytes_and_a_bad_tag() {
    let mut reg = engine_registry();
    reg.register_resource_cursor::<TestBoard>("test_board");

    fn restore_with_board_blob(
        reg: &SnapshotRegistry,
        blob: Vec<u8>,
    ) -> Result<RestoreReport, RestoreError> {
        let mut world = sim_world(); // TestBoard absent -> tagged `false`
        let mut snap = take(&world, reg);
        for (name, b) in snap.entries.iter_mut() {
            if *name == "test_board" {
                *b = EntryBlob::Resource(blob.clone());
            }
        }
        restore(&mut world, &snap, reg)
    }

    // Absence tag (0) + trailing bytes: corruption, not a clean removal.
    assert!(
        matches!(
            restore_with_board_blob(&reg, vec![0, 0xAB]),
            Err(RestoreError::DecodeFailed { .. })
        ),
        "trailing bytes after an absence tag were accepted"
    );
    // A non-canonical presence tag (2) is not `true`.
    assert!(
        matches!(
            restore_with_board_blob(&reg, vec![2]),
            Err(RestoreError::DecodeFailed { .. })
        ),
        "a non-canonical presence tag (2) was accepted as `true`"
    );
    // Sanity: the clean absence blob (just the tag) still applies.
    assert!(
        restore_with_board_blob(&reg, vec![0]).is_ok(),
        "a clean absence tag must still remove-and-apply"
    );
}

/// **A resolved blob with trailing garbage is refused** (third-pass re-audit). `resolve`
/// reads only a prefix and cannot itself assert the whole blob was consumed (it holds only
/// `&mut Reader`); the insert closure now checks `finish()` after a `Some`, so a valid
/// prefix followed by bytes nobody wrote is `DecodeFailed`, not a silent success.
#[test]
fn a_resolved_blob_with_trailing_bytes_is_rejected() {
    let mut reg = engine_registry();
    reg.register_resolved::<Playing>("playing");
    let mut world = sim_world();
    let boss = *live_ids(&mut world).get("placement:boss-1").unwrap();
    world.entity_mut(boss).insert((
        Catalog(vec![("smash".into(), 20.0)]),
        Playing {
            power: 20.0,
            id: "smash".into(),
            t: 0.25,
        },
    ));
    let mut snap = take(&world, &reg);

    // Append a byte nobody encoded to the boss's `playing` row.
    for (name, blob) in snap.entries.iter_mut() {
        if *name == "playing" {
            if let EntryBlob::Component(rows) = blob {
                for (id, bytes) in rows.iter_mut() {
                    if id == "placement:boss-1" {
                        bytes.push(0xFF);
                    }
                }
            }
        }
    }
    match restore(&mut world, &snap, &reg) {
        Err(RestoreError::DecodeFailed { entry, .. }) => assert_eq!(entry, "playing"),
        other => panic!("a resolved blob with trailing garbage was accepted: {other:?}"),
    }
}

/// **A truncated resolved blob is a DECODE FAILURE, distinct from absent content**
/// (the resolved-codec `resolve -> Result` residual, now CLOSED). A blob missing its
/// trailing bytes makes a `Reader` primitive return `None` mid-decode — a corrupt
/// wire input, `DecodeFailed` (which aborts the restore), NOT the content-change
/// `Unapplied` a legitimately vanished authored half earns (see
/// `a_resolved_component_that_names_missing_content_is_dropped_and_denies_lossless`).
/// Before `resolve` returned `Result<Option<_>, ResolveDecodeError>`, both a truncated
/// blob and absent content mapped to `None`, so a corrupt blob was silently laundered
/// as a content change. The catalog here is PRESENT — the truncation is detected
/// regardless, because `resolve` decodes the whole blob before it looks up content.
#[test]
fn a_truncated_resolved_blob_is_a_decode_failure_not_a_content_change() {
    let mut reg = engine_registry();
    reg.register_resolved::<Playing>("playing");
    let mut world = sim_world();
    let boss = *live_ids(&mut world).get("placement:boss-1").unwrap();
    world.entity_mut(boss).insert((
        Catalog(vec![("smash".into(), 20.0)]),
        Playing {
            power: 20.0,
            id: "smash".into(),
            t: 0.25,
        },
    ));
    let mut snap = take(&world, &reg);

    // Drop the trailing `t` f32 (4 bytes) from the boss's `playing` row: `id` still
    // decodes, but reading `t` hits the end of the buffer — a truncated blob.
    for (name, blob) in snap.entries.iter_mut() {
        if *name == "playing" {
            if let EntryBlob::Component(rows) = blob {
                for (id, bytes) in rows.iter_mut() {
                    if id == "placement:boss-1" {
                        bytes.truncate(bytes.len().saturating_sub(4));
                    }
                }
            }
        }
    }
    match restore(&mut world, &snap, &reg) {
        Err(RestoreError::DecodeFailed { entry, .. }) => assert_eq!(entry, "playing"),
        other => panic!(
            "a truncated resolved blob must be DecodeFailed, not laundered as a \
             content change: {other:?}"
        ),
    }
}

/// **Restore refuses to reconstruct a dead dynamic entity — cleanly** (re-audit
/// findings 4 + 5).
///
/// A `SimId::spawned(..)` entity that existed at the snapshot tick and is gone now has
/// no room to rebuild it and no spawn recipe, so rebuilding it from blobs alone is not
/// exact. This is a RECONSTRUCTION refusal — the entity DIED inside the window — not a
/// "birth inside the window": an entity spawned AFTER the snapshot is future-only and
/// simply despawned. And it is PREFLIGHTED: restore refuses before it mutates the
/// world, so a would-be-despawned future entity is left standing. (A `placement:` id
/// with no room record still respawns bare: the headless-fixture path, which
/// `restore_forgets_the_future_and_remembers_the_dead` covers.)
#[test]
fn restore_refuses_to_reconstruct_a_dead_dynamic_entity_without_a_recipe() {
    let reg = engine_registry();
    let mut world = sim_world();

    // A dynamic child (its id contains `/`), present at snapshot time.
    let child = SimId::spawned(&SimId::placement("boss-1"), 3);
    assert!(
        child.as_str().contains('/'),
        "the spawned vocabulary uses `/`"
    );
    world.spawn((child.clone(), kin(Vec2::ZERO, Vec2::ZERO)));
    let snap = take(&world, &reg);

    // It dies inside the window...
    let entity = *live_ids(&mut world).get(child.as_str()).unwrap();
    world.despawn(entity);
    // ...and a fresh future-only entity appears that a MUTATING restore would despawn.
    let future = world
        .spawn((
            SimId::placement("future-canary"),
            kin(Vec2::ZERO, Vec2::ZERO),
        ))
        .id();

    match restore(&mut world, &snap, &reg) {
        Err(RestoreError::UnsupportedDynamicReconstruction { sim_id }) => {
            assert_eq!(sim_id, child.as_str());
        }
        other => {
            panic!("restore did not refuse to reconstruct a dead dynamic entity: {other:?}")
        }
    }

    // The refusal was PREFLIGHTED: the world was not touched, so the future-only
    // entity that a mutating restore would have despawned is still standing (finding 5).
    assert!(
        world.get::<SimId>(future).is_some(),
        "restore despawned a future entity before refusing — the unsupported-\
         reconstruction check is not the clean pre-mutation preflight finding 5 asks for"
    );
}

/// **Stale state is measured AFTER reconciliation, not before** (audit H4).
///
/// A future-only entity — not in the snapshot, so `restore` despawns it — carries an
/// UNREGISTERED component. Measured at the top (the old ordering), its component was
/// counted as stale: a false positive on an entity about to cease to exist. Measured
/// over the post-reconciliation roster, it does not appear, because the debt a rewind
/// leaves behind is the debt on the entities that SURVIVE the rewind.
#[test]
fn stale_state_is_measured_after_reconciliation_not_before() {
    let reg = engine_registry();
    let mut world = sim_world();
    let snap = take(&world, &reg);

    // Future-only (a fresh id the snapshot never knew) with an unregistered component.
    world.spawn((
        SimId::placement("future-ghost"),
        kin(Vec2::ZERO, Vec2::ZERO),
        UnregisteredThing(7),
    ));

    let report = restore(&mut world, &snap, &reg).unwrap();
    assert_eq!(report.despawned, 1, "the future-only ghost was despawned");

    let probe = std::any::TypeId::of::<UnregisteredThing>();
    assert!(
        !report
            .stale_components
            .iter()
            .any(|c| c.type_id == Some(probe)),
        "an unregistered component on a DESPAWNED entity leaked into stale_components — \
         stale state was measured before reconciliation (audit H4): {:?}",
        report.stale_components
    );
}

/// **A corrupted STANDALONE blob makes restore refuse LOUDLY and transactionally**
/// (audit M3/S2.5; re-audit finding 5).
///
/// A registered codec that cannot read a blob it was handed is a codec failure. The
/// old `debug_assert!(false)` dropped the component silently in release builds,
/// leaving stale state reading as restored. Now restore returns `DecodeFailed`, names
/// the entry, and — for a standalone codec (`body_kinematics` is a plain component) —
/// decode-preflights it BEFORE any mutation, so the refusal leaves the world untouched.
/// A future-only entity a mutating restore would despawn proves it. Poison test,
/// co-located with the enforcement (atomicity rule).
#[test]
fn restore_refuses_a_corrupted_blob_rather_than_leaving_stale_state() {
    let reg = engine_registry();
    let mut world = sim_world();
    let mut snap = take(&world, &reg);

    // Corrupt a PLAIN-component row (`body_kinematics`, a standalone-decodable codec):
    // one byte short, so `decode_one` returns `None` (a truncated blob is rejected, not
    // guessed). Targeting the standalone codec is what exercises the pre-mutation
    // decode preflight rather than the apply-time cursor/resolved path.
    let corrupted = "body_kinematics";
    let mut hit = false;
    for (name, blob) in snap.entries.iter_mut() {
        if *name == corrupted {
            if let EntryBlob::Component(rows) = blob {
                let (_, b) = rows
                    .iter_mut()
                    .find(|(_, b)| !b.is_empty())
                    .expect("a non-empty body_kinematics row");
                b.pop();
                hit = true;
            }
        }
    }
    assert!(hit, "expected a non-empty `{corrupted}` row to corrupt");

    // A future-only entity a MUTATING restore would despawn in its first pass.
    let future = world
        .spawn((
            SimId::placement("future-canary"),
            kin(Vec2::ZERO, Vec2::ZERO),
        ))
        .id();

    match restore(&mut world, &snap, &reg) {
        Err(RestoreError::DecodeFailed { entry, .. }) => assert_eq!(entry, corrupted),
        other => {
            panic!("restore accepted a corrupted blob instead of refusing: {other:?}")
        }
    }

    // Transactional: the standalone decode preflight refused before mutating, so the
    // future entity is still standing (finding 5).
    assert!(
        world.get::<SimId>(future).is_some(),
        "restore despawned a future entity before refusing a corrupted standalone blob \
         — the decode preflight is not transactional (finding 5)"
    );
}

/// Taking a snapshot of a restored world yields the identical snapshot. Restore
/// is idempotent, which is what a rollback window replays across.
#[test]
fn take_after_restore_is_the_snapshot_you_restored() {
    let reg = engine_registry();
    let mut world = sim_world();
    let snap = take(&world, &reg);
    restore(&mut world, &snap, &reg).unwrap();
    assert_eq!(take(&world, &reg), snap);
}

/// **`restore` patches a survivor; it does not rebuild it.**
///
/// The whole reason the sketch's despawn-everything is wrong: an entity present
/// in both worlds keeps its authored config — its brain, its moveset — because
/// nothing ever took it away. What the registry does not know it also does not
/// rewind, and `stale_components` names that instead of pretending it is gone.
#[test]
fn restore_reports_the_components_it_could_not_rewind() {
    let mut reg = engine_registry();
    reg.declare_derived::<DerivedThing>("rebuilt every tick by the same system");
    let mut world = sim_world();
    let boss = world
        .try_query_filtered::<Entity, With<SimId>>()
        .unwrap()
        .iter(&world)
        .next()
        .unwrap();
    world
        .entity_mut(boss)
        .insert((UnregisteredThing(7), DerivedThing));

    let unclaimed = reg.unclaimed_components(&world);
    assert_eq!(unclaimed.len(), 1, "got {unclaimed:?}");
    assert_eq!(
        unclaimed[0].type_id,
        Some(std::any::TypeId::of::<UnregisteredThing>()),
        "the ledger keys on TypeId, because component NAMES need bevy's \
         `debug` feature and would all dedup to one placeholder without it"
    );

    let snap = take(&world, &reg);
    // The sim advances: the unregistered thing changes, as a live timer would.
    let mut e = world.entity_mut(boss);
    e.insert(UnregisteredThing(9));

    let report = restore(&mut world, &snap, &reg).unwrap();
    assert_eq!(report.patched, 2, "both entities survived and were patched");
    assert_eq!(report.respawned, 0);
    // Not lossless: an unregistered component survives, stale. `restore` now measures
    // the resource term itself; in this crate's tests bevy's `debug` names are off, so
    // the census is unreliable and `lossless()` refuses on that ground alone — which is
    // the point of the census flag (finding 6): it does not falsely succeed blind.
    assert!(!report.lossless());
    assert!(!report.resource_census_reliable);
    assert_eq!(report.stale_components, unclaimed);

    // It SURVIVED — and it is stale, still reading the tick we rewound FROM.
    // A moveset would be correct here; a timer is the bug the ledger tracks.
    let survivor = world
        .try_query::<&UnregisteredThing>()
        .unwrap()
        .iter(&world)
        .copied()
        .next();
    assert_eq!(
        survivor,
        Some(UnregisteredThing(9)),
        "restore left the unregistered component alone — that is what `stale` means"
    );

    // ...and once it is DECLARED derived, the ledger is clean, because "derived"
    // is a promise that some per-frame system rebuilds it.
    reg.declare_derived::<UnregisteredThing>("pretend");
    assert!(reg.unclaimed_components(&world).is_empty());
}

/// **A component the entity did not have at the snapshot tick is REMOVED.**
///
/// Patching that only ever inserted would leave a shield the body raised after
/// the snapshot standing through the rewind. Restoring exactly means taking it
/// away, and the registered hash is what proves it happened.
#[test]
fn patching_removes_a_component_the_snapshot_never_had() {
    let reg = engine_registry();
    let mut world = sim_world();
    let before = reg.hash_world(&world);
    let snap = take(&world, &reg);

    // The player body has no `BodyHealth` in `sim_world`. Give it one.
    let player = *live_ids(&mut world).get("slot:0").unwrap();
    world.entity_mut(player).insert(BodyHealth::new(Health {
        current: 1,
        max: 1,
        invulnerable: false,
    }));
    assert_ne!(reg.hash_world(&world), before);

    restore(&mut world, &snap, &reg).unwrap();
    assert_eq!(
        reg.hash_world(&world),
        before,
        "the late component lingered"
    );
    assert!(world.entity(player).get::<BodyHealth>().is_none());
}

fn live_ids(world: &mut World) -> std::collections::BTreeMap<String, Entity> {
    let mut q = world.query::<(Entity, &SimId)>();
    q.iter(world)
        .map(|(e, id)| (id.as_str().to_string(), e))
        .collect()
}

/// **A body with no `SimId` walks out of a rollback.** `restore` despawns the
/// REGISTERED set, and an unidentified body is not in it. This is the bug class
/// N3.1's identity pin exists to close, and until it is closed the count is
/// reported at every restore rather than left to a playtest.
#[test]
fn an_unidentified_body_survives_the_restore_and_is_counted() {
    let reg = engine_registry();
    let mut world = sim_world();
    let snap = take(&world, &reg);
    world.spawn(kin(Vec2::splat(5.0), Vec2::ZERO)); // no SimId: a ghost

    let report = restore(&mut world, &snap, &reg).unwrap();
    assert_eq!(report.unidentified_survivors, 1);
    assert!(
        !report.lossless(),
        "a restore that leaves a body standing did not restore the world"
    );
    assert!(
        report.stale_components.is_empty(),
        "nothing was stale — a whole BODY was kept that should not have been"
    );
}

/// **A cursor rewinds a survivor without re-serializing its authored half.**
///
/// The waypoints never enter a blob; the segment does. This is only sound because
/// `restore` patches survivors — an entity that still exists still has its path.
#[test]
fn a_cursor_rewinds_the_mutable_half_and_leaves_the_authored_half_alone() {
    let mut reg = engine_registry();
    reg.register_cursor::<Patrol>("patrol");
    let mut world = sim_world();
    let boss = *live_ids(&mut world).get("placement:boss-1").unwrap();
    world.entity_mut(boss).insert(Patrol {
        waypoints: vec![0.0, 10.0, 20.0],
        segment: 1,
    });

    let snap = take(&world, &reg);
    // The authored waypoints (12 bytes) must NOT enter the blob — only the 4-byte segment
    // cursor does. Measure the DELTA the cursor adds (its key + segment), a proxy immune to
    // the identity-roster (finding 1) and resource-cursor presence-tag (finding 4) overhead
    // that the old absolute `< 200` threshold folded in and that now trips it.
    world.entity_mut(boss).remove::<Patrol>();
    let without = take(&world, &reg).size_bytes();
    world.entity_mut(boss).insert(Patrol {
        waypoints: vec![0.0, 10.0, 20.0],
        segment: 1,
    });
    let with = take(&world, &reg).size_bytes();
    assert!(
        with - without < 30,
        "the authored waypoints leaked into the blob: the cursor added {} bytes",
        with - without
    );

    world.entity_mut(boss).get_mut::<Patrol>().unwrap().segment = 2;
    restore(&mut world, &snap, &reg).unwrap();

    let patrol = world.entity(boss).get::<Patrol>().unwrap();
    assert_eq!(patrol.segment, 1, "the cursor rewound");
    assert_eq!(
        patrol.waypoints,
        vec![0.0, 10.0, 20.0],
        "the authored half was never touched"
    );
}

/// **A cursor cannot rebuild a respawn, and does not pretend to.** There is no
/// authored half to apply it to. `RestoreReport::respawned` is the warning, which
/// is why a rollback window must not span a spawn.
#[test]
fn a_cursor_cannot_rebuild_an_entity_that_no_longer_exists() {
    let mut reg = engine_registry();
    reg.register_cursor::<Patrol>("patrol");
    let mut world = sim_world();
    let boss = *live_ids(&mut world).get("placement:boss-1").unwrap();
    world.entity_mut(boss).insert(Patrol {
        waypoints: vec![0.0, 10.0],
        segment: 1,
    });
    let snap = take(&world, &reg);

    world.despawn(boss);
    let report = restore(&mut world, &snap, &reg).unwrap();
    assert_eq!(report.respawned, 1);

    let back = *live_ids(&mut world).get("placement:boss-1").unwrap();
    assert!(
        world.entity(back).get::<Patrol>().is_none(),
        "a cursor has nothing to apply itself to on a naked respawn — it must not \
         invent a path"
    );
}

/// **A resource cursor tags its presence and reports an incomplete restore** (re-audit
/// finding 4).
///
/// An absent resource and a present-but-empty cursor used to both encode to `[]`, so
/// restore could not tell "it did not exist at the snapshot" from "its cursor was empty",
/// and a snapshot-present resource that was gone at restore silently applied nothing while
/// reporting success. The presence tag closes both.
#[test]
fn a_resource_cursor_tags_presence_and_reports_an_absent_target() {
    let mut reg = engine_registry();
    reg.register_resource_cursor::<TestBoard>("test_board");

    // (a) Present at snapshot, GONE at restore: a cursor cannot rebuild a resource from
    // nothing, so the restore is incomplete — reported, not swallowed as success.
    let mut world = sim_world();
    world.insert_resource(TestBoard {
        slots: 4,
        assigned: 2,
    });
    let snap = take(&world, &reg);
    world.remove_resource::<TestBoard>();
    let report = restore(&mut world, &snap, &reg).unwrap();
    assert_eq!(
        report.resource_cursors_unresolved, 1,
        "a snapshot-present resource absent at restore must be reported"
    );
    // The link to losslessness, isolated (a headless lib build has no reliable census, so
    // `report.lossless()` is false for that reason alone — pin the term itself instead).
    let probe = RestoreReport {
        resource_cursors_unresolved: 1,
        resource_census_reliable: true,
        ..RestoreReport::default()
    };
    assert!(
        !probe.lossless(),
        "an unresolved resource cursor must deny losslessness on its own"
    );

    // (b) ABSENT at snapshot, PRESENT at restore: the tag lets restore remove a resource
    // born after the snapshot, where the old empty-blob no-op left it standing.
    let mut world = sim_world();
    let absent = take(&world, &reg); // TestBoard never inserted -> tagged absent
    world.insert_resource(TestBoard {
        slots: 4,
        assigned: 9,
    });
    restore(&mut world, &absent, &reg).unwrap();
    assert!(
        world.get_resource::<TestBoard>().is_none(),
        "restore did not remove a resource that did not exist at the snapshot tick"
    );

    // (c) Present on BOTH sides: the cursor rewinds the mutable half exactly, nothing
    // reported unresolved.
    let mut world = sim_world();
    world.insert_resource(TestBoard {
        slots: 4,
        assigned: 2,
    });
    let snap = take(&world, &reg);
    world.get_resource_mut::<TestBoard>().unwrap().assigned = 7;
    let report = restore(&mut world, &snap, &reg).unwrap();
    assert_eq!(
        world.get_resource::<TestBoard>().unwrap().assigned,
        2,
        "the cursor rewound the mutable half"
    );
    assert_eq!(report.resource_cursors_unresolved, 0);
}

/// **The unit-enum discriminants are a WIRE FORMAT, and this test is the format.**
///
/// Declaration order is one refactor away from being a different order. If someone
/// moves `Chase` above `Patrol` in `CharacterAiMode`, every snapshot ever taken
/// starts decoding patrolling enemies as chasing ones — silently, because both
/// are valid states. Pinning the bytes here means that refactor fails a test
/// instead of a playtest.
#[test]
fn a_unit_enums_wire_discriminant_never_moves() {
    use ambition_characters::actor::ai::CharacterAiMode as Ai;
    use ambition_engine_core::player_state::BodyMode as Mode;

    for (mode, byte) in [
        (Ai::Idle, 0u8),
        (Ai::Patrol, 1),
        (Ai::Chase, 2),
        (Ai::Telegraph, 3),
        (Ai::Attack, 4),
        (Ai::Recover, 5),
        (Ai::Stunned, 6),
        (Ai::Dead, 7),
    ] {
        assert_eq!(encode_one(&mode), vec![byte], "{mode:?} moved");
        assert_eq!(decode_one::<Ai>(&[byte]), Some(mode));
    }
    for (mode, byte) in [
        (Mode::Standing, 0u8),
        (Mode::Crouching, 1),
        (Mode::Crawling, 2),
        (Mode::Sliding, 3),
        (Mode::MorphBall, 4),
        (Mode::Climbing, 5),
    ] {
        assert_eq!(encode_one(&mode), vec![byte], "{mode:?} moved");
    }
}

/// An unknown discriminant is `None`, never the default. A blob this build cannot
/// read is a bug to surface, not a state to guess — and `Idle` would be a very
/// plausible guess.
#[test]
fn an_unknown_discriminant_is_rejected_rather_than_defaulted() {
    use ambition_characters::actor::ai::CharacterAiMode as Ai;
    assert_eq!(decode_one::<Ai>(&[8]), None);
    assert_eq!(decode_one::<Ai>(&[255]), None);
    assert_eq!(decode_one::<Ai>(&[]), None);
    assert_eq!(decode_one::<Ai>(&[0, 0]), None, "trailing byte");
}

/// A component that references authored content by id — the `MovePlayback` shape,
/// in miniature. The catalog stays on the entity; the blob carries a name.
#[derive(Component, Clone, Debug, PartialEq)]
struct Catalog(Vec<(String, f32)>);

#[derive(Component, Debug, PartialEq)]
struct Playing {
    /// Resolved out of the `Catalog`. Never in a blob.
    power: f32,
    /// The choice, and the clock.
    id: String,
    t: f32,
}

impl SnapshotResolve for Playing {
    fn encode_ref(&self, out: &mut Vec<u8>) {
        put_str(out, &self.id);
        put_f32(out, self.t);
    }
    fn resolve(
        entity: &bevy::ecs::world::EntityWorldMut<'_>,
        r: &mut Reader<'_>,
    ) -> Result<Option<Self>, ResolveDecodeError> {
        // Decode the whole blob first (id + t) so a truncated blob is `Err`, then
        // resolve `power` out of the still-held `Catalog` (absent → `Ok(None)`).
        let id = r.str().ok_or(ResolveDecodeError)?;
        let t = r.f32().ok_or(ResolveDecodeError)?;
        let Some(catalog) = entity.get::<Catalog>() else {
            return Ok(None);
        };
        let Some(power) = catalog
            .0
            .iter()
            .find(|(name, _)| name == id)
            .map(|(_, p)| *p)
        else {
            return Ok(None);
        };
        Ok(Some(Playing {
            power,
            id: id.to_string(),
            t,
        }))
    }
}

/// **A resolved component restores its PRESENCE, not just its value.** A move is
/// inserted when it starts and removed when it ends, so a rollback must both add
/// and drop it — which a cursor cannot do.
#[test]
fn a_resolved_component_rebuilds_itself_from_content_the_entity_still_holds() {
    let mut reg = engine_registry();
    reg.register_resolved::<Playing>("playing");
    let mut world = sim_world();
    let boss = *live_ids(&mut world).get("placement:boss-1").unwrap();
    world.entity_mut(boss).insert((
        Catalog(vec![("jab".into(), 3.0), ("smash".into(), 20.0)]),
        Playing {
            power: 20.0,
            id: "smash".into(),
            t: 0.25,
        },
    ));

    // The resolved move must encode only a REFERENCE (its id + progress), never the
    // catalog it resolves `power` out of. Measure the DELTA the move adds to the
    // snapshot — a proxy immune to identity-roster and active-room overhead, which is
    // identical with and without the move (re-audit finding 3 added the roster, so an
    // absolute threshold no longer isolates the blob).
    world.entity_mut(boss).remove::<Playing>();
    let without_move = take(&world, &reg).size_bytes();
    world.entity_mut(boss).insert(Playing {
        power: 20.0,
        id: "smash".into(),
        t: 0.25,
    });
    let snap = take(&world, &reg);
    assert!(
        snap.size_bytes() - without_move < 40,
        "the catalog leaked into the blob: the resolved move added {} bytes",
        snap.size_bytes() - without_move
    );

    // The move ends. The component goes away.
    world.entity_mut(boss).remove::<Playing>();
    restore(&mut world, &snap, &reg).unwrap();
    assert_eq!(
        world.entity(boss).get::<Playing>(),
        Some(&Playing {
            power: 20.0,
            id: "smash".into(),
            t: 0.25
        }),
        "the move came back, and its power was resolved out of the catalog"
    );

    // ...and a move that started AFTER the snapshot is dropped.
    world.entity_mut(boss).insert(Playing {
        power: 3.0,
        id: "jab".into(),
        t: 0.0,
    });
    let empty = take(&world, &reg);
    world.entity_mut(boss).remove::<Playing>();
    restore(&mut world, &empty, &reg).unwrap();
    assert!(world.entity(boss).get::<Playing>().is_some());
}

/// A name the content no longer knows leaves the component OFF, rather than
/// resolving to a plausible neighbour. Impossible in a rollback; loud in a save.
///
/// **And it is not lossless** (re-audit finding 3): the registered `Playing` row the
/// snapshot carried did not come back, so restore reports one unapplied row and
/// `lossless()` is false — where the old bare-`true` resolved insert reported success for
/// a row that never returned. Dropping the component stays correct (a save whose content
/// changed should not guess); claiming the restore was complete does not.
#[test]
fn a_resolved_component_that_names_missing_content_is_dropped_and_denies_lossless() {
    let mut reg = engine_registry();
    reg.register_resolved::<Playing>("playing");
    let mut world = sim_world();
    let boss = *live_ids(&mut world).get("placement:boss-1").unwrap();
    world.entity_mut(boss).insert((
        Catalog(vec![("smash".into(), 20.0)]),
        Playing {
            power: 20.0,
            id: "smash".into(),
            t: 0.25,
        },
    ));
    let snap = take(&world, &reg);

    // The content changed under us.
    world.entity_mut(boss).insert(Catalog(vec![]));
    let report = restore(&mut world, &snap, &reg).unwrap();
    assert!(
        world.entity(boss).get::<Playing>().is_none(),
        "a name the content forgot must not resolve to a neighbour"
    );
    assert_eq!(
        report.unapplied_rows, 1,
        "the dropped resolved row must be counted, not swallowed as success"
    );
    assert!(
        !report.lossless(),
        "a restore that silently dropped registered state is not lossless (finding 3)"
    );
}

/// **The boss's seeded RNG rewinds, and so does its step cursor.**
///
/// netcode.md's N3.1 checklist: *"every seeded RNG resource (sim randomness MUST
/// be a registered seeded resource — an unregistered RNG is a determinism bug
/// N0.4 will catch)"*. The boss's lives inside `Brain`, next to its authored
/// tuning, so it rides a `SnapshotCursor` rather than a codec.
#[test]
fn a_boss_brain_rewinds_its_seed_its_cursor_and_its_clocks() {
    use ambition_characters::brain::boss_pattern::{
        BossMacroState, BossPatternCfg, BossPatternState, CyclePhase,
    };
    use ambition_characters::brain::{Brain, StateMachineCfg};

    let reg = engine_registry();
    let mut world = sim_world();
    let boss = *live_ids(&mut world).get("placement:boss-1").unwrap();

    let mut brain = Brain::StateMachine(StateMachineCfg::BossPattern {
        cfg: BossPatternCfg::neutral_test(),
        state: BossPatternState::default(),
    });
    {
        let s = brain.boss_pattern_state_mut().expect("a boss brain");
        s.rng_seed = 0xDEAD_BEEF_CAFE_F00D;
        s.step_index = 3;
        s.step_elapsed = 0.75;
        s.pattern_timer = 12.5;
        s.cycle_phase = CyclePhase::Windup;
        s.macro_state = BossMacroState::Retreat {
            remaining_s: 0.5,
            retreat_pos: ae_vec(40.0, -8.0),
        };
        s.last_hp = Some(77);
    }
    world.entity_mut(boss).insert(brain);
    let before = reg.hash_world(&world);
    let snap = take(&world, &reg);

    // The fight advances: the boss draws from its RNG and moves on.
    {
        let mut brain = world.entity_mut(boss);
        let mut brain = brain.get_mut::<Brain>().unwrap();
        let s = brain.boss_pattern_state_mut().unwrap();
        s.rng_seed = 1;
        s.step_index = 5;
        s.step_elapsed = 0.0;
        s.macro_state = BossMacroState::Engage;
    }
    assert_ne!(reg.hash_world(&world), before, "the fight must have moved");

    restore(&mut world, &snap, &reg).unwrap();
    let brain = world.entity(boss).get::<Brain>().unwrap();
    let s = brain.boss_pattern_state().unwrap();
    assert_eq!(s.rng_seed, 0xDEAD_BEEF_CAFE_F00D, "the seed rewound");
    assert_eq!(s.step_index, 3, "the step cursor rewound");
    assert_eq!(s.step_elapsed, 0.75);
    assert_eq!(s.last_hp, Some(77));
    assert!(
        matches!(s.macro_state, BossMacroState::Retreat { retreat_pos, .. } if retreat_pos.x == 40.0),
        "a boss that rewinds into Retreat rewinds to the same retreat POSITION"
    );
    assert_eq!(reg.hash_world(&world), before);
}

fn ae_vec(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

/// Diagnostics are hashed and never snapshotted: you cannot restore a count.
///
/// And so `unidentified_bodies` measures something `restore` cannot fix — which
/// is precisely why it is hashed. The canary sees the stray body that the
/// rollback left standing, and cries desync, which is the correct verdict.
#[test]
fn a_diagnostic_is_hashed_but_never_snapshotted() {
    let reg = engine_registry();
    let mut world = sim_world();
    let clean = reg.hash_world(&world);
    let snap = take(&world, &reg);
    assert!(
        !snap
            .entries
            .iter()
            .any(|(n, _)| *n == "unidentified_bodies"),
        "a count has no blob"
    );

    world.spawn(kin(Vec2::ZERO, Vec2::ZERO)); // no SimId
    restore(&mut world, &snap, &reg).unwrap();
    assert_ne!(
        reg.hash_world(&world),
        clean,
        "the stray body outlived the restore, and the canary must say so"
    );
}

/// The snapshot's rows are sorted, so two equal worlds produce `==` snapshots
/// whatever order their archetypes happened to be walked in.
#[test]
fn two_equal_worlds_take_equal_snapshots() {
    let reg = engine_registry();
    let a = take(&sim_world(), &reg);
    let b = take(&sim_world(), &reg);
    assert_eq!(a, b);
    assert_eq!(a.tick, 11);
    assert!(a.size_bytes() > 0);
}

/// ADR 0024 §9: a snapshot preserves the active movement POLICY — identity,
/// authored parameters, and policy-private runtime state (ride surface, arc
/// position, tangential speed, depth lane; crawler attachment) — while the
/// current environmental frame is deliberately NOT model state (nothing here
/// encodes a gravity direction; restore re-resolves it from the live world).
#[test]
fn restore_rewinds_the_movement_policy_and_its_private_state() {
    use ambition_engine_core::{
        AdhesiveCrawlerMotion, AxisManeuverState, AxisSweptMotion, CrawlerParams, CrawlerState,
        MomentumParams, MotionModel, SurfaceMomentumMotion, SurfaceMotion, SurfaceRef,
    };

    let reg = engine_registry();
    let mut world = World::new();
    world.insert_resource(ambition_time::SimTick(1));
    let riding = SurfaceMotion::Riding {
        on: SurfaceRef::Chain(2),
        s: 731.5,
        v_t: -880.0,
    };
    let mut momentum_params = MomentumParams::default();
    momentum_params.top_speed = 1234.0;
    let rider = world
        .spawn((
            SimId::placement("rider"),
            kin(Vec2::new(50.0, 60.0), Vec2::new(700.0, 0.0)),
            MotionModel::SurfaceMomentum(SurfaceMomentumMotion {
                params: momentum_params,
                state: riding,
                depth_lane: -1,
            }),
        ))
        .id();
    let crawler = world
        .spawn((
            SimId::placement("crawler"),
            kin(Vec2::new(9.0, 9.0), Vec2::ZERO),
            MotionModel::AdhesiveCrawler(AdhesiveCrawlerMotion {
                params: CrawlerParams {
                    crawl_speed: 77.0,
                    max_fall_speed: 500.0,
                },
                state: CrawlerState::attached(Vec2::new(-1.0, 0.0)),
            }),
        ))
        .id();
    // An axis body mid-maneuver: its private state (ADR 0024 O4) rides the
    // MotionModel codec, so a rewind resumes the dash / cling / coyote grace.
    let maneuver = AxisManeuverState {
        coyote_timer: 0.09,
        dash_timer: 0.14,
        wall_clinging: true,
        buffer_jump: 0.05,
        blink_grace_timer: 0.2,
        ..Default::default()
    };
    let jumper = world
        .spawn((
            SimId::placement("jumper"),
            kin(Vec2::new(1.0, 2.0), Vec2::new(300.0, -100.0)),
            MotionModel::AxisSwept(AxisSweptMotion {
                params: Default::default(),
                state: maneuver,
            }),
        ))
        .id();

    let snap = take(&world, &reg);

    // Wreck all three policies: swap the rider to axis-swept (losing its
    // ride), shed the crawler, and clear the jumper's in-flight maneuvers.
    world.entity_mut(rider).insert(MotionModel::default());
    world
        .entity_mut(crawler)
        .insert(MotionModel::AdhesiveCrawler(AdhesiveCrawlerMotion::new(
            CrawlerParams::default(),
        )));
    world.entity_mut(jumper).insert(MotionModel::default());

    restore(&mut world, &snap, &reg).unwrap();

    let restored = world.get::<MotionModel>(rider).unwrap();
    let MotionModel::SurfaceMomentum(motion) = restored else {
        panic!("restore must bring back the surface-momentum policy");
    };
    assert_eq!(motion.state, riding, "ride surface/arc/speed rewound");
    assert_eq!(motion.depth_lane, -1, "depth lane rewound");
    assert_eq!(motion.params.top_speed, 1234.0, "authored params rewound");

    let restored = world.get::<MotionModel>(crawler).unwrap();
    let MotionModel::AdhesiveCrawler(motion) = restored else {
        panic!("restore must bring back the crawler policy");
    };
    assert_eq!(
        motion.state.attachment(),
        Some(Vec2::new(-1.0, 0.0)),
        "the clung surface rewound"
    );
    assert_eq!(motion.params.crawl_speed, 77.0);

    let restored = world.get::<MotionModel>(jumper).unwrap();
    let MotionModel::AxisSwept(axis) = restored else {
        panic!("restore must bring back the axis policy");
    };
    assert_eq!(
        axis.state, maneuver,
        "the axis policy's private maneuver state rewound exactly"
    );
}
