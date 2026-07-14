//! The Bevy wiring around the headless `state.rs` machine.
//! `populate_encounter_registry` (startup) loads specs from LDtk + the save and
//! spawns one encounter ENTITY per spec (E1 — the live state lives on the
//! entity's `EncounterState` component, not in a resource map);
//! `update_encounters_from_world` is the per-frame tick over those entities:
//! death/area-exit cancellation, switch-armed trigger entry, `tick_intro_or_wave`,
//! applying `SpawnCommand`s to ECS mobs, auto-greening the cleared switch +
//! reward chest, music request, presentation read-model, save projection, and
//! trace push.

use bevy::prelude::*;

use ambition_engine_core as ae;
use ambition_platformer_primitives::lifecycle::SessionCommands;

use super::{
    load_encounter_specs_from_ldtk, Encounter, EncounterEvent, EncounterMusicRequest,
    EncounterParticipants, EncounterPhase, EncounterRegistry, EncounterRun, EncounterState,
    EncounterSwitchIndex, EncounterView, SwitchActivationQueue,
};

/// Bevy startup system: load encounter specs from the embedded LDtk
/// project, spawn one encounter entity per spec, and apply persisted states
/// from the save.
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
        let mut state = EncounterState {
            spec: Some(spec),
            ..Default::default()
        };
        state.apply_persisted(persisted);
        let entity = commands
            .spawn((
                Encounter::new(id.clone()),
                state,
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

/// Encounter cancellation: encounters that are `Active` only persist
/// while the player is in the matching active area. Walking out
/// (e.g. through the entry LoadingZone) resets the encounter to
/// `Inactive` so the camera zoom + lock release on exit. This is
/// deliberate sandbox UX — the encounter is "in play" only while the
/// player is actually inside the room.
pub fn update_encounters_from_world(
    mut commands: SessionCommands<'_, '_>,
    world_time: Res<ambition_time::WorldTime>,
    mut died_messages: MessageReader<crate::ActorDiedMessage>,
    mut encounters: Query<(&Encounter, &mut EncounterState, &mut EncounterParticipants)>,
    mut save: ResMut<ambition_persistence::save::SandboxSave>,
    mut switch_activations: ResMut<SwitchActivationQueue>,
    switch_index: Res<EncounterSwitchIndex>,
    mut trace: ResMut<crate::trace::GameplayTraceBuffer>,
    player_body_q: Query<&crate::actor::BodyKinematics, With<crate::actor::PlayerEntity>>,
    mut music_request: ambition_platformer_primitives::lifecycle::SessionWorldMut<EncounterMusicRequest>,
    mut encounter_view: ResMut<EncounterView>,
    mut quests: ResMut<ambition_persistence::quest::QuestRegistry>,
    mut banner_requests: MessageWriter<crate::features::GameplayBannerRequested>,
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
    let mut events: Vec<(String, Vec<EncounterEvent>)> = Vec::new();

    // 0. Player death this frame? Fail any in-flight encounter,
    //    drop the lock wall, and despawn carryover encounter mobs
    //    (the player reset already rebuilt room-local state, but the
    //    encounter's live Minion participants still reference the old
    //    mobs — clearing them here makes the next tick a clean fresh
    //    attempt). The death-respawn path already moved the player back
    //    to the room spawn, so the trigger AABB will re-fire on entry.
    let died_this_frame = died_messages.read().next().is_some();
    if died_this_frame {
        for (enc, mut state, mut participants) in &mut encounters {
            let in_flight = matches!(
                state.phase,
                EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
            );
            if in_flight {
                let evs = state.on_player_death(&mut participants);
                if !evs.is_empty() {
                    events.push((enc.id.clone(), evs));
                }
                // After failing, snap to Inactive so the trigger can
                // fire fresh once the player walks back in.
                state.phase = EncounterPhase::Inactive;
                state.lock_active = false;
                state.run = EncounterRun::default();
                participants.members.clear();
                crate::features::despawn_encounter_mobs(&mut commands, &encounter_mobs, &enc.id);
            }
        }
    }

    // 1. Cancel encounters whose area the player has left. Snaps back
    //    to Inactive so the camera zoom + lock release on exit. A
    //    fresh attempt will fire next time the player re-enters.
    for (enc, mut state, mut participants) in &mut encounters {
        let in_flight = matches!(
            state.phase,
            EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
        );
        if in_flight && enc.id != active_area {
            state.phase = EncounterPhase::Inactive;
            state.lock_active = false;
            state.run = EncounterRun::default();
            participants.members.clear();
            events.push((
                enc.id.clone(),
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
    let armed_active = switch_index.encounter_armed(&active_area);
    if let Some((_, mut state, mut participants)) = encounters
        .iter_mut()
        .find(|(enc, _, _)| enc.id == active_area)
    {
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
                participants.members.clear();
            }
            if armed_active {
                // Iterate every player so any player walking into
                // the trigger fires the encounter — single-player
                // behavior preserved because the iterator has one
                // entity today. OVERNIGHT-TODO #17.8 (iterate-all-
                // players "any player triggers" pattern).
                for body in &player_body_q {
                    let started = state.maybe_start(&mut participants, body.pos, body.size);
                    if !started.is_empty() {
                        events.push((active_area.clone(), started));
                        break;
                    }
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
    if let Some((_, mut state, mut participants)) = encounters
        .iter_mut()
        .find(|(enc, _, _)| enc.id == active_area)
    {
        if matches!(
            state.phase,
            EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
        ) {
            // Snapshot alive ids from the runtime BEFORE ticking, and refresh
            // each live `Minion` participant's `alive` from it. The reducer's
            // `retain` runs before its spawn loop, and this refresh runs before
            // the reducer, so the mobs spawned in THIS tick (added after the
            // refresh) aren't immediately reaped — they're tested against the
            // next frame's snapshot.
            let alive_lookup: std::collections::HashSet<String> = encounter_mobs
                .iter()
                .filter(|(_, mob, _, combat)| mob.encounter_id == active_area && combat.alive)
                .map(|(_, _, id, _)| id.as_str().to_string())
                .collect();
            for member in &mut participants.members {
                member.alive = alive_lookup.contains(&member.id);
            }
            let evs = state.tick_intro_or_wave(dt, &mut participants);
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

    // 5. Auto-flip the linked switch to on (green) when the encounter
    //    just cleared. The script's last beat is "switch goes green"
    //    so the player can see they finished it. The encounter-mobs
    //    cleanup happens too so the world is clean for the next time
    //    they re-arm.
    if let Some(encounter_id) = just_cleared_id {
        if let Some(switch_id) = switch_index.switch_id_for_encounter(&encounter_id) {
            save.data_mut().set_switch(&switch_id, true);
        }
        crate::features::despawn_encounter_mobs(&mut commands, &encounter_mobs, &encounter_id);
        // Polish: surface a celebration banner so the player gets
        // explicit "you cleared it" feedback (not just an ambient
        // green switch).
        banner_requests.write(crate::features::GameplayBannerRequested::new(
            format!("ARENA CLEAR — {encounter_id}"),
            3.0,
        ));
        // Quest hook: a "clear encounter" step can advance now.
        quests.push_event(
            ambition_persistence::quest::QuestAdvanceEvent::EncounterCleared(encounter_id.clone()),
        );
    }

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
        // Toggle the persisted switch state so the switch sprite reads flipped.
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
            // Re-arming: snap the encounter back to Inactive and
            // drop carryover mobs.
            if let Some((_, mut state, mut participants)) = encounters
                .iter_mut()
                .find(|(enc, _, _)| enc.id == target_id)
            {
                let in_flight = matches!(
                    state.phase,
                    EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
                );
                if !in_flight {
                    state.phase = EncounterPhase::Inactive;
                    state.lock_active = false;
                    state.run = EncounterRun::default();
                    participants.members.clear();
                }
            }
            crate::features::despawn_encounter_mobs(&mut commands, &encounter_mobs, &target_id);
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

    // 6b. Reward chest sync runs after switch resets so a re-arm in this
    //     same tick cannot spawn a deferred ECS chest that the clear path
    //     cannot see yet. Gather the cleared encounters' (id, spec) so the
    //     reward sync stays decoupled from the encounter state representation.
    let cleared_specs: Vec<(String, super::EncounterSpec)> = encounters
        .iter()
        .filter(|(_, state, _)| matches!(state.phase, EncounterPhase::Cleared))
        .filter_map(|(enc, state, _)| state.spec.clone().map(|spec| (enc.id.clone(), spec)))
        .collect();
    crate::features::sync_encounter_reward_chests_ecs(
        &mut commands,
        session_scope,
        save.data(),
        &cleared_specs,
        &reward_chests,
    );

    // 7. Lock-wall management: while any encounter is in Starting or Active, a
    //    solid lock wall seals the arena exits. The wall is NOT mutated into the
    //    authored base here — `contribute_encounter_lock_walls` (WorldPrep)
    //    derives it onto the collision overlay's `gate_solids` each frame from
    //    the encounter entities' live phase this tick just updated. Keeps
    //    `RoomGeometry` authored-immutable mid-room.

    // 8. Music: pick the first encounter currently in flight and request its
    //    track (the base-priority source of the shared `EncounterMusicRequest`);
    //    otherwise clear it. Writing the base source every frame — including
    //    `None` — is safe: `desired_track()` ranks `priority_track` above
    //    `base_track`, so this can't clobber a concurrent focused fight's music.
    let active_track = encounters.iter().find_map(|(_, s, _)| {
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
    music_request.base_track = active_track;

    // 8b. Publish the presentation read-model (§6): the camera zoom the active
    //     encounters want. Cross-crate presentation reads `EncounterView`, not
    //     the entities. `max`-based, so it is query-order-independent.
    encounter_view.camera_zoom =
        ambition_encounter::active_encounter_camera_zoom(encounters.iter().map(|(_, s, _)| s));

    // 9. Project phase to the save (Cleared/Failed survive, others
    //    collapse to Untouched).
    for (enc, state, _) in &encounters {
        let persisted = state.to_persisted();
        let current = save.data().encounter(&enc.id);
        if persisted != current {
            save.data_mut().set_encounter(&enc.id, persisted);
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
