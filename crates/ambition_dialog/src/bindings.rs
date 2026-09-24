//! Content-free Yarn binding state, presentation cues, and vocabulary installers.
//!
//! Hosts own game-specific commands/functions and refresh [`YarnStateMirror`] from their state.

use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use bevy_yarnspinner::prelude::DialogueRunner;

/// Host-refreshed projection read by closure-shaped Yarn library functions.
///
/// This is not an authority: facts already exposed through the authored-condition
/// catalog must be queried there rather than duplicated here. The `Arc<RwLock<_>>`
/// lets runner-library closures read the projection without borrowing a Bevy resource.
///
/// ⭐⭐ **AND THE "QUERIED THERE" RULE HAS TEETH NOW — THIS TYPE HAS SHRUNK
/// `flag`, `bosses_cleared`, `quests_active` and `wallet_balance` left this type
/// by that rule. Each Yarn function kept its name and became a registered
/// system that reads the live authority. `ask_wallet_balance` reads `BodyWallet`
/// on the `PrimaryPlayer` directly; it never needed the catalog.
///
/// A new field claims that nothing else can answer the question. Before you
/// add one, ask what already reads the fact and whether the function can read
/// that instead. The remaining fields are `visit_counts` (dialogue's own
/// bookkeeping) and content-owned `extras`.
///
/// Migrated functions take `&mut World`; the rest are closures over the `Arc`.
/// closures over the `Arc`, which is what the sentence above describes.
#[derive(Default, Clone, Debug)]
pub struct YarnStateMirrorData {
    /// dialogue id → visit count.
    pub visit_counts: std::collections::HashMap<String, u32>,
    /// Content-owned string values; the generic refresh does not modify them.
    pub extras: std::collections::HashMap<String, String>,
}

#[derive(Resource, Default, Clone)]
pub struct YarnStateMirror(pub Arc<RwLock<YarnStateMirrorData>>);

/// Per-frame presentation cues derived from Yarn line markup.
/// Cleared before the bridge publishes cues for the current frame.
#[derive(Resource, Default, Debug, Clone)]
pub struct YarnPresentationCue {
    /// True iff the most recent line carried `[shout]` markup.
    pub shout: bool,
    /// True iff the most recent line carried `[whisper]` markup.
    pub whisper: bool,
}

/// Ordering boundary after the host refreshes [`YarnStateMirror`].
/// Content systems that consume the refreshed projection run after this set.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct YarnStateMirrorRefreshed;

/// Ordering boundary after [`YarnPresentationCue`] is cleared for the frame.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct YarnPresentationCueCleared;

/// Reset markup cues before the bridge writes cues for the current frame.
pub fn clear_yarn_presentation_cue(mut cue: ResMut<YarnPresentationCue>) {
    cue.shout = false;
    cue.whisper = false;
}

/// Registers custom Yarn commands/functions when the `DialogueRunner` is spawned.
pub type YarnBindingInstaller = fn(&mut Commands, &mut DialogueRunner, &YarnStateMirror);

/// Registered installers for host/content Yarn vocabulary.
#[derive(Resource, Default)]
pub struct YarnContentBindings {
    pub installers: Vec<YarnBindingInstaller>,
}

/// Initializes binding resources and schedules the per-frame cue reset.
pub struct YarnBindingsPlugin;

impl Plugin for YarnBindingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<YarnStateMirror>();
        app.init_resource::<YarnPresentationCue>();
        app.init_resource::<YarnContentBindings>();
        app.add_systems(
            Update,
            clear_yarn_presentation_cue.in_set(YarnPresentationCueCleared),
        );
    }
}

#[cfg(test)]
mod mirror_field_burden {
    use super::YarnStateMirrorData;

    /// Adding a field to the mirror must be a decision.
    ///
    /// The destructure below has no `..`, so a new field fails to compile
    /// (E0027) and brings its author to this note. See the type's doc for the rule.
    ///
    /// The two remaining fields are not world facts:
    /// * `visit_counts`: dialogue's own bookkeeping. Only the runner knows how
    ///   many times a node was entered.
    /// * `extras`: content-owned strings that the generic refresh does not touch.
    ///
    /// `mod bindings` is `#[cfg(feature = "ui")]` and `default = []`, so this
    /// guard runs only with `--features ui`. Every build that runs Yarn enables `ui`.
    /// `--features ui`, and exit 0 without it.
    #[test]
    fn every_field_on_the_mirror_is_one_nothing_else_can_answer() {
        let data = YarnStateMirrorData::default();
        // No `..`: this is the whole type, on purpose.
        let YarnStateMirrorData {
            visit_counts,
            extras,
        } = &data;
        assert!(
            visit_counts.is_empty() && extras.is_empty(),
            "a default mirror mirrors nothing yet"
        );
    }
}
