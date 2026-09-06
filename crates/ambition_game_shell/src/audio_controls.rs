//! The universal audio controls the shell offers every experience.
//!
//! ⛔⛔ THIS LIVED INSIDE `pause_menu`, WHICH IS `#[cfg(feature =
//! "basic_presentation")]`, AND THAT BROKE THE CRATE AT DEFAULT FEATURES.
//! `plugin.rs` is NOT gated, and once the launcher's settings tab needed these
//! rows it imported them unconditionally from a module that may not exist:
//! *"could not find `pause_menu` in the crate root"*. Reported by the peer
//! session 2026-09-05, blocking its builds, because `ambition_content` pulls
//! this crate in transitively.
//!
//! ⚠ I never saw it because every check I ran passed `--features
//! basic_presentation`. The same shape as the regression an assembled-host test
//! caught for me an hour earlier: the configuration I exercised was not the
//! configuration that breaks.
//!
//! ⭐ The fix is not to gate the import. These are SHELL IDENTITY -- "the four
//! global audio fields that belong on the universal shell menu" -- and they now
//! have two consumers, the pause menu and the launcher's settings tab, one of
//! which lives in ungated code. A type's home should match its SCOPE, so it
//! moved out of the presentation-gated module rather than the ungated caller
//! learning to live without it.


/// The universal audio controls the shell offers every experience.
///
/// This is shell presentation identity, not the full settings-menu IR. The mutation law stays
/// canonical in `ambition_persistence::settings::AudioSettings`; this enum only says which four
/// global audio fields belong on the universal shell menu.
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

    // ⚠ PRESENTATION-ONLY, so gated with the presentation. `ALL` and `adjust`
    // are unconditional: `plugin.rs` applies an adjustment whether or not any
    // renderer is compiled in, because the COMMAND exists either way. A label
    // and a formatted value only mean something to a menu that draws.
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
