//! The Bevy adapters around the generic encounter lifecycle (E8/E9).
//!
//! `project_live_encounter_occurrences` builds each live room's encounter
//! OCCURRENCES from the room's authored encounters + the save: one entity per
//! (live room, encounter) carrying the generic authority set (`Encounter` +
//! `EncounterLifecycle` + `EncounterObjective` + `EncounterParticipants`) plus
//! the wave policy (`EncounterWaves`), stamped into its room.
//!
//! `drive_wave_encounters` (EncounterSimulation) is the wave ADAPTER: it emits
//! lifecycle COMMANDS (trigger entry → `Start`, player death → `Fail`+`Reset`,
//! area exit → `Reset`), refreshes participant liveness from the ECS mobs, and
//! advances the spawn cadence — it never mutates the phase. The generic reducer
//! (`ambition_encounter::reduce_encounter_lifecycles`, positioned by the
//! runtime in `Progression`) is the only lifecycle owner.
//!
//! `apply_wave_encounter_effects` (Progression, after the reducer) reacts to
//! lifecycle EVENTS: switch auto-green + mob cleanup + banner + quest on
//! completion, reward-chest sync, music request, presentation read-model, save
//! projection, and the trace sink.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::SessionCommands;

use ambition_encounter::{
    Encounter, EncounterCommand, EncounterCommandKind, EncounterEvent, EncounterEventMsg,
    EncounterLifecycle, EncounterMusicRequest, EncounterParticipants,
    EncounterView, EncounterWaves, WAVES_EXHAUSTED_SIGNAL,
};

use crate::load_encounter_specs_from_rooms;
use ambition_encounter::switches::EncounterSwitchIndex;

/// Build each live room's encounter OCCURRENCES: for every live room, the
/// occurrence of each encounter its room authors, stamped into that room
/// (see `ambition_encounter::occurrence`), with its lifecycle from the save.
///
/// ⛔ ONE OCCURRENCE PER (LIVE ROOM, AUTHORED ENCOUNTER). This used to spawn
/// one entity per authored encounter, for every room of the set, once per
/// session. Two live rooms of one room then shared one lifecycle, one wave run
/// and one member list, and a mob spawned for either could be matched to
/// neither. The authored id stays the durable key (the save, quests, switch
/// links); the room is what tells two runs of it apart.
///
/// ⛔ AN OCCURRENCE IS A ROOM OCCUPANT. It is `RoomScopedEntity`, so the room
/// transaction that replaces or retires its room retires it too, and this
/// builds the new room's occurrences on the next tick. What survives a room
/// is what the save holds: a Completed or Failed outcome. An in-flight attempt
/// ended with its room before, too (the driver reset it on exit).
///
/// Session-scoped as well, so retiring the session takes every occurrence.
/// Rooms are visited in live-room order, so every peer spawns the same
/// carriers in the same order.
pub fn project_live_encounter_occurrences(
    mut commands: ambition_platformer2d_shared_tangle::lifecycle::SessionCommands,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    // , and it is done: encounters come off the ROOM IR now, not off an `LdtkProject`.
    // `EncounterTrigger` and `LockWall` are ordinary emissions like every other authored
    // family, which is what took the LDtk crate out of this file.
    //
    // Optional because a composition may have no rooms installed — a headless
    // fixture, a shell at a non-gameplay route.
    rooms: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    // the App's authored wave book. Optional for the same reason the project is: a composition
    // with no authored encounters is an empty set, not an error.
    waves: Option<Res<ambition_encounter::EncounterWaveBook>>,
    standing: Query<
        (&Encounter, &ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance),
        With<EncounterWaves>,
    >,
) {
    // A shell host at a non-gameplay route has no session to own the
    // occurrences: sleep. A legacy/headless app with no session lifecycle
    // installed gets the unscoped spawn mode, as before.
    let Some(scope) = commands.spawn_scope() else {
        return;
    };
    let Some(rooms) = rooms else {
        return;
    };
    let standing: std::collections::BTreeSet<(
        &str,
        ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    )> = standing
        .iter()
        .map(|(encounter, stamp)| (encounter.id.as_str(), stamp.0))
        .collect();
    let mut live: Vec<_> = rooms.live_rooms().collect();
    live.sort_by_key(|(room, _)| *room);
    let mut built = 0usize;
    for (room, definition) in live {
        let spec = rooms.rooms().spec(definition);
        let entries = load_encounter_specs_from_rooms(
            std::slice::from_ref(spec),
            save.data(),
            waves.as_deref(),
        );
        for (id, spec, persisted) in entries {
            if standing.contains(&(id.as_str(), room)) {
                continue;
            }
            let lifecycle = EncounterLifecycle::from_persisted(spec.intro_seconds, persisted);
            let waves = EncounterWaves::new(spec);
            let objective = waves.objective();
            let mut entity = commands.spawn((
                Encounter::new(id.clone()),
                // Stable simulation identity (E11): the authority enters the
                // snapshot roster / state hash under its own namespace. The
                // room beside it tells two occurrences apart.
                ambition_platformer2d_shared_tangle::sim_id::SimId::encounter(&id),
                ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity,
                lifecycle,
                objective,
                EncounterParticipants::default(),
            ));
            // Authored staging policy (E12): generic consumers derive the lock
            // wall / camera zoom / base track from the LIFECYCLE + these, never
            // from the wave component.
            if let Some(wall) = waves.spec.lock_wall.clone() {
                entity.insert(ambition_encounter::EncounterLockWall(wall));
            }
            entity.insert(ambition_encounter::EncounterCameraZoom(
                waves.spec.camera_zoom,
            ));
            if !waves.spec.music_track.is_empty() {
                entity.insert(ambition_encounter::EncounterTrack(
                    waves.spec.music_track.clone(),
                ));
            }
            entity.insert(waves);
            scope.in_room(Some(room)).apply_to(&mut entity);
            built += 1;
        }
    }
    if built > 0 {
        bevy::log::info!(
            target: "ambition_platformer2d::encounter",
            "encounter occurrences: {built} built for the live rooms",
        );
    }
}

