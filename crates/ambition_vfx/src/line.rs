//! Shared line looks for content validation and rendering.

use crate::lightning::LightningLook;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaserLook {
    pub glow: [f32; 4],
    pub core: [f32; 4],
    /// The core's width over the ray's.
    pub core_of_width: f32,
    /// The halo's width over the core's.
    pub glow_scale: f32,
    /// Breaths a second.
    pub pulse_hz: f32,
    /// The light where it lands.
    pub contact_glow: f32,
}

impl LaserLook {
    pub const RED: LaserLook = LaserLook {
        glow: [1.0, 0.15, 0.1, 0.55],
        core: [1.0, 0.92, 0.86, 1.0],
        core_of_width: 0.3,
        glow_scale: 3.5,
        pulse_hz: 14.0,
        contact_glow: 22.0,
    };
}

/// How a line is drawn. A ray names one ([`LineLook::named`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineLook {
    Lightning(LightningLook),
    Laser(LaserLook),
}

impl LineLook {
    /// The look a ray strike names, or `None`.
    pub fn named(name: &str) -> Option<LineLook> {
        match name {
            "laser" => Some(LineLook::Laser(LaserLook::RED)),
            other => LightningLook::named(other).map(LineLook::Lightning),
        }
    }

    pub fn flicker_s(&self) -> f32 {
        match self {
            LineLook::Lightning(look) => look.flicker_s,
            LineLook::Laser(_) => f32::INFINITY,
        }
    }
}

