//! Encode geometry values for rollback.

use ambition_snapshot::{snapshot_pod, snapshot_unit_enum};

snapshot_pod!(crate::geometry::CenteredAabb {
    center: vec2,
    half_size: vec2,
});

snapshot_unit_enum!(crate::reference_frame::GameplayFramePolicy {
    ControlledBodyLocal = 0,
    AccelerationFrame = 1,
    WorldSpace = 2,
    ScreenSpace = 3,
});
