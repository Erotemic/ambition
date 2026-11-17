//! Room story scripts read simulation facts and request domain actions.
//! Each invocation belongs to one live room. Records use the room root.

use ambition_characters::actor::{BodyCombat, BodyHealth, CharacterForm};
use ambition_characters::control::DrivingParticipant;
use ambition_extension_host::{
    AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox,
};
use ambition_extension_sdk::wire::{WireError, WireReader};
use ambition_extension_sdk::{phases::ACTOR_THINK, Port, PortKey, PortRole};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::{
    authored_logic::{AuthoredAsk, CommandCatalog, RunAuthoredCommand},
    lifecycle::{LiveRoomInstance, LiveRooms, RoomFreeze, RoomInstanceRoot},
    sim_id::SimId,
};
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use bevy::prelude::*;

/// One `actor_think` invocation for each live instance of the selected room
/// definition. Its scope is the room root. No invocation runs while its
/// room clock is held. The trigger contains simulation facts only.
pub struct StoryTickPort;
/// Submit an authored command in the invocation's room. Lowering prepares
/// the line against `CommandCatalog`. A bad line or absent room is refused
/// and logged. The command owner applies the effect through its usual path.
pub struct StoryCommandPort;
/// Queue a known cutscene in the invocation's room. A missing library id
/// is refused and logged. Playback owns the queue, skip policy and cursor.
pub struct PlayCutscenePort;

