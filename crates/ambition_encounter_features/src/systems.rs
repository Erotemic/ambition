//! The Bevy adapters around the generic encounter lifecycle (E8/E9).
//!
//! `populate_encounter_registry` (startup) loads specs from LDtk + the save and
//! spawns one encounter ENTITY per spec carrying the generic authority set
//! (`Encounter` + `EncounterLifecycle` + `EncounterObjective` +
//! `EncounterParticipants`) plus the wave policy (`EncounterWaves`).
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
    EncounterLifecycle, EncounterMusicRequest, EncounterParticipants, EncounterRegistry,
    EncounterView, EncounterWaves, WAVES_EXHAUSTED_SIGNAL,
};

use crate::load_encounter_specs_from_rooms;
use ambition_encounter::switches::EncounterSwitchIndex;

/// Bevy startup system: load encounter specs from the embedded LDtk
/// project, spawn one encounter entity per spec carrying the generic
/// authority set + the wave policy, and apply persisted states from the save.
///
/// The authorities are SESSION-SCOPED: they belong to the live gameplay
/// session exactly like the boss wraps, so retiring the session tears them
/// down with everything else it owns. An unscoped authority would survive
/// retirement while `SessionTeardownPlugin` clears the registry — and the
/// next session's repopulation would then mint a DUPLICATE entity (and a
/// duplicate `SimId::encounter`) per spec.
pub fn populate_encounter_registry(
    mut commands: ambition_platformer2d_shared_tangle::lifecycle::SessionCommands,
    mut registry: ResMut<EncounterRegistry>,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    // , and it is done: encounters come off the ROOM IR now, not off an `LdtkProject`.
    // `EncounterTrigger` and `LockWall` are ordinary emissions like every other authored
    // family, which is what took the LDtk crate out of this file.
    //
    // Optional because a composition may have no rooms installed — a headless
    // fixture, a shell at a non-gameplay route.
    rooms: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
    >,
    // the App's authored wave book. Optional for the same reason the project is: a composition
    // with no authored encounters is an empty set, not an error.
    waves: Option<Res<ambition_encounter::EncounterWaveBook>>,
) {
    if registry.specs_loaded {
        return;
    }
    // A shell host at a non-gameplay route has no session to own the
    // authorities: sleep WITHOUT setting `specs_loaded`, so the first tick of
    // an activated session populates. A legacy/headless app with no session
    // lifecycle installed gets the unscoped spawn mode, as before.
    let Some(scope) = commands.spawn_scope() else {
        return;
    };
    // Returning without latching costs one `Option` test per tick and cannot. nothing outside this
    // system reads `EncounterRegistry::specs_loaded` (checked), so never latching in a room-less
    // composition is inert.
    let Some(rooms) = rooms else {
        return;
    };
    let entries = load_encounter_specs_from_rooms(&rooms.rooms, save.data(), waves.as_deref());
    let count = entries.len();
    for (id, spec, persisted) in entries {
        let lifecycle = EncounterLifecycle::from_persisted(spec.intro_seconds, persisted);
        let waves = EncounterWaves::new(spec);
        let objective = waves.objective();
        let mut entity = commands.spawn((
            Encounter::new(id.clone()),
            // Stable simulation identity (E11): the authority enters the
            // snapshot roster / state hash under its own namespace.
            ambition_platformer2d_shared_tangle::sim_id::SimId::encounter(&id),
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
        scope.apply_to(&mut entity);
        let entity = entity.id();
        registry.insert(id, entity);
    }
    registry.specs_loaded = true;
    // One-line census so "did encounters load?" is checkable from
    // the log without grepping the LDtk. Mirrors the pattern in
    // `populate_boss_encounter_registry` + the catalog sprite census.
    bevy::log::info!(
        target: "ambition_platformer2d::encounter",
        "encounter registry: {count} encounter entit(ies) spawned from the room set",
    );
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
        &ambition_platformer2d_core::BodyKinematics,
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
    session_world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_world::rooms::RoomSet,
    >,
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
    let active_area = session_world.active_spec().id.clone();
    if player_body_q.is_empty() {
        return;
    }
    // Sim clock: encounter trigger / cancellation timers freeze in
    // bullet-time alongside the player (ADR 0010); we don't want a
    // grace-window to tick down while the world is stopped.
    let dt = world_time.sim_dt();

    // 0. Player death this frame? Fail any in-flight encounter (the trace /
    //    save see the loss), then Reset it in the same command batch so the
    //    trigger re-fires cleanly on re-entry. The ownership-driven cleanup
    //    adapter (E10) reacts to the resulting Failed/Reset events — no
    //    despawn logic here.
    let mut ending_this_tick: std::collections::HashSet<String> = std::collections::HashSet::new();
    let died_this_frame = died_messages.read().next().is_some();
    if died_this_frame {
        for (enc, lifecycle, _waves, _participants) in &encounters {
            if lifecycle.phase().in_flight() {
                lifecycle_commands
                    .write(EncounterCommand::new(&enc.id, EncounterCommandKind::Fail));
                lifecycle_commands
                    .write(EncounterCommand::new(&enc.id, EncounterCommandKind::Reset));
                ending_this_tick.insert(enc.id.clone());
            }
        }
    }

    // 1. Reset encounters whose area the player has left, so the camera zoom
    //    + lock release on exit. (E10 makes this cleanup ownership-driven: the
    //    Reset event despawns the encounter's SPAWNED mobs — pre-E10 they
    //    lingered until a death or re-arm, which was accidental, not policy.)
    for (enc, lifecycle, _waves, _participants) in &encounters {
        if lifecycle.phase().in_flight() && enc.id != active_area {
            lifecycle_commands.write(EncounterCommand::new(&enc.id, EncounterCommandKind::Reset));
            ending_this_tick.insert(enc.id.clone());
        }
    }

    // 2. Trigger entry. The SWITCH is the source of truth for "armed":
    //    switch off = armed (red), switch on = disabled (green). A stale
    //    terminal phase resets in the same command batch (the reducer applies
    //    Reset then Start in order), so a persisted Completed/Failed doesn't
    //    lock out re-triggering after a switch toggle.
    let armed_active = switch_index.encounter_armed(&active_area);
    if let Some((enc, lifecycle, waves, mut participants)) = encounters
        .iter_mut()
        .find(|(enc, _, _, _)| enc.id == active_area)
    {
        if !lifecycle.phase().in_flight() && armed_active {
            // Iterate every player so any player walking into the trigger
            // fires the encounter — single-player behavior preserved because
            // the iterator has one entity today. OVERNIGHT-TODO #17.8.
            let trigger = waves.spec.trigger_aabb();
            let entered = player_body_q.iter().any(|body| {
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
                    lifecycle_commands
                        .write(EncounterCommand::new(&enc.id, EncounterCommandKind::Reset));
                }
                participants.members.clear();
                lifecycle_commands
                    .write(EncounterCommand::new(&enc.id, EncounterCommandKind::Start));
            }
        }
    }

    // 3. Drive the active-area wave director while its lifecycle is Active
    //    (the reducer's phase from this frame's Progression pass — the
    //    adapters read the authority, one frame behind at most).
    // (instance id, character, brain kind, pos, size) — the three identity
    // questions kept apart all the way to the spawner.
    for (enc, lifecycle, mut waves, mut participants) in &mut encounters {
        if enc.id != active_area || ending_this_tick.contains(&enc.id) {
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
                    .filter(|(_, mob, _, _)| mob.encounter_id == enc.id)
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
                    lifecycle_commands
                        .write(EncounterCommand::signal(&enc.id, WAVES_EXHAUSTED_SIGNAL));
                }
                for event in events {
                    // The SpawnCommands go out on the bus like every other
                    // event; `features::serve_encounter_spawn_commands` reads
                    // them. This driver no longer serves its own requests.
                    events_out.write(EncounterEventMsg::new(&enc.id, event));
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
        save.data_mut()
            .set_flag(format!("switch_{}_used", activation.id), true);
        quests.push_event(ambition_persistence::quest::QuestAdvanceEvent::FlagSet(
            "test_switch_toggled".into(),
        ));
        match &activation.action {
            ambition_encounter::switches::SwitchAction::FlipGravity => {
                commands.queue(|world: &mut bevy::prelude::World| {
                    let mut base = world
                        .resource_mut::<ambition_platformer2d_shared_tangle::gravity::BaseGravity>(
                        );
                    base.dir = -base.dir;
                });
            }
            // Cardinal gravity switch (Noether Chamber kernel faces): the face
            // becomes the new "down". Deferred world command (tuple limit).
            ambition_encounter::switches::SwitchAction::SetGravity(face) => {
                let [x, y] = face.direction();
                let dir = bevy::prelude::Vec2::new(x, y);
                commands.queue(move |world: &mut bevy::prelude::World| {
                    world
                        .resource_mut::<ambition_platformer2d_shared_tangle::gravity::BaseGravity>()
                        .dir = dir;
                });
            }
            ambition_encounter::switches::SwitchAction::ResetEncounter => {
                let target_id = if activation.target_encounter.is_empty() {
                    active_area.clone()
                } else {
                    activation.target_encounter.clone()
                };
                if !activation.on {
                    // Re-arming: Reset the encounter (the reducer refuses Start
                    // from a terminal phase, so a stale Completed/Failed must
                    // clear); the ownership-driven cleanup adapter (E10) drops
                    // carryover mobs off the Reset event.
                    if let Some((_, lifecycle, _, _)) =
                        encounters.iter().find(|(enc, _, _, _)| enc.id == target_id)
                    {
                        if !lifecycle.phase().in_flight() {
                            lifecycle_commands.write(EncounterCommand::new(
                                &target_id,
                                EncounterCommandKind::Reset,
                            ));
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
            // Authored but not a kind this engine acts on. The string road could
            // not tell this apart from a handled action that did nothing.
            ambition_encounter::switches::SwitchAction::Unhandled(_) => {}
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
        &EncounterLifecycle,
        Option<&ambition_encounter::EncounterCameraZoom>,
        Option<&ambition_encounter::EncounterTrack>,
    )>,
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

    // Music: pick the first encounter currently in flight with an authored
    // track and request it (the base-priority source of the shared
    // `EncounterMusicRequest`); otherwise clear it. Generic over the
    // lifecycle + staging policy (E12). Writing the base source every frame —
    // including `None` — is safe: `desired_track()` ranks `priority_track`
    // above `base_track`, so this can't clobber a concurrent focused fight's
    // music.
    let active_track = staged.iter().find_map(|(lifecycle, _, track)| {
        if lifecycle.phase().in_flight() {
            track.map(|t| t.0.clone())
        } else {
            None
        }
    });
    music_request.base_track = active_track;

    // Publish the presentation read-model (§6): the camera zoom the active
    // encounters want, from the authored staging policy (E12). Cross-crate
    // presentation reads `EncounterView`, not the entities. `max`-based, so
    // it is query-order-independent.
    encounter_view.camera_zoom = ambition_encounter::active_encounter_camera_zoom(
        staged
            .iter()
            .filter_map(|(lifecycle, zoom, _)| zoom.map(|z| (lifecycle.phase(), z.0))),
    );

    // Project the lifecycle to the save (Completed/Failed survive, in-flight
    // collapses to Untouched). Wave encounters only — a boss wrap persists
    // through `save.bosses`, keyed by placement.
    for (enc, lifecycle, waves, _) in &encounters {
        if waves.is_none() {
            continue;
        }
        let persisted = lifecycle.to_persisted();
        let current = save.data().encounter(&enc.id);
        if persisted != current {
            save.data_mut().set_encounter(&enc.id, persisted);
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
        &Encounter,
        &mut EncounterParticipants,
        Option<&ambition_encounter::EncounterCleanupPolicy>,
    )>,
    // The GENERIC durable-id → live-entity resolution: a participant's id is
    // the payload of its body's `SimId::placement(..)` — for a wave mob (its
    // `FeatureId`) and a boss member (its config id) alike. Resolving through
    // canonical simulation identity (not a type-specific marker query) means a
    // snapshot-restored participant, whose entity CACHE is nulled by design,
    // still cleans up correctly even if the encounter ends before a
    // specialized adapter re-heals the cache.
    sim_entities: Query<(Entity, &ambition_platformer2d_shared_tangle::sim_id::SimId)>,
) {
    let mut ended: Vec<String> = Vec::new();
    for msg in events_in.read() {
        if matches!(
            msg.event,
            EncounterEvent::Completed | EncounterEvent::Failed | EncounterEvent::Reset
        ) && !ended.contains(&msg.encounter)
        {
            ended.push(msg.encounter.clone());
        }
    }
    for encounter_id in ended {
        let Some((_, mut participants, policy)) = encounters
            .iter_mut()
            .find(|(enc, _, _)| enc.id == encounter_id)
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
                        .find(|(_, sim)| **sim == wanted)
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
