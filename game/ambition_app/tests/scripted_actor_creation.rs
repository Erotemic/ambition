//! A loaded behavior factory creates native bodies with independent typed state.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{extension::{self, BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}},
    characters::actor::{BodyWallet, CharacterForm, limb::LimbRig},
    combat::{extension::status::BodyStatuses, components::RuntimeStagedActor},
    platformer::{markers::PrimaryPlayerOnly, sim_id::{SimId, SimIdCounter}, construction::SpawnOrigin}};
use bevy::prelude::*;

const LIBRARY: &str = r#"
fn memory() {
    #{key: "child", fields: [
        #{tag: 1, name: "label", kind: "text 32"},
        #{tag: 2, name: "period", kind: "f32"},
        #{tag: 3, name: "elapsed", kind: "f32"},
        #{tag: 4, name: "shots", kind: "u32"},
        #{tag: 5, name: "applied", kind: "u32"},
        #{tag: 6, name: "trades", kind: "u32"},
    ]}
}
fn node(key, character, at, period) {
    #{key: key, character: character, at: at, allegiance: "neutral",
      starting_wallet: 3, holds: ["creation_probe.child"], state: [#{schema: "child", value: #{
          label: key, period: period, elapsed: 0.0, shots: 0, applied: 0, trades: 0
      }}]}
}
fn tick(ctx) {
    let state = ctx.state.child;
    state.elapsed += ctx.dt;
    if state.elapsed >= state.period {
        state.elapsed -= state.period;
        state.shots += 1;
        if state.shots == 1 {
            ctx.requests.push(request("inventory_exchange", #{
                reply: #{request: 1, selector: "creation_probe::composition/child_trade_result"},
                exchange: #{pay_coins: 1, grants: [#{id: "manacell", count: 1}]},
            }));
        }
        ctx.requests.push(request("projectile", #{
            reply: #{request: state.shots, selector: "creation_probe::composition/child_result"},
            origin: ctx.trigger.position, dir: [0.0, -1.0], speed: 20.0,
            damage: 1, max_lifetime: 2.0, half_extent: [2.0, 2.0], visual_id: ""
        }));
    }
    ctx.state.child = state;
    ctx
}
"#;
const ROOT: &str = r#"
import "lib/factory.rhai" as factory;
fn declare() {
    #{provider: "creation_probe", key: "composition", schemas: [
        #{key: "parent", fields: [
            #{tag: 1, name: "sent", kind: "bool"},
            #{tag: 2, name: "created", kind: "u32"},
            #{tag: 3, name: "refused", kind: "u32"},
            #{tag: 4, name: "children", kind: "seq 4 body"},
        ]}, factory::memory()
    ], entries: [
        #{key: "build_team", phase: "technique_execution", trigger: #{port: "status_tick", selector: "creation_probe.parent"},
          writes: ["parent", "child"], requests: ["create_actors"], limit: 4},
        #{key: "result", phase: "actor_think", trigger: #{port: "action_result", selector: "result"}, writes: ["parent"]},
        #{key: "child", phase: "technique_execution", trigger: #{port: "status_tick", selector: "creation_probe.child"},
          writes: ["child"], requests: ["projectile", "inventory_exchange"], limit: 2},
        #{key: "child_result", phase: "actor_think", trigger: #{port: "action_result", selector: "child_result"},
          writes: ["child"]},
        #{key: "child_trade_result", phase: "actor_think", trigger: #{port: "action_result", selector: "child_trade_result"}, writes: ["child"]},
    ]}
}
fn build_team() {
    if this.state.parent.sent { return; }
    this.state.parent.sent = true;
    let p = this.trigger.position;
    let a = factory::node("sentry", "practice_sandbag", [p[0] + 240.0, p[1] - 300.0], 0.05);
    let b = factory::node("giant", "npc_giant_gnu", [p[0] + 800.0, p[1] - 600.0], 0.1);
    this.requests.push(request("create_actors", #{
        reply: #{request: 1, selector: "creation_probe::composition/result"}, actors: [a, b],
        links: [#{from: "sentry", to: "giant", kind: "grudge"}]
    }));
    let bad = factory::node("missing", "no_such_character", p, 0.1);
    this.requests.push(request("create_actors", #{
        reply: #{request: 2, selector: "creation_probe::composition/result"}, actors: [a, bad]
    }));
    let typo = factory::node("typo", "practice_sandbag", p, 0.1);
    typo.state[0].value.typo = 2;
    this.requests.push(request("create_actors", #{
        reply: #{request: 3, selector: "creation_probe::composition/result"}, actors: [typo]
    }));
    this.requests.push(request("create_actors", #{
        reply: #{request: 4, selector: "creation_probe::composition/result"}, actors: [a, b],
        links: [#{from: "sentry", to: "missing", kind: "grudge"}]
    }));
}
fn result() {
    if this.trigger.ticket.domain != "world.create_actors" || !this.trigger.terminal { return; }
    if this.trigger.stage == "applied" {
        if this.trigger.code != "actors_created" || this.trigger.subjects.len() != 2 { throw "phantom creation"; }
        this.state.parent.created += 1;
        this.state.parent.children = this.trigger.subjects;
    } else if this.trigger.stage == "rejected" {
        if this.trigger.code != "actor_creation_refused" { throw "wrong refusal"; }
        this.state.parent.refused += 1;
    } else { throw "unexpected creation outcome"; }
}
fn child() { this = factory::tick(this); }
fn child_trade_result() {
    if !this.trigger.terminal { return; }
    if this.trigger.stage != "applied" || this.trigger.code != "inventory_exchanged" {
        throw "the constructed wallet could not fund the trade";
    }
    this.state.child.trades += 1;
}
fn child_result() {
    if this.trigger.ticket.domain != "projectiles.spawn" || !this.trigger.terminal { return; }
    if this.trigger.stage != "applied" || this.trigger.code != "projectile_created"
        || this.trigger.subjects.len() != 1 { throw "a child shot did not exist"; }
    this.state.child.applied += 1;
}
"#;

