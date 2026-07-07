//! Intro sequence story content.
//!
//! This submodule layers named story content on top of generic sandbox systems:
//! intro dialogue nodes, cutscene scripts and room bindings, placeholder NPC sprite
//! registry rows, banter, route state, and the [`plugin::IntroPlugin`] that installs
//! them into live sandbox resources.
//!
//! Keeping intro content isolated here preserves the sandbox/game split: generic
//! machinery stays in `ambition_actors`, while narrative content lives in the game
//! content layer.

pub mod banter;
pub mod cutscene;
pub mod dialog;
pub mod plugin;
pub mod route_state;
pub mod sprites;

#[cfg(test)]
mod tests;

pub use plugin::IntroPlugin;
