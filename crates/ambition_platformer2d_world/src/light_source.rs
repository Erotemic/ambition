//! A lamp in a room: a light that a dark room's light map draws
//! (`docs/planning/engine/dark-rooms-and-tunnels.md`, DR2). Presentation
//! only: no simulation fact reads it.

use ambition_platformer2d_core::Vec2;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LightSource {
    /// Its middle, in room coordinates.
    pub position: Vec2,
    /// How far its light reaches, in world units.
    pub radius: f32,
    /// Its colour, linear.
    pub color: [f32; 3],
    /// How much of the darkness it takes away at its middle, 0 to 1.
    pub intensity: f32,
}
