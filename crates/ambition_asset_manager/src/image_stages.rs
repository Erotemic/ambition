//! Per-image stage ledger: when an image was demanded, when its decoded pixels
//! reached `Assets<Image>`, and when the GPU copy was prepared.
//!
//! A frame spike during a room reveal can come from the IO pool (decode), the
//! main world (insert), or the render world (upload). This ledger keeps the
//! three instants per asset, so a late image can say which stage was late.
//!
//! The ledger is process-global: the demand is recorded by a free function with
//! no world (`load_sheet_image`), the insertion by a main-world system, and the
//! GPU preparation by a render-world system. It is a diagnostic. It is never
//! rollback state.
//!
//! Do not use this ledger to make decisions. Asset ids are local to an App and
//! can collide across Apps, so a process-global ledger cannot answer an
//! App-local question. The reveal authority is [`AppGpuPreparedImages`], one
//! per App. The ledger mirrors every stamp for the `[image-gpu]` lines, the
//! insert→gpu timings and the census.
//!
//! Coverage is the images demanded through a road that calls
//! [`note_demand`]: `load_sheet_image` and the manifest catalog's
//! `load_optional`. An image that arrives by another road still gets its
//! insertion and GPU stamps, but reports `demand=unknown`.
//!
//! The road vocabulary is closed at eight. Add a road only with a reason:
//!
//! ```text
//! character-sheet  parallax  fx-sheet  boss-sheet
//! asset-manifest   portrait  projectile-art  held-item
//! ```
//!
//! They name content art decoded at runtime, because a room reveal waits on
//! that art. Menu icons, shell presentation images and prop pngs are not
//! stamped: they are small, load once, and a reveal does not wait for them.
//! `demand=unknown` on those is expected.
//!
//! `demand=unknown` has a second cause: a dropped image loses its demand row
//! (see [`ImageStageLedger::removed`]), so a re-decode can arrive unattributed.
//!
//! Keyed by [`UntypedAssetId`] so a generic `load_optional::<T>` can record a
//! demand without knowing it is an image. The census asks only about image ids.

use bevy::asset::UntypedAssetId;
use bevy::prelude::Resource;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

/// One image's stage instants. `None` means "not reached yet" or, for the
/// demand, "not demanded through the funnel".
#[derive(Clone, Debug)]
pub struct ImageStages {
    /// Where the demand came from (`"character-sheet"`, `"parallax"`, …), when
    /// it came through the funnel.
    pub source: Option<&'static str>,
    pub path: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub demanded_at: Option<Instant>,
    #[cfg(not(target_arch = "wasm32"))]
    pub inserted_at: Option<Instant>,
    #[cfg(not(target_arch = "wasm32"))]
    pub gpu_prepared_at: Option<Instant>,
    /// The readiness fact, on every target: this App's render world has
    /// prepared the image.
    ///
    /// This is separate from `gpu_prepared_at` because `Instant` is
    /// native-only. If readiness read the timestamp, wasm would never wait for
    /// the GPU upload. A readiness decision reads this field; a report reads
    /// the timestamp.
    pub gpu_prepared: bool,
    /// The first frame this image would be drawn: the fourth stage.
    ///
    /// The other three stages are about the asset arriving. Only this one says
    /// the asset was used.
    ///
    /// `None` means either "no render world" (a `NoWindow` or headless
    /// composition, where this is never set) or "not drawn yet". The asking
    /// App's [`RenderWorldPresent`] separates the two.
    #[cfg(not(target_arch = "wasm32"))]
    pub first_drawn_at: Option<Instant>,
    pub megapixels: f64,
    /// Whether gameplay was live when the pixels were inserted.
    pub live_at_insert: Option<bool>,
    /// Whether gameplay was live the first time this image was drawn.
    ///
    /// This is the pop. A cover exists so a room's art arrives before anybody
    /// can see the room. An image first drawn while gameplay is live appeared
    /// in front of the player. `live_at_insert` answers a different question
    /// (did the decode cost a live frame); the two can disagree either way.
    ///
    /// `None` where nothing could tell: no game mode, or no render world.
    pub live_at_first_draw: Option<bool>,
    /// How many times this path has been inserted since the process started
    /// (1 = the first decode). A second insertion is a re-decode: the asset was
    /// dropped and demanded again, or loaded under two ids. See asset open work
    /// 5 in `asset-preparation-and-residency.md`.
    pub insertions_of_path: u32,
}

impl ImageStages {
    fn blank() -> Self {
        Self {
            source: None,
            path: None,
            #[cfg(not(target_arch = "wasm32"))]
            demanded_at: None,
            #[cfg(not(target_arch = "wasm32"))]
            inserted_at: None,
            #[cfg(not(target_arch = "wasm32"))]
            gpu_prepared_at: None,
            gpu_prepared: false,
            #[cfg(not(target_arch = "wasm32"))]
            first_drawn_at: None,
            megapixels: 0.0,
            live_at_insert: None,
            live_at_first_draw: None,
            insertions_of_path: 0,
        }
    }

    /// Demand → insertion, when both are known.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn demand_to_insert(&self) -> Option<Duration> {
        Some(self.inserted_at?.duration_since(self.demanded_at?))
    }

    /// Insertion → GPU preparation, when both are known.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn insert_to_gpu(&self) -> Option<Duration> {
        Some(self.gpu_prepared_at?.duration_since(self.inserted_at?))
    }

    /// `demand→insert 123ms via character-sheet`, `first demanded via <road>`
    /// for a re-decode, or `demand=unknown`.
    ///
    /// A re-decode has a known demander but no valid wait: `removed` deleted
    /// its row, and the path's first demand instant belongs to the earlier
    /// decode. So it names the demander and gives no duration.
    /// `demand=unknown` means the image arrived by a road that stamps nothing.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn demand_phrase(&self) -> String {
        match (self.demand_to_insert(), self.source) {
            (Some(d), Some(source)) => {
                format!("demand→insert {:.0}ms via {source}", d.as_secs_f64() * 1e3)
            }
            (None, Some(source)) => format!("first demanded via {source}"),
            _ => "demand=unknown (not through load_sheet_image)".to_string(),
        }
    }
}
/// Whether this App has a render world. This is a per-App fact, so it is an
/// App resource and not a field on the process-global ledger. Otherwise a
/// headless App beside a rendering one (as in parallel `app_it` tests) would
/// wait forever for a GPU that never looks at its images.
///
/// Absent means `false`: an App that never installed the census never had a
/// render world stamping stage 3.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderWorldPresent(pub bool);

impl RenderWorldPresent {
    /// Read the fact from an App that may not have the resource at all.
    pub fn from_option(present: Option<&Self>) -> Self {
        present.copied().unwrap_or_default()
    }

