//! The gameplay defaults asset: what a fresh platformer body starts with.
//!
//! ⭐⭐ IT LIVES IN `assets` BECAUSE IT IS AN ASSET, AND THAT ONE LINE OF
//! HOUSEKEEPING IS WORTH TWO MODULES. Measured 2026-09-06: the residual actor
//! kernel has ONE strongly-connected component of 15 top-level modules, and
//! `assets -> session` — a SINGLE `use` of this type, from `assets/loading.rs`
//! — was what held `assets` and `character_sprites` inside it. Removing that one
//! edge drops the knot from 15 modules to 13.
//!
//! ⛔ THE TYPE NEVER BELONGED TO `session`. It is a `Deserialize + Asset` struct
//! of two fields from the core crate with a `load` reading a `.ron`; it
//! names nothing in `session` and nothing in the crate at all. It was filed
//! beside the system that first registered a handle for it, and a data type
//! filed beside its first consumer is how a dependency graph acquires an edge
//! nobody intended.
//!
//! ⚠ THE `include_str!` PATH IS RELATIVE TO THIS FILE: a move one level deeper
//! needs it rewritten. It compiles only under `static_content`, so a default
//! check does not see a wrong path; `cargo check --features static_content`
//! does. The disk path is from the crate directory and does not move.

use bevy::prelude::Resource;
use bevy::asset::Asset;
use bevy::reflect::TypePath;
use serde::Deserialize;

use ambition_platformer2d_core as ae;

pub const PLATFORMER_DEFAULTS_ASSET: &str = "ambition/platformer_defaults.ron";

#[derive(Clone, Debug, Deserialize, Asset, TypePath, Resource)]
pub struct Platformer2dGameplayDefaults {
    pub abilities: ae::AbilitySet,
    pub tuning: ae::MovementTuning,
}

/// The defaults file in the source tree.
const PLATFORMER_DEFAULTS_FILE: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/ambition/platformer_defaults.ron");

/// The defaults text, when the build embeds it (`static_content`).
///
/// An `include_str!` is a compilation input: embedded unconditionally, a
/// movement tuning edit compiled this crate and the 14 crates after it (14.10 s
/// measured, 2026-10-01).
#[cfg(feature = "static_content")]
const PLATFORMER_DEFAULTS_RON: Option<&str> =
    Some(include_str!("../../assets/ambition/platformer_defaults.ron"));
#[cfg(not(feature = "static_content"))]
const PLATFORMER_DEFAULTS_RON: Option<&str> = None;

impl Platformer2dGameplayDefaults {
    /// The authored defaults: embedded under `static_content`, else read off
    /// disk, so an edit costs a restart and no Rust build.
    ///
    /// # Panics
    ///
    /// When the file cannot be read or does not parse. The game cannot start
    /// without its movement defaults, and the message names the file.
    pub fn load() -> Self {
        let text = match PLATFORMER_DEFAULTS_RON {
            Some(text) => std::borrow::Cow::Borrowed(text),
            None => std::borrow::Cow::Owned(std::fs::read_to_string(PLATFORMER_DEFAULTS_FILE).unwrap_or_else(
                |error| {
                    panic!(
                        "{PLATFORMER_DEFAULTS_FILE} is not readable ({error}); a build with no source \
                         tree must turn on `static_content`"
                    )
                },
            )),
        };
        ron::from_str(&text)
            .unwrap_or_else(|error| panic!("{PLATFORMER_DEFAULTS_FILE} does not parse: {error}"))
    }
}
