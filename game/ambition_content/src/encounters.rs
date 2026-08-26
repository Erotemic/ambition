//! Content encounter customers on the GENERIC lifecycle (E13).
//!
//! The Noether attunement is the first non-boss, non-wave encounter: a
//! signal-driven, NO-ACTOR puzzle in the symmetry room (the Noether Chamber).
//! Flip the chamber's gravity through all four kernel faces and the encounter
//! completes — every symmetry visited, every conservation law honored.
//!
//! The generic encounter contract this module proves (docs/systems/boss-encounter-architecture.md): content
//! adds rules WITHOUT adding another lifecycle, objective evaluator, cleanup
//! path, or presentation authority. Everything here is either generic
//! vocabulary (the authority components at spawn), a command EMITTER (room
//! entry → `Start`), or an effect CONSUMER (the celebration off the generic
//! `Completed` event). The engine names none of it; the lifecycle reducer
//! decides everything.
//!
//! The `(switch id, signal key)` const table and the system that walked it are gone; what is
//! left of the pairing is the level.

use bevy::prelude::*;

use ambition_encounter::{
    Encounter, EncounterCommand, EncounterCommandKind, EncounterEvent, EncounterEventMsg,
    EncounterLifecycle, EncounterObjective, EncounterParticipants, EncounterPhase, Objective,
};
use ambition_persistence::save_data::PersistedEncounterState;
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, SimScheduleExt,
};

/// The puzzle's stable encounter id (and save-flag namespace).
pub const SYMMETRY_ATTUNEMENT_ID: &str = "symmetry_attunement";

/// The room whose entry starts the attunement.
const SYMMETRY_ROOM_ID: &str = "symmetry_room";

/// Save flag remembering a completed attunement across save/load.
pub const SYMMETRY_ATTUNEMENT_FLAG: &str = "symmetry_attunement_complete";

/// The four kernel-face facts the puzzle's win condition is made of.
///
/// what survived is an encounter stating its own win condition, which is
/// legitimate and is a different sentence from *"this switch does that"*: the
/// puzzle is complete when all four symmetries have been visited, and only the
/// puzzle knows that.
const KERNEL_SIGNALS: [&str; 4] = [
    "gravity_down",
    "gravity_left",
    "gravity_up",
    "gravity_right",
];

/// Spawn the attunement authority once: the generic component set and nothing else — no waves, no
/// participants, no bespoke state.
///
/// SESSION-SCOPED, like every encounter authority: the session that activated
/// the puzzle owns it, so retirement tears it down and the next session's
/// spawn cannot mint a duplicate `SimId:encounter` (,
/// ). A shell host at a non-gameplay route sleeps; a headless app
/// without session lifecycle gets the unscoped legacy mode.
pub fn spawn_symmetry_attunement(
    mut commands: ambition_platformer2d_shared_tangle::lifecycle::SessionCommands,
    existing: Query<&Encounter>,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
) {
    let Some(scope) = commands.spawn_scope() else {
        return;
    };
    if existing.iter().any(|enc| enc.id == SYMMETRY_ATTUNEMENT_ID) {
        return;
    }
    let mut lifecycle = EncounterLifecycle::default();
    if save.data().flag(SYMMETRY_ATTUNEMENT_FLAG) {
        lifecycle.apply_persisted(PersistedEncounterState::Cleared);
    }
    let mut entity = commands.spawn((
        Encounter::new(SYMMETRY_ATTUNEMENT_ID),
        ambition_platformer2d_shared_tangle::sim_id::SimId::encounter(SYMMETRY_ATTUNEMENT_ID),
        lifecycle,
        EncounterObjective::win(Objective::All(
            KERNEL_SIGNALS
                .iter()
                .map(|signal| Objective::ReceiveSignal((*signal).to_string()))
                .collect(),
        )),
        EncounterParticipants::default(),
    ));
    scope.apply_to(&mut entity);
}

/// Command EMITTER: entering the Noether Chamber starts the attunement.
///
/// The four switches now carry their own `on_activate` lines and the engine performs them
/// through the authored-command contract, so there is no adapter here at all — the level talks
/// to the encounter domain directly.
///
/// what is left is room ENTRY, which is a genuinely different shape: it is
/// a level-triggered condition on the active room rather than an edge on a
/// placement, and no authored surface expresses it yet. Naming that limit is
/// better than inventing a second one to hide it.
pub fn drive_symmetry_attunement(
    room_set: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d::world::rooms::RoomSet,
    >,
    encounters: Query<(&Encounter, &EncounterLifecycle)>,
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
}

/// Effect CONSUMER: the generic `Completed` event pays the puzzle out —
/// a celebration banner and the persistent save flag. No lifecycle authority
/// here; the reducer already decided.
pub fn celebrate_symmetry_attunement(
    mut events: MessageReader<EncounterEventMsg>,
    mut banners: MessageWriter<
        ambition_platformer2d_actor_monolith::features::GameplayBannerRequested,
    >,
    mut save: ResMut<ambition_persistence::save::AmbitionGameSave>,
) {
    for msg in events.read() {
        if msg.encounter == SYMMETRY_ATTUNEMENT_ID && matches!(msg.event, EncounterEvent::Completed)
        {
            banners.write(
                ambition_platformer2d_actor_monolith::features::GameplayBannerRequested::new(
                    "NOETHER ATTUNEMENT — every symmetry conserved".to_string(),
                    4.0,
                ),
            );
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
                .in_set(Platformer2dSimulationPhaseMonolith::GameplayEffects),
        );
        app.add_systems(
            sim,
            celebrate_symmetry_attunement
                .in_set(Platformer2dSimulationPhaseMonolith::Progression)
                .after(ambition_encounter::EncounterLifecycleSet),
        );
    }
}

#[cfg(test)]
mod tests;
