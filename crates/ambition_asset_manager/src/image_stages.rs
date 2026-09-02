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
//! [`note_demand`]: `load_sheet_image` and the manifest catalog's
//! `load_optional`. An image that arrives by another road still gets its
//! insertion and GPU stamps (keyed by asset id when the census first sees it)
//! but reports `demand=unknown`.
//!
//! ⛔ THE ROAD VOCABULARY IS CLOSED AT EIGHT, and an addition wants a reason:
//!
//! ```text
//! character-sheet  parallax  fx-sheet  boss-sheet
//! asset-manifest   portrait  projectile-art  held-item
//! ```
//!
//! They name CONTENT ART decoded at runtime, because that is the population a
//! room reveal waits on. ⚠ Menu icons, shell presentation images and prop pngs
//! are deliberately NOT stamped — small, loaded once, and not what a reveal
//! waits for; labelling them would make these rows less comparable rather than
//! more. A `demand=unknown` on one of those is expected, not work.
//!
//! ⚠ AND `demand=unknown` HAS A SECOND CAUSE, so it is not simply "a road still
//! to route": see [`ImageStageLedger::removed`] — a dropped image loses its
//! demand row, so a RE-DECODE arrives unattributed. Two very different facts
//! print the same word.
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

    /// `demand→insert 123ms via character-sheet`, `first demanded via <road>`
    /// for a re-decode, or `demand=unknown`.
    ///
    /// ⚠ THE THREE ARE DIFFERENT FACTS AND USED TO PRINT AS TWO. A re-decode has
    /// a known demander but no honest wait — `removed` took its row, and the
    /// path's first demand instant belongs to the earlier decode — so quoting a
    /// duration would be inventing one. It says who asked and stops there.
    /// `demand=unknown` is now reserved for what it claims: an image that
    /// reached `Assets<Image>` by a road that stamps nothing.
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
    /// The FIRST demand recorded for a path, kept beside the insertion count and
    /// for the same reason.
    ///
    /// ⛔⛔ WITHOUT THIS, A RE-DECODE IS UNATTRIBUTABLE. `removed` deletes the
    /// whole per-id row, demand included, and `demand()` only ever runs at a LOAD
    /// call site — a second `load` of a resident path is a handle lookup, not a
    /// decode. So a demote-then-redecode came back reading `demand=unknown`,
    /// which is also what an unrouted road prints. Two very different facts, one
    /// word, and chasing the wrong one costs an afternoon looking for roads that
    /// are already stamped.
    ///
    /// ⇒ The count survived a removal and the attribution for it did not, which
    /// is backwards: the wasted decode is exactly the one whose demander you want
    /// named, and `dropped_before_gpu` exists to count that population.
    demand_by_path: BTreeMap<String, (&'static str, Instant)>,
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
            // ⭐ A RE-DECODE INHERITS THE PATH'S FIRST DEMAND. `removed` deleted
            // the per-id row, so this insertion's row is blank even though the
            // file was demanded by a known road earlier — without this the
            // re-decode prints `demand=unknown`, which is also what an UNROUTED
            // load prints, and the two are not the same fact at all.
            //
            // ⚠ `demanded_at` is the FIRST demand's instant, not this decode's,
            // so `wait()` would measure from the wrong moment. Only the SOURCE is
            // adopted; the row keeps no `demanded_at`, and the readout says
            // "first demanded via <road>" rather than quoting a duration it
            // cannot honestly compute.
            let inherited = self.demand_by_path.get(&path).map(|(source, _)| *source);
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

    /// Every image inserted and not yet removed, in id order — the per-row
    /// form of [`Self::resident_by_road`], for a census that wants the PATHS.
    pub fn resident_rows(&self) -> impl Iterator<Item = &ImageStages> {
        self.rows.values().filter(|row| row.inserted_at.is_some())
    }

    /// Every row the ledger holds — demanded, inserted or both.
    pub fn rows(&self) -> impl Iterator<Item = &ImageStages> {
        self.rows.values()
    }

    /// The UNROUTED resident images, largest first: every image that came from a
    /// FILE and reached `Assets<Image>` without passing a stamped demand road.
    ///
    /// ⛔⛔ THE ONE BUCKET A COUNT CANNOT ANSWER. Unrouted means *nobody claims
    /// to have asked for this*, so the next question is always WHICH — and until
    /// this existed the only way to find out was to probe the ledger by hand.
    /// That is how the Hall's one unrouted image was identified on 2026-09-02
    /// (the LDtk editor-preview tileset), and it should not have taken a bespoke
    /// probe.
    ///
    /// ⛔ FILE-BACKED ONLY, and the split is the whole point. See
    /// [`Self::procedural_resident`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn unrouted_resident(&self) -> Vec<(f64, &str)> {
        let mut rows: Vec<(f64, &str)> = self
            .rows
            .values()
            .filter(|row| row.inserted_at.is_some() && row.source.is_none())
            .filter_map(|row| Some((row.megapixels, row.path.as_deref()?)))
            .collect();
        // Megapixels descending, then path, so two censuses diff cleanly and the
        // expensive one is the one that gets read.
        rows.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(b.1)));
        rows
    }

    /// Resident images that came from no file at all — inserted directly into
    /// `Assets<Image>` rather than decoded: render targets, procedural sprites,
    /// shader inputs.
    ///
    /// ⛔⛔ NOT THE SAME FACT AS UNROUTED, AND THEY SHARED A BUCKET. A row with
    /// `source == None` was keyed `"?"` whether it was a FILE nobody stamped or
    /// an image with no file to stamp — and the second kind can never acquire a
    /// demand road, because there is no load to stamp. Measured 2026-09-02 on
    /// the Hall: 24 of the 24 "unrouted" images had no path at all, so a census
    /// line reading `UNROUTED(no demand) 24×4.5MP` reported 24 findings where
    /// there were none, and on the host would have buried the one that matters
    /// (the 7.6 MP LDtk editor-preview tileset) inside its own noise.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn procedural_resident(&self) -> (usize, f64) {
        self.rows
            .values()
            .filter(|row| row.inserted_at.is_some() && row.source.is_none())
            .filter(|row| row.path.is_none())
            .fold((0usize, 0f64), |(n, mp), row| (n + 1, mp + row.megapixels))
    }

    /// WHAT IS RESIDENT, BY THE ROAD THAT DEMANDED IT: megapixels of every
    /// image inserted and not yet removed, grouped by source label (asset open
    /// work 4 asks for the owner of retained assets before any eviction policy;
    /// this is the measurement that names the owners). Images no road stamped
    /// group under [`ROAD_UNROUTED`] when they came from a file and
    /// [`ROAD_PROCEDURAL`] when they did not. Deterministic order, so two
    /// censuses diff cleanly.
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

