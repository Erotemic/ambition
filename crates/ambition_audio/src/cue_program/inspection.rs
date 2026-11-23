use super::*;
use crate::music_policy::{MusicExitRule, MusicTimingRequirement};
use serde_json::{Value, json};

impl PreparedMusicCues {
    /// Join game policy with the selected producer facts. Decode and native admission remain separate.
    pub fn inspect(&self) -> Value {
        let cues: Vec<_> = self.cues().map(|cue| {
            let sections: Vec<_> = cue.sections.iter().map(|section| json!({
                "prepared": section,
                "end": if section.musical_duration_seconds.is_some() { "MusicalEnd" } else { "AssetEnd" },
                "required_voices": section.sources.len(),
                "section_exit_wait_bound_seconds": wait_bound(section, &cue.playback.section_exit),
                "ending_exit_wait_bound_seconds": wait_bound(section, &cue.playback.ending_exit),
            })).collect();
            json!({
                "id": cue.id, "media_identity": cue.media_identity, "playback": cue.playback,
                "sections": sections, "states": cue.states,
                "outro_state": cue.outro_state, "ending_wait_gains": cue.ending_wait_gains,
            })
        }).collect();
        json!({
            "cues": cues,
            "ungenerated": self.ungenerated,
            "encounter_bindings": self.encounter_bindings,
            "room_bindings": self.room_bindings,
            "boss_bindings": self.boss_bindings,
            "room_phase_tracks": self.room_phase_tracks,
            "supported_timing": [MusicTimingRequirement::FrameDriven],
            "execution": {
                "clock": "IndependentPlayback",
                "scene_pause": "Continue",
                "scene_scale": "KeepPlaybackRate",
                "entry": "SectionStart",
                "loop_release": "WholeProductFade",
                "ending_wait_mix": "GainsOnly",
            },
            "unsupported_operations": [
                "ScheduledPickup", "IndependentLoopTail", "BridgeOverlay", "JoinScenePosition",
            ],
            "maximum_voices_per_group": MAX_MUSIC_LAYERS,
            "maximum_overlap_groups": crate::music_policy::MAX_MUSIC_PLAYBACK_GROUPS,
            "readiness": "Prepared definitions do not prove decode, voice admission, or speaker output.",
            "wait_bounds": "Musical bounds exclude preparation and frame observation latency. Null requires decode facts or has no exit.",
        })
    }
}

fn wait_bound(section: &PreparedMusicSection, exit: &MusicExitRule) -> Option<f64> {
    if !section.looped && !matches!(exit, MusicExitRule::Immediate) {
        return section.file_duration_seconds;
    }
    let pass = section
        .looped
        .then_some(
            section
                .file_duration_seconds
                .or(section.musical_duration_seconds),
        )
        .flatten();
    match exit {
        MusicExitRule::Immediate => Some(0.0),
        MusicExitRule::AssetEnd => section.file_duration_seconds,
        MusicExitRule::MusicalEnd => section.musical_duration_seconds,
        MusicExitRule::Markers(ids) => {
            let rate = section.sources.first()?.product.as_ref()?.sample_rate;
            let mut exits: Vec<_> = section
                .markers
                .iter()
                .filter(|marker| {
                    ids.contains(&marker.id)
                        && marker.role == crate::music_media::MusicMarkerRole::Exit
                })
                .map(|marker| marker.frame as f64 / f64::from(rate))
                .collect();
            exits.sort_by(f64::total_cmp);
            let first = *exits.first()?;
            let edge = pass.map_or(first, |period| {
                period - exits.last().copied().unwrap() + first
            });
            Some(
                exits
                    .windows(2)
                    .map(|pair| pair[1] - pair[0])
                    .fold(edge, f64::max),
            )
        }

        MusicExitRule::Bars(bars) => match &section.clock {
            PreparedMusicClock::Unclocked => None,
            PreparedMusicClock::Constant {
                seconds_per_bar,
                frames,
                first_bar_seconds,
                ..
            } => {
                let quantum = frames.map_or(0.0, |origin| 1.0 / f64::from(origin.sample_rate));
                let period = (seconds_per_bar * *bars as f64 + quantum).max(*first_bar_seconds);
                Some(pass.map_or(period, |pass| period.min(pass)))
            }
            PreparedMusicClock::Boundaries { bar_seconds } => {
                let mut previous = 0.0;
                let mut bound: f64 = 0.0;
                for exit in bar_seconds.iter().step_by(*bars).copied().chain(pass) {
                    bound = bound.max(exit - previous);
                    previous = exit;
                }
                Some(bound)
            }
        },
    }
}
