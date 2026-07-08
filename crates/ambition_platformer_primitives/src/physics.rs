//! Shared secondary-physics settings resource.
//!
//! The Avian adapter stays in `ambition_actors::world::physics`; this tiny
//! resource lives below render/app so presentation systems can receive the same
//! settings value without depending on actor machinery.

use bevy::prelude::Resource;

/// Runtime switch/tuning for secondary physics. It intentionally does not
/// affect the custom platformer controller.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PhysicsSandboxSettings {
    pub debris_enabled: bool,
    pub static_room_colliders: bool,
    pub default_lifetime: f32,
}

impl Default for PhysicsSandboxSettings {
    fn default() -> Self {
        Self {
            debris_enabled: true,
            static_room_colliders: true,
            default_lifetime: 4.2,
        }
    }
}
