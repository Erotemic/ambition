//! Text census of texture decoding, in the style of the `[startup]` and
//! `[schedule-census]` loggers in `ambition_dev_tools::profiling`.
//!
//! Native profile symbols name the PNG decoder but not the asset. This census
//! prints which sheets were decoded, how many megapixels, and when. It writes
//! to stderr, so `scripts/profile_desktop.sh` puts each line in the timeline
//! chunk where the decode happened.

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
    // Accumulated on every platform. Only native builds report it, because a
    // browser build has no terminal for the rolling window.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    total_bytes: u64,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    window_images: u64,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    window_megapixels: f64,
    /// How many images' bytes were derived from the texture descriptor rather
    /// than measured, because their CPU copy had been dropped. Reported so the
    /// total says how much of itself was measured.
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

    /// How many of the counted images had their bytes derived rather than
    /// measured. `0` means every byte in `total_bytes` was seen directly.
    pub fn derived_byte_images(&self) -> u64 {
        self.derived_byte_images
    }

    /// Bytes of decoded image data seen so far. Cumulative, never decremented:
    /// this counts decode work, so a rise with a flat `total_images` means the
    /// same asset was decoded again.
    ///
    /// This counts `AssetEvent::Added`, which fires before the render world
    /// extracts the image. "Decoded" here is upstream of "ready to draw"; the
    /// extract (`extract_render_asset<GpuImage>`) is a separate cost.
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
}

/// How long a demand-to-first-draw wait must be before `[image-drawn]` reports it.
///
/// A tenth of a second (six frames at 60Hz) is where a late sprite becomes a
/// visible pop. Shorter waits are the ordinary cost of streaming.
///
/// Native-only, like its only consumer `stamp_first_drawn_images`. First-draw
/// is telemetry measured in `Instant`s, so it may be gated; the GPU readiness
/// stamp may not.
#[cfg(not(target_arch = "wasm32"))]
const NOTABLE_DRAW_WAIT: std::time::Duration = std::time::Duration::from_millis(100);

/// How many unrouted images the census names before it says `+N more`.
/// The bucket is meant to be small; a long list is a different problem.
#[cfg(not(target_arch = "wasm32"))]
const UNROUTED_NAMED: usize = 8;