    pub fn is_present(self) -> bool {
        self.0
    }
}

/// The images THIS App's render world has prepared.
///
/// Asset ids are local to an App and can collide across Apps, so the
/// process-global [`ImageStageLedger`] cannot hold this. One instance is made
/// per App and inserted into both that App's main world and its render
/// sub-app. The stamper and the reveal share one `Arc`; a sibling App holds a
/// different one.
///
/// The global ledger still mirrors these stamps for reports. It must not
/// decide whether a cover lifts.
#[derive(Default, Debug)]
struct AppReadiness {
    /// Images this App's main world has seen arrive and not yet seen prepared.
    /// This candidate set must be App-local too. A shared set never fills on
    /// wasm, and with two rendering Apps the first render world would consume
    /// the other App's candidate.
    awaiting: HashSet<UntypedAssetId>,
    /// Images whose current contents this App's render world has prepared.
    ///
    /// "Current", not "ever": Bevy re-prepares an asset that was modified in
    /// place, and the reveal barrier treats this set as proof. The proof must
    /// last only as long as the contents it proves.
    prepared: HashSet<UntypedAssetId>,
}

/// This App's GPU-readiness authority: the candidate set AND the answer, shared
/// between its own main and render worlds by the `Arc`.
#[derive(Resource, Clone, Default, Debug)]
pub struct AppGpuPreparedImages(Arc<Mutex<AppReadiness>>);

impl AppGpuPreparedImages {
    /// This App's main world saw `id`'s current contents arrive in
    /// `Assets<Image>` (an `Added` or a `Modified`). Recorded on every target:
    /// this is the readiness input, not telemetry.
    ///
    /// This retires any earlier proof. A `Modified` means new bytes that Bevy
    /// will prepare again, and an `Added` for a recycled id names different
    /// contents. The old stamp no longer applies.
    pub fn mark_awaiting(&self, id: UntypedAssetId) {
        if let Ok(mut state) = self.0.lock() {
            state.prepared.remove(&id);
            state.awaiting.insert(id);
        }
    }

    /// `id` has no GPU representation and no claim to one: Bevy's `Unused`,
    /// which removes it from `RenderAssets`.
    ///
    /// Use `Unused` only. A plain `Removed` is the main-world handle going
    /// away, and Bevy's render-asset pipeline treats it differently.
    ///
    /// Clear both sets: the id can be pending (dropped before the GPU saw it)
    /// or prepared (dropped after).
    pub fn mark_retired(&self, id: UntypedAssetId) {
        if let Ok(mut state) = self.0.lock() {
            state.awaiting.remove(&id);
            state.prepared.remove(&id);
        }
    }

