//! The universal audio controls the shell offers every experience.
//!
//! Not gated on `basic_presentation`: the pause menu and the launcher settings
//! tab both use these, and `plugin.rs` is ungated. Check changes to this
//! module at default features too.


/// The universal audio controls the shell offers every experience.
///
/// This enum only says which four global audio fields the shell menu shows.
/// The mutation rules live in `ambition_persistence::settings::AudioSettings`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShellAudioControl {
    Mute,
    MasterVolume,
    MusicVolume,
    SfxVolume,
}

impl ShellAudioControl {
    pub(crate) const ALL: [Self; 4] = [
        Self::Mute,
        Self::MasterVolume,
        Self::MusicVolume,
        Self::SfxVolume,
    ];

    // Labels and values are for presentation only, so they are gated. `ALL`
    // and `adjust` are not: `plugin.rs` applies adjustments without a renderer.
    #[cfg(feature = "basic_presentation")]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Mute => "Mute",
            Self::MasterVolume => "Master Volume",
            Self::MusicVolume => "Music Volume",
            Self::SfxVolume => "Sound Volume",
        }
    }

    #[cfg(feature = "basic_presentation")]
    pub(crate) fn value(self, settings: &ambition_persistence::settings::UserSettings) -> String {
        use ambition_persistence::settings::AudioSettings;

        match self {
            Self::Mute => if settings.audio.muted { "On" } else { "Off" }.to_owned(),
            Self::MasterVolume => {
                format!("{}%", AudioSettings::percent(settings.audio.master_volume))
            }
            Self::MusicVolume => {
                format!("{}%", AudioSettings::percent(settings.audio.music_volume))
            }
            Self::SfxVolume => {
                format!("{}%", AudioSettings::percent(settings.audio.sfx_volume))
            }
        }
    }

    pub(crate) fn adjust(self, direction: i32, settings: &mut ambition_persistence::settings::UserSettings) {
        use ambition_persistence::settings::AudioSettings;

        let step = if direction < 0 {
            -AudioSettings::VOLUME_STEP
        } else {
            AudioSettings::VOLUME_STEP
        };
        match self {
            Self::Mute => settings.audio.toggle_mute(),
            Self::MasterVolume => settings.audio.nudge_master(step),
            Self::MusicVolume => settings.audio.nudge_music(step),
            Self::SfxVolume => settings.audio.nudge_sfx(step),
        }
    }
}
