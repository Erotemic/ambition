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

use std::collections::BTreeMap;

use bevy::math::bounding::IntersectsVolume;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use ambition_engine as ae;
use ambition_engine::PersistedEncounterState;

use crate::ldtk_world::LdtkProject;

/// One mob to spawn during a wave. Position is local to the
/// encounter room (LDtk authoring will translate marker → spec).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncounterMobSpec {
    /// Display label for HUD / trace.
    pub kind: String,
    /// Spawn position in active-area-local coordinates.
    pub spawn: [f32; 2],
}

/// One wave of mobs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncounterWaveSpec {
    pub label: String,
    pub mobs: Vec<EncounterMobSpec>,
}

/// Whole encounter authored data: ordered list of waves plus the
/// activation AABB and the camera-zoom factor to apply while the
/// encounter is active.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncounterSpec {
    pub id: String,
    pub waves: Vec<EncounterWaveSpec>,
    /// AABB in active-area-local coordinates that triggers the
    /// encounter when the player enters.
    pub trigger_min: [f32; 2],
    pub trigger_size: [f32; 2],
    /// Camera zoom multiplier while the encounter is active. `1.0`
    /// disables the zoom-out.
    pub camera_zoom: f32,
}

impl EncounterSpec {
    pub fn trigger_aabb(&self) -> ae::Aabb {
        ae::aabb_from_min_size(
            ae::Vec2::new(self.trigger_min[0], self.trigger_min[1]),
            ae::Vec2::new(self.trigger_size[0], self.trigger_size[1]),
        )
    }
}

/// Live encounter phase the sandbox can render / lock against.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EncounterPhase {
    #[default]
    Inactive,
    /// Waves are spawning / being fought.
    Active {
        wave_index: usize,
        remaining_mobs: usize,
    },
    /// All waves cleared. Lock is released.
    Cleared,
    /// Player died inside the encounter. Reset path will return
    /// the encounter to `Inactive`.
    Failed,
}

impl EncounterPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::Active { .. } => "active",
            Self::Cleared => "cleared",
            Self::Failed => "failed",
        }
    }

    pub fn locks_exits(self) -> bool {
        matches!(self, Self::Active { .. })
    }

    pub fn wave_index(self) -> Option<usize> {
        match self {
            Self::Active { wave_index, .. } => Some(wave_index),
            _ => None,
        }
    }

    pub fn remaining_mobs(self) -> Option<usize> {
        match self {
            Self::Active { remaining_mobs, .. } => Some(remaining_mobs),
            _ => None,
        }
    }
}

/// Bevy resource holding the live encounter state plus the authored
/// spec it's tracking. Populated by the LDtk encounter loader when
/// `EncounterTrigger` markers land in an active area.
#[derive(Resource, Default)]
pub struct EncounterState {
    pub spec: Option<EncounterSpec>,
    pub phase: EncounterPhase,
    /// True when the encounter's lock should seal exits this frame.
    /// Cached so multiple consumers don't have to call `phase.locks_exits`.
    pub lock_active: bool,
}

impl EncounterState {
    /// Reconstruct the live phase from a `PersistedEncounterState`.
    /// Called after a save load so the encounter starts in its
    /// persisted terminal state instead of `Inactive`.
    pub fn apply_persisted(&mut self, persisted: PersistedEncounterState) {
        self.phase = match persisted {
            PersistedEncounterState::Untouched => EncounterPhase::Inactive,
            PersistedEncounterState::Cleared => EncounterPhase::Cleared,
            PersistedEncounterState::Failed => EncounterPhase::Failed,
        };
        self.lock_active = self.phase.locks_exits();
    }

    /// Project the live phase onto the persisted shape. `Active`
    /// collapses to `Untouched` because the save represents a
    /// resumable terminal state, not a mid-fight snapshot.
    pub fn to_persisted(&self) -> PersistedEncounterState {
        match self.phase {
            EncounterPhase::Inactive | EncounterPhase::Active { .. } => {
                PersistedEncounterState::Untouched
            }
            EncounterPhase::Cleared => PersistedEncounterState::Cleared,
            EncounterPhase::Failed => PersistedEncounterState::Failed,
        }
    }
}

