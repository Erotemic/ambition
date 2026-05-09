//! Reusable encounter / wave system.
//!
//! An "encounter" is a scripted sequence of mob waves with explicit
//! lock / unlock semantics: entering the trigger zone starts the
//! sequence, exits are sealed until all waves are defeated, the
//! player dies → reset / unlock, all enemies defeated → cleared and
//! exits unlock.
//!
//! This module owns the *system* side. The actual lab room and its
//! LDtk authoring live in `docs/mob_lab.md` plus the sandbox LDtk
//! file. Today the resource is initialized empty; a follow-up patch
//! adds the LDtk markers + the spawn pipeline.

use bevy::prelude::*;

use ambition_engine as ae;
use ambition_engine::AabbExt;
use ambition_engine::PersistedEncounterState;

use crate::ldtk_world::LdtkProject;

mod events;
mod registry;
mod spec;
mod state;

pub use events::EncounterEvent;
pub use registry::{EncounterController, EncounterRegistry, SwitchActivation};
pub use spec::{EncounterMobSpec, EncounterSpec, EncounterWaveSpec, LockWallSpec};
pub use state::{EncounterPhase, EncounterRun, EncounterState, ENCOUNTER_INTER_WAVE_DELAY_SECONDS};

/// Read all `EncounterTrigger` + `LockWall` markers in the active
/// LDtk project, build matching `EncounterSpec`s, and register them.
///
/// Runs once after startup (or after a hot reload). The mob_lab area
/// gets its waves from a hard-coded `mob_lab_wave_specs()` rather
/// than from LDtk EnemySpawn markers, so the spawn timeline (delays
/// between waves and within waves) lives in code where it's easier to
/// tune than in the LDtk JSON.
pub fn load_encounter_specs_from_ldtk(
    project: &LdtkProject,
    save: &ae::SandboxSaveData,
) -> Vec<(String, EncounterSpec, PersistedEncounterState)> {
    let mut out = Vec::new();
    for level in &project.levels {
        let area_id = level.active_area();
        let Some(layer) = level.ambition_layer() else {
            continue;
        };
        let Some(trigger) = layer
            .entity_instances
            .iter()
            .find(|e| e.identifier == "EncounterTrigger")
        else {
            continue;
        };
        let trigger_id = crate::ldtk_world::field_string(trigger, "id")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| area_id.clone());
        let camera_zoom = crate::ldtk_world::field_f32(trigger, "camera_zoom").unwrap_or(1.5);
        let trigger_min = [trigger.px[0] as f32, trigger.px[1] as f32];
        let trigger_size = [trigger.width as f32, trigger.height as f32];

        // Pick up the LockWall marker (one per area, optional).
        let lock_wall = layer
            .entity_instances
            .iter()
            .find(|e| e.identifier == "LockWall")
            .map(|e| LockWallSpec {
                min: [e.px[0] as f32, e.px[1] as f32],
                size: [e.width as f32, e.height as f32],
            });

        // Hard-coded waves for known encounters. Falls back to one
        // wave assembled from LDtk EnemySpawn markers for areas the
        // sandbox doesn't have a builder for yet.
        let waves = match trigger_id.as_str() {
            "mob_lab" => mob_lab_wave_specs(),
            _ => fallback_waves_from_enemy_spawns(layer),
        };

        let spec = EncounterSpec {
            id: trigger_id.clone(),
            waves,
            trigger_min,
            trigger_size,
            camera_zoom,
            lock_wall,
            intro_seconds: 2.5,
            // mob_lab is now driven by generated_music.rs: intro -> adaptive stem loops -> outro.
            music_track: if trigger_id == "mob_lab" {
                String::new()
            } else {
                "pulse_drift_voyage".into()
            },
        };
        let persisted = save.encounter(&trigger_id);
        out.push((trigger_id, spec, persisted));
    }
    out
}

