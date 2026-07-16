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

use ambition_engine_core as ae;
use ambition_platformer_primitives::lifecycle::SessionCommands;

use super::{
    load_encounter_specs_from_ldtk, Encounter, EncounterCommand, EncounterCommandKind,
    EncounterEvent, EncounterEventMsg, EncounterLifecycle, EncounterMusicRequest,
    EncounterParticipants, EncounterRegistry, EncounterSwitchIndex, EncounterView, EncounterWaves,
    SwitchActivationQueue, WAVES_EXHAUSTED_SIGNAL,
};

/// Bevy startup system: load encounter specs from the embedded LDtk
/// project, spawn one encounter entity per spec carrying the generic
/// authority set + the wave policy, and apply persisted states from the save.
pub fn populate_encounter_registry(
    mut commands: Commands,
    mut registry: ResMut<EncounterRegistry>,
    save: Res<ambition_persistence::save::SandboxSave>,
    // Optional: a RON-only app (demo shell, generated rooms) installs no
    // LDtk project — that's an empty encounter set, not an error. (W4 will
    // route encounter loading through RoomEmission instead of the project.)
    project: Option<Res<crate::ldtk_world::SandboxLdtkProject>>,
) {
    if registry.specs_loaded {
        return;
    }
    let Some(project) = project else {
        registry.specs_loaded = true;
        return;
    };
    let entries = load_encounter_specs_from_ldtk(&project.0, save.data());
    let count = entries.len();
    for (id, spec, persisted) in entries {
        let mut lifecycle = EncounterLifecycle::with_intro(spec.intro_seconds);
        lifecycle.apply_persisted(persisted);
        let waves = EncounterWaves::new(spec);
        let objective = waves.objective();
        let entity = commands
            .spawn((
                Encounter::new(id.clone()),
                // Stable simulation identity (E11): the authority enters the
                // snapshot roster / state hash under its own namespace.
                ambition_platformer_primitives::sim_id::SimId::encounter(&id),
                lifecycle,
                objective,
                waves,
                EncounterParticipants::default(),
            ))
            .id();
        registry.insert(id, entity);
    }
    registry.specs_loaded = true;
    // One-line census so "did encounters load?" is checkable from
    // the log without grepping the LDtk. Mirrors the pattern in
    // `populate_boss_encounter_registry` + the catalog sprite census.
    bevy::log::info!(
        target: "ambition::encounter",
        "encounter registry: {count} encounter entit(ies) spawned from LDtk",
    );
}

