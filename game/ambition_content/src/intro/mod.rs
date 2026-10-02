//! Intro sequence story content.
//!
//! This submodule layers named story content on top of generic sandbox systems:
//! intro dialogue nodes, placeholder NPC sprite
//! registry rows, banter, route state, and the [`plugin::IntroPlugin`] that installs
//! them into live sandbox resources. The intro's cutscene scripts are content
//! (`assets/data/cutscenes/intro.ron`), and each intro room names its own in
//! `intro.ldtk`.
//!
//! Keeping intro content isolated here preserves the sandbox/game split: generic
//! machinery stays in `ambition_platformer2d_actor_monolith`, while narrative content lives in the game
//! content layer.

pub mod banter;
pub mod dialog;
pub mod plugin;
pub mod route_state;
pub mod sprites;

#[cfg(test)]
mod tests;

pub use plugin::IntroPlugin;
