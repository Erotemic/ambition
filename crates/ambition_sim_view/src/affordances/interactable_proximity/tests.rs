//! The prompt names what the press reaches: the same live room, door rule and
//! facing gate as the interact systems (review 2026-10-08, findings 2 and 3).

use super::*;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};

/// Where the body stands. It is 24 by 40 and faces right.
const BODY: ae::Vec2 = ae::Vec2::new(100.0, 100.0);

/// One thing in reach of the body, for one arm.
#[derive(Clone, Copy)]
enum Thing {
    Npc,
    Chest,
    Switch,
    /// A switch the body must face.
    GatedSwitch,
}

/// One arm: a thing at `offset` from the body, in the body's live room or the
/// other one, with a door at `door` from the body or no door.
#[derive(Clone, Copy)]
struct Arm {
    thing: Thing,
    offset: f32,
    in_the_other_room: bool,
    door: Option<f32>,
}

impl Arm {
    fn beside(thing: Thing) -> Self {
        Self {
            thing,
            offset: 10.0,
            in_the_other_room: false,
            door: None,
        }
    }
}

/// The prompt the body gets in one arm. Two live rooms are live, and each
/// instantiates the one room, whose only door is at `arm.door`.
fn prompt(arm: Arm) -> InteractVariant {
    let mut app = App::new();
    let mut room = ambition_platformer2d_world::rooms::RoomSpec::new(
        "room",
        ae::World::new("room", ae::Vec2::new(640.0, 480.0), ae::Vec2::ZERO, Vec::new()),
    );
    if let Some(door) = arm.door {
        room.loading_zones = vec![ambition_platformer2d_world::rooms::LoadingZone {
            id: "door".into(),
            name: "door".into(),
            activation: ambition_platformer2d_world::rooms::LoadingZoneActivation::Door,
            aabb: ae::Aabb::new(BODY + ae::Vec2::new(door, 0.0), ae::Vec2::new(24.0, 24.0)),
        }];
    }
    let set = ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic("room", vec![room], Vec::new());
    let definition = set.activation_definition();
    ambition_platformer2d_world::rooms::insert_room_set(app.world_mut(), set);
    let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
    for room in rooms {
        app.world_mut().spawn((RoomInstanceRoot, room, definition));
    }
    app.init_resource::<NearestInteractable>();
    app.add_systems(Update, update_nearest_interactable);

    app.world_mut().spawn((
        ambition_platformer2d_core::BodyKinematics {
            pos: BODY,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(24.0, 40.0),
            facing: 1.0,
        },
        ambition_platformer2d_shared_tangle::markers::PlayerEntity,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
        InRoomInstance(rooms[0]),
    ));
    let at = BODY + ae::Vec2::new(arm.offset, 0.0);
    let aabb = ae::Aabb::new(at, ae::Vec2::new(16.0, 24.0));
    let thing = match arm.thing {
        Thing::Npc => {
            let interactable = ambition_interaction::Interactable::new(
                "guide",
                "Talk",
                aabb,
                ambition_interaction::InteractionKind::Npc {
                    character_id: None,
                    dialogue_id: Some("hub_guide".into()),
                    patrol_radius: 0.0,
                    patrol_path_id: None,
                    brain_override: None,
                },
            );
            app.world_mut()
                .spawn((
                    ActorInteraction { interactable },
                    ActorDisposition::Peaceful,
                ))
                .id()
        }
        Thing::Chest => app
            .world_mut()
            .spawn(ChestFeature::new(ambition_interaction::Chest::new("chest", None)))
            .id(),
        Thing::Switch | Thing::GatedSwitch => app
            .world_mut()
            .spawn(SwitchFeature::new(ambition_encounter::SwitchActivation {
                id: "switch".into(),
                action: "open".into(),
                target_encounter: String::new(),
            }))
            .id(),
    };
    app.world_mut().entity_mut(thing).insert((
        FeatureSimEntity,
        CenteredAabb::from_center_size(at, ae::Vec2::new(16.0, 24.0)),
        InRoomInstance(rooms[usize::from(arm.in_the_other_room)]),
    ));
    if matches!(arm.thing, Thing::GatedSwitch) {
        app.world_mut()
            .entity_mut(thing)
            .insert(ambition_combat::components::RequiresFacing);
    }
    app.update();
    app.world().resource::<NearestInteractable>().0.clone()
}

/// ⭐ A DOOR NEARER THAN THE PERSON TAKES THE PRESS, SO THE PROMPT DOES NOT
/// SAY TALK. The body stands in a door 2 px away, and the person is 10 px
/// away: the press is for the door. The controls: with no door the prompt
/// says Talk, and with the person nearer than the door it says Talk.
#[test]
fn a_door_nearer_than_the_person_takes_the_talk_prompt() {
    let npc = Arm::beside(Thing::Npc);
    assert_eq!(
        (
            prompt(Arm { door: Some(2.0), ..npc }),
            prompt(npc),
            prompt(Arm { door: Some(16.0), ..npc }),
        ),
        (InteractVariant::None, InteractVariant::Talk, InteractVariant::Talk),
        "(door at 2 px, no door, door at 16 px), with the person at 10 px: the prompt"
    );
}

/// ⭐ A THING IN THE OTHER LIVE ROOM GIVES NO PROMPT. Two live rooms share
/// one coordinate space, so a person, a chest or a switch of the other room
/// can stand where the body stands. The press does not reach them (OW1 cut
/// 7b), so the prompt does not name them. The controls: each in the body's
/// own room.
#[test]
fn a_thing_in_the_other_live_room_gives_no_prompt() {
    let prompts = |in_the_other_room: bool| {
        [Thing::Npc, Thing::Chest, Thing::Switch].map(|thing| {
            prompt(Arm {
                in_the_other_room,
                ..Arm::beside(thing)
            })
        })
    };
    assert_eq!(
        (prompts(true), prompts(false)),
        (
            [InteractVariant::None, InteractVariant::None, InteractVariant::None],
            [InteractVariant::Talk, InteractVariant::Open, InteractVariant::Activate],
        ),
        "([person, chest, switch] in the other live room, the same in the body's own room): the prompt"
    );
}

/// ⭐ Q63: A SWITCH THE BODY MUST FACE GIVES NO PROMPT FROM BEHIND. The body
/// faces right. The controls: the same switch ahead, and an ungated switch
/// behind.
#[test]
fn a_facing_gated_switch_gives_no_prompt_from_behind() {
    let behind = |thing| prompt(Arm { offset: -10.0, ..Arm::beside(thing) });
    assert_eq!(
        (
            behind(Thing::GatedSwitch),
            prompt(Arm::beside(Thing::GatedSwitch)),
            behind(Thing::Switch),
        ),
        (InteractVariant::None, InteractVariant::Activate, InteractVariant::Activate),
        "(gated behind, gated ahead, ungated behind): the prompt"
    );
}
