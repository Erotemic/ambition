//! The smallest thing A9 calls a supported profile: a body that moves against
//! world geometry, with no renderer, no audio, no inventory, no encounters and
//! no game content.
//!
//! # What it deliberately does NOT ask for
//!
//! Its manifest names `ambition_platformer2d` with `default-features = false`
//! and no feature list. Anything in its compile closure is something the engine
//! supplies implicitly, and that is the measurement — see the A9 row.
//!
//! # Why it is a separate fixture from `minimal_game`
//!
//! `minimal_game` boots BOTH faces from one module on purpose, and its windowed
//! arm is the slice-B leak it was written to pin. That makes it the windowed
//! sentinel: it asks for the renderer, so the renderer is legitimately in its
//! closure and no ratchet built on it can say a featureless consumer links none.

use ambition_platformer2d::app::prelude::*;

pub const HEADLESS_EXPERIENCE: &str = "headless_profile";
pub const HEADLESS_LAUNCHER_ROUTE: &str = "headless_profile_launcher";
pub const HEADLESS_GAMEPLAY_ROUTE: &str = "headless_profile_gameplay";

/// The entire game: one walker, one floor.
#[derive(Default)]
pub struct HeadlessModule;

impl GameModule for HeadlessModule {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest::new(HEADLESS_EXPERIENCE)
    }

    fn define(&self, module: &mut ModuleDraft) {
        module
            .experience(HEADLESS_EXPERIENCE)
            .launcher_route(HEADLESS_LAUNCHER_ROUTE)
            .gameplay_route(HEADLESS_GAMEPLAY_ROUTE)
            .characters(experience::ROSTER_RON)
            .no_audio()
            .playable(
                "Headless Profile",
                "A body and a floor — the A9 minimum, and nothing else",
                experience::CHARACTER_ID,
                experience::ROOM_ID,
                vec![experience::room()],
            );
    }
}

pub mod experience;
