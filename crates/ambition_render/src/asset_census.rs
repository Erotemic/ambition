//! Text census of texture decoding, in the style of the `[startup]` and
//! `[schedule-census]` loggers in `ambition_dev_tools::profiling`.
//!
//! Launch profiles kept showing the same shape: the first seconds dominated by
//! `fdeflate::Decompressor::read` and `png::filter::paeth::unfilter` across the
//! IO task pools, while the startup logger reported ~100ms and looked innocent.
//! Native symbols name the DECODER but never the ASSET, so a profile could
//! prove "we are decoding PNGs" and never answer the question that matters:
//! WHICH sheets, HOW MANY megapixels, and WHEN.
//!
//! This answers exactly that, on stderr, so `scripts/profile_desktop.sh` stamps
//! it into the timeline chunk the decode happened in.

use bevy::asset::AssetEvent;
use bevy::image::Image;
use bevy::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

/// Running totals for decoded images.
#[derive(Resource)]
pub struct ImageCensus {
    #[cfg(not(target_arch = "wasm32"))]
    started_at: Instant,
    #[cfg(not(target_arch = "wasm32"))]
    window_started_at: Instant,
    total_images: u64,
    total_megapixels: f64,
    // accumulated on every platform, REPORTED only where there is a periodic
    // census to print — the report is `not(wasm32)`, because a browser build has
    // no terminal to print a rolling window to. Keeping the fields identical
    // across platforms keeps the accounting identical; only the readout differs.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    total_bytes: u64,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    window_images: u64,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    window_megapixels: f64,
    /// How many images' bytes were DERIVED from the texture descriptor rather
    /// than measured, because their CPU copy had been dropped.
    ///
    /// ⭐ Reported so the total says how much of itself it actually saw. A byte
    /// count that silently switches from measured to derived is the same class of
    /// lie as a count of zero from an instrument that never reports the category.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    derived_byte_images: u64,
}

impl Default for ImageCensus {
    fn default() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            started_at: Instant::now(),
            #[cfg(not(target_arch = "wasm32"))]
            window_started_at: Instant::now(),
            total_images: 0,
            total_megapixels: 0.0,
            total_bytes: 0,
            window_images: 0,
            window_megapixels: 0.0,
            derived_byte_images: 0,
        }
    }
}

impl ImageCensus {
    /// Per-image lines only for sheets big enough to matter. Below this a
    /// texture is a UI glyph or an icon, and one line each would bury the
    /// sheets that actually cost decode time.
    pub const NOTABLE_MEGAPIXELS: f64 = 1.0;
    /// Rollup cadence, matching the frame census so the two interleave
    /// readably in one log.
    pub const WINDOW_SECS: f64 = 5.0;

    /// Total pixels decoded so far, in megapixels.
    pub fn total_megapixels(&self) -> f64 {
        self.total_megapixels
    }

    /// Total decoded images seen so far.
    pub fn total_images(&self) -> u64 {
        self.total_images
    }

