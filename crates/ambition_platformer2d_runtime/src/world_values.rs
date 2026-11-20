//! Named world values: state the durable save keeps and every room reads.
//!
//! A script writes one with `set_world_value` or `add_world_count`. The
//! world domain applies a write on a tick no room can simulate again, the
//! rule `world_posts` keeps for signals: under one rollback authority (local
//! play, GGRS) that is the writing tick itself; a room that rewinds alone
//! (SIMULATION-ISLANDS) has its write held until the confirmed boundary
//! passes the writing tick, and a rewind of that room takes back what it
//! wrote after its save. So a value is never written by a tick that is later
//! simulated differently, and the save, which a room replay does not rewind,
//! is its store.
//!
//! Each applied write is queued in the session's change mailbox, a rollback
//! component of the session root, and the next actor read cut invokes the
//! entries whose `world_value_changed` selector is the key, at the session.

use ambition_extension_sdk::{action::ActionStage, phases::ACTOR_THINK, script::ScriptValue};
use ambition_platformer2d_core::{ConfirmedFrameBoundary, snapshot::{Reader, SnapshotState, put_bytes}};
use ambition_platformer2d_shared_tangle::action_results::{begin_action_request, publish_action_result,
    register_action_domain, ActionCancellationPolicy};
use ambition_platformer2d_shared_tangle::lifecycle::*;
use ambition_world_port::behavior::*;
use bevy::prelude::*;

/// The most world values the save keeps.
pub const MAX_WORLD_VALUES: usize = 256;
/// The most writes held for confirmation at once.
pub const MAX_HELD_WORLD_WRITES: usize = 64;
/// The largest encoded world value.
pub const MAX_WORLD_VALUE_BYTES: u32 = 1024;
/// The most changes one session's mailbox holds between two read cuts.
pub const MAX_PENDING_WORLD_CHANGES: usize = 64;

#[derive(Clone, Debug)]
enum Write { Set { value: ScriptValue, expect: Option<u32> }, Add(i32) }

#[derive(Clone, Debug)]
struct Held { write: RoomWrite, key: String, op: Write }

/// Writes from rooms that rewind alone, waiting for their tick to be confirmed.
#[derive(Resource, Default)]
pub struct HeldWorldWrites(Vec<Held>);

impl HeldWorldWrites {
    pub fn len(&self) -> usize { self.0.len() }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
}

/// The applied writes the session's entries have not heard yet.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct WorldValueChanges { rows: Vec<WorldValue> }
ambition_extension_sdk::port_record! { WorldValueChanges { rows: Vec<WorldValue> } }
impl SnapshotState for WorldValueChanges {
    fn encode(&self, out: &mut Vec<u8>) {
        let mut bytes = Vec::new(); ambition_extension_sdk::structure::put(self, &mut bytes); put_bytes(out, &bytes);
    }
    fn decode(reader: &mut Reader<'_>) -> Option<Self> {
        ambition_extension_sdk::wire::decode_all(reader.bytes()?, ambition_extension_sdk::structure::read).ok()
    }
}

/// A world value key: 1 to 48 bytes of `a-z`, `0-9`, `_`, `.` and `-`.
pub fn valid_world_key(key: &str) -> bool { valid_signal_topic(key) }

/// A script reads a revision as a whole number; past `u32::MAX` it stays there.
fn script_revision(saved: u64) -> u32 { u32::try_from(saved).unwrap_or(u32::MAX) }

fn encode(value: &ScriptValue) -> Vec<u8> {
    let mut bytes = Vec::new(); ambition_extension_sdk::structure::put(value, &mut bytes); bytes
}
fn decode(bytes: &[u8]) -> ScriptValue {
    ambition_extension_sdk::wire::decode_all(bytes, ambition_extension_sdk::structure::read).unwrap_or(ScriptValue::Unit)
}

/// Every world value the save keeps, in key order.
pub fn world_values(world: &World) -> WorldValues {
    let Some(save) = world.get_resource::<ambition_persistence::save::AmbitionGameSave>() else { return WorldValues::default(); };
    WorldValues {values: save.data().world_values().iter()
        .map(|row| WorldValue {key: row.key.clone(), value: decode(&row.value), revision: script_revision(row.revision)}).collect()}
}

