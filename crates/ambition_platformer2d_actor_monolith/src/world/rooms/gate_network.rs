//! The gate network's simulation step: a terminal opens at a press, a
//! terminal's request dials, and each link runs out.
//!
//! The rules are `ambition_platformer2d_world::rooms::gate_network`. This
//! step reads the world facts they need (the save's switches and flags, the
//! session's rooms) and writes [`GateNetworkState`] and what the player
//! learns. It moves no body: a traveller goes through an open ring by the
//! room-transition road (`detect_room_transition_system`).

use bevy::prelude::{MessageReader, Res, ResMut};

use ambition_platformer2d_world::rooms::{
    gate_address_known_flag, resolve_dial, GateFacts, GateNetworkCatalog, GateNetworkState, GateTerminalAction,
    GateTerminalIntent,
};

/// One tick of the gate network.
///
/// 1. A press of a `GateTerminal` switch opens the terminal of its ring.
/// 2. Each terminal request of the open terminal is carried out: a dial
///    opens a link or records why not, a hangup shuts the ring's link, a
///    close leaves the terminal.
/// 3. Each link's timer runs, and a link that ran out closes.
///
/// A terminal whose ring's room is no longer live closes: nobody stands at
/// it. The terminal's requests are host intents, so a replayed tick sees the
/// same requests.
#[allow(clippy::too_many_arguments)]
pub fn run_gate_network(
    catalog: Option<Res<GateNetworkCatalog>>,
    mut state: ResMut<GateNetworkState>,
    switches: Option<Res<ambition_encounter::switches::ResolvedSwitchActivations>>,
    mut intents: MessageReader<GateTerminalIntent>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    world_time: Res<ambition_time::WorldTime>,
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
            Some(ring) => state.terminal = Some(ring.zone.clone()),
            None => bevy::log::warn_once!(
                "the gate terminal `{}` dials from no ring of the gate network",
                activation.id
            ),
        }
    }
    let live: Vec<String> = rooms.live_specs().into_iter().map(|(_, spec)| spec.id.clone()).collect();
    if let Some(open) = state.terminal.clone() {
        if !catalog.ring(&open).is_some_and(|ring| live.iter().any(|room| *room == ring.room)) {
            state.terminal = None;
        }
    }
    for intent in intents.read() {
        // A request reaches only the terminal that is open: a dial is made at
        // the ring, by a body that stands there.
        if state.terminal.as_deref() != Some(intent.ring.as_str()) {
            continue;
        }
        match &intent.action {
            GateTerminalAction::Close => state.terminal = None,
            GateTerminalAction::Hangup => {
                state.links.remove(&intent.ring);
            }
            GateTerminalAction::Dial(address) => {
                // A redial replaces this ring's own link: the old link shuts
                // first, so it neither blocks the new one as busy nor stays
                // open beside it.
                state.links.remove(&intent.ring);
                let data = save.data();
                let switch_on = |id: &str| data.switch(id);
                let flag = |id: &str| data.flag(id);
                let zone_exists = |room: &str, zone: &str| rooms.rooms().has_zone(room, zone);
                let facts = GateFacts { switch_on: &switch_on, flag: &flag, zone_exists: &zone_exists };
                match resolve_dial(&catalog, &state, &intent.ring, address, &facts) {
                    Ok(far) => {
                        let (far_room, far_zone) = (far.room.clone(), far.zone.clone());
                        let word = far.address.clone();
                        state.open_link(&intent.ring, &far_room, &far_zone, address);
                        state.report(&intent.ring, address.clone(), Ok(far_zone));
                        // A dial that answers teaches the address.
                        if let Some(word) = word {
                            save.data_mut().set_flag(gate_address_known_flag(&word), true);
                        }
                    }
                    Err(refusal) => state.report(&intent.ring, address.clone(), Err(refusal)),
                }
            }
        }
    }
    let dt = world_time.sim_dt();
    if dt > 0.0 && !state.links.is_empty() {
        state.tick(dt);
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
        || state.terminal.is_some()
}
