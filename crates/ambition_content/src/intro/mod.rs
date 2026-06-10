//! Intro sequence story content.
//!
//! Everything in this submodule is *content* layered on top of the
//! generic sandbox systems (dialog, cutscenes, sprites, LDtk rooms).
//! The module owns:
//!
//! - [`dialog`] — IntroDialog enum + node tables for the 8 intro NPCs
//!   (Creator wake / final, Oiler, Gate Janitor, lab raider,
//!   salvage guard, news board, manifest kiosk). Hooked into
//!   `ambition_sandbox::dialog::DialogMode::Intro(_)` so the existing dialog
//!   runtime / UI surfaces them with no extra plumbing.
//!
//! - [`cutscene`] — intro cutscene scripts + room→cutscene bindings.
//!   Inserted into [`ambition_sandbox::presentation::cutscene::CutsceneLibrary`] and
//!   [`ambition_sandbox::presentation::cutscene::RoomCutsceneBindings`] by [`plugin::IntroPlugin`]
//!   at startup — sandbox code never references intro by name.
//!
//! - [`sprites`] — placeholder NPC sprite registry rows mapping the
//!   intro NPC names from `intro.ldtk` to existing toon-target
//!   spritesheets (per the doc's placeholder table). Loaded by
//!   [`plugin::IntroPlugin`] into [`ambition_sandbox::presentation::character_sprites::CharacterSpriteAssets`].
//!
//! - [`plugin`] — [`IntroPlugin`], the Bevy plugin that runs the
//!   startup systems wiring intro content into the live sandbox
//!   resources.
//!
//! Keeping intro content isolated here is intentional groundwork for a
//! future `ambition_sandbox` vs `ambition_game` crate split. The
//! sandbox stays a generic engine demo; the game wraps it with
//! narrative content like this module.

pub mod banter;
pub mod cutscene;
pub mod dialog;
pub mod plugin;
pub mod route_state;
pub mod sprites;

#[cfg(test)]
mod tests;

pub use plugin::IntroPlugin;
