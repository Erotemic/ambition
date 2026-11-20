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
    ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, "story.play_cutscene",
        ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    // After the queue starts a cutscene and before it ticks: a cutscene with
    // no beats starts and ends on one tick, and is seen playing here.
    let sim = ambition_platformer2d_shared_tangle::schedule::SimScheduleExt::sim_schedule(app);
    app.add_systems(
        sim,
        answer_cutscene_asks
            .run_if(|asks: Query<(), With<CutsceneAsks>>| !asks.is_empty())
            .after(crate::cutscene::drain_cutscene_triggers)
            .before(crate::cutscene::tick_active_cutscene)
            .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::Cutscene),
    );
}

/// A `play_cutscene` with a reply, waiting for its cutscene to start or end.
#[derive(Clone, Debug, PartialEq)]
pub struct CutsceneAsk {
    id: String,
    room: u32,
    ticket: ambition_extension_sdk::action::ActionTicket,
    began: bool,
}
ambition_extension_sdk::port_record! { CutsceneAsk { id: String, room: u32,
    ticket: ambition_extension_sdk::action::ActionTicket, began: bool } }

/// The cutscenes a scope asked for and has not been answered about. It sits
/// on the entity whose mailbox holds the tickets, so it rewinds with them.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct CutsceneAsks { rows: Vec<CutsceneAsk> }
ambition_extension_sdk::port_record! { CutsceneAsks { rows: Vec<CutsceneAsk> } }
impl ambition_platformer2d_core::snapshot::SnapshotState for CutsceneAsks {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut bytes = Vec::new();
        ambition_extension_sdk::structure::put(self, &mut bytes);
        ambition_platformer2d_core::snapshot::put_bytes(out, &bytes);
    }
    fn decode(reader: &mut ambition_platformer2d_core::snapshot::Reader<'_>) -> Option<Self> {
        ambition_extension_sdk::wire::decode_all(reader.bytes()?, ambition_extension_sdk::structure::read).ok()
    }
}
/// The most cutscene asks one scope holds unanswered.
pub const MAX_CUTSCENE_ASKS: usize = 8;

pub(crate) fn register_rollback_state(registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar) {
    registrar.rollback_component_canonical::<CutsceneAsks>("ambition_platformer2d_runtime", "script.cutscene_asks");
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
                .is_some_and(|active| active.conversations().any(|live| {
                    live.initiator
                        .or(live.talker)
                        .and_then(|body| rooms.of(body))
                        .is_none_or(|at| at == *room)
                }));
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
            world, submitted.state_scope, &submitted.entry, &submitted.value);
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

fn caller_room(world: &World, scope: Entity) -> Option<LiveRoomInstance> {
    world.get::<LiveRoomInstance>(scope).copied()
        .or_else(|| world.get::<ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>(scope).map(|at| at.0))
}

fn lower_cutscenes(world: &mut World) {
    use ambition_extension_sdk::action::ActionStage;
    use ambition_platformer2d_shared_tangle::action_results::{begin_action_request, publish_action_result};
    let submitted: Vec<_> = world.resource_mut::<ExtensionOutbox>().drain::<PlayCutscenePort>(&ACTOR_THINK);
    for submitted in submitted {
        let dispatch = begin_action_request::<PlayCutscenePort>(world, submitted.state_scope, &submitted.entry, &submitted.value);
        if !dispatch.execute { continue; }
        let id = submitted.value.id;
        let asks_full = dispatch.reply.is_some() && world.get::<CutsceneAsks>(submitted.state_scope)
            .is_some_and(|asks| asks.rows.len() >= MAX_CUTSCENE_ASKS);
        let checked = (|| -> Result<u32, &'static str> {
            let script = world.resource::<ambition_cutscene::CutsceneLibrary>().get(&id).ok_or("unknown_cutscene")?;
            let room = caller_room(world, submitted.scope).ok_or("no_room")?;
            if script.seen_flag.as_deref().is_some_and(|seen| world.get_resource::<ambition_persistence::save::AmbitionGameSave>()
                .is_some_and(|save| save.data().flag(seen))) {
                return Err("cutscene_already_seen");
            }
            if asks_full { return Err("cutscene_ask_queue_full"); }
            Ok(room.ordinal())
        })();
        match checked {
            Ok(room) => {
                world.resource_mut::<ambition_cutscene::CutsceneTriggerQueue>().request_in(id.clone(), room);
                if let Some(reply) = dispatch.reply.as_ref() {
                    world.entity_mut(submitted.state_scope).entry::<CutsceneAsks>().or_default().into_mut().rows
                        .push(CutsceneAsk {id, room, ticket: reply.ticket.clone(), began: false});
                    publish_action_result(world, Some(reply), ActionStage::Admitted, false, "cutscene_queued", "", Vec::new());
                }
            }
            Err(code) => {
                if dispatch.reply.is_none() {
                    warn!("cutscene {id:?} from {} refused: {code}", submitted.entry);
                }
                publish_action_result(world, dispatch.reply.as_ref(), ActionStage::Rejected, true, code, "", Vec::new());
            }
        }
    }
}

