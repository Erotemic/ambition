//! Dialogue typewriter SFX selection and throttling.
//!
//! The reusable dialogue runtime owns reveal cadence and generic fallback cues.
//! A game provider contributes speaker identities through
//! [`DialogueVoiceCatalog`] (`ambition_sfx::voice`), so this crate never names
//! a particular cast.

use ambition_sfx::voice::DialogueVoiceCatalog;
use ambition_sfx::{ids, SfxId};

use crate::runtime::DialogSpeechStyle;

/// How many newly-visible alphanumeric characters pass between normal
/// typewriter blips. Spaces and punctuation act like visual pauses and do not
/// advance the counter.
const TALK_BLIP_CHAR_INTERVAL: usize = 5;

/// Return true when a reveal tick should emit one dialogue blip.
///
/// `previous_visible_chars` and `visible_chars` are character counts into
/// `line`, not byte offsets. If a low framerate tick reveals many characters at
/// once, this still emits at most one blip for the frame.
pub(crate) fn should_play_talk_blip(
    line: &str,
    previous_visible_chars: usize,
    visible_chars: usize,
) -> bool {
    if visible_chars <= previous_visible_chars {
        return false;
    }
    let previous_voice_chars = voiced_char_count(line, previous_visible_chars);
    let visible_voice_chars = voiced_char_count(line, visible_chars);
    if visible_voice_chars <= previous_voice_chars {
        return false;
    }
    next_interval_index(previous_voice_chars) < next_interval_index(visible_voice_chars)
}

/// Resolve the talk-blip cue for a line.
///
/// Normal speech consults provider content and falls back to the generic blip.
/// Whisper and shout remain generic until a provider actually authors styled
/// voiceprints.
pub(crate) fn talk_blip_id_for_speaker(
    catalog: Option<&DialogueVoiceCatalog>,
    speaker_label: &str,
    dialogue_id: &str,
    style: DialogSpeechStyle,
) -> SfxId {
    match style {
        DialogSpeechStyle::Normal => catalog
            .and_then(|catalog| catalog.resolve(speaker_label, dialogue_id))
            .unwrap_or(ids::DIALOGUE_BLIP_GENERIC),
        DialogSpeechStyle::Whisper => ids::DIALOGUE_BLIP_WHISPER_GENERIC,
        DialogSpeechStyle::Shout => ids::DIALOGUE_BLIP_SHOUT_GENERIC,
    }
}

fn next_interval_index(count: usize) -> usize {
    count / TALK_BLIP_CHAR_INTERVAL
}

fn voiced_char_count(line: &str, visible_chars: usize) -> usize {
    line.chars()
        .take(visible_chars)
        .filter(|ch| ch.is_alphanumeric())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALPHA: SfxId = SfxId::from_static("test.dialogue.alpha");

    #[test]
    fn blip_throttle_ignores_spaces_and_punctuation() {
        assert!(!should_play_talk_blip("Hi, Sam!", 0, 4));
        assert!(should_play_talk_blip("Hello, Sam!", 0, 7));
        assert!(!should_play_talk_blip("Hello, Sam!", 7, 8));
    }

    #[test]
    fn unknown_and_styled_speech_use_generic_variants() {
        let mut catalog = DialogueVoiceCatalog::default();
        catalog.register_voiceprint(ALPHA, &["alpha"], &[]).unwrap();

        assert_eq!(
            talk_blip_id_for_speaker(Some(&catalog), "Mystery", "", DialogSpeechStyle::Normal,),
            ids::DIALOGUE_BLIP_GENERIC,
        );
        assert_eq!(
            talk_blip_id_for_speaker(Some(&catalog), "alpha", "", DialogSpeechStyle::Whisper,),
            ids::DIALOGUE_BLIP_WHISPER_GENERIC,
        );
        assert_eq!(
            talk_blip_id_for_speaker(None, "alpha", "", DialogSpeechStyle::Shout),
            ids::DIALOGUE_BLIP_SHOUT_GENERIC,
        );
    }
}
