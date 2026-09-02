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
        let snapshot = row.clone();
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

    /// An image the main world dropped before the GPU ever saw it (a demoted
    /// tier, a room left). Not an error; it just stops being awaited.
    pub fn removed(&mut self, id: UntypedAssetId) {
        self.awaiting_gpu.retain(|awaiting| *awaiting != id);
        self.rows.remove(&id);
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
        // And a removal before preparation stops the wait without a report.
        ledger.inserted(id(4), 1.0, None, None, Instant::now());
        ledger.removed(id(4));
        assert!(ledger.awaiting_gpu().is_empty());
        assert!(ledger.gpu_prepared(id(4), Instant::now()).is_none());
    }
}