/// Log every notable texture as it lands, plus a periodic rollup.
///
/// `AssetEvent::Added` fires when the asset reaches `Assets<Image>`, after the
/// IO pool decoded it. So these timestamps mark decode completion, which lines
/// up with a frame spike and a sprite re-bind.
#[cfg(not(target_arch = "wasm32"))]
pub fn report_image_census(
    mut events: MessageReader<AssetEvent<Image>>,
    images: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
    mut census: ResMut<ImageCensus>,
    // Optional and per-App. Absent means this App has no render world (for
    // example a headless probe beside a rendering sibling).
    render_world: Option<Res<image_stages::RenderWorldPresent>>,
    // Optional, because a composition may have no game mode (a capture tool,
    // a headless probe). Absent means "cannot tell", so do not report a hitch.
    mode: Option<
        Res<bevy::state::state::State<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
    >,
    // Flush on exit as well as on the window boundary. Otherwise a capture that
    // ends inside one window prints no `[image-census]` line.
    mut exits: MessageReader<bevy::app::AppExit>,
    // "Live" means a player is in a world: the mode allows gameplay and a
    // session root exists. The shell host boots in `Playing` with only the
    // launcher, so mode alone would mark launcher art as a gameplay hitch.
    sessions: Query<(), With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>>,
    // This App's readiness authority. The arrival recorded below is the
    // candidate half; the render world supplies the prepared half. Both must be
    // App-local; see `AppReadiness`.
    prepared_here: Option<Res<image_stages::AppGpuPreparedImages>>,
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
            AssetEvent::Added { id } => {
                // The readiness candidate, through the same definition the web
                // path uses. Record it before the notability filter: the reveal
                // waits on every image, not only big ones.
                note_image_arrival(prepared_here.as_deref(), *id);
                *id
            }
            AssetEvent::Modified { id } => {
                // Readiness only, not a census row. The bytes changed in place,
                // so the GPU copy must be re-proven. It is not a new decode, and
                // counting it would corrupt the re-decode census.
                note_image_arrival(prepared_here.as_deref(), *id);
                continue;
            }
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                // Readiness retires on `Unused` only: in Bevy's render-asset
                // pipeline `Unused` removes the render representation, and
                // `Removed` only drops the main-world handle. The telemetry row
                // is dropped for both.
                if matches!(event, AssetEvent::Unused { .. }) {
                    note_image_retired(prepared_here.as_deref(), *id);
                }
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
        // Measure the CPU copy when it exists; derive it when it does not.
        // `image.data` is `None` when the main-world copy was dropped
        // (`RenderAssetUsages::RENDER_WORLD`). Reporting 0 would make decoded
        // bytes fall without a real saving, so use the texture descriptor:
        // width x height x bytes-per-block.
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
            // The asset path is the one fact a perf symbol cannot give.
            let path = asset_server
                .get_path(id)
                .map(|path| path.to_string())
                .unwrap_or_else(|| "<runtime-generated>".to_string());
            // Say which stage was late: a late decode and a stalled upload look
            // the same in a frame-time trace.
            let mut demand = stages.demand_phrase();
            if stages.insertions_of_path > 1 {
                // The same file decoded again (dropped and demanded back, or
                // loaded under a second id). The pixels were paid for twice.
                demand.push_str(&format!(" RE-DECODE #{}", stages.insertions_of_path));
            }
            // Emit `live=` on both branches. A log with no marker predates it
            // and must read as "unknown", not "none".
            //
            // `live=1` is a contract violation: a big image decoded during
            // gameplay costs a frame the player can feel. It is a warning, not
            // an error, because some late assets are legitimate (an
            // unpredictable summon, a dev spawn).
            let live = u8::from(during_gameplay);
            // The same frame stamp as `[world-event]`, so ordering against
            // `room-loaded` compares integers. The census runs in `Last`, so its
            // wall time can read after a `room-loaded` it actually preceded.
            let frame = ambition_platformer2d_shared_tangle::world_log::frame();
            // A runtime-generated image (an atlas or render target, no path)
            // is not a content decode, and cannot be demanded at match
            // preparation. Still report it, in its own sentence.
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
    // Stay silent through quiet windows, so the windows that decoded something
    // stay visible. The exit flush prints the last partial window.
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
        // Resident images by the road that demanded them, so a transition's
        // growth reads per owner.
        // Rename the unrouted rows. `resident_by_road` keys a row with no
        // `source` as `"?"`, which reads like an unknown road. Those images
        // passed no stamped demand; they were inserted directly.
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
        // Name the unrouted files too, because the next question is always
        // "which?". Capped, because a census line is not a manifest.
        //
        // File-backed only. Procedural inserts have no load to stamp and never
        // leave this bucket; listing them would push real findings past the cap.
        let unrouted: Vec<String> = ledger
            .unrouted_resident()
            .into_iter()
            .take(UNROUTED_NAMED)
            .map(|(mp, path)| format!("{mp:.1}MP {path}"))
            .collect();
        let unrouted_total = ledger.unrouted_resident().len();
        // Only where a draw is possible. Without a render world nothing is
        // extracted, so every resident image would read as "never drawn".
        let render_world_present =
            image_stages::RenderWorldPresent::from_option(render_world.as_deref());
        let never_drawn: Option<(usize, f64, Vec<String>)> =
            render_world_present.is_present().then(|| {
                let by_road = ledger.never_drawn_by_road();
                let (count, megapixels) = by_road
                    .values()
                    .fold((0usize, 0f64), |(n, mp), (c, road_mp)| {
                        (n + c, mp + road_mp)
                    });
                // By owner, because an eviction decision starts from an owner.
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
        // The GPU half on the same line as the decode half, so a backlog is
        // visible. `awaiting` is inserted-but-not-yet-prepared: nonzero at the
        // end of a quiet window means the upload pacer (or a missing render
        // world) is holding pixels the main world already paid for.
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
            // `-` where a draw is not observable, which differs from "nothing
            // drawn yet".
            never_drawn.map_or("-".to_string(), |(n, mp, rows)| {
                format!("{n} ({mp:.1}MP: {})", rows.join(", "))
            }),
            by_road.join(", "),
        );
        // One line, only when there is something to say. An unrouted image is
        // eager loading nobody asked for or a demand road that stamps nothing.
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

/// Wasm has no `Instant`, so the report is a no-op there (use browser
/// devtools). The arrival record still runs: without it the render world has
/// no candidates, nothing is stamped prepared, and the room cover never lifts.
/// Both targets call the same [`note_image_arrivals`].
#[cfg(target_arch = "wasm32")]
pub fn report_image_census(
    mut events: MessageReader<AssetEvent<Image>>,
    _images: Res<Assets<Image>>,
    _asset_server: Res<AssetServer>,
    _census: ResMut<ImageCensus>,
    prepared_here: Option<Res<image_stages::AppGpuPreparedImages>>,
) {
    note_image_arrivals(&mut events, prepared_here.as_deref());
}

/// Record every arriving image as a GPU-readiness candidate for this App.
///
/// Shared by both targets, so the web path cannot do less than the native
/// one. It is testable without a render world.
pub fn note_image_arrivals(
    events: &mut MessageReader<AssetEvent<Image>>,
    prepared_here: Option<&image_stages::AppGpuPreparedImages>,
) {
    for event in events.read() {
        match event {
            // Both name the current contents of that id needing to reach the
            // GPU. See `mark_awaiting`.
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                note_image_arrival(prepared_here, *id);
            }
            AssetEvent::Unused { id } => note_image_retired(prepared_here, *id),
            _ => {}
        }
    }
}

/// What "an image arrived" means for readiness, defined once.
///
/// Both `report_image_census` bodies call this. The native one cannot call
/// [`note_image_arrivals`], because it must also see `Removed`/`Unused` in the
/// same drain and a `MessageReader` yields each event once. So the shared item
/// is the record, not the loop.
pub fn note_image_arrival(
    prepared_here: Option<&image_stages::AppGpuPreparedImages>,
    id: AssetId<Image>,
) {
    if let Some(prepared_here) = prepared_here {
        prepared_here.mark_awaiting(id.untyped());
    }
}

/// What "an image is gone" means for readiness, defined once beside
/// [`note_image_arrival`].
pub fn note_image_retired(
    prepared_here: Option<&image_stages::AppGpuPreparedImages>,
    id: AssetId<Image>,
) {
    if let Some(prepared_here) = prepared_here {
        prepared_here.mark_retired(id.untyped());
    }
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

    /// The plugin must install the per-App set. `stamp_gpu_prepared_images`
    /// takes it as `Option<Res<_>>`, so without it the system runs and silently
    /// skips the authoritative write. Tests that construct the set cannot see
    /// its absence, so this one goes through the composition.
    ///
    /// Not covered: that the render sub-app got this set rather than a fresh
    /// one (no sub-app exists headless).
    /// `a_clone_is_the_same_set_because_one_app_shares_it_across_worlds` pins
    /// the `Arc` semantics.
    #[test]
    fn the_plugin_installs_the_per_app_prepared_set_even_without_a_render_world() {
        let mut app = App::new();
        app.add_plugins(ImageStagePlugin);
        assert!(
            app.world()
                .get_resource::<image_stages::AppGpuPreparedImages>()
                .is_some(),
            "ImageStagePlugin must install AppGpuPreparedImages on the App's main world; \
             without it `stamp_gpu_prepared_images` sees `None` and silently skips the \
             authoritative write, falling back to the process-global answer",
        );
    }

    /// The arrival record is the readiness pipeline's input. If it is
    /// discarded, no image becomes a candidate and the cover never lifts. A
    /// `--target wasm32` check cannot see that, so this pins the behaviour.
    ///
    /// It tests `note_image_arrival`, which both `report_image_census` bodies
    /// call. It does not prove each body still calls it.
    #[test]
    fn an_arriving_image_becomes_a_candidate_for_this_app() {
        let prepared_here = image_stages::AppGpuPreparedImages::default();
        let arrived = AssetId::<Image>::invalid();

        assert_eq!(
            prepared_here.awaiting_count(),
            0,
            "non-vacuity: nothing is owed before the arrival",
        );

        note_image_arrival(Some(&prepared_here), arrived);

        assert_eq!(
            prepared_here.awaiting_ids(),
            vec![arrived.untyped()],
            "an arrived image must be a candidate this App's render world looks for; \
             discarding the event is what left the web cover up forever",
        );
        assert!(
            prepared_here
                .is_awaiting_gpu(arrived.untyped(), image_stages::RenderWorldPresent(true)),
            "and it is still OWED until the render world stamps it",
        );
    }

    /// An App with no readiness authority must not panic: a headless probe has
    /// no render world and gets no resource.
    #[test]
    fn an_arrival_without_an_authority_is_a_no_op_rather_than_a_panic() {
        note_image_arrival(None, AssetId::<Image>::invalid());
    }
}

/// Stage 3 of the image ledger: the GPU copy exists.
///
/// Runs in the render world after Bevy's `prepare_assets::<GpuImage>`, and
/// checks only the ids the main world inserted that are not yet prepared. The
/// pacer (`RenderAssetBytesPerFrame`) defers uploads across frames; this shows
/// the deferral as `insert→gpu` growing while `awaiting gpu` stays nonzero.
///
/// Runs on every target. The stamp is the readiness fact the web reveal
/// barrier depends on; only the `[image-gpu]` report needs a clock.
pub fn stamp_gpu_prepared_images(
    gpu_images: Res<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>,
    // This App's set, not the process ledger. Asset ids are App-local and can
    // collide, so a global set would let one App's upload lift another App's
    // cover. The `Arc` inside is shared with this App's main world.
    prepared_here: Option<Res<image_stages::AppGpuPreparedImages>>,
    // The clock is native-only; the readiness fact above is not.
    #[cfg(not(target_arch = "wasm32"))] started_at: Res<ImageStageClock>,
) {
    // Candidates come from this App, not the process ledger. The ledger's
    // `awaiting_gpu` is one process-wide list, and `gpu_prepared()` consumes
    // the entry, so two Apps sharing an id would steal each other's candidate.
    let Some(prepared_here) = prepared_here.as_deref() else {
        return;
    };
    let awaiting = prepared_here.awaiting_ids();
    if awaiting.is_empty() {
        return;
    }
    let prepared: Vec<_> = awaiting
        .into_iter()
        .filter(|id| {
            id.try_typed::<Image>()
                .is_ok_and(|id| gpu_images.get(id).is_some())
        })
        .collect();
    if prepared.is_empty() {
        return;
    }
    let mut ledger = image_stages::ledger();
    // The authoritative write. It happens before, and independently of, the
    // ledger mirror below: `gpu_prepared` returns `None` when there is nothing
    // to report, and readiness must not inherit that early exit. Unconditional:
    // the web reveal barrier reads it.
    for id in &prepared {
        prepared_here.mark_prepared(*id);
    }

    // One `#[cfg]` boundary for everything that needs a clock. Many small
    // gates make it easy to put a `#[cfg]` on the wrong item.
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

/// The `[image-gpu]` line. Native-only because every field is a duration and
/// the web has no `Instant`.
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

/// Stamp the fourth stage: this image was extracted, so this frame would draw it.
///
/// The earlier stages say the asset arrived; this one says it is used.
/// `ExtractedSprites` is filled after visibility culling, so an id there means
/// "this frame would draw it".
///
/// `SpriteBatch` is one step later and stronger (it survived batching), but runs
/// after `RenderSystems::Queue`. Use extraction until a measurement shows it
/// over-reports.
///
/// Writes at most once per image: `ImageStageLedger::first_drawn` returns
/// `None` after the first stamp. So the per-frame cost is a lock and a walk
/// over the extracted list, and the ledger does not grow each frame.
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
        // Gate on the wait, not the size. `NOTABLE_MEGAPIXELS` suits full-tier
        // art; under the room sprite-tier cap a whole character sheet is about
        // 0.3 MP and would never report. Also, this stage reports first use, and
        // the fact that matters is how long the image waited to be seen.
        if waited < NOTABLE_DRAW_WAIT {
            continue;
        }
        let at = now.duration_since(started_at.0).as_secs_f64();
        // `POP` names an image whose first draw came during live gameplay: the
        // cover did not cover it.
        // "POP" only applies where a cover existed. `capture_scene` boots
        // straight into `playing`, so every first draw there is during
        // gameplay. `saw_covered_frame` separates the two cases.
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

        // The readiness half is not native-only. Only `ImageStageClock` (it
        // holds an `Instant`) is. If the readiness stamp were gated too, the
        // browser would lift its cover before the GPU upload.

        // Insert above the early return so a test can check it:
        // `stamp_gpu_prepared_images` takes it as `Option<Res<_>>` and silently
        // skips the write when it is absent.
        //
        // Inert on an App with no render world:
        // `is_awaiting_gpu(id, RenderWorldPresent(false))` is false whatever
        // the set holds (pinned by `a_headless_app_is_never_awaiting`).
        let prepared_here = image_stages::AppGpuPreparedImages::default();
        app.insert_resource(prepared_here.clone());

        if app.get_sub_app(RenderApp).is_none() {
            return;
        }
        // From here on a reveal may wait for stage 3; see
        // `ImageStageLedger::is_awaiting_gpu`. Insert on this App's main world,
        // not the process ledger, and before the sub-app is borrowed.
        app.insert_resource(image_stages::RenderWorldPresent(true));

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
        // The readiness stamp, on every target: the reveal barrier reads it.
        render_app.add_systems(
            Render,
            stamp_gpu_prepared_images.after(RenderSystems::PrepareAssets),
        );
        // After extraction, where `ExtractedSprites` is filled. Native-only is
        // correct here: first-draw is telemetry, and no readiness decision
        // reads it.
        #[cfg(not(target_arch = "wasm32"))]
        render_app.add_systems(
            Render,
            stamp_first_drawn_images.after(RenderSystems::ExtractCommands),
        );
    }
}
