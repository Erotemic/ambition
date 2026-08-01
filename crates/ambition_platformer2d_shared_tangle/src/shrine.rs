//! Shared presentation pulse state for save/heal shrines.

use bevy::prelude::Resource;

#[derive(Resource, Default)]
pub struct ShrineActivationPulse {
    pub remaining: f32,
}
