//! Ambition's authored movement, ability, and combat defaults.

use ambition_platformer2d::assets::gameplay_defaults::Platformer2dGameplayDefaults;

pub const PLATFORMER_DEFAULTS_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ambition/platformer_defaults.ron");

static SOURCE: ambition_platformer2d::content::PackText =
    ambition_platformer2d::content_text!("assets/ambition/platformer_defaults.ron");

pub fn load() -> Platformer2dGameplayDefaults {
    let text = SOURCE.text().unwrap_or_else(|error| panic!("{error}"));
    Platformer2dGameplayDefaults::parse(&text)
        .unwrap_or_else(|error| panic!("{PLATFORMER_DEFAULTS_FILE}: {error}"))
}
