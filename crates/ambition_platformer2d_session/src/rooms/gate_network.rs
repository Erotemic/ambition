//! The gate network's simulation step: a terminal opens at a press, a
//! terminal's request dials, and each link runs out.
//!
//! The rules are `ambition_platformer2d_world::rooms::gate_network`. This
//! step reads the world facts they need (the save's switches and flags, the
//! session's rooms) and writes [`GateNetworkState`] and what the player
//! learns. It moves no body: a traveller goes through an open ring by the
//! room-transition road (`detect_room_transition_system`).

use bevy::prelude::{MessageReader, Query, Res, ResMut, With, Without};

use ambition_platformer2d_world::rooms::{
    gate_address_known_flag, resolve_dial, GateFacts, GateNetworkCatalog, GateNetworkState, GateRingPositions,
    GateTerminalAction, GateTerminalIntent, RingKind,
};

/// One tick of the gate network.
///
/// 1. A press of a `GateTerminal` switch opens the terminal of its ring. A
///    portal arch's terminal ATTUNES the arch: standing at it teaches its
///    address, so other arches list it (a portal is learned by visiting it).
/// 2. Each terminal request of the open terminal is carried out: a dial
///    opens a link or records why not, a hangup shuts the ring's link, a
///    close leaves the terminal.
/// 3. Each link's timer runs, and a link that ran out closes.
///
/// Terminal use ends when its actor leaves play or leaves the ring's room.
/// Requests are host intents, so a replayed tick sees the same requests.
#[allow(clippy::too_many_arguments)]
pub fn run_gate_network(
    catalog: Option<Res<GateNetworkCatalog>>,
    mut state: ResMut<GateNetworkState>,
    switches: Option<Res<ambition_encounter::switches::ResolvedSwitchActivations>>,
    mut intents: MessageReader<GateTerminalIntent>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    world_time: Res<ambition_time::WorldTime>,
    positions: Option<Res<GateRingPositions>>,
    bodies: ambition_platformer2d_shared_tangle::lifecycle::LiveBodies,
    out_of_play: Query<(), With<ambition_combat::death_rules::OutOfPlay>>,
    passages: Query<&super::passage::ZonePassage, Without<ambition_combat::death_rules::OutOfPlay>>,
    pending: Option<Res<crate::lifecycle_commit::PendingLifecycleCommit>>,
    phases: Option<Res<ambition_platformer2d_world::rooms::GatePortalPhases>>,
) {
    let Some(catalog) = catalog else {
        intents.clear();
        return;
    };
    for activation in switches.iter().flat_map(|switches| switches.0.iter()) {
        if !matches!(activation.action, ambition_encounter::switches::SwitchAction::GateTerminal) {
            continue;
        }
        match catalog.ring_of_terminal(&activation.id) {
            Some(ring) => {
                if let Some(subject) = activation.subject.as_ref().filter(|subject| bodies.entity_of(subject).is_some()) {
                    state.terminals.insert(subject.clone(), ring.zone.clone());
                }
                if ring.kind == RingKind::Portal {
                    save.data_mut().set_flag(gate_address_known_flag(ring.address_key()), true);
                }
            }
            None => bevy::log::warn_once!(
                "the gate terminal `{}` dials from no ring of the gate network",
                activation.id
            ),
        }
    }
    state.terminals.retain(|subject, zone| {
        let room = bodies.entity_of(subject).filter(|body| !out_of_play.contains(*body)).and_then(|body| rooms.spec_of(body));
        catalog.ring(zone).is_some_and(|ring| room.is_some_and(|room| room.id == ring.room))
    });
    for intent in intents.read() {
        // A request reaches only the terminal that is open: a dial is made at
        // the ring, by a body that stands there.
        if state.terminals.get(&intent.subject) != Some(&intent.ring) {
            continue;
        }
        match &intent.action {
            GateTerminalAction::Close => { state.terminals.remove(&intent.subject); },
            GateTerminalAction::Hangup => {
                state.close_link(&intent.ring);
            }
            GateTerminalAction::Dial(address) => {
                // A redial replaces this ring's own link: the old link shuts
                // first, so it neither blocks the new one as busy nor stays
                // open beside it.
                // Where the ring leads now, if anywhere: a dial to another
                // place is a redial, and to the same place changes nothing.
                let leads_to = state.route_from(&intent.ring).map(|route| route.far.to_string());
                state.close_link(&intent.ring);
                let data = save.data();
                let switch_on = |id: &str| data.switch(id);
                let flag = |id: &str| data.flag(id);
                let zone_exists = |room: &str, zone: &str| rooms.rooms().has_zone(room, zone);
                let distance = |a: &str, b: &str| positions.as_deref().and_then(|positions| positions.distance(a, b));
                let facts =
                    GateFacts { switch_on: &switch_on, flag: &flag, zone_exists: &zone_exists, distance: &distance };
                match resolve_dial(&catalog, &state, &intent.ring, address, &facts) {
                    Ok(far) => {
                        let far_zone = far.zone.clone();
                        let known = gate_address_known_flag(far.address_key());
                        if let Err(refusal) = state.open_link(&catalog, &intent.ring, &far_zone) {
                            state.report(&intent.subject, &intent.ring, address.clone(), Err(refusal));
                            continue;
                        }
                        // The open wormhole closes before the new one opens.
                        if leads_to.as_ref().is_some_and(|before| *before != far_zone) {
                            state.open_after_close(&intent.ring);
                        }
                        state.report(&intent.subject, &intent.ring, address.clone(), Ok(far_zone));
                        // A dial that answers teaches the address.
                        save.data_mut().set_flag(known, true);
                    }
                    Err(refusal) => state.report(&intent.subject, &intent.ring, address.clone(), Err(refusal)),
                }
            }
        }
    }
    if let Some(phases) = phases.as_deref() {
        state.release_closed(|ring| phases.phase(ring) == ambition_platformer2d_world::rooms::GatePortalPhase::Off);
    }
    let dt = world_time.sim_dt();
    if dt > 0.0 && !state.links.is_empty() {
        let pending_zone = pending.as_deref().and_then(|pending| pending.peek())
            .and_then(|operation| operation.kind.source_zone());
        state.tick(dt, |ring| pending_zone == Some(ring) || passages.iter().any(|passage|
            passage.kind == super::passage::PassageKind::Gate && passage.zone == ring));
    }
}

/// Whether the gate network has work this tick: a switch was pressed, a
/// request waits, or a link is open.
///
/// Each input is optional: a composition without switches, terminal intents
/// or a network has no work for the network.
pub fn the_gate_network_has_work(
    state: Option<Res<GateNetworkState>>,
    switches: Option<Res<ambition_encounter::switches::ResolvedSwitchActivations>>,
    intents: Option<Res<bevy::ecs::message::Messages<GateTerminalIntent>>>,
) -> bool {
    let Some(state) = state else {
        return false;
    };
    switches.is_some_and(|switches| !switches.0.is_empty())
        || intents.is_some_and(|intents| !intents.is_empty())
        || !state.links.is_empty()
        || !state.terminals.is_empty()
}