impl EncounterState {
    /// Try to start the encounter when the player enters the
    /// trigger AABB. Returns the trace events the caller should
    /// push to the gameplay trace. No-op if the encounter is
    /// already active or no spec is loaded.
    pub fn maybe_start(
        &mut self,
        player_pos: ae::Vec2,
        player_size: ae::Vec2,
    ) -> Vec<EncounterEvent> {
        let mut events = Vec::new();
        let Some(spec) = self.spec.clone() else {
            return events;
        };
        if !matches!(self.phase, EncounterPhase::Inactive) {
            return events;
        }
        let player_aabb = ae::aabb_from_min_size(
            ae::Vec2::new(
                player_pos.x - player_size.x * 0.5,
                player_pos.y - player_size.y * 0.5,
            ),
            player_size,
        );
        let trigger = spec.trigger_aabb();
        if !trigger.intersects(&player_aabb) {
            return events;
        }
        let first = spec.waves.first().cloned();
        if let Some(wave) = first {
            self.phase = EncounterPhase::Active {
                wave_index: 0,
                remaining_mobs: wave.mobs.len(),
            };
            self.lock_active = true;
            events.push(EncounterEvent::Started {
                id: spec.id.clone(),
            });
            events.push(EncounterEvent::WaveStarted {
                wave_index: 0,
                label: wave.label.clone(),
            });
            for mob in &wave.mobs {
                events.push(EncounterEvent::EnemySpawned {
                    kind: mob.kind.clone(),
                });
            }
            events.push(EncounterEvent::LockChanged { locked: true });
        } else {
            // No waves authored → cleared immediately.
            self.phase = EncounterPhase::Cleared;
            self.lock_active = false;
            events.push(EncounterEvent::Started {
                id: spec.id.clone(),
            });
            events.push(EncounterEvent::Cleared { id: spec.id });
            events.push(EncounterEvent::LockChanged { locked: false });
        }
        events
    }

    /// Resolve a mob defeat. Advances the wave / clears the
    /// encounter if all mobs in the active wave are down.
    pub fn on_mob_defeated(&mut self) -> Vec<EncounterEvent> {
        let mut events = Vec::new();
        let Some(spec) = self.spec.clone() else {
            return events;
        };
        let EncounterPhase::Active {
            wave_index,
            remaining_mobs,
        } = self.phase
        else {
            return events;
        };
        let next_remaining = remaining_mobs.saturating_sub(1);
        if next_remaining > 0 {
            self.phase = EncounterPhase::Active {
                wave_index,
                remaining_mobs: next_remaining,
            };
            return events;
        }
        // Wave cleared — advance.
        let next_wave = wave_index + 1;
        if let Some(wave) = spec.waves.get(next_wave) {
            self.phase = EncounterPhase::Active {
                wave_index: next_wave,
                remaining_mobs: wave.mobs.len(),
            };
            events.push(EncounterEvent::WaveStarted {
                wave_index: next_wave,
                label: wave.label.clone(),
            });
            for mob in &wave.mobs {
                events.push(EncounterEvent::EnemySpawned {
                    kind: mob.kind.clone(),
                });
            }
        } else {
            self.phase = EncounterPhase::Cleared;
            self.lock_active = false;
            events.push(EncounterEvent::Cleared { id: spec.id });
            events.push(EncounterEvent::LockChanged { locked: false });
        }
        events
    }

    /// Player died — reset the encounter to inactive + unlock so a
    /// fresh attempt can begin.
    pub fn on_player_death(&mut self) -> Vec<EncounterEvent> {
        let mut events = Vec::new();
        let id = self.spec.as_ref().map(|s| s.id.clone());
        if matches!(self.phase, EncounterPhase::Active { .. }) {
            self.phase = EncounterPhase::Failed;
            self.lock_active = false;
            if let Some(id) = id {
                events.push(EncounterEvent::Failed { id });
            }
            events.push(EncounterEvent::LockChanged { locked: false });
        }
        events
    }

    /// Return to a fresh attempt — called by the sandbox after
    /// the player respawns.
    pub fn reset_for_retry(&mut self) {
        if matches!(self.phase, EncounterPhase::Failed | EncounterPhase::Cleared) {
            self.phase = EncounterPhase::Inactive;
            self.lock_active = false;
        }
    }

    pub fn hud_summary(&self) -> String {
        match self.phase {
            EncounterPhase::Inactive => "encounter inactive".into(),
            EncounterPhase::Active {
                wave_index,
                remaining_mobs,
            } => {
                let total = self.spec.as_ref().map(|s| s.waves.len()).unwrap_or(0);
                format!(
                    "encounter wave {}/{}  remaining {}",
                    wave_index + 1,
                    total,
                    remaining_mobs
                )
            }
            EncounterPhase::Cleared => "encounter cleared".into(),
            EncounterPhase::Failed => "encounter failed".into(),
        }
    }
}

