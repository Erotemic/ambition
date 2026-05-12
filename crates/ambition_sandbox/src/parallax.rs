//! Lightweight presentation-side parallax backgrounds.
//!
//! Backgrounds are art assets, not gameplay state. They follow the active
//! camera with a configurable parallax factor so distant layers drift more
//! slowly than gameplay tiles. Profiles are chosen from room metadata (biome /
//! visual theme), which keeps the renderer data-driven without entangling it in
//! gameplay simulation.

mod layers;
mod profiles;
mod systems;

pub use layers::{parallax_layer_translation, ParallaxLayer};
pub use profiles::{
    cave_parallax_profile, cove_parallax_profile, default_parallax_profile, parallax_profile_named,
    select_parallax_profile, ParallaxLayerProfile, ParallaxProfile,
};
pub use systems::{sync_parallax_layers, ParallaxPlugin};
