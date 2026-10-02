//! The seam: a room's authored `while_live` line → a prepared call → one
//! request per tick while the room is live. That the shipped world says its
//! line is pinned in `ambition_content`, and that the composed game starts the
//! attunement is pinned by the app fixture.

use super::*;

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, CommandDescriptor, CommandId, CommandOutcome, ParamKind, ParamSpec, PublishCommand,
};
use ambition_platformer2d_shared_tangle::lifecycle::{
    session_world_component, LiveRoomInstance, RoomInstanceRoot,
};
use ambition_platformer2d_world::rooms::{insert_room_set, RoomSet, RoomSpec};

/// A verb of a domain the engine does not know, so nothing below passes
/// because it named something real.
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

fn room(id: &str, while_live: Option<&str>) -> RoomSpec {
    let mut spec = RoomSpec::new(
        id,
        ambition_platformer2d_core::World::new(
            id,
            ambition_platformer2d_core::Vec2::new(320.0, 240.0),
            ambition_platformer2d_core::Vec2::new(16.0, 16.0),
            Vec::new(),
        ),
    );
    spec.metadata.while_live = while_live.map(str::to_string);
    spec
}

/// `hall` is always live (#0) and names no line. `chamber` names `line` and
/// is live (#1) only when `chamber_live`. Returns the bell after `ticks`.
fn rings(chamber_live: bool, line: &str, ticks: usize) -> Vec<String> {
    let mut app = App::new();
    app.init_resource::<Bell>()
        .init_resource::<AuthoredRoomCommands>()
        .add_message::<RunAuthoredCommand>()
        .publish_command(ring_descriptor(), ring);
    insert_room_set(
        app.world_mut(),
        RoomSet::from_parts_or_panic(
            "hall",
            vec![room("hall", None), room("chamber", Some(line))],
            Vec::new(),
        ),
    );
    let definition = |app: &App, id: &str| {
        session_world_component::<RoomSet>(app.world())
            .and_then(|rooms| rooms.definition_by_id(id))
            .expect("the set has the room")
    };
    let hall = definition(&app, "hall");
    app.world_mut().spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION, hall));
    if chamber_live {
        let chamber = definition(&app, "chamber");
        app.world_mut()
            .spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION.next(), chamber));
    }
    app.add_systems(
        Update,
        (
            prepare_authored_room_commands,
            request_authored_room_commands,
            ambition_platformer2d_shared_tangle::authored_logic::commands::run_requested_authored_commands,
        )
            .chain(),
    );
    for _ in 0..ticks {
        app.update();
    }
    app.world().resource::<Bell>().0.clone()
}

/// The line is asked for on each tick while its room is live, also as one of
/// two live rooms. The control: with only `hall` live, nothing rings.
#[test]
fn a_live_rooms_line_is_asked_for_on_each_tick_it_is_live() {
    assert_eq!(rings(false, "bystander.ring C", 2), Vec::<String>::new());
    assert_eq!(rings(true, "bystander.ring C", 2), vec!["C".to_string(), "C".to_string()]);
}

/// A line this composition cannot perform is dropped at preparation (with a
/// warning), so the room asks for nothing and the tick goes on.
#[test]
fn a_line_that_does_not_prepare_asks_for_nothing() {
    assert_eq!(rings(true, "bystander.whistle C", 2), Vec::<String>::new());
}
