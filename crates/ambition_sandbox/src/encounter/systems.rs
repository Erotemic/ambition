use bevy::prelude::*;

use ambition_engine as ae;

use super::lock_walls::sync_lock_walls;
use super::{
    clear_encounter_reward, encounter_armed_by_switch, load_encounter_specs_from_ldtk,
    switch_id_for_encounter, sync_encounter_reward_chests, EncounterController, EncounterEvent,
    EncounterMusicRequest, EncounterPhase, EncounterRegistry, EncounterRun, SwitchActivationQueue,
};

/// Bevy startup system: load encounter specs from the embedded LDtk
/// project and apply persisted states from the save.
pub fn populate_encounter_registry(
    mut registry: ResMut<EncounterRegistry>,
    save: Res<crate::save::SandboxSave>,
    project: Res<crate::ldtk_world::SandboxLdtkProject>,
    mut commands: Commands,
) {
    if registry.specs_loaded {
        return;
    }
    let entries = load_encounter_specs_from_ldtk(&project.0, save.data());
    for (id, spec, persisted) in entries {
        let state = registry.ensure(&id);
        state.spec = Some(spec);
        state.apply_persisted(persisted);
        // One controller entity per encounter. The state component is
        // attached separately by `sync_encounter_controller_states` so
        // hot reload + spec changes can flip components without
        // respawning entities.
        commands.spawn((
            EncounterController {
                encounter_id: id.clone(),
            },
            Name::new(format!("EncounterController:{id}")),
        ));
    }
    registry.specs_loaded = true;
}

/// Mirror the registry's live `EncounterPhase` onto the matching
/// controller entity's seldom_state state component. Drops any other
/// encounter-state component first so phase changes are clean.
pub fn sync_encounter_controller_states(
    registry: Res<EncounterRegistry>,
    mut commands: Commands,
    controllers: Query<(Entity, &EncounterController)>,
) {
    if !registry.is_changed() {
        return;
    }
    for (entity, controller) in &controllers {
        let Some(state) = registry.get(&controller.encounter_id) else {
            continue;
        };
        let mut entity_commands = commands.entity(entity);
        entity_commands
            .remove::<ae::EncounterDormant>()
            .remove::<ae::EncounterStarting>()
            .remove::<ae::EncounterActive>()
            .remove::<ae::EncounterCleared>()
            .remove::<ae::EncounterFailed>();
        match state.phase {
            EncounterPhase::Inactive => {
                entity_commands.insert(ae::EncounterDormant);
            }
            EncounterPhase::Starting { remaining } => {
                entity_commands.insert(ae::EncounterStarting { remaining });
            }
            EncounterPhase::Active {
                wave_index,
                remaining_mobs,
            } => {
                let total_waves = state
                    .spec
                    .as_ref()
                    .map(|s| s.waves.len() as u8)
                    .unwrap_or(0);
                entity_commands.insert(ae::EncounterActive {
                    wave_index: wave_index as u8,
                    remaining_mobs: remaining_mobs as u8,
                    total_waves,
                });
            }
            EncounterPhase::Cleared => {
                entity_commands.insert(ae::EncounterCleared);
            }
            EncounterPhase::Failed => {
                entity_commands.insert(ae::EncounterFailed);
            }
        }
    }
}