fn options() -> ambition_app::rl_sim::Platformer2dSimHarnessOptions {
    let mut save = ambition_platformer2d::session::AmbitionGameSaveData::default();
    save.set_flag("intro_wake_seen", true);
    crate::common::fixed_60hz_room_options("intro_wake_room").with_save(save)
}

fn prepared() -> extension::PreparedScript {
    extension::PreparedScript::with_libraries("creation.rhai", ROOT, extension::script_ports(), 200_000,
        Default::default(), |id| {
            if id != "lib/factory.rhai" { return Err(format!("unknown {id}")); }
            Ok(extension::LibrarySource {name: id.into(), text: LIBRARY.into()})
        }).unwrap()
}

fn setup(sim: &mut Platformer2dSimHarness) -> [Entity; 2] {
    sim.step_n(AgentAction::default(), 4);
    let world = sim.world_mut();
    let primary = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    let creator = world.query::<(Entity, &CharacterForm)>().iter(world)
        .find(|(_, form)| form.id() == "npc_creator").unwrap().0;
    for body in [primary, creator] {
        world.entity_mut(body).insert(BodyStatuses(vec!["creation_probe.parent".into()]));
    }
    sim.rebase_rollback_history().unwrap();
    [primary, creator]
}

fn parent_record(sim: &Platformer2dSimHarness, body: Entity) -> &extension::sdk::Record {
    sim.world().get::<BodyRecords>(body).unwrap().get(&SchemaKey::new("creation_probe", "parent", 1)).unwrap()
}

