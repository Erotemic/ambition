//! Correlated native mechanism creation in every body request phase: lifts
//! the platform domain moves, doors, and switches that send a lift or open a
//! door, made together.
//!
//! Each part keeps its native owner. A switch is the same interactable
//! placement an authored `Switch` lowers to, with the `MoveLift` or
//! `ToggleDoor` action bound to its part's construction identity. The lift is
//! a switch-sent row in the caller's room's `MovingPlatformSet`, and its
//! object occurrence (a `MechanismLift` row of the same batch) is what a
//! caller retires to take it away alone. A door is its own occurrence (a
//! `MechanismDoor`), which stands in its room's gate solids while shut. All
//! belong to the caller's room, like scripted breakables.

use ambition_extension_host::{ExtensionOutbox, LowersIn};
use ambition_extension_sdk::action::ActionStage;
use ambition_platformer2d_shared_tangle::action_results::publish_action_result;
use ambition_world_port::{mechanisms::*, object_target::ObjectTarget};
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// The most parts one request may make.
const MAX_PARTS: usize = 8;

pub fn install(app: &mut App) {
    app.add_observer(ambition_platformer2d_world::platforms::a_retired_lift_leaves_its_room);
    ambition_platformer2d_shared_tangle::action_results::register_action_domain(app, "world.create_mechanism",
        ambition_platformer2d_shared_tangle::action_results::ActionCancellationPolicy::Unsupported);
    ambition_extension_host::install_submitted_request_in_body_phases!(app, CreateMechanismPort,
        "ambition_platformer2d_world", lower);
}

fn lower<L: LowersIn>(world: &mut World) {
    let submitted = world.resource_mut::<ExtensionOutbox>().drain::<CreateMechanismPort>(&L::PHASE);
    for submitted in submitted {
        let request = submitted.value;
        let dispatch = ambition_platformer2d_shared_tangle::action_results::begin_action_request::<CreateMechanismPort>(
            world, submitted.state_scope, &submitted.entry, &request);
        if !dispatch.execute {
            continue;
        }
        match create_mechanism(world, submitted.scope, &request) {
            Ok(parts) => {
                let subjects = parts
                    .into_iter()
                    .map(|entity| {
                        ambition_platformer2d_shared_tangle::action_results::object_action_subject(world, entity)
                            .expect("a published part has a live occurrence")
                    })
                    .collect();
                publish_action_result(world, dispatch.reply.as_ref(), ActionStage::Applied, true,
                    "mechanism_created", "", subjects);
            }
            Err(message) => {
                publish_action_result(world, dispatch.reply.as_ref(), ActionStage::Rejected, true,
                    "mechanism_refused", message, Vec::new());
            }
        }
    }
}

fn finite_box(at: [f32; 2], size: [f32; 2]) -> bool {
    at.iter().chain(&size).all(|v| v.is_finite()) && size[0] > 0.0 && size[1] > 0.0
}

/// The part a switch works, by its index in the request.
#[derive(Clone, Copy)]
enum Works {
    Lift(usize),
    Door(usize),
}

