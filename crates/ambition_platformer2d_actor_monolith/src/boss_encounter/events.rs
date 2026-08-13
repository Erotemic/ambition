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

/// **A boss's exposed phase changed, announced by the system that committed the
/// change.** (P0.2)
///
/// ⛔⛔ **the edge must come from here and nowhere else.**
/// `boss_phase_transition_feedback` used to re-derive it, diffing each boss's
/// current phase against a `Local<HashMap<String, BossEncounterPhase>>`. A
/// `Local` is not rollback state: it is not restored when the host rewinds. So
/// after a rollback the map still held the phase the abandoned pass reached, the
/// re-simulated frame's diff came out EMPTY, and the transition's gameplay
/// consequence — a `DamageBox` shockwave the player is meant to dodge — was
/// silently lost on the timeline the session actually settled on. The mirror
/// failure is available too: any ordering that leaves the map holding an older
/// phase manufactures a transition that never happened.
///
/// ⭐ **and the authority already existed.** `ActorPhaseState::tick` returns
/// `BossPhaseEvent::PhaseChanged { from, to }` at the moment it commits the swap;
/// `update_boss_encounters` was already fanning that event to `publish_events`
/// for the banner and the cutscene. The feedback system was the one consumer
/// reconstructing what it had been handed. This message carries it instead.
///
/// ⚠ **written and read in the SAME frame**, by systems in the same sim schedule
/// (`ProgressionSet::BossAdvance` → `BossHazards`). That is what makes it
/// rollback-correct without being rollback state: a re-simulation re-runs the
/// phase machine, which re-produces the event from restored authoritative state
/// if and only if the corrected timeline really crosses the threshold. A message
/// held ACROSS frames would be the opposite — cross-frame simulation truth that
/// rollback wipes.
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