#[test]
fn a_behavior_factory_builds_real_body_clusters_with_state_and_replays() {
    let source = prepared();
    let mut sim = Platformer2dSimHarness::build(options().with_sync_test_rollback_settings(4, 12), |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        app.init_resource::<extension::ExtensionProfile>();
        extension::add_prepared_script(app, "creation", &source);
        Ok(())
    }).unwrap();
    let parents = setup(&mut sim);
    let bag_before = sim.world().resource::<ambition_platformer2d::items::OwnedItems>()
        .count(ambition_platformer2d::items::Item::ManaCell);
    let before: Vec<_> = parents.iter().map(|body| sim.world().get::<SimIdCounter>(*body).unwrap().0).collect();
    sim.step_n(AgentAction::default(), 24);
    let mut roots = Vec::new();
    for (index, parent) in parents.into_iter().enumerate() {
        let state = parent_record(&sim, parent);
        assert_eq!(state.get(FieldRef(1)).unwrap().as_u32(), Some(1));
        assert_eq!(state.get(FieldRef(2)).unwrap().as_u32(), Some(3));
        let Value::Seq(ids) = state.get(FieldRef(3)).unwrap() else { panic!("child identities") };
        assert_eq!(ids.len(), 2);
        assert_eq!(sim.world().get::<SimIdCounter>(parent).unwrap().0, before[index] + 2,
            "refused batches must not spend identities");
        for (child, id) in ids.iter().enumerate() {
            let id = id.as_ref_id().unwrap();
            let mut query = sim.world().try_query::<(Entity, &SimId)>().unwrap();
            let found: Vec<_> = query.iter(sim.world()).filter(|(_, actual)| actual.as_str() == id).collect();
            assert_eq!(found.len(), 1, "one live birth per returned identity");
            let entity = found[0].0;
            assert!(sim.world().get::<RuntimeStagedActor>(entity).is_some());
            assert!(matches!(sim.world().get::<SpawnOrigin>(entity), Some(SpawnOrigin::Dynamic {parent: source, ..})
                if Some(source) == sim.world().get::<SimId>(parent)));
            let record = sim.world().get::<BodyRecords>(entity).unwrap()
                .get(&SchemaKey::new("creation_probe", "child", 1)).unwrap();
            assert_eq!(record.get(FieldRef(0)).unwrap().as_text(), Some(if child == 0 {"sentry"} else {"giant"}));
            assert!(record.get(FieldRef(3)).unwrap().as_u32().unwrap() > 0, "the seeded behavior must run");
            assert!(record.get(FieldRef(4)).unwrap().as_u32().unwrap() > 0, "the native domain must confirm shot creation");
            assert_eq!(record.get(FieldRef(5)).unwrap().as_u32(), Some(1));
            assert_eq!(sim.world().get::<BodyWallet>(entity).unwrap().balance, 2);
            if child == 1 {
                let rig = sim.world().get::<LimbRig>(entity).expect("a complete giant cluster");
                use ambition_platformer2d::characters::actor::limb::LimbSlot;
                assert!(rig.get(LimbSlot::HAND_LEFT).is_some() && rig.get(LimbSlot::HAND_RIGHT).is_some());
                for hand in rig.limbs.values() {
                    assert_eq!(sim.world().get::<ambition_platformer2d::characters::actor::ActorFaction>(*hand),
                        sim.world().get::<ambition_platformer2d::characters::actor::ActorFaction>(entity));
                }
                assert!(sim.world().get::<SimIdCounter>(entity).unwrap().0 > rig.limbs.len() as u64,
                    "projectiles use identities after the reserved limbs");
            }
            roots.push(entity);
        }
    }
    assert_eq!(roots.len(), 4);
    assert_eq!(sim.world().resource::<ambition_platformer2d::items::OwnedItems>()
        .count(ambition_platformer2d::items::Item::ManaCell), bag_before + 4);
    for pair in roots.chunks_exact(2) {
        let aggression = sim.world().get::<ambition_platformer2d::combat::components::ActorAggression>(pair[0]).unwrap();
        assert_eq!(aggression.grudge.and_then(|grudge| grudge.body()), Some(pair[1]),
            "the native relation must name the actual birth after replay");
    }
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
    sim.rollback_health().unwrap();
    for (path, cost) in &sim.world().resource::<extension::ExtensionProfile>().0 {
        if path.starts_with("creation_probe::") && cost.calls > 0 {
            println!("ACTOR_CREATION_COST {path} calls={} backend_us={} host_us={}", cost.calls,
                cost.backend.as_micros() as f64 / cost.calls as f64, cost.total.as_micros() as f64 / cost.calls as f64);
        }
    }
    let batch: Vec<_> = [("native_sentry", "practice_sandbag"), ("native_giant", "npc_giant_gnu")]
        .into_iter().map(|(key, character)| extension::RuntimeActor {
            key: key.into(), character: character.into(),
            aabb: bevy::math::bounding::Aabb2d::new(Vec2::new(600.0, -400.0), Vec2::new(16.0, 24.0)),
            faction: ambition_platformer2d::characters::actor::ActorFaction::Neutral,
            face_left: false, starting_wallet: None, holds: Vec::new(),
        }).collect();
    let start = std::time::Instant::now();
    let native = extension::create_actors(sim.world_mut(), parents[0], &batch, &[], Default::default()).unwrap();
    println!("ACTOR_CREATION_NATIVE roots={} elapsed_us={}", native.len(), start.elapsed().as_micros());
    for body in native {
        assert_eq!(sim.world().get::<SimId>(body.entity), Some(&body.id));
        assert!(sim.world().get::<BodyWallet>(body.entity).is_none());
    }
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 8);
    sim.rollback_health().unwrap();
}

