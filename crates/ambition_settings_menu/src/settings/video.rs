//! Video options that a host can add to its own menu.

use super::{cycle, enum_index, SettingsOption, SettingsOptionId};
use ambition_persistence::settings::video::{
    FramePaceCap, GameplayViewShape, VideoSettings, VsyncMode,
};

/// Select only the options that the host supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoMenuOption {
    /// Requires a presentation that uses the gameplay view shape.
    ViewShape,
    /// Requires the host frame limiter.
    FramePacing,
    /// Requires a window that uses the V-sync setting.
    Vsync,
}

impl VideoMenuOption {
    pub const fn key(self) -> &'static str {
        match self {
            Self::ViewShape => "view-shape",
            Self::FramePacing => "frame-cap",
            Self::Vsync => "vsync",
        }
    }

    /// Read the current value for a menu row.
    pub fn row(self, v: &VideoSettings) -> SettingsOption {
        match self {
            Self::ViewShape => {
                let (i, n) = enum_index(&GameplayViewShape::ALL, v.view_shape);
                cycle(
                    SettingsOptionId::ViewShape,
                    "View Shape",
                    v.view_shape.label(),
                    i,
                    n,
                    "Use a 4:3 view to draw fewer pixels.",
                )
            }
            Self::FramePacing => {
                let (i, n) = enum_index(&FramePaceCap::ALL, v.frame_cap);
                cycle(
                    SettingsOptionId::FramePacing,
                    "Frame Cap",
                    v.frame_cap.label(),
                    i,
                    n,
                    "Limit the frame rate. Auto follows the display refresh.",
                )
            }
            Self::Vsync => {
                let (i, n) = enum_index(&VsyncMode::ALL, v.vsync);
                cycle(
                    SettingsOptionId::Vsync,
                    "V-Sync",
                    v.vsync.label(),
                    i,
                    n,
                    "Synchronize frame presentation with the display refresh.",
                )
            }
        }
    }

    /// Select the previous value for a negative direction, or the next value.
    pub fn adjust(self, video: &mut VideoSettings, direction: i32) {
        match self {
            Self::ViewShape => {
                video.view_shape = if direction < 0 {
                    GameplayViewShape::prev(video.view_shape)
                } else {
                    GameplayViewShape::next(video.view_shape)
                };
            }
            Self::FramePacing => {
                video.frame_cap = if direction < 0 {
                    FramePaceCap::prev(video.frame_cap)
                } else {
                    FramePaceCap::next(video.frame_cap)
                };
            }
            Self::Vsync => {
                video.vsync = if direction < 0 {
                    VsyncMode::prev(video.vsync)
                } else {
                    VsyncMode::next(video.vsync)
                };
            }
        }
    }
}
