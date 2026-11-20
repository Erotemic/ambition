//! The committed world road between rollback islands: `post_world_signal`.
//!
//! A room that rewinds alone (SIMULATION-ISLANDS) can run a tick again with
//! other inputs, so what it sends to another room must wait until no replay
//! can take the sending tick back. A post is held with its room and tick
//! ([`RoomWrite`]). A rewind of the sending room drops the posts it made after
//! its save, and the replay posts them again. Once the confirmed-frame
//! boundary passes the sending tick, the next live tick admits the post to
//! every method subscribed to the sender's signal on its topic, wherever each
//! subscriber is then, and logs the admission. A replay of a recipient's room
//! that runs the admission tick queues the row again into the mailbox its
//! rewind restored, so each timeline holds the post exactly once.
//!
//! Without islands (one rollback authority: local play, GGRS) sender and
//! subscribers share one timeline, and a post is delivered at once.
//! The contract is `docs/planning/engine/addressed-behavior-inbox.md`,
//! "Committed world boundary between islands".

use ambition_extension_sdk::{ObjectRef, Port};
use ambition_platformer2d_core::ConfirmedFrameBoundary;
use ambition_platformer2d_shared_tangle::lifecycle::*;
use ambition_world_port::behavior::*;
use bevy::{ecs::world::DeferredWorld, prelude::*};

/// The most posts held for confirmation at once, across the world.
pub const MAX_HELD_WORLD_POSTS: usize = 64;

#[derive(Clone, Debug)]
struct Held { write: RoomWrite, source: ObjectRef, event: String, bytes: Vec<u8> }

/// One row the world road queued: replayed when its recipient's room runs
/// `tick` again.
#[derive(Clone, Debug)]
struct Admitted { tick: u64, recipient: ObjectRef, key: String, source: ObjectRef, event: String, bytes: Vec<u8> }

/// The world road's posts: held until confirmed, and admitted, by tick.
#[derive(Resource, Default)]
pub struct WorldPosts { held: Vec<Held>, admitted: Vec<Admitted> }

impl WorldPosts {
    /// Posts waiting for their sending tick to be confirmed.
    pub fn held(&self) -> usize { self.held.len() }
    /// Rows queued by the road that a room replay can still run again.
    pub fn admitted(&self) -> usize { self.admitted.len() }
}

