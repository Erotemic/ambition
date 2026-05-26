//! Dialogue content types + registry.
//!
//! Every authored dialogue tree lives in
//! `assets/data/dialogue/registry.ron`. This module owns the data
//! types (`DialogChoice`, `DialogNode`, `DialogTree`, redirect rules)
//! and the `LazyLock<DialogRegistry>` that the runtime, validator,
//! and redirect systems all read from.
//!
//! Add a tree: append a key under `trees` in the RON file. Add a
//! redirect: append a `(from, gate, to)` tuple under `redirects`. No
//! Rust change needed for either.

use std::collections::HashMap;
use std::sync::LazyLock;

/// One selectable option inside a `DialogNode`. Selecting it either
/// advances to `next_node`, closes the dialogue (`close_after`), or
/// falls through to the node's `default_next` if both are `None`.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct DialogChoice {
    pub label: String,
    pub next_node: Option<usize>,
    pub note: Option<String>,
    #[serde(default)]
    pub close_after: bool,
}

/// One beat in a dialogue tree. `default_next` is consulted only when
/// the node carries no options.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct DialogNode {
    pub speaker: String,
    pub line: String,
    #[serde(default)]
    pub options: Vec<DialogChoice>,
    pub default_next: Option<usize>,
}

/// One authored dialogue tree. `id` matches the LDtk
/// `NpcSpawn.dialogue_id` field; `label` is the human-readable name
/// shown in the dialogue title bar.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct DialogTree {
    pub id: String,
    pub label: String,
    pub nodes: Vec<DialogNode>,
}

/// Condition gate for a redirect rule. Today: world-state predicates
/// driven by the sandbox save layer.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub enum DialogRedirectGate {
    /// True iff the named boss encounter is `Cleared`. Argument is
    /// the canonical encounter id (e.g. `"mockingbird"`).
    BossCleared(String),
    /// True iff the named save flag is set. Argument is the flag id
    /// (e.g. `"p1_stabilizer_received"`).
    FlagSet(String),
}

/// "When a player walks up to a `from` NPC and the `gate` predicate
/// passes, swap the dialogue tree to `to`." Read each frame by
/// [`crate::dialog::redirect_post_quest_dialog`].
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct DialogRedirectRule {
    pub from: String,
    pub to: String,
    pub gate: DialogRedirectGate,
}

/// Parsed contents of `assets/data/dialogue/registry.ron`.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct DialogRegistry {
    pub trees: HashMap<String, DialogTree>,
    pub redirects: Vec<DialogRedirectRule>,
}

/// Single source of truth for sandbox + intro dialogue. Loaded once
/// on first access via `LazyLock` so non-system call sites (validator,
/// tests, route-state helpers) can read without a Bevy resource.
pub static DIALOG_REGISTRY: LazyLock<DialogRegistry> = LazyLock::new(|| {
    const DIALOG_REGISTRY_RON: &str =
        include_str!("../../assets/data/dialogue/registry.ron");
    ron::from_str(DIALOG_REGISTRY_RON).unwrap_or_else(|err| {
        panic!(
            "assets/data/dialogue/registry.ron failed to deserialize as DialogRegistry: {err}"
        )
    })
});

/// Stable fallback id for any NPC that doesn't carry an authored
/// dialogue. Matches the `generic_npc` row in `registry.ron`.
pub const GENERIC_DIALOGUE_ID: &str = "generic_npc";

/// Look up a dialogue tree by id. Returns `None` for unknown ids;
/// the runtime substitutes the generic tree as a fallback.
pub fn tree_for(dialogue_id: &str) -> Option<&'static DialogTree> {
    DIALOG_REGISTRY.trees.get(dialogue_id)
}

/// All dialogue ids the registry knows about. Used by the LDtk
/// content validator to approve `NpcSpawn.dialogue_id` fields.
pub fn known_dialogue_ids() -> Vec<&'static str> {
    DIALOG_REGISTRY
        .trees
        .keys()
        .map(|s| s.as_str())
        .collect()
}

