//! the App-level tests build their own world, the shape that can pass with
//! production wiring absent. What they own is the seam: authored text → a
//! prepared call → a request. That the shipped world actually SAYS it is pinned
//! in `ambition_content`, and that the whole chain runs in the composed game is
//! pinned by the app fixture.

use super::*;

use ambition_platformer2d_ldtk::{
    ActiveLdtkProject, LdtkEntityInstance, LdtkFieldInstance, LdtkLayerInstance, LdtkLevel,
    LdtkProject,
};
use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, CommandDescriptor, CommandId, CommandOutcome, ParamKind, ParamSpec, PublishCommand,
};
use serde_json::Value;

const SWITCH: &str = "kernel_switch_down";
const LINE: &str = "bystander.ring C";

fn field(identifier: &str, value: &str) -> LdtkFieldInstance {
    LdtkFieldInstance {
        identifier: identifier.into(),
        value: Value::String(value.into()),
        real_editor_values: vec![Value::Null],
    }
}

/// One level whose `activeArea` is `symmetry_room`, holding one `Switch`.
///
/// `on_activate` present by default: the interesting negative case is a switch
/// WITHOUT it, so the fixture's default is the boring one.
fn project_with_one_switch(on_activate: Option<&str>) -> LdtkProject {
    let mut fields = vec![
        field("id", SWITCH),
        field("name", "kernel gravity down"),
        field("action", "SetGravityDown"),
    ];
    if let Some(on_activate) = on_activate {
        fields.push(field(ON_ACTIVATE_FIELD, on_activate));
    }
    LdtkProject {
        json_version: "1.5.3".into(),
        levels: vec![LdtkLevel {
            identifier: "symmetry_room".into(),
            iid: "level-iid".into(),
            world_x: 0,
            world_y: 0,
            px_wid: 1280,
            px_hei: 1280,
            field_instances: vec![field("activeArea", "symmetry_room")],
            layer_instances: vec![LdtkLayerInstance {
                identifier: "Ambition".into(),
                layer_type: "Entities".into(),
                c_wid: 80,
                c_hei: 80,
                grid_size: 16,
                entity_instances: vec![
                    // the converter refuses an area without one; the old
                    // hand-walk never asked. See the sibling lock-wall fixture.
                    LdtkEntityInstance {
                        iid: "PlayerStart-test-symmetry".into(),
                        identifier: "PlayerStart".into(),
                        pivot: vec![0.0, 0.0],
                        px: [96, 96],
                        width: 16,
                        height: 16,
                        field_instances: Vec::new(),
                    },
                    LdtkEntityInstance {
                        iid: "Switch-test-kernel".into(),
                        identifier: "Switch".into(),
                        pivot: vec![0.0, 0.0],
                        px: [606, 736],
                        width: 68,
                        height: 24,
                        field_instances: fields,
                    },
                ],
                int_grid_csv: Vec::new(),
                grid_tiles: Vec::new(),
            }],
        }],
    }
}

/// The fixture project, CONVERTED — the road production takes.
///
/// the fixture stays an `LdtkProject` on purpose: the command line comes off the room IR now,
/// and converting here means a `convert_switch` that stops emitting `on_activate` fails in
/// these tests rather than at runtime.
fn room_with_one_switch(
    on_activate: Option<&str>,
    room_id: &str,
) -> ambition_platformer2d_world::rooms::RoomSpec {
    project_with_one_switch(on_activate)
        .to_room_set_with_entry(
            "symmetry_room",
            &ambition_platformer2d_ldtk::LdtkVocabulary::engine(),
        )
        .unwrap_or_else(|errors| panic!("fixture converts to rooms: {errors:?}"))
        .rooms
        .into_iter()
        .find(|room| room.id == room_id)
        .unwrap_or_else(|| {
            ambition_platformer2d_world::rooms::RoomSpec::new(
                room_id,
                ambition_platformer2d_core::World::new(
                    room_id,
                    ambition_platformer2d_core::Vec2::new(1024.0, 768.0),
                    ambition_platformer2d_core::Vec2::new(96.0, 96.0),
                    Vec::new(),
                ),
            )
        })
}