/// Check the whole request, then make it, or refuse with nothing made. Answers
/// the switches' entities, then the lifts' occurrences, then the doors, each
/// in request order.
fn create_mechanism(world: &mut World, caller: Entity, request: &CreateMechanism) -> Result<Vec<Entity>, String> {
    use ambition_entity_catalog::placements::{InteractableSpec, InteractionKindSpec, PlacementSchema};
    use ambition_platformer2d_construction::{ActorConstructionParams, ActorConstructionRequest};
    use ambition_platformer2d_world::{collision::MovingPlatformSet, platforms::MovingPlatformState, placements::PlacementRecord};

    let parts = request.lifts.len() + request.doors.len() + request.switches.len();
    if parts == 0 || parts > MAX_PARTS {
        return Err(format!("A mechanism makes 1 to {MAX_PARTS} parts; this one asks for {parts}."));
    }
    let mut keys = BTreeSet::new();
    let keyed = request.lifts.iter().map(|lift| &lift.key)
        .chain(request.doors.iter().map(|door| &door.key))
        .chain(request.switches.iter().map(|switch| &switch.key));
    for key in keyed {
        if key.is_empty() || key.len() > 128 || !keys.insert(key) {
            return Err(format!("Part key {key:?} must be nonempty, unique in the mechanism and at most 128 bytes."));
        }
    }
    for lift in &request.lifts {
        if !finite_box(lift.at, lift.size) || !lift.rise.is_finite() || lift.rise == 0.0 || !(lift.speed > 0.0) {
            return Err(format!("Lift {:?} needs a finite box of positive size, a nonzero rise and a positive speed.", lift.key));
        }
    }
    for door in &request.doors {
        if !finite_box(door.at, door.size) {
            return Err(format!("Door {:?} needs a finite box of positive size.", door.key));
        }
    }
    let mut sends = Vec::with_capacity(request.switches.len());
    for switch in &request.switches {
        if !finite_box(switch.at, switch.size) || switch.prompt.len() > 128 {
            return Err(format!("Switch {:?} needs a finite box of positive size and a prompt of at most 128 bytes.", switch.key));
        }
        let works = match &switch.sends {
            ObjectTarget::Local(key) => request.lifts.iter().position(|lift| &lift.key == key).map(Works::Lift)
                .or_else(|| request.doors.iter().position(|door| &door.key == key).map(Works::Door))
                .ok_or_else(|| {
                    format!("Switch {:?}: field `sends` names {key:?}, which is no lift or door of this mechanism.", switch.key)
                })?,
            ObjectTarget::Live(_) => {
                return Err(format!(
                    "Switch {:?}: field `sends` must name a lift or door of this mechanism (#{{local: key}}).",
                    switch.key
                ));
            }
        };
        sends.push(works);
    }

    let context = ambition_platformer2d_construction::runtime::RuntimeConstruction::prepare(world, caller, parts)?;
    let lift_ids: Vec<_> = (0..request.lifts.len()).map(|index| context.id(index)).collect();
    let door_ids: Vec<_> = (0..request.doors.len()).map(|index| context.id(request.lifts.len() + index)).collect();
    let mut room_root = world.query_filtered::<(
        Entity,
        &ambition_platformer2d_shared_tangle::lifecycle::SessionScopedEntity,
        &ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    ), With<ambition_platformer2d_shared_tangle::lifecycle::RoomInstanceRoot>>();
    let root = room_root
        .iter(world)
        .find(|(_, owner, room)| owner.0 == context.session() && **room == context.room())
        .map(|(root, ..)| root)
        .ok_or("The caller's room has no root.")?;

    let lowering = world
        .get_resource::<ambition_platformer2d_construction::placements::PlacementLoweringRegistry>()
        .ok_or("The interactable placement capability is not installed.")?;
    let mut rows = Vec::with_capacity(parts);
    let mut names = BTreeMap::new();
    // Each lift's occurrence: an object of the batch, named by its key.
    for (slot, (lift, id)) in request.lifts.iter().zip(&lift_ids).enumerate() {
        names.insert(lift.key.clone(), id.clone());
        rows.push(ActorConstructionRequest {
            sim_id: id.clone(),
            origin: context.origin(slot),
            parameters: ActorConstructionParams::MechanismLift { platform: id.as_str().to_string() },
            relations: Vec::new(),
        });
    }
    // Each door: an occurrence that holds its own box and state.
    for (index, (door, id)) in request.doors.iter().zip(&door_ids).enumerate() {
        names.insert(door.key.clone(), id.clone());
        let min = Vec2::from(door.at) - Vec2::from(door.size) * 0.5;
        rows.push(ActorConstructionRequest {
            sim_id: id.clone(),
            origin: context.origin(request.lifts.len() + index),
            parameters: ActorConstructionParams::MechanismDoor {
                id: id.as_str().to_string(),
                min: min.into(),
                size: door.size,
                open: door.open,
            },
            relations: Vec::new(),
        });
    }
    for (index, (switch, works)) in request.switches.iter().zip(&sends).enumerate() {
        let slot = request.lifts.len() + request.doors.len() + index;
        let id = context.id(slot);
        // The switch's id is its construction identity, kept free of the
        // wire's separator; its target is its part's (the parser keeps the
        // rest of the payload as the target).
        let (action, target) = match *works {
            Works::Lift(lift) => ("MoveLift", &lift_ids[lift]),
            Works::Door(door) => ("ToggleDoor", &door_ids[door]),
        };
        let payload = ambition_encounter::SwitchActivation {
            id: id.as_str().replace(':', "."),
            action: action.into(),
            target_encounter: target.as_str().to_string(),
        }
        .to_custom_payload();
        let mut record = PlacementRecord::new(
            id.as_str(),
            PlacementSchema::Interactable(InteractableSpec::new(switch.prompt.clone(), InteractionKindSpec::Custom(payload))),
            ambition_platformer2d_core::Aabb::new(switch.at.into(), (Vec2::from(switch.size) * 0.5).into()),
        );
        record.name = switch.key.clone();
        let lower = lowering.try_interpreter_for(&record, "runtime").map_err(|error| error.to_string())?;
        names.insert(switch.key.clone(), id.clone());
        rows.push(ActorConstructionRequest {
            sim_id: id,
            origin: context.origin(slot),
            parameters: ActorConstructionParams::Placement { record, paths: Vec::new(), lower },
            relations: Vec::new(),
        });
    }
    // Switches first, then lifts, then doors: the answer's order since lifts
    // and doors had none.
    let made = request.lifts.len() + request.doors.len();
    let ids: Vec<_> = rows[made..].iter().chain(&rows[..made]).map(|row| row.sim_id.clone()).collect();
    let receipt = context.commit(world, rows, &names, &BTreeMap::<_, &()>::new(), |_, _| {})?;

    // Committed: the lifts join the room's platform set. Nothing below refuses.
    let lifts = request.lifts.iter().zip(&lift_ids).map(|(lift, id)| {
        MovingPlatformState::lift(id.as_str(), lift.key.clone(), Vec2::from(lift.at).into(), Vec2::from(lift.size).into(), lift.rise, lift.speed)
            .sent_by_switch()
    });
    match world.get_mut::<MovingPlatformSet>(root) {
        Some(mut set) => set.0.extend(lifts),
        None => {
            world.entity_mut(root).insert(MovingPlatformSet(lifts.collect()));
        }
    }
    Ok(ids.iter().map(|id| receipt.entity(id).expect("the complete batch committed every part")).collect())
}