/// Trace events emitted by the encounter state machine. The sandbox
/// projects these into `GameplayTraceEvent` (today the projectile
/// path uses a typed Projectile variant; encounters can extend the
/// trace enum the same way when wiring lands).
#[derive(Clone, Debug, PartialEq)]
pub enum EncounterEvent {
    Started { id: String },
    WaveStarted { wave_index: usize, label: String },
    EnemySpawned { kind: String },
    Cleared { id: String },
    Failed { id: String },
    LockChanged { locked: bool },
}

impl EncounterEvent {
    pub fn label(&self) -> String {
        match self {
            Self::Started { id } => format!("encounter_started:{id}"),
            Self::WaveStarted { wave_index, label } => {
                format!("encounter_wave_started:{wave_index}:{label}")
            }
            Self::EnemySpawned { kind } => format!("encounter_enemy_spawned:{kind}"),
            Self::Cleared { id } => format!("encounter_cleared:{id}"),
            Self::Failed { id } => format!("encounter_failed:{id}"),
            Self::LockChanged { locked } => format!("encounter_lock_changed:{locked}"),
        }
    }
}

// ─── Registry + LDtk loader + Bevy systems ──────────────────────────────

/// Multi-encounter registry. Keyed by encounter id (matching the
/// `EncounterTrigger.id` field in LDtk). Replaces the older
/// single-encounter `Res<EncounterState>` so the sandbox can carry
/// more than one encounter at once.
#[derive(Resource, Default)]
pub struct EncounterRegistry {
    pub encounters: BTreeMap<String, EncounterState>,
    /// Tracks whether the current LDtk file has been scanned for
    /// encounter triggers yet. Reset by hot reload so an edited LDtk
    /// re-populates the specs.
    pub specs_loaded: bool,
}

impl EncounterRegistry {
    pub fn get(&self, id: &str) -> Option<&EncounterState> {
        self.encounters.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut EncounterState> {
        self.encounters.get_mut(id)
    }

    pub fn ensure(&mut self, id: &str) -> &mut EncounterState {
        self.encounters.entry(id.to_string()).or_default()
    }

    /// True if any encounter is currently locking exits.
    pub fn any_lock_active(&self) -> bool {
        self.encounters.values().any(|e| e.lock_active)
    }

    /// Camera zoom multiplier sourced from the active encounter (if
    /// any). 1.0 if no encounter is active.
    pub fn active_camera_zoom(&self) -> f32 {
        for state in self.encounters.values() {
            if matches!(state.phase, EncounterPhase::Active { .. }) {
                if let Some(spec) = &state.spec {
                    if spec.camera_zoom > 1.0 {
                        return spec.camera_zoom;
                    }
                }
            }
        }
        1.0
    }
}

/// One activation request from a switch interaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchActivation {
    pub id: String,
    pub action: String,
    pub target_encounter: String,
}

impl SwitchActivation {
    /// Parse the `Custom("switch:<id>:<action>:<target>")` payload
    /// produced by `entity_to_runtime` for `Switch` LDtk entities.
    pub fn parse_custom(payload: &str) -> Option<Self> {
        let mut parts = payload.split(':');
        if parts.next()? != "switch" {
            return None;
        }
        let id = parts.next()?.to_string();
        let action = parts.next()?.to_string();
        let target_encounter = parts.next().unwrap_or("").to_string();
        Some(Self {
            id,
            action,
            target_encounter,
        })
    }
}

/// Marker component for the per-encounter seldom_state controller
/// entity. The encounter system spawns one per registered encounter
/// and keeps its sparse-set state component (`EncounterDormant`,
/// `EncounterActive`, `EncounterCleared`, `EncounterFailed`) in sync
/// with the registry's phase. HUD / debug systems can query by state
/// component without touching the resource.
#[derive(Component, Clone, Debug)]
pub struct EncounterController {
    pub encounter_id: String,
}

