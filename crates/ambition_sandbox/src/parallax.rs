//! Lightweight presentation-side parallax backgrounds.
//!
//! Backgrounds are art assets, not gameplay state. They follow the active
//! camera with a configurable parallax factor so distant layers drift more
//! slowly than gameplay tiles. The current default profile is intentionally
//! simple and asset-backed; later room/LDtk metadata can choose profiles per
//! area without changing the rendering mechanics.

mod layers;
mod profiles;
mod systems;

pub use layers::{parallax_layer_translation, ParallaxLayer};
pub use profiles::{default_parallax_profile, ParallaxLayerProfile, ParallaxProfile};
pub use systems::{spawn_default_parallax_background, sync_parallax_layers, ParallaxPlugin};
