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
/// FOUR TIMES BY IT.** `flag` left when `world.flag_set` was published;
/// `bosses_cleared` and `quests_active` left on 2026-09-04 when `boss.cleared`
/// and `quest.active` were; `wallet_balance` left on 2026-09-05. Each time the
/// Yarn function stayed, its NAME stayed, and authored `.yarn` content was
/// untouched — the function became a registered system reading the live
/// authority instead of a closure over a field here.
///
/// ⛔⛔ **AND THE WALLET ONE CORRECTS A SENTENCE THIS DOC USED TO MAKE.** It said
/// `wallet_balance` was *"deliberately not migrating"* because a NUMBER cannot
/// pass through the catalog's boolean-outcome shape *"without inventing a
/// comparison vocabulary"*. That reasoning was sound about the CATALOG and wrong
/// about the MIRROR: the two are different claims, and only the first was
/// checked. `ask_wallet_balance` is now a registered system reading `BodyWallet`
/// on the `PrimaryPlayer` directly — it never needed the catalog, so the mirror
/// was holding a projection for a reason that did not apply to it.
/// ⇒ *"The catalog cannot answer this"* does NOT imply *"this field must
/// exist"*. Ask what reads the field, not what the catalog can express.
///
/// ⇒ **So a new field is a claim that NOTHING ELSE can answer the question**,
/// and the burden is on the field. What is left is `visit_counts` — dialogue's
/// own bookkeeping rather than a world fact — and content-owned `extras`.
/// ⛔ An empty mirror is not the goal; one authority per question is.
///
/// ⚠ NOT EVERY READER IS A CLOSURE ANY MORE. The functions that migrated take
/// `&mut World`, because a catalog evaluator does; the ones still here are
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

    /// ⛔⛔ ADDING A FIELD TO THE MIRROR IS A CLAIM, AND THIS MAKES YOU MAKE IT.
    ///
    /// The type's own doc says a new field asserts that NOTHING ELSE can answer
    /// the question, and that the burden is on the field. That burden was prose,
    /// and prose is what let `wallet_balance` sit here for weeks behind a reason
    /// that did not apply to it: *"a NUMBER cannot pass through the catalog's
    /// boolean outcome"* was sound about the CATALOG and irrelevant to the
    /// MIRROR, because `ask_wallet_balance` never needed the catalog — it reads
    /// `BodyWallet` directly. Four fields have now left by that argument
    /// (`flag`, `bosses_cleared`, `quests_active`, `wallet_balance`).
    ///
    /// ⭐ THE DESTRUCTURE BELOW HAS NO `..`, so a fifth field does not compile
    /// (E0027) and lands its author on this note instead of on nothing. That is
    /// the point: not to forbid a field, but to make adding one a decision
    /// somebody wrote down rather than a default.
    ///
    /// ⚠ THE TWO SURVIVORS, and why each is not a world fact:
    /// * `visit_counts` — dialogue's own bookkeeping. Nothing outside the
    ///   dialogue runner knows how many times a node was entered, so there is no
    ///   live authority to read instead.
    /// * `extras` — content-owned strings the generic refresh deliberately does
    ///   not touch. An escape hatch for a host, not an engine fact.
    ///
    /// ⇒ If you are adding a field, the question to answer first is not "can the
    /// catalog express this?" but "what already READS this, and could the
    /// function read that instead?"
    ///
    /// ⚠ THIS BITES ON THE `ui` LANE ONLY, and saying so is part of the guard.
    /// `mod bindings` is `#[cfg(feature = "ui")]` and `default = []`, so
    /// `cargo test -p ambition_dialog` compiles none of this file — a field
    /// added while running only default features produces no error here. Every
    /// build that actually runs Yarn turns `ui` on, so the shipped path is
    /// covered; a default-feature run is not. Verified by poisoning: adding a
    /// fifth field yields `error[E0027]: pattern does not mention field` under
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
