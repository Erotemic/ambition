/// Trace + side-effect events emitted by the encounter state machine.
/// The sandbox projects these into `GameplayTraceEvent` and routes
/// `SpawnCommand` to ECS actor spawning.
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
    /// Side-effect: spawn a real ECS encounter mob with the given id /
    /// brain / world position / size.
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
