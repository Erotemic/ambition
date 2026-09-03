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

use ambition_sprite_sheet::game_assets::image_stages;
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

    /// How many of the counted images had their bytes DERIVED rather than
    /// measured. `0` means every byte in `total_bytes` was seen directly.
    pub fn derived_byte_images(&self) -> u64 {
        self.derived_byte_images
    }

    /// Bytes of decoded image data seen so far. Cumulative, never decremented:
    /// this counts DECODE WORK, so a rise with a flat `total_images` means the
    /// same asset was decoded again, which is the churn the number is for.
    ///
    /// ⚠ DECODE, NOT GPU-READY. This counts `AssetEvent::Added`, which fires when
    /// the image reaches `Assets<Image>` — the main world is done with it and the
    /// render world has not touched it yet. The frame cost measured on hardware is
    /// the EXTRACT that follows (`extract_render_asset<GpuImage>`, 454.9ms max
    /// against a 0.1ms mean), so "decoded" here is upstream of "ready to draw".
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
}

/// Log every notable texture as it lands, plus a periodic rollup.
///
/// `AssetEvent::Added` fires when the asset reaches `Assets<Image>` — after the
/// IO pool decoded it — so these timestamps mark decode COMPLETION, which is
/// what lines up with a frame spike and a sprite re-bind.
// ⛔ A `#[cfg]` GATES THE NEXT ITEM ONLY. The constant below was inserted
// between this function's gate and the function, which silently moved the gate
// onto the constant and made the NATIVE census unconditional: 16 wasm errors
// ("defined multiple times", `Instant`) that only the web job — which the
// default gate plan does not run — could see.
/// How long a demand→first-draw wait has to be before `[image-drawn]` says so.
///
/// A tenth of a second: six frames at 60Hz, which is the point a sprite arriving
/// stops being a load and starts being a POP the player can see. Shorter waits
/// are the ordinary cost of streaming and would bury the ones that are not.
///
/// ⛔ Native-only, matching its ONLY consumer `stamp_first_drawn_images`. That
/// system stayed `not(wasm32)` when the GPU stamp beside it stopped being — the
/// distinction the web-reveal fix turns on: first-draw is pure telemetry
/// measured in `Instant`s, so it may be gated; the readiness fact may not.
#[cfg(not(target_arch = "wasm32"))]
const NOTABLE_DRAW_WAIT: std::time::Duration = std::time::Duration::from_millis(100);

/// How many unrouted images the census names before it says `+N more`.
///
/// Eight, because the bucket is meant to be small: a census that has to print a
/// hundred of these is reporting a different problem, and the count says so.
#[cfg(not(target_arch = "wasm32"))]
const UNROUTED_NAMED: usize = 8;

