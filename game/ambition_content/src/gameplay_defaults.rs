//! Ambition's authored movement and combat defaults.

use ambition_platformer2d::assets::gameplay_defaults::Platformer2dGameplayDefaults;

pub const PLATFORMER_DEFAULTS_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ambition/platformer_defaults.ron");

static SOURCE: ambition_platformer2d::content::PackText =
    ambition_platformer2d::content_text!("assets/ambition/platformer_defaults.ron");

pub fn load() -> Platformer2dGameplayDefaults {
    let text = SOURCE.text().unwrap_or_else(|error| panic!("{error}"));
    Platformer2dGameplayDefaults::parse(&text)
        .unwrap_or_else(|error| panic!("{PLATFORMER_DEFAULTS_FILE}: {error}"))
}

#[cfg(test)]
mod tests {
    /// Ambition's crush policy parses, names only characters the shipped
    /// catalog has and no surface, and holds numbers in range.
    #[test]
    fn the_crush_policy_names_real_characters() {
        let policy = super::load().crush.expect("Ambition states a crush policy");
        let catalog = crate::character_catalog::shipped_catalog();
        let ids: Vec<&str> = catalog.iter().map(|(id, _)| id.as_str()).collect();
        policy.validate(std::iter::empty(), ids.iter().copied()).unwrap_or_else(|error| panic!("{error}"));
        assert!(policy.bodies.contains_key("player_robot_v3"), "Scout's resistance is authored");
        assert!(policy.game.lift_authority.is_some(), "lifts have a rule");
    }
}
