//! `AMBITION_RENDER_ASSET_MB_PER_FRAME` — the per-frame GPU upload budget, in
//! megabytes, handed to Bevy's [`bevy::render::render_asset::RenderAssetBytesPerFrame`].
//! An experiment knob for the asset campaign (see
//! `docs/planning/engine/asset-preparation-and-residency.md`); unset means
//! Bevy's unlimited default. Read once. A value that does not parse as a
//! positive integer is a mistake worth stopping for, not a silent "unlimited".

pub const RENDER_ASSET_MB_PER_FRAME_ENV: &str = "AMBITION_RENDER_ASSET_MB_PER_FRAME";

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
