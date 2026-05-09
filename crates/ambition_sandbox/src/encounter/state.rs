use bevy::math::bounding::IntersectsVolume;
use bevy::prelude::Resource;

use ambition_engine as ae;
use ambition_engine::PersistedEncounterState;

use super::{EncounterEvent, EncounterMobSpec, EncounterSpec};

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

/// Extra breathing room between waves after the previous wave is fully defeated.
///
/// This is intentionally short: long enough for the player and the adaptive
/// music to register that the fight escalated, but not long enough to make the
/// encounter feel idle.
pub const ENCOUNTER_INTER_WAVE_DELAY_SECONDS: f32 = 0.70;

fn add_inter_wave_delay(mobs: &[EncounterMobSpec]) -> Vec<EncounterMobSpec> {
    mobs.iter()
        .cloned()
        .map(|mut mob| {
            mob.delay += ENCOUNTER_INTER_WAVE_DELAY_SECONDS;
            mob
        })
        .collect()
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
                    pending: add_inter_wave_delay(&next.mobs),
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
                    pending: add_inter_wave_delay(&next.mobs),
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
        let id = self.spec.as_ref().map(|s| s.id.as_str()).unwrap_or("--");
        match self.phase {
            EncounterPhase::Inactive => format!("[{id}] inactive"),
            EncounterPhase::Starting { remaining } => {
                let bar = countdown_bar(remaining, 3.0);
                format!("[{id}] LOCKED IN — wave 1 in {remaining:.1}s {bar}")
            }
            EncounterPhase::Active {
                wave_index,
                remaining_mobs,
            } => {
                let total = self.spec.as_ref().map(|s| s.waves.len()).unwrap_or(0);
                let label = self
                    .spec
                    .as_ref()
                    .and_then(|s| s.waves.get(wave_index).map(|w| w.label.as_str()))
                    .unwrap_or("wave");
                format!(
                    "[{id}] WAVE {}/{} :: {label} :: {} left",
                    wave_index + 1,
                    total,
                    remaining_mobs
                )
            }
            EncounterPhase::Cleared => format!("[{id}] CLEARED"),
            EncounterPhase::Failed => format!("[{id}] FAILED — reset to retry"),
        }
    }

    /// Human-readable status for the encounter's first cleared event,
    /// surfaced as a HUD banner on the frame the encounter ends.
    pub fn celebratory_banner(&self) -> Option<String> {
        match self.phase {
            EncounterPhase::Cleared => self
                .spec
                .as_ref()
                .map(|s| format!("ARENA CLEAR — {}", s.id)),
            _ => None,
        }
    }
}

fn countdown_bar(remaining: f32, total: f32) -> String {
    if total <= 0.0 {
        return String::new();
    }
    let ratio = (1.0 - (remaining / total).clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let filled = (ratio * 8.0).round() as usize;
    let empty = 8usize.saturating_sub(filled);
    format!("[{}{}]", "#".repeat(filled), ".".repeat(empty))
}
