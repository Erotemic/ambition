//! A script holds a pair of portals through the portal owner and reads back
//! what it holds: the pair is open while it is asked for, and gone once the
//! script stops asking.

#![cfg(all(feature = "rl_sim", feature = "portal"))]

use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::{
    combat::extension::status::BodyStatuses,
    extension::{BodyRecords, ExtensionFaults, sdk::{FieldRef, SchemaKey, Value}},
    platformer::markers::PrimaryPlayerOnly,
};
use bevy::prelude::*;

const RECORD: SchemaKey = SchemaKey::new("pair_probe", "pairs", 1);
const SOURCE: &str = r#"
fn declare() {
    #{ provider: "pair_probe", key: "door",
       schemas: [#{ key: "pairs", fields: [
           #{ tag: 1, name: "step", kind: "u32" },
           #{ tag: 2, name: "seen", kind: "text 64" },
       ] }],
       entries: [
           #{ key: "hold", phase: "technique_execution",
              trigger: #{ port: "status_tick", selector: "pair_probe.hold" },
              reads: ["held_portal_pairs"], writes: ["pairs"], requests: ["held_portal_pair"], limit: 1 },
       ],
    }
}
fn end(at, nx) { #{ at: at, normal: [nx, 0.0], half_length: 24.0 } }
fn state_of(pairs) {
    for pair in pairs {
        if pair.slot == 7 { return if pair.open { "O" } else { "x" }; }
    }
    "-"
}
fn hold() {
    let step = this.state.pairs.step + 1;
    this.state.pairs.step = step;
    // Read on steps 5 and 15 what the last lowering left; hold for ten.
    if step == 5 || step == 15 { this.state.pairs.seen += state_of(this.observe.held_portal_pairs.pairs); }
    if step <= 10 {
        let p = this.trigger.position;
        this.requests.push(request("held_portal_pair", #{ slot: 7,
            ends: [end([p[0] - 160.0, p[1] - 120.0], 1.0), end([p[0] + 160.0, p[1] - 120.0], -1.0)],
            looks: [1, 2] }));
    }
}
"#;

/// ⭐ Open while asked, gone once not. Sync-test host, so the pair's record
/// and its two portals survive the rewinds.
#[test]
fn a_script_holds_a_portal_pair_and_reads_it_back() {
    use ambition_platformer2d::portal::extension::HeldPortalOf;
    let dir = std::env::temp_dir().join(format!("ambition_held_portal_pair_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("door.rhai");
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
    let player = world.query_filtered::<Entity, PrimaryPlayerOnly>().single(world).unwrap();
    world.entity_mut(player).insert(BodyStatuses(vec!["pair_probe.hold".into()]));
    sim.rebase_rollback_history().unwrap();
    sim.step_n(AgentAction::default(), 6);
    let held = |sim: &mut Platformer2dSimHarness| {
        let world = sim.world_mut();
        world.query::<&HeldPortalOf>().iter(world).filter(|of| of.0 == player).count()
    };
    assert_eq!(held(&mut sim), 2, "both ends are open while the script asks");
    sim.step_n(AgentAction::default(), 12);
    sim.rollback_health().unwrap();
    assert_eq!(held(&mut sim), 0, "no end stays open once the script stops asking");
    let faults = sim.world().resource::<ExtensionFaults>();
    assert_eq!(faults.total, 0, "{:?}", faults.recent.first());
    let record = sim.world().get::<BodyRecords>(player).unwrap().get(&RECORD).cloned().unwrap();
    assert_eq!(record.get(FieldRef(1)).unwrap(), &Value::Text("O-".into()),
        "the script reads its pair open while it holds it, and no slot after");
    let _ = std::fs::remove_dir_all(dir);
}
