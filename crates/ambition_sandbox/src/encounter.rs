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

use bevy::math::bounding::IntersectsVolume;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use ambition_engine as ae;
use ambition_engine::PersistedEncounterState;

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
}