/// Apply one write to the save and queue its change. The code names the outcome.
fn apply(world: &mut World, key: &str, op: &Write) -> Result<&'static str, &'static str> {
    let (current, revision) = {
        let save = world.get_resource::<ambition_persistence::save::AmbitionGameSave>().ok_or("no_world_save")?;
        if save.data().world_value(key).is_none() && save.data().world_values().len() >= MAX_WORLD_VALUES {
            return Err("world_value_store_full");
        }
        save.data().world_value(key).map_or((ScriptValue::Unit, 0), |row| (decode(&row.value), script_revision(row.revision)))
    };
    let value = match op {
        Write::Set { expect: Some(expect), .. } if *expect != revision => return Err("revision_changed"),
        Write::Set { value, .. } => value.clone(),
        Write::Add(delta) => match current {
            ScriptValue::Unit => ScriptValue::Int(i64::from(*delta)),
            ScriptValue::Int(count) => ScriptValue::Int(count.checked_add(i64::from(*delta)).ok_or("world_count_overflow")?),
            _ => return Err("not_a_count"),
        },
    };
    let Some(home) = session_world_entity(world) else { return Err("no_session"); };
    if world.get::<WorldValueChanges>(home).is_some_and(|changes| changes.rows.len() >= MAX_PENDING_WORLD_CHANGES) {
        return Err("world_change_queue_full");
    }
    let written = world.resource_mut::<ambition_persistence::save::AmbitionGameSave>().data_mut()
        .put_world_value(key, encode(&value));
    world.entity_mut(home).entry::<WorldValueChanges>().or_default().into_mut()
        .rows.push(WorldValue {key: key.to_owned(), value, revision: script_revision(written)});
    Ok(match op { Write::Set { .. } => "world_value_set", Write::Add(_) => "world_count_added" })
}

/// Check a write, then apply it now, or hold it when its room rewinds alone.
fn admit(world: &mut World, scope: Entity, key: &str, op: Write) -> Result<&'static str, &'static str> {
    if !valid_world_key(key) { return Err("invalid_world_key"); }
    if let Write::Set { value, .. } = &op {
        let checked = crate::object_behaviors::checked_data(value, MAX_WORLD_VALUE_BYTES).ok_or("world_value_bound")?;
        if checked != *value { return Err("world_value_bound"); }
    }
    match simulation_island(world, scope) {
        Some(SimulationIsland::Room(_, room)) => {
            if world.resource::<HeldWorldWrites>().0.len() >= MAX_HELD_WORLD_WRITES { return Err("world_write_queue_full"); }
            let tick = world.resource::<ambition_time::SimTick>().0;
            world.resource_mut::<HeldWorldWrites>().0.push(Held {write: RoomWrite {room, tick}, key: key.to_owned(), op});
            Ok("world_value_held")
        }
        Some(SimulationIsland::Session(_)) => apply(world, key, &op),
        None => Err("writer_without_authority"),
    }
}