/// Read all `EncounterTrigger` markers in the active LDtk project,
/// build matching `EncounterSpec`s, and register them.
///
/// Runs once after startup (or after a hot reload). Idempotent: it
/// only adds encounters that aren't already registered, so manual
/// registrations from tests or future story crates aren't clobbered.
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
        // Find the encounter trigger first (only one per area for now).
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
        // Encounter trigger position in level-local coords. Encounter
        // logic uses active-area-local coords so add the level's
        // offset within its activeArea origin (we approximate with
        // the level's worldX/Y for now — the sandbox keeps both in
        // the same frame for single-level areas like mob_lab).
        let trigger_min = [trigger.px[0] as f32, trigger.px[1] as f32];
        let trigger_size = [trigger.width as f32, trigger.height as f32];

        // Collect EnemySpawn entities in the same level into one
        // wave; future patches can extend with per-spawn `wave` ints.
        let mut wave_mobs = Vec::new();
        for entity in &layer.entity_instances {
            if entity.identifier != "EnemySpawn" {
                continue;
            }
            let kind = crate::ldtk_world::field_string(entity, "brain")
                .unwrap_or_else(|| "sandbag_finite".into());
            wave_mobs.push(EncounterMobSpec {
                kind,
                spawn: [
                    entity.px[0] as f32 + entity.width as f32 * 0.5,
                    entity.px[1] as f32 + entity.height as f32 * 0.5,
                ],
            });
        }
        let waves = if wave_mobs.is_empty() {
            Vec::new()
        } else {
            vec![EncounterWaveSpec {
                label: "wave 1".into(),
                mobs: wave_mobs,
            }]
        };

        let spec = EncounterSpec {
            id: trigger_id.clone(),
            waves,
            trigger_min,
            trigger_size,
            camera_zoom,
        };
        let persisted = save.encounter(&trigger_id);
        out.push((trigger_id, spec, persisted));
    }
    out
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
/// Encounter cancellation: encounters that are `Active` only persist
/// while the player is in the matching active area. Walking out
/// (e.g. through the entry LoadingZone) resets the encounter to
/// `Inactive` so the camera zoom + lock release on exit. This is
/// deliberate sandbox UX — the encounter is "in play" only while the
/// player is actually inside the room.
pub fn update_encounters_from_world(
    mut registry: ResMut<EncounterRegistry>,
    mut save: ResMut<crate::save::SandboxSave>,
    mut switch_activations: ResMut<SwitchActivationQueue>,
    mut trace: ResMut<crate::trace::GameplayTraceBuffer>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    room_set: Res<crate::rooms::RoomSet>,
) {
    let active_area = room_set.active_spec().id.clone();
    let player_pos = runtime.player.pos;
    let player_size = runtime.player.size;
    let mut events: Vec<(String, Vec<EncounterEvent>)> = Vec::new();

    // Cancel `Active` encounters whose area the player has left. The
    // camera zoom + lock can't keep enforcing themselves outside the
    // room, so the encounter snaps back to Inactive (a fresh attempt
    // is available next time the player re-enters the trigger).
    for (id, state) in registry.encounters.iter_mut() {
        if matches!(state.phase, EncounterPhase::Active { .. }) && id != &active_area {
            state.phase = EncounterPhase::Inactive;
            state.lock_active = false;
            events.push((id.clone(), vec![EncounterEvent::LockChanged { locked: false }]));
        }
    }

    // Trigger entry: only the active area's encounter can fire, and
    // only when its persisted state isn't Cleared (a Cleared encounter
    // is "off" — the player can re-arm it via the switch).
    if let Some(state) = registry.encounters.get_mut(&active_area) {
        if matches!(state.phase, EncounterPhase::Inactive) {
            let started = state.maybe_start(player_pos, player_size);
            if !started.is_empty() {
                events.push((active_area.clone(), started));
            }
        }
    }

    // Switch toggles. Pressing the switch flips between Cleared (green
    // / encounter disabled) and Inactive (red / encounter armed). The
    // sandbox makes this a free toggle — pressing the switch never
    // requires beating the encounter first. Future story crates can
    // gate on the persisted encounter state if they want one-way
    // semantics.
    let activations = std::mem::take(&mut switch_activations.0);
    for activation in activations {
        let target_id = if activation.target_encounter.is_empty() {
            active_area.clone()
        } else {
            activation.target_encounter.clone()
        };
        let Some(state) = registry.encounters.get_mut(&target_id) else {
            continue;
        };
        if matches!(activation.action.as_str(), "ResetEncounter") {
            // Toggle: Cleared ↔ Inactive. Active / Failed both fold
            // into Inactive on press (ResetEncounter is a "make this
            // encounter armed again" verb).
            let now_cleared = matches!(state.phase, EncounterPhase::Cleared);
            state.phase = if now_cleared {
                EncounterPhase::Inactive
            } else {
                EncounterPhase::Cleared
            };
            state.lock_active = false;
            // Persisted switch reflects the encounter state so the
            // visual color reads from the save unambiguously after a
            // reload: switch on (green) ⇔ encounter Cleared.
            let target_on = matches!(state.phase, EncounterPhase::Cleared);
            save.data_mut().set_switch(&activation.id, target_on);
            // Update the runtime switch's color immediately so the
            // sprite tint flips on the same frame instead of waiting
            // for the save → render path.
            runtime.features.set_switch_on(&activation.id, target_on);
        }
    }

    // Sync runtime switch colors with the persisted save state every
    // frame (cheap; the loop is bounded by switch count). Covers the
    // case where the save was loaded at startup and the runtime
    // hasn't been told yet.
    let switch_states: Vec<(String, bool)> = save
        .data()
        .switches
        .iter()
        .map(|s| (s.id.clone(), s.on))
        .collect();
    for (id, on) in switch_states {
        runtime.features.set_switch_on(&id, on);
    }

    // Project current phases to the save (Cleared/Failed survive,
    // Inactive collapses to Untouched, Active doesn't write yet).
    for (id, state) in registry.encounters.iter() {
        let persisted = state.to_persisted();
        let current = save.data().encounter(id);
        if persisted != current {
            save.data_mut().set_encounter(id, persisted);
        }
    }

    // Push trace events.
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

/// FIFO queue of switch activations produced by the feature runtime
/// each frame. The encounter system drains it and applies the
/// matching reset.
#[derive(Resource, Default)]
pub struct SwitchActivationQueue(pub Vec<SwitchActivation>);

#[cfg(test)]
mod tests {
    use super::*;

    fn lab_spec() -> EncounterSpec {
        EncounterSpec {
            id: "mob_lab".into(),
            waves: vec![
                EncounterWaveSpec {
                    label: "wave 1".into(),
                    mobs: vec![EncounterMobSpec {
                        kind: "dummy".into(),
                        spawn: [100.0, 100.0],
                    }],
                },
                EncounterWaveSpec {
                    label: "wave 2".into(),
                    mobs: vec![
                        EncounterMobSpec {
                            kind: "dummy".into(),
                            spawn: [120.0, 100.0],
                        },
                        EncounterMobSpec {
                            kind: "dummy".into(),
                            spawn: [180.0, 100.0],
                        },
                    ],
                },
            ],
            trigger_min: [0.0, 0.0],
            trigger_size: [400.0, 200.0],
            camera_zoom: 1.5,
        }
    }

    #[test]
    fn entering_trigger_starts_first_wave() {
        let mut state = EncounterState::default();
        state.spec = Some(lab_spec());
        let events = state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        assert!(state.lock_active);
        assert_eq!(
            state.phase,
            EncounterPhase::Active {
                wave_index: 0,
                remaining_mobs: 1,
            }
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, EncounterEvent::Started { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, EncounterEvent::LockChanged { locked: true })));
    }

    #[test]
    fn standing_outside_trigger_does_not_start() {
        let mut state = EncounterState::default();
        state.spec = Some(lab_spec());
        let events = state.maybe_start(ae::Vec2::new(2000.0, 50.0), ae::Vec2::new(20.0, 30.0));
        assert!(events.is_empty());
        assert_eq!(state.phase, EncounterPhase::Inactive);
        assert!(!state.lock_active);
    }

    #[test]
    fn defeating_all_mobs_clears_each_wave_and_then_encounter() {
        let mut state = EncounterState::default();
        state.spec = Some(lab_spec());
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        // Wave 1 has 1 mob; defeat it.
        let _ = state.on_mob_defeated();
        // Wave 2 has 2 mobs; defeat both.
        let _ = state.on_mob_defeated();
        let events = state.on_mob_defeated();
        assert_eq!(state.phase, EncounterPhase::Cleared);
        assert!(!state.lock_active);
        assert!(events
            .iter()
            .any(|e| matches!(e, EncounterEvent::Cleared { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, EncounterEvent::LockChanged { locked: false })));
    }

    #[test]
    fn player_death_during_active_encounter_unlocks_and_marks_failed() {
        let mut state = EncounterState::default();
        state.spec = Some(lab_spec());
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        let events = state.on_player_death();
        assert_eq!(state.phase, EncounterPhase::Failed);
        assert!(!state.lock_active);
        assert!(events
            .iter()
            .any(|e| matches!(e, EncounterEvent::Failed { .. })));
    }

    #[test]
    fn reset_for_retry_returns_to_inactive_after_failure() {
        let mut state = EncounterState::default();
        state.spec = Some(lab_spec());
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        state.on_player_death();
        state.reset_for_retry();
        assert_eq!(state.phase, EncounterPhase::Inactive);
    }

    #[test]
    fn lock_active_truthy_during_active_phase() {
        let mut state = EncounterState::default();
        state.spec = Some(lab_spec());
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        assert!(state.phase.locks_exits());
        assert!(state.lock_active);
    }

    #[test]
    fn hud_summary_shows_wave_progress() {
        let mut state = EncounterState::default();
        state.spec = Some(lab_spec());
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        let summary = state.hud_summary();
        assert!(summary.contains("wave 1/2"));
        assert!(summary.contains("remaining 1"));
    }

    // ── SwitchActivation parsing ──────────────────────────────────

    #[test]
    fn switch_activation_parses_full_payload() {
        let act = SwitchActivation::parse_custom("switch:reset:ResetEncounter:mob_lab").unwrap();
        assert_eq!(act.id, "reset");
        assert_eq!(act.action, "ResetEncounter");
        assert_eq!(act.target_encounter, "mob_lab");
    }

    #[test]
    fn switch_activation_tolerates_empty_target() {
        let act = SwitchActivation::parse_custom("switch:reset:ResetEncounter:").unwrap();
        assert_eq!(act.target_encounter, "");
    }

    #[test]
    fn switch_activation_rejects_non_switch_payload() {
        assert!(SwitchActivation::parse_custom("door:foo:bar").is_none());
        assert!(SwitchActivation::parse_custom("switch").is_none());
    }

    // ── EncounterRegistry ──────────────────────────────────────────

    #[test]
    fn registry_ensure_creates_default_state() {
        let mut reg = EncounterRegistry::default();
        let state = reg.ensure("mob_lab");
        assert_eq!(state.phase, EncounterPhase::Inactive);
    }

    #[test]
    fn registry_active_camera_zoom_picks_active_encounter() {
        let mut reg = EncounterRegistry::default();
        let mut spec = lab_spec();
        spec.camera_zoom = 1.6;
        let state = reg.ensure("mob_lab");
        state.spec = Some(spec);
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        assert_eq!(reg.active_camera_zoom(), 1.6);
    }

    #[test]
    fn registry_camera_zoom_falls_back_to_one_when_inactive() {
        let mut reg = EncounterRegistry::default();
        reg.ensure("mob_lab").spec = Some({
            let mut s = lab_spec();
            s.camera_zoom = 1.6;
            s
        });
        // Phase still Inactive — no zoom applied.
        assert_eq!(reg.active_camera_zoom(), 1.0);
    }

    #[test]
    fn apply_persisted_cleared_keeps_lock_off() {
        let mut state = EncounterState::default();
        state.spec = Some(lab_spec());
        state.apply_persisted(PersistedEncounterState::Cleared);
        assert_eq!(state.phase, EncounterPhase::Cleared);
        assert!(!state.lock_active);
    }

    #[test]
    fn to_persisted_collapses_active_to_untouched() {
        let mut state = EncounterState::default();
        state.spec = Some(lab_spec());
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        assert_eq!(state.to_persisted(), PersistedEncounterState::Untouched);
    }

    // ── LDtk loader ────────────────────────────────────────────────

    #[test]
    fn load_encounter_specs_picks_up_mob_lab() {
        let project = LdtkProject::load_embedded();
        let save = ae::SandboxSaveData::default();
        let entries = load_encounter_specs_from_ldtk(&project, &save);
        let mob_lab = entries
            .iter()
            .find(|(id, _, _)| id == "mob_lab")
            .expect("mob_lab encounter should be loadable");
        assert!(!mob_lab.1.waves.is_empty());
        assert!(mob_lab.1.camera_zoom > 1.0);
        assert_eq!(mob_lab.2, PersistedEncounterState::Untouched);
    }

    #[test]
    fn load_encounter_specs_respects_persisted_cleared() {
        let project = LdtkProject::load_embedded();
        let mut save = ae::SandboxSaveData::default();
        save.set_encounter("mob_lab", PersistedEncounterState::Cleared);
        let entries = load_encounter_specs_from_ldtk(&project, &save);
        let (_, _, state) = entries
            .iter()
            .find(|(id, _, _)| id == "mob_lab")
            .expect("mob_lab encounter should be loadable");
        assert_eq!(*state, PersistedEncounterState::Cleared);
    }
}
