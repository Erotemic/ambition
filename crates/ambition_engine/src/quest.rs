//! Quest data types and progression rules.
//!
//! A quest is a fixed-order list of named steps. Progression is driven
//! by *advance events* (the sandbox feeds them in from gameplay):
//! talking to an NPC, defeating a boss, picking up an item, entering a
//! room. Each step declares the events that satisfy it; when the
//! current step's predicate is met, the quest advances.
//!
//! This module is intentionally Bevy-free so the same data can be
//! serialized into the save file (`PersistedQuest`) and used from
//! tests / headless / RL drivers.
//!
//! Failure paths (timed quests, mutually-exclusive choices, side
//! quests that gate later content) are not modeled yet — the quest
//! state machine only encodes the simplest "do these in order"
//! pattern. When real quests demand more, extend `QuestStepCondition`
//! with disjunction / failure variants.

use serde::{Deserialize, Serialize};

use crate::save::PersistedQuestState;

/// A single advance event the sandbox emits during gameplay.
///
/// String ids are used (not integer keys) so authoring can stay
/// data-driven. Keep the spelling in sync with the LDtk entity ids
/// and `EnemyRuntime::id` when matching.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuestAdvanceEvent {
    /// Player talked to an NPC with id `npc_id`.
    NpcTalked(String),
    /// Player picked up an item by id `item_id`.
    ItemCollected(String),
    /// Player defeated a named boss.
    BossDefeated(String),
    /// Player cleared a named encounter.
    EncounterCleared(String),
    /// A world flag flipped to on. Use this to wire one-shot quest
    /// triggers without inventing a new variant per condition.
    FlagSet(String),
    /// Player entered a named room.
    RoomEntered(String),
}

impl QuestAdvanceEvent {
    pub fn label(&self) -> String {
        match self {
            Self::NpcTalked(id) => format!("npc_talked:{id}"),
            Self::ItemCollected(id) => format!("item_collected:{id}"),
            Self::BossDefeated(id) => format!("boss_defeated:{id}"),
            Self::EncounterCleared(id) => format!("encounter_cleared:{id}"),
            Self::FlagSet(id) => format!("flag_set:{id}"),
            Self::RoomEntered(id) => format!("room_entered:{id}"),
        }
    }
}

/// What an event payload must look like for a step to advance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestStepCondition {
    NpcTalked(String),
    ItemCollected(String),
    BossDefeated(String),
    EncounterCleared(String),
    FlagSet(String),
    RoomEntered(String),
}

impl QuestStepCondition {
    /// True when `event` satisfies this step.
    pub fn matches(&self, event: &QuestAdvanceEvent) -> bool {
        matches!((self, event),
            (Self::NpcTalked(a), QuestAdvanceEvent::NpcTalked(b)) if a == b,
        ) || matches!((self, event),
            (Self::ItemCollected(a), QuestAdvanceEvent::ItemCollected(b)) if a == b,
        ) || matches!((self, event),
            (Self::BossDefeated(a), QuestAdvanceEvent::BossDefeated(b)) if a == b,
        ) || matches!((self, event),
            (Self::EncounterCleared(a), QuestAdvanceEvent::EncounterCleared(b)) if a == b,
        ) || matches!((self, event),
            (Self::FlagSet(a), QuestAdvanceEvent::FlagSet(b)) if a == b,
        ) || matches!((self, event),
            (Self::RoomEntered(a), QuestAdvanceEvent::RoomEntered(b)) if a == b,
        )
    }
}

/// One step in a quest. The `description` is shown in the quest log;
/// `condition` decides when this step is satisfied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestStepSpec {
    pub description: String,
    pub condition: QuestStepCondition,
}

impl QuestStepSpec {
    pub fn new(description: impl Into<String>, condition: QuestStepCondition) -> Self {
        Self {
            description: description.into(),
            condition,
        }
    }
}

/// A complete quest: title, summary shown in the menu, and ordered
/// steps. Quests advance one step at a time; once the last step's
/// condition fires, the quest moves to `Completed`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestSpec {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub steps: Vec<QuestStepSpec>,
}

impl QuestSpec {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        summary: impl Into<String>,
        steps: Vec<QuestStepSpec>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: summary.into(),
            steps,
        }
    }
}

/// Live quest state — what the runtime mutates as the player progresses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestState {
    pub spec: QuestSpec,
    pub progression: PersistedQuestState,
    /// Index of the active step. Valid while `progression == InProgress`.
    /// For `Completed`/`Failed` it's clamped to `steps.len()`; for
    /// `NotStarted` it's 0.
    pub step: u8,
}

impl QuestState {
    pub fn new(spec: QuestSpec) -> Self {
        Self {
            spec,
            progression: PersistedQuestState::NotStarted,
            step: 0,
        }
    }

    /// Hydrate from the save resource. Out-of-range step values clamp
    /// to the last step so a corrupt save doesn't crash the runtime.
    pub fn apply_persisted(&mut self, state: PersistedQuestState, step: u8) {
        let max = self.spec.steps.len() as u8;
        self.progression = state;
        self.step = step.min(max.saturating_sub(1));
        if matches!(
            state,
            PersistedQuestState::Completed | PersistedQuestState::Failed
        ) {
            self.step = max.saturating_sub(1);
        }
    }

    /// Begin the quest. No-op if it's already started or finished.
    pub fn start(&mut self) -> bool {
        if !matches!(self.progression, PersistedQuestState::NotStarted) {
            return false;
        }
        if self.spec.steps.is_empty() {
            self.progression = PersistedQuestState::Completed;
        } else {
            self.progression = PersistedQuestState::InProgress;
            self.step = 0;
        }
        true
    }

