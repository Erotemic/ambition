//! Generic Yarn binding machinery — the reusable half of the old
//! `dialog/yarn_bindings.rs`.
//!
//! A host's game-specific Yarn *commands* (`<<give_item>>`) and *functions*
//! (`<<if boss_cleared("x")>>`) reference actor/save state, so they stay
//! host-side. What lives here is only the content-free plumbing they need:
//!
//! - [`YarnStateMirror`] / [`YarnStateMirrorData`] — the shared snapshot that
//!   synchronous Yarn `library` functions read from. The *shape* is generic
//!   (flags / bosses / quests / visit counts / inventory / wallet / an
//!   open-ended `extras` map); the per-frame *refresh* that fills it from a
//!   particular game's save is host-side.
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

/// Snapshot of game state the Yarn `library` functions read from,
/// refreshed each frame by a host-side system. Wrapped in
/// `Arc<RwLock<...>>` so the closures registered on the runner's
/// `Library` (which capture by move) can read it without taking a
/// Bevy resource.
///
/// Yarn `library` functions are synchronous pure functions — they
/// can't take a `Res<...>` like a Bevy system can. The mirror shape
/// solves that: a refresh system updates the snapshot inside the lock
/// once per frame, and function closures lock-and-read on every Yarn
/// `<<if>>` evaluation.
#[derive(Default, Clone, Debug)]
pub struct YarnStateMirrorData {
    /// flag id → on/off.
    pub flags: std::collections::HashMap<String, bool>,
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
    /// Item `dialog_id()` → held count, mirrored from the host's live
    /// inventory so `inventory_has(...)` can read it.
    pub inventory_counts: std::collections::HashMap<String, u32>,
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
        app.add_systems(Update, clear_yarn_presentation_cue);
    }
}
