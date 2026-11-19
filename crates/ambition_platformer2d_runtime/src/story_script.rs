//! Room story scripts read simulation facts and request domain actions.
//! Each invocation belongs to one live room. Records use the room root.

use ambition_characters::actor::{BodyCombat, BodyHealth, CharacterForm};
use ambition_characters::control::DrivingParticipant;
use ambition_extension_host::{
    AdmittedExtensions, ExtensionAppExt, ExtensionInvocations, ExtensionOutbox,
};
use ambition_extension_sdk::{phases::ACTOR_THINK, Port};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::{
    authored_logic::{AuthoredAsk, CommandCatalog, RunAuthoredCommand},
    lifecycle::{LiveRoomInstance, LiveRooms, RoomFreeze, RoomInstanceRoot},
    sim_id::SimId,
};
use ambition_platformer2d_world::rooms::LiveRoomSpecs;
use bevy::prelude::*;

use ambition_world_port::*;

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
    #[cfg(feature = "dialogue")]
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
                .is_none_or(|ordinal| ordinal == room.ordinal());
        #[cfg(feature = "dialogue")]
        let busy = busy || conversation
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

fn lower_commands(world: &mut World) {
    use ambition_platformer2d_shared_tangle::action_results::publish_action_result;
    use ambition_extension_sdk::action::ActionStage;
    let submitted: Vec<_> = world.resource_mut::<ExtensionOutbox>().drain::<StoryCommandPort>(&ACTOR_THINK);
    for submitted in submitted {
        let dispatch = ambition_platformer2d_shared_tangle::action_results::begin_action_request::<StoryCommandPort>(
            world, submitted.scope, &submitted.entry, &submitted.value);
        if !dispatch.execute { continue; }
        let room = world.get::<LiveRoomInstance>(submitted.scope).copied()
            .or_else(|| world.get::<ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>(submitted.scope).map(|at| at.0));
        let prepared = room.ok_or_else(|| "The caller has no live room.".to_owned())
            .and_then(|_| world.resource::<CommandCatalog>().prepare_line(&submitted.value.line).map_err(|problem| problem.to_string()));
        match prepared {
            Ok(prepared) => {
                world.write_message(RunAuthoredCommand::prepared(&prepared,
                    AuthoredAsk::new("script", submitted.entry.to_string())).in_room(room).with_reply(dispatch.reply));
            }
            Err(problem) => {
                publish_action_result(world, dispatch.reply.as_ref(), ActionStage::Rejected, true,
                    "command_invalid", problem.clone(), Vec::new());
                warn!("story command from {} refused: {problem}", submitted.entry);
            }
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