    /// Try to advance the quest with one event. Returns true when the
    /// event consumed a step (caller should refresh the save).
    pub fn try_advance(&mut self, event: &QuestAdvanceEvent) -> bool {
        if !matches!(self.progression, PersistedQuestState::InProgress) {
            return false;
        }
        let Some(step) = self.spec.steps.get(self.step as usize) else {
            return false;
        };
        if !step.condition.matches(event) {
            return false;
        }
        let next = self.step.saturating_add(1);
        if (next as usize) >= self.spec.steps.len() {
            self.progression = PersistedQuestState::Completed;
            self.step = self.spec.steps.len().saturating_sub(1) as u8;
        } else {
            self.step = next;
        }
        true
    }

    pub fn current_step(&self) -> Option<&QuestStepSpec> {
        self.spec.steps.get(self.step as usize)
    }

    pub fn is_active(&self) -> bool {
        matches!(self.progression, PersistedQuestState::InProgress)
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.progression, PersistedQuestState::Completed)
    }

    pub fn hud_summary(&self) -> String {
        match self.progression {
            PersistedQuestState::NotStarted => format!("[ ] {}", self.spec.title),
            PersistedQuestState::InProgress => match self.current_step() {
                Some(step) => format!("[…] {} — {}", self.spec.title, step.description),
                None => format!("[…] {}", self.spec.title),
            },
            PersistedQuestState::Completed => format!("[✓] {}", self.spec.title),
            PersistedQuestState::Failed => format!("[✗] {}", self.spec.title),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_steps_quest() -> QuestSpec {
        QuestSpec::new(
            "first_steps",
            "First Steps",
            "Find your bearings in the hub.",
            vec![
                QuestStepSpec::new(
                    "Talk to the warden in the hub.",
                    QuestStepCondition::NpcTalked("hub_warden".into()),
                ),
                QuestStepSpec::new(
                    "Clear the mob lab.",
                    QuestStepCondition::EncounterCleared("mob_lab".into()),
                ),
                QuestStepSpec::new(
                    "Defeat the gradient sentinel.",
                    QuestStepCondition::BossDefeated("gradient_sentinel".into()),
                ),
            ],
        )
    }

    #[test]
    fn cannot_advance_before_start() {
        let mut quest = QuestState::new(first_steps_quest());
        assert!(!quest.try_advance(&QuestAdvanceEvent::NpcTalked("hub_warden".into())));
        assert_eq!(quest.progression, PersistedQuestState::NotStarted);
    }

    #[test]
    fn started_quest_advances_through_each_step_in_order() {
        let mut quest = QuestState::new(first_steps_quest());
        assert!(quest.start());
        assert!(quest.is_active());
        assert_eq!(quest.step, 0);

        // Wrong event: no advance.
        assert!(!quest.try_advance(&QuestAdvanceEvent::NpcTalked("librarian".into())));
        assert_eq!(quest.step, 0);

        assert!(quest.try_advance(&QuestAdvanceEvent::NpcTalked("hub_warden".into())));
        assert_eq!(quest.step, 1);

        assert!(quest.try_advance(&QuestAdvanceEvent::EncounterCleared("mob_lab".into())));
        assert_eq!(quest.step, 2);

        assert!(quest.try_advance(&QuestAdvanceEvent::BossDefeated("gradient_sentinel".into())));
        assert!(quest.is_complete());
    }

    #[test]
    fn completed_quest_does_not_re_advance() {
        let mut quest = QuestState::new(QuestSpec::new(
            "trivial",
            "Trivial",
            "summary",
            vec![QuestStepSpec::new(
                "go",
                QuestStepCondition::FlagSet("done".into()),
            )],
        ));
        quest.start();
        quest.try_advance(&QuestAdvanceEvent::FlagSet("done".into()));
        assert!(quest.is_complete());
        // A second matching event must not cycle the state.
        assert!(!quest.try_advance(&QuestAdvanceEvent::FlagSet("done".into())));
        assert!(quest.is_complete());
    }

    #[test]
    fn empty_quest_completes_immediately_on_start() {
        let mut quest = QuestState::new(QuestSpec::new("empty", "Empty", "", vec![]));
        quest.start();
        assert!(quest.is_complete());
    }

    #[test]
    fn apply_persisted_clamps_step_index() {
        let mut quest = QuestState::new(first_steps_quest());
        quest.apply_persisted(PersistedQuestState::InProgress, 99);
        assert_eq!(quest.step, 2);
    }

    #[test]
    fn quest_advance_event_label_uses_kind_prefix() {
        assert_eq!(
            QuestAdvanceEvent::NpcTalked("guide".into()).label(),
            "npc_talked:guide"
        );
        assert_eq!(
            QuestAdvanceEvent::BossDefeated("sentinel".into()).label(),
            "boss_defeated:sentinel"
        );
        assert_eq!(
            QuestAdvanceEvent::FlagSet("met_any_hub_npc".into()).label(),
            "flag_set:met_any_hub_npc"
        );
    }

    #[test]
    fn step_condition_matches_only_compatible_event_kinds() {
        let cond = QuestStepCondition::NpcTalked("guide".into());
        assert!(cond.matches(&QuestAdvanceEvent::NpcTalked("guide".into())));
        // Different kind: no match.
        assert!(!cond.matches(&QuestAdvanceEvent::FlagSet("guide".into())));
        // Same kind, different id: no match.
        assert!(!cond.matches(&QuestAdvanceEvent::NpcTalked("librarian".into())));
    }

    #[test]
    fn cannot_start_already_started_quest() {
        let mut quest = QuestState::new(first_steps_quest());
        assert!(quest.start());
        // Second start is a no-op.
        assert!(!quest.start());
    }
}
