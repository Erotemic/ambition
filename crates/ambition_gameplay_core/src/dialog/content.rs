//! Dialogue content types — minimal post-Yarn migration.
//!
//! Authored conversation content lives in
//! `assets/dialogue/sandbox/*.yarn` and is loaded by
//! `bevy_yarnspinner`. The runtime types here cover only what the
//! UI view-model + LDtk validator still need:
//!
//! - [`DialogChoice`] — the runtime option representation written
//!   by the Yarn bridge into [`crate::dialog::DialogState.current_options`]
//!   and rendered by `sync_dialog_ui`. `next_node` / `note` /
//!   `close_after` are vestigial fields kept for the existing UI
//!   layout code (the renderer reads `label` and ignores the rest);
//!   they're set to `None` / `false` by the bridge.
//! - [`known_dialogue_ids`] — Yarn node ids `NpcSpawn.dialogue_id` may
//!   reference. The LDtk content validator uses this to flag typos.
//!
//! Adding a dialogue: edit the matching `.yarn` zone file. The validator
//! derives accepted ids from the `title:` headers.
//!
//! The pre-migration `DialogTree` / `DialogNode` / `DialogRedirectRule`
//! / `DialogRegistry` types and the RON registry loader have been
//! retired. Boss-cleared / flag-set redirects are now inline
//! `<<if boss_cleared("x")>>` branches inside the `.yarn` files.

/// Option emitted by Yarn's `PresentOptions` event, in the shape
/// the existing UI renderer expects. The bridge fills `label` from
/// the Yarn option's text; the other fields are vestigial.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DialogChoice {
    pub label: String,
    /// Vestigial — was the RON-era "next tree-node index". Yarn
    /// dispatches via `OptionId`, stored on `DialogState.yarn_option_ids`.
    pub next_node: Option<usize>,
    /// Vestigial — was the RON-era "system aside text after picking
    /// this option". Inline Yarn lines now carry asides directly.
    pub note: Option<String>,
    /// Vestigial — was the RON-era "this option closes the dialog".
    /// Yarn's runner reports closure via `DialogueCompleted`.
    pub close_after: bool,
}

const SANDBOX_YARN_SOURCES: &[&str] = &[
    include_str!("../../assets/dialogue/sandbox/cove.yarn"),
    include_str!("../../assets/dialogue/sandbox/dojo.yarn"),
    include_str!("../../assets/dialogue/sandbox/factions.yarn"),
    include_str!("../../assets/dialogue/sandbox/hall.yarn"),
    include_str!("../../assets/dialogue/sandbox/intro.yarn"),
    include_str!("../../assets/dialogue/sandbox/kernel.yarn"),
    include_str!("../../assets/dialogue/sandbox/symmetry.yarn"),
];

fn yarn_title_ids(source: &'static str) -> impl Iterator<Item = &'static str> {
    source.lines().filter_map(|line| {
        let title = line.strip_prefix("title:")?.trim();
        (!title.is_empty()).then_some(title)
    })
}

/// Validator surface (LDtk content_validation reads this). Folds in the
/// per-character Hall-of-Characters dialogue ids declared in the catalog
/// (`hall_dialogue_id`), so authored `hall_<id>` nodes are accepted without a
/// second hand-maintained list — the catalog is their single source of truth.
pub fn known_dialogue_ids() -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = Vec::new();
    for source in SANDBOX_YARN_SOURCES {
        for title in yarn_title_ids(source) {
            ids.push(title);
            if let Some((root, _)) = title.split_once("__") {
                ids.push(root);
            }
        }
    }
    ids.extend(
        crate::character_roster::EMBEDDED_CATALOG
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
}
