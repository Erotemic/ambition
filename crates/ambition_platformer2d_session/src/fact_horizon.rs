//! The world facts' checkpoint (Q164): the save's flags and quest steps go
//! back with a death as the bag does, so a fact never stays while the object
//! it came with is lost.
//!
//! The baseline is taken where the bag's is: at each checkpoint commit, and
//! when a session's save is applied (the facts a session begins with, which a
//! death before any checkpoint returns to). A New Game's wipe makes the fresh
//! save's facts the baseline (`reset::begin_fresh_run`).
//!
//! A restore pins the baseline only when it spares no participant: in co-op a
//! death never rolls a shared fact back behind a survivor (Q151, Q164). Dialogue
//! visits are not facts here: what was seen is kept apart from what it gave
//! (Q134).

use bevy::prelude::*;

use ambition_persistence::save::AmbitionGameSave;
use ambition_persistence::save_data::{AmbitionGameSaveData, PersistedFlag, PersistedQuest};
use ambition_platformer2d_shared_tangle::lifecycle::{
    CheckpointCommitted, SaveRestored, SessionWorldMut,
};

/// The flags and quest steps a save records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CheckpointFacts {
    flags: Vec<PersistedFlag>,
    quests: Vec<PersistedQuest>,
}

impl CheckpointFacts {
    /// The facts `save` records now.
    pub fn of(save: &AmbitionGameSaveData) -> Self {
        Self { flags: save.flags().to_vec(), quests: save.quests().to_vec() }
    }

    /// Put `save`'s flags and quest steps back to these, one row at a time
    /// through the save's own setters. Visits, encounters, bosses and every
    /// other record are left as they are.
    pub fn restore_into(&self, save: &mut AmbitionGameSaveData) {
        let set: Vec<String> = save.flags().iter().map(|flag| flag.id.clone()).collect();
        for id in set {
            if !self.flags.iter().any(|flag| flag.id == id) {
                save.set_flag(id, false);
            }
        }
        for flag in &self.flags {
            save.set_flag(flag.id.clone(), flag.on);
        }
        let recorded: Vec<String> = save.quests().iter().map(|quest| quest.id.clone()).collect();
        for id in recorded {
            if !self.quests.iter().any(|quest| quest.id == id) {
                save.set_quest(id, Default::default(), 0);
            }
        }
        for quest in &self.quests {
            save.set_quest(quest.id.clone(), quest.state, quest.step);
        }
    }

    /// Whether `save` records these facts: the same flags on, the same quests
    /// at the same steps, in any order.
    pub fn recorded_in(&self, save: &AmbitionGameSaveData) -> bool {
        fn on(flags: &[PersistedFlag]) -> Vec<&str> {
            let mut ids: Vec<&str> = flags.iter().filter(|flag| flag.on).map(|flag| flag.id.as_str()).collect();
            ids.sort_unstable();
            ids
        }
        fn steps(quests: &[PersistedQuest]) -> Vec<(&str, u8, u8)> {
            let mut rows: Vec<(&str, u8, u8)> =
                quests.iter().map(|quest| (quest.id.as_str(), quest.state as u8, quest.step)).collect();
            rows.sort_unstable();
            rows
        }
        on(&self.flags) == on(save.flags()) && steps(&self.quests) == steps(save.quests())
    }

    /// Entity-free value projection, in sorted order.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::{checksum_bytes, put_str, put_u8, put_u64};
        let mut flags: Vec<&PersistedFlag> = self.flags.iter().collect();
        flags.sort_by(|a, b| a.id.cmp(&b.id));
        let mut quests: Vec<&PersistedQuest> = self.quests.iter().collect();
        quests.sort_by(|a, b| a.id.cmp(&b.id));
        let mut bytes = Vec::new();
        put_u64(&mut bytes, flags.len() as u64);
        for flag in flags {
            put_str(&mut bytes, &flag.id);
            put_u8(&mut bytes, u8::from(flag.on));
        }
        put_u64(&mut bytes, quests.len() as u64);
        for quest in quests {
            put_str(&mut bytes, &quest.id);
            put_u8(&mut bytes, quest.state as u8);
            put_u8(&mut bytes, quest.step);
        }
        checksum_bytes(&bytes)
    }
}

/// The facts at the last committed checkpoint, or when the session's save was
/// applied. `None` until one of those happened: a restore then pins no facts,
/// as before this domain.
///
/// Rollback state with a real value: a commit changes it mid-frame.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct FactsBaseline(Option<CheckpointFacts>);

impl FactsBaseline {
    pub fn remembered(&self) -> Option<&CheckpointFacts> {
        self.0.as_ref()
    }

    pub fn adopt(&mut self, facts: CheckpointFacts) {
        if self.0.as_ref() != Some(&facts) {
            self.0 = Some(facts);
        }
    }

    pub fn checksum(&self) -> u64 {
        self.0.as_ref().map_or(0, |facts| facts.checksum() | 1)
    }
}

/// The facts one committed restore puts back. Installed for the duration of
/// `CheckpointDomainApply` and removed after, as the other domains' inputs are.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct FactCheckpointRestoreInputs(pub CheckpointFacts);

/// Take the baseline on a checkpoint commit.
pub fn capture_facts_baseline(
    mut commits: MessageReader<CheckpointCommitted>,
    save: Option<Res<AmbitionGameSave>>,
    baseline: Option<SessionWorldMut<FactsBaseline>>,
) {
    let committed = commits.read().count() > 0;
    let (Some(save), Some(mut baseline)) = (save, baseline) else {
        return;
    };
    if committed {
        baseline.adopt(CheckpointFacts::of(save.data()));
    }
}

/// Take the baseline from the save a session applies: the loaded file, or
/// the empty save of a first run. Runs while the save waits to be applied.
pub fn adopt_facts_baseline_from_save(
    restored: Res<SaveRestored>,
    save: Res<AmbitionGameSave>,
    baseline: Option<SessionWorldMut<FactsBaseline>>,
) {
    if restored.0 {
        return;
    }
    if let Some(mut baseline) = baseline {
        baseline.adopt(CheckpointFacts::of(save.data()));
    }
}

/// Put the facts back (checkpoint reducer, in `CheckpointDomainApply`), and
/// the quest registry with them, by the rule a session builds it by.
pub fn restore_facts_to_checkpoint(
    inputs: Option<Res<FactCheckpointRestoreInputs>>,
    save: Option<ResMut<AmbitionGameSave>>,
    quests: Option<ResMut<ambition_persistence::quest::QuestRegistry>>,
) {
    let (Some(inputs), Some(mut save)) = (inputs, save) else {
        return;
    };
    if inputs.0.recorded_in(save.data()) {
        return;
    }
    inputs.0.restore_into(save.data_mut());
    if let Some(mut quests) = quests {
        quests.rehydrate(save.data());
    }
}

/// Install the fact horizon: its baseline on the session root, its capture
/// and adoption, and its reducer.
pub fn install_fact_horizon(app: &mut App) {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        require_on_session_root, CheckpointCapture, CheckpointDomainApply, DurableRestoreSet,
    };
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;
    let sim = app.sim_schedule();
    require_on_session_root::<FactsBaseline>(app);
    app.add_systems(
        sim,
        (
            capture_facts_baseline
                .run_if(ambition_platformer2d_core::a_message_waits::<CheckpointCommitted>)
                .in_set(CheckpointCapture),
            adopt_facts_baseline_from_save.in_set(DurableRestoreSet::Domains),
        ),
    )
    .add_systems(CheckpointDomainApply, restore_facts_to_checkpoint);
}
