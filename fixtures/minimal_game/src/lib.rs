//! The smallest game this engine can stand up.
//!
//! Minimal external consumer used to measure optional-capability closure.
//! It asks for as little as a runnable game can so linked capabilities reflect
//! what the engine supplies implicitly rather than what content requested.
//!
//! # What it deliberately does NOT author
//!
//! No characters. No enemies. No combat. No LDtk world. No audio. No menus. No
//! persistence. If any of those are linked into the binary, that is the
//! measurement — see `§2e capability footprint` in the slice evidence.
//!
//! # Why `no_characters()` is a call and not an omission
//!
//! `PlatformerAssetsPlugin` refuses to invent an empty character catalog, and its comment says why:
//! *"silently substituting an empty catalog is how a game ships with its bosses drawn as the
//! fallback body and nobody notices."* That is right.
//!
//! So the engine still refuses to guess, and the consumer now has a word for
//! it. That is the whole shape of this slice: not *make the demand go away*,
//! but *make the true answer expressible*.

use ambition_platformer2d::app::prelude::*;

/// This game's ids. A route the shell can reach, and nothing else.
pub const MINIMAL_EXPERIENCE: &str = "minimal_game";
pub const MINIMAL_LAUNCHER_ROUTE: &str = "minimal_game_launcher";
pub const MINIMAL_GAMEPLAY_ROUTE: &str = "minimal_game_gameplay";

/// The window title the visible face uses, shared so a test and a binary cannot
/// disagree about which app they are talking about.
pub const MINIMAL_WINDOW_TITLE: &str = "Ambition — minimal game";

/// The entire game.
#[derive(Default)]
pub struct MinimalModule;

impl GameModule for MinimalModule {
    fn manifest(&self) -> ModuleManifest {
        // No asset source: this game authors no art. The engine's own tree
        // still resolves, which is itself part of the measurement — a game that
        // ships no assets should not have to pretend it does.
        ModuleManifest::new(MINIMAL_EXPERIENCE)
    }

    fn define(&self, module: &mut ModuleDraft) {
        module
            .experience(MINIMAL_EXPERIENCE)
            .launcher_route(MINIMAL_LAUNCHER_ROUTE)
            .gameplay_route(MINIMAL_GAMEPLAY_ROUTE)
            .characters(minimal_experience::MINIMAL_ROSTER_RON)
            .no_audio()
            .playable(
                "Minimal Game",
                "Movement only — the smallest thing that is still a game",
                minimal_experience::MINIMAL_CHARACTER_ID,
                minimal_experience::MINIMAL_ROOM_ID,
                vec![minimal_experience::minimal_room()],
            )
            ;
    }
}

// `MinimalExperiencePlugin` stood here. It registered the route, then only the
// audio fragment, and now nothing: `playable()` declares the experience and
// `no_audio()` declares the silence.
//
// The whole game is now a declaration. It installs no plugin, spawns no system, and touches no
// `App`.

pub mod minimal_experience;
