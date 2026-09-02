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
//! a diagnostic — and it is never rollback state.
//!
//! ⛔⛔ THIS HEADER USED TO SAY "nothing authoritative reads it". THAT WAS FALSE
//! AND IT COST A CORRECTNESS BUG. `inspect_room_asset_manifest` read
//! [`ImageStageLedger::is_awaiting_gpu`] as a REVEAL CONDITION — the cover over a
//! room transition would not lift until this ledger said the GPU had the pixels.
//! A process-global structure keyed by `UntypedAssetId` was deciding an App-local
//! question, and asset ids are LOCAL TO AN App (this repository has measured them
//! colliding), so one App's upload could lift another App's cover.
//!
//! ⭐ Since 2026-09-02 the authority is [`AppGpuPreparedImages`], one per App,
//! written by that App's render-world stamper and read by that App's reveal. The
//! ledger still MIRRORS every stamp and must keep doing so — the `[image-gpu]`
//! lines, the insert→gpu timings and the census all read it. What it may not do
//! is decide. If you are about to read this ledger to make a decision rather
//! than to print a number, that is the bug this paragraph is here to stop.
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
    /// The first frame this image would actually be DRAWN — the fourth stage.
    ///
    /// ⭐⭐ THE OTHER THREE ARE ALL ABOUT THE ASSET ARRIVING. Demand, insert and
    /// GPU say it was asked for, decoded and uploaded; none of them says it was
    /// ever USED. That gap is why the re-decode census and the reveal barrier
    /// both have to talk about *"prepared"* rather than *"drawn"*, and why
    /// `[image-dropped]` can only report pixels decoded for nobody after the
    /// fact.
    ///
    /// ⛔ ABSENT MEANS TWO DIFFERENT THINGS AND A READER MUST NOT CONFLATE THEM:
    /// "no render world at all" (a `NoWindow` or headless composition, where
    /// nothing is ever extracted and this can never be set) and "drawn by
    /// nobody yet". [`RenderWorldPresent`] — the asking App's — is the fact
    /// that separates them, the same asymmetry `is_awaiting_gpu` documents.
    #[cfg(not(target_arch = "wasm32"))]
    pub first_drawn_at: Option<Instant>,
    pub megapixels: f64,
    /// Whether gameplay was live when the pixels were inserted.
    pub live_at_insert: Option<bool>,
    /// Whether gameplay was live the first time this image was DRAWN.
    ///
    /// ⭐⭐ THIS IS THE POP, AND IT IS THE FACT THE WHOLE HITCH LANE IS ABOUT.
    /// A cover exists so a room's art arrives before anybody can see the room;
    /// an image whose FIRST DRAW happens while gameplay is live is one the cover
    /// did not cover — it appeared in front of the player. `live_at_insert`
    /// beside it answers a different question (did the DECODE cost a live
    /// frame), and the two can disagree in both directions: art decoded under
    /// the cover and first drawn minutes later is fine, and art decoded live but
    /// never seen is waste rather than a pop.
    ///
    /// `None` where nothing could tell — no game mode, or no render world.
    pub live_at_first_draw: Option<bool>,
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
/// Whether THIS App has a render world. A per-App fact, and therefore an App
/// resource rather than a field on the process-global ledger.
///
/// ⛔⛔ IT WAS A `bool` ON THE LEDGER, AND THE LEDGER IS A `static` SHARED BY
/// EVERY APP IN THE PROCESS. That field answered "did ANY App in this process
/// install a render plugin", which is a different question from "does the App
/// asking have one" the moment a headless App runs beside a rendering one — and
/// `app_it` is exactly that process, with one `[[test]]` target running its
/// files as parallel threads. A headless App would be told its images were
/// awaiting a GPU that will never look at them, and a reveal gated on that waits
/// forever. Latent while all 97 `VisibleRenderMode` uses were `NoWindow`; live
/// the day one test builds a render world beside one that does not.
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
/// ⛔⛔ WHY THIS EXISTS AND THE LEDGER CANNOT DO IT. [`ImageStageLedger`] is
/// process-global and keyed by `UntypedAssetId`, and asset ids are LOCAL TO AN
/// App — this repository has measured them colliding across Apps. So a render
/// world preparing image id 7 in App A marked "id 7 is prepared" for App B as
/// well, and B's reveal cover could lift on A's upload. The ledger's own
/// `is_awaiting_gpu` says as much in its doc: it "cannot know which App is
/// asking".
///
/// ⭐ THE FIX IS OWNERSHIP, NOT A BETTER KEY. One of these is created per App
/// and inserted into BOTH that App's main world and its render sub-app, so the
/// stamper and the reveal read the same `Arc` and a sibling App holds a
/// different one. Two Apps cannot see each other's preparation because they
/// never share the set.
///
/// ⚠ The global ledger still MIRRORS these stamps, and must keep doing so: the
/// `[image-gpu]` lines, the insert→gpu timings and the census all read it. What
/// it may no longer do is DECIDE whether a cover lifts.
#[derive(Resource, Clone, Default, Debug)]
pub struct AppGpuPreparedImages(Arc<Mutex<HashSet<UntypedAssetId>>>);