/// The walk finds an authored switch verb.
#[test]
fn an_authored_switch_verb_is_found_with_its_switch_id() {
    let found = authored_switch_commands(&room_with_one_switch(Some(LINE), "symmetry_room"));
    assert_eq!(
        found,
        vec![AuthoredSwitchCommand {
            switch_id: SWITCH.to_string(),
            line: LINE.to_string(),
        }]
    );
}

/// A `Switch` with no `on_activate` is not this system's business.
///
/// every switch shipped before this field existed is one of these — the
/// encounter arming gate, the reset path, the sand sim — and they must keep
/// working untouched.
#[test]
fn a_switch_with_no_authored_verb_is_left_to_its_other_consumers() {
    assert!(authored_switch_commands(&room_with_one_switch(None, "symmetry_room")).is_empty());
}

/// Only the active room's switches.
///
/// this is what the deleted const table's comment worried about — *"a gravity
/// switch authored in some other room must not count"* — and the authored form
/// gets it for free: location comes with the placement.
#[test]
fn switches_in_another_room_are_not_found() {
    assert!(authored_switch_commands(&room_with_one_switch(Some(LINE), "drain_alley")).is_empty());
}

// ── the seam ────────────────────────────────────────────────────────────────

/// The verb the test's own domain publishes. a domain the engine has never
/// heard of, so nothing below can be passing because it named something real.
#[derive(Resource, Default)]
struct Bell(Vec<String>);

fn ring(world: &mut World, args: &[AuthoredArg]) -> CommandOutcome {
    let Some(note) = args[0].as_name() else {
        return CommandOutcome::refused("not a name");
    };
    world.resource_mut::<Bell>().0.push(note.to_string());
    CommandOutcome::Done
}

fn ring_descriptor() -> CommandDescriptor {
    CommandDescriptor {
        id: CommandId::new("bystander", "ring"),
        summary: "ring a bell",
        params: &[ParamSpec {
            name: "note",
            kind: ParamKind::Name,
            summary: "which note",
        }],
    }
}

fn world_with_one_authored_switch(on_activate: Option<&str>) -> App {
    let mut app = App::new();
    app.insert_resource(ActiveLdtkProject(project_with_one_switch(on_activate)));
    app.init_resource::<Bell>()
        .init_resource::<AuthoredSwitchCommands>()
        .add_message::<ambition_encounter::switches::SwitchActivated>()
        .add_message::<RunAuthoredCommand>()
        .publish_command(ring_descriptor(), ring);
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "symmetry_room",
            vec![room_with_one_switch(on_activate, "symmetry_room")],
            Vec::new(),
        ),
    );
    app.add_systems(
        Update,
        (
            prepare_authored_switch_commands,
            request_authored_switch_commands,
            ambition_platformer2d_shared_tangle::authored_logic::commands::run_requested_authored_commands,
        )
            .chain(),
    );
    app
}

fn press(app: &mut App, switch_id: &str) {
    app.world_mut()
        .write_message(ambition_encounter::switches::SwitchActivated {
            activation: ambition_encounter::SwitchActivation {
                id: switch_id.to_string(),
                action: "SetGravityDown".to_string(),
                target_encounter: String::new(),
            },
            pos: ambition_platformer2d_core::Vec2::ZERO,
        });
}

fn rung(app: &App) -> &[String] {
    &app.world().resource::<Bell>().0
}

