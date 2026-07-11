//! Boss-encounter presentation sink.
//!
//! `publish_events` fans an entity-local [`BossPhaseEvent`] out to the
//! presentation layer: a `PhaseChanged` drives the gameplay banner text + queues
//! the `boss_intro_<id>` cutscene, and a change INTO `Death` adds the victory
//! banner. Called by `systems` after every phase-machine tick.
//!
//! Music is deliberately NOT set here: `update_boss_encounters` owns the
//! adaptive-music request as a LEVEL-triggered lifetime (it re-derives the track
//! from the current phase every tick and clears it when no boss is fighting), so
//! an edge-triggered set here would only be overwritten the same tick. One music
//! authority, not two.

use crate::boss_encounter::{BossEncounterPhase, BossPhaseEvent};
use crate::cutscene_trigger::CutsceneTriggerQueue;

pub(super) fn publish_events(
    encounter_id: &str,
    event: &BossPhaseEvent,
    cutscene_queue: &mut CutsceneTriggerQueue,
    banner: &mut crate::features::GameplayBanner,
) {
    // Only the exposed phase change carries banner/cutscene; the brief
    // `TransitionLockStarted` tell has no presentation of its own.
    let BossPhaseEvent::PhaseChanged { to, .. } = event else {
        return;
    };
    if matches!(to, BossEncounterPhase::Intro) {
        cutscene_queue.request(format!("boss_intro_{encounter_id}"));
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
    // The victory banner supersedes the "DEFEATED" phase banner on a kill.
    if matches!(to, BossEncounterPhase::Death) {
        banner.show(format!("VICTORY: {encounter_id}"), 2.5);
    }
}