#[cfg(not(target_arch = "wasm32"))]
pub fn report_image_census(
    mut events: MessageReader<AssetEvent<Image>>,
    images: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
    mut census: ResMut<ImageCensus>,
    // ⭐ OPTIONAL AND PER-APP. Absent means this App never installed a render
    // world, which is the honest reading for a headless probe sharing the
    // process with a rendering sibling — the process-global bool this replaced
    // could not tell the two apart.
    render_world: Option<Res<image_stages::RenderWorldPresent>>,
    // ⭐ OPTIONAL, because a composition may have no game mode at all (a capture
    // tool, a headless probe). Absent means "cannot tell", which must read as
    // "do not accuse", not as "not during gameplay".
    mode: Option<
        Res<bevy::state::state::State<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
    >,
    // The census flushes on the way OUT as well as on the window boundary: a
    // capture that finishes inside one window — which every hall entry does
    // now — otherwise ends with no `[image-census]` line at all, and the run's
    // resident-by-road answer dies with the process.
    mut exits: MessageReader<bevy::app::AppExit>,
    // "Live" means a PLAYER is in a world: the game mode allows gameplay AND
    // a session root exists. The shell host boots in `Playing` with nothing
    // but the launcher on screen, so mode alone stamped every boot decode
    // `live=1 — DECODED DURING GAMEPLAY, so it cost a frame`, which read as a
    // gameplay hitch for art the launcher loads under its own cover.
    sessions: Query<(), With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>>,
) {
    let live_known = mode
        .as_ref()
        .map(|mode| mode.get().allows_gameplay() && !sessions.is_empty());
    let during_gameplay = live_known.unwrap_or(false);
    // The render world reports GPU preparation and has no `GameMode`; tell the
    // shared ledger what the main world knows, once per frame.
    image_stages::ledger().set_gameplay_live(live_known);
    for event in events.read() {
        let id = match event {
            AssetEvent::Added { id } => *id,
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                // Decoded and dropped before any GPU saw it: the wasted half
                // of the decode budget, named per file when it is big.
                let dropped = image_stages::ledger().removed(id.untyped());
                if let Some(dropped) = dropped {
                    if dropped.megapixels >= ImageCensus::NOTABLE_MEGAPIXELS {
                        let at = census.started_at.elapsed().as_secs_f64();
                        eprintln!(
                            "[image-dropped] {at:8.3}s {:6.1}MP {} — decoded, never uploaded ({})",
                            dropped.megapixels,
                            dropped.path.as_deref().unwrap_or("<runtime-generated>"),
                            dropped.demand_phrase(),
                        );
                    }
                }
                continue;
            }
            _ => continue,
        };
        let Some(image) = images.get(id) else {
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
        // Stage 2 of 3 on the ledger (demand is stage 1, GPU preparation is
        // stage 3, stamped by the render world). Every image, not only the
        // notable ones: the GPU stamp needs the id awaited.
        let stages = image_stages::ledger().inserted(
            id.untyped(),
            megapixels,
            live_known,
            asset_server.get_path(id).map(|path| path.to_string()),
            Instant::now(),
        );

        if megapixels >= ImageCensus::NOTABLE_MEGAPIXELS {
            let at = census.started_at.elapsed().as_secs_f64();
            // The asset PATH is the whole point: it is the one thing a perf
            // symbol can never tell you, and the only handle you can act on.
            let path = asset_server
                .get_path(id)
                .map(|path| path.to_string())
                .unwrap_or_else(|| "<runtime-generated>".to_string());
            // Which STAGE was late is the question the hall hitch left open:
            // a decode that finished 600ms after its demand and an upload
            // that stalled a frame look identical in a frame-time trace.
            let mut demand = stages.demand_phrase();
            if stages.insertions_of_path > 1 {
                // The same file decoded again: dropped and demanded back, or
                // loaded under a second id. Either way the pixels were paid
                // for twice (asset open work 5).
                demand.push_str(&format!(" RE-DECODE #{}", stages.insertions_of_path));
            }
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
            // The same frame stamp as `[world-event]`, so "before or after
            // `room-loaded`" is a comparison of two integers rather than of two
            // wall clocks: the census runs in `Last`, after a long activation
            // frame's work, so its time can read AFTER a `room-loaded` that the
            // insertion in `PreUpdate` of the same frame actually preceded.
            let frame = ambition_platformer2d_shared_tangle::world_log::frame();
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
                    "[image] {at:8.3}s f{frame:>7} {width}x{height} {megapixels:6.1}MP live={live} {path} \
                     — allocated during gameplay. No asset path, so this is generated \
                     (an atlas or a render target), not content that could have been \
                     demanded earlier."
                );
            } else if during_gameplay {
                eprintln!(
                    "[image] {at:8.3}s f{frame:>7} {width}x{height} {megapixels:6.1}MP live={live} {path} \
                     {demand} — DECODED DURING GAMEPLAY, so it cost a frame. If a match \
                     needs it, demand it at match preparation."
                );
            } else {
                eprintln!(
                    "[image] {at:8.3}s f{frame:>7} {width}x{height} {megapixels:6.1}MP live={live} {path} {demand}"
                );
            }
        }
    }

    let now = Instant::now();
    let exiting = exits.read().next().is_some();
    if !exiting
        && now.duration_since(census.window_started_at).as_secs_f64() < ImageCensus::WINDOW_SECS
    {
        return;
    }
    // Stay silent through quiet windows: a steady stream of "+0 images" lines
    // would drown the windows that actually decoded something. The exit flush
    // is the exception: it prints whatever the last partial window holds.
    let (
        gpu_count,
        gpu_megapixels,
        gpu_p50,
        gpu_max,
        awaiting,
        re_decodes,
        dropped,
        dropped_mp,
        by_road,
        unrouted,
        unrouted_total,
        never_drawn,
    ) = {
        let mut ledger = image_stages::ledger();
        let (count, megapixels, p50, max) = ledger.take_gpu_window();
        // Who owns what is resident, in the ledger's own words: the road that
        // demanded each image. Printed only on windows that changed something,
        // beside the totals, so a transition's growth reads per owner.
        // ⛔ NAME THE UNROUTED ROWS. `resident_by_road` keys a row whose `source`
        // is `None` as `"?"`, which reads as "some road I did not catch" and is
        // not what it means: those images never passed a stamped demand at all —
        // they were inserted directly rather than decoded from a file. On
        // 2026-09-02 a Hall census reported `? 22×4.5MP` and it was read as a
        // small population of art. It was NO art: every routed count was zero,
        // and a measurement was published on the strength of the misreading.
        let by_road: Vec<String> = ledger
            .resident_by_road()
            .into_iter()
            .map(|(road, (count, mp))| {
                let road = match road {
                    image_stages::ROAD_UNROUTED => "UNROUTED(no demand)",
                    image_stages::ROAD_PROCEDURAL => "PROCEDURAL(no file)",
                    road => road,
                };
                format!("{road} {count}×{mp:.1}MP")
            })
            .collect();
        // ⭐ AND NAME THEM. A count of FILES nobody claims to have asked for is
        // the one row whose next question is always WHICH — the Hall's single
        // unrouted image took a bespoke ledger probe to identify on 2026-09-02,
        // and a host run should not have to repeat that. Capped, because the
        // point is the expensive ones and a census line is not a manifest.
        //
        // ⛔ FILE-BACKED ONLY. Procedural inserts have no load to stamp and can
        // never leave this bucket; listing them would print 24 non-findings and
        // push the real one past the cap.
        let unrouted: Vec<String> = ledger
            .unrouted_resident()
            .into_iter()
            .take(UNROUTED_NAMED)
            .map(|(mp, path)| format!("{mp:.1}MP {path}"))
            .collect();
        let unrouted_total = ledger.unrouted_resident().len();
        // ⛔⛔ ONLY WHERE A DRAW IS POSSIBLE. Without a render world nothing is
        // ever extracted, so EVERY resident image is "never drawn" and the row
        // would accuse a headless run of waste it cannot commit. The ledger's
        // own doc says the two readings need separating; this is the separation.
        let render_world_present =
            image_stages::RenderWorldPresent::from_option(render_world.as_deref());
        let never_drawn: Option<(usize, f64, Vec<String>)> =
            render_world_present.is_present().then(|| {
                let by_road = ledger.never_drawn_by_road();
                let (count, megapixels) =
                    by_road.values().fold((0usize, 0f64), |(n, mp), (c, road_mp)| {
                        (n + c, mp + road_mp)
                    });
                // ⭐ BY OWNER, because "23.2 MP was never drawn" invites `whose?`
                // and the roads are the answer. An eviction conversation starts
                // from an owner, not from a total.
                let rows = by_road
                    .into_iter()
                    .map(|(road, (c, mp))| {
                        let road = match road {
                            image_stages::ROAD_UNROUTED => "UNROUTED",
                            image_stages::ROAD_PROCEDURAL => "PROCEDURAL",
                            road => road,
                        };
                        format!("{road} {c}×{mp:.1}MP")
                    })
                    .collect();
                (count, megapixels, rows)
            });
        (
            count,
            megapixels,
            p50,
            max,
            ledger.awaiting_gpu().len(),
            ledger.re_decodes,
            ledger.dropped_before_gpu,
            ledger.dropped_before_gpu_megapixels,
            by_road,
            unrouted,
            unrouted_total,
            never_drawn,
        )
    };
    if census.window_images > 0 || gpu_count > 0 || exiting {
        let at = now.duration_since(census.started_at).as_secs_f64();
        #[cfg(not(target_arch = "wasm32"))]
        let ms = |d: Option<std::time::Duration>| {
            d.map_or("-".to_string(), |d| {
                format!("{:.0}ms", d.as_secs_f64() * 1e3)
            })
        };
        // The GPU half on the same line as the decode half, so a window that
        // decoded 40MP and uploaded 12MP reads as the backlog it is. `awaiting`
        // is inserted-but-not-yet-prepared: nonzero at the end of a quiet
        // window means the upload pacer (or a missing render world) is holding
        // pixels the main world already paid for.
        eprintln!(
            "[image-census] {at:8.3}s +{} images (+{:.1}MP) | total {} images, {:.1}MP, {:.1}MB resident \
             | gpu +{gpu_count} (+{gpu_megapixels:.1}MP) insert→gpu p50 {} max {} | awaiting gpu {awaiting} \
             | re-decodes {re_decodes} | dropped before gpu {dropped} ({dropped_mp:.1}MP) \
             | never drawn {} | resident by road: {}",
            census.window_images,
            census.window_megapixels,
            census.total_images,
            census.total_megapixels,
            census.total_bytes as f64 / 1.0e6,
            ms(gpu_p50),
            ms(gpu_max),
            // `-` where a draw is not observable at all, which is a different
            // fact from "nothing has been drawn yet".
            never_drawn.map_or("-".to_string(), |(n, mp, rows)| {
                format!("{n} ({mp:.1}MP: {})", rows.join(", "))
            }),
            by_road.join(", "),
        );
        // ⛔ ONE LINE, AND ONLY WHEN THERE IS SOMETHING TO SAY. An unrouted image
        // is either eager loading nobody asked for or a demand road that stamps
        // nothing; both are findings, and neither is readable from a count.
        if unrouted_total > 0 {
            let more = unrouted_total.saturating_sub(unrouted.len());
            let tail = if more > 0 {
                format!(" (+{more} more)")
            } else {
                String::new()
            };
            eprintln!(
                "[image-unrouted] {at:8.3}s {unrouted_total} file(s) decoded with no demand stamp: {}{tail}",
                unrouted.join(", "),
            );
        }
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

/// Stage 3 of the image ledger: the GPU copy exists.
///
/// Runs in the RENDER world after Bevy's `prepare_assets::<GpuImage>`, and asks
/// only about the ids the main world inserted and nobody has yet seen prepared
/// — a handful at a time, not a walk over every texture. The pacer
/// (`RenderAssetBytesPerFrame`) defers uploads across frames, and this is the
/// instrument that shows the deferral: `insert→gpu` grows while `awaiting gpu`
/// on the census line stays nonzero.
///
/// ⛔⛔ THIS RUNS ON EVERY TARGET, AND USED TO BE NATIVE-ONLY. Stamping is the
/// READINESS FACT the web reveal barrier depends on; only the `[image-gpu]`
/// REPORT below needs a clock. Gating the whole system on `not(wasm32)` left
/// the browser with nothing ever prepared, hence never anything awaited, hence
/// a cover that lifted before the GPU had the pixels.
pub fn stamp_gpu_prepared_images(
    gpu_images: Res<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>,
    // ⛔ THIS APP'S SET, not the process ledger. Asset ids are App-local and this
    // repository has measured them colliding, so a global "id 7 is prepared"
    // let one App's upload lift another App's cover. The `Arc` inside is the
    // same one this App's main world reads; a sibling App holds a different one.
    prepared_here: Option<Res<image_stages::AppGpuPreparedImages>>,
    // ⛔ THE CLOCK IS NATIVE-ONLY AND THE FACT ABOVE IS NOT. Gating this whole
    // system on the clock's target is what left the browser with nothing ever
    // prepared, hence nothing awaited, hence a cover that lifted early.
    #[cfg(not(target_arch = "wasm32"))] started_at: Res<ImageStageClock>,
) {
    let mut ledger = image_stages::ledger();
    if ledger.awaiting_gpu().is_empty() {
        return;
    }
    let prepared: Vec<_> = ledger
        .awaiting_gpu()
        .iter()
        .copied()
        .filter(|id| {
            id.try_typed::<Image>()
                .is_ok_and(|id| gpu_images.get(id).is_some())
        })
        .collect();
    if prepared.is_empty() {
        return;
    }
    // ⭐ THE AUTHORITATIVE WRITE, and it happens BEFORE the ledger mirror below
    // and independently of it. The ledger's `gpu_prepared` consumes a row and
    // returns `None` when there is nothing to report — a census concern. Reveal
    // readiness must not inherit that early-exit, so it is stamped here for
    // every id the GPU actually has.
    //
    // ⛔ UNCONDITIONAL ON PURPOSE. This is the readiness FACT the web reveal
    // barrier reads; the clock below is the only native-only part.
    if let Some(prepared_here) = prepared_here.as_deref() {
        for id in &prepared {
            prepared_here.mark_prepared(*id);
        }
    }

    // ⛔ ONE `#[cfg]` BOUNDARY, NOT EIGHT. An earlier draft gated eight separate
    // statements, which is the shape that put a `#[cfg]` on the wrong item and
    // broke the whole web build once already. The FACT above is unconditional;
    // everything that needs a clock lives behind this one boundary.
    #[cfg(not(target_arch = "wasm32"))]
    let clock = (Instant::now(), ledger.gameplay_live());

    for id in prepared {
        let stamped = ledger.gpu_prepared(
            id,
            #[cfg(not(target_arch = "wasm32"))]
            Some(clock.0),
        );
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(stages) = stamped {
            report_gpu_prepared(&stages, clock.0, clock.1, started_at.0);
        }
        #[cfg(target_arch = "wasm32")]
        let _ = stamped;
    }
}

/// The `[image-gpu]` line. Native-only because every field it prints is a
/// duration, and the web has no `Instant` to measure one against — which is
/// exactly the distinction that had to be drawn: the REPORT needs a clock, the
/// readiness FACT above does not.
#[cfg(not(target_arch = "wasm32"))]
fn report_gpu_prepared(
    stages: &image_stages::ImageStages,
    now: Instant,
    live: Option<bool>,
    started_at: Instant,
) {
    if stages.megapixels < ImageCensus::NOTABLE_MEGAPIXELS {
        return;
    }
    let at = now.duration_since(started_at).as_secs_f64();
    let ms = |d: Option<std::time::Duration>| {
        d.map_or("-".to_string(), |d| {
            format!("{:.0}ms", d.as_secs_f64() * 1e3)
        })
    };
    // `live=` mirrors the `[image]` line: a big upload while gameplay is live
    // is a frame the player felt, whichever stage owned the wait.
    let live = live.map_or("?".to_string(), |live| u8::from(live).to_string());
    eprintln!(
        "[image-gpu] {at:8.3}s {:6.1}MP live={live} {} insert→gpu {} demand→insert {} via {}",
        stages.megapixels,
        stages.path.as_deref().unwrap_or("<runtime-generated>"),
        ms(stages.insert_to_gpu()),
        ms(stages.demand_to_insert()),
        stages.source.unwrap_or("?"),
    );
}

/// Stamp the FOURTH stage: this image was extracted, so this frame would draw it.
///
/// ⭐⭐ THE FIRST STAGE THAT IS ABOUT USE. Demand, insert and GPU all say the
/// asset ARRIVED; none says anybody wanted it on screen. `ExtractedSprites` is
/// filled AFTER visibility culling, so an id appearing there means "this frame
/// would draw it", which is the honest meaning of resident use and closer than
/// anything the three earlier stages can say.
///
/// ⛔ `SpriteBatch` IS ONE STEP LATER AND STRICTLY STRONGER — it survived
/// batching — at the cost of running after `RenderSystems::Queue`. Extraction is
/// preferred until a measurement shows it over-reports, which is the trade the
/// scoping note asked for.
///
/// ⛔⛔ AND IT WRITES AT MOST ONCE PER IMAGE. Extraction runs every frame for
/// every visible sprite; `ImageStageLedger::first_drawn` returns `None` after
/// the first stamp, so the hot path here is a lock and a walk over the extracted
/// list, and the ledger does not grow a per-frame write on the whole visible
/// set. Without that rule this instrument's own cost would be part of what it
/// measures.
#[cfg(not(target_arch = "wasm32"))]
pub fn stamp_first_drawn_images(
    sprites: Res<bevy::sprite_render::ExtractedSprites>,
    started_at: Res<ImageStageClock>,
) {
    if sprites.sprites.is_empty() {
        return;
    }
    let now = Instant::now();
    let mut ledger = image_stages::ledger();
    let covered = ledger.saw_covered_frame();
    for sprite in sprites.sprites.iter() {
        let Some(waited) = ledger.first_drawn(sprite.image_handle_id.untyped(), now) else {
            continue;
        };
        let Some(stages) = ledger.get(sprite.image_handle_id.untyped()) else {
            continue;
        };
        // ⛔⛔ GATED ON THE WAIT, NOT THE SIZE, and the other stages' threshold
        // would have made this line unprintable in exactly the rooms it is for.
        // `NOTABLE_MEGAPIXELS` is 1.0 and its rationale — "below this a texture
        // is a UI glyph or an icon" — was written for FULL-tier art. Under the
        // room sprite-tier cap a whole character sheet is about 0.3 MP, so in
        // the capped gallery (the one room with a measured hitch) nothing would
        // ever have reached the bar: measured, zero `[image-drawn]` lines across
        // five captures.
        //
        // ⭐ AND SIZE IS THE WRONG QUESTION HERE ANYWAY. The three earlier
        // stages report DECODE cost, where big means expensive. This one reports
        // FIRST USE, where the interesting fact is how long the thing waited to
        // be seen — a small sheet that took a second to reach the screen is a
        // visible pop, and a large one drawn immediately is not a problem.
        if waited < NOTABLE_DRAW_WAIT {
            continue;
        }
        let at = now.duration_since(started_at.0).as_secs_f64();
        // ⭐⭐ `POP` IS THE WHOLE POINT OF THIS LINE. A cover exists so a room's
        // art arrives before anyone can see the room; an image whose FIRST DRAW
        // lands while gameplay is live is one the cover did not cover, and it
        // appeared in front of the player. Named rather than left as `live=1`,
        // because the reader of a hitch log should not have to know which way
        // the flag points.
        // ⛔⛔ AND "POP" ONLY MEANS SOMETHING WHERE A COVER EXISTED. `capture_scene`
        // boots straight into `playing` on every road it has, so there every
        // first draw is trivially during gameplay — eighteen "findings" in a
        // hall shot, all of them the harness. `saw_covered_frame` is the fact
        // that separates "the cover did not cover this" from "nothing here has
        // a cover", and without it this line reports the instrument.
        let pop = match stages.live_at_first_draw {
            Some(true) if covered => " POP (drawn during gameplay, after the cover)",
            Some(true) => " live=1 (this composition never covered anything)",
            Some(false) => "",
            None => " live=?",
        };
        eprintln!(
            "[image-drawn] {at:8.3}s {:6.1}MP {} demand→draw {:.0}ms via {}{pop}",
            stages.megapixels,
            stages.path.as_deref().unwrap_or("<runtime-generated>"),
            waited.as_secs_f64() * 1e3,
            stages.source.unwrap_or("?"),
        );
    }
}

/// The census clock, mirrored into the render world so `[image-gpu]` lines sit
/// on the same timeline as `[image]` lines. Native only, like every stage stamp.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Clone, Copy)]
pub struct ImageStageClock(pub Instant);

/// Installs the render-world half of the stage ledger. A no-op when the app
/// has no render world (`NoWindow`, headless): then nothing is ever prepared
/// on a GPU, and `awaiting gpu` on the census line correctly grows — that is
/// the readout saying the pixels were decoded for nobody.
pub struct ImageStagePlugin;

impl Plugin for ImageStagePlugin {
    fn build(&self, app: &mut App) {
        use bevy::render::{Render, RenderApp, RenderSystems};

        // ⛔⛔ THE READINESS HALF IS NOT `not(wasm32)`, AND IT USED TO BE. The
        // whole of this body was native-only because `ImageStageClock` holds an
        // `Instant`. That gated the FACT along with the TELEMETRY: on the web,
        // `RenderWorldPresent` was never inserted and `is_gpu_prepared` was a
        // stub returning `false`, so `is_awaiting_gpu` was always false and the
        // browser lifted its cover the moment pixels reached `Assets<Image>` —
        // skipping exactly the GPU upload the barrier exists to move under the
        // cover. Every branch type-checks, so the wasm CHECK could not see it.
        // The clock stays native; the stamp does not.
        if app.get_sub_app(RenderApp).is_none() {
            return;
        }
        // From here on a reveal may wait for stage 3; see
        // `ImageStageLedger::is_awaiting_gpu`. ⛔ ON THIS APP'S MAIN WORLD, not
        // on the process ledger: a sibling App in the same process has its own
        // answer. Inserted BEFORE the sub-app is borrowed.
        app.insert_resource(image_stages::RenderWorldPresent(true));

        // ⭐ ONE SET, BOTH WORLDS, THIS App. The main world reads it as the
        // reveal-readiness term and the render sub-app writes it; they share the
        // `Arc` because they are the same App. A second rendering App builds its
        // own, so preparation in one cannot settle the other even when their
        // local asset ids collide.
        //
        // ⛔ ON EVERY TARGET, like the stamp that writes it. `stamp_gpu_prepared_images`
        // takes this as an `Option<Res<_>>`, so leaving it uninserted on the web
        // would compile, run, and silently skip the authoritative write — the
        // reveal would be back to reading a process-global answer.
        let prepared_here = image_stages::AppGpuPreparedImages::default();
        app.insert_resource(prepared_here.clone());

        // One clock for both halves: whichever side initialises the census
        // first fixes the zero, and the other reads it.
        #[cfg(not(target_arch = "wasm32"))]
        let clock = {
            app.init_resource::<ImageCensus>();
            ImageStageClock(app.world().resource::<ImageCensus>().started_at)
        };

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.insert_resource(prepared_here);
        #[cfg(not(target_arch = "wasm32"))]
        render_app.insert_resource(clock);
        // THE READINESS STAMP, on every target — the reveal barrier reads it.
        render_app.add_systems(
            Render,
            stamp_gpu_prepared_images.after(RenderSystems::PrepareAssets),
        );
        // ⛔ AFTER EXTRACTION, which is where `ExtractedSprites` is filled — a
        // SIBLING of the hook above rather than new machinery: same sub-app,
        // same ledger. Native-only, and unlike the stamp above that is correct:
        // first-draw is pure TELEMETRY measured in `Instant`s, and no readiness
        // decision reads it. Gating the GPU stamp the same way is what put the
        // hole in the web reveal.
        #[cfg(not(target_arch = "wasm32"))]
        render_app.add_systems(
            Render,
            stamp_first_drawn_images.after(RenderSystems::ExtractCommands),
        );
    }
}
