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
#[derive(Default, Clone, Debug)]
pub struct YarnStateMirrorData {
    /// canonical quest ids whose state is `InProgress`.
    pub quests_active: std::collections::HashSet<String>,
    /// dialogue id → visit count.
    pub visit_counts: std::collections::HashMap<String, u32>,
    /// Content-owned string values; the generic refresh does not modify them.
    pub extras: std::collections::HashMap<String, String>,
    /// Primary wallet balance exposed to Yarn functions such as `wallet_balance`.
    pub wallet_balance: i32,
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
