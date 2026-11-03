//! The per-frame GPU upload budget: Bevy's
//! [`RenderAssetBytesPerFrame`], the ONE pacing authority for asset arrival
//! (`docs/planning/engine/readiness-driven-room-transitions.md`).
//!
//! The hitch asset loading causes is the render world preparing many finished
//! images in one frame (the 542 ms hall-entry frame, 150 MP uploaded at once).
//! Decoding runs off the main thread and is not paced at all: every demanded
//! character starts on the frame it is demanded. What is paced is the upload,
//! in real bytes (`GpuImage` reports its byte length), at the stage that costs
//! the frame.
//!
//! * While gameplay is visible, uploads spend at most
//!   [`VISIBLE_UPLOAD_BYTES_PER_FRAME`] a frame; Bevy defers whole images past
//!   it to later frames, always admitting at least one so nothing stalls.
//! * While a loading cover hides the frame, the budget is lifted: a hitch
//!   nobody sees is the fastest way to be ready.
//!
//! `AMBITION_RENDER_ASSET_MB_PER_FRAME` overrides the visible budget, in
//! megabytes (an experiment knob). A value that does not parse as a positive
//! integer is a mistake worth stopping for, not a silent default.

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetBytesPerFrame;

use ambition_platformer2d::render::rendering::presentation_coverage::PresentationCoverage;
use ambition_platformer2d::platformer::lifecycle::SessionRoot;

pub const RENDER_ASSET_MB_PER_FRAME_ENV: &str = "AMBITION_RENDER_ASSET_MB_PER_FRAME";

/// The upload budget while gameplay is visible: 16 MiB, a 2048 x 2048 RGBA
/// page, about a millisecond and a half of PCIe transfer. A starting value,
/// not a measured one: the frame-time/byte relationship on the GPU host is
/// still owed (readiness plan, "Background streaming").
pub const VISIBLE_UPLOAD_BYTES_PER_FRAME: usize = 16 * 1024 * 1024;

pub fn render_asset_mb_per_frame() -> Option<usize> {
    static VALUE: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    *VALUE.get_or_init(|| {
        let raw = std::env::var(RENDER_ASSET_MB_PER_FRAME_ENV).ok()?;
        match raw.trim().parse::<usize>() {
            Ok(mb) if mb > 0 => Some(mb),
            _ => panic!("{RENDER_ASSET_MB_PER_FRAME_ENV}={raw:?} is not a positive whole number of megabytes"),
        }
    })
}

/// The budget while gameplay is visible: the knob's, else the default.
pub fn visible_upload_bytes_per_frame() -> usize {
    render_asset_mb_per_frame().map_or(VISIBLE_UPLOAD_BYTES_PER_FRAME, |mb| mb * 1024 * 1024)
}

/// Install the budget and the system that lifts it under a loading cover.
pub fn install_render_asset_budget(app: &mut App) {
    app.insert_resource(RenderAssetBytesPerFrame::new(visible_upload_bytes_per_frame()))
        .add_systems(Last, pace_render_asset_uploads);
}

/// Lift the upload budget while a cover hides the frame; restore it when
/// gameplay is visible again.
pub fn pace_render_asset_uploads(
    mut coverage: PresentationCoverage,
    sessions: Query<(), With<SessionRoot>>,
    mut budget: ResMut<RenderAssetBytesPerFrame>,
) {
    let max_bytes = (!sessions.is_empty() && !coverage.hides_all_gameplay()).then(visible_upload_bytes_per_frame);
    if budget.max_bytes != max_bytes {
        budget.max_bytes = max_bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::platformer::lifecycle::SessionScopeId;
    use ambition_platformer2d::presentation::gameplay_presentation::PresentationCover;

    #[test]
    fn a_cover_over_one_view_keeps_the_gameplay_upload_budget() {
        let mut app = App::new();
        app.insert_resource(RenderAssetBytesPerFrame::new(visible_upload_bytes_per_frame()));
        app.add_systems(Update, pace_render_asset_uploads);
        let session = app.world_mut().spawn(SessionRoot(SessionScopeId(1))).id();
        let cover = app.world_mut().spawn((PresentationCover,
            ComputedNode { size: Vec2::new(640.0, 720.0), ..default() },
            UiGlobalTransform::from_xy(320.0, 360.0))).id();
        app.update();
        assert_eq!(app.world().resource::<RenderAssetBytesPerFrame>().max_bytes, Some(visible_upload_bytes_per_frame()));
        app.world_mut().get_mut::<ComputedNode>(cover).unwrap().size.x = 1280.0;
        *app.world_mut().get_mut::<UiGlobalTransform>(cover).unwrap() = UiGlobalTransform::from_xy(640.0, 360.0);
        app.update();
        assert_eq!(app.world().resource::<RenderAssetBytesPerFrame>().max_bytes, None);
        app.world_mut().entity_mut(cover).despawn();
        app.update();
        assert_eq!(app.world().resource::<RenderAssetBytesPerFrame>().max_bytes, Some(visible_upload_bytes_per_frame()));
        app.world_mut().entity_mut(session).despawn();
        app.update();
        assert_eq!(app.world().resource::<RenderAssetBytesPerFrame>().max_bytes, None);
    }
}