/// [`ImageStageLedger::resident_by_road`] key for a FILE-backed image that
/// reached `Assets<Image>` without passing a stamped demand road. A finding:
/// something loaded art and no road said so.
pub const ROAD_UNROUTED: &str = "?";

/// [`ImageStageLedger::resident_by_road`] key for an image with no file behind
/// it — inserted directly rather than decoded.
///
/// ⛔ NOT A FINDING, and it shared a key with one until 2026-09-02. A procedural
/// image can never acquire a demand road, because there is no load to stamp;
/// counting it as unrouted put 24 non-findings in the Hall's bucket and would
/// have buried the single real one on the host.
pub const ROAD_PROCEDURAL: &str = "~procedural";

static LEDGER: Mutex<ImageStageLedger> = Mutex::new(ImageStageLedger {
    rows: BTreeMap::new(),
    awaiting_gpu: Vec::new(),
    gameplay_live: None,
    insertions_by_path: BTreeMap::new(),
    demand_by_path: BTreeMap::new(),
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

    /// ⛔⛔ A RE-DECODE KNOWS WHO ASKED FOR IT THE FIRST TIME.
    ///
    /// `removed` deletes the per-id row, and `demand()` only runs at a LOAD call
    /// site — a second `load` of a resident path is a handle lookup, not a
    /// decode. So a demote-then-redecode used to come back reading
    /// `demand=unknown`, which is ALSO what an image loaded by an unstamped road
    /// prints. Two entirely different facts wearing one word: one is "this road
    /// needs routing", the other is "this file was decoded twice". Chasing the
    /// first when it was the second costs an afternoon.
    ///
    /// The path's first demand now outlives the row, exactly as
    /// `insertions_by_path` already did.
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

        // The same FILE decoded again under a new asset id, with nothing calling
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

        // ⚠ NO WAIT IS QUOTED. `demanded_at` belongs to the FIRST decode;
        // measuring this insertion against it would invent a duration.
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

    /// ⛔⛔ AN UNROUTED FILE AND A PROCEDURAL IMAGE ARE NOT THE SAME FINDING,
    /// AND THEY SHARED A BUCKET.
    ///
    /// `source == None` was keyed `"?"` for both — a FILE that decoded with
    /// nobody claiming to have asked for it, and an image with no file at all.
    /// The second can never acquire a demand road, because there is no load to
    /// stamp. Measured on the Hall 2026-09-02: 24 of the 24 "unrouted" images
    /// had no path, so the census line read `UNROUTED(no demand) 24×4.5MP` and
    /// every one of them was a non-finding — while the one that matters on the
    /// host (the 7.6 MP LDtk editor-preview tileset) would have been the 25th
    /// entry in a bucket nobody could read.
    ///
    /// ⛔ BOTH HALVES, because either alone passes on a ledger that puts
    /// everything in one bucket: the split is what is being pinned, not the
    /// presence of a key.
    #[test]
    fn a_file_nobody_demanded_is_a_finding_and_a_procedural_insert_is_not() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();
        // A FILE with no demand stamp: something loaded art and no road said so.
        ledger.inserted(id(30), 7.6, None, Some("preview_tileset.png".into()), t0);
        // Two images with no file behind them at all.
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
