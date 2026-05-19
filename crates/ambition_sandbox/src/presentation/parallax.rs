//! Lightweight presentation-side parallax backgrounds (DATA-DRIVEN STUB).
//!
//! Backgrounds are art assets, not gameplay state. They follow the active
//! camera with a configurable parallax factor so distant layers drift more
//! slowly than gameplay tiles. Profiles are chosen from room metadata (biome /
//! visual theme), which keeps the renderer data-driven without entangling it in
//! gameplay simulation.
//!
//! ## Status (2026-05-19)
//!
//! The active parallax pipeline lives in
//! [`crate::presentation::rendering::parallax`]. This module is an
//! authored-data prototype (`ParallaxLayer`, `ParallaxProfile`,
//! `ParallaxLayerProfile`, biome-keyed profile factories,
//! `ParallaxPlugin`) staged by the 2026-05-17 themed-module reorg but
//! not yet wired into any `App::add_plugins` call site. The
//! module-wide `allow(dead_code)` below silences the dead-code lint
//! while the wiring patch is still in flight; remove the allow when
//! the plugin is added or delete the module if the migration is
//! abandoned.
#![allow(dead_code)]

mod layers;
mod profiles;
mod systems;