    /// Bytes of decoded image data seen so far. Cumulative, never decremented:
    /// this counts DECODE WORK, so a rise with a flat `total_images` means the
    /// same asset was decoded again, which is the churn the number is for.
    /// How many of the counted images had their bytes DERIVED rather than
    /// measured. `0` means every byte in `total_bytes` was seen directly.
    pub fn derived_byte_images(&self) -> u64 {
        self.derived_byte_images
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
}

/// Log every notable texture as it lands, plus a periodic rollup.
///
/// `AssetEvent::Added` fires when the asset reaches `Assets<Image>` — after the
/// IO pool decoded it — so these timestamps mark decode COMPLETION, which is
/// what lines up with a frame spike and a sprite re-bind.
#[cfg(not(target_arch = "wasm32"))]
pub fn report_image_census(
    mut events: MessageReader<AssetEvent<Image>>,
    images: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
    mut census: ResMut<ImageCensus>,
    // ⭐ OPTIONAL, because a composition may have no game mode at all (a capture
    // tool, a headless probe). Absent means "cannot tell", which must read as
    // "do not accuse", not as "not during gameplay".
    mode: Option<Res<bevy::state::state::State<
        ambition_platformer2d_shared_tangle::schedule::GameMode,
    >>>,
) {
    let during_gameplay = mode.is_some_and(|mode| mode.get().allows_gameplay());
    for event in events.read() {
        let AssetEvent::Added { id } = event else {
            continue;
        };
        let Some(image) = images.get(*id) else {
            continue;
        };
        let (width, height) = (image.width(), image.height());
        let megapixels = f64::from(width) * f64::from(height) / 1.0e6;
        // ⭐ MEASURE THE CPU COPY WHEN IT EXISTS, DERIVE IT WHEN IT DOES NOT.
        // `image.data` is `None` for an image whose main-world copy was dropped
        // (`RenderAssetUsages::RENDER_WORLD`), and reporting 0 for those would make
        // "decoded bytes" FALL every time somebody moved an asset to render-world
        // only — a spectacular fake win, and the readout could not tell it from a
        // real one. The decode still happened; the pixels still exist on the GPU.
        // ⇒ fall back to the texture's own descriptor: width x height x bytes-per-block.
        let bytes = match image.data.as_ref() {
            Some(data) => data.len() as u64,
            None => {
                census.derived_byte_images += 1;
                let per_pixel = u64::from(
                    image
                        .texture_descriptor
                        .format
                        .block_copy_size(None)
                        .unwrap_or(4),
                );
                u64::from(width) * u64::from(height) * per_pixel
            }
        };

        census.total_images += 1;
        census.total_megapixels += megapixels;
        census.total_bytes += bytes;
        census.window_images += 1;
        census.window_megapixels += megapixels;

        if megapixels >= ImageCensus::NOTABLE_MEGAPIXELS {
            let at = census.started_at.elapsed().as_secs_f64();
            // The asset PATH is the whole point: it is the one thing a perf
            // symbol can never tell you, and the only handle you can act on.
            let path = asset_server
                .get_path(*id)
                .map(|path| path.to_string())
                .unwrap_or_else(|| "<runtime-generated>".to_string());
            // ⭐⭐ `live=` IS EMITTED ON BOTH BRANCHES, DELIBERATELY. A reader
            // that sees no marker at all is reading a log from before this
            // existed, and must say "unknown" rather than "none" — an absent
            // marker is not evidence of a clean run. That is the same trap as a
            // count of zero from an instrument that never reports the category.
            //
            // ⛔ `live=1` is a CONTRACT VIOLATION: a big image decoded while
            // gameplay is running is a frame the player felt. The 2026-08-29
            // hardware run tied every one of five frame-spike clusters to a
            // decode burst, monotone in megapixels, up to a 516ms frame for
            // +307MP of 4096x4096 character sheets.
            //
            // ⚠ A warning, not an error: a legitimately late asset exists (an
            // unpredictable summon, a dev spawn). What is never legitimate is
            // not KNOWING.
            let live = u8::from(during_gameplay);
            // ⛔ A RUNTIME-GENERATED IMAGE IS NOT A CONTENT DECODE, AND TELLING
            // SOMEBODY TO "DEMAND IT AT MATCH PREPARATION" IS ADVICE THEY CANNOT
            // TAKE. Caught within an hour of shipping this warning: a headless
            // match flagged two 2048x2048 `<runtime-generated>` images — an atlas
            // allocated the first time text draws, with no path and no
            // preparation step to move it to. It is still worth REPORTING (it is
            // 16MB a match) but under its own sentence.
            let generated = path == "<runtime-generated>";
            if during_gameplay && generated {
                eprintln!(
                    "[image] {at:8.3}s {width}x{height} {megapixels:6.1}MP live={live} {path} \
                     — allocated during gameplay. No asset path, so this is generated \
                     (an atlas or a render target), not content that could have been \
                     demanded earlier."
                );
            } else if during_gameplay {
                eprintln!(
                    "[image] {at:8.3}s {width}x{height} {megapixels:6.1}MP live={live} {path} \
                     — DECODED DURING GAMEPLAY, so it cost a frame. If a match needs \
                     it, demand it at match preparation."
                );
            } else {
                eprintln!(
                    "[image] {at:8.3}s {width}x{height} {megapixels:6.1}MP live={live} {path}"
                );
            }
        }
    }

    let now = Instant::now();
    if now.duration_since(census.window_started_at).as_secs_f64() < ImageCensus::WINDOW_SECS {
        return;
    }
    // Stay silent through quiet windows: a steady stream of "+0 images" lines
    // would drown the windows that actually decoded something.
    if census.window_images > 0 {
        let at = now.duration_since(census.started_at).as_secs_f64();
        eprintln!(
            "[image-census] {at:8.3}s +{} images (+{:.1}MP) | total {} images, {:.1}MP, {:.1}MB resident",
            census.window_images,
            census.window_megapixels,
            census.total_images,
            census.total_megapixels,
            census.total_bytes as f64 / 1.0e6,
        );
    }
    census.window_images = 0;
    census.window_megapixels = 0.0;
    census.window_started_at = now;
}

/// Wasm has no `Instant`; the census stays a no-op there (use browser devtools).
#[cfg(target_arch = "wasm32")]
pub fn report_image_census(
    mut events: MessageReader<AssetEvent<Image>>,
    _images: Res<Assets<Image>>,
    _asset_server: Res<AssetServer>,
    _census: ResMut<ImageCensus>,
) {
    events.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notable_threshold_admits_sheets_and_rejects_icons() {
        // A 4150x4046 character sheet is the thing worth a line.
        let sheet = f64::from(4150u32) * f64::from(4046u32) / 1.0e6;
        assert!(sheet >= ImageCensus::NOTABLE_MEGAPIXELS);
        // A 64x64 icon is not.
        let icon = f64::from(64u32) * f64::from(64u32) / 1.0e6;
        assert!(icon < ImageCensus::NOTABLE_MEGAPIXELS);
    }

    #[test]
    fn census_starts_empty() {
        let census = ImageCensus::default();
        assert_eq!(census.total_images(), 0);
        assert_eq!(census.total_megapixels(), 0.0);
    }
}
