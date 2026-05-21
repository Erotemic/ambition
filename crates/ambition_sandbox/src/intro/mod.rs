//! Intro sequence story content.
//!
//! Everything in this submodule is *content* layered on top of the
//! generic sandbox systems (dialog, cutscenes, sprites, LDtk rooms).
//! The module owns:
//!
//! - [`dialog`] — IntroDialog enum + node tables for the 8 intro NPCs
//!   (Creator wake / final, Oiler, Gate Janitor, Framebreaker,
//!   Nazi salvage guard, news board, manifest kiosk). Hooked into
//!   `crate::dialog::DialogMode::Intro(_)` so the existing dialog
//!   runtime / UI surfaces them with no extra plumbing.
//!
//! - [`cutscene`] — intro cutscene scripts + room→cutscene bindings.
//!   Inserted into [`crate::presentation::cutscene::CutsceneLibrary`] and
//!   [`crate::presentation::cutscene::RoomCutsceneBindings`] by [`plugin::IntroPlugin`]
//!   at startup — sandbox code never references intro by name.
//!
//! - [`sprites`] — placeholder NPC sprite registry rows mapping the
//!   intro NPC names from `intro.ldtk` to existing toon-target
//!   spritesheets (per the doc's placeholder table). Loaded by
//!   [`plugin::IntroPlugin`] into [`crate::presentation::character_sprites::CharacterSpriteAssets`].
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
