//! Loaded rules receive native hit decisions for bodies and live rooms.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    characters::actor::{BodyCombat, BodyHealth, BodyWallet, CharacterForm, Health},
    combat::{
        components::ActiveCombatant, extension::status::BodyStatuses, rules::HitReceptionRules,
        CombatCapabilities,
    },
    engine_core as ae,
    extension::{
        sdk::{FieldRef, SchemaKey, Value},
        BodyRecords, ExtensionFaults,
    },
    platformer::{
        lifecycle::{LiveRooms, RoomInstanceRoot},
        markers::PrimaryPlayerOnly,
        schedule::{CombatSet, SimScheduleExt},
        sim_id::SimId,
    },
    world::scoped_rules::{DeclaredRules, RulesScope},
};
use bevy::prelude::*;

const RECORD: SchemaKey = SchemaKey::new("reception_probe", "hits", 1);
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "reception_probe", key: "observe",
       schemas: [#{ key: "hits", fields: [
           #{ tag: 1, name: "wallet", kind: "u32" },
           #{ tag: 2, name: "damaged", kind: "u32" },
           #{ tag: 3, name: "ignored", kind: "u32" },
           #{ tag: 4, name: "incoming", kind: "u32" },
           #{ tag: 5, name: "outgoing", kind: "u32" },
           #{ tag: 6, name: "truncated", kind: "u32" },
           #{ tag: 7, name: "queued", kind: "u32" },
           #{ tag: 8, name: "created", kind: "u32" },
       ] }],
       entries: [#{ key: "observe", phase: "hit_reaction",
           trigger: #{ port: "hit_receptions", selector: "reception_probe.observe" },
           writes: ["hits"], requests: ["hold_status", "projectile"], limit: 2 },
           #{ key: "created", phase: "actor_think", trigger: #{ port: "action_result", selector: "created" }, writes: ["hits"] }],
    }
}
fn observe() {
    let batch = this.trigger;
    let owned = false;
    if batch.events.len() > 2 { throw "the room budget was ignored"; }
    if batch.truncated { this.state.hits.truncated += 1; }
    for hit in batch.events {
        if hit.victim.id == () || hit.attacker == () || hit.attacker.id == () {
            throw "the native receiver lost its identity";
        }
        if hit.source != "melee" { throw "the native cause changed"; }
        if hit.health_depleted { throw "the test hit was lethal"; }
        if hit.incoming { this.state.hits.incoming += 1; owned = true; }
        if hit.outgoing { this.state.hits.outgoing += 1; owned = true; }
        if hit.decision == "wallet_shielded" {
            if hit.damage != 0 || hit.wallet_spent <= 0 { throw "false damage through a shield"; }
            this.state.hits.wallet += 1;
        } else if hit.decision == "damaged" {
            if hit.damage <= 0 || hit.raw_damage != 2 { throw "the accepted damage was lost"; }
            this.state.hits.damaged += 1;
        } else if hit.decision == "ignored" {
            if hit.damage != 0 { throw "a rejected hit reports damage"; }
            this.state.hits.ignored += 1;
        } else { throw "unexpected native decision"; }
    }
    if owned {
        this.requests.push(request("hold_status", #{ kind: "reception_probe.received", held: true }));
        this.state.hits.queued += 1;
        this.requests.push(request("projectile", #{
            reply: #{ request: this.state.hits.queued, selector: "reception_probe::observe/created" },
            origin: [200.0, 80.0], dir: [1.0, 0.0], speed: 100.0,
            damage: 1, max_lifetime: 3.0, half_extent: [2.0, 2.0],
        }));
    }
 }
fn created() {
    if this.trigger.stage == "applied" {
        if this.trigger.code != "projectile_created" || this.trigger.subjects.len() != 1 { throw "lost creation result"; }
        this.state.hits.created += 1;
    }
}
"#;

#[test]
fn loaded_rules_receive_bounded_native_decisions_on_either_controller_and_replay() {
    let dir = std::env::temp_dir().join(format!("ambition_hit_receptions_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("receptions.rhai");
    std::fs::write(&file, SOURCE).unwrap();
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("intro_wake_room")
            .with_save(save)
            .with_extension_module_files(vec![file])
            .with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    let world = sim.world_mut();
    let player = world
        .query_filtered::<Entity, PrimaryPlayerOnly>()
        .single(world)
        .unwrap();
    let teacher = world
        .query::<(Entity, &CharacterForm)>()
        .iter(world)
        .find(|(_, form)| form.id() == "npc_creator")
        .unwrap()
        .0;
    let root = world
        .query_filtered::<Entity, With<RoomInstanceRoot>>()
        .single(world)
        .unwrap();
    // The native shield is derived from authored character data and room rules.
    for body in [player, teacher] {
        world.entity_mut(body).insert(CharacterForm::new("sanic"));
    }
    world.get_resource_or_insert_with(DeclaredRules::<ambition_platformer2d::damage::wallet_shield::WalletShieldRule>::default)
        .declare(RulesScope::EveryRoom, ambition_platformer2d::damage::wallet_shield::WalletShieldRule);
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    for (body, balance) in [(player, 11), (teacher, 7)] {
        world.entity_mut(body).insert((
            ActiveCombatant,
            BodyWallet { balance },
            BodyHealth::new(Health::new(10)),
        ));
        *world.get_mut::<BodyCombat>(body).unwrap() = BodyCombat::default();
    }
    world
        .get_mut::<CombatCapabilities>(teacher)
        .unwrap()
        .never_dies = false;
    for body in [player, teacher] {
        world
            .entity_mut(body)
            .insert(BodyStatuses(vec!["reception_probe.observe".into()]));
    }
    world
        .get_resource_or_insert_with(DeclaredRules::<HitReceptionRules>::default)
        .declare(
            RulesScope::EveryRoom,
            HitReceptionRules {
                max_events: 2,
                room_observers: vec!["reception_probe.observe".into()],
            },
        );
    let player_id = world.get::<SimId>(player).unwrap().clone();
    let teacher_id = world.get::<SimId>(teacher).unwrap().clone();
    let first = world.resource::<ambition_platformer2d::time::SimTick>().0 + 2;
    let schedule = sim.app_mut().sim_schedule();
    sim.app_mut().add_systems(
        schedule,
        (move |tick: Res<ambition_platformer2d::time::SimTick>,
               mut hits: ambition_platformer2d::combat::HitWriter<'_, '_>,
               bodies: Query<(Entity, &SimId, &ae::BodyKinematics)>,
               rooms: LiveRooms| {
            if tick.0 != first && tick.0 != first + 90 {
                return;
            }
            let (player, _, _) = bodies.iter().find(|(_, id, _)| **id == player_id).unwrap();
            let (teacher, _, _) = bodies.iter().find(|(_, id, _)| **id == teacher_id).unwrap();
            let repeats = if tick.0 == first { 3 } else { 1 };
            for (victim, attacker, count) in [(teacher, player, repeats), (player, teacher, 1)] {
                let (_, _, kin) = bodies.get(victim).unwrap();
                for _ in 0..count {
                    hits.write(ambition_platformer2d::combat::HitEvent { status: None,
                        origin: None,
                        properties: Default::default(),
                        contact: None,
                        pieces: Vec::new(),
                        strike_sfx: None,
                        volume: kin.aabb().into(),
                        damage: 2,
                        source: ambition_platformer2d::combat::HitSource::Melee,
                        attacker: Some(attacker),
                        room: rooms.of(victim),
                        target: ambition_platformer2d::combat::HitTarget::Body(victim),
                        mode: ambition_platformer2d::combat::HitMode::Knockback,
                        knockback: None,
                        ignored_targets: Vec::new(),
                        attacker_move_instance: None,
                    });
                }
            }
        })
        .in_set(CombatSet::Resolve)
        .before(ambition_platformer2d::actors::body::apply_feature_hit_events),
    );
    let before_shots = sim.world().resource::<ambition_platformer2d::projectiles::ProjectileSeqCounter>().0;
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 105);
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    for body in [player, teacher, root] {
        let record = sim
            .world()
            .get::<BodyRecords>(body)
            .unwrap()
            .get(&RECORD)
            .unwrap();
        assert_eq!(record.get(FieldRef(6)).unwrap(), record.get(FieldRef(7)).unwrap(),
            "late counter shots receive actual creation replies");
        assert_eq!(
            record.get(FieldRef(0)).unwrap(),
            &Value::U32(2),
            "both wallet defenses were reported"
        );
        assert_eq!(
            record.get(FieldRef(1)).unwrap(),
            &Value::U32(2),
            "both native damage paths were reported"
        );
        assert_eq!(
            record.get(FieldRef(2)).unwrap(),
            &Value::U32(1),
            "a repeated hit was rejected by the resolver"
        );
        assert_eq!(record.get(FieldRef(5)).unwrap(), &Value::U32(1));
        if body == root {
            assert_eq!(record.get(FieldRef(3)).unwrap(), &Value::U32(0));
            assert_eq!(record.get(FieldRef(4)).unwrap(), &Value::U32(0));
        } else {
            assert!(matches!(record.get(FieldRef(3)).unwrap(), Value::U32(n) if *n > 0));
            assert!(matches!(record.get(FieldRef(4)).unwrap(), Value::U32(n) if *n > 0));
        }
    }
    for body in [player, teacher] {
        assert!(sim
            .world()
            .get::<BodyStatuses>(body)
            .unwrap()
            .0
            .iter()
            .any(|key| key == "reception_probe.received"));
    }
    assert!(sim.world().resource::<ambition_platformer2d::projectiles::ProjectileSeqCounter>().0 > before_shots,
        "late reaction requests reached the native materializer");
    sim.rollback_health().unwrap();
    let _ = std::fs::remove_dir_all(dir);
}