#[derive(Clone, Debug, PartialEq)]
pub struct StoryTick {
    pub busy: bool,
    pub flags: Vec<String>,
    pub bodies: Vec<StoryBody>,
    pub marks: Vec<StoryMark>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoryBody {
    pub id: String,
    pub form: String,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub on_ground: bool,
    pub alive: bool,
    pub driven: bool,
    pub struck_recently: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoryCommand {
    pub line: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayCutscene {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoryMark {
    pub name: String,
    pub position: [f32; 2],
}

ambition_extension_sdk::port_record! {
    StoryTick {
        /// A conversation or cutscene currently holds this room.
        busy: bool,
        /// The save flags that are true, sorted by name.
        flags: Vec<String>,
        /// This room's bodies, sorted by simulation identity.
        bodies: Vec<StoryBody>,
        /// Named LDtk labels and their points, in room pixels.
        marks: Vec<StoryMark>,
    }
}
ambition_extension_sdk::port_record! {
    StoryMark {
        /// The authored label name.
        name: String,
        /// The label's point in room pixels.
        position: [f32; 2],
    }
}
ambition_extension_sdk::port_record! {
    StoryBody {
        /// The body's simulation identity.
        id: String,
        /// The character id.
        form: String,
        /// The body's centre, in room pixels with +Y down.
        position: [f32; 2],
        /// The body's width and height.
        size: [f32; 2],
        /// The shared movement system reports ground contact.
        on_ground: bool,
        /// The body is alive.
        alive: bool,
        /// A participant drives this body.
        driven: bool,
        /// The shared hit resolver reports a recent strike on this body.
        struck_recently: bool,
    }
}
ambition_extension_sdk::port_record! {
    StoryCommand {
        /// A command line from the authored command catalog. It runs in this room.
        line: String,
    }
}
ambition_extension_sdk::port_record! {
    PlayCutscene {
        /// A script id from the cutscene library. It plays in this room.
        id: String,
    }
}

macro_rules! story_port {
    ($port:ty, $key:literal, $role:ident, $value:ty) => {
        impl Port for $port {
            const KEY: PortKey = PortKey::new($key, 1);
            const ROLE: PortRole = PortRole::$role;
            type Value = $value;
            fn encode(value: &Self::Value, out: &mut Vec<u8>) {
                ambition_extension_sdk::structure::put(value, out)
            }
            fn decode(reader: &mut WireReader<'_>) -> Result<Self::Value, WireError> {
                ambition_extension_sdk::structure::read(reader)
            }
        }
        ambition_extension_sdk::script_port!($port);
    };
}
story_port!(StoryTickPort, "ambition.story.tick", Trigger, StoryTick);
story_port!(
    StoryCommandPort,
    "ambition.story.command",
    Request,
    StoryCommand
);
story_port!(
    PlayCutscenePort,
    "ambition.story.play_cutscene",
    Request,
    PlayCutscene
);

pub fn install(app: &mut App) {
    app.install_extension_trigger::<StoryTickPort, _>(
        ACTOR_THINK,
        "ambition_platformer2d_runtime",
        queue_story_ticks.run_if(ambition_extension_host::trigger_is_bound::<StoryTickPort>),
    );
    app.install_extension_submitted_request::<StoryCommandPort, _>(
        ACTOR_THINK,
        "ambition_platformer2d_runtime",
        lower_commands,
    );
    app.install_extension_submitted_request::<PlayCutscenePort, _>(
        ACTOR_THINK,
        "ambition_platformer2d_runtime",
        lower_cutscenes,
    );
}

/// The selector is a room definition id. Each live instance gets its own invocation.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn queue_story_ticks(
    admitted: Res<AdmittedExtensions>,
    time: Res<ambition_time::WorldTime>,
    mut invocations: ResMut<ExtensionInvocations>,
    roots: Query<(Entity, &LiveRoomInstance), With<RoomInstanceRoot>>,
    bodies: Query<(
        Entity,
        &SimId,
        &CharacterForm,
        &ae::BodyKinematics,
        &ae::BodyGroundState,
        &BodyHealth,
        &BodyCombat,
        Has<DrivingParticipant>,
    )>,
    rooms: LiveRooms,
    specs: LiveRoomSpecs,
    pace: RoomFreeze,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    conversation: Option<Res<ambition_conversation::ActiveConversation>>,
    cutscene: Res<ambition_cutscene::ActiveCutscene>,
) {
    if time.sim_dt() <= 0.0 {
        return;
    }
    let selectors: std::collections::BTreeSet<_> = admitted
        .0
        .entries
        .iter()
        .filter(|entry| entry.descriptor.trigger.port == StoryTickPort::KEY)
        .map(|entry| entry.descriptor.trigger.selector.to_string())
        .collect();
    let mut roots: Vec<_> = roots.iter().collect();
    roots.sort_by_key(|(_, room)| **room);
    for (root, room) in roots {
        if pace.scale_of(root) <= 0.0 {
            continue;
        }
        let Some(spec) = specs
            .spec_of(root)
            .filter(|spec| selectors.contains(&spec.id))
        else {
            continue;
        };
        let busy = cutscene.is_playing()
            && cutscene
                .room
                .is_none_or(|ordinal| ordinal == room.ordinal())
            || conversation
                .as_ref()
                .and_then(|active| active.live())
                .is_some_and(|live| {
                    live.initiator
                        .or(live.talker)
                        .and_then(|body| rooms.of(body))
                        .is_none_or(|at| at == *room)
                });
        let mut members: Vec<_> = bodies
            .iter()
            .filter(|(body, ..)| rooms.of(*body) == Some(*room))
            .map(
                |(_, id, form, kin, ground, health, combat, driven)| StoryBody {
                    id: id.as_str().to_owned(),
                    form: form.id().to_owned(),
                    position: [kin.pos.x, kin.pos.y],
                    size: [kin.size().x, kin.size().y],
                    on_ground: ground.on_ground,
                    alive: health.alive(),
                    driven,
                    struck_recently: combat.recently_struck(),
                },
            )
            .collect();
        members.sort_by(|a, b| a.id.cmp(&b.id));
        let mut flags: Vec<_> = save
            .data()
            .flags()
            .iter()
            .filter(|flag| flag.on)
            .map(|flag| flag.id.clone())
            .collect();
        flags.sort();
        let mut marks: Vec<_> = spec
            .debug_labels
            .iter()
            .map(|label| {
                let at = label.payload.position;
                StoryMark {
                    name: label.name.clone(),
                    position: [at.x, at.y],
                }
            })
            .collect();
        marks.sort_by(|a, b| a.name.cmp(&b.name));
        invocations.trigger::<StoryTickPort>(
            &ACTOR_THINK,
            spec.id.clone(),
            root,
            None,
            false,
            StoryTick {
                busy,
                flags,
                bodies: members,
                marks,
            },
        );
    }
}

fn lower_commands(
    mut outbox: ResMut<ExtensionOutbox>,
    catalog: Res<CommandCatalog>,
    rooms: LiveRooms,
    mut commands: MessageWriter<RunAuthoredCommand>,
) {
    for submitted in outbox.drain::<StoryCommandPort>(&ACTOR_THINK) {
        let Some(room) = rooms.of(submitted.scope) else {
            warn!(
                "story command from {} refused: its scope has no live room",
                submitted.entry
            );
            continue;
        };
        match catalog.prepare_line(&submitted.value.line) {
            Ok(prepared) => {
                commands.write(
                    RunAuthoredCommand::prepared(
                        &prepared,
                        AuthoredAsk::new("script", submitted.entry.to_string()),
                    )
                    .in_room(Some(room)),
                );
            }
            Err(problem) => warn!("story command from {} refused: {problem}", submitted.entry),
        }
    }
}

fn lower_cutscenes(
    mut outbox: ResMut<ExtensionOutbox>,
    library: Res<ambition_cutscene::CutsceneLibrary>,
    rooms: LiveRooms,
    mut queue: ResMut<ambition_cutscene::CutsceneTriggerQueue>,
) {
    for submitted in outbox.drain::<PlayCutscenePort>(&ACTOR_THINK) {
        if !library.scripts.contains_key(&submitted.value.id) {
            warn!(
                "cutscene {:?} from {} refused: no script has this id",
                submitted.value.id, submitted.entry
            );
            continue;
        }
        if let Some(room) = rooms.of(submitted.scope) {
            queue.request_in(submitted.value.id, room.ordinal());
        }
    }
}
