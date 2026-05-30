use crate::presentation::cutscene::CutsceneTriggerQueue;

pub(super) fn publish_events(
    encounter_id: &str,
    events: &[crate::boss_encounter::BossEncounterEvent],
    music_request: &mut crate::encounter::BossEncounterMusicRequest,
    cutscene_queue: &mut CutsceneTriggerQueue,
    banner: &mut crate::features::GameplayBanner,
) {
    for event in events {
        match event {
            crate::boss_encounter::BossEncounterEvent::PhaseChanged { to, .. } => {
                if matches!(to, crate::boss_encounter::BossEncounterPhase::Intro) {
                    cutscene_queue.request(format!("boss_intro_{encounter_id}"));
                }
                let text = match to {
                    crate::boss_encounter::BossEncounterPhase::Intro => {
                        format!("BOSS APPROACHES — {encounter_id}")
                    }
                    crate::boss_encounter::BossEncounterPhase::Phase1 => "PHASE 1".to_string(),
                    crate::boss_encounter::BossEncounterPhase::Transition => "...".to_string(),
                    crate::boss_encounter::BossEncounterPhase::Phase2 => "PHASE 2".to_string(),
                    crate::boss_encounter::BossEncounterPhase::Stagger => {
                        "STAGGERED — punish".to_string()
                    }
                    crate::boss_encounter::BossEncounterPhase::Enrage => "ENRAGED".to_string(),
                    crate::boss_encounter::BossEncounterPhase::Death => "DEFEATED".to_string(),
                    crate::boss_encounter::BossEncounterPhase::Dormant => String::new(),
                };
                banner.show(text, 1.4);
            }
            crate::boss_encounter::BossEncounterEvent::MusicRequested { track } => {
                if !track.is_empty() {
                    music_request.desired_track = Some(track.clone());
                }
            }
            crate::boss_encounter::BossEncounterEvent::DamageApplied { .. } => {}
            crate::boss_encounter::BossEncounterEvent::Defeated => {
                // Death cutscene swap could go here in a richer build.
                banner.show(format!("VICTORY: {encounter_id}"), 2.5);
            }
        }
    }
}