    /// The candidates this App's render world should look for. A snapshot, so
    /// the lock is not held across the GPU query.
    pub fn awaiting_ids(&self) -> Vec<UntypedAssetId> {
        self.0
            .lock()
            .map(|state| state.awaiting.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn awaiting_count(&self) -> usize {
        self.0.lock().map(|state| state.awaiting.len()).unwrap_or(0)
    }

    /// Record that this App's render world has a GPU copy of `id`'s current
    /// contents.
    pub fn mark_prepared(&self, id: UntypedAssetId) {
        if let Ok(mut state) = self.0.lock() {
            state.awaiting.remove(&id);
            state.prepared.insert(id);
        }
    }

    /// Has this App prepared `id`?
    pub fn is_prepared(&self, id: UntypedAssetId) -> bool {
        self.0
            .lock()
            .is_ok_and(|state| state.prepared.contains(&id))
    }

    pub fn prepared_count(&self) -> usize {
        self.0.lock().map(|state| state.prepared.len()).unwrap_or(0)
    }

    /// The reveal-readiness term: this App draws, and has not yet prepared `id`.
    ///
    /// This needs positive proof: an id with no stamp yet is owed, not ready.
    /// A headless App answers `false`, because it never prepares anything.
    pub fn is_awaiting_gpu(&self, id: UntypedAssetId, render_world: RenderWorldPresent) -> bool {
        render_world.is_present() && !self.is_prepared(id)
    }
}

/// The ledger behind the process-global. Pure, so the arithmetic is testable
/// without an asset server or a render world.
#[derive(Default)]
pub struct ImageStageLedger {
    rows: BTreeMap<UntypedAssetId, ImageStages>,
    /// Images inserted but not yet seen prepared on the GPU — what the render
    /// world polls, kept small on purpose.
    awaiting_gpu: Vec<UntypedAssetId>,
    /// The main world's last word on whether gameplay is live, for the render
    /// world's report (it has no `GameMode` of its own).
    gameplay_live: Option<bool>,
    saw_covered_frame: bool,
    /// Insertions per path, across ids and across removals: the re-decode
    /// census. Survives `removed`.
    insertions_by_path: BTreeMap<String, u32>,
    /// The first demand recorded for a path. Like the insertion count, it
    /// survives `removed`.
    ///
    /// `removed` deletes the per-id row, and `demand()` runs only at a load
    /// call site (a second `load` of a resident path is a handle lookup). So
    /// without this a re-decode reads `demand=unknown`, like an unrouted road.
    /// The wasted decode is the one whose demander must be named.
    ///
    /// Native-only: it holds an `Instant`. `image_stages` compiles on wasm
    /// when the `bevy` feature is on, so an ungated field breaks the wasm
    /// build.
    #[cfg(not(target_arch = "wasm32"))]
    demand_by_path: BTreeMap<String, (&'static str, Instant)>,
    /// Total insertions that were a path's second or later.
    pub re_decodes: u64,
    /// Images removed after insertion and before any GPU preparation: decoded
    /// and never drawn.
    pub dropped_before_gpu: u64,
    pub dropped_before_gpu_megapixels: f64,
    /// Totals since the process started, for the census summary line.
    pub gpu_prepared_total: u64,
    pub gpu_prepared_megapixels: f64,
    /// Rolling window (drained by the census line).
    pub window_gpu_prepared: u64,
    pub window_gpu_megapixels: f64,
    pub window_insert_to_gpu: Vec<Duration>,
}

impl ImageStageLedger {
    fn row(&mut self, id: UntypedAssetId) -> &mut ImageStages {
        self.rows.entry(id).or_insert_with(ImageStages::blank)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn demand(&mut self, id: UntypedAssetId, source: &'static str, path: String, at: Instant) {
        // First demand wins: a second `load` of the same path is a handle
        // lookup, not a second decode, and the wait that matters is the first.
        if self.row(id).demanded_at.is_some() {
            return;
        }
        // Beside the insertion count, and surviving `removed` the same way, so a
        // re-decode of this path can still say who first asked for it.
        self.demand_by_path
            .entry(path.clone())
            .or_insert((source, at));
        let row = self.row(id);
        row.demanded_at = Some(at);
        row.source = Some(source);
        row.path = Some(path);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn inserted(
        &mut self,
        id: UntypedAssetId,
        megapixels: f64,
        live: Option<bool>,
        path: Option<String>,
        at: Instant,
    ) -> ImageStages {
        let row = self.row(id);
        row.inserted_at = Some(at);
        row.megapixels = megapixels;
        row.live_at_insert = live;
        // The census knows the path even when no demand was recorded; keep it
        // so the GPU line can still name the file.
        if row.path.is_none() {
            row.path = path;
        }
        let path = row.path.clone();
        let snapshot = row.clone();
        let snapshot = if let Some(path) = path {
            let count = self.insertions_by_path.entry(path.clone()).or_default();
            *count += 1;
            if *count > 1 {
                self.re_decodes += 1;
            }
            let count = *count;
            // A re-decode inherits the path's first demand source. `removed`
            // deleted the per-id row, so without this the re-decode prints
            // `demand=unknown`, like an unrouted load.
            //
            // Adopt only the source. `demanded_at` would be the first demand's
            // instant, so the row keeps none and the readout gives no duration.
            #[cfg(not(target_arch = "wasm32"))]
            let inherited = self.demand_by_path.get(&path).map(|(source, _)| *source);
            // WASM never stamps a demand (`note_demand` is a no-op there), so
            // there is no road to inherit and nothing to look up.
            #[cfg(target_arch = "wasm32")]
            let inherited: Option<&'static str> = None;
            let row = self.row(id);
            row.insertions_of_path = count;
            if row.source.is_none() {
                row.source = inherited;
            }
            row.clone()
        } else {
            snapshot
        };
        if !self.awaiting_gpu.contains(&id) {
            self.awaiting_gpu.push(id);
        }
        snapshot
    }

    pub fn set_gameplay_live(&mut self, live: Option<bool>) {
        // Remember that a cover existed at all. `capture_scene` is in
        // `playing` from boot, so on that road every first draw is "during
        // gameplay". This flag separates "the cover did not cover this" from
        // "this composition has no cover".
        if live == Some(false) {
            self.saw_covered_frame = true;
        }
        self.gameplay_live = live;
    }

    /// Has this process ever observed a frame where gameplay was not live?
    ///
    /// A `false` here means no cover, no countdown and no transition has run,
    /// so [`ImageStages::live_at_first_draw`] is `true` for everything and says
    /// nothing. See [`Self::set_gameplay_live`].
    pub fn saw_covered_frame(&self) -> bool {
        self.saw_covered_frame
    }

    #[allow(dead_code)]
    pub fn gameplay_live(&self) -> Option<bool> {
        self.gameplay_live
    }

    pub fn awaiting_gpu(&self) -> &[UntypedAssetId] {
        &self.awaiting_gpu
    }

    /// Readiness term: a render world exists and has not yet been seen to
    /// prepare `id`. `false` when no render world stamps stage 3.
    ///
    /// The caller supplies the render-world fact from its own App's
    /// [`RenderWorldPresent`]. The ledger is process-global and cannot know
    /// which App is asking.
    ///
    /// This needs positive proof. The insertion stamp comes from `Last`, and
    /// room readiness polls in `Update`, so on the frame an image lands the
    /// poll runs before the row exists. An id with no row is therefore owed.
    /// The cost is one frame of cover per image on an unpaced upload.
    ///
    /// A reveal that waits on this moves the upload of its cast under the
    /// cover, where a byte-per-frame budget can pace it.
    pub fn is_awaiting_gpu(&self, id: UntypedAssetId, render_world: RenderWorldPresent) -> bool {
        render_world.is_present() && !self.is_gpu_prepared(id)
    }

    /// The render world has stamped `id` prepared (stage 3). One definition
    /// for every target.
    pub fn is_gpu_prepared(&self, id: UntypedAssetId) -> bool {
        self.rows.get(&id).is_some_and(|row| row.gpu_prepared)
    }

    /// The render world saw `id` prepared. Returns the row for reporting.
    ///
    /// `at` is optional because the web has no `Instant`. `None` still marks
    /// the image prepared; only the duration is lost.
    pub fn gpu_prepared(
        &mut self,
        id: UntypedAssetId,
        #[cfg(not(target_arch = "wasm32"))] at: Option<Instant>,
    ) -> Option<ImageStages> {
        let position = self
            .awaiting_gpu
            .iter()
            .position(|awaiting| *awaiting == id)?;
        self.awaiting_gpu.swap_remove(position);
        let row = self.row(id);
        row.gpu_prepared = true;
        #[cfg(not(target_arch = "wasm32"))]
        {
            row.gpu_prepared_at = at;
        }
        let snapshot = row.clone();
        self.gpu_prepared_total += 1;
        self.gpu_prepared_megapixels += snapshot.megapixels;
        self.window_gpu_prepared += 1;
        self.window_gpu_megapixels += snapshot.megapixels;
        // The DURATION is telemetry and needs two `Instant`s; the counts above
        // are the fact and are kept on every target.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(d) = snapshot.insert_to_gpu() {
            self.window_insert_to_gpu.push(d);
        }
        Some(snapshot)
    }

    /// This image was extracted for drawing: the fourth stage, the first one
    /// about use and not arrival.
    ///
    /// First write wins. Extraction runs every frame for every visible sprite,
    /// so overwriting would add a per-frame write for the whole visible set.
    ///
    /// Returns demand→draw when this call stamped the row and the demand is
    /// known. Returns `None` on every later frame, so the caller prints once.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn first_drawn(&mut self, id: UntypedAssetId, at: Instant) -> Option<Duration> {
        let live = self.gameplay_live;
        let row = self.row(id);
        if row.first_drawn_at.is_some() {
            return None;
        }
        row.first_drawn_at = Some(at);
        row.live_at_first_draw = live;
        let demanded = row.demanded_at?;
        Some(at.duration_since(demanded))
    }

    /// What is resident and never drawn, grouped by the road that demanded it.
    /// This names the owners of undrawn megapixels (the same buckets as
    /// [`Self::resident_by_road`]), so an eviction discussion can start from
    /// an owner.
    ///
    /// Same caveat as [`Self::resident_never_drawn`]: without a render world
    /// this returns every resident image. Check the asking App's
    /// [`RenderWorldPresent`] before reporting it as a finding.
    ///
    /// [`ROAD_PROCEDURAL`] is never a finding here. The stage is stamped from
    /// `ExtractedSprites`, and a render target, shader input or material
    /// texture is never an extracted sprite, so those rows are always "never
    /// drawn". Only file-backed roads answer a residency question here.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn never_drawn_by_road(&self) -> BTreeMap<&'static str, (usize, f64)> {
        let mut by_road: BTreeMap<&'static str, (usize, f64)> = BTreeMap::new();
        for row in self
            .rows
            .values()
            .filter(|row| row.inserted_at.is_some() && row.first_drawn_at.is_none())
        {
            let key = row.source.unwrap_or(if row.path.is_some() {
                ROAD_UNROUTED
            } else {
                ROAD_PROCEDURAL
            });
            let entry = by_road.entry(key).or_default();
            entry.0 += 1;
            entry.1 += row.megapixels;
        }
        by_road
    }

    /// Every resident image the render world has never extracted, largest first.
    ///
    /// Meaningful only with a render world. Without one, this returns every
    /// resident image. The asking App's [`RenderWorldPresent`] separates the
    /// two readings; check it before reporting waste.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn resident_never_drawn(&self) -> Vec<(f64, &str)> {
        let mut rows: Vec<(f64, &str)> = self
            .rows
            .values()
            .filter(|row| row.inserted_at.is_some() && row.first_drawn_at.is_none())
            .filter_map(|row| Some((row.megapixels, row.path.as_deref()?)))
            .collect();
        rows.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(b.1)));
        rows
    }

    /// An image the main world dropped. Returns the row when it was dropped
    /// BEFORE the GPU ever saw it — decoded for nobody (a demoted tier, a room
    /// left mid-load, a demand nothing displayed). Not an error; it is the
    /// wasted half of the decode budget, and it is counted.
    pub fn removed(&mut self, id: UntypedAssetId) -> Option<ImageStages> {
        let was_awaiting = self.awaiting_gpu.contains(&id);
        self.awaiting_gpu.retain(|awaiting| *awaiting != id);
        let row = self.rows.remove(&id)?;
        if was_awaiting {
            self.dropped_before_gpu += 1;
            self.dropped_before_gpu_megapixels += row.megapixels;
            return Some(row);
        }
        None
    }

    /// Drain the rolling GPU window into `(count, megapixels, p50, max)`.
    pub fn take_gpu_window(&mut self) -> (u64, f64, Option<Duration>, Option<Duration>) {
        let count = std::mem::take(&mut self.window_gpu_prepared);
        let megapixels = std::mem::take(&mut self.window_gpu_megapixels);
        let mut waits = std::mem::take(&mut self.window_insert_to_gpu);
        waits.sort_unstable();
        let p50 = waits.get(waits.len() / 2).copied();
        let max = waits.last().copied();
        (count, megapixels, p50, max)
    }

    pub fn get(&self, id: UntypedAssetId) -> Option<&ImageStages> {
        self.rows.get(&id)
    }

    /// Every image inserted and not yet removed, in id order: the per-row
    /// form of [`Self::resident_by_road`].
    ///
    /// Native-only: residency is defined by `inserted_at`, which this module
    /// does not keep on wasm.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn resident_rows(&self) -> impl Iterator<Item = &ImageStages> {
        self.rows.values().filter(|row| row.inserted_at.is_some())
    }

    /// Every row the ledger holds — demanded, inserted or both.
    pub fn rows(&self) -> impl Iterator<Item = &ImageStages> {
        self.rows.values()
    }

    /// The unrouted resident images, largest first: every image that came from
    /// a file and reached `Assets<Image>` without a stamped demand road.
    ///
    /// Use this to find which image is unrouted. File-backed only; see
    /// [`Self::procedural_resident`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn unrouted_resident(&self) -> Vec<(f64, &str)> {
        let mut rows: Vec<(f64, &str)> = self
            .rows
            .values()
            .filter(|row| row.inserted_at.is_some() && row.source.is_none())
            .filter_map(|row| Some((row.megapixels, row.path.as_deref()?)))
            .collect();
        // Megapixels descending, then path, so two censuses diff cleanly.
        rows.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(b.1)));
        rows
    }

    /// Resident images that came from no file: inserted directly into
    /// `Assets<Image>` rather than decoded (render targets, procedural
    /// sprites, shader inputs).
    ///
    /// These are not unrouted. They have no load to stamp, so they can never
    /// get a demand road. Keep them apart so they do not hide real unrouted
    /// files.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn procedural_resident(&self) -> (usize, f64) {
        self.rows
            .values()
            .filter(|row| row.inserted_at.is_some() && row.source.is_none())
            .filter(|row| row.path.is_none())
            .fold((0usize, 0f64), |(n, mp), row| (n + 1, mp + row.megapixels))
    }

    /// What is resident, by the road that demanded it: megapixels of every
    /// image inserted and not yet removed, grouped by source label. This names
    /// the owners of retained assets (asset open work 4). Images no road
    /// stamped group under [`ROAD_UNROUTED`] (from a file) or
    /// [`ROAD_PROCEDURAL`] (no file). Deterministic order, so two censuses diff
    /// cleanly.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn resident_by_road(&self) -> BTreeMap<&'static str, (usize, f64)> {
        let mut by_road: BTreeMap<&'static str, (usize, f64)> = BTreeMap::new();
        for row in self.rows.values().filter(|row| row.inserted_at.is_some()) {
            let key = row.source.unwrap_or(if row.path.is_some() {
                ROAD_UNROUTED
            } else {
                ROAD_PROCEDURAL
            });
            let entry = by_road.entry(key).or_default();
            entry.0 += 1;
            entry.1 += row.megapixels;
        }
        by_road
    }
}

