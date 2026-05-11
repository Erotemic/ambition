use ambition_engine as ae;

use crate::cutscene::CutsceneTriggerQueue;

pub(super) fn publish_events(
    encounter_id: &str,
    events: &[ae::BossEncounterEvent],
    music_request: &mut crate::encounter::EncounterMusicRequest,
    cutscene_queue: &mut CutsceneTriggerQueue,
    features: &mut crate::features::FeatureRuntime,
) {
    for event in events {
        match event {
            ae::BossEncounterEvent::PhaseChanged { to, .. } => {
                if matches!(to, ae::BossEncounterPhase::Intro) {
                    cutscene_queue.request(format!("boss_intro_{encounter_id}"));
                }
                features.banner = match to {
                    ae::BossEncounterPhase::Intro => format!("BOSS APPROACHES — {encounter_id}"),
                    ae::BossEncounterPhase::Phase1 => "PHASE 1".to_string(),
                    ae::BossEncounterPhase::Transition => "...".to_string(),
                    ae::BossEncounterPhase::Phase2 => "PHASE 2".to_string(),
                    ae::BossEncounterPhase::Stagger => "STAGGERED — punish".to_string(),
                    ae::BossEncounterPhase::Enrage => "ENRAGED".to_string(),
                    ae::BossEncounterPhase::Death => "DEFEATED".to_string(),
                    ae::BossEncounterPhase::Dormant => String::new(),
                };
                features.banner_timer = 1.4;
            }
            ae::BossEncounterEvent::MusicRequested { track } => {
                if !track.is_empty() {
                    music_request.desired_track = Some(track.clone());
                }
            }
            ae::BossEncounterEvent::DamageApplied { .. } => {}
            ae::BossEncounterEvent::Defeated => {
                // Death cutscene swap could go here in a richer build.
                features.banner = format!("VICTORY: {encounter_id}");
                features.banner_timer = 2.5;
            }
        }
    }
}
