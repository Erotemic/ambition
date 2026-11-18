//! Each game's dialogue timing and input rules.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TextReveal {
    Instant,
    Typewriter { chars_per_second: f32 },
}

impl TextReveal {
    pub fn visible_chars(self, total: usize, elapsed_s: f32) -> usize {
        match self {
            Self::Instant => total,
            Self::Typewriter { chars_per_second } => {
                if elapsed_s >= total as f32 / chars_per_second { return total; }
                ((elapsed_s.max(0.0) * chars_per_second).floor() as usize).min(total)
            },
        }
    }

    pub fn duration(self, text: &str) -> f32 {
        match self {
            Self::Instant => 0.0,
            Self::Typewriter { chars_per_second } => text.chars().count() as f32 / chars_per_second,
        }
    }

    pub fn visible_text(self, text: &str, elapsed_s: f32) -> &str {
        let count = self.visible_chars(text.chars().count(), elapsed_s);
        let end = text.char_indices().nth(count).map_or(text.len(), |(byte, _)| byte);
        &text[..end]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevealConfirm {
    #[default]
    RevealOnly,
    RevealAndAdvance,
    Ignore,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum LineAdvance {
    #[default]
    Manual,
    Automatic {
        delay_seconds: f32,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ChoiceReveal {
    #[default]
    Immediate,
    Staggered {
        options_per_second: f32,
    },
}

/// Settings are copied when a conversation starts. A reload affects the next conversation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DialogSettings {
    pub text_reveal: TextReveal,
    pub confirm_while_revealing: RevealConfirm,
    pub line_advance: LineAdvance,
    pub choice_reveal: ChoiceReveal,
    pub close_on_back: bool,
    pub speech_blips: bool,
}

impl Default for DialogSettings {
    fn default() -> Self {
        Self {
            text_reveal: TextReveal::Typewriter {
                chars_per_second: 112.5,
            },
            confirm_while_revealing: RevealConfirm::RevealOnly,
            line_advance: LineAdvance::Manual,
            choice_reveal: ChoiceReveal::Immediate,
            close_on_back: true,
            speech_blips: true,
        }
    }
}

impl DialogSettings {
    /// A confirm can reveal the line before it can advance its source.
    pub fn reveal_confirm_action(self, complete: bool) -> Option<DialogConfirmAction> {
        if complete { return None; }
        match self.confirm_while_revealing {
            RevealConfirm::RevealOnly => Some(DialogConfirmAction::Reveal),
            RevealConfirm::Ignore => Some(DialogConfirmAction::Wait),
            RevealConfirm::RevealAndAdvance => None,
        }
    }

    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if let TextReveal::Typewriter { chars_per_second } = self.text_reveal {
            if !chars_per_second.is_finite() || chars_per_second <= 0.0 {
                problems.push("text_reveal.chars_per_second must be finite and positive".into());
            }
        }
        if let ChoiceReveal::Staggered { options_per_second } = self.choice_reveal {
            if !options_per_second.is_finite() || options_per_second <= 0.0 {
                problems
                    .push("choice_reveal.options_per_second must be finite and positive".into());
            }
        }
        if let LineAdvance::Automatic { delay_seconds } = self.line_advance {
            if !delay_seconds.is_finite() || delay_seconds < 0.0 {
                problems.push("line_advance.delay_seconds must be finite and nonnegative".into());
            }
        }
        problems
    }
}

#[cfg(test)]
mod shared_line_tests {
    use super::*;

    #[test]
    fn timed_lines_share_reveal_confirmation_and_safe_unicode_boundaries() {
        let settings = DialogSettings { text_reveal: TextReveal::Typewriter { chars_per_second: 10.0 }, ..Default::default() };
        let text = "é好🙂done";
        assert_eq!(settings.text_reveal.visible_text(text, 0.2), "é好");
        assert_eq!(settings.reveal_confirm_action(false), Some(DialogConfirmAction::Reveal));
        let duration = settings.text_reveal.duration(text);
        assert_eq!(settings.text_reveal.visible_text(text, duration), text);
        assert_eq!(settings.reveal_confirm_action(true), None);
        let instant = DialogSettings { text_reveal: TextReveal::Instant, ..settings };
        assert_eq!(instant.text_reveal.visible_text(text, 0.0), text);
    }
}

/// The action the next confirm performs. Presentation uses the same answer as input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DialogConfirmAction {
    Wait,
    Reveal,
    #[default]
    Advance,
    Select,
    Close,
}

#[cfg(feature = "content_pack")]
pub const DIALOGUE_SETTINGS_SCHEMA: &str = "dialogue_settings";

#[cfg(feature = "content_pack")]
pub fn dialogue_settings_schema() -> ambition_content_pack::SchemaRegistration {
    ambition_content_pack::rules_schema::<DialogSettings>(
        DIALOGUE_SETTINGS_SCHEMA,
        1,
        "dialogue",
        "Dialogue text reveal, confirmation, advancement and choice timing.",
        DialogSettings::problems,
    )
}

#[cfg(feature = "content_pack")]
pub fn lowered_dialogue_settings(
    pack: &ambition_content_pack::PreparedContentPack,
) -> Option<&DialogSettings> {
    ambition_content_pack::lowered_rules(pack, DIALOGUE_SETTINGS_SCHEMA)
}
