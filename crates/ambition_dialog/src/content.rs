//! Dialogue content types — minimal post-Yarn migration.
//!
//! Authored conversation content is CONTENT: the game's `.yarn` set and
//! the validator's known-dialogue-id surface live in
//! `ambition_content::dialogue::yarn` (R3.2). The runtime type here
//! covers only what the UI view-model still needs:
//!
//! - [`DialogChoice`] — the runtime option representation written
//!   by the Yarn bridge into [`crate::DialogState`]`.current_options`
//!   and rendered by `sync_dialog_ui`. `next_node` / `note` /
//!   `close_after` are vestigial fields kept for the existing UI
//!   layout code (the renderer reads `label` and ignores the rest);
//!   they're set to `None` / `false` by the bridge.
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