/// The set [`drive_wave_encounters`] runs in.
///
/// Lock-wall visuals read `gate_solids` after the encounter has populated it,
/// which is what "runs late in the frame" meant when a renderer named this
/// function to say so.
///
/// ONE member — the wave drive is the thing that decides gate state.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WaveEncounterDriven;

/// The wave COMMAND adapter + spawn-cadence director. Emits lifecycle commands
/// (never phase writes); the generic reducer applies them later this frame.
///
/// Cancellation policy (deliberate sandbox UX): an encounter is "in play" only
/// while the player is actually inside its area — walking out resets it so the
/// camera zoom + lock release on exit, and a fresh attempt fires on re-entry.
pub fn drive_wave_encounters(
    mut commands: SessionCommands<'_, '_>,
    world_time: Res<ambition_time::WorldTime>,
    mut died_messages: MessageReader<ambition_combat::death_rules::ActorDiedMessage>,
    mut encounters: Query<(
        Entity,
        &Encounter,
        &EncounterLifecycle,
        &mut EncounterWaves,
        &mut EncounterParticipants,
    )>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    // ⭐ THE QUEUE IS NOT DRAINED HERE ANY MORE. This reads what the switch
    // domain published; the drain, the parse and the persisted toggle all
    // belong to `ambition_encounter::switches::drain_switch_activations`.
    resolved_switches: Res<ambition_encounter::switches::ResolvedSwitchActivations>,
    switch_index: Res<EncounterSwitchIndex>,
    player_body_q: Query<
        (Entity, &ambition_platformer2d_core::BodyKinematics),
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
    mut quests: ResMut<ambition_persistence::quest::QuestRegistry>,
    mut lifecycle_commands: MessageWriter<EncounterCommand>,
    mut events_out: MessageWriter<EncounterEventMsg>,
    // ⭐ THE ROOM SET IS ALL THAT SURVIVES. This system used to take the
    // character catalog, the prepared cast and the authored sheets as well —
    // every one of them a BODY-CONSTRUCTION input it needed only because it
    // served its own spawn requests. Serving moved to
    // `features::serve_encounter_spawn_commands`, and the inputs went with it.
    //
    // Every live room's, each player's own (OW1 cut 7c): with two live rooms
    // both rooms' encounters run.
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    encounter_mobs: Query<(
        Entity,
        &ambition_combat::components::EncounterMob,
        &ambition_combat::components::FeatureId,
        // AC3.1.A: the HP authority. Participant liveness decides wave
        // completion, so it must not lag a frame behind a mirror.
        &ambition_characters::actor::BodyHealth,
    )>,
    // ⭐ THE CHEST QUERY LEFT WITH THE REWARD RETIRE (2026-09-03). This system
    // drives waves; it has no business reading an encounter's reward entities,
    // and it only ever did because the retire was wedged into its switch loop.
) {
    // The session gate stays: this system spawns nothing now, but it wrote
    // persisted switch state and quest flags before the drain split out, and
    // gating the whole driver on a live session is the behaviour it has always
    // had.
    if commands.spawn_scope().is_none() {
        return;
    }
    // Each occurrence runs in its own live room (see
    // `ambition_encounter::occurrence`): its stamp names it, and a player, a
    // mob or a death belongs to it when it is in that room.
    let room_of = |entity: Entity| rooms.live().of(entity);
    let is_live = |room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>| {
        room.is_some_and(|room| rooms.definition_in(room).is_some())
    };
    if player_body_q.is_empty() {
        return;
    }
    // Sim clock: encounter trigger / cancellation timers freeze in
    // bullet-time alongside the player (ADR 0010); we don't want a
    // grace-window to tick down while the world is stopped.
    let dt = world_time.sim_dt();

    // 0. Player death this frame? Fail each in-flight encounter of the room
    //    the player died in (the trace / save see the loss), then Reset it in
    //    the same command batch so the trigger re-fires cleanly on re-entry.
    //    The ownership-driven cleanup adapter (E10) reacts to the resulting
    //    Failed/Reset events — no despawn logic here.
    //
    //    ⛔ A DEATH ENDS ITS OWN ROOM'S ATTEMPT (OW1 cut 7f). With two live
    //    rooms, the other player's fight goes on. A death whose room cannot be
    //    told ends nothing.
    let mut ending_this_tick: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    let died_in: std::collections::BTreeSet<_> = died_messages
        .read()
        .filter_map(|died| room_of(died.victim))
        .collect();
    if !died_in.is_empty() {
        for (occurrence, enc, lifecycle, _waves, _participants) in &encounters {
            let room = room_of(occurrence);
            if lifecycle.phase().in_flight() && room.is_some_and(|room| died_in.contains(&room)) {
                lifecycle_commands.write(
                    EncounterCommand::new(&enc.id, EncounterCommandKind::Fail).in_room(room),
                );
                lifecycle_commands.write(
                    EncounterCommand::new(&enc.id, EncounterCommandKind::Reset).in_room(room),
                );
                ending_this_tick.insert(occurrence);
            }
        }
    }

    // 1. Reset encounters whose area the player has left, so the camera zoom
    //    + lock release on exit. (E10 makes this cleanup ownership-driven: the
    //    Reset event despawns the encounter's SPAWNED mobs — pre-E10 they
    //    lingered until a death or re-arm, which was accidental, not policy.)
    for (occurrence, enc, lifecycle, _waves, _participants) in &encounters {
        let room = room_of(occurrence);
        if lifecycle.phase().in_flight() && !is_live(room) {
            lifecycle_commands
                .write(EncounterCommand::new(&enc.id, EncounterCommandKind::Reset).in_room(room));
            ending_this_tick.insert(occurrence);
        }
    }

    // 2. Trigger entry. The SWITCH is the source of truth for "armed":
    //    switch off = armed (red), switch on = disabled (green). A stale
    //    terminal phase resets in the same command batch (the reducer applies
    //    Reset then Start in order), so a persisted Completed/Failed doesn't
    //    lock out re-triggering after a switch toggle.
    // The first encounter of each live room, as the one room had one.
    let mut triggered_rooms = std::collections::BTreeSet::new();
    for (occurrence, enc, lifecycle, waves, mut participants) in encounters.iter_mut() {
        let room = room_of(occurrence);
        if !is_live(room) || !triggered_rooms.insert(room) {
            continue;
        }
        if !lifecycle.phase().in_flight() && switch_index.encounter_armed(&enc.id) {
            // Iterate every player so any player walking into the trigger
            // fires the encounter — single-player behavior preserved because
            // the iterator has one entity today. OVERNIGHT-TODO #17.8.
            let trigger = waves.spec.trigger_aabb();
            // A player in this occurrence's live room.
            let entered = player_body_q
                .iter()
                .filter(|(player, _)| room_of(*player) == room)
                .any(|(_, body)| {
                use bevy::math::bounding::IntersectsVolume;
                let player_aabb = ae::aabb_from_min_size(
                    ae::Vec2::new(
                        body.pos.x - body.size.x * 0.5,
                        body.pos.y - body.size.y * 0.5,
                    ),
                    body.size,
                );
                trigger.intersects(&player_aabb)
            });
            if entered {
                if !matches!(
                    lifecycle.phase(),
                    ambition_encounter::EncounterPhase::Inactive
                ) {
                    lifecycle_commands.write(
                        EncounterCommand::new(&enc.id, EncounterCommandKind::Reset).in_room(room),
                    );
                }
                participants.members.clear();
                lifecycle_commands
                    .write(EncounterCommand::new(&enc.id, EncounterCommandKind::Start).in_room(room));
            }
        }
    }

    // 3. Drive the active-area wave director while its lifecycle is Active
    //    (the reducer's phase from this frame's Progression pass — the
    //    adapters read the authority, one frame behind at most).
    // (instance id, character, brain kind, pos, size) — the three identity
    // questions kept apart all the way to the spawner.
    for (occurrence, enc, lifecycle, mut waves, mut participants) in &mut encounters {
        let room = room_of(occurrence);
        if !is_live(room) || ending_this_tick.contains(&occurrence) {
            continue;
        }
        match lifecycle.phase() {
            ambition_encounter::EncounterPhase::Active => {
                // Refresh each Minion participant's liveness + cached entity
                // from the runtime BEFORE the director tick (live resolution
                // is a cache; the durable identity is the id). Mobs spawned
                // later this tick are appended with `alive = true` and
                // refreshed next frame (by then their entities exist).
                let lookup: std::collections::HashMap<String, (Entity, bool)> = encounter_mobs
                    .iter()
                    // This occurrence's mobs: its id, in its room. Two
                    // occurrences mint the same mob ids.
                    .filter(|(mob_entity, mob, _, _)| {
                        mob.encounter_id == enc.id && room_of(*mob_entity) == room
                    })
                    // AC3.1.A: participant liveness decides wave completion, so it
                    // reads the HP authority rather than the once-per-frame mirror.
                    .map(|(entity, _, id, health)| {
                        (id.as_str().to_string(), (entity, health.alive()))
                    })
                    .collect();
                for member in &mut participants.members {
                    match lookup.get(&member.id) {
                        Some((entity, alive)) => {
                            member.entity = Some(*entity);
                            member.alive = *alive;
                        }
                        None => {
                            member.entity = None;
                            member.alive = false;
                        }
                    }
                }
                let mut events = Vec::new();
                let exhausted = waves.tick_active(dt, &mut participants, &mut events);
                if exhausted {
                    lifecycle_commands.write(
                        EncounterCommand::signal(&enc.id, WAVES_EXHAUSTED_SIGNAL).in_room(room),
                    );
                }
                for event in events {
                    // The SpawnCommands go out on the bus like every other
                    // event; `features::serve_encounter_spawn_commands` reads
                    // them. This driver no longer serves its own requests.
                    // The room goes with the request: a mob is served into
                    // its occurrence's live room.
                    events_out.write(EncounterEventMsg::new(&enc.id, event).in_room(room));
                }
            }
            ambition_encounter::EncounterPhase::Inactive => {
                // A fresh attempt begins with a fresh run (spawn_counter
                // survives so mob ids never collide across attempts).
                if waves.run.wave_index.is_some() || waves.run.exhausted_signaled {
                    waves.reset_run();
                }
            }
            _ => {}
        }
    }

    // 4. Spawn requests are SERVED ELSEWHERE (2026-09-03). The wave director
    //    emits `EncounterEvent::SpawnCommand` and this system used to pull them
    //    out of its own local vector and build the bodies itself — driving and
    //    serving in one place. `features::serve_encounter_spawn_commands` is
    //    the server now: the domain says what it wants spawned, and the layer
    //    that owns body construction decides how.

    // 5. Switch REACTIONS. The queue is drained ONCE, by
    //    `ambition_encounter::switches::drain_switch_activations`, which parses
    //    each action into a typed `SwitchAction` and owns the persisted switch
    //    write. This reads the published result.
    //
    //    ⛔ WHY THE DRAIN LEFT: four unrelated policies used to share this loop
    //    — a quest flag for every activation, FlipGravity, the four SetGravity
    //    faces, and the encounter reset — and each was reachable only from
    //    INSIDE it, after a save-mutating toggle and behind early `continue`s.
    //    That is what pinned the reward retire to this adapter. Order is still
    //    part of the value, so there is still exactly one drain; what changed is
    //    that it is not this system.
    for activation in &resolved_switches.0 {
        // Quest hook: every switch interaction sets a generic flag that quests
        // can listen for, whatever the action was.
        save.data_mut().set_flag("test_switch_toggled", true);
        save.data_mut().set_flag(
            ambition_encounter::switches::switch_used_flag(&activation.id),
            true,
        );
        quests.push_event(ambition_persistence::quest::QuestAdvanceEvent::FlagSet(
            "test_switch_toggled".into(),
        ));
        match &activation.action {
            // A gravity switch turns the ambient of the live room it is in,
            // and of no other (customer 2): Bob's switch does not turn Alice's
            // world. An activation that names no room is the sole live room's.
            ambition_encounter::switches::SwitchAction::FlipGravity => {
                let room = activation.room;
                commands.queue(move |world: &mut bevy::prelude::World| {
                    let room = room.or_else(|| {
                        ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<
                            ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
                        >(world)
                        .copied()
                    });
                    let mut base = world
                        .resource_mut::<ambition_platformer2d_shared_tangle::gravity::BaseGravity>(
                        );
                    let dir = -base.dir_in(room);
                    base.turn(room, dir);
                });
            }
            // Cardinal gravity switch (Noether Chamber kernel faces): the face
            // becomes the new "down". Deferred world command (tuple limit).
            ambition_encounter::switches::SwitchAction::SetGravity(face) => {
                let [x, y] = face.direction();
                let dir = bevy::prelude::Vec2::new(x, y);
                let room = activation.room;
                commands.queue(move |world: &mut bevy::prelude::World| {
                    let room = room.or_else(|| {
                        ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<
                            ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
                        >(world)
                        .copied()
                    });
                    world
                        .resource_mut::<ambition_platformer2d_shared_tangle::gravity::BaseGravity>()
                        .turn(room, dir);
                });
            }
            ambition_encounter::switches::SwitchAction::ResetEncounter => {
                if !activation.on {
                    // Re-arming: Reset the encounter (the reducer refuses Start
                    // from a terminal phase, so a stale Completed/Failed must
                    // clear); the ownership-driven cleanup adapter (E10) drops
                    // carryover mobs off the Reset event.
                    // An unnamed target is the switch's own room's.
                    let area = rooms
                        .definition_named(activation.room)
                        .map(|definition| rooms.rooms().spec(definition).id.clone())
                        .unwrap_or_default();
                    let target = activation.target_encounter_in(
                        &area,
                        encounters
                            .iter()
                            .map(|(_, enc, _, waves, _)| (enc.id.as_str(), waves.spec.room_id.as_str())),
                    );
                    // The occurrence in the switch's own live room.
                    if let Some((occurrence, enc, lifecycle, _, _)) = target.and_then(|id| {
                        encounters.iter().find(|(occurrence, enc, ..)| {
                            enc.id == id
                                && ambition_encounter::occurrence::addresses(
                                    rooms.live(),
                                    activation.room,
                                    *occurrence,
                                )
                        })
                    }) {
                        if !lifecycle.phase().in_flight() {
                            lifecycle_commands.write(
                                EncounterCommand::new(&enc.id, EncounterCommandKind::Reset)
                                    .in_room(room_of(occurrence)),
                            );
                        }
                    }
                    // ⭐ THE REWARD RETIRE LEFT (2026-09-03). It is
                    // `features::retire_rewards_for_rearmed_encounters` now,
                    // on the runtime-composed reward plugin, reacting to the
                    // same published activation this arm reads. It could not be
                    // a system until the drain split out: its trigger was a
                    // POSITION in this loop, and nothing outside could observe
                    // the edge.
                }
            }
            // ⭐⭐ THE VARIANT'S OWN DOC SAYS IT IS "carried rather than dropped
            // SO A CONSUMER CAN REPORT IT" -- and until 2026-09-06 both consumers
            // ignored it silently, so the sentence was true about the type and
            // false about the tree. An authored `SetGravtiyUp` typo reached here
            // and vanished; the string road it replaced could not tell an
            // unhandled action from a handled one that did nothing, and neither
            // could this.
            // ⚠ It fires on NOTHING today: all 14 authored switch actions across
            // the shipped worlds parse (8 `ResetEncounter`, 1 `FlipGravity`, 4
            // `SetGravity<Face>`, 1 `ToggleFlag`; counted 2026-09-29 over every
            // `.ldtk` and `.ldtkl`). An earlier count here said 85 of 85, while
            // the one `ToggleFlag` switch fell into this arm on every press. This
            // is a latent report for the first typo, not noise -- which is why it
            // is a warning and not an error.
            // The persisted toggle is the whole effect, and the drain made it.
            ambition_encounter::switches::SwitchAction::ToggleFlag => {}
            ambition_encounter::switches::SwitchAction::Unhandled(action) => {
                bevy::log::warn!(
                    target: "ambition_encounter::switches",
                    "switch {} names action {:?}, which this engine does not act on \
                     -- the switch will toggle and do nothing",
                    activation.id,
                    action,
                );
            }
        }
    }
}