/// Hold or deliver each post, and answer its sender. The sender is the object
/// its script runs for; a script cannot name another.
fn post_world_signals<L: ambition_extension_host::LowersIn>(world: &mut World) {
    use ambition_platformer2d_shared_tangle::action_results::{begin_action_request, publish_action_result};
    use ambition_extension_sdk::action::ActionStage;
    let requests = world.resource_mut::<ambition_extension_host::ExtensionOutbox>().drain::<PostWorldSignalPort>(&L::PHASE);
    for submitted in requests {
        let request = submitted.value;
        let dispatch = begin_action_request::<PostWorldSignalPort>(world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute { continue; }
        let result = (|| -> Result<&'static str, &'static str> {
            if !valid_signal_topic(&request.topic) { return Err("invalid_signal_topic"); }
            ensure_object_occurrence(world, submitted.scope).ok_or("emitter_without_identity")?;
            let from = object_reference(world, submitted.scope).ok_or("emitter_without_identity")?;
            let authority = simulation_island(world, submitted.scope).ok_or("emitter_without_authority")?;
            let bound = crate::object_behaviors::limits(world, submitted.scope).max_data_bytes;
            let data = crate::object_behaviors::checked_data(&request.data, bound).ok_or("signal_data_bound")?;
            let event = crate::behavior_events::signal_event(world, &request.topic).ok_or("signals_not_installed")?;
            let mut bytes = Vec::new();
            ObjectSignalPort::encode(&ObjectSignal {from, topic: request.topic.clone(), data}, &mut bytes);
            let SimulationIsland::Room(_, room) = authority else {
                // One timeline: every subscriber hears it now.
                deliver(world, from, &event, &bytes, None);
                return Ok("world_signal_delivered");
            };
            if world.resource::<WorldPosts>().held.len() >= MAX_HELD_WORLD_POSTS { return Err("world_post_queue_full"); }
            let tick = world.resource::<ambition_time::SimTick>().0;
            world.resource_mut::<WorldPosts>().held.push(Held {write: RoomWrite {room, tick}, source: from, event, bytes});
            Ok("world_signal_held")
        })();
        let (stage, code) = match result {
            Ok(code) => (ActionStage::Applied, code),
            Err(code) => (ActionStage::Rejected, code),
        };
        publish_action_result(world, dispatch.reply.as_ref(), stage, true, code, "", Vec::new());
    }
}

/// Queue `bytes` from `source` for every method subscribed to `event`, where
/// each is now; with `log`, record each row against this tick.
fn deliver(world: &mut World, source: ObjectRef, event: &str, bytes: &[u8], mut log: Option<&mut Vec<Admitted>>) {
    let mut deferred = DeferredWorld::from(&mut *world);
    let recipients = crate::behavior_events::recipients(&deferred, source, event);
    let tick = deferred.resource::<ambition_time::SimTick>().0;
    for (recipient, entity, key) in recipients {
        if !crate::behavior_events::queue_row(&mut deferred, entity, source, event, key.clone(), bytes) {
            warn!(?source, ?recipient, event, "A world post found its subscriber's mailbox full.");
            continue;
        }
        if let Some(log) = log.as_deref_mut() {
            log.push(Admitted {tick, recipient, key, source, event: event.to_owned(), bytes: bytes.to_vec()});
        }
    }
}

/// On a live tick: admit every held post whose sending tick is confirmed.
fn admit_confirmed_posts(world: &mut World) {
    let boundary = world.get_resource::<ConfirmedFrameBoundary>().copied();
    let now = world.resource::<ambition_time::SimTick>().0;
    let mut posts = world.remove_resource::<WorldPosts>().unwrap_or_default();
    posts.admitted.retain(|row| row.tick.saturating_add(ROOM_REWIND_HORIZON_TICKS) >= now);
    let (ready, waiting): (Vec<Held>, Vec<Held>) = std::mem::take(&mut posts.held).into_iter()
        .partition(|post| boundary.is_none_or(|boundary| boundary.is_confirmed(post.write.tick as i32)));
    posts.held = waiting;
    for post in ready {
        deliver(world, post.source, &post.event, &post.bytes, Some(&mut posts.admitted));
    }
    world.insert_resource(posts);
}

/// While a room replays: queue again each row the road admitted to that room
/// on the tick now running. The rewind restored its mailbox from before it.
fn readmit_in_replay(world: &mut World) {
    let Some(replay) = world.get_resource::<RoomReplay>().copied() else { return; };
    let now = world.resource::<ambition_time::SimTick>().0;
    let rows: Vec<Admitted> = world.resource::<WorldPosts>().admitted.iter().filter(|row| row.tick == now).cloned().collect();
    if rows.is_empty() { return; }
    let objects = ObjectIndex::capture(world);
    let mut deferred = DeferredWorld::from(&mut *world);
    for row in rows {
        let Ok(entity) = objects.resolve(SessionScopeId(row.recipient.session), row.recipient) else { continue; };
        if !replay.allows(object_scope_room(&deferred, entity)) { continue; }
        crate::behavior_events::queue_row(&mut deferred, entity, row.source, &row.event, row.key, &row.bytes);
    }
}

/// A rewind of `room` to `saved_at` takes back the posts it made after it.
fn retract_held_posts(world: &mut World, room: LiveRoomInstance, saved_at: u64) {
    if let Some(mut posts) = world.get_resource_mut::<WorldPosts>() {
        posts.held.retain(|post| !post.write.retracted_by(room, saved_at));
    }
}

fn forget_posts(mut posts: ResMut<WorldPosts>) {
    *posts = WorldPosts::default();
}

pub(crate) fn install(app: &mut App) {
    use ambition_platformer2d_shared_tangle::schedule::{SimScheduleExt, Platformer2dSimulationPhase};
    app.init_resource::<WorldPosts>();
    ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, "world.post_signal",
        ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    // A body entry of any phase may post (a status tick, a contact, a hit).
    ambition_extension_host::install_submitted_request_in_body_phases!(app, PostWorldSignalPort,
        "ambition_platformer2d_runtime", post_world_signals);
    register_room_write_retractor(app, "resource.world_posts", retract_held_posts);
    // A new session counts its rooms and ticks from the start again.
    app.add_message::<SessionScopeActivated>();
    app.add_systems(Update, forget_posts.in_set(SessionScopeSet::Activate).run_if(on_message::<SessionScopeActivated>));
    let schedule = app.sim_schedule();
    app.add_systems(schedule, (
        admit_confirmed_posts.run_if(not(resource_exists::<RoomReplay>)),
        readmit_in_replay.run_if(resource_exists::<RoomReplay>),
    ).in_set(Platformer2dSimulationPhase::GameplayEffects));
}
