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

/// One mob to spawn during a wave.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncounterMobSpec {
    /// `EnemyBrain::Custom(kind)` payload — picks the archetype
    /// (`small_skitter`, `medium_striker`, `large_brute`, ...).
    pub kind: String,
    /// Spawn position in active-area-local coordinates (the mob's
    /// center, not top-left).
    pub spawn: [f32; 2],
    /// Mob hitbox size; defaults to a sensible per-archetype value.
    pub size: [f32; 2],
    /// Seconds after the wave starts before this mob spawns. `0.0`
    /// means "with the wave".
    pub delay: f32,
}

impl EncounterMobSpec {
    pub fn new(kind: impl Into<String>, spawn: [f32; 2]) -> Self {
        Self {
            kind: kind.into(),
            spawn,
            size: [22.0, 38.0],
            delay: 0.0,
        }
    }

    pub fn with_size(mut self, size: [f32; 2]) -> Self {
        self.size = size;
        self
    }

    pub fn with_delay(mut self, delay: f32) -> Self {
        self.delay = delay;
        self
    }
}

/// One wave of mobs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncounterWaveSpec {
    pub label: String,
    pub mobs: Vec<EncounterMobSpec>,
}

/// Marker for an encounter-spawned solid wall (the "lock wall" that
/// appears in the doorway while the encounter is Active and is
/// removed when the encounter ends).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LockWallSpec {
    pub min: [f32; 2],
    pub size: [f32; 2],
}

impl LockWallSpec {
    pub fn aabb(&self) -> ae::Aabb {
        ae::aabb_from_min_size(
            ae::Vec2::new(self.min[0], self.min[1]),
            ae::Vec2::new(self.size[0], self.size[1]),
        )
    }
}

/// Whole encounter authored data: ordered list of waves plus the
/// activation AABB, intro/music settings, optional lock wall, and the
/// camera-zoom factor to apply while the encounter is active.
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
    /// Optional dynamic wall that spawns when the encounter goes
    /// Active and is removed when it leaves Active.
    pub lock_wall: Option<LockWallSpec>,
    /// Seconds the encounter spends in `Starting` (intro) before the
    /// first wave kicks off. The camera + lock + music change happen
    /// at the start of `Starting`; enemies don't spawn until `Active`.
    pub intro_seconds: f32,
    /// Music track id to play while the encounter is Active. Empty
    /// disables the music swap.
    pub music_track: String,
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
///
/// `Starting` is the brief intro window after the player crosses the
/// trigger but before the first wave's mobs spawn — the camera zoom,
/// lock wall, and music change happen at the *start* of `Starting`,
/// then enemies begin to appear `intro_seconds` later when the phase
/// transitions to `Active`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum EncounterPhase {
    #[default]
    Inactive,
    /// Intro: camera zoom + lock wall + music are already in play;
    /// counting down to wave 1 spawn.
    Starting { remaining: f32 },
    /// Waves are spawning / being fought.
    Active {
        wave_index: usize,
        remaining_mobs: usize,
    },
    /// All waves cleared. Lock is released, music + camera revert.
    Cleared,
    /// Player died inside the encounter. Reset path will return
    /// the encounter to `Inactive`.
    Failed,
}

impl Eq for EncounterPhase {}