/// Wave EFFECT adapter (Progression, after the generic reducer): reacts to
/// this frame's lifecycle events and projects wave-encounter state onto its
/// consumers — switch auto-green + celebration + quest + mob cleanup on
/// completion, reward-chest sync, music request, presentation read-model,
/// save projection, and the trace sink for every encounter event.
pub fn apply_wave_encounter_effects(
    // Not `mut`: this adapter stopped spawning when the reward sync left.
    commands: SessionCommands<'_, '_>,
    mut events_in: MessageReader<EncounterEventMsg>,
    encounters: Query<(
        &Encounter,
        &EncounterLifecycle,
        Option<&EncounterWaves>,
        Option<&EncounterParticipants>,
    )>,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
    switch_index: Res<EncounterSwitchIndex>,
    mut trace: ResMut<ambition_gameplay_trace::GameplayTraceBuffer>,
    player_body_q: Query<
        &ambition_platformer2d_core::BodyKinematics,
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
    mut music_request: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldMut<
        EncounterMusicRequest,
    >,
    mut encounter_view: ResMut<EncounterView>,
    mut quests: ResMut<ambition_persistence::quest::QuestRegistry>,
    mut banner_requests: MessageWriter<ambition_combat::events::GameplayBannerRequested>,
    // The staging-policy view (E12): lifecycle + authored presentation
    // effects, with no wave requirement — any encounter kind stages alike.
    staged: Query<(
        Entity,
        &EncounterLifecycle,
        Option<&ambition_encounter::EncounterCameraZoom>,
        Option<&ambition_encounter::EncounterTrack>,
    )>,
    // The live room of each staged encounter, whose music it asks for.
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    // ⭐ THIS ADAPTER NO LONGER SPAWNS ANYTHING. The reward-chest sync it used to
    // call was its only spawner; reward chests are the feature layer's now, and
    // the chest query left with them.
    // ⛔ The GUARD stays. It gated this whole system on a live session, so
    // dropping it would newly run the trace, quest, banner and music
    // projections in a world that has no session — a behaviour change that
    // belongs to whoever removes the last caller, not to this inversion.
    if commands.spawn_scope().is_none() {
        return;
    }
    // Trace sink first — every encounter event (generic reducer + wave
    // director) lands in the gameplay trace regardless of the player guard
    // below, in the same `encounter:<id>:<label>` format as before E8.
    let tick = trace.current_tick();
    let mut completed_wave_ids: Vec<String> = Vec::new();
    for msg in events_in.read() {
        trace.push_event(ambition_gameplay_trace::GameplayTraceEvent::Sfx {
            tick,
            label: format!("encounter:{}:{}", msg.encounter, msg.event.label()),
        });
        if matches!(msg.event, EncounterEvent::Completed) {
            // Wave-encounter completion effects apply only to encounters that
            // actually carry the wave policy (a boss wrap or signal encounter
            // has its own reward/consequence adapters).
            let is_wave = encounters
                .iter()
                .any(|(enc, _, waves, _)| enc.id == msg.encounter && waves.is_some());
            if is_wave {
                completed_wave_ids.push(msg.encounter.clone());
            }
        }
    }
    // ⛔⛔ ABOVE THE PLAYER-BODY GUARD, AND THAT IS THE POINT. This sat BELOW
    // it, so the sentence above — "writing the base source every frame,
    // including `None`" — described a property the code did not have.
    // `player_body_q` is empty during a DEATH and across a ROOM TRANSITION,
    // which is exactly when an encounter stops being in flight, so the track
    // LATCHED at its last value and kept playing into the next room.
    // ⇒ It reads only `staged`; nothing between the guard and here touched the
    // player body, so the guard never protected this projection — it only
    // sequenced it.
    // Jon 2026-09-06: "the music changes in a way I was not expecting and
    // seems to get into some sort of stuck state."

    // Music: in each live room, pick the first encounter currently in flight
    // with an authored track and request it (the base-priority source of the
    // shared `EncounterMusicRequest`); a room with none gets none. Generic over the
    // lifecycle + staging policy (E12). Writing the base source every frame —
    // including `None` — is safe: `desired_track()` ranks `priority_track`
    // above `base_track`, so this can't clobber a concurrent focused fight's
    // music.
    let active_tracks = staged.iter().filter_map(|(occurrence, lifecycle, _, track)| {
        if lifecycle.phase().in_flight() {
            track.map(|t| (live.of(occurrence), t.0.clone()))
        } else {
            None
        }
    });
    music_request.set_base_tracks(active_tracks);

    if player_body_q.is_empty() {
        return;
    }

    // Completion effects: auto-flip the linked switch to on (green) so the player can see they
    // finished it, surface a celebration banner, and advance any "clear encounter" quest step.
    for encounter_id in &completed_wave_ids {
        // ⛔ ALL of them, not the first. `encounter_armed` arms on ANY red link,
        // so greening one switch of two leaves the encounter armed and the
        // driver re-starts the fight it just completed. See
        // `EncounterSwitchIndex::switch_ids_for_encounter`.
        for switch_id in switch_index.switch_ids_for_encounter(encounter_id) {
            save.data_mut().set_switch(&switch_id, true);
        }
        banner_requests.write(ambition_combat::events::GameplayBannerRequested::new(
            format!("ARENA CLEAR — {encounter_id}"),
            3.0,
        ));
        quests.push_event(
            ambition_persistence::quest::QuestAdvanceEvent::EncounterCleared(encounter_id.clone()),
        );
    }

    // ⭐ REWARD CHESTS ARE NOT SYNCED FROM HERE ANY MORE. This adapter used to
    // read `EncounterLifecycle::phase`, assemble the cleared `(id, spec)` pairs
    // and push them into the feature layer. The encounter domain publishes
    // `ambition_encounter::rewards::ClearedEncounters` now and the feature
    // layer's own `EncounterRewardSyncPlugin` reads it, so the kernel no longer
    // has to know how an encounter says "completed".


    // Publish the presentation read-model (§6): the camera zoom the active
    // encounters want, from the authored staging policy (E12). Cross-crate
    // presentation reads `EncounterView`, not the entities. `max`-based, so
    // it is query-order-independent.
    encounter_view.camera_zoom = ambition_encounter::active_encounter_camera_zoom(
        staged
            .iter()
            .filter_map(|(_, lifecycle, zoom, _)| zoom.map(|z| (lifecycle.phase(), z.0))),
    );

    // Project the lifecycle to the save (Completed/Failed survive, in-flight
    // collapses to Untouched). Wave encounters only — a boss wrap persists
    // through `save.bosses`, keyed by placement.
    //
    // ⛔ ONE SAVE KEY, ANY NUMBER OF OCCURRENCES. The save keys the authored
    // encounter, and two live rooms of one room each run an occurrence of it.
    // Writing each in query order flipped the key between them every frame, so
    // the occurrences are folded first: a clear in any room is the clear.
    let mut folded: std::collections::BTreeMap<
        &str,
        ambition_persistence::save_data::PersistedEncounterState,
    > = std::collections::BTreeMap::new();
    for (enc, lifecycle, waves, _) in &encounters {
        if waves.is_none() {
            continue;
        }
        let persisted = lifecycle.to_persisted();
        folded
            .entry(enc.id.as_str())
            .and_modify(|held| *held = the_outcome_that_stands(*held, persisted))
            .or_insert(persisted);
    }
    for (id, persisted) in folded {
        if save.data().encounter(id) != persisted {
            save.data_mut().set_encounter(id, persisted);
        }
    }
}

