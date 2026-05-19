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

// All items in `layers`, `profiles`, and `systems` are consumed
// internally by their siblings; nothing outside `presentation::parallax`
// touches them, so no pub-use re-exports are needed at this facade.