/// [`ImageStageLedger::resident_by_road`] key for a file-backed image that
/// reached `Assets<Image>` without passing a stamped demand road. A finding:
/// something loaded art and no road said so.
pub const ROAD_UNROUTED: &str = "?";

/// [`ImageStageLedger::resident_by_road`] key for an image with no file behind
/// it, inserted directly rather than decoded.
///
/// Not a finding: a procedural image has no load to stamp, so it can never
/// get a demand road.
pub const ROAD_PROCEDURAL: &str = "~procedural";

static LEDGER: Mutex<ImageStageLedger> = Mutex::new(ImageStageLedger {
    rows: BTreeMap::new(),
    awaiting_gpu: Vec::new(),
    gameplay_live: None,
    saw_covered_frame: false,
    insertions_by_path: BTreeMap::new(),
    #[cfg(not(target_arch = "wasm32"))]
    demand_by_path: BTreeMap::new(),
    re_decodes: 0,
    dropped_before_gpu: 0,
    dropped_before_gpu_megapixels: 0.0,
    gpu_prepared_total: 0,
    gpu_prepared_megapixels: 0.0,
    window_gpu_prepared: 0,
    window_gpu_megapixels: 0.0,
    window_insert_to_gpu: Vec::new(),
});

/// The process ledger. A poisoned lock is recovered: this is a diagnostic, and
/// a panic elsewhere must not take the census down with it.
pub fn ledger() -> MutexGuard<'static, ImageStageLedger> {
    LEDGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Record a demand raised now.