/// Answer each asked cutscene from playback: `started` the first tick it is
/// seen playing, and terminal `cutscene_ended` the first tick after it stops.
/// One that is neither playing nor waiting in the queue and never began was
/// passed over by the queue (its seen flag was set meanwhile): `cutscene_not_played`.
fn answer_cutscene_asks(world: &mut World) {
    use ambition_extension_sdk::action::{ActionResult, ActionStage};
    use ambition_platformer2d_shared_tangle::action_results::ActionMailbox;
    let now = world.get_resource::<ambition_time::SimTick>().map_or(0, |tick| tick.0);
    let playing = world.get_resource::<ambition_cutscene::ActiveCutscene>().and_then(|active| {
        Some((active.runtime.as_ref()?.script.id.clone(), active.room?))
    });
    let queued = world.get_resource::<ambition_cutscene::CutsceneTriggerQueue>().map(|queue| queue.0.clone()).unwrap_or_default();
    let replay = world.get_resource::<ambition_platformer2d_shared_tangle::lifecycle::RoomReplay>().cloned();
    let mut holders = world.query_filtered::<Entity, With<CutsceneAsks>>();
    let holders: Vec<Entity> = holders.iter(world).collect();
    for holder in holders {
        let mut asks = world.get::<CutsceneAsks>(holder).unwrap().clone();
        let mut answers = Vec::new();
        asks.rows.retain_mut(|ask| {
            if replay.as_ref().is_some_and(|replay| !replay.allows(Some(LiveRoomInstance::from_ordinal(ask.room)))) {
                return true;
            }
            let plays = playing.as_ref().is_some_and(|(id, room)| *id == ask.id && *room == ask.room);
            let waits = queued.iter().any(|trigger| trigger.id == ask.id && trigger.room == Some(ask.room));
            if plays {
                if !ask.began {
                    ask.began = true;
                    answers.push((ask.ticket.clone(), ActionStage::Applied, false, "cutscene_started"));
                }
                true
            } else if ask.began {
                answers.push((ask.ticket.clone(), ActionStage::Completed, true, "cutscene_ended"));
                false
            } else if waits {
                true
            } else {
                answers.push((ask.ticket.clone(), ActionStage::Failed, true, "cutscene_not_played"));
                false
            }
        });
        let Some(mut mailbox) = world.get_mut::<ActionMailbox>(holder) else {
            world.entity_mut(holder).remove::<CutsceneAsks>();
            continue;
        };
        // A canceled or expired ticket is no longer waited for.
        asks.rows.retain(|ask| mailbox.is_pending(&ask.ticket));
        for (ticket, stage, terminal, code) in answers {
            mailbox.publish(ActionResult {ticket, stage, terminal, tick: now, code: code.into(),
                message: String::new(), subjects: Vec::new()});
        }
        if *world.get::<CutsceneAsks>(holder).unwrap() != asks {
            *world.get_mut::<CutsceneAsks>(holder).unwrap() = asks;
        }
    }
}
