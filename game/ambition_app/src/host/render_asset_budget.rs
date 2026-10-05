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

use ambition_platformer2d::load_presentation::LoadPresentationModel;
use ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState;

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

/// Whether a loading cover hides the frame: a room transition's cover is up,
/// or a load foreground is showing.
pub fn frame_is_covered(transitions: Option<&RoomTransitionLoadState>, model: Option<&LoadPresentationModel>) -> bool {
    transitions
        .and_then(|transitions| transitions.active.as_ref())
        .is_some_and(|active| active.cover_required && active.cover_presented)
        || model.is_some_and(|model| model.visible)
}

/// Lift the upload budget while a cover hides the frame; restore it when
/// gameplay is visible again.
pub fn pace_render_asset_uploads(
    transitions: Option<Res<RoomTransitionLoadState>>,
    model: Option<Res<LoadPresentationModel>>,
    mut budget: ResMut<RenderAssetBytesPerFrame>,
) {
    let max_bytes = (!frame_is_covered(transitions.as_deref(), model.as_deref())).then(visible_upload_bytes_per_frame);
    if budget.max_bytes != max_bytes {
        budget.max_bytes = max_bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The budget is lifted only while something covers the frame; visible
    /// gameplay always has one.
    #[test]
    fn uploads_are_paced_only_while_gameplay_is_visible() {
        assert!(!frame_is_covered(None, None));
        let mut model = LoadPresentationModel::default();
        assert!(!frame_is_covered(None, Some(&model)), "a load in its hidden grace covers nothing");
        model.visible = true;
        assert!(frame_is_covered(None, Some(&model)), "a visible load foreground covers the frame");
        let transitions = RoomTransitionLoadState::default();
        assert!(!frame_is_covered(Some(&transitions), None), "no transition, no cover");
        assert!(VISIBLE_UPLOAD_BYTES_PER_FRAME > 0);
    }
}
