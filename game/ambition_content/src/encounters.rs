//! Content encounter customers on the GENERIC lifecycle (E13).
//!
//! The Noether attunement is the first non-boss, non-wave encounter: a
//! signal-driven, NO-ACTOR puzzle in the symmetry room (the Noether Chamber).
//! Flip the chamber's gravity through all four kernel faces and the encounter
//! completes — every symmetry visited, every conservation law honored.
//!
//! The exit bar this module proves (encounter-orchestration.md E13): content
//! adds rules WITHOUT adding another lifecycle, objective evaluator, cleanup
//! path, or presentation authority. Everything here is either generic
//! vocabulary (the authority components at spawn), a command EMITTER (room
//! entry → `Start`, kernel flips → `Signal`), or an effect CONSUMER (the
//! celebration off the generic `Completed` event). The engine names none of
//! it; the lifecycle reducer decides everything.

use bevy::prelude::*;

use ambition_encounter::{
    Encounter, EncounterCommand, EncounterCommandKind, EncounterEvent, EncounterEventMsg,
    EncounterLifecycle, EncounterObjective, EncounterParticipants, EncounterPhase, Objective,
};
use ambition_persistence::save_data::PersistedEncounterState;
use ambition_platformer_primitives::schedule::{SandboxSet, SimScheduleExt};

/// The puzzle's stable encounter id (and save-flag namespace).
pub const SYMMETRY_ATTUNEMENT_ID: &str = "symmetry_attunement";

/// The room whose entry starts the attunement.
const SYMMETRY_ROOM_ID: &str = "symmetry_room";

/// Save flag remembering a completed attunement across save/load.
pub const SYMMETRY_ATTUNEMENT_FLAG: &str = "symmetry_attunement_complete";

/// The four kernel-face facts the puzzle consumes: LDtk `Switch` action →
/// the stable signal key content publishes for it.
const KERNEL_FACES: [(&str, &str); 4] = [
    ("SetGravityDown", "gravity_down"),
    ("SetGravityLeft", "gravity_left"),
    ("SetGravityUp", "gravity_up"),
    ("SetGravityRight", "gravity_right"),
];

/// Spawn the attunement authority once: the generic component set and nothing
/// else — no waves, no participants, no bespoke state. A previously completed
/// attunement (save flag) starts terminal, so the reducer refuses a restart.
pub fn spawn_symmetry_attunement(
    mut commands: Commands,
    existing: Query<&Encounter>,
    save: Res<ambition_persistence::save::SandboxSave>,
) {
    if existing.iter().any(|enc| enc.id == SYMMETRY_ATTUNEMENT_ID) {
        return;
    }
    let mut lifecycle = EncounterLifecycle::default();
    if save.data().flag(SYMMETRY_ATTUNEMENT_FLAG) {
        lifecycle.apply_persisted(PersistedEncounterState::Cleared);
    }
    commands.spawn((
        Encounter::new(SYMMETRY_ATTUNEMENT_ID),
        ambition_platformer_primitives::sim_id::SimId::encounter(SYMMETRY_ATTUNEMENT_ID),
        lifecycle,
        EncounterObjective::win(Objective::All(
            KERNEL_FACES
                .iter()
                .map(|(_, signal)| Objective::ReceiveSignal((*signal).to_string()))
                .collect(),
        )),
        EncounterParticipants::default(),
    ));
}

/// Command EMITTER: entering the Noether Chamber starts the attunement;
/// every kernel-face gravity flip publishes its stable signal fact. The
/// generic objective (`All` of the four signals) completes it — this adapter
/// never touches the phase.
pub fn drive_symmetry_attunement(
    room_set: ambition_platformer_primitives::lifecycle::SessionWorldRef<
        ambition_actors::rooms::RoomSet,
    >,
    encounters: Query<(&Encounter, &EncounterLifecycle)>,
    mut switches: MessageReader<ambition_actors::features::SwitchActivated>,
    mut lifecycle_commands: MessageWriter<EncounterCommand>,
) {
    let Some((_, lifecycle)) = encounters
        .iter()
        .find(|(enc, _)| enc.id == SYMMETRY_ATTUNEMENT_ID)
    else {
        return;
    };
    if room_set.active_spec().id == SYMMETRY_ROOM_ID
        && matches!(lifecycle.phase, EncounterPhase::Inactive)
    {
        lifecycle_commands.write(EncounterCommand::new(
            SYMMETRY_ATTUNEMENT_ID,
            EncounterCommandKind::Start,
        ));
    }
    for switch in switches.read() {
        if let Some((_, signal)) = KERNEL_FACES
            .iter()
            .find(|(action, _)| *action == switch.activation.action)
        {
            lifecycle_commands.write(EncounterCommand::signal(SYMMETRY_ATTUNEMENT_ID, *signal));
        }
    }
}

/// Effect CONSUMER: the generic `Completed` event pays the puzzle out —
/// a celebration banner and the persistent save flag. No lifecycle authority
/// here; the reducer already decided.
pub fn celebrate_symmetry_attunement(
    mut events: MessageReader<EncounterEventMsg>,
    mut banners: MessageWriter<ambition_actors::features::GameplayBannerRequested>,
    mut save: ResMut<ambition_persistence::save::SandboxSave>,
) {
    for msg in events.read() {
        if msg.encounter == SYMMETRY_ATTUNEMENT_ID && matches!(msg.event, EncounterEvent::Completed)
        {
            banners.write(ambition_actors::features::GameplayBannerRequested::new(
                "NOETHER ATTUNEMENT — every symmetry conserved".to_string(),
                4.0,
            ));
            save.data_mut().set_flag(SYMMETRY_ATTUNEMENT_FLAG, true);
        }
    }
}

/// The content encounter customers' plugin: emitters before the generic
/// reducer, the celebration after it. Added by `AmbitionContentPlugin`.
pub struct AmbitionEncounterContentPlugin;

impl Plugin for AmbitionEncounterContentPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            (spawn_symmetry_attunement, drive_symmetry_attunement)
                .chain()
                .in_set(SandboxSet::GameplayEffects),
        );
        app.add_systems(
            sim,
            celebrate_symmetry_attunement
                .in_set(SandboxSet::Progression)
                .after(ambition_encounter::EncounterLifecycleSet),
        );
    }
}