#[test]
fn an_edit_to_the_factory_library_changes_live_bodies_without_recreating_them() {
    struct Files(std::path::PathBuf);
    impl Drop for Files { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
    let files = Files(std::env::temp_dir().join(format!("ambition_actor_factory_{}", std::process::id())));
    std::fs::create_dir_all(files.0.join("lib")).unwrap();
    std::fs::write(files.0.join("composition.rhai"), ROOT).unwrap();
    let library = files.0.join("lib/factory.rhai");
    std::fs::write(&library, LIBRARY).unwrap();
    let mut sim = Platformer2dSimHarness::new_with_options(
        options().with_extension_module_files(vec![files.0.join("composition.rhai")])).unwrap();
    let parents = setup(&mut sim);
    sim.step_n(AgentAction::default(), 24);
    let ids: Vec<_> = parents.iter().map(|parent| parent_record(&sim, *parent).get(FieldRef(3)).unwrap().clone()).collect();
    let generation = sim.world().resource::<extension::ExtensionGeneration>().clone();
    std::fs::write(&library, LIBRARY
        .replace("state.elapsed += ctx.dt;", "state.period = 0.2; state.elapsed += ctx.dt;")
        .replace("starting_wallet: 3", "starting_wallet: 30")).unwrap();
    sim.step_n(AgentAction::default(), 50);
    assert_ne!(*sim.world().resource::<extension::ExtensionGeneration>(), generation);
    for (parent, before) in parents.into_iter().zip(ids) {
        let state = parent_record(&sim, parent);
        assert_eq!(state.get(FieldRef(1)).unwrap().as_u32(), Some(1));
        assert_eq!(state.get(FieldRef(3)).unwrap(), &before);
        let Value::Seq(ids) = before else { panic!("identities") };
        for id in ids {
            let mut query = sim.world().try_query::<(Entity, &SimId)>().unwrap();
            let entity = query.iter(sim.world()).find(|(_, actual)| Some(actual.as_str()) == id.as_ref_id()).unwrap().0;
            let child = sim.world().get::<BodyRecords>(entity).unwrap()
                .get(&SchemaKey::new("creation_probe", "child", 1)).unwrap();
            assert_eq!(child.get(FieldRef(1)).unwrap().as_f32(), Some(0.2));
            assert_eq!(sim.world().get::<BodyWallet>(entity).unwrap().balance, 2);
            assert_eq!(child.get(FieldRef(5)).unwrap().as_u32(), Some(1));
            assert!(child.get(FieldRef(3)).unwrap().as_u32().unwrap() > 0);
        }
    }
    let next_parent = extension::create_actors(sim.world_mut(), parents[0], &[extension::RuntimeActor {
        key: "next_factory".into(), character: "practice_sandbag".into(),
        aabb: bevy::math::bounding::Aabb2d::new(Vec2::new(600.0, -400.0), Vec2::new(16.0, 24.0)),
        faction: ambition_platformer2d::characters::actor::ActorFaction::Neutral,
        face_left: false, starting_wallet: None, holds: vec!["creation_probe.parent".into()],
    }], &[], Default::default()).unwrap().remove(0).entity;
    assert!(sim.world().get::<BodyWallet>(next_parent).is_none());
    sim.step_n(AgentAction::default(), 24);
    let state = parent_record(&sim, next_parent);
    let Value::Seq(ids) = state.get(FieldRef(3)).unwrap() else { panic!("new child identities") };
    assert_eq!(ids.len(), 2);
    for id in ids {
        let mut query = sim.world().try_query::<(Entity, &SimId)>().unwrap();
        let entity = query.iter(sim.world()).find(|(_, actual)| Some(actual.as_str()) == id.as_ref_id()).unwrap().0;
        assert_eq!(sim.world().get::<BodyWallet>(entity).unwrap().balance, 29);
        assert_eq!(sim.world().get::<BodyRecords>(entity).unwrap()
            .get(&SchemaKey::new("creation_probe", "child", 1)).unwrap()
            .get(FieldRef(5)).unwrap().as_u32(), Some(1));
    }
    assert_eq!(sim.world().resource::<ExtensionFaults>().total, 0);
}