impl AppGpuPreparedImages {
    /// Record that this App's render world has a GPU copy of `id`.
    pub fn mark_prepared(&self, id: UntypedAssetId) {
        if let Ok(mut set) = self.0.lock() {
            set.insert(id);
        }
    }

    /// Has THIS App prepared `id`?
    pub fn is_prepared(&self, id: UntypedAssetId) -> bool {
        self.0.lock().is_ok_and(|set| set.contains(&id))
    }

    pub fn prepared_count(&self) -> usize {
        self.0.lock().map(|set| set.len()).unwrap_or(0)
    }

    /// The reveal-readiness term: this App draws, and has not yet prepared `id`.
    ///
    /// ⭐ POSITIVE PROOF, exactly as the ledger's version was: an id with no
    /// stamp yet is OWED, not assumed ready. A headless App answers `false`
    /// because it never prepares anything and nothing may wait on it.
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
    /// ⛔ NATIVE-ONLY, because it carries an `Instant` and `Instant` is
    /// native-only in this module. Leaving it ungated broke the WASM build
    /// outright: `image_stages` is compiled whenever the `bevy` feature is on,
    /// which the web composition turns on, so the field's type named a type that
    /// was not in scope there. Found by review 2026-09-02 and reproduced with
    /// `cargo check --target wasm32-unknown-unknown`.
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
        // ⛔⛔ REMEMBER THAT A COVER EXISTED AT ALL. `capture_scene` puts the app
        // in `playing` from boot on every road it has — measured 2026-09-02:
        // `[game-mode] 0.633s initial playing`, before the room even loads — so
        // on that road EVERY first draw is trivially "during gameplay" and a POP
        // readout would report eighteen findings where there is no cover to have
        // failed. This is the fact that separates "the cover did not cover this"
        // from "this composition has no cover", and a readout that skips it is
        // measuring its own harness.
        if live == Some(false) {
            self.saw_covered_frame = true;
        }
        self.gameplay_live = live;
    }

    /// Has this process ever observed a frame where gameplay was NOT live?
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

    /// The render world's stamp exists in this process (the plugin found a
    /// render app to install into).

    /// READINESS TERM: a render world exists and has NOT yet been seen to
    /// prepare `id`. `false` whenever no render world stamps stage 3 — a
    /// headless run never waits on a GPU it does not have.
    ///
    /// ⛔ THE CALLER SUPPLIES THE RENDER-WORLD FACT, from ITS OWN App's
    /// [`RenderWorldPresent`]. The ledger is process-global and cannot know
    /// which App is asking.
    ///
    /// ⭐ POSITIVE PROOF, not "not known to be waiting". The insertion stamp
    /// comes from the main world's `Last` (an `AssetEvent::Added` reader) while
    /// room readiness polls in `Update`, so on the frame an image lands the
    /// poll runs BEFORE the row exists; a term that read the awaiting list
    /// called that image ready, latched the reveal, and only then did `Last`
    /// stamp it and the render world (possibly) defer its upload under a
    /// byte-per-frame budget — the exact frame this term exists to keep under
    /// the cover. So the question is "has the GPU stamp landed", and an id with
    /// no row yet is owed like any other. The cost is one frame of cover per
    /// image on an unpaced upload, which is cover time.
    ///
    /// A room whose reveal waits on this converts the upload of its cast from a
    /// frame after the cover lifts into cover time: the pixels were paid for
    /// either way, and under a byte-per-frame budget they pace while the cover
    /// still holds.
    pub fn is_awaiting_gpu(&self, id: UntypedAssetId, render_world: RenderWorldPresent) -> bool {
        render_world.is_present() && !self.is_gpu_prepared(id)
    }

    /// The render world has stamped `id` prepared (stage 3). The proof the
    /// readiness term above asks for.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn is_gpu_prepared(&self, id: UntypedAssetId) -> bool {
        self.rows
            .get(&id)
            .is_some_and(|row| row.gpu_prepared_at.is_some())
    }

    /// No stage-3 stamp exists on the web (the stamper is native-only, and no
    /// App inserts [`RenderWorldPresent(true)`](RenderWorldPresent) there), so
    /// nothing is ever proven — and nothing is ever owed, because the term
    /// above is off.
    #[cfg(target_arch = "wasm32")]
    pub fn is_gpu_prepared(&self, _id: UntypedAssetId) -> bool {
        false
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

    /// This image was extracted for drawing — the FOURTH stage, and the first
    /// one that is about USE rather than arrival.
    ///
    /// ⛔⛔ FIRST WRITE WINS, and that is not tidiness. Extraction runs every
    /// frame for every visible sprite, so a stamp that overwrote would be a
    /// per-frame write on the whole visible set and the ledger's own cost would
    /// show up in what it measures. The question is *"when was this first
    /// drawn"*, which is asked once and answered forever.
    ///
    /// Returns the elapsed demand→draw when this call is the one that stamped
    /// it and the demand is known, so a caller can report the wait without
    /// re-reading the row; `None` on every later frame, which is also how a
    /// caller knows not to print.
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

    /// What is resident and NEVER DRAWN, by the road that demanded it.
    ///
    /// ⭐⭐ THE TOTAL CANNOT ANSWER THE QUESTION IT RAISES. A run that reports
    /// "23.2 MP never drawn" immediately invites *"whose?"* — and the owners are
    /// exactly the buckets [`Self::resident_by_road`] already names, so an
    /// eviction conversation can start from an owner instead of a number. It is
    /// what decides whether the FX set's 9.6 MP is an effect vocabulary or a
    /// preload (asset open work 2's third row).
    ///
    /// ⛔ SAME RENDER-WORLD CAVEAT AS [`Self::resident_never_drawn`]: without one
    /// this returns every resident image under its road and means "nobody could
    /// have drawn anything". The caller must consult the ASKING App's
    /// [`RenderWorldPresent`] before printing it as a finding — the ledger is
    /// process-global and cannot know which App is reading it.
    ///
    /// ⛔⛔ AND [`ROAD_PROCEDURAL`] IS NEVER A FINDING IN THIS READOUT, whatever
    /// its megapixels say. The stage is stamped from `ExtractedSprites`, and a
    /// render target, a shader input or a material texture is never a sprite —
    /// it is written to or sampled, not extracted. So those rows are
    /// PERMANENTLY "never drawn" by construction, and a reader chasing the 4-6
    /// MP this bucket reports in a hall capture is chasing the instrument rather
    /// than the assets. Only the file-backed roads answer a residency question
    /// here.
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
    /// ⛔⛔ ONLY MEANINGFUL WITH A RENDER WORLD, and the caller must say so.
    /// Without one nothing is ever extracted, so this returns EVERY resident
    /// image and means "nobody could have drawn anything" — not "these were
    /// decoded for nobody". [`RenderWorldPresent`] — the ASKING App's, not the
    /// process's — separates the two readings, and a readout that prints this
    /// without consulting it is accusing a headless run of waste it cannot
    /// commit.
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

    /// Every image inserted and not yet removed, in id order — the per-row
    /// form of [`Self::resident_by_road`], for a census that wants the PATHS.
    ///
    /// ⛔ Native-only for the same reason as [`Self::demand_by_path`]: residency
    /// here is defined by `inserted_at`, a timestamp this module does not keep
    /// on WASM.
    #[cfg(not(target_arch = "wasm32"))]
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

    /// The readiness term is POSITIVE proof of the GPU stamp while a render
    /// world is present — and NOTHING without a render world.
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
        // ⛔⛔ THE PER-APP POINT, and the whole reason this argument exists: ONE
        // ledger answers BOTH Apps in the same breath, on the same id. While the
        // fact lived on the ledger these two calls could not disagree, so a
        // rendering sibling made a headless App wait for a GPU nothing would
        // ever stamp.
        assert_ne!(
            ledger.is_awaiting_gpu(id(10), headless),
            ledger.is_awaiting_gpu(id(10), rendering),
            "the same ledger must answer a headless App and a rendering App differently"
        );
        // ⭐ THE RACE: readiness polls in `Update`, the insertion is stamped in
        // `Last`. An id the ledger has not seen yet is OWED, not ready — the
        // old "awaiting list contains it" reading called it ready here.
        assert!(
            ledger.is_awaiting_gpu(id(11), rendering),
            "not yet stamped inserted: the GPU has not proven anything, so owed"
        );
        ledger.gpu_prepared(id(10), t0 + Duration::from_millis(5));
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

    /// ⛔⛔ FIRST WRITE WINS, AND THE SECOND CALL MUST SAY NOTHING.
    ///
    /// The fourth stage is stamped from the render world's extraction, which
    /// runs EVERY FRAME for EVERY VISIBLE SPRITE. A stamp that overwrote would
    /// be a per-frame write on the whole visible set, and the ledger's own cost
    /// would land in what it measures — so the rule is not tidiness, it is the
    /// reason the stage can exist at all.
    ///
    /// ⛔ AND THE RETURN IS THE TELL. A caller prints the demand→draw wait when
    /// it gets one; a `None` on the second frame is how it knows not to print
    /// the same line sixty times a second.
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

    /// ⭐⭐ A FIRST DRAW WHILE GAMEPLAY IS LIVE IS A POP, and that is the fact
    /// the whole hitch lane is about: a cover exists so a room's art arrives
    /// before anyone can see the room. This pins that the flag follows the
    /// ledger's live state at the DRAW rather than at the insert — the two
    /// answer different questions and can disagree in both directions.
    #[test]
    fn a_first_draw_while_gameplay_is_live_is_recorded_as_one() {
        let mut ledger = ImageStageLedger::default();
        let t0 = Instant::now();

        // Decoded under a cover, drawn under it too: not a pop.
        ledger.set_gameplay_live(Some(false));
        ledger.demand(id(60), "character-sheet", "covered.png".into(), t0);
        ledger.inserted(id(60), 2.0, Some(false), None, t0);
        ledger.first_drawn(id(60), t0 + Duration::from_millis(20));
        assert_eq!(
            ledger.get(id(60)).and_then(|r| r.live_at_first_draw),
            Some(false),
        );

        // Decoded under the cover and first drawn AFTER it lifted: a pop, and
        // `live_at_insert` cannot see it — which is why this field exists.
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

    /// ⛔⛔ AND A COMPOSITION THAT NEVER COVERS ANYTHING CANNOT HAVE A POP.
    ///
    /// `capture_scene` boots straight into `playing` on every road it has, so
    /// every first draw there is trivially "during gameplay" — a POP readout
    /// would have reported eighteen findings in one hall shot, all of them the
    /// harness. This is the fact that separates "the cover did not cover this"
    /// from "nothing here has a cover", and it is what the readout consults
    /// before it says the word.
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

    /// ⛔⛔ AND THE TOTAL CANNOT SAY WHOSE. A run reporting "23.2 MP never
    /// drawn" invites exactly one question, and the roads answer it: an eviction
    /// conversation starts from an owner, not from a number. This pins that the
    /// split adds up to the total and that a drawn image leaves its OWN bucket
    /// rather than the whole road.
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

    /// ⛔⛔ NEVER-DRAWN IS NOT A FINDING WITHOUT A RENDER WORLD, and the list
    /// cannot tell the caller that — only the asking App's
    /// [`RenderWorldPresent`] can. This pins the shape a readout has to
    /// respect: with nothing extracted,
    /// EVERY resident image is "never drawn", which on a headless road means
    /// nobody could have drawn anything rather than that the pixels were wasted.
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod app_local_gpu_readiness {
    use super::*;

    /// The same shape the module's other tests use — `bevy::image::Image` and a
    /// bare `uuid` crate are neither of them reachable here, and inventing them
    /// is how this test first failed to compile.
    fn id(n: u128) -> UntypedAssetId {
        bevy::asset::AssetId::<bevy::asset::LoadedUntypedAsset>::Uuid {
            uuid: bevy::asset::uuid::Uuid::from_u128(n),
        }
        .untyped()
    }

    /// ⛔⛔ THE ACCEPTANCE CONDITION. Two rendering Apps that share an asset id —
    /// which this repository has measured happening, because ids are App-LOCAL —
    /// must not settle each other's reveal.
    ///
    /// Before `AppGpuPreparedImages` this was unprovable: the only readiness
    /// authority was a process-global ledger keyed by that id, so "App A prepared
    /// id 7" and "App B prepared id 7" were the same sentence.
    #[test]
    fn preparation_in_one_app_does_not_settle_another_that_shares_the_id() {
        let a = AppGpuPreparedImages::default();
        let b = AppGpuPreparedImages::default();
        let rendering = RenderWorldPresent(true);
        let shared = id(7);

        // Non-vacuity: both are waiting on the SAME id before anything happens.
        assert!(a.is_awaiting_gpu(shared, rendering));
        assert!(b.is_awaiting_gpu(shared, rendering));

        a.mark_prepared(shared);

        assert!(!a.is_awaiting_gpu(shared, rendering), "A prepared it, so A is settled");
        assert!(
            b.is_awaiting_gpu(shared, rendering),
            "B must STILL be waiting: A's render world uploaded A's image, and the \
             fact that the two Apps happen to number it the same is not evidence \
             about B. This is the assertion the process-global ledger could not make."
        );
        assert_eq!(a.prepared_count(), 1);
        assert_eq!(b.prepared_count(), 0);
    }

    /// A headless App never prepares anything, so nothing may wait on it.
    #[test]
    fn a_headless_app_is_never_awaiting() {
        let headless = AppGpuPreparedImages::default();
        assert!(!headless.is_awaiting_gpu(id(7), RenderWorldPresent(false)));
        assert!(headless.is_awaiting_gpu(id(7), RenderWorldPresent(true)));
    }

    /// The set is shared through its `Arc`, which is how the render sub-app's
    /// write reaches the main world's read inside ONE App.
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
