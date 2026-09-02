//! Per-image STAGE ledger: when an image was demanded, when its decoded pixels
//! reached `Assets<Image>`, and when the GPU copy was prepared.
//!
//! A frame spike during a room reveal has three different owners — the IO
//! pool decoding, the main world inserting, the render world uploading — and
//! `[image]` lines alone name only the middle one. This ledger keeps the three
//! instants per asset so a late image can say WHICH stage was late, and how
//! long after it was asked for.
//!
//! Process-global on purpose: the demand is recorded by a free function with no
//! world in hand (`load_sheet_image`), the insertion by a main-world system, and
//! the GPU preparation by a RENDER-world system. Three worlds, one ledger. It is
//! a diagnostic — nothing authoritative reads it, and it is never rollback state.
//!
//! Coverage is exactly the images demanded through a road that calls
//! [`note_demand`]: `load_sheet_image` (character sheet pages, parallax layers,
//! fx sheets, boss pages) and the manifest catalog's `load_optional`. An image
//! that arrives by another road still gets its insertion and GPU stamps (keyed
//! by asset id when the census first sees it) but reports `demand=unknown`,
//! which is the honest answer and also the list of roads still to route.
//!
//! Keyed by [`UntypedAssetId`] so a generic `load_optional::<T>` can record a
//! demand without knowing it is an image; the census only ever asks about
//! image ids, so a non-image row is simply never consulted.

use bevy::asset::UntypedAssetId;
use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};
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
    pub megapixels: f64,
    /// Whether gameplay was live when the pixels were inserted.
    pub live_at_insert: Option<bool>,
    /// How many times THIS PATH has been inserted since the process started
    /// (1 = the first decode). A second insertion of the same path is a
    /// re-decode: the asset was dropped and demanded again, or loaded twice
    /// under two ids — asset open work 5 in
    /// `asset-preparation-and-residency.md`.
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
            megapixels: 0.0,
            live_at_insert: None,
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

    /// `demand→insert 123ms via character-sheet`, or `demand=unknown`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn demand_phrase(&self) -> String {
        match (self.demand_to_insert(), self.source) {
            (Some(d), Some(source)) => {
                format!("demand→insert {:.0}ms via {source}", d.as_secs_f64() * 1e3)
            }
            _ => "demand=unknown (not through load_sheet_image)".to_string(),
        }
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
    /// Insertions per PATH, across ids and across removals: the re-decode
    /// census. Survives `removed`, which is the point.
    insertions_by_path: BTreeMap<String, u32>,
    /// Whether a render world is stamping stage 3 at all. `false` in a
    /// headless or `NoWindow` composition, where nothing is ever prepared on a
    /// GPU — and where a readiness rule that waited for it would wait forever.
    render_world_present: bool,
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
        let row = self.row(id);
        // First demand wins: a second `load` of the same path is a handle
        // lookup, not a second decode, and the wait that matters is the first.
        if row.demanded_at.is_none() {
            row.demanded_at = Some(at);
            row.source = Some(source);
            row.path = Some(path);
        }
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
            let count = self.insertions_by_path.entry(path).or_default();
            *count += 1;
            if *count > 1 {
                self.re_decodes += 1;
            }
            let count = *count;
            let row = self.row(id);
            row.insertions_of_path = count;
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
        self.gameplay_live = live;
    }

    pub fn gameplay_live(&self) -> Option<bool> {
        self.gameplay_live
    }

    pub fn awaiting_gpu(&self) -> &[UntypedAssetId] {
        &self.awaiting_gpu
    }

    /// The render world's stamp exists in this process (the plugin found a
    /// render app to install into).
    pub fn set_render_world_present(&mut self, present: bool) {
        self.render_world_present = present;
    }

    pub fn render_world_present(&self) -> bool {
        self.render_world_present
    }

    /// READINESS TERM: `id` was decoded and inserted, and a render world exists
    /// that has not yet prepared it. `false` whenever no render world stamps
    /// stage 3 — a headless run never waits on a GPU it does not have — and
    /// `false` for an id this ledger never saw inserted, which is the loading
    /// stages' question, not this one.
    ///
    /// A room whose reveal waits on this converts the upload of its cast from a
    /// frame after the cover lifts into cover time: the pixels were paid for
    /// either way, and under a byte-per-frame budget they pace while the cover
    /// still holds.
    pub fn is_awaiting_gpu(&self, id: UntypedAssetId) -> bool {
        self.render_world_present && self.awaiting_gpu.contains(&id)
    }

    /// The render world saw `id` prepared. Returns the row for reporting.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn gpu_prepared(&mut self, id: UntypedAssetId, at: Instant) -> Option<ImageStages> {
        let position = self
            .awaiting_gpu
            .iter()
            .position(|awaiting| *awaiting == id)?;
        self.awaiting_gpu.swap_remove(position);
        let row = self.row(id);
        row.gpu_prepared_at = Some(at);
        let snapshot = row.clone();
        self.gpu_prepared_total += 1;
        self.gpu_prepared_megapixels += snapshot.megapixels;
        self.window_gpu_prepared += 1;
        self.window_gpu_megapixels += snapshot.megapixels;
        if let Some(d) = snapshot.insert_to_gpu() {
            self.window_insert_to_gpu.push(d);
        }
        Some(snapshot)
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
}

static LEDGER: Mutex<ImageStageLedger> = Mutex::new(ImageStageLedger {
    rows: BTreeMap::new(),
    awaiting_gpu: Vec::new(),
    gameplay_live: None,
    insertions_by_path: BTreeMap::new(),
    render_world_present: false,
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
            .gpu_prepared(id(1), t0 + Duration::from_millis(150))
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

    /// The readiness term is exactly "inserted, render world present, not yet
    /// prepared" — and NOTHING without a render world.
    #[test]
    fn the_gpu_readiness_term_only_holds_while_a_render_world_owes_the_upload() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();
        ledger.inserted(id(10), 1.0, None, None, t0);
        assert!(
            !ledger.is_awaiting_gpu(id(10)),
            "no render world: a reveal must never wait on a GPU that does not exist"
        );
        ledger.set_render_world_present(true);
        assert!(
            ledger.is_awaiting_gpu(id(10)),
            "inserted and unprepared: owed"
        );
        assert!(
            !ledger.is_awaiting_gpu(id(11)),
            "never inserted: not this term's question"
        );
        ledger.gpu_prepared(id(10), t0 + Duration::from_millis(5));
        assert!(!ledger.is_awaiting_gpu(id(10)), "prepared: ready");
        ledger.inserted(id(12), 1.0, None, None, t0);
        ledger.removed(id(12));
        assert!(
            !ledger.is_awaiting_gpu(id(12)),
            "dropped before upload: nothing owed"
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
        assert!(ledger.gpu_prepared(id(3), Instant::now()).is_none());
        // And a removal before preparation stops the wait, and is counted as a
        // decode nobody drew.
        ledger.inserted(id(4), 1.5, None, None, Instant::now());
        let dropped = ledger
            .removed(id(4))
            .expect("dropped before the GPU saw it");
        assert_eq!(dropped.megapixels, 1.5);
        assert!(ledger.awaiting_gpu().is_empty());
        assert!(ledger.gpu_prepared(id(4), Instant::now()).is_none());
        assert_eq!(ledger.dropped_before_gpu, 1);
        // A removal AFTER preparation is an ordinary retirement.
        ledger.inserted(id(5), 1.0, None, None, Instant::now());
        ledger.gpu_prepared(id(5), Instant::now());
        assert!(ledger.removed(id(5)).is_none());
        assert_eq!(ledger.dropped_before_gpu, 1);
    }
}