impl EncounterPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::Starting { .. } => "starting",
            Self::Active { .. } => "active",
            Self::Cleared => "cleared",
            Self::Failed => "failed",
        }
    }

    pub fn locks_exits(self) -> bool {
        matches!(self, Self::Starting { .. } | Self::Active { .. })
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

/// Per-encounter run state: the live wave's pending and alive mobs
/// plus the elapsed-since-wave-start timer driving delayed sub-spawns.
/// Lives outside `EncounterPhase` so the phase enum stays cheap to
/// copy / pattern-match.
#[derive(Clone, Debug, Default)]
pub struct EncounterRun {
    /// MobSpecs the active wave hasn't spawned yet (decreasing-delay
    /// order; entries pop from the front when their delay elapses).
    pub pending: Vec<EncounterMobSpec>,
    /// Ids of mobs spawned by the active wave that are still alive.
    /// The encounter system removes ids whose matching enemy is dead.
    pub alive_ids: Vec<String>,
    /// Seconds since the active wave started.
    pub wave_elapsed: f32,
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
    /// Live wave run state (pending / alive_ids / wave_elapsed). Reset
    /// on every wave start.
    pub run: EncounterRun,
    /// Bumped each time a unique mob id needs to be generated. Lets
    /// successive encounter attempts produce non-colliding ids.
    pub spawn_counter: u32,
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
            EncounterPhase::Inactive
            | EncounterPhase::Starting { .. }
            | EncounterPhase::Active { .. } => PersistedEncounterState::Untouched,
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
        if spec.waves.is_empty() {
            // No waves authored → clear immediately.
            self.phase = EncounterPhase::Cleared;
            self.lock_active = false;
            self.run = EncounterRun::default();
            events.push(EncounterEvent::Started {
                id: spec.id.clone(),
            });
            events.push(EncounterEvent::Cleared { id: spec.id });
            events.push(EncounterEvent::LockChanged { locked: false });
            return events;
        }
        // Enter intro: lock wall + camera + music ramp on this frame;
        // first wave's mobs spawn after the intro elapses.
        self.phase = EncounterPhase::Starting {
            remaining: spec.intro_seconds.max(0.0),
        };
        self.lock_active = true;
        self.run = EncounterRun::default();
        events.push(EncounterEvent::Started {
            id: spec.id.clone(),
        });
        events.push(EncounterEvent::LockChanged { locked: true });
        events
    }

    /// Drive the encounter forward by `dt`. Resolves the intro
    /// countdown, spawns delayed mobs, removes dead alive_ids, and
    /// advances waves. Returns the events the caller routes through
    /// trace + spawning.
    ///
    /// `enemy_alive` is a closure: given a mob id, returns whether
    /// the corresponding enemy is still alive. Lets the encounter
    /// system stay headless-testable by not depending on the runtime.
    pub fn tick_intro_or_wave(
        &mut self,
        dt: f32,
        mut enemy_alive: impl FnMut(&str) -> bool,
    ) -> Vec<EncounterEvent> {
        let mut events = Vec::new();
        let Some(spec) = self.spec.clone() else {
            return events;
        };

        // Intro: count down, then transition to Active{wave 0}.
        if let EncounterPhase::Starting { remaining } = self.phase {
            let next = remaining - dt;
            if next > 0.0 {
                self.phase = EncounterPhase::Starting { remaining: next };
                return events;
            }
            self.phase = EncounterPhase::Active {
                wave_index: 0,
                remaining_mobs: spec.waves[0].mobs.len(),
            };
            self.run = EncounterRun {
                pending: spec.waves[0].mobs.clone(),
                alive_ids: Vec::new(),
                wave_elapsed: 0.0,
            };
            events.push(EncounterEvent::WaveStarted {
                wave_index: 0,
                label: spec.waves[0].label.clone(),
            });
        }

        let EncounterPhase::Active { wave_index, .. } = self.phase else {
            return events;
        };

        // Advance wave clock.
        self.run.wave_elapsed += dt;

        // 1. Drop alive_ids whose runtime enemy is dead. MUST run
        //    BEFORE the spawn loop so newly-spawned mobs aren't
        //    immediately reaped: the caller's `enemy_alive` lookup
        //    was built from the runtime BEFORE this tick fires, so
        //    it doesn't know about mobs spawned later in this same
        //    tick. Spawning first then retaining would unconditionally
        //    drop every just-spawned id and the wave would clear in
        //    a single tick. (This was the "encounter ends after 2
        //    seconds" bug.)
        self.run.alive_ids.retain(|id| enemy_alive(id));

        // 2. Spawn pending mobs whose delay has elapsed.
        let mut still_pending = Vec::with_capacity(self.run.pending.len());
        for mob in std::mem::take(&mut self.run.pending) {
            if mob.delay <= self.run.wave_elapsed {
                self.spawn_counter = self.spawn_counter.saturating_add(1);
                let id = format!(
                    "encounter:{}:w{}:{}",
                    spec.id, wave_index, self.spawn_counter
                );
                events.push(EncounterEvent::EnemySpawned {
                    kind: mob.kind.clone(),
                });
                events.push(EncounterEvent::SpawnCommand {
                    id: id.clone(),
                    kind: mob.kind.clone(),
                    pos: mob.spawn,
                    size: mob.size,
                });
                self.run.alive_ids.push(id);
            } else {
                still_pending.push(mob);
            }
        }
        self.run.pending = still_pending;

        // Update remaining_mobs on the phase for HUD parity.
        let remaining_mobs = self.run.pending.len() + self.run.alive_ids.len();
        self.phase = EncounterPhase::Active {
            wave_index,
            remaining_mobs,
        };

        // Wave complete → next wave or Cleared.
        if remaining_mobs == 0 {
            let next_wave = wave_index + 1;
            if let Some(next) = spec.waves.get(next_wave) {
                self.phase = EncounterPhase::Active {
                    wave_index: next_wave,
                    remaining_mobs: next.mobs.len(),
                };
                self.run = EncounterRun {
                    pending: next.mobs.clone(),
                    alive_ids: Vec::new(),
                    wave_elapsed: 0.0,
                };
                events.push(EncounterEvent::WaveStarted {
                    wave_index: next_wave,
                    label: next.label.clone(),
                });
            } else {
                self.phase = EncounterPhase::Cleared;
                self.lock_active = false;
                self.run = EncounterRun::default();
                events.push(EncounterEvent::Cleared { id: spec.id });
                events.push(EncounterEvent::LockChanged { locked: false });
            }
        }
        events
    }

    /// Legacy helper kept for tests that drive defeats one at a time
    /// (rather than via the alive-id retain path). Prefer
    /// `tick_intro_or_wave`. Marks one alive_id (the last one) as
    /// dead and re-runs wave-progress logic.
    pub fn on_mob_defeated(&mut self) -> Vec<EncounterEvent> {
        let mut events = Vec::new();
        let Some(spec) = self.spec.clone() else {
            return events;
        };
        let EncounterPhase::Active { wave_index, .. } = self.phase else {
            return events;
        };
        if self.run.alive_ids.is_empty() && self.run.pending.is_empty() {
            // Wave already empty — fall through to clear logic below.
        } else if !self.run.alive_ids.is_empty() {
            self.run.alive_ids.pop();
        } else {
            return events;
        }
        let remaining = self.run.pending.len() + self.run.alive_ids.len();
        self.phase = EncounterPhase::Active {
            wave_index,
            remaining_mobs: remaining,
        };
        if remaining == 0 {
            let next_wave = wave_index + 1;
            if let Some(next) = spec.waves.get(next_wave) {
                self.phase = EncounterPhase::Active {
                    wave_index: next_wave,
                    remaining_mobs: next.mobs.len(),
                };
                self.run = EncounterRun {
                    pending: next.mobs.clone(),
                    alive_ids: Vec::new(),
                    wave_elapsed: 0.0,
                };
                events.push(EncounterEvent::WaveStarted {
                    wave_index: next_wave,
                    label: next.label.clone(),
                });
            } else {
                self.phase = EncounterPhase::Cleared;
                self.lock_active = false;
                self.run = EncounterRun::default();
                events.push(EncounterEvent::Cleared { id: spec.id });
                events.push(EncounterEvent::LockChanged { locked: false });
            }
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
            EncounterPhase::Starting { remaining } => {
                format!("encounter starting in {:.1}s", remaining)
            }
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

/// Trace + side-effect events emitted by the encounter state machine.
/// The sandbox projects these into `GameplayTraceEvent` and routes
/// `SpawnCommand` to `FeatureRuntime::spawn_enemy`.
#[derive(Clone, Debug, PartialEq)]
pub enum EncounterEvent {
    Started {
        id: String,
    },
    WaveStarted {
        wave_index: usize,
        label: String,
    },
    /// Trace-only "an enemy is about to spawn" marker. The actual
    /// spawn happens via `SpawnCommand`.
    EnemySpawned {
        kind: String,
    },
    /// Side-effect: spawn a real enemy in `FeatureRuntime` with the
    /// given id / brain / world position / size.
    SpawnCommand {
        id: String,
        kind: String,
        pos: [f32; 2],
        size: [f32; 2],
    },
    Cleared {
        id: String,
    },
    Failed {
        id: String,
    },
    LockChanged {
        locked: bool,
    },
}

impl EncounterEvent {
    pub fn label(&self) -> String {
        match self {
            Self::Started { id } => format!("encounter_started:{id}"),
            Self::WaveStarted { wave_index, label } => {
                format!("encounter_wave_started:{wave_index}:{label}")
            }
            Self::EnemySpawned { kind } => format!("encounter_enemy_spawned:{kind}"),
            Self::SpawnCommand { id, kind, .. } => {
                format!("encounter_spawn_command:{kind}:{id}")
            }
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
    /// any). 1.0 if no encounter is in flight. The camera starts
    /// zooming during `Starting` so the ramp finishes before wave 1
    /// spawns.
    pub fn active_camera_zoom(&self) -> f32 {
        for state in self.encounters.values() {
            if matches!(
                state.phase,
                EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
            ) {
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
            music_track: "pulse_drift_voyage".into(),
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
                EncounterMobSpec::new("medium_striker", [right_x, floor_y]).with_size(goblin_size),
                // Big goblin reinforcement on a timer, fires whether
                // or not the goblins are still up.
                EncounterMobSpec::new("large_brute", [(left_x + right_x) * 0.5, floor_y - 18.0])
                    .with_size(big_size)
                    .with_delay(3.5),
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
/// Encounter cancellation: encounters that are `Active` only persist
/// while the player is in the matching active area. Walking out
/// (e.g. through the entry LoadingZone) resets the encounter to
/// `Inactive` so the camera zoom + lock release on exit. This is
/// deliberate sandbox UX — the encounter is "in play" only while the
/// player is actually inside the room.
pub fn update_encounters_from_world(
    time: Res<Time>,
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
        if let Some(switch_id) =
            switch_id_for_encounter(&encounter_id, &runtime.features.switches)
        {
            save.data_mut().set_switch(&switch_id, true);
            runtime.features.set_switch_on(&switch_id, true);
        }
        runtime.features.despawn_encounter_enemies(&encounter_id);
        // Quest hook: a "clear encounter" step can advance now.
        quests.push_event(ae::QuestAdvanceEvent::EncounterCleared(
            encounter_id.clone(),
        ));
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
        quests.push_event(ae::QuestAdvanceEvent::FlagSet(
            "test_switch_toggled".into(),
        ));
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
mod tests {
    use super::*;

    /// Drive an EncounterState past `Starting` into the first wave's
    /// `Active` phase. The lab_spec uses `intro_seconds: 0.0` so a
    /// single tick is enough.
    fn advance_past_intro(state: &mut EncounterState) {
        let _ = state.tick_intro_or_wave(0.001, |_| true);
    }

    fn lab_spec() -> EncounterSpec {
        EncounterSpec {
            id: "mob_lab".into(),
            waves: vec![
                EncounterWaveSpec {
                    label: "wave 1".into(),
                    mobs: vec![EncounterMobSpec::new("dummy", [100.0, 100.0])],
                },
                EncounterWaveSpec {
                    label: "wave 2".into(),
                    mobs: vec![
                        EncounterMobSpec::new("dummy", [120.0, 100.0]),
                        EncounterMobSpec::new("dummy", [180.0, 100.0]),
                    ],
                },
            ],
            trigger_min: [0.0, 0.0],
            trigger_size: [400.0, 200.0],
            camera_zoom: 1.5,
            lock_wall: None,
            // Tests want immediate spawn on entry — skip the intro
            // delay so `entering_trigger_starts_first_wave` etc. can
            // check the Active state right after `maybe_start`.
            intro_seconds: 0.0,
            music_track: String::new(),
        }
    }

    #[test]
    fn entering_trigger_starts_first_wave() {
        let mut state = EncounterState::default();
        state.spec = Some(lab_spec());
        let events = state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        assert!(state.lock_active);
        assert!(matches!(state.phase, EncounterPhase::Starting { .. }));
        assert!(events
            .iter()
            .any(|e| matches!(e, EncounterEvent::Started { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, EncounterEvent::LockChanged { locked: true })));
        // After the intro tick, we land in Active{wave 0}.
        advance_past_intro(&mut state);
        assert_eq!(
            state.phase,
            EncounterPhase::Active {
                wave_index: 0,
                remaining_mobs: 1,
            }
        );
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
        advance_past_intro(&mut state);
        // Wave 1 has 1 mob; spawn it then mark defeated.
        let _ = state.tick_intro_or_wave(0.001, |_| true);
        let _ = state.on_mob_defeated();
        // Wave 2 has 2 mobs; spawn + defeat both.
        let _ = state.tick_intro_or_wave(0.001, |_| true);
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
        advance_past_intro(&mut state);
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
        advance_past_intro(&mut state);
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
        advance_past_intro(&mut state);
        assert!(state.phase.locks_exits());
    }

    #[test]
    fn hud_summary_shows_wave_progress() {
        let mut state = EncounterState::default();
        state.spec = Some(lab_spec());
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        advance_past_intro(&mut state);
        let summary = state.hud_summary();
        assert!(summary.contains("wave 1/2"), "got: {summary}");
        assert!(summary.contains("remaining 1"), "got: {summary}");
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

    #[test]
    fn ldtk_switch_runtime_id_matches_activation_payload() {
        // Regression for the bug where the Switch RoomObject id was
        // entity.iid (e.g. "Switch-4072") but the
        // SwitchActivation payload's id was the LDtk `id` field
        // ("mob_lab_reset_switch"). That mismatch made
        // `FeatureRuntime::set_switch_on(activation.id)` a no-op and
        // the switch sprite stayed stuck red.
        let project = LdtkProject::load_embedded();
        let room_set = project.to_room_set().expect("mob_lab world composes");
        let mob_lab = room_set
            .rooms
            .iter()
            .find(|r| r.id == "mob_lab")
            .expect("mob_lab room");
        let switch_object = mob_lab
            .world
            .objects
            .iter()
            .find(|o| {
                matches!(
                    &o.kind,
                    ae::RoomObjectKind::Interactable(i)
                        if matches!(&i.kind, ae::InteractionKind::Custom(s)
                            if s.starts_with("switch:"))
                )
            })
            .expect("mob_lab has a switch interactable");
        let payload = match &switch_object.kind {
            ae::RoomObjectKind::Interactable(i) => match &i.kind {
                ae::InteractionKind::Custom(s) => s.clone(),
                _ => panic!("switch kind"),
            },
            _ => panic!("switch object kind"),
        };
        let activation = SwitchActivation::parse_custom(&payload).expect("parse");
        assert_eq!(
            switch_object.id, activation.id,
            "RoomObject.id must equal the SwitchActivation.id so set_switch_on works"
        );
    }

    #[test]
    fn mob_lab_loaded_spec_has_three_waves_lockwall_and_intro() {
        let project = LdtkProject::load_embedded();
        let save = ae::SandboxSaveData::default();
        let entries = load_encounter_specs_from_ldtk(&project, &save);
        let (_, spec, _) = entries
            .iter()
            .find(|(id, _, _)| id == "mob_lab")
            .expect("mob_lab encounter should be loadable");
        assert_eq!(
            spec.waves.len(),
            3,
            "expected 3 waves; got {}",
            spec.waves.len()
        );
        assert_eq!(spec.waves[0].mobs.len(), 2);
        assert_eq!(spec.waves[1].mobs.len(), 3, "wave 2 = 2 goblins + 1 big");
        assert_eq!(spec.waves[2].mobs.len(), 2, "wave 3 = 2 big goblins");
        // Wave 2's third mob should have a delay > 0 (the timer-based
        // big-goblin reinforcement).
        assert!(
            spec.waves[1].mobs.iter().any(|m| m.delay > 0.0),
            "wave 2 should have at least one delayed sub-spawn"
        );
        assert!(
            spec.lock_wall.is_some(),
            "mob_lab spec should pick up the LockWall marker"
        );
        assert!(spec.intro_seconds > 0.0);
        assert_eq!(spec.music_track, "pulse_drift_voyage");
    }

    // ── Multi-wave spawning behavior ───────────────────────────────

    #[test]
    fn intro_delays_first_wave_spawn_until_elapsed() {
        let mut state = EncounterState::default();
        let mut spec = lab_spec();
        spec.intro_seconds = 1.5;
        state.spec = Some(spec);
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        // Halfway through the intro: still Starting, no spawns yet.
        let evs = state.tick_intro_or_wave(0.5, |_| true);
        assert!(matches!(state.phase, EncounterPhase::Starting { .. }));
        assert!(!evs
            .iter()
            .any(|e| matches!(e, EncounterEvent::SpawnCommand { .. })));
        // After the rest of the intro: Active + a spawn command.
        let evs = state.tick_intro_or_wave(1.2, |_| true);
        assert!(matches!(state.phase, EncounterPhase::Active { .. }));
        assert!(evs
            .iter()
            .any(|e| matches!(e, EncounterEvent::SpawnCommand { .. })));
    }

    #[test]
    fn delayed_sub_spawn_holds_then_fires() {
        let mut state = EncounterState::default();
        let mut spec = lab_spec();
        spec.intro_seconds = 0.0;
        // One immediate, one delayed-by-2s.
        spec.waves = vec![EncounterWaveSpec {
            label: "wave 1".into(),
            mobs: vec![
                EncounterMobSpec::new("medium_striker", [100.0, 100.0]),
                EncounterMobSpec::new("large_brute", [200.0, 100.0]).with_delay(2.0),
            ],
        }];
        state.spec = Some(spec);
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        // Tick once: intro elapses, wave 1 starts, immediate mob spawns.
        let evs = state.tick_intro_or_wave(0.5, |_| true);
        let immediate_spawns = evs
            .iter()
            .filter(|e| matches!(e, EncounterEvent::SpawnCommand { .. }))
            .count();
        assert_eq!(immediate_spawns, 1);
        // Tick to 1.0s wave-elapsed: still nothing new.
        let evs = state.tick_intro_or_wave(0.5, |_| true);
        assert_eq!(
            evs.iter()
                .filter(|e| matches!(e, EncounterEvent::SpawnCommand { .. }))
                .count(),
            0
        );
        // Tick past 2.0s: delayed mob fires.
        let evs = state.tick_intro_or_wave(1.5, |_| true);
        assert_eq!(
            evs.iter()
                .filter(|e| matches!(e, EncounterEvent::SpawnCommand { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn wave_clears_only_when_all_pending_and_alive_are_resolved() {
        let mut state = EncounterState::default();
        let mut spec = lab_spec();
        spec.intro_seconds = 0.0;
        spec.waves = vec![
            EncounterWaveSpec {
                label: "wave 1".into(),
                mobs: vec![
                    EncounterMobSpec::new("medium_striker", [100.0, 100.0]),
                    EncounterMobSpec::new("medium_striker", [200.0, 100.0]).with_delay(1.0),
                ],
            },
            EncounterWaveSpec {
                label: "wave 2".into(),
                mobs: vec![EncounterMobSpec::new("large_brute", [150.0, 100.0])],
            },
        ];
        state.spec = Some(spec);
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        // Intro tick: Starting → Active{wave 0}; immediate mob spawned.
        // Closure says "alive" so the just-spawned id sticks.
        let _ = state.tick_intro_or_wave(0.001, |_| true);
        // 0.5s elapsed: alive mob marked dead, but the delayed mob
        // hasn't fired yet → wave still pending.
        let _ = state.tick_intro_or_wave(0.5, |_| false);
        assert!(matches!(
            state.phase,
            EncounterPhase::Active { wave_index: 0, .. }
        ));
        // 1.001s wave-elapsed: delayed mob spawns. Retain runs first
        // (no alive ids to drop; closure won't see new id this tick).
        let _ = state.tick_intro_or_wave(0.5, |_| false);
        // Still wave 1: the just-spawned mob is alive in the encounter
        // bookkeeping (not yet been retained against a stale lookup).
        assert!(
            matches!(state.phase, EncounterPhase::Active { wave_index: 0, .. }),
            "wave 1 should hold while the just-spawned mob is alive"
        );
        // Next tick: retain drops the just-spawned mob (closure
        // returns false), wave clears, wave 2 starts.
        let _ = state.tick_intro_or_wave(0.001, |_| false);
        assert!(
            matches!(state.phase, EncounterPhase::Active { wave_index: 1, .. }),
            "expected wave 2 active, got {:?}",
            state.phase
        );
    }

    #[test]
    fn just_spawned_mob_survives_one_tick_before_retain() {
        // Regression for the "encounter ends after 2 seconds" bug:
        // newly-spawned mobs were immediately reaped because retain
        // ran AFTER spawn with a stale alive_lookup. The fix is to
        // run retain BEFORE spawn so the new id has a frame to live.
        let mut state = EncounterState::default();
        let mut spec = lab_spec();
        spec.intro_seconds = 0.0;
        spec.waves = vec![EncounterWaveSpec {
            label: "wave 1".into(),
            mobs: vec![EncounterMobSpec::new("medium_striker", [100.0, 100.0])],
        }];
        state.spec = Some(spec);
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        // Intro elapses + spawn happens. Closure returns false (the
        // runtime hasn't seen the new id yet — the bug condition).
        let _ = state.tick_intro_or_wave(0.001, |_| false);
        // The mob must still be tracked: the wave shouldn't be cleared.
        assert!(
            matches!(state.phase, EncounterPhase::Active { wave_index: 0, remaining_mobs: 1 }),
            "just-spawned mob must survive the first tick; got {:?}",
            state.phase
        );
    }

    // ── Switch arming gate (helpers) ───────────────────────────────

    fn switch_runtime(payload: &str, on: bool) -> crate::features::SwitchRuntime {
        let id = SwitchActivation::parse_custom(payload)
            .map(|a| a.id)
            .unwrap_or_else(|| "x".into());
        crate::features::SwitchRuntime {
            id,
            name: "test".into(),
            pos: ae::Vec2::ZERO,
            size: ae::Vec2::splat(16.0),
            interactable: ae::Interactable::new(
                "x",
                "x",
                ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(8.0)),
                ae::InteractionKind::Custom(payload.into()),
            ),
            custom_payload: payload.into(),
            on,
        }
    }

    #[test]
    fn encounter_armed_when_no_linked_switch() {
        // No switch in the runtime → armed by default.
        assert!(encounter_armed_by_switch("mob_lab", &[]));
    }

    #[test]
    fn encounter_armed_when_linked_switch_off() {
        // Switch off (red) = armed.
        let switches = vec![switch_runtime(
            "switch:mob_lab_reset_switch:ResetEncounter:mob_lab",
            false,
        )];
        assert!(encounter_armed_by_switch("mob_lab", &switches));
    }

    #[test]
    fn encounter_disarmed_when_linked_switch_on() {
        // Switch on (green) = disabled.
        let switches = vec![switch_runtime(
            "switch:mob_lab_reset_switch:ResetEncounter:mob_lab",
            true,
        )];
        assert!(!encounter_armed_by_switch("mob_lab", &switches));
    }

    #[test]
    fn unrelated_switches_dont_arm_other_encounters() {
        // Switch targets boss_room; mob_lab has no linked switch
        // → mob_lab is armed by default.
        let switches = vec![switch_runtime(
            "switch:boss_reset_switch:ResetEncounter:boss_room",
            true,
        )];
        assert!(encounter_armed_by_switch("mob_lab", &switches));
        assert!(!encounter_armed_by_switch("boss_room", &switches));
    }

    #[test]
    fn switch_id_for_encounter_finds_linked_switch() {
        let switches = vec![
            switch_runtime("switch:other_switch:ResetEncounter:other_room", false),
            switch_runtime(
                "switch:mob_lab_reset_switch:ResetEncounter:mob_lab",
                false,
            ),
        ];
        assert_eq!(
            switch_id_for_encounter("mob_lab", &switches),
            Some("mob_lab_reset_switch".into())
        );
        assert_eq!(switch_id_for_encounter("nonexistent", &switches), None);
    }

    // ── Lock wall sync ─────────────────────────────────────────────

    #[test]
    fn sync_lock_walls_inserts_and_removes_block() {
        use ambition_engine::Block;
        let mut world = ae::World::new(
            "test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::ZERO,
            vec![Block::solid(
                "floor",
                ae::Vec2::ZERO,
                ae::Vec2::new(2000.0, 16.0),
            )],
        );
        let mut reg = EncounterRegistry::default();
        let mut spec = lab_spec();
        spec.lock_wall = Some(LockWallSpec {
            min: [100.0, 100.0],
            size: [32.0, 200.0],
        });
        let state = reg.ensure("mob_lab");
        state.spec = Some(spec);
        state.maybe_start(ae::Vec2::new(50.0, 50.0), ae::Vec2::new(20.0, 30.0));
        sync_lock_walls(&mut world, &reg);
        assert!(world.blocks.iter().any(|b| b.name == "lockwall:mob_lab"));
        // Force back to Inactive — wall should be removed.
        let state = reg.ensure("mob_lab");
        state.phase = EncounterPhase::Inactive;
        sync_lock_walls(&mut world, &reg);
        assert!(!world.blocks.iter().any(|b| b.name == "lockwall:mob_lab"));
    }
}
