//! Generic Yarn binding machinery — the reusable half of the old
//! `dialog/yarn_bindings.rs`.
//!
//! A host's game-specific Yarn *commands* (`<<give_item>>`) and *functions*
//! (`<<if boss_cleared("x")>>`) reference actor/save state, so they stay
//! host-side. What lives here is only the content-free plumbing they need:
//!
//! - [`YarnStateMirror`] / [`YarnStateMirrorData`] — the shared snapshot that
//!   CLOSURE-shaped Yarn `library` functions read from. The *shape* is generic
//!   (bosses / quests / visit counts / wallet / an open-ended
//!   `extras` map); the per-frame *refresh* that fills it from a particular
//!   game's save is host-side. ⚠ it is a PROJECTION — read its type doc before
//!   adding a slice, because a fact the condition catalog can answer belongs in
//!   the catalog and nowhere else.
//! - [`YarnPresentationCue`] / [`clear_yarn_presentation_cue`] — the markup
//!   cue surface the bridge writes for `[shout]` / `[whisper]` lines.
//! - [`YarnBindingInstaller`] / [`YarnContentBindings`] — the extension seam:
//!   hosts and content plugins push installers that register their vocabulary
//!   on the runner when it spawns. The bridge itself owns only the reusable
//!   presentation commands (`present_speaker` and `portrait_clip`), never named
//!   game content.
//! - [`YarnBindingsPlugin`] — inits the resources + schedules the per-frame
//!   cue reset. Hosts add their state-mirror refresh separately.

use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use bevy_yarnspinner::prelude::DialogueRunner;

// ===== Shared state mirror =====================================

/// Snapshot of game state that CLOSURE-shaped Yarn `library` functions read
/// from, refreshed each frame by a host-side system. Wrapped in
/// `Arc<RwLock<...>>` so a closure registered on the runner's `Library` (which
/// captures by move) can read it without taking a Bevy resource.
///
/// # ⛔ This is a PROJECTION, not a place where a fact is decided
///
/// **This type's doc used to say Yarn functions *"can't take a `Res<...>` like a
/// Bevy system can"*, and that is FALSE of the crate in the lockfile.** A
/// `SystemId<In<P>, O>` implements `YarnFn`, and `bevy_yarnspinner` threads the
/// interpreter's `&mut World` down to it — so a Yarn function CAN be a Bevy
/// system and CAN read live state. The mirror was a workaround for a limit that
/// is not there.
///
/// ⇒ **anything the engine's condition catalog can answer must be ASKED, never
/// mirrored here.** A fact with a published condition and a mirror slice has two
/// definition sites that can disagree, which is the second-authority shape this
/// project refuses elsewhere; the `flags` slice was exactly that and is gone. See
/// `docs/planning/engine/authored-gameplay-logic-and-orchestration.md` and
/// `ambition_platformer2d_actor_monolith::dialog::authored_conditions`.
///
/// ⚠ **what is left is the remainder, and it is legitimate.** Encounter/quest
/// state, per-node visit counts, wallet and content `extras` have no published
/// condition, and a `f32`-returning function (`visit_count`, `wallet_balance`)
/// could not use a boolean condition verb even if one existed. Closures over this
/// snapshot stay the right shape for those until a domain publishes the question.
#[derive(Default, Clone, Debug)]
pub struct YarnStateMirrorData {
    /// canonical boss encounter ids in `Cleared` state.
    pub bosses_cleared: std::collections::HashSet<String>,
    /// canonical quest ids whose state is `InProgress`.
    pub quests_active: std::collections::HashSet<String>,
    /// dialogue id → visit count.
    pub visit_counts: std::collections::HashMap<String, u32>,
    /// Content-fed string values keyed by name (e.g. a boss room's
    /// current heavy-object id). The generic refresh never touches
    /// these; content-side systems mirror their own state in and
    /// content-installed Yarn functions read them. Keeps named content
    /// out of this generic mirror.
    pub extras: std::collections::HashMap<String, String>,
    /// Player money, mirrored from the primary player's wallet so a
    /// merchant dialogue can show the balance / gate purchases
    /// (`wallet_balance`, `can_afford`).
    pub wallet_balance: i32,
}

