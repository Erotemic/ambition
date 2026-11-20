//! A body's replicated motion row, and the guest's adoption of it.
//!
//! The host's body is the authority; a guest draws it where the host has it.
//! `write_into` sets the guest's copy to the host's state whole, which no kernel
//! step may admit or alter: the guest's own simulation does not move that body.
//! So it is a named exemption in the pose and velocity authority rules.

use ambition_platformer2d_core::BodyKinematics;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Where a body is, how it moves and which way it faces: what a guest needs to
/// draw it where the host has it (NA-2's 5 floats).
#[derive(Component, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyRow {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub facing: f32,
}

impl BodyRow {
    pub fn of(body: &BodyKinematics) -> Self {
        Self { pos: body.pos.to_array(), vel: body.vel.to_array(), facing: body.facing }
    }

    pub fn write_into(&self, body: &mut BodyKinematics) {
        body.pos = Vec2::from_array(self.pos);
        body.vel = Vec2::from_array(self.vel);
        body.facing = self.facing;
    }
}