/// The wave COMMAND adapter + spawn-cadence director. Emits lifecycle commands
/// (never phase writes); the generic reducer applies them later this frame.
///
/// Cancellation policy (deliberate sandbox UX): an encounter is "in play" only
/// while the player is actually inside its area — walking out resets it so the
/// camera zoom + lock release on exit, and a fresh attempt fires on re-entry.
pub fn drive_wave_encounters(
    mut commands: SessionCommands<'_, '_>,
    world_time: Res<ambition_time::WorldTime>,
    mut died_messages: MessageReader<crate::ActorDiedMessage>,
    mut encounters: Query<(
        &Encounter,
        &EncounterLifecycle,
        &mut EncounterWaves,
        &mut EncounterParticipants,
    )>,
    mut save: ResMut<ambition_persistence::save::SandboxSave>,
    mut switch_activations: ResMut<SwitchActivationQueue>,
    switch_index: Res<EncounterSwitchIndex>,
    player_body_q: Query<&crate::actor::BodyKinematics, With<crate::actor::PlayerEntity>>,
    mut quests: ResMut<ambition_persistence::quest::QuestRegistry>,
    mut lifecycle_commands: MessageWriter<EncounterCommand>,
    mut events_out: MessageWriter<EncounterEventMsg>,
    session_content: (
        ambition_platformer_primitives::lifecycle::SessionWorldRef<crate::rooms::RoomSet>,
        Res<ambition_characters::actor::character_catalog::CharacterCatalog>,
        Res<crate::features::CharacterRoster>,
    ),
    encounter_mobs: Query<(
        Entity,
        &crate::features::EncounterMob,
        &crate::features::FeatureId,
        &ambition_characters::actor::BodyCombat,
    )>,
    reward_chests: Query<
        (
            Entity,
            &crate::features::EncounterRewardChest,
            &crate::features::FeatureId,
            Option<&crate::features::Opened>,
        ),
        With<crate::features::ChestFeature>,
    >,
) {
    let Some(session_scope) = commands.spawn_scope() else {
        return;
    };
    let active_area = session_content.0.active_spec().id.clone();
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
            if lifecycle.phase.in_flight() {
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
        if lifecycle.phase.in_flight() && enc.id != active_area {
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
        if !lifecycle.phase.in_flight() && armed_active {
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
                    lifecycle.phase,
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
    let mut spawn_commands: Vec<(String, String, [f32; 2], [f32; 2])> = Vec::new();
    for (enc, lifecycle, mut waves, mut participants) in &mut encounters {
        if enc.id != active_area || ending_this_tick.contains(&enc.id) {
            continue;
        }
        match lifecycle.phase {
            ambition_encounter::EncounterPhase::Active => {
                // Refresh each Minion participant's liveness + cached entity
                // from the runtime BEFORE the director tick (live resolution
                // is a cache; the durable identity is the id). Mobs spawned
                // later this tick are appended with `alive = true` and
                // refreshed next frame (by then their entities exist).
                let lookup: std::collections::HashMap<String, (Entity, bool)> = encounter_mobs
                    .iter()
                    .filter(|(_, mob, _, _)| mob.encounter_id == enc.id)
                    .map(|(entity, _, id, combat)| {
                        (id.as_str().to_string(), (entity, combat.alive))
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
                    if let EncounterEvent::SpawnCommand {
                        id,
                        kind,
                        pos,
                        size,
                    } = &event
                    {
                        spawn_commands.push((id.clone(), kind.clone(), *pos, *size));
                    }
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

    // 4. Apply spawn commands to ECS actor entities.
    for (id, kind, pos, size) in spawn_commands {
        crate::features::spawn_encounter_mob(
            &mut commands,
            &session_content.1,
            &session_content.2,
            session_scope,
            active_area.clone(),
            id,
            ambition_entity_catalog::placements::CharacterBrain::Custom(kind),
            ae::Vec2::new(pos[0], pos[1]),
            ae::Vec2::new(size[0], size[1]),
        );
    }

    // 5. Switch toggles. Just toggle the persisted switch state; the
    //    trigger gate consults `switch.on` directly. When the player
    //    re-arms (toggles to off), also drop any encounter-spawned
    //    mobs from a prior attempt and Reset any stale terminal phase
    //    so the next trigger fires cleanly.
    let activations = std::mem::take(&mut switch_activations.0);
    for activation in activations {
        // Quest hook: every switch interaction sets a generic flag
        // that quests can listen for. Specific switches will key on
        // their own ids via `switch:<id>` flags.
        save.data_mut().set_flag("test_switch_toggled", true);
        save.data_mut()
            .set_flag(format!("switch_{}_used", activation.id), true);
        quests.push_event(ambition_persistence::quest::QuestAdvanceEvent::FlagSet(
            "test_switch_toggled".into(),
        ));
        // Hub gravity switch: a `Switch` whose `action` is "FlipGravity" INVERTS
        // the room's ambient gravity ([`crate::physics::BaseGravity`]) — "down"
        // becomes the opposite of wherever it currently points, so the switch
        // still works after a Noether-Chamber sideways SetGravity (fable review
        // 2026-07-02 §B13: the old `dir.y = -dir.y` was a no-op on sideways
        // gravity). Done as a deferred world command so this system needn't take
        // `BaseGravity` as another param (Bevy's tuple limit).
        if activation.action.as_str() == "FlipGravity" {
            commands.queue(|world: &mut bevy::prelude::World| {
                let mut base = world.resource_mut::<crate::physics::BaseGravity>();
                base.dir = -base.dir;
            });
            let new_on = !save.data().switch(&activation.id);
            save.data_mut().set_switch(&activation.id, new_on);
            continue;
        }
        // Cardinal gravity switch (Noether Chamber kernel faces): a `Switch`
        // whose `action` is "SetGravityDown|Up|Left|Right" sets the room's
        // ambient gravity ([`crate::physics::BaseGravity`]) to that direction so
        // that side becomes the new "down". NOTE: the action must NOT contain a
        // colon — `SwitchActivation::to_custom_payload`/`parse_custom` round-trip
        // through a `:`-delimited string, so a colon in the action is silently
        // truncated. Deferred world command (tuple limit). Persist the
        // switch as on so its sprite reads engaged.
        if let Some(dir_token) = activation.action.as_str().strip_prefix("SetGravity") {
            let dir = match dir_token {
                "Up" => bevy::prelude::Vec2::new(0.0, -1.0),
                "Left" => bevy::prelude::Vec2::new(-1.0, 0.0),
                "Right" => bevy::prelude::Vec2::new(1.0, 0.0),
                _ => bevy::prelude::Vec2::new(0.0, 1.0), // "Down" / fallback
            };
            commands.queue(move |world: &mut bevy::prelude::World| {
                world.resource_mut::<crate::physics::BaseGravity>().dir = dir;
            });
            save.data_mut().set_switch(&activation.id, true);
            continue;
        }
        if !matches!(activation.action.as_str(), "ResetEncounter") {
            continue;
        }
        let new_on = !save.data().switch(&activation.id);
        save.data_mut().set_switch(&activation.id, new_on);
        // ECS switch state is mirrored from the save and indexed for the next frame.

        let target_id = if activation.target_encounter.is_empty() {
            active_area.clone()
        } else {
            activation.target_encounter.clone()
        };
        if !new_on {
            // Re-arming: Reset the encounter (the reducer refuses Start from a
            // terminal phase, so a stale Completed/Failed must clear); the
            // ownership-driven cleanup adapter (E10) drops carryover mobs off
            // the Reset event.
            if let Some((_, lifecycle, _, _)) =
                encounters.iter().find(|(enc, _, _, _)| enc.id == target_id)
            {
                if !lifecycle.phase.in_flight() {
                    lifecycle_commands.write(EncounterCommand::new(
                        &target_id,
                        EncounterCommandKind::Reset,
                    ));
                }
            }
            // Also drop any reward chest from a prior clear so the
            // next clear pays out fresh, and clear the persisted
            // "reward dropped" flag so re-clearing actually re-spawns
            // the chest. The orphaned `FeatureVisual` entity is
            // healed by `sync_visuals` on the next spawn (same id →
            // same entity, sprite restored from `chest_state_sprite`).
            crate::features::clear_encounter_reward_ecs(
                &mut commands,
                save.data_mut(),
                &reward_chests,
                &target_id,
            );
        }
    }
}

/// Wave EFFECT adapter (Progression, after the generic reducer): reacts to
/// this frame's lifecycle events and projects wave-encounter state onto its
/// consumers — switch auto-green + celebration + quest + mob cleanup on
/// completion, reward-chest sync, music request, presentation read-model,
/// save projection, and the trace sink for every encounter event.
pub fn apply_wave_encounter_effects(
    mut commands: SessionCommands<'_, '_>,
    mut events_in: MessageReader<EncounterEventMsg>,
    encounters: Query<(
        &Encounter,
        &EncounterLifecycle,
        Option<&EncounterWaves>,
        Option<&EncounterParticipants>,
    )>,
    mut save: ResMut<ambition_persistence::save::SandboxSave>,
    switch_index: Res<EncounterSwitchIndex>,
    mut trace: ResMut<crate::trace::GameplayTraceBuffer>,
    player_body_q: Query<&crate::actor::BodyKinematics, With<crate::actor::PlayerEntity>>,
    mut music_request: ambition_platformer_primitives::lifecycle::SessionWorldMut<
        EncounterMusicRequest,
    >,
    mut encounter_view: ResMut<EncounterView>,
    mut quests: ResMut<ambition_persistence::quest::QuestRegistry>,
    mut banner_requests: MessageWriter<crate::features::GameplayBannerRequested>,
    reward_chests: Query<
        (
            Entity,
            &crate::features::EncounterRewardChest,
            &crate::features::FeatureId,
            Option<&crate::features::Opened>,
        ),
        With<crate::features::ChestFeature>,
    >,
) {
    let Some(session_scope) = commands.spawn_scope() else {
        return;
    };
    // Trace sink first — every encounter event (generic reducer + wave
    // director) lands in the gameplay trace regardless of the player guard
    // below, in the same `encounter:<id>:<label>` format as before E8.
    let tick = trace.current_tick();
    let mut completed_wave_ids: Vec<String> = Vec::new();
    for msg in events_in.read() {
        trace.push_event(crate::trace::GameplayTraceEvent::Sfx {
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

    // Completion effects: auto-flip the linked switch to on (green) so the
    // player can see they finished it, surface a celebration banner, and
    // advance any "clear encounter" quest step. (Mob despawn moved to the
    // ownership-driven cleanup adapter, E10.)
    for encounter_id in &completed_wave_ids {
        if let Some(switch_id) = switch_index.switch_id_for_encounter(encounter_id) {
            save.data_mut().set_switch(&switch_id, true);
        }
        banner_requests.write(crate::features::GameplayBannerRequested::new(
            format!("ARENA CLEAR — {encounter_id}"),
            3.0,
        ));
        quests.push_event(
            ambition_persistence::quest::QuestAdvanceEvent::EncounterCleared(encounter_id.clone()),
        );
    }

    // Reward chest sync: gather the completed encounters' (id, spec) so the
    // reward sync stays decoupled from the encounter state representation.
    let cleared_specs: Vec<(String, super::EncounterSpec)> = encounters
        .iter()
        .filter(|(_, lifecycle, waves, _)| {
            matches!(
                lifecycle.phase,
                ambition_encounter::EncounterPhase::Completed
            ) && waves.is_some()
        })
        .filter_map(|(enc, _, waves, _)| waves.map(|w| (enc.id.clone(), w.spec.clone())))
        .collect();
    crate::features::sync_encounter_reward_chests_ecs(
        &mut commands,
        session_scope,
        save.data(),
        &cleared_specs,
        &reward_chests,
    );

    // Music: pick the first wave encounter currently in flight and request its
    // track (the base-priority source of the shared `EncounterMusicRequest`);
    // otherwise clear it. Writing the base source every frame — including
    // `None` — is safe: `desired_track()` ranks `priority_track` above
    // `base_track`, so this can't clobber a concurrent focused fight's music.
    let active_track = encounters.iter().find_map(|(_, lifecycle, waves, _)| {
        if lifecycle.phase.in_flight() {
            waves
                .map(|w| w.spec.music_track.clone())
                .filter(|t| !t.is_empty())
        } else {
            None
        }
    });
    music_request.base_track = active_track;

    // Publish the presentation read-model (§6): the camera zoom the active
    // encounters want. Cross-crate presentation reads `EncounterView`, not
    // the entities. `max`-based, so it is query-order-independent.
    encounter_view.camera_zoom = ambition_encounter::active_encounter_camera_zoom(
        encounters
            .iter()
            .filter_map(|(_, lifecycle, waves, _)| waves.map(|w| (lifecycle.phase, &w.spec))),
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
/// - **Adopted** participants are NEVER touched — they pre-existed the
///   orchestration (a boss survives its wrap retiring).
/// - **Spawned** participants despawn under the default
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
    encounter_mobs: Query<
        (Entity, &crate::features::FeatureId),
        With<crate::features::EncounterMob>,
    >,
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
        if matches!(policy.spawned, ambition_encounter::SpawnedCleanup::Keep) {
            continue;
        }
        participants.members.retain(|member| {
            if member.ownership != ambition_encounter::Ownership::Spawned {
                return true;
            }
            let entity = member.entity.or_else(|| {
                encounter_mobs
                    .iter()
                    .find(|(_, id)| id.as_str() == member.id)
                    .map(|(entity, _)| entity)
            });
            if let Some(entity) = entity {
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.despawn();
                }
            }
            false
        });
    }
}