#[derive(Resource, Default, Clone)]
pub struct YarnStateMirror(pub Arc<RwLock<YarnStateMirrorData>>);

// ===== Markup cue ==============================================

/// Per-frame presentation cue surface populated by the bridge's
/// `on_present_line` observer whenever a Yarn line carries `[shout]`
/// or `[whisper]` markup. Camera shake / audio pitch consumers read
/// this in their normal Update systems; the cue clears each frame
/// via [`clear_yarn_presentation_cue`] before the bridge writes the
/// next one.
#[derive(Resource, Default, Debug, Clone)]
pub struct YarnPresentationCue {
    /// True iff the most recent line carried `[shout]` markup.
    pub shout: bool,
    /// True iff the most recent line carried `[whisper]` markup.
    pub whisper: bool,
}

/// **The set the yarn STATE MIRROR runs in — the mirror is current.**
///
/// The middle link of the yarn pipeline, and the vocabulary lives HERE rather
/// than beside the system that joins it. The engine's
/// `refresh_yarn_state_mirror` is the member; a game's content systems wait on
/// this set to read the refreshed mirror.
///
/// ⛔ that placement is not stylistic — `engine.dialog-vocab-dialog-crate`
/// enforces it, and it fired the moment this set was first defined in
/// `ambition_platformer2d_actor_monolith`. Reusable dialog vocabulary belongs in
/// this crate; a game reaching for a dialog NAME must not have to name the
/// monolith to get it. The old leaf pin slipped the rule on a technicality —
/// the forbidden token is a type name and `refresh_yarn_state_mirror` is a
/// lowercase function — so naming the boundary is what surfaced the dependency
/// the policy was written to prevent.
///
/// ⚠ ONE member. A game adding its own mirror joins the CONSUMER side, after
/// this set; it does not belong inside the refresh.
///
/// ⚠ named `...Refreshed`, not `...Mirrored`, because [`YarnStateMirror`] is
/// already a type in this crate and the two would read as the same thing.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct YarnStateMirrorRefreshed;

/// **The set [`clear_yarn_presentation_cue`] runs in — the cue is reset.**
///
/// The first link in a three-layer yarn pipeline that, until now, each layer
/// addressed by naming the layer below it: this crate clears the cue, the
/// engine's `YarnStateMirrorRefreshed` refreshes the mirror from it, and a game's
/// content systems read the mirror. Three crates, two leaf pins.
///
/// ⚠ ONE member — clearing the cue IS the whole step.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct YarnPresentationCueCleared;

/// Reset the markup cue once per frame. Runs before the bridge
/// observer fires (which writes the cue for THIS frame's line).
pub fn clear_yarn_presentation_cue(mut cue: ResMut<YarnPresentationCue>) {
    cue.shout = false;
    cue.whisper = false;
}

// ===== Content extension seam ===================================

/// One installer: registers a set of custom Yarn commands and/or
/// library functions on the runner. Runs once when the singleton
/// `DialogueRunner` is spawned.
pub type YarnBindingInstaller = fn(&mut Commands, &mut DialogueRunner, &YarnStateMirror);

/// Registered Yarn vocabulary installers. The host pushes its
/// generic game commands/functions here; content plugins push named
/// vocabulary (e.g. the cut-rope boss commands) so this crate names
/// no game content.
#[derive(Resource, Default)]
pub struct YarnContentBindings {
    pub installers: Vec<YarnBindingInstaller>,
}

// ===== Plugin ===================================================

/// Inits the generic binding resources and schedules the per-frame
/// cue reset. The host adds its own state-mirror refresh
/// (`.after(clear_yarn_presentation_cue)`) and pushes its game
/// bindings installer.
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