fn lower_sets<L: ambition_extension_host::LowersIn>(world: &mut World) {
    let requests = world.resource_mut::<ambition_extension_host::ExtensionOutbox>().drain::<SetWorldValuePort>(&L::PHASE);
    for submitted in requests {
        let request = submitted.value;
        let dispatch = begin_action_request::<SetWorldValuePort>(world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute { continue; }
        let result = admit(world, submitted.scope, &request.key,
            Write::Set {value: request.value.clone(), expect: request.expect});
        answer(world, dispatch.reply.as_ref(), result);
    }
}

fn lower_adds<L: ambition_extension_host::LowersIn>(world: &mut World) {
    let requests = world.resource_mut::<ambition_extension_host::ExtensionOutbox>().drain::<AddWorldCountPort>(&L::PHASE);
    for submitted in requests {
        let request = submitted.value;
        let dispatch = begin_action_request::<AddWorldCountPort>(world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute { continue; }
        let result = admit(world, submitted.scope, &request.key, Write::Add(request.delta));
        answer(world, dispatch.reply.as_ref(), result);
    }
}

fn answer(world: &mut World, reply: Option<&ambition_platformer2d_shared_tangle::action_results::ActionReply>, result: Result<&'static str, &'static str>) {
    let (stage, code) = match result {
        Ok(code) => (ActionStage::Applied, code),
        Err(code) => (ActionStage::Rejected, code),
    };
    publish_action_result(world, reply, stage, true, code, "", Vec::new());
}

/// On a live tick: apply every held write whose tick is confirmed, in the
/// order they were written. A held write the value refuses now (a changed
/// revision, a full store) is dropped with a warning: its writer was told
/// only that it was held.
fn apply_confirmed_writes(world: &mut World) {
    let boundary = world.get_resource::<ConfirmedFrameBoundary>().copied();
    let held = std::mem::take(&mut world.resource_mut::<HeldWorldWrites>().0);
    let (ready, waiting): (Vec<Held>, Vec<Held>) = held.into_iter()
        .partition(|write| boundary.is_none_or(|boundary| boundary.is_confirmed(write.write.tick as i32)));
    world.resource_mut::<HeldWorldWrites>().0 = waiting;
    for write in ready {
        if let Err(code) = apply(world, &write.key, &write.op) {
            warn!(key = write.key, code, "A held world value write was refused when it was applied.");
        }
    }
}

/// A rewind of `room` to `saved_at` takes back the writes it made after it.
fn retract_held_writes(world: &mut World, room: LiveRoomInstance, saved_at: u64) {
    if let Some(mut held) = world.get_resource_mut::<HeldWorldWrites>() {
        held.0.retain(|write| !write.write.retracted_by(room, saved_at));
    }
}

fn forget_held_writes(mut held: ResMut<HeldWorldWrites>) { held.0.clear(); }

/// Invoke the entries listening to each change, at the session, and empty the mailbox.
fn queue_changes(world: &mut World) {
    let Some(home) = session_world_entity(world) else { return; };
    let Some(mut changes) = world.get_mut::<WorldValueChanges>(home) else { return; };
    if changes.rows.is_empty() { return; }
    let rows = std::mem::take(&mut changes.rows);
    let mut invocations = world.resource_mut::<ambition_extension_host::ExtensionInvocations>();
    for row in rows {
        invocations.trigger::<WorldValueChangedPort>(&ACTOR_THINK, row.key.clone(), home, None, false, row);
    }
}

pub(crate) fn register_rollback_state(registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar) {
    registrar.rollback_component_canonical::<WorldValueChanges>("ambition_platformer2d_runtime", "script.world_value_changes");
}

pub(crate) fn install(app: &mut App) {
    use ambition_extension_host::ExtensionAppExt;
    use ambition_platformer2d_shared_tangle::schedule::{SimScheduleExt, Platformer2dSimulationPhase};
    app.init_resource::<HeldWorldWrites>();
    require_on_session_root::<WorldValueChanges>(app);
    register_action_domain(app, "world.set_value", ActionCancellationPolicy::Unsupported);
    register_action_domain(app, "world.add_count", ActionCancellationPolicy::Unsupported);
    // Any body entry may write (a status tick, a contact, a hit), as it may post.
    ambition_extension_host::install_submitted_request_in_body_phases!(app, SetWorldValuePort,
        "ambition_platformer2d_runtime", lower_sets);
    ambition_extension_host::install_submitted_request_in_body_phases!(app, AddWorldCountPort,
        "ambition_platformer2d_runtime", lower_adds);
    app.install_extension_observation::<WorldValuesPort>(ACTOR_THINK, "ambition_platformer2d_runtime",
        |world, _| Some(world_values(world)));
    app.install_extension_trigger::<WorldValueChangedPort, _>(ACTOR_THINK, "ambition_platformer2d_runtime", queue_changes);
    register_room_write_retractor(app, "resource.held_world_writes", retract_held_writes);
    app.add_message::<SessionScopeActivated>();
    app.add_systems(Update, forget_held_writes.in_set(SessionScopeSet::Activate).run_if(on_message::<SessionScopeActivated>));
    let schedule = app.sim_schedule();
    app.add_systems(schedule, apply_confirmed_writes.run_if(not(resource_exists::<RoomReplay>))
        .in_set(Platformer2dSimulationPhase::GameplayEffects));
}
