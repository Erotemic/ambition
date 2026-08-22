//! Runtime dialogue option data consumed by the UI view model.
//!
//! Authored dialogue lives in game-owned Yarn content.

/// Option emitted by Yarn `PresentOptions` in the UI renderer's runtime shape.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DialogChoice {
    pub label: String,
    /// Compatibility field; Yarn dispatches through `DialogState.yarn_option_ids`.
    pub next_node: Option<usize>,
    /// Compatibility field; Yarn carries aside text inline.
    pub note: Option<String>,
    /// Compatibility field; Yarn reports closure via `DialogueCompleted`.
    pub close_after: bool,
}
