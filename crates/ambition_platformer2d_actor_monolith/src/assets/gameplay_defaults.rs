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
//! of two fields from the core crate with a `load_embedded` reading a `.ron`; it
//! names nothing in `session` and nothing in the crate at all. It was filed
//! beside the system that first registered a handle for it, and a data type
//! filed beside its first consumer is how a dependency graph acquires an edge
//! nobody intended.
//!
//! ⚠ THE `include_str!` PATH SURVIVED THE MOVE BY ARITHMETIC, NOT BY LUCK BEING
//! CHECKED: it is relative to the FILE, and `src/session/data.rs` and
//! `src/assets/gameplay_defaults.rs` are the same depth. A move one level
//! deeper would have needed it rewritten, silently, at compile time.

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

impl Platformer2dGameplayDefaults {
    pub fn load_embedded() -> Self {
        ron::from_str(include_str!("../../assets/ambition/platformer_defaults.ron"))
            .expect("embedded assets/ambition/platformer_defaults.ron should parse")
    }
}
