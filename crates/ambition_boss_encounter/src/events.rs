//! Boss-encounter presentation sink.
//!
//! `publish_events` sends an entity-local [`BossPhaseEvent`] to the
//! presentation layer: a `PhaseChanged` sets the gameplay banner text and
//! queues the `boss_intro_<id>` cutscene, and a change into `Death` adds the
//! victory banner. Called by `systems` after every phase-machine tick.
//!
//! Music is not set here. `update_boss_encounters` owns the adaptive-music
//! request as a level-triggered lifetime (it re-derives the track from the
//! current phase every tick and clears it when no boss is fighting), so an
//! edge-triggered set here would be overwritten the same tick.

use crate::{BossEncounterPhase, BossPhaseEvent};
use ambition_cutscene::CutsceneTriggerQueue;

/// Same-frame notification that a boss phase transition was committed.
///
/// Produced by the rollback-owned phase machine and consumed later in the same
/// simulation frame (`BossAdvance` -> `BossHazards`). It must not be retained as
/// cross-frame state; resimulation reproduces it from restored phase authority.
#[derive(bevy::ecs::message::Message, Clone, Copy, Debug)]
pub struct BossPhaseChanged {
    /// The boss whose phase changed.
    pub boss: bevy::prelude::Entity,
    pub from: BossEncounterPhase,
    pub to: BossEncounterPhase,
}

pub(super) fn publish_events(
    encounter_id: &str,
    event: &BossPhaseEvent,
    cutscene_queue: Option<&mut CutsceneTriggerQueue>,
    banner: &mut ambition_combat::GameplayBanner,
) {
    // Only the exposed phase change carries banner/cutscene; the brief
    // `TransitionLockStarted` tell has no presentation of its own.
    let BossPhaseEvent::PhaseChanged { to, .. } = event else {
        return;
    };
    if let (BossEncounterPhase::Intro, Some(queue)) = (to, cutscene_queue) {
        queue.request(format!("boss_intro_{encounter_id}"));
    }
    let text = match to {
        BossEncounterPhase::Intro => format!("BOSS APPROACHES — {encounter_id}"),
        BossEncounterPhase::Phase1 => "PHASE 1".to_string(),
        BossEncounterPhase::Transition => "...".to_string(),
        BossEncounterPhase::Phase2 => "PHASE 2".to_string(),
        BossEncounterPhase::Stagger => "STAGGERED — punish".to_string(),
        BossEncounterPhase::Enrage => "ENRAGED".to_string(),
        BossEncounterPhase::Death => "DEFEATED".to_string(),
        BossEncounterPhase::Dormant => String::new(),
    };
    banner.show(text, 1.4);
    // The victory banner replaces the "DEFEATED" phase banner on a kill.
    if matches!(to, BossEncounterPhase::Death) {
        banner.show(format!("VICTORY: {encounter_id}"), 2.5);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A boss that enters its intro requests `boss_intro_<id>`, and only then.
    /// A composition without cutscenes has no queue: the intro still shows its
    /// banner (`world-without-cutscenes` profile).
    #[test]
    fn an_intro_requests_its_cutscene_where_there_is_a_queue() {
        let intro = BossPhaseEvent::PhaseChanged { from: BossEncounterPhase::Dormant, to: BossEncounterPhase::Intro };
        let phase1 = BossPhaseEvent::PhaseChanged { from: BossEncounterPhase::Intro, to: BossEncounterPhase::Phase1 };
        let mut queue = CutsceneTriggerQueue::default();
        let mut banner = ambition_combat::GameplayBanner::default();
        publish_events("warden", &intro, Some(&mut queue), &mut banner);
        publish_events("warden", &phase1, Some(&mut queue), &mut banner);
        assert_eq!(queue.0, vec!["boss_intro_warden".to_string()], "the intro's request, and not phase 1's");

        let mut banner = ambition_combat::GameplayBanner::default();
        publish_events("warden", &intro, None, &mut banner);
        assert_eq!(banner.text, "BOSS APPROACHES — warden", "with no queue the intro still shows its banner");
    }
}