/// Build the canonical mob-lab wave spec — the user-authored fight
/// sequence:
///
/// - Wave 1: 2 mid-tier enemies, one each side (no sandbag respawn).
/// - Wave 2: 2 goblins immediately + 1 big goblin after a few seconds
///   (delay-based sub-spawn — wave 2 doesn't clear until all three
///   are down).
/// - Wave 3: 2 big goblins.
///
/// Positions assume the mob_lab arena floor (y=608) and span from
/// the divider-jamb edge (~x=720) to the back wall (~x=1584). The
/// arena is roughly 850x600 of usable space.
pub fn mob_lab_wave_specs() -> Vec<EncounterWaveSpec> {
    // Active-area-local coords. The arena floor is y=608 and the
    // doorway opening is at x=480-704. The encounter trigger spans
    // x=920-1160, so wave mobs sit deeper still — past the trigger
    // so they're visible after the camera zooms out and so the
    // player has crossed into the arena before the wall slams.
    let left_x: f32 = 1180.0;
    let right_x: f32 = 1500.0;
    let floor_y: f32 = 580.0; // ~30 px above the floor (mob centered)
    let goblin_size = [22.0, 38.0];
    let big_size = [32.0, 56.0];
    vec![
        EncounterWaveSpec {
            label: "wave 1 — flank the doorway".into(),
            mobs: vec![
                EncounterMobSpec::new("medium_striker", [left_x, floor_y]).with_size(goblin_size),
                EncounterMobSpec::new("medium_striker", [right_x, floor_y]).with_size(goblin_size),
            ],
        },
        EncounterWaveSpec {
            label: "wave 2 — goblins + heavy".into(),
            mobs: vec![
                EncounterMobSpec::new("medium_striker", [left_x, floor_y]).with_size(goblin_size),
                EncounterMobSpec::new("medium_striker", [right_x, floor_y])
                    .with_size(goblin_size)
                    .with_delay(0.70),
                // Big goblin reinforcement on a timer, fires whether
                // or not the goblins are still up.
                EncounterMobSpec::new("large_brute", [(left_x + right_x) * 0.5, floor_y - 18.0])
                    .with_size(big_size)
                    .with_delay(2.60),
            ],
        },
        EncounterWaveSpec {
            label: "wave 3 — heavy duo".into(),
            mobs: vec![
                EncounterMobSpec::new("large_brute", [left_x, floor_y - 18.0]).with_size(big_size),
                EncounterMobSpec::new("large_brute", [right_x, floor_y - 18.0]).with_size(big_size),
            ],
        },
    ]
}

fn fallback_waves_from_enemy_spawns(
    layer: &crate::ldtk_world::LdtkLayerInstance,
) -> Vec<EncounterWaveSpec> {
    let mut wave_mobs = Vec::new();
    for entity in &layer.entity_instances {
        if entity.identifier != "EnemySpawn" {
            continue;
        }
        let kind = crate::ldtk_world::field_string(entity, "brain")
            .unwrap_or_else(|| "medium_striker".into());
        wave_mobs.push(EncounterMobSpec::new(
            kind,
            [
                entity.px[0] as f32 + entity.width as f32 * 0.5,
                entity.px[1] as f32 + entity.height as f32 * 0.5,
            ],
        ));
    }
    if wave_mobs.is_empty() {
        Vec::new()
    } else {
        vec![EncounterWaveSpec {
            label: "wave 1".into(),
            mobs: wave_mobs,
        }]
    }
}

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

/// Drive each registered encounter from the player's position +
/// switch activations. Routes `EncounterEvent`s through the trace
/// recorder, mirrors phase changes into the save resource, and
/// handles switch toggle commands.
///
/// Drop the encounter's reward chest (if any) and clear the persisted
/// "looted" flag so the next clear pays out a fresh chest. Called by
/// the switch-reset re-arming branch; pure helper so unit tests can
/// drive the cycle without a Bevy app.
pub fn clear_encounter_reward(
    features: &mut crate::features::FeatureRuntime,
    save: &mut ae::SandboxSaveData,
    encounter_id: &str,
) {
    features.despawn_encounter_chest(encounter_id);
    let reward_flag = format!("encounter_{encounter_id}_reward_dropped");
    save.set_flag(reward_flag, false);
}

/// Save-flag id used to remember whether the player has already opened
/// (looted) a given encounter's reward chest. Persists across
/// save/load so a re-spawned chest correctly reads as opened.
pub fn encounter_reward_looted_flag(encounter_id: &str) -> String {
    format!("encounter_{encounter_id}_reward_dropped")
}

/// Position the reward chest is spawned at, given an encounter spec.
/// Bottom edge of the chest snaps to the trigger AABB's `max.y` (the
/// lower edge in y-down world space, which the LDtk authoring puts
/// on the arena floor). Pulled out as a helper so the placement
/// formula has one home and tests can pin it.
pub fn encounter_reward_chest_pos(spec: &EncounterSpec, chest_size: ae::Vec2) -> ae::Vec2 {
    let trigger = spec.trigger_aabb();
    ae::Vec2::new(trigger.center().x, trigger.max.y - chest_size.y * 0.5)
}

