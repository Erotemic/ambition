//! `EncounterEvent` — the output stream of the encounter lifecycle reducer and
//! the wave director. Trace markers (Started/Completed/…) plus the one
//! side-effect variant `SpawnCommand` the wave adapter turns into a real ECS
//! mob. [`EncounterEventMsg`] is the ECS message form: the reducer publishes
//! `{encounter, event}` and effect adapters (switch auto-green, banners,
//! quests, trace) react without owning the lifecycle.

/// One lifecycle/wave event. The encounter id travels on the enclosing
/// [`EncounterEventMsg`]; the variants carry only their own payload.
#[derive(Clone, Debug, PartialEq)]
pub enum EncounterEvent {
    /// The lifecycle left `Inactive` (Start command accepted).
    Started,
    /// The win objective was met or `Complete` was commanded.
    Completed,
    /// The fail objective was met or `Fail` was commanded.
    Failed,
    /// The lifecycle returned to `Inactive` for a fresh attempt (Reset
    /// command accepted from a non-Inactive phase).
    Reset,
    /// The exit-seal state changed (derived from the phase transition).
    LockChanged { locked: bool },
    /// A signal key was recorded on the lifecycle (first receipt only).
    SignalReceived { key: String },
    /// A wave began (wave director).
    WaveStarted { wave_index: usize, label: String },
    /// Trace-only "an enemy is about to spawn" marker. The actual
    /// spawn happens via `SpawnCommand`.
    EnemySpawned { kind: String },
    /// Side-effect: spawn a real ECS encounter mob (wave director).
    ///
    /// The three identity questions travel separately: `id` is WHICH body
    /// instance (minted per spawn, unique across attempts), `character` is WHO
    /// it is (a catalog character id, `None` for a marker-derived mob that
    /// names none), and `kind` is HOW IT THINKS (the `CharacterBrain::Custom`
    /// archetype payload).
    SpawnCommand {
        id: String,
        character: Option<String>,
        kind: String,
        pos: [f32; 2],
        size: [f32; 2],
    },
}

impl EncounterEvent {
    pub fn label(&self) -> String {
        match self {
            Self::Started => "encounter_started".to_string(),
            Self::Completed => "encounter_completed".to_string(),
            Self::Failed => "encounter_failed".to_string(),
            Self::Reset => "encounter_reset".to_string(),
            Self::LockChanged { locked } => format!("encounter_lock_changed:{locked}"),
            Self::SignalReceived { key } => format!("encounter_signal:{key}"),
            Self::WaveStarted { wave_index, label } => {
                format!("encounter_wave_started:{wave_index}:{label}")
            }
            Self::EnemySpawned { kind } => format!("encounter_enemy_spawned:{kind}"),
            Self::SpawnCommand { id, kind, .. } => {
                format!("encounter_spawn_command:{kind}:{id}")
            }
        }
    }
}

/// The ECS message wrapper: which encounter the event belongs to. Written by
/// the lifecycle reducer (and the wave adapter for wave events); read by
/// effect adapters and the trace.
#[derive(bevy::prelude::Message, Clone, Debug)]
pub struct EncounterEventMsg {
    pub encounter: String,
    pub event: EncounterEvent,
}

impl EncounterEventMsg {
    pub fn new(encounter: impl Into<String>, event: EncounterEvent) -> Self {
        Self {
            encounter: encounter.into(),
            event,
        }
    }
}
