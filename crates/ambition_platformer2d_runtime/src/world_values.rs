//! Named world values: state the durable save keeps and every room reads.
//!
//! A script writes one with `set_world_value` or `add_world_count`. The
//! world domain applies a write on a tick no room can simulate again, the
//! rule `world_posts` keeps for signals: under one rollback authority (local
//! play, GGRS) that is the writing tick itself; a room that rewinds alone
//! (SIMULATION-ISLANDS) has its write held until the confirmed boundary
//! passes the writing tick, and a rewind of that room takes back what it
//! wrote after its save. A held write's writer hears `Admitted`, which is not
//! terminal, then the world's decision under the same ticket: `Applied`, or
//! the refusal the write meets then (a compare-and-set whose revision another
//! write changed). A replay of the writer's room tells it that decision again
//! on the tick it was first told. So a value is never written by a tick that is later
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

/// A write held for confirmation, with the reply its writer waits on: the
/// writer was told only that it was admitted, and hears the world's decision
/// (applied, or the refusal it meets then) under the same ticket.
#[derive(Clone, Debug)]
struct Held {
    write: RoomWrite,
    key: String,
    op: Write,
    reply: Option<ambition_platformer2d_shared_tangle::action_results::ActionReply>,
}

/// What admission did with a write that passed its checks.
enum Admission {
    /// The world applied it now.
    Applied(&'static str),
    /// Held until its tick is confirmed: pending, not yet a world change.
    Held(&'static str),
}

/// One decision the world told a held write's writer, by the live tick it
/// was told on: a replay of the writer's room that runs that tick tells it
/// again, into the mailbox its rewind restored.
#[derive(Clone, Debug)]
struct Decided {
    tick: u64,
    reply: ambition_platformer2d_shared_tangle::action_results::ActionReply,
    stage: ActionStage,
    code: &'static str,
}

/// Writes from rooms that rewind alone, waiting for their tick to be
/// confirmed, and the decisions told to their writers.
#[derive(Resource, Default)]
pub struct HeldWorldWrites { held: Vec<Held>, decided: Vec<Decided> }

impl HeldWorldWrites {
    /// Writes waiting for their tick to be confirmed.
    pub fn len(&self) -> usize { self.held.len() }
    pub fn is_empty(&self) -> bool { self.held.is_empty() }
    /// Decisions a replay of a writer's room can still tell again.
    pub fn decided(&self) -> usize { self.decided.len() }
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
fn admit(world: &mut World, scope: Entity, key: &str, op: Write,
    reply: Option<&ambition_platformer2d_shared_tangle::action_results::ActionReply>) -> Result<Admission, &'static str> {
    if !valid_world_key(key) { return Err("invalid_world_key"); }
    if let Write::Set { value, .. } = &op {
        let checked = crate::object_behaviors::checked_data(value, MAX_WORLD_VALUE_BYTES).ok_or("world_value_bound")?;
        if checked != *value { return Err("world_value_bound"); }
    }
    match simulation_island(world, scope) {
        Some(SimulationIsland::Room(_, room)) => {
            if world.resource::<HeldWorldWrites>().held.len() >= MAX_HELD_WORLD_WRITES { return Err("world_write_queue_full"); }
            let tick = world.resource::<ambition_time::SimTick>().0;
            world.resource_mut::<HeldWorldWrites>().held.push(Held {write: RoomWrite {room, tick}, key: key.to_owned(), op,
                reply: reply.cloned()});
            Ok(Admission::Held("world_value_held"))
        }
        Some(SimulationIsland::Session(_)) => apply(world, key, &op).map(Admission::Applied),
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
            Write::Set {value: request.value.clone(), expect: request.expect}, dispatch.reply.as_ref());
        answer(world, dispatch.reply.as_ref(), result);
    }
}

fn lower_adds<L: ambition_extension_host::LowersIn>(world: &mut World) {
    let requests = world.resource_mut::<ambition_extension_host::ExtensionOutbox>().drain::<AddWorldCountPort>(&L::PHASE);
    for submitted in requests {
        let request = submitted.value;
        let dispatch = begin_action_request::<AddWorldCountPort>(world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute { continue; }
        let result = admit(world, submitted.scope, &request.key, Write::Add(request.delta), dispatch.reply.as_ref());
        answer(world, dispatch.reply.as_ref(), result);
    }
}

/// A held write is `Admitted` and not terminal: the writer hears its real
/// outcome when the world decides it ([`apply_confirmed_writes`]).
fn answer(world: &mut World, reply: Option<&ambition_platformer2d_shared_tangle::action_results::ActionReply>, result: Result<Admission, &'static str>) {
    let (stage, terminal, code) = match result {
        Ok(Admission::Applied(code)) => (ActionStage::Applied, true, code),
        Ok(Admission::Held(code)) => (ActionStage::Admitted, false, code),
        Err(code) => (ActionStage::Rejected, true, code),
    };
    publish_action_result(world, reply, stage, terminal, code, "", Vec::new());
}

/// On a live tick: apply every held write whose tick is confirmed, in the
/// order they were written, and tell each writer the world's decision: the
/// write applied, or the refusal it met now (a changed revision, a full
/// store). Its writer was told only that it was admitted.
fn apply_confirmed_writes(world: &mut World) {
    let boundary = world.get_resource::<ConfirmedFrameBoundary>().copied();
    let now = world.resource::<ambition_time::SimTick>().0;
    let mut store = world.resource_mut::<HeldWorldWrites>();
    store.decided.retain(|row| row.tick.saturating_add(ROOM_REWIND_HORIZON_TICKS) >= now);
    let held = std::mem::take(&mut store.held);
    let (ready, waiting): (Vec<Held>, Vec<Held>) = held.into_iter()
        .partition(|write| boundary.is_none_or(|boundary| boundary.is_confirmed(write.write.tick as i32)));
    store.held = waiting;
    for write in ready {
        let (stage, code) = match apply(world, &write.key, &write.op) {
            Ok(code) => (ActionStage::Applied, code),
            Err(code) => (ActionStage::Rejected, code),
        };
        let Some(reply) = write.reply else { continue; };
        publish_action_result(world, Some(&reply), stage, true, code, "", Vec::new());
        world.resource_mut::<HeldWorldWrites>().decided.push(Decided {tick: now, reply, stage, code});
    }
}

/// While a room replays: tell each writer in it again the decision it was
/// told on the tick now running. The rewind restored its mailbox from before.
fn retell_in_replay(world: &mut World) {
    let Some(replay) = world.get_resource::<RoomReplay>().copied() else { return; };
    let now = world.resource::<ambition_time::SimTick>().0;
    let rows: Vec<Decided> = world.resource::<HeldWorldWrites>().decided.iter().filter(|row| row.tick == now).cloned().collect();
    for row in rows {
        let Some(entity) = row.reply.recipient.resolve(world) else { continue; };
        if !replay.allows(object_scope_room(world, entity)) { continue; }
        publish_action_result(world, Some(&row.reply), row.stage, true, row.code, "", Vec::new());
    }
}

/// A rewind of `room` to `saved_at` takes back the writes it made after it.
fn retract_held_writes(world: &mut World, room: LiveRoomInstance, saved_at: u64) {
    if let Some(mut held) = world.get_resource_mut::<HeldWorldWrites>() {
        held.held.retain(|write| !write.write.retracted_by(room, saved_at));
    }
}

fn forget_held_writes(mut held: ResMut<HeldWorldWrites>) { *held = HeldWorldWrites::default(); }

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

/// The gated walls of the live room `scope` is in, and which of them stand:
/// a wall stands while the gate's own writer puts its solid in that room.
pub fn lock_walls(world: &World, scope: Entity) -> LockWalls {
    use ambition_platformer2d_world::gated_lock_walls::{authored_gated_lock_walls, GATED_LOCK_BLOCK_PREFIX};
    let room = world.get::<LiveRoomInstance>(scope).filter(|_| world.get::<RoomInstanceRoot>(scope).is_some()).copied()
        .or_else(|| object_scope_room(world, scope));
    let Some(room) = room else { return LockWalls::default(); };
    let Some(mut roots) = world.try_query_filtered::<(Entity, &LiveRoomInstance), With<RoomInstanceRoot>>() else {
        return LockWalls::default();
    };
    let Some(root) = roots.iter(world).find(|(_, live)| **live == room).map(|(root, _)| root) else {
        return LockWalls::default();
    };
    let Some(spec) = ambition_platformer2d_world::rooms::live_room_spec_of(world, root) else { return LockWalls::default(); };
    let solids = world.get::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>(root)
        .map(|overlay| overlay.gate_solids.clone()).unwrap_or_default();
    LockWalls {walls: authored_gated_lock_walls(spec).into_iter().map(|wall| {
        let name = format!("{GATED_LOCK_BLOCK_PREFIX}{}", wall.id);
        LockWallState {standing: solids.iter().any(|block| block.name == name), id: wall.id, gated_by: wall.gated_by}
    }).collect()}
}

const KEY: ambition_platformer2d_shared_tangle::authored_logic::ParamSpec = ambition_platformer2d_shared_tangle::authored_logic::ParamSpec {
    name: "key", kind: ambition_platformer2d_shared_tangle::authored_logic::ParamKind::Name,
    summary: "the world value's key, as `set_world_value` writes it",
};

/// `world.value_is(key, value)`: the world value `key` is `value`.
pub fn value_is_descriptor() -> ambition_platformer2d_shared_tangle::authored_logic::ConditionDescriptor {
    use ambition_platformer2d_shared_tangle::authored_logic::*;
    ConditionDescriptor {
        id: ConditionId::new("world", "value_is"),
        summary: "true while the named world value is the given text, whole number or truth",
        params: &[KEY, ParamSpec {name: "value", kind: ParamKind::Name,
            summary: "the value, written as text: `lowered`, `3`, `true`"}],
    }
}

/// `world.value_is`: an absent value is no value, so not satisfied.
pub fn value_is(world: &World, args: &[ambition_platformer2d_shared_tangle::authored_logic::AuthoredArg])
    -> ambition_platformer2d_shared_tangle::authored_logic::ConditionOutcome {
    use ambition_platformer2d_shared_tangle::authored_logic::{ConditionOutcome, WhyNot};
    let (Some(key), Some(wanted)) = (args[0].as_name(), args[1].as_name()) else {
        return ConditionOutcome::unanswerable("`key` and `value` must be names");
    };
    let Some(save) = world.get_resource::<ambition_persistence::save::AmbitionGameSave>() else {
        return ConditionOutcome::unanswerable("no save layer is installed, so no world values exist");
    };
    let holds = save.data().world_value(key).is_some_and(|row| match decode(&row.value) {
        ScriptValue::Text(text) => text == wanted,
        ScriptValue::Int(number) => wanted.parse::<i64>() == Ok(number),
        ScriptValue::Bool(truth) => wanted.parse::<bool>() == Ok(truth),
        _ => false,
    });
    ConditionOutcome::from_bool(holds, || WhyNot::new("world.value_is", key, "the world value is not that"))
}

/// `world.count_at_least(key, at_least)`: the world count `key` has reached `at_least`.
pub fn count_at_least_descriptor() -> ambition_platformer2d_shared_tangle::authored_logic::ConditionDescriptor {
    use ambition_platformer2d_shared_tangle::authored_logic::*;
    ConditionDescriptor {
        id: ConditionId::new("world", "count_at_least"),
        summary: "true once the named world count is at least the given number (an absent count is 0)",
        params: &[KEY, ParamSpec {name: "at_least", kind: ParamKind::Number, summary: "the threshold"}],
    }
}

/// `world.count_at_least`: a value that is not a count never reaches it.
pub fn count_at_least(world: &World, args: &[ambition_platformer2d_shared_tangle::authored_logic::AuthoredArg])
    -> ambition_platformer2d_shared_tangle::authored_logic::ConditionOutcome {
    use ambition_platformer2d_shared_tangle::authored_logic::{ConditionOutcome, WhyNot};
    let (Some(key), Some(threshold)) = (args[0].as_name(), args[1].as_number()) else {
        return ConditionOutcome::unanswerable("`key` must be a name and `at_least` a number");
    };
    let Some(save) = world.get_resource::<ambition_persistence::save::AmbitionGameSave>() else {
        return ConditionOutcome::unanswerable("no save layer is installed, so no world values exist");
    };
    let count = save.data().world_value(key).map_or(Some(0), |row| match decode(&row.value) {
        ScriptValue::Int(count) => Some(count),
        _ => None,
    });
    ConditionOutcome::from_bool(count.is_some_and(|count| count as f64 >= threshold),
        || WhyNot::new("world.count_at_least", key, "the world count is lower"))
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
    app.install_extension_observation::<LockWallsPort>(ACTOR_THINK, "ambition_platformer2d_runtime",
        |world, scope| Some(lock_walls(world, scope)));
    app.install_extension_trigger::<WorldValueChangedPort, _>(ACTOR_THINK, "ambition_platformer2d_runtime", queue_changes);
    register_room_write_retractor(app, "resource.held_world_writes", retract_held_writes);
    app.add_message::<SessionScopeActivated>();
    app.add_systems(Update, forget_held_writes.in_set(SessionScopeSet::Activate).run_if(on_message::<SessionScopeActivated>));
    let schedule = app.sim_schedule();
    app.add_systems(schedule, (
        apply_confirmed_writes.run_if(not(resource_exists::<RoomReplay>)),
        retell_in_replay.run_if(resource_exists::<RoomReplay>),
    ).in_set(Platformer2dSimulationPhase::GameplayEffects));
}