/// Idempotent reward-chest sync. For every encounter currently in
/// `Cleared` state with a loaded spec, ensure a chest with the
/// canonical `encounter_chest_<id>` id is in `features.chests` at
/// the on-floor position, with `chest.opened` mirroring the
/// persisted "looted" flag.
///
/// Runs each tick; cheap because:
///   - `spawn_chest` short-circuits on duplicate id;
///   - the registry usually has at most a few encounters loaded.
///
/// Called from `update_encounters_from_world` so it runs in the
/// same frame as the `Cleared` event AND on every subsequent
/// frame including the first one after save+reload.
pub fn sync_encounter_reward_chests(
    features: &mut crate::features::FeatureRuntime,
    save: &ae::SandboxSaveData,
    registry: &EncounterRegistry,
) {
    let chest_size = ae::Vec2::new(28.0, 28.0);
    for (encounter_id, state) in registry.encounters.iter() {
        if !matches!(state.phase, EncounterPhase::Cleared) {
            continue;
        }
        let Some(spec) = state.spec.as_ref() else {
            continue;
        };
        let chest_id = format!("encounter_chest_{encounter_id}");
        let chest_pos = encounter_reward_chest_pos(spec, chest_size);
        // `spawn_chest` is idempotent on the id, so re-running per
        // frame is a hash-set check after the first spawn.
        features.spawn_chest(
            chest_id.clone(),
            Some(ae::PickupKind::Health { amount: 2 }),
            chest_pos,
            chest_size,
        );
        // Mirror the persisted "looted" flag onto the live chest.
        // Without this, save+reload would re-spawn the chest as
        // closed even after the player already looted it.
        let looted = save.flag(&encounter_reward_looted_flag(encounter_id));
        if let Some(chest) = features.chests.iter_mut().find(|c| c.id == chest_id) {
            chest.opened = looted;
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

/// Whether `encounter_id` is currently armed (will fire when the
/// player crosses the trigger). Looks up linked switches in the
/// runtime: a switch with `target_encounter == encounter_id` arms
/// the encounter when its `on` flag is false (red). Multiple linked
/// switches OR together (any one off → armed). No linked switches
/// means the encounter is always armed.
pub fn encounter_armed_by_switch(
    encounter_id: &str,
    switches: &[crate::features::SwitchRuntime],
) -> bool {
    let mut found = false;
    for sw in switches {
        let Some(act) = SwitchActivation::parse_custom(&sw.custom_payload) else {
            continue;
        };
        if act.target_encounter != encounter_id {
            continue;
        }
        found = true;
        if !sw.on {
            // Off (red) = armed.
            return true;
        }
    }
    !found
}

/// Find the switch id (LDtk `id` field, matching the persisted save's
/// `switches` key) that targets `encounter_id`, if any. Returns the
/// first match — multi-switch encounters can extend this later.
pub fn switch_id_for_encounter(
    encounter_id: &str,
    switches: &[crate::features::SwitchRuntime],
) -> Option<String> {
    for sw in switches {
        let Some(act) = SwitchActivation::parse_custom(&sw.custom_payload) else {
            continue;
        };
        if act.target_encounter == encounter_id {
            return Some(act.id);
        }
    }
    None
}

/// Insert / remove the encounter lock wall solid blocks based on
/// the live phase of each encounter. Block name format is
/// `lockwall:<encounter_id>` so the system can find and remove only
/// the blocks it owns.
fn sync_lock_walls(world: &mut ae::World, registry: &EncounterRegistry) {
    // Collect the desired (min, size) of each lock-wall block (one
    // per Starting/Active encounter that has an authored LockWall).
    let mut desired: std::collections::HashMap<String, (ae::Vec2, ae::Vec2)> =
        std::collections::HashMap::new();
    for (id, state) in &registry.encounters {
        if !matches!(
            state.phase,
            EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
        ) {
            continue;
        }
        let Some(spec) = state.spec.as_ref() else {
            continue;
        };
        let Some(wall) = spec.lock_wall.as_ref() else {
            continue;
        };
        desired.insert(
            id.clone(),
            (
                ae::Vec2::new(wall.min[0], wall.min[1]),
                ae::Vec2::new(wall.size[0], wall.size[1]),
            ),
        );
    }

    // Drop any present-but-unwanted lock walls.
    world.blocks.retain(|b| {
        if let Some(stripped) = b.name.strip_prefix("lockwall:") {
            desired.contains_key(stripped)
        } else {
            true
        }
    });

    // Insert any wanted-but-missing lock walls.
    for (id, (min, size)) in desired {
        let name = format!("lockwall:{id}");
        if !world.blocks.iter().any(|b| b.name == name) {
            world.blocks.push(ae::Block::solid(name, min, size));
        }
    }
}

/// Music request from the encounter system to the audio backend.
/// The encounter writes `desired_track` (Some(track_id) while an
/// encounter is in flight, None when default music should resume);
/// the audio-feature-gated `apply_encounter_music` system in
/// `audio.rs` swaps the music channel only when the desired track
/// changes.
#[derive(Resource, Default, Debug, Clone)]
pub struct EncounterMusicRequest {
    pub desired_track: Option<String>,
    /// The track id we last applied so we can detect transitions
    /// (None ↔ Some(other) ↔ Some(other2)).
    pub last_applied: Option<String>,
}

/// FIFO queue of switch activations produced by the feature runtime
/// each frame. The encounter system drains it and applies the
/// matching reset.
#[derive(Resource, Default)]
pub struct SwitchActivationQueue(pub Vec<SwitchActivation>);

#[cfg(test)]
mod tests;