/// PRESSING AN AUTHORED SWITCH ASKS FOR THE VERB THE LEVEL NAMED — AND NOTHING
/// IN THIS FILE NAMES IT.
///
/// both terms are observed: the bell is silent on a frame with no
/// activation, and rings on the frame with one. A version asserting only the end
/// state would pass with the activation channel ignored entirely.
#[test]
fn pressing_an_authored_switch_asks_for_the_verb_the_level_named() {
    let mut app = world_with_one_authored_switch(Some(LINE));

    app.update();
    assert!(
        rung(&app).is_empty(),
        "the verb happened without anybody pressing the switch"
    );
    assert_eq!(
        app.world().resource::<AuthoredSwitchCommands>().len(),
        1,
        "the room's one authored line prepared"
    );

    press(&mut app, SWITCH);
    app.update();
    assert_eq!(rung(&app), ["C"]);

    // A quiet frame afterwards: one press, one ring.
    app.update();
    assert_eq!(
        rung(&app),
        ["C"],
        "the verb repeated without a second press"
    );
}

/// A SWITCH THE LEVEL DID NOT GIVE A VERB ASKS FOR NOTHING.
#[test]
fn pressing_a_switch_with_no_authored_verb_asks_for_nothing() {
    let mut app = world_with_one_authored_switch(None);
    press(&mut app, SWITCH);
    app.update();
    assert!(rung(&app).is_empty());
    assert!(app.world().resource::<AuthoredSwitchCommands>().is_empty());
}

/// AN UNPERFORMABLE LINE IS REFUSED WHEN THE ROOM IS PREPARED, NOT WHEN THE
/// SWITCH IS PRESSED.
///
/// this is the acceptance clause "validation occurs before runtime", stated
/// as a behaviour rather than as a claim. The line below names a verb no
/// composition publishes. It is rejected while the room's rules are read — so it
/// is never in the prepared set at all, and the press finds nothing rather than
/// discovering the problem mid-tick.
///
/// the good line is exercised in the same test, because a preparer that
/// prepared NOTHING would pass the negative half and be just as broken.
#[test]
fn a_line_no_composition_can_perform_never_reaches_the_prepared_set() {
    let mut app = world_with_one_authored_switch(Some("nobody.cares C"));
    app.update();
    assert!(
        app.world().resource::<AuthoredSwitchCommands>().is_empty(),
        "an unpublished verb was prepared anyway; then preparation validates nothing"
    );
    press(&mut app, SWITCH);
    app.update();
    assert!(rung(&app).is_empty());

    let mut good = world_with_one_authored_switch(Some(LINE));
    good.update();
    assert_eq!(
        good.world().resource::<AuthoredSwitchCommands>().len(),
        1,
        "the same preparer must still accept a line the catalog can perform"
    );
}

#[test]
fn an_argument_the_descriptor_does_not_declare_is_refused_when_the_room_is_read() {
    let mut app = world_with_one_authored_switch(Some("bystander.ring C sharp"));
    app.update();
    assert!(app.world().resource::<AuthoredSwitchCommands>().is_empty());
    press(&mut app, SWITCH);
    app.update();
    assert!(rung(&app).is_empty());
}

/// A REPLACED ROOM SET INVALIDATES THE PREPARED CALLS.
///
/// carried across from the sibling system, which shipped without it once: a
/// hot reload that swaps the authored source under an unchanged room id kept
/// serving rules computed from content that is no longer loaded.
///
/// This is the test that says the signal followed rather than being dropped on the way.
#[test]
fn swapping_the_room_set_alone_invalidates_the_prepared_calls() {
    let mut app = world_with_one_authored_switch(Some(LINE));
    app.update();
    assert_eq!(app.world().resource::<AuthoredSwitchCommands>().len(), 1);

    // A quiet frame keeps the prepared set — this is what makes the assertion
    // below about invalidation rather than about recomputation.
    app.update();
    assert_eq!(app.world().resource::<AuthoredSwitchCommands>().len(), 1);

    {
        let mut rooms = app.world_mut().query_filtered::<
            &mut ambition_platformer2d_world::rooms::RoomSet,
            bevy::prelude::With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
        >();
        let mut set = rooms
            .iter_mut(app.world_mut())
            .next()
            .expect("the fixture installs a room set");
        for room in &mut set.rooms {
            room.switch_commands.clear();
        }
    }
    app.update();
    assert!(
        app.world().resource::<AuthoredSwitchCommands>().is_empty(),
        "the prepared set must track the replaced room set"
    );
}
