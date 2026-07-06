//! Declarative movement paths for moving platforms, spike balls, patrol
//! dummies, and scripted hazards.
//!
//! This is world/geometry vocabulary: a path is a list of `Vec2` waypoints
//! plus a speed and playback mode. It lives beside `World`/`Block`/
//! `SurfaceChain` in the engine because it carries spatial primitives and is
//! authored INPUT to the sim, not actor mind/behavior data.

use crate::Vec2;

/// Declarative movement path for moving platforms, spike balls, patrol dummies,
/// and later scripted boss hazards.
#[derive(Clone, Debug, PartialEq)]
pub struct KinematicPath {
    pub points: Vec<Vec2>,
    pub speed: f32,
    pub mode: KinematicPathMode,
    pub start_offset_seconds: f32,
}

impl KinematicPath {
    pub fn line(a: Vec2, b: Vec2, speed: f32) -> Self {
        Self {
            points: vec![a, b],
            speed,
            mode: KinematicPathMode::PingPong,
            start_offset_seconds: 0.0,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.points.len() >= 2 && self.speed > 0.0
    }
}

/// Playback style for a kinematic path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KinematicPathMode {
    Once,
    Loop,
    PingPong,
}
