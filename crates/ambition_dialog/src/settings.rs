//! Each game's dialogue timing and input rules.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TextReveal {
    Instant,
    Typewriter { chars_per_second: f32 },
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
