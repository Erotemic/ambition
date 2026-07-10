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
pub fn known_dialogue_ids() -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = Vec::new();
    for (_, source) in YARN_SOURCES {
        for title in yarn_title_ids(source) {
            ids.push(title);
            if let Some((root, _)) = title.split_once("__") {
                ids.push(root);
            }
        }
    }
    ids.extend(
        ambition_actors::character_roster::catalog()
            .characters
            .values()
            .filter_map(|entry| entry.hall_dialogue_id.as_deref()),
    );
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dialogue_ids_are_derived_from_yarn_titles() {
        let ids = known_dialogue_ids();
        assert!(ids.contains(&"creator_intro"));
        assert!(ids.contains(&"oiler_post_stabilizer"));
        assert!(ids.contains(&"hub_guide__test_sfx"));
        assert!(ids.contains(&"hall_player"));
        assert_eq!(ids.windows(2).filter(|pair| pair[0] == pair[1]).count(), 0);
    }

    #[test]
    fn catalog_hall_dialogue_ids_are_known() {
        // known_dialogue_ids() folds the catalog ids in so the LDtk validator
        // accepts authored hall_<id> nodes without a second list.
        let known = known_dialogue_ids();
        for expected in [
            "hall_pirate_admiral",
            "hall_stochastic_parrot",
            "hall_architect",
        ] {
            assert!(
                known.contains(&expected),
                "{expected} should be in known_dialogue_ids() via the catalog fold-in"
            );
        }
    }

    #[test]
    fn every_catalog_hall_dialogue_id_has_a_yarn_node() {
        // The dangling-id bug: a catalog row authors `hall_dialogue_id:
        // Some("hall_x")` but `hall.yarn` has no `title: hall_x` node, so
        // Inspecting that pedestal starts an unknown node at runtime (silent
        // in tests, broken in the game). Pure-text cross-check — no Yarn
        // runtime — so it runs in every config and fails at `cargo test`.
        let yarn = YARN_SOURCES
            .iter()
            .find(|(name, _)| name.ends_with("hall.yarn"))
            .map(|(_, text)| *text)
            .expect("hall.yarn is a registered source");
        let nodes: std::collections::HashSet<&str> = yarn
            .lines()
            .filter_map(|l| l.strip_prefix("title:"))
            .map(str::trim)
            .collect();

        let missing: Vec<(&String, &str)> = ambition_actors::character_roster::catalog()
            .characters
            .iter()
            .filter_map(|(id, entry)| {
                entry
                    .hall_dialogue_id
                    .as_deref()
                    .filter(|hid| !nodes.contains(hid))
                    .map(|hid| (id, hid))
            })
            .collect();

        assert!(
            missing.is_empty(),
            "catalog hall_dialogue_id(s) with no matching `title:` node in \
             hall.yarn (Inspect would start an unknown node):\n{}",
            missing
                .iter()
                .map(|(id, hid)| format!("  {id} -> {hid}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    /// The DEFAULT player character wears `player`, and the Hall's player
    /// pedestal IS `player`. So on the one interaction every playthrough is
    /// likeliest to make, the speaker is the listener — and the engine SUPPRESSES
    /// a self-conversation that has no `__self` branch
    /// (`ambition_dialog::DialogueNodeIndex::entry_node`).
    ///
    /// Without this node the player's own pedestal would silently become
    /// un-talkable. The mirror scene is content; that it must exist is structure.
    #[test]
    fn the_player_pedestal_has_a_self_branch_because_the_default_character_is_the_player() {
        // `known_dialogue_ids()` folds in catalog rows, so the roster must exist.
        // Self-sufficient rather than order-dependent on a sibling test.
        crate::character_catalog::install();
        assert_eq!(
            crate::character_catalog::PLAYABLE_ROSTER[0],
            "player",
            "this guard assumes the default worn character",
        );
        let known = known_dialogue_ids();
        assert!(
            known.contains(&"hall_player__self"),
            "hall.yarn must author `hall_player__self`: the default player wears \
             `player`, so interacting with the `player` pedestal is self-talk, which \
             the engine suppresses unless content authored the branch",
        );
    }

    /// Every `<root>__self` branch belongs to a real root node. A self branch for
    /// a dialogue that does not exist is dead content.
    #[test]
    fn every_self_branch_has_a_root_node() {
        for (name, source) in YARN_SOURCES {
            let titles: Vec<&str> = yarn_title_ids(source).collect();
            for title in &titles {
                if let Some(root) = title.strip_suffix("__self") {
                    assert!(
                        titles.contains(&root),
                        "{name}: `{title}` is a self branch of `{root}`, which has no \
                         `title:` node in the same file",
                    );
                }
            }
        }
    }
}
