//! D-C exit check: two mode-scoped rules plugins coexist in one app.
//!
//! This is the demo-hosting seam's contract, executable. Ambition hosts several
//! demos' rulesets in ONE binary; each is awake only inside the rooms its mode
//! tag claims, and the state it owns dies when the player leaves those rooms.
//!
//! Both halves are asserted here:
//!   1. `in_mode("a")`-gated systems do not run while the active room says `b`.
//!   2. `ModeScopedEntity("a")` entities are despawned when the active room's
//!      mode changes away from `a` — while `b`'s entities, and `b`'s own rules,
//!      keep running across that same transition.

use bevy::prelude::*;

use ambition_platformer_primitives::lifecycle::{ModeScopedEntity, SpawnScopedExt as _};
use ambition_runtime::{despawn_departed_mode_entities, in_mode};
use ambition_world::rooms::{ActiveRoomMetadata, RoomMetadata};

/// How many times each mode's gated rule has run.
#[derive(Resource, Default, Debug, PartialEq, Eq)]
struct RuleTicks {
    a: u32,
    b: u32,
}

/// A demo's rules plugin, in the shape the seam prescribes: ONE system list,
/// and a constructor flag deciding whether it is gated on a mode (hosted inside
/// Ambition) or runs unconditionally (the demo standing alone).
struct DemoRulesPlugin {
    mode: &'static str,
    hosted: bool,
}

impl DemoRulesPlugin {
    fn hosted(mode: &'static str) -> Self {
        Self { mode, hosted: true }
    }
}

impl Plugin for DemoRulesPlugin {
    /// Two hosted demos are two instances of this same fixture type. The seam's
    /// whole claim is that rulesets coexist, so they must not dedup by TypeId.
    fn is_unique(&self) -> bool {
        false
    }

    fn build(&self, app: &mut App) {
        let mode = self.mode;
        let rule = move |mut ticks: ResMut<RuleTicks>| match mode {
            "a" => ticks.a += 1,
            "b" => ticks.b += 1,
            other => panic!("unknown demo mode {other}"),
        };
        if self.hosted {
            app.add_systems(Update, rule.run_if(in_mode(mode)));
        } else {
            app.add_systems(Update, rule);
        }
    }
}

fn set_mode(app: &mut App, mode: Option<&str>) {
    ambition_platformer_primitives::lifecycle::session_world_component_mut::<ActiveRoomMetadata>(app.world_mut()).expect("active session room metadata").0 = RoomMetadata {
        mode: mode.map(str::to_string),
        ..Default::default()
    };
}

fn mode_scoped_entities(app: &mut App) -> Vec<String> {
    let mut query = app.world_mut().query::<&ModeScopedEntity>();
    let mut modes: Vec<String> = query
        .iter(app.world())
        .map(|scope| scope.0.clone())
        .collect();
    modes.sort();
    modes
}

/// Both rulesets installed; only the active room's mode is awake. Neither
/// plugin knows the other exists, and neither owns a global state.
fn two_hosted_demos() -> App {
    let mut app = App::new();
    ambition_platformer_primitives::lifecycle::insert_session_world_component(app.world_mut(), ActiveRoomMetadata::default());
    app.init_resource::<RuleTicks>();
    // The sweep as the engine group schedules it, minus the sim-schedule
    // plumbing this test does not need. `SandboxSet::Progression` membership is
    // what orders it against `sync_active_room_metadata` in a real app.
    app.add_systems(Update, despawn_departed_mode_entities);
    app.add_plugins(DemoRulesPlugin::hosted("a"));
    app.add_plugins(DemoRulesPlugin::hosted("b"));
    app
}

#[test]
fn a_mode_gated_rule_runs_only_inside_its_own_mode() {
    let mut app = two_hosted_demos();

    // No world / no mode: the base game. Neither hosted ruleset wakes.
    app.update();
    assert_eq!(
        *app.world().resource::<RuleTicks>(),
        RuleTicks { a: 0, b: 0 }
    );

    set_mode(&mut app, Some("a"));
    app.update();
    app.update();
    assert_eq!(
        *app.world().resource::<RuleTicks>(),
        RuleTicks { a: 2, b: 0 }
    );

    set_mode(&mut app, Some("b"));
    app.update();
    assert_eq!(
        *app.world().resource::<RuleTicks>(),
        RuleTicks { a: 2, b: 1 },
        "mode `a`'s systems must not run while the active room says `b`"
    );

    // Back to the base game: both sleep again. A mode is a room property, not a
    // latch some plugin owns.
    set_mode(&mut app, None);
    app.update();
    assert_eq!(
        *app.world().resource::<RuleTicks>(),
        RuleTicks { a: 2, b: 1 }
    );
}

#[test]
fn leaving_a_mode_despawns_only_that_modes_entities() {
    let mut app = two_hosted_demos();
    set_mode(&mut app, Some("a"));

    // Each hosted ruleset spawns its mode-owner entity.
    app.world_mut().commands().spawn_mode_scoped("a", ());
    app.world_mut().commands().spawn_mode_scoped("b", ());
    let survivor = app.world_mut().commands().spawn(()).id();
    app.world_mut().flush();
    assert_eq!(mode_scoped_entities(&mut app), vec!["a", "b"]);

    // Entering `b` retires `a`'s state and leaves `b`'s standing.
    set_mode(&mut app, Some("b"));
    app.update();
    assert_eq!(
        mode_scoped_entities(&mut app),
        vec!["b"],
        "the departed mode's entities are swept; the entered mode's are not"
    );
    assert!(
        app.world().get_entity(survivor).is_ok(),
        "an unscoped entity is not a mode's to despawn"
    );

    // Returning to the base game retires the last mode too.
    set_mode(&mut app, None);
    app.update();
    assert!(mode_scoped_entities(&mut app).is_empty());
}

/// A room transition WITHIN a mode (metadata changes, mode does not) must not
/// tear the mode's state down — that is the whole difference between a
/// mode-scoped entity and a room-scoped one.
#[test]
fn a_room_change_inside_the_same_mode_spares_the_modes_entities() {
    let mut app = two_hosted_demos();
    set_mode(&mut app, Some("a"));
    app.world_mut().commands().spawn_mode_scoped("a", ());
    app.world_mut().flush();

    ambition_platformer_primitives::lifecycle::session_world_component_mut::<ActiveRoomMetadata>(app.world_mut()).expect("active session room metadata").0 = RoomMetadata {
        mode: Some("a".into()),
        biome: Some("a_second_room".into()),
        ..Default::default()
    };
    app.update();
    assert_eq!(mode_scoped_entities(&mut app), vec!["a"]);
}

/// The standalone half of the constructor flag: ungated, the demo's rules run
/// everywhere, because when the demo IS the game there is no mode to leave.
#[test]
fn a_standalone_ruleset_runs_with_no_mode_at_all() {
    let mut app = App::new();
    ambition_platformer_primitives::lifecycle::insert_session_world_component(app.world_mut(), ActiveRoomMetadata::default());
    app.init_resource::<RuleTicks>();
    app.add_plugins(DemoRulesPlugin {
        mode: "a",
        hosted: false,
    });
    app.update();
    assert_eq!(app.world().resource::<RuleTicks>().a, 1);
}
