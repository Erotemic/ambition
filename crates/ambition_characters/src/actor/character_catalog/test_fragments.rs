//! Catalog data shared by registration tests.

use super::CharacterCatalogFragment;

pub(crate) const A: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "alpha": (
                display_name: "Alpha", manifest: "a.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "idle", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

pub(crate) const B: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "beta": (
                display_name: "Beta", manifest: "b.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "idle", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

pub(crate) fn fragment(provider: &str, default_id: &str, ron: &str) -> CharacterCatalogFragment {
    CharacterCatalogFragment::from_ron(provider, Some(default_id), ron).unwrap()
}
