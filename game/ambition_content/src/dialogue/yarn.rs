//! The game's authored Yarn dialogue set — CONTENT data, evicted from the
//! engine core (R3.2: the engine ships no dialogue).
//!
//! One `.yarn` file per zone under `assets/dialogue/sandbox/`; the sources
//! are embedded and handed to `bevy_yarnspinner` IN MEMORY, so no asset-root
//! coupling remains and desktop / web / Android all load the same bytes.
//!
//! Single source of truth: [`yarn_spinner_plugin`] registers exactly
//! [`YARN_SOURCES`]; the `yarn_compile` integration test compiles exactly the
//! same set as one project (matching startup); [`known_dialogue_ids`] derives
//! the validator's accepted ids from the same texts. A new `.yarn` added here
//! is automatically covered by all three.

/// `(logical name, source text)` for every Yarn file the game loads.
pub const YARN_SOURCES: &[(&str, &str)] = &[
    (
        "dialogue/sandbox/intro.yarn",
        include_str!("../../assets/dialogue/sandbox/intro.yarn"),
    ),
    (
        "dialogue/sandbox/kernel.yarn",
        include_str!("../../assets/dialogue/sandbox/kernel.yarn"),
    ),
    (
        "dialogue/sandbox/factions.yarn",
        include_str!("../../assets/dialogue/sandbox/factions.yarn"),
    ),
    (
        "dialogue/sandbox/cove.yarn",
        include_str!("../../assets/dialogue/sandbox/cove.yarn"),
    ),
    (
        "dialogue/sandbox/dojo.yarn",
        include_str!("../../assets/dialogue/sandbox/dojo.yarn"),
    ),
    (
        "dialogue/sandbox/symmetry.yarn",
        include_str!("../../assets/dialogue/sandbox/symmetry.yarn"),
    ),
    (
        "dialogue/sandbox/hall.yarn",
        include_str!("../../assets/dialogue/sandbox/hall.yarn"),
    ),
];

/// Registers Yarn Spinner with the game's dialogue set as IN-MEMORY sources
/// (no folder scan, no asset-root dependency — identical on desktop, web,
/// and Android).
#[cfg(feature = "ui")]
pub fn yarn_spinner_plugin() -> bevy_yarnspinner::prelude::YarnSpinnerPlugin {
    use bevy_yarnspinner::prelude::{YarnFile, YarnFileSource, YarnSpinnerPlugin};
    YarnSpinnerPlugin::with_yarn_sources(
        YARN_SOURCES
            .iter()
            .map(|(name, text)| YarnFileSource::InMemory(YarnFile::new(*name, *text))),
    )
}

fn yarn_title_ids(source: &'static str) -> impl Iterator<Item = &'static str> {
    source.lines().filter_map(|line| {
        let title = line.strip_prefix("title:")?.trim();
        (!title.is_empty()).then_some(title)
    })
}

/// Validator surface (the LDtk content validator reads this): every Yarn node
/// id `NpcSpawn.dialogue_id` may reference. Folds in the per-character
/// Hall-of-Characters dialogue ids declared in the catalog
/// (`hall_dialogue_id`), so authored `hall_<id>` nodes are accepted without a
/// second hand-maintained list — the catalog is their single source of truth.
pub fn known_dialogue_ids(
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for (_, source) in YARN_SOURCES {
        for title in yarn_title_ids(source) {
            ids.push(title.to_string());
            if let Some((root, _)) = title.split_once("__") {
                ids.push(root.to_string());
            }
        }
    }
    ids.extend(
        catalog
            .characters
            .values()
            .filter_map(|entry| entry.hall_dialogue_id.clone()),
    );
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests;