/// Encounter cancellation: encounters that are `Active` only persist
/// while the player is in the matching active area. Walking out
/// (e.g. through the entry LoadingZone) resets the encounter to
/// `Inactive` so the camera zoom + lock release on exit. This is
/// deliberate sandbox UX — the encounter is "in play" only while the
/// player is actually inside the room.
pub fn update_encounters_from_world(
    time: Res<Time>,
    mut died_messages: MessageReader<crate::PlayerDiedMessage>,
    mut registry: ResMut<EncounterRegistry>,
    mut save: ResMut<crate::save::SandboxSave>,
    mut switch_activations: ResMut<SwitchActivationQueue>,
    mut trace: ResMut<crate::trace::GameplayTraceBuffer>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut world: ResMut<crate::GameWorld>,
    mut music_request: ResMut<EncounterMusicRequest>,
    mut quests: ResMut<crate::quest::QuestRegistry>,
    room_set: Res<crate::rooms::RoomSet>,
) {
    let active_area = room_set.active_spec().id.clone();
    let player_pos = runtime.player.pos;
    let player_size = runtime.player.size;
    let dt = time.delta_secs();
    let mut events: Vec<(String, Vec<EncounterEvent>)> = Vec::new();

    // 0. Player death this frame? Fail any in-flight encounter,
    //    drop the lock wall, and despawn carryover encounter mobs
    //    (`runtime.reset` already rebuilt FeatureRuntime, but the
    //    encounter alive_ids still reference the old ids — clearing
    //    them here makes the next tick a clean fresh attempt). The
    //    death-respawn path already moved the player back to the
    //    room spawn, so the trigger AABB will re-fire on next entry.
    let died_this_frame = died_messages.read().next().is_some();
    if died_this_frame {
        for (id, state) in registry.encounters.iter_mut() {
            let in_flight = matches!(
                state.phase,
                EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
            );
            if in_flight {
                let evs = state.on_player_death();
                if !evs.is_empty() {
                    events.push((id.clone(), evs));
                }
                // After failing, snap to Inactive so the trigger can
                // fire fresh once the player walks back in.
                state.phase = EncounterPhase::Inactive;
                state.lock_active = false;
                state.run = EncounterRun::default();
                runtime.features.despawn_encounter_enemies(id);
            }
        }
    }

    // 1. Cancel encounters whose area the player has left. Snaps back
    //    to Inactive so the camera zoom + lock release on exit. A
    //    fresh attempt will fire next time the player re-enters.
    for (id, state) in registry.encounters.iter_mut() {
        let in_flight = matches!(
            state.phase,
            EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
        );
        if in_flight && id != &active_area {
            state.phase = EncounterPhase::Inactive;
            state.lock_active = false;
            state.run = EncounterRun::default();
            events.push((
                id.clone(),
                vec![EncounterEvent::LockChanged { locked: false }],
            ));
        }
    }

    // 2. Trigger entry. The SWITCH is the source of truth for "armed":
    //    switch off = armed (red), switch on = disabled (green).
    //    Phase Cleared/Failed snap back to Inactive here so a stale
    //    persisted state doesn't lock out re-triggering after a
    //    switch toggle. The trigger only fires when the encounter
    //    isn't currently in flight AND the linked switch is off.
    let armed_active = encounter_armed_by_switch(&active_area, &runtime.features.switches);
    if let Some(state) = registry.encounters.get_mut(&active_area) {
        let in_flight = matches!(
            state.phase,
            EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
        );
        if !in_flight {
            // Snap any stale Cleared/Failed back to Inactive so the
            // trigger can fire on the next pass when the switch is
            // armed.
            if !matches!(state.phase, EncounterPhase::Inactive) {
                state.phase = EncounterPhase::Inactive;
                state.lock_active = false;
                state.run = EncounterRun::default();
            }
            if armed_active {
                let started = state.maybe_start(player_pos, player_size);
                if !started.is_empty() {
                    events.push((active_area.clone(), started));
                }
            }
        }
    }

    // 3. Tick the active-area encounter (intro countdown / wave
    //    progression / mob death tracking). Capture whether this
    //    tick produced a Cleared event so we can auto-flip the
    //    linked switch to green afterwards.
    let mut spawn_commands: Vec<(String, String, [f32; 2], [f32; 2])> = Vec::new();
    let mut just_cleared_id: Option<String> = None;
    if let Some(state) = registry.encounters.get_mut(&active_area) {
        if matches!(
            state.phase,
            EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
        ) {
            // Snapshot alive ids from the runtime BEFORE ticking. The
            // tick's `retain` runs before its spawn loop now, so the
            // freshly-spawned mobs in this tick aren't immediately
            // reaped (they'll be tested against the next frame's
            // snapshot).
            let alive_lookup: std::collections::HashSet<String> = runtime
                .features
                .enemies
                .iter()
                .filter(|e| e.alive)
                .map(|e| e.id.clone())
                .collect();
            let evs = state.tick_intro_or_wave(dt, |id| alive_lookup.contains(id));
            for ev in &evs {
                match ev {
                    EncounterEvent::SpawnCommand {
                        id,
                        kind,
                        pos,
                        size,
                    } => spawn_commands.push((id.clone(), kind.clone(), *pos, *size)),
                    EncounterEvent::Cleared { id } => {
                        just_cleared_id = Some(id.clone());
                    }
                    _ => {}
                }
            }
            if !evs.is_empty() {
                events.push((active_area.clone(), evs));
            }
        }
    }

    // 4. Apply spawn commands to FeatureRuntime.
    for (id, kind, pos, size) in spawn_commands {
        runtime.features.spawn_enemy(
            id,
            ae::EnemyBrain::Custom(kind),
            ae::Vec2::new(pos[0], pos[1]),
            ae::Vec2::new(size[0], size[1]),
        );
    }

    // 5. Auto-flip the linked switch to on (green) when the encounter
    //    just cleared. The script's last beat is "switch goes green"
    //    so the player can see they finished it. The encounter-mobs
    //    cleanup happens too so the world is clean for the next time
    //    they re-arm.
    if let Some(encounter_id) = just_cleared_id {
        if let Some(switch_id) = switch_id_for_encounter(&encounter_id, &runtime.features.switches)
        {
            save.data_mut().set_switch(&switch_id, true);
            runtime.features.set_switch_on(&switch_id, true);
        }
        runtime.features.despawn_encounter_enemies(&encounter_id);
        // Polish: surface a celebration banner so the player gets
        // explicit "you cleared it" feedback (not just an ambient
        // green switch).
        runtime.features.banner = format!("ARENA CLEAR — {encounter_id}");
        runtime.features.banner_timer = 3.0;
        // Quest hook: a "clear encounter" step can advance now.
        quests.push_event(ae::QuestAdvanceEvent::EncounterCleared(
            encounter_id.clone(),
        ));
    }

    // 5b. Reward chest sync: every Cleared encounter whose spec is
    //     loaded must have its reward chest live in `features.chests`.
    //     Runs idempotently each frame so save+reload (encounter
    //     loaded already in `Cleared`, features rebuilt empty) drops
    //     the chest just like the first clear does. `spawn_chest`
    //     itself is idempotent by id, so re-running is cheap.
    //
    //     The `encounter_<id>_reward_dropped` save flag now means
    //     "the chest was looted" (not "the chest was paid out"). The
    //     spawn step applies the flag onto `chest.opened` so a
    //     re-spawned chest shows its persisted looted state.
    sync_encounter_reward_chests(&mut runtime.features, save.data(), &registry);

    // 6. Switch toggles. Just toggle the persisted switch state; the
    //    trigger gate consults `switch.on` directly. When the player
    //    re-arms (toggles to off), also drop any encounter-spawned
    //    mobs from a prior attempt and snap any stale Cleared/Failed
    //    phase back to Inactive so the next trigger fires cleanly.
    let activations = std::mem::take(&mut switch_activations.0);
    for activation in activations {
        // Quest hook: every switch interaction sets a generic flag
        // that quests can listen for. Specific switches will key on
        // their own ids via `switch:<id>` flags.
        save.data_mut().set_flag("test_switch_toggled", true);
        save.data_mut()
            .set_flag(format!("switch_{}_used", activation.id), true);
        quests.push_event(ae::QuestAdvanceEvent::FlagSet("test_switch_toggled".into()));
        if !matches!(activation.action.as_str(), "ResetEncounter") {
            continue;
        }
        let new_on = !save.data().switch(&activation.id);
        save.data_mut().set_switch(&activation.id, new_on);
        runtime.features.set_switch_on(&activation.id, new_on);

        let target_id = if activation.target_encounter.is_empty() {
            active_area.clone()
        } else {
            activation.target_encounter.clone()
        };
        if !new_on {
            // Re-arming: snap the encounter back to Inactive and
            // drop carryover mobs.
            if let Some(state) = registry.encounters.get_mut(&target_id) {
                let in_flight = matches!(
                    state.phase,
                    EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
                );
                if !in_flight {
                    state.phase = EncounterPhase::Inactive;
                    state.lock_active = false;
                    state.run = EncounterRun::default();
                }
            }
            runtime.features.despawn_encounter_enemies(&target_id);
            // Also drop any reward chest from a prior clear so the
            // next clear pays out fresh, and clear the persisted
            // "reward dropped" flag so re-clearing actually re-spawns
            // the chest. The orphaned `FeatureVisual` entity is
            // healed by `sync_visuals` on the next spawn (same id →
            // same entity, sprite restored from `chest_state_sprite`).
            clear_encounter_reward(&mut runtime.features, save.data_mut(), &target_id);
        }
    }

    // 6. Mirror persisted switch state onto the runtime each frame
    //    (cheap; loop is bounded by switch count).
    let switch_states: Vec<(String, bool)> = save
        .data()
        .switches
        .iter()
        .map(|s| (s.id.clone(), s.on))
        .collect();
    for (id, on) in switch_states {
        runtime.features.set_switch_on(&id, on);
    }

    // 7. Lock-wall management: while any encounter is in Starting or
    //    Active, the lock wall block needs to be present in the
    //    GameWorld. When the encounter leaves those phases, pull it
    //    out. Identified by the block name `lockwall:<encounter_id>`.
    sync_lock_walls(&mut world.0, &registry);

    // 8. Music: pick the first encounter currently in flight and
    //    request its track; otherwise request the default. The
    //    audio-feature-gated `apply_encounter_music` system reacts.
    let active_track = registry.encounters.iter().find_map(|(_, s)| {
        if matches!(
            s.phase,
            EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
        ) {
            s.spec
                .as_ref()
                .map(|sp| sp.music_track.clone())
                .filter(|t| !t.is_empty())
        } else {
            None
        }
    });
    music_request.desired_track = active_track;

    // 9. Project phase to the save (Cleared/Failed survive, others
    //    collapse to Untouched).
    for (id, state) in registry.encounters.iter() {
        let persisted = state.to_persisted();
        let current = save.data().encounter(id);
        if persisted != current {
            save.data_mut().set_encounter(id, persisted);
        }
    }

    // 10. Push trace events.
    let tick = trace.current_tick();
    for (encounter_id, evs) in events {
        for ev in evs {
            trace.push_event(crate::trace::GameplayTraceEvent::Sfx {
                tick,
                label: format!("encounter:{encounter_id}:{}", ev.label()),
            });
        }
    }
}
