//! Dialogue typewriter SFX selection and throttling.
//!
//! Dialogue SFX are deliberately data-id driven: gameplay emits an open-ended
//! [`SfxMessage::Play`](ambition_sfx::SfxMessage::Play) with an authored cue id,
//! and the bank decides whether that id exists. This module keeps the selection
//! rules pure so the reveal system stays small and tests do not need Bevy audio.

use ambition_sfx::ids;
use ambition_sfx::SfxId;

use super::runtime::DialogSpeechStyle;

/// How many newly-visible alphanumeric characters pass between normal
/// typewriter blips. Spaces and punctuation act like visual pauses and do not
/// advance the counter.
const TALK_BLIP_CHAR_INTERVAL: usize = 5;

/// Return true when a reveal tick should emit one dialogue blip.
///
/// `previous_visible_chars` and `visible_chars` are character counts into
/// `line`, not byte offsets. If a low framerate tick reveals many characters at
/// once, this still emits at most one blip for the frame.
pub(in crate::dialog) fn should_play_talk_blip(
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
/// Normal speech prefers a speaker-specific voiceprint and falls back to
/// `dialogue.blip.generic`. Whisper/shout markup use the generic styled blips
/// today because there are not yet per-speaker styled voiceprints in the bank;
/// the function is shaped so those can be added later without changing the
/// reveal system.
pub(in crate::dialog) fn talk_blip_id_for_speaker(
    speaker_label: &str,
    dialogue_id: &str,
    style: DialogSpeechStyle,
) -> SfxId {
    match style {
        DialogSpeechStyle::Whisper => {
            styled_blip_for_speaker(speaker_label, dialogue_id, DialogSpeechStyle::Whisper)
                .unwrap_or(ids::DIALOGUE_BLIP_WHISPER_GENERIC)
        }
        DialogSpeechStyle::Shout => {
            styled_blip_for_speaker(speaker_label, dialogue_id, DialogSpeechStyle::Shout)
                .unwrap_or(ids::DIALOGUE_BLIP_SHOUT_GENERIC)
        }
        DialogSpeechStyle::Normal => normal_blip_for_speaker(speaker_label, dialogue_id)
            .unwrap_or(ids::DIALOGUE_BLIP_GENERIC),
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

fn styled_blip_for_speaker(
    _speaker_label: &str,
    _dialogue_id: &str,
    _style: DialogSpeechStyle,
) -> Option<SfxId> {
    // No per-speaker whisper/shout typewriter voiceprints are authored yet.
    // Keep this seam explicit so a future `dialogue.blip.whisper.oiler` or
    // `dialogue.blip.shout.creator` can be added here without touching the
    // reveal tick.
    None
}

fn normal_blip_for_speaker(speaker_label: &str, dialogue_id: &str) -> Option<SfxId> {
    let speaker = normalized_key(speaker_label);
    let dialogue = normalized_key(dialogue_id);
    let key = if speaker.is_empty() {
        &dialogue
    } else {
        &speaker
    };

    exact_normal_blip(key)
        .or_else(|| alias_normal_blip(key))
        .or_else(|| exact_normal_blip(&dialogue))
        .or_else(|| alias_normal_blip(&dialogue))
}

fn exact_normal_blip(key: &str) -> Option<SfxId> {
    Some(match key {
        "alice" | "hall_npc_alice" => ids::DIALOGUE_BLIP_ALICE,
        "architect" | "architect_npc" | "hall_architect" => ids::DIALOGUE_BLIP_ARCHITECT,
        "bob" | "hall_npc_bob" => ids::DIALOGUE_BLIP_BOB,
        "creator" | "creator_final" | "hall_npc_creator" | "hall_npc_creator_final" => {
            ids::DIALOGUE_BLIP_CREATOR
        }
        "dark_lord" | "hall_npc_dark_lord" => ids::DIALOGUE_BLIP_DARK_LORD,
        "gate_janitor" | "hall_npc_gate_janitor" => ids::DIALOGUE_BLIP_GATE_JANITOR,
        "goblin_chieftain" | "fretjaw" | "fretjaw_cantina_chieftain" => {
            ids::DIALOGUE_BLIP_GOBLIN_CHIEFTAIN
        }
        "hand_saint" => ids::DIALOGUE_BLIP_HAND_SAINT,
        "kernel_guide" | "kernel_guide_npc" | "hall_npc_kernel_guide" => {
            ids::DIALOGUE_BLIP_KERNEL_GUIDE
        }
        "manifest_clerk" | "hall_npc_manifest_clerk" => ids::DIALOGUE_BLIP_MANIFEST_CLERK,
        "merchant_prototype" | "merchant_prototype_npc" | "hall_npc_merchant_prototype" => {
            ids::DIALOGUE_BLIP_MERCHANT_PROTOTYPE
        }
        "military_general" | "general" | "general_hero" | "hall_npc_general" => {
            ids::DIALOGUE_BLIP_MILITARY_GENERAL
        }
        "news_board" | "drain_market_bulletin" => ids::DIALOGUE_BLIP_NEWS_BOARD,
        "ninja" => ids::DIALOGUE_BLIP_NINJA,
        "oiler" | "hall_npc_oiler" => ids::DIALOGUE_BLIP_OILER,
        "pirate" | "pirate_admiral" | "admiral" | "hall_pirate_admiral" => {
            ids::DIALOGUE_BLIP_PIRATE
        }
        "pulse_voyager" | "captain_pulse" | "hall_npc_pulse_voyager_captain" => {
            ids::DIALOGUE_BLIP_PULSE_VOYAGER
        }
        "robot" | "hall_robot" => ids::DIALOGUE_BLIP_ROBOT,
        "tech_bro" | "chadwick_iii" | "chadwick_disruptor_iii" => ids::DIALOGUE_BLIP_TECH_BRO,
        "vault_keeper" | "vault_keeper_npc" | "hall_npc_vault_keeper" => {
            ids::DIALOGUE_BLIP_VAULT_KEEPER
        }
        "weird_hermit" => ids::DIALOGUE_BLIP_WEIRD_HERMIT,
        _ => return None,
    })
}

fn alias_normal_blip(key: &str) -> Option<SfxId> {
    if key.is_empty() {
        return None;
    }
    if key.contains("alice") {
        return Some(ids::DIALOGUE_BLIP_ALICE);
    }
    if key.contains("architect") {
        return Some(ids::DIALOGUE_BLIP_ARCHITECT);
    }
    if key.contains("bob") {
        return Some(ids::DIALOGUE_BLIP_BOB);
    }
    if key.contains("creator") {
        return Some(ids::DIALOGUE_BLIP_CREATOR);
    }
    if key.contains("dark_lord") {
        return Some(ids::DIALOGUE_BLIP_DARK_LORD);
    }
    if key.contains("gate_janitor") {
        return Some(ids::DIALOGUE_BLIP_GATE_JANITOR);
    }
    if key.contains("goblin") || key.contains("chieftain") || key.contains("fretjaw") {
        return Some(ids::DIALOGUE_BLIP_GOBLIN_CHIEFTAIN);
    }
    if key.contains("hand_saint") {
        return Some(ids::DIALOGUE_BLIP_HAND_SAINT);
    }
    if key.contains("kernel") {
        return Some(ids::DIALOGUE_BLIP_KERNEL_GUIDE);
    }
    if key.contains("manifest_clerk") {
        return Some(ids::DIALOGUE_BLIP_MANIFEST_CLERK);
    }
    if key.contains("merchant") {
        return Some(ids::DIALOGUE_BLIP_MERCHANT_PROTOTYPE);
    }
    if key.contains("general") {
        return Some(ids::DIALOGUE_BLIP_MILITARY_GENERAL);
    }
    if key.contains("news") || key.contains("bulletin") {
        return Some(ids::DIALOGUE_BLIP_NEWS_BOARD);
    }
    if key.contains("ninja")
        || key.contains("shadow")
        || key.contains("oni")
        || key.contains("duelist")
    {
        return Some(ids::DIALOGUE_BLIP_NINJA);
    }
    if key.contains("oiler") {
        return Some(ids::DIALOGUE_BLIP_OILER);
    }
    if key.contains("pirate")
        || key.contains("admiral")
        || key.contains("quartermaster")
        || key.contains("navigator")
        || key.contains("lookout")
        || key.contains("raider")
        || key.contains("broadside")
        || key.contains("iron_mary")
        || key.contains("salt_annet")
    {
        return Some(ids::DIALOGUE_BLIP_PIRATE);
    }
    if key.contains("pulse") {
        return Some(ids::DIALOGUE_BLIP_PULSE_VOYAGER);
    }
    if key.contains("robot")
        || key.contains("automaton")
        || key.contains("synthetic")
        || key.contains("smart_house")
    {
        return Some(ids::DIALOGUE_BLIP_ROBOT);
    }
    if key.contains("tech") || key.contains("chadwick") {
        return Some(ids::DIALOGUE_BLIP_TECH_BRO);
    }
    if key.contains("vault_keeper") {
        return Some(ids::DIALOGUE_BLIP_VAULT_KEEPER);
    }
    if key.contains("weird_hermit") || key.contains("hermit") {
        return Some(ids::DIALOGUE_BLIP_WEIRD_HERMIT);
    }
    None
}

fn normalized_key(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut last_was_sep = true;
    for ch in raw.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            last_was_sep = false;
        } else if !last_was_sep {
            out.push('_');
            last_was_sep = true;
        }
    }
    if out.ends_with('_') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blip_throttle_ignores_spaces_and_punctuation() {
        assert!(!should_play_talk_blip("Hi, Bob!", 0, 4));
        assert!(should_play_talk_blip("Hello, Bob!", 0, 7));
        assert!(!should_play_talk_blip("Hello, Bob!", 7, 8));
    }

    #[test]
    fn known_speakers_map_to_voiceprints() {
        assert_eq!(
            talk_blip_id_for_speaker("Bob", "", DialogSpeechStyle::Normal),
            ids::DIALOGUE_BLIP_BOB,
        );
        assert_eq!(
            talk_blip_id_for_speaker("Shadow Oni Leader", "", DialogSpeechStyle::Normal),
            ids::DIALOGUE_BLIP_NINJA,
        );
        assert_eq!(
            talk_blip_id_for_speaker("", "hall_npc_oiler", DialogSpeechStyle::Normal),
            ids::DIALOGUE_BLIP_OILER,
        );
    }

    #[test]
    fn unknown_and_styled_speech_fall_back_to_generic_variants() {
        assert_eq!(
            talk_blip_id_for_speaker("Mystery", "", DialogSpeechStyle::Normal),
            ids::DIALOGUE_BLIP_GENERIC,
        );
        assert_eq!(
            talk_blip_id_for_speaker("Bob", "", DialogSpeechStyle::Whisper),
            ids::DIALOGUE_BLIP_WHISPER_GENERIC,
        );
        assert_eq!(
            talk_blip_id_for_speaker("Bob", "", DialogSpeechStyle::Shout),
            ids::DIALOGUE_BLIP_SHOUT_GENERIC,
        );
    }
}