/// Ownership-driven participant cleanup (E10): when an encounter's lifecycle
/// ENDS (Completed / Failed / Reset), consult each participant's [`Ownership`]
/// and the encounter's optional
/// [`EncounterCleanupPolicy`](ambition_encounter::EncounterCleanupPolicy):
///
/// - Adopted participants are NEVER touched — they pre-existed the
///   orchestration (a boss survives its wrap retiring).
/// - Spawned participants despawn under the default
///   [`SpawnedCleanup::DespawnOnEnd`](ambition_encounter::SpawnedCleanup)
///   (and their relation records leave the list — the entities left the
///   world); an authored `Keep` policy hands them to the room instead.
///
/// Cleanup never asks what KIND of encounter ended — the relations + policy
/// carry everything. Resolution uses the cached `member.entity`, falling back
/// to the wave-mob id lookup for a participant spawned so recently the cache
/// has not seen its entity yet (same-tick end).
pub fn apply_encounter_cleanup(
    mut commands: Commands,
    mut events_in: MessageReader<EncounterEventMsg>,
    mut encounters: Query<(
        Entity,
        &Encounter,
        &mut EncounterParticipants,
        Option<&ambition_encounter::EncounterCleanupPolicy>,
    )>,
    // Which occurrence ended, and which room its mobs are in: two occurrences
    // share an id and mint the same mob ids.
    live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    // The GENERIC durable-id → live-entity resolution: a participant's id is
    // the payload of its body's `SimId::placement(..)` — for a wave mob (its
    // `FeatureId`) and a boss member (its config id) alike. Resolving through
    // canonical simulation identity (not a type-specific marker query) means a
    // snapshot-restored participant, whose entity CACHE is nulled by design,
    // still cleans up correctly even if the encounter ends before a
    // specialized adapter re-heals the cache.
    sim_entities: Query<(Entity, &ambition_platformer2d_shared_tangle::sim_id::SimId)>,
) {
    let mut ended: Vec<(String, Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>)> =
        Vec::new();
    for msg in events_in.read() {
        let occurrence = (msg.encounter.clone(), msg.room);
        if matches!(
            msg.event,
            EncounterEvent::Completed | EncounterEvent::Failed | EncounterEvent::Reset
        ) && !ended.contains(&occurrence)
        {
            ended.push(occurrence);
        }
    }
    for (encounter_id, room) in ended {
        let room = ambition_encounter::occurrence::message_room(&live, room);
        let Some((_, _, mut participants, policy)) = encounters
            .iter_mut()
            .find(|(occurrence, enc, _, _)| enc.id == encounter_id && live.of(*occurrence) == room)
        else {
            continue;
        };
        let policy = policy.copied().unwrap_or_default();
        let despawn = matches!(
            policy.spawned,
            ambition_encounter::SpawnedCleanup::DespawnOnEnd
        );
        // Both policies RELEASE the spawned participants from the ended
        // encounter — the relation reflects what the encounter still owns,
        // which after its end is nothing it spawned. `DespawnOnEnd`
        // additionally removes the released bodies from the world; `Keep`
        // leaves them alive as ordinary unowned actors (explicit release
        // semantics, not a silent still-owned leftover).
        participants.members.retain(|member| {
            if member.ownership != ambition_encounter::Ownership::Spawned {
                return true;
            }
            if despawn {
                let wanted =
                    ambition_platformer2d_shared_tangle::sim_id::SimId::placement(&member.id);
                let entity = member.entity.or_else(|| {
                    sim_entities
                        .iter()
                        .find(|(entity, sim)| **sim == wanted && live.of(*entity) == room)
                        .map(|(entity, _)| entity)
                });
                if let Some(entity) = entity {
                    if let Ok(mut entity_commands) = commands.get_entity(entity) {
                        entity_commands.despawn();
                    }
                }
            }
            false
        });
    }
}

/// Which of two occurrences' outcomes the save keeps for their one authored
/// encounter: a clear over a loss, and either over no outcome.
fn the_outcome_that_stands(
    held: ambition_persistence::save_data::PersistedEncounterState,
    other: ambition_persistence::save_data::PersistedEncounterState,
) -> ambition_persistence::save_data::PersistedEncounterState {
    use ambition_persistence::save_data::PersistedEncounterState as Outcome;
    let rank = |outcome: Outcome| match outcome {
        Outcome::Cleared => 2,
        Outcome::Failed => 1,
        _ => 0,
    };
    if rank(other) > rank(held) {
        other
    } else {
        held
    }
}