#[cfg(not(target_arch = "wasm32"))]
pub fn note_demand(id: UntypedAssetId, source: &'static str, path: String) {
    ledger().demand(id, source, path, Instant::now());
}

#[cfg(target_arch = "wasm32")]
pub fn note_demand(_id: UntypedAssetId, _source: &'static str, _path: String) {}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    fn id(n: u128) -> UntypedAssetId {
        bevy::asset::AssetId::<bevy::asset::LoadedUntypedAsset>::Uuid {
            uuid: bevy::asset::uuid::Uuid::from_u128(n),
        }
        .untyped()
    }

    /// A re-decode knows who first asked for it.
    ///
    /// `removed` deletes the per-id row, and `demand()` runs only at a load
    /// call site. Without the per-path demand, a re-decode reads
    /// `demand=unknown`, like an unstamped road. The path's first demand now
    /// outlives the row, as `insertions_by_path` does.
    #[test]
    fn a_re_decode_inherits_the_road_that_first_demanded_the_path() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();

        ledger.demand(id(1), "character-sheet", "hero.png".into(), t0);
        let first = ledger.inserted(id(1), 7.4, Some(true), None, t0 + Duration::from_millis(80));
        assert_eq!(first.source, Some("character-sheet"));
        assert_eq!(first.insertions_of_path, 1);

        // Dropped before the GPU ever saw it — the wasted decode this ledger
        // exists to count. The row, and with it the demand, is gone.
        assert!(
            ledger.removed(id(1)).is_some(),
            "premise: dropped before GPU"
        );

        // The same file decoded again under a new asset id, with nothing calling
        // `demand()` for it — the shape a quality demote-and-restore produces.
        let second = ledger.inserted(
            id(2),
            7.4,
            Some(true),
            Some("hero.png".into()),
            t0 + Duration::from_millis(400),
        );

        assert_eq!(
            second.source,
            Some("character-sheet"),
            "the re-decode must name the road that first demanded this path, not \
             report `unknown` and read like an unrouted load",
        );
        assert_eq!(second.insertions_of_path, 2, "and it is the path's second");
        assert_eq!(ledger.re_decodes, 1);

        // No wait is quoted: `demanded_at` belongs to the first decode.
        assert_eq!(second.demand_to_insert(), None);
        assert_eq!(second.demand_phrase(), "first demanded via character-sheet");
    }

    #[test]
    fn the_three_stages_measure_from_the_first_demand() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();
        ledger.demand(id(1), "character-sheet", "a.png".into(), t0);
        // A second demand of the same image is a handle lookup, not a new wait.
        ledger.demand(
            id(1),
            "parallax",
            "a.png".into(),
            t0 + Duration::from_millis(500),
        );
        let row = ledger.inserted(
            id(1),
            4.0,
            Some(true),
            None,
            t0 + Duration::from_millis(120),
        );
        assert_eq!(row.source, Some("character-sheet"));
        assert_eq!(row.demand_to_insert(), Some(Duration::from_millis(120)));
        assert_eq!(ledger.awaiting_gpu(), &[id(1)]);

        let row = ledger
            .gpu_prepared(id(1), Some(t0 + Duration::from_millis(150)))
            .expect("an awaited image reports when prepared");
        assert_eq!(row.insert_to_gpu(), Some(Duration::from_millis(30)));
        assert!(ledger.awaiting_gpu().is_empty());
        assert_eq!(ledger.gpu_prepared_total, 1);
        let (count, mp, p50, max) = ledger.take_gpu_window();
        assert_eq!((count, mp), (1, 4.0));
        assert_eq!(p50, Some(Duration::from_millis(30)));
        assert_eq!(max, Some(Duration::from_millis(30)));
        // Drained.
        assert_eq!(ledger.take_gpu_window().0, 0);
    }

    #[test]
    fn a_second_insertion_of_one_path_is_a_re_decode_even_under_a_new_id() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();
        let first = ledger.inserted(id(7), 2.0, None, Some("hall/a.png".into()), t0);
        assert_eq!(first.insertions_of_path, 1);
        ledger.removed(id(7));
        let again = ledger.inserted(id(8), 2.0, None, Some("hall/a.png".into()), t0);
        assert_eq!(again.insertions_of_path, 2, "the path was decoded twice");
        assert_eq!(ledger.re_decodes, 1);
        // A different path is not a re-decode of this one.
        let other = ledger.inserted(id(9), 2.0, None, Some("hall/b.png".into()), t0);
        assert_eq!(other.insertions_of_path, 1);
        assert_eq!(ledger.re_decodes, 1);
    }

    /// The readiness term needs positive proof of the GPU stamp while a render
    /// world is present, and is `false` without a render world.
    #[test]
    fn the_gpu_readiness_term_wants_the_gpu_stamp_while_a_render_world_is_present() {
        let mut ledger = ImageStageLedger::default();
        let headless = RenderWorldPresent(false);
        let rendering = RenderWorldPresent(true);
        let t0 = Instant::now();
        ledger.inserted(id(10), 1.0, None, None, t0);
        assert!(
            !ledger.is_awaiting_gpu(id(10), headless),
            "no render world: a reveal must never wait on a GPU that does not exist"
        );
        assert!(
            ledger.is_awaiting_gpu(id(10), rendering),
            "inserted and unprepared: owed"
        );
        // The render-world fact is per App: one ledger gives different
        // answers to a rendering App and a headless App for the same id.
        assert_ne!(
            ledger.is_awaiting_gpu(id(10), headless),
            ledger.is_awaiting_gpu(id(10), rendering),
            "the same ledger must answer a headless App and a rendering App differently"
        );
        // Readiness polls in `Update`; the insertion is stamped in `Last`. An
        // id the ledger has not seen yet is owed, not ready.
        assert!(
            ledger.is_awaiting_gpu(id(11), rendering),
            "not yet stamped inserted: the GPU has not proven anything, so owed"
        );
        ledger.gpu_prepared(id(10), Some(t0 + Duration::from_millis(5)));
        assert!(
            !ledger.is_awaiting_gpu(id(10), rendering),
            "prepared: ready"
        );
        assert!(ledger.is_gpu_prepared(id(10)));
        ledger.inserted(id(12), 1.0, None, None, t0);
        ledger.removed(id(12));
        assert!(
            ledger.is_awaiting_gpu(id(12), rendering),
            "dropped before upload: no proof, so still owed if anything still asks"
        );
    }

    /// The readiness fact must not depend on a timestamp. `Instant` is
    /// native-only, so wasm would never see an image as prepared and would
    /// lift the cover before the GPU upload.
    ///
    /// Passing `None` for the timestamp reproduces the wasm case in a native
    /// test.
    #[test]
    fn a_gpu_stamp_with_no_clock_still_makes_the_image_ready() {
        let mut ledger = ImageStageLedger::default();
        let rendering = RenderWorldPresent(true);
        let t0 = Instant::now();
        ledger.inserted(id(40), 1.0, None, None, t0);
        assert!(
            ledger.is_awaiting_gpu(id(40), rendering),
            "premise: inserted and unprepared is owed"
        );

        // The clockless stamp, as on the web.
        assert!(ledger.gpu_prepared(id(40), None).is_some());
        assert!(
            ledger.is_gpu_prepared(id(40)),
            "a target with no Instant still PROVED the GPU has it; readiness is \
             a fact, not a duration"
        );
        assert!(
            !ledger.is_awaiting_gpu(id(40), rendering),
            "so the reveal must stop waiting — on the web exactly as on native"
        );

        // The telemetry is absent, not faked.
        let row = ledger.rows.get(&id(40)).expect("the row exists");
        assert!(row.gpu_prepared, "the fact is recorded");
        assert!(
            row.gpu_prepared_at.is_none(),
            "and no timestamp was invented to carry it"
        );
    }

    /// A native stamp records both, so the report keeps its duration.
    #[test]
    fn a_gpu_stamp_with_a_clock_records_the_fact_and_the_duration() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();
        ledger.inserted(id(41), 1.0, None, None, t0);
        let stamped = ledger
            .gpu_prepared(id(41), Some(t0 + Duration::from_millis(7)))
            .expect("the row was awaiting");
        assert!(stamped.gpu_prepared);
        assert_eq!(
            stamped.insert_to_gpu(),
            Some(Duration::from_millis(7)),
            "the native road still measures insert→gpu"
        );
    }

    /// The resident census groups what is inserted-and-not-removed by road,
    /// and a removal leaves the census.
    #[test]
    fn resident_megapixels_are_grouped_by_the_road_that_demanded_them() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();
        ledger.demand(id(20), "character-sheet", "a.png".into(), t0);
        ledger.demand(id(21), "character-sheet", "b.png".into(), t0);
        ledger.demand(id(22), "parallax", "sky.png".into(), t0);
        ledger.inserted(id(20), 4.0, None, None, t0);
        ledger.inserted(id(21), 2.0, None, None, t0);
        ledger.inserted(id(22), 1.0, None, None, t0);
        ledger.inserted(id(23), 0.5, None, Some("icon.png".into()), t0);
        // Demanded but not yet inserted: not resident.
        ledger.demand(id(24), "parallax", "far.png".into(), t0);
        let census = ledger.resident_by_road();
        assert_eq!(census.get("character-sheet"), Some(&(2, 6.0)));
        assert_eq!(census.get("parallax"), Some(&(1, 1.0)));
        assert_eq!(
            census.get("?"),
            Some(&(1, 0.5)),
            "an unstamped image is counted, under `?`"
        );
        ledger.removed(id(20));
        assert_eq!(
            ledger.resident_by_road().get("character-sheet"),
            Some(&(1, 2.0))
        );
    }

    /// An unrouted file and a procedural image are different findings.
    ///
    /// A file with no demand stamp is a finding. An image with no file can
    /// never get a demand road, because it has no load to stamp. Check both
    /// buckets: either check alone passes on a ledger that puts everything in
    /// one bucket.
    #[test]
    fn a_file_nobody_demanded_is_a_finding_and_a_procedural_insert_is_not() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();
        // A file with no demand stamp: something loaded art and no road said so.
        ledger.inserted(id(30), 7.6, None, Some("preview_tileset.png".into()), t0);
        // Two images with no file behind them.
        ledger.inserted(id(31), 0.3, None, None, t0);
        ledger.inserted(id(32), 1.0, None, None, t0);

        let census = ledger.resident_by_road();
        assert_eq!(
            census.get(ROAD_UNROUTED),
            Some(&(1, 7.6)),
            "the unrouted bucket must hold the FILE and only the file",
        );
        assert_eq!(
            census.get(ROAD_PROCEDURAL),
            Some(&(2, 1.3)),
            "images with no file belong in their own bucket, not among findings",
        );

        assert_eq!(
            ledger.unrouted_resident(),
            vec![(7.6, "preview_tileset.png")],
            "the named unrouted list is what a host run reads; a procedural \
             insert in it is a name nobody can act on",
        );
        assert_eq!(ledger.procedural_resident(), (2, 1.3));
    }

    /// First write wins, and the second call returns nothing.
    ///
    /// Extraction runs every frame for every visible sprite, so an overwriting
    /// stamp would add a per-frame write for the whole visible set. The
    /// caller prints the demand→draw wait only when it gets `Some`.
    #[test]
    fn the_first_draw_is_stamped_once_and_later_frames_report_nothing() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();
        ledger.demand(id(40), "character-sheet", "hero.png".into(), t0);
        ledger.inserted(id(40), 4.0, None, None, t0);

        let first = t0 + Duration::from_millis(120);
        let waited = ledger
            .first_drawn(id(40), first)
            .expect("the first draw of a demanded image reports its wait");
        assert_eq!(waited, Duration::from_millis(120));

        assert_eq!(
            ledger.first_drawn(id(40), first + Duration::from_millis(16)),
            None,
            "a later frame re-stamped the first draw: the stage would be a \
             per-frame write on every visible sprite, and its own cost would be \
             part of what it measures",
        );
        assert_eq!(
            ledger.get(id(40)).and_then(|row| row.first_drawn_at),
            Some(first),
            "the later frame moved the instant, so `first_drawn_at` is not the \
             FIRST draw at all",
        );
    }

    /// A first draw while gameplay is live is a pop. The flag follows the live
    /// state at the draw, not at the insert.
    #[test]
    fn a_first_draw_while_gameplay_is_live_is_recorded_as_one() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();

        // Decoded under a cover and drawn under it: not a pop.
        ledger.set_gameplay_live(Some(false));
        ledger.demand(id(60), "character-sheet", "covered.png".into(), t0);
        ledger.inserted(id(60), 2.0, Some(false), None, t0);
        ledger.first_drawn(id(60), t0 + Duration::from_millis(20));
        assert_eq!(
            ledger.get(id(60)).and_then(|r| r.live_at_first_draw),
            Some(false),
        );

        // Decoded under the cover and first drawn after it lifted: a pop that
        // `live_at_insert` cannot see.
        ledger.demand(id(61), "character-sheet", "late.png".into(), t0);
        ledger.inserted(id(61), 2.0, Some(false), None, t0);
        ledger.set_gameplay_live(Some(true));
        ledger.first_drawn(id(61), t0 + Duration::from_millis(900));
        let row = ledger.get(id(61)).expect("the late sheet has a row");
        assert_eq!(
            row.live_at_insert,
            Some(false),
            "the decode happened under the cover, and that is what makes this \
             the interesting case",
        );
        assert_eq!(
            row.live_at_first_draw,
            Some(true),
            "an image first drawn after the cover lifted is a POP, and reading \
             the insert's liveness instead reports the frame it did not cost",
        );
    }

    /// A composition that never covers anything cannot have a pop.
    ///
    /// `capture_scene` boots straight into `playing`, so every first draw
    /// there is "during gameplay". The readout checks `saw_covered_frame`
    /// before it reports a pop.
    #[test]
    fn a_process_that_never_covered_a_frame_can_report_no_pop() {
        let mut ledger = ImageStageLedger::default();
        assert!(
            !ledger.saw_covered_frame(),
            "a fresh ledger has seen no cover, which is what makes the default \
             reading conservative",
        );
        ledger.set_gameplay_live(Some(true));
        assert!(
            !ledger.saw_covered_frame(),
            "live frames alone are not evidence a cover exists; a harness that \
             boots into `playing` reports only these",
        );
        ledger.set_gameplay_live(Some(false));
        assert!(
            ledger.saw_covered_frame(),
            "one not-live frame is the whole evidence a cover ran, and without \
             noticing it the readout calls every first draw a pop",
        );
        ledger.set_gameplay_live(Some(true));
        assert!(
            ledger.saw_covered_frame(),
            "the fact is that a cover EXISTED, so it must not be cleared when \
             play resumes",
        );
    }

    /// Never-drawn megapixels split by road. The split adds up to the total,
    /// and a drawn image leaves only its own bucket.
    #[test]
    fn never_drawn_splits_by_owner_and_the_split_adds_up() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();
        ledger.demand(id(50), "character-sheet", "hero.png".into(), t0);
        ledger.demand(id(51), "character-sheet", "rival.png".into(), t0);
        ledger.demand(id(52), "fx-sheet", "sparks.png".into(), t0);
        for id_n in [50u128, 51, 52] {
            ledger.inserted(id(id_n), 2.0, None, None, t0);
        }

        let split = ledger.never_drawn_by_road();
        assert_eq!(split.get("character-sheet"), Some(&(2, 4.0)));
        assert_eq!(split.get("fx-sheet"), Some(&(1, 2.0)));
        let total: usize = split.values().map(|(n, _)| n).sum();
        assert_eq!(
            total,
            ledger.resident_never_drawn().len(),
            "the by-road split and the flat list disagree about how many images \
             were never drawn, so one of the two readouts is lying",
        );

        ledger.first_drawn(id(50), t0 + Duration::from_millis(10));
        let split = ledger.never_drawn_by_road();
        assert_eq!(
            split.get("character-sheet"),
            Some(&(1, 2.0)),
            "drawing one sheet emptied or failed to shrink its own road",
        );
        assert_eq!(
            split.get("fx-sheet"),
            Some(&(1, 2.0)),
            "drawing a character sheet moved another road's count",
        );
    }

    /// Never-drawn is not a finding without a render world. With nothing
    /// extracted, every resident image is "never drawn". Only the asking App's
    /// [`RenderWorldPresent`] tells the caller which reading applies.
    #[test]
    fn every_resident_image_is_never_drawn_until_something_extracts_one() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();
        ledger.demand(id(41), "character-sheet", "a.png".into(), t0);
        ledger.demand(id(42), "parallax", "sky.png".into(), t0);
        ledger.inserted(id(41), 4.0, None, None, t0);
        ledger.inserted(id(42), 1.0, None, None, t0);
        assert_eq!(
            ledger.resident_never_drawn(),
            vec![(4.0, "a.png"), (1.0, "sky.png")],
            "largest first, so the expensive one is the one that gets read",
        );

        ledger.first_drawn(id(41), t0 + Duration::from_millis(50));
        assert_eq!(
            ledger.resident_never_drawn(),
            vec![(1.0, "sky.png")],
            "an image that was drawn is off the list; anything else makes the \
             readout unable to distinguish waste from work",
        );
    }

    #[test]
    fn an_image_that_arrived_by_another_road_says_so() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();
        let row = ledger.inserted(id(2), 1.0, None, Some("b.png".into()), t0);
        assert_eq!(row.path.as_deref(), Some("b.png"));
        assert_eq!(row.demand_to_insert(), None);
        assert!(row.demand_phrase().starts_with("demand=unknown"));
    }

    #[test]
    fn a_prepared_report_for_an_image_nobody_awaited_is_none() {
        let mut ledger = ImageStageLedger::default();
        assert!(ledger.gpu_prepared(id(3), Some(Instant::now())).is_none());
        // A removal before preparation stops the wait and counts as a decode
        // nobody drew.
        ledger.inserted(id(4), 1.5, None, None, Instant::now());
        let dropped = ledger
            .removed(id(4))
            .expect("dropped before the GPU saw it");
        assert_eq!(dropped.megapixels, 1.5);
        assert!(ledger.awaiting_gpu().is_empty());
        assert!(ledger.gpu_prepared(id(4), Some(Instant::now())).is_none());
        assert_eq!(ledger.dropped_before_gpu, 1);
        // A removal after preparation is an ordinary retirement.
        ledger.inserted(id(5), 1.0, None, None, Instant::now());
        ledger.gpu_prepared(id(5), Some(Instant::now()));
        assert!(ledger.removed(id(5)).is_none());
        assert_eq!(ledger.dropped_before_gpu, 1);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod app_local_gpu_readiness {
    use super::*;

    /// Same shape as the module's other tests: `bevy::image::Image` and a bare
    /// `uuid` crate are not reachable here.
    fn id(n: u128) -> UntypedAssetId {
        bevy::asset::AssetId::<bevy::asset::LoadedUntypedAsset>::Uuid {
            uuid: bevy::asset::uuid::Uuid::from_u128(n),
        }
        .untyped()
    }

    /// Two rendering Apps that share an asset id must not settle each other's
    /// reveal. Asset ids are App-local and can collide.
    #[test]
    fn preparation_in_one_app_does_not_settle_another_that_shares_the_id() {
        let a = AppGpuPreparedImages::default();
        let b = AppGpuPreparedImages::default();
        let rendering = RenderWorldPresent(true);
        let shared = id(7);

        // Non-vacuity: both wait on the same id before anything happens.
        assert!(a.is_awaiting_gpu(shared, rendering));
        assert!(b.is_awaiting_gpu(shared, rendering));

        a.mark_prepared(shared);

        assert!(
            !a.is_awaiting_gpu(shared, rendering),
            "A prepared it, so A is settled"
        );
        assert!(
            b.is_awaiting_gpu(shared, rendering),
            "B must STILL be waiting: A's render world uploaded A's image, and the \
             fact that the two Apps happen to number it the same is not evidence \
             about B. This is the assertion the process-global ledger could not make."
        );
        assert_eq!(a.prepared_count(), 1);
        assert_eq!(b.prepared_count(), 0);
    }

    /// Preparing in App A must not consume App B's candidate.
    ///
    /// The test above proves A's stamp is not B's answer. This one proves it
    /// is not B's candidate either. With a shared candidate list,
    /// `gpu_prepared()` removes the entry, so the other App would never see the
    /// id again and its cover would never lift.
    #[test]
    fn preparing_one_app_leaves_the_other_app_its_own_candidate() {
        let a = AppGpuPreparedImages::default();
        let b = AppGpuPreparedImages::default();
        let shared = id(7);

        a.mark_awaiting(shared);
        b.mark_awaiting(shared);
        assert_eq!(a.awaiting_ids(), vec![shared]);
        assert_eq!(b.awaiting_ids(), vec![shared]);

        a.mark_prepared(shared);

        assert!(
            a.awaiting_ids().is_empty(),
            "A prepared it, so A stops looking"
        );
        assert_eq!(
            b.awaiting_ids(),
            vec![shared],
            "B must still have its own candidate: A's render world consuming a shared \
             global entry is exactly the defect this set replaces",
        );
        assert!(!b.is_prepared(shared));
    }

    /// The candidate half of the readiness fact: an arrival makes an id something
    /// this App's render world will look for, and preparing it stops the search.
    #[test]
    fn an_arrival_becomes_a_candidate_and_preparation_retires_it() {
        let app = AppGpuPreparedImages::default();
        assert_eq!(
            app.awaiting_count(),
            0,
            "nothing is owed before anything arrives"
        );

        app.mark_awaiting(id(1));
        app.mark_awaiting(id(2));
        assert_eq!(app.awaiting_count(), 2);

        app.mark_prepared(id(1));
        assert_eq!(app.awaiting_ids(), vec![id(2)]);
        assert_eq!(app.prepared_count(), 1);
        assert!(app.is_prepared(id(1)));
    }

    /// An arrival for an already-prepared id re-opens it.
    ///
    /// The stamp means "these contents are on the GPU". An `Added` for a
    /// recycled id names a different image, and a `Modified` names different
    /// bytes. Both must retire the old stamp, or a barrier lifts over a GPU
    /// copy that no longer exists.
    #[test]
    fn arriving_again_retires_the_proof_and_makes_the_id_pending() {
        let app = AppGpuPreparedImages::default();
        app.mark_awaiting(id(3));
        app.mark_prepared(id(3));
        assert!(app.is_prepared(id(3)));

        app.mark_awaiting(id(3));

        assert!(
            !app.is_prepared(id(3)),
            "the proof was about contents that have just been replaced"
        );
        assert_eq!(
            app.awaiting_ids(),
            vec![id(3)],
            "and the current contents are pending, so the render world looks again"
        );
    }

    /// The full generation cycle of a modified image: arrive, prepare, settle;
    /// modify, go pending, prepare again, settle again.
    ///
    /// The middle assertion is the important one: right after the modify,
    /// `is_awaiting_gpu` must say "owed" while Bevy prepares the new copy.
    #[test]
    fn a_modified_image_is_pending_again_until_the_gpu_has_the_new_contents() {
        let app = AppGpuPreparedImages::default();
        let drawing = RenderWorldPresent(true);

        app.mark_awaiting(id(5));
        assert!(
            app.is_awaiting_gpu(id(5), drawing),
            "arrived, not yet uploaded"
        );
        app.mark_prepared(id(5));
        assert!(!app.is_awaiting_gpu(id(5), drawing), "settled");

        // The bytes change in place. Bevy re-extracts and re-prepares.
        app.mark_awaiting(id(5));
        assert!(
            app.is_awaiting_gpu(id(5), drawing),
            "the modified contents have NOT reached the GPU; a barrier must wait"
        );

        app.mark_prepared(id(5));
        assert!(!app.is_awaiting_gpu(id(5), drawing), "settled again");
    }

    /// `Unused` retires the id from both sets, from either state.
    ///
    /// An image dropped before the GPU saw it must leave the candidate queue,
    /// or the render world polls for it forever. One dropped after must lose
    /// its proof, because `Unused` removes the render representation.
    #[test]
    fn an_unused_image_leaves_both_the_queue_and_the_proof() {
        let app = AppGpuPreparedImages::default();

        app.mark_awaiting(id(8));
        app.mark_retired(id(8));
        assert!(
            app.awaiting_ids().is_empty(),
            "a dropped candidate stops being polled"
        );
        assert!(!app.is_prepared(id(8)));

        app.mark_awaiting(id(9));
        app.mark_prepared(id(9));
        app.mark_retired(id(9));
        assert!(
            !app.is_prepared(id(9)),
            "`Unused` removes the render representation, so the proof goes with it"
        );
        assert!(app.awaiting_ids().is_empty());
    }

    /// A headless App never prepares anything, so nothing may wait on it.
    #[test]
    fn a_headless_app_is_never_awaiting() {
        let headless = AppGpuPreparedImages::default();
        assert!(!headless.is_awaiting_gpu(id(7), RenderWorldPresent(false)));
        assert!(headless.is_awaiting_gpu(id(7), RenderWorldPresent(true)));
    }

    /// The set is shared through its `Arc`, so the render sub-app's write
    /// reaches the main world's read inside one App.
    #[test]
    fn a_clone_is_the_same_set_because_one_app_shares_it_across_worlds() {
        let main_world = AppGpuPreparedImages::default();
        let render_world = main_world.clone();
        render_world.mark_prepared(id(3));
        assert!(
            main_world.is_prepared(id(3)),
            "the render sub-app's stamp must be visible to the App's own main world",
        );
    }
}
