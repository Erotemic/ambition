//! A RUNNING GAME PLAYS AN EDIT TO ITS SPRITE SHEETS, with no Rust build and
//! no restart.
//!
//! A build that reads its sprite tree off disk (no `embedded_sprites`) adds
//! [`SpriteSheetReloadPlugin`]. The plugin looks at each sheet manifest
//! (`*_spritesheet.ron`), each body rig (`*_body_rig.ron`), each part flipbook
//! (`*_parts.ron`) and each page image of those sheets and flipbooks. One look at
//! the published tree (a stat of each of those files) costs 0.2 to 0.4 s in a
//! debug build (measured 2026-10-16), so a background thread looks when the
//! operating system reports a change in a sprite folder (`notify`), and a
//! frame only takes the changes it found. Where no report is possible it
//! looks once a second.
//!
//! The rules of the road:
//!
//! - A changed page image changes only pixels, so it is loaded again at once
//!   (`AssetServer::reload`), also inside a session. Bevy loads again only an
//!   image that is loaded.
//! - A changed sheet manifest or body rig is read whole and parsed by the
//!   look that saw it (about a second in a debug build, off the frame). A
//!   table that changes only where frames are drawn from (pages, rects; a
//!   repack) is published at once, also inside a session: its new pages are
//!   loaded at once, so its old rects would cut the new pixels. A table that
//!   changes records simulation or a cast reads (body sizes, hurtboxes,
//!   attack volumes, a boss's size, hurt parts, landmarks, portrait names;
//!   `PreparedSheetTable::changes_mechanics_of`) waits for the end of the
//!   session, as a restated content schema does. Nothing is published while
//!   a route prepares a session. A publication is one generation
//!   (`baked_sheet_rons::publish_prepared_sheet_table`): the `SheetRegistry`
//!   resource is stated again, each cast made from the sheets is stated again
//!   where the mechanics changed ([`SheetDerivedRestaters`]), each resident
//!   character realization is stale (a worn one is made again in place and
//!   its body binds again; the others are retired), each prop sheet and
//!   effect sheet made from a changed sheet is made again, and the pages of each changed sheet
//!   are loaded again.
//! - A table with a file that does not parse is refused, with each file
//!   named, and the running table stays.
//!
//! Portrait manifests and sprite pack catalogs are on it too. Not on this road
//! yet: a prop row of the asset catalog (see
//! `docs/planning/engine/content-generation-and-reload.md`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use ambition_sprite_sheet::baked_sheet_rons as sheet_table;
use bevy::prelude::*;


/// Time between two looks where the operating system reports no changes, and
/// between two checks of the background thread that its watch still lives.
const LOOK_EVERY: Duration = Duration::from_secs(1);

/// Adds the sprite sheet watch where the build reads its sprite tree off
/// disk. In a build that embeds its sprites it adds nothing.
#[derive(Default)]
pub struct SpriteSheetReloadPlugin {
    /// The folder whose `assets/` holds the tree to watch; `None` for the
    /// published tree the build reads.
    pub owner: Option<PathBuf>,
    /// No background thread: the watch looks only when
    /// [`SpriteSheetWatch::look_now`] is called. For a test that steps the
    /// watch itself.
    pub manual: bool,
}

impl Plugin for SpriteSheetReloadPlugin {
    fn build(&self, app: &mut App) {
        let owner = self.owner.clone().or_else(|| sheet_table::published_tree_owner().map(Path::to_path_buf));
        let Some(owner) = owner else {
            return;
        };
        let watch = SpriteSheetWatch::new(owner);
        if !self.manual {
            watch.look_in_the_background();
        }
        app.insert_resource(watch);
        app.add_systems(Update, (take_what_the_watch_found, publish_the_staged_sheets).chain());
    }
}

/// What a composition derived from the sheet records when it prepared, and
/// how to state it again: a cast's body scale, inset hurtboxes and rig are
/// read from its sheets once, when its definitions are made. Each runs at the
/// publication of a sheet generation, before a frame can build a body, so no
/// body is built from the new records and a scale of the old ones.
#[derive(Resource, Default, Clone)]
pub struct SheetDerivedRestaters(Vec<(String, Arc<dyn Fn(&mut World) + Send + Sync>)>);

impl SheetDerivedRestaters {
    /// The names of the restaters, in the order they run.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(name, _)| name.as_str())
    }
}

/// Run `restate` at each publication of a sheet generation (see
/// [`SheetDerivedRestaters`]). `name` says what it states again.
pub fn restate_with_the_sheets(app: &mut App, name: impl Into<String>, restate: impl Fn(&mut World) + Send + Sync + 'static) {
    app.world_mut()
        .get_resource_or_insert_with(SheetDerivedRestaters::default)
        .0
        .push((name.into(), Arc::new(restate)));
}

/// What one look found.
#[derive(Default)]
struct Found {
    /// The keys of the changed, added or removed manifests.
    manifests: BTreeSet<String>,
    /// The whole table, read again and parsed, when a manifest changed. The
    /// parse costs about a second in a debug build, so the look does it.
    table: Option<Result<sheet_table::PreparedSheetTable, sheet_table::SheetTableRefused>>,
    /// Whether the table changes a fact that simulation or a cast reads
    /// (`PreparedSheetTable::changes_mechanics_of`). The digest of the
    /// records is a cost in proportion to the table, so the look finds it.
    changes_mechanics: bool,
    /// The changed page images.
    pages: Vec<PathBuf>,
}

/// The files of one tree, with the time each changed when it was last seen.
struct Scanner {
    owner: PathBuf,
    /// The first look takes the times and finds nothing. It is not done when
    /// the watch is made: an App is built many times in a test run, and a
    /// look costs about 100 ms.
    seen_once: bool,
    /// Looks taken, the first included.
    looks: u32,
    manifests: Vec<(String, PathBuf, Option<SystemTime>)>,
    pages: Vec<(PathBuf, Option<SystemTime>)>,
}

impl Scanner {
    fn new(owner: PathBuf) -> Self {
        Self { owner, seen_once: false, looks: 0, manifests: Vec::new(), pages: Vec::new() }
    }

    fn look(&mut self) -> Found {
        self.looks += 1;
        if !self.seen_once {
            self.seen_once = true;
            self.manifests = stamped_manifests(&self.owner);
            self.stamp_pages();
            return Found::default();
        }
        let mut found = Found::default();
        for (page, seen) in &mut self.pages {
            let now = modified(page);
            if now != *seen {
                *seen = now;
                found.pages.push(page.clone());
            }
        }
        // A file can be added or removed, so the list is read again each look.
        let now = stamped_manifests(&self.owner);
        found.manifests = now
            .iter()
            .filter(|(key, _, seen)| !self.manifests.iter().any(|(was, _, before)| was == key && before == seen))
            .map(|(key, ..)| key.clone())
            .collect();
        found.manifests.extend(
            self.manifests.iter().filter(|(was, ..)| !now.iter().any(|(key, ..)| key == was)).map(|(was, ..)| was.clone()),
        );
        self.manifests = now;
        if !found.manifests.is_empty() {
            let table = sheet_table::prepare_sheet_table(sheet_table::read_sheet_table_under(&self.owner));
            found.changes_mechanics = table.as_ref().is_ok_and(|table| table.changes_mechanics_of(sheet_table::current()));
            found.table = Some(table);
        }
        found
    }

    /// The pages of the manifests `keys`, under the current sheet generation.
    fn pages_of(&self, keys: &BTreeSet<String>) -> Vec<PathBuf> {
        pages_of(&self.manifests.iter().filter(|(key, ..)| keys.contains(key)).cloned().collect::<Vec<_>>())
    }

    /// Read the page list again after a publication: a sheet can name other
    /// pages now.
    fn stamp_pages(&mut self) {
        self.pages = stamped(pages_of(&self.manifests));
    }
}

/// The watch over one sprite tree.
#[derive(Resource)]
pub struct SpriteSheetWatch {
    owner: PathBuf,
    scanner: Arc<Mutex<Scanner>>,
    /// What the looks found and no frame took yet.
    found: Arc<Mutex<Vec<Found>>>,
    /// The keys of the changed manifests, waiting for their publication.
    staged: BTreeSet<String>,
    /// The table to publish: at once where it changes only where frames are
    /// drawn from, else at the end of the session.
    candidate: Option<sheet_table::PreparedSheetTable>,
    /// Whether `candidate` changes a fact that simulation or a cast reads.
    candidate_changes_mechanics: bool,
    /// Sheet generations this watch published.
    pub published: u32,
    /// Page images this watch loaded again.
    pub reloaded_pages: u32,
    /// Why the last changed table was refused, while no later one was
    /// published.
    pub refused: Option<String>,
}

impl SpriteSheetWatch {
    /// A watch over the tree whose `assets/` folder is under `owner`.
    pub fn new(owner: PathBuf) -> Self {
        Self {
            scanner: Arc::new(Mutex::new(Scanner::new(owner.clone()))),
            owner,
            found: Arc::default(),
            staged: BTreeSet::new(),
            candidate: None,
            candidate_changes_mechanics: false,
            published: 0,
            reloaded_pages: 0,
            refused: None,
        }
    }

    /// Look at the files now, on this thread. The next update takes what the
    /// look found.
    pub fn look_now(&self) {
        let found = self.scanner.lock().unwrap_or_else(PoisonError::into_inner).look();
        self.found.lock().unwrap_or_else(PoisonError::into_inner).push(found);
    }

    /// A thread that looks when the operating system reports a change in a
    /// sprite folder, else once each [`LOOK_EVERY`] where no report is
    /// possible. It ends when the watch is dropped.
    fn look_in_the_background(&self) {
        let scanner = Arc::downgrade(&self.scanner);
        let found = Arc::downgrade(&self.found);
        let reports = ChangeReports::of(&self.owner);
        let spawned = std::thread::Builder::new().name("sprite-sheet-watch".into()).spawn(move || {
            // The first look takes the baseline: it costs a full look once.
            let mut changed = true;
            loop {
                let (Some(scanner), Some(found)) = (scanner.upgrade(), found.upgrade()) else {
                    return;
                };
                if changed {
                    let look = scanner.lock().unwrap_or_else(PoisonError::into_inner).look();
                    if !look.manifests.is_empty() || !look.pages.is_empty() {
                        found.lock().unwrap_or_else(PoisonError::into_inner).push(look);
                    }
                }
                drop((scanner, found));
                changed = reports.wait(LOOK_EVERY);
            }
        });
        if let Err(error) = spawned {
            warn!(target: "ambition_platformer2d::sprites", "no sprite sheet watch: {error}");
        }
    }

    /// The looks at the tree so far, the first included.
    pub fn looks(&self) -> u32 {
        self.scanner.lock().unwrap_or_else(PoisonError::into_inner).looks
    }

    /// A changed sheet table waits for its publication.
    pub fn is_staged(&self) -> bool {
        self.candidate.is_some()
    }

    /// The asset path of `page`: its path under the tree's `assets/` folder.
    fn asset_path(&self, page: &Path) -> Option<String> {
        page.strip_prefix(self.owner.join("assets")).ok().map(|path| path.to_string_lossy().replace('\\', "/"))
    }
}

/// What the operating system reports about the sprite folders: a file under
/// them changed. Without it a look costs a stat of each file of the tree
/// (about 0.2 to 0.4 s in a debug build) each second.
struct ChangeReports {
    #[cfg(not(target_arch = "wasm32"))]
    reports: Option<(notify::RecommendedWatcher, std::sync::mpsc::Receiver<notify::Result<notify::Event>>)>,
}

impl ChangeReports {
    #[cfg(not(target_arch = "wasm32"))]
    fn of(owner: &Path) -> Self {
        use notify::Watcher;
        let (sender, receiver) = std::sync::mpsc::channel();
        let watcher = notify::recommended_watcher(sender).and_then(|mut watcher| {
            let dirs = sheet_table::sprite_dirs_under(owner).into_iter().chain([sheet_table::pack_catalog_dir_under(owner)]);
            for dir in dirs.filter(|dir| dir.is_dir()) {
                watcher.watch(&dir, notify::RecursiveMode::Recursive)?;
            }
            Ok(watcher)
        });
        match watcher {
            Ok(watcher) => Self { reports: Some((watcher, receiver)) },
            Err(error) => {
                warn!(target: "ambition_platformer2d::sprites", "no change reports for the sprite tree ({error}); it is looked at each {LOOK_EVERY:?}");
                Self { reports: None }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn of(_owner: &Path) -> Self {
        Self {}
    }

    /// Wait at most `longest` for a report of a change to a file a sheet
    /// generation or a page is read from. True when one came (and the reports
    /// that follow it within a short time), or when no reports are possible.
    fn wait(&self, longest: Duration) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some((_, receiver)) = &self.reports {
            let relevant = |report: notify::Result<notify::Event>| {
                // An error can be a lost report: look.
                report.map_or(true, |event| event.paths.iter().any(|path| watched_file(path)))
            };
            let Ok(report) = receiver.recv_timeout(longest) else {
                return false;
            };
            let mut changed = relevant(report);
            // A save is several reports, and a publish writes many files:
            // one look for all of them.
            while let Ok(report) = receiver.recv_timeout(SETTLE) {
                changed |= relevant(report);
            }
            return changed;
        }
        std::thread::sleep(longest);
        true
    }
}

/// The time a burst of change reports is given to end before the look.
const SETTLE: Duration = Duration::from_millis(150);

/// A file a sheet generation or a page image is read from.
fn watched_file(path: &Path) -> bool {
    use ambition_sprite_sheet::baked_part_flipbooks::PART_FLIPBOOK_SUFFIX;
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.ends_with("_spritesheet.ron")
        || name.ends_with(sheet_table::BODY_RIG_SUFFIX)
        || name.ends_with(PART_FLIPBOOK_SUFFIX)
        || name.ends_with("_portraits.ron")
        || name == "ultrapack.json"
        || name.ends_with(".png")
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|metadata| metadata.modified()).ok()
}

/// Each sheet manifest, body rig and part flipbook, the files a sheet
/// generation is read from. A rig or a flipbook is keyed by its file name, so
/// no such key is a manifest key.
fn stamped_manifests(owner: &Path) -> Vec<(String, PathBuf, Option<SystemTime>)> {
    use ambition_sprite_sheet::baked_part_flipbooks::{part_flipbook_files_under, PART_FLIPBOOK_SUFFIX};
    let rigs = sheet_table::body_rig_files_under(owner)
        .into_iter()
        .map(|(target, path)| (format!("{target}{}", sheet_table::BODY_RIG_SUFFIX), path));
    let flipbooks = part_flipbook_files_under(owner).into_iter().map(|(key, path)| (format!("{key}{PART_FLIPBOOK_SUFFIX}"), path));
    // A portrait is keyed by its path under `assets/` and a catalog by its
    // tier's: neither is a manifest key.
    let catalogs = sheet_table::pack_catalog_files_under(owner).into_iter().map(|(tier, path)| (format!("sprite_packs/{tier}/ultrapack.json"), path));
    sheet_table::sheet_files_under(owner)
        .into_iter()
        .chain(rigs)
        .chain(flipbooks)
        .chain(sheet_table::portrait_files_under(owner))
        .chain(catalogs)
        .map(|(key, path)| {
            let seen = modified(&path);
            (key, path, seen)
        })
        .collect()
}

fn stamped(paths: Vec<PathBuf>) -> Vec<(PathBuf, Option<SystemTime>)> {
    paths
        .into_iter()
        .map(|path| {
            let seen = modified(&path);
            (path, seen)
        })
        .collect()
}

/// Each page image the current sheet generation names for these manifests:
/// a page file is beside its manifest.
fn pages_of(manifests: &[(String, PathBuf, Option<SystemTime>)]) -> Vec<PathBuf> {
    use ambition_sprite_sheet::baked_part_flipbooks::PART_FLIPBOOK_SUFFIX;
    // Each folder is listed once: about 160 flipbooks share a few folders.
    let mut listed: std::collections::HashMap<PathBuf, Vec<PathBuf>> = std::collections::HashMap::new();
    let registry = sheet_table::current().registry();
    let mut pages = BTreeSet::new();
    let portraits = sheet_table::current().portraits();
    for (key, path, _) in manifests {
        let Some(dir) = path.parent() else {
            continue;
        };
        if let Some(portrait) = portraits.get(key) {
            pages.insert(dir.join(&portrait.image));
            continue;
        }
        // A flipbook's atlas pages are beside it, named after it
        // (`<target>_parts.png`, `<target>_parts_1.png`), and a pack tier's
        // pages beside its catalog (`ultrapack_<n>.png`).
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        let prefix = match name.strip_suffix(PART_FLIPBOOK_SUFFIX) {
            Some(target) => Some(format!("{target}_parts")),
            None => (name == "ultrapack.json").then(|| "ultrapack".to_owned()),
        };
        if let Some(prefix) = prefix {
            let files = listed
                .entry(dir.to_path_buf())
                .or_insert_with(|| std::fs::read_dir(dir).into_iter().flatten().flatten().map(|entry| entry.path()).collect());
            let pages_here = files.iter().cloned().filter(|page| {
                page.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".png"))
            });
            pages.extend(pages_here);
            continue;
        }
        let Some(record) = registry.get(key) else {
            continue;
        };
        for page in 0..record.page_count().max(1) {
            pages.insert(dir.join(record.page_image(page)));
        }
    }
    pages.into_iter().collect()
}

fn reload(asset_server: Option<&AssetServer>, paths: Vec<String>) -> u32 {
    let Some(asset_server) = asset_server else {
        return 0;
    };
    for path in &paths {
        asset_server.reload(path.clone());
    }
    paths.len() as u32
}

/// Load each changed page image again at once, and stage each changed
/// manifest for the end of the session.
fn take_what_the_watch_found(watch: Option<ResMut<SpriteSheetWatch>>, asset_server: Option<Res<AssetServer>>) {
    let Some(mut watch) = watch else {
        return;
    };
    let found = std::mem::take(&mut *watch.found.lock().unwrap_or_else(PoisonError::into_inner));
    for look in found {
        let paths: Vec<String> = look.pages.iter().filter_map(|page| watch.asset_path(page)).collect();
        watch.reloaded_pages += reload(asset_server.as_deref(), paths);
        match look.table {
            None => {}
            Some(Ok(table)) => {
                let when = if look.changes_mechanics { "from the next session" } else { "at once (only where frames are drawn from changed)" };
                info!(target: "ambition_platformer2d::sprites", "sprite sheets changed: {:?}; they are played {when}", look.manifests);
                watch.staged.extend(look.manifests);
                watch.candidate = Some(table);
                watch.candidate_changes_mechanics = look.changes_mechanics;
                watch.refused = None;
            }
            // The table is read whole, so a broken one replaces any change
            // staged before it.
            Some(Err(refused)) => {
                let why = format!("the sprite sheets do not parse; the running sheets stay:\n{refused}");
                error!(target: "ambition_platformer2d::sprites", "{why}");
                watch.staged.clear();
                watch.candidate = None;
                watch.refused = Some(why);
            }
        }
    }
}

/// Publish a staged sheet table where no route prepares a session: a table
/// that changes only where frames are drawn from at once, and any other table
/// where no session is live.
fn publish_the_staged_sheets(world: &mut World) {
    let Some(changes_mechanics) = world.get_resource::<SpriteSheetWatch>().filter(|watch| watch.is_staged()).map(|watch| watch.candidate_changes_mechanics) else {
        return;
    };
    let in_a_session = world.get_resource::<crate::actor::ActiveSessionScope>().is_some_and(|scope| scope.current().is_some());
    let preparing = world.get_resource::<crate::game_shell::ShellRouter>().is_some_and(|router| router.pending.is_some());
    if (in_a_session && changes_mechanics) || preparing {
        return;
    }
    let (table, staged) = {
        let mut watch = world.resource_mut::<SpriteSheetWatch>();
        let Some(table) = watch.candidate.take() else {
            return;
        };
        (table, std::mem::take(&mut watch.staged))
    };
    let generation = sheet_table::publish_prepared_sheet_table(table);
    info!(target: "ambition_platformer2d::sprites", "sprite sheet generation {generation} published; changed: {staged:?}");
    world.insert_resource(ambition_sprite_sheet::baked_sheet_registry());
    if world.contains_resource::<ambition_sprite_sheet::portrait::PortraitSheetRegistry>() {
        world.insert_resource(ambition_sprite_sheet::portrait::baked_portrait_registry());
    }
    // A restater reads only facts `changes_mechanics_of` compares, so a
    // table that changes none of them leaves each cast as it is.
    let restaters = if changes_mechanics { world.get_resource::<SheetDerivedRestaters>().cloned().unwrap_or_default() } else { SheetDerivedRestaters::default() };
    for (name, restate) in &restaters.0 {
        info!(target: "ambition_platformer2d::sprites", "restating `{name}` from sheet generation {generation}");
        restate(world);
    }
    crate::actors::assets::game_assets::reload_prop_sheets_of_targets(world, &staged);
    crate::actors::assets::game_assets::reload_fx_sheets_of_targets(world, &staged);
    // Each resident realization is now of an older generation: the residency
    // convergence makes a worn one again in place (its body keeps drawing
    // until the new one is ready, then binds again) and retires the others
    // (`converge_character_residency_to_active_quality`).
    let paths: Vec<String> = {
        let watch = world.resource::<SpriteSheetWatch>();
        let mut scanner = watch.scanner.lock().unwrap_or_else(PoisonError::into_inner);
        let changed = scanner.pages_of(&staged);
        scanner.stamp_pages();
        changed.iter().filter_map(|page| watch.asset_path(page)).collect()
    };
    let reloaded = reload(world.get_resource::<AssetServer>(), paths);
    let mut watch = world.resource_mut::<SpriteSheetWatch>();
    watch.published += 1;
    watch.reloaded_pages += reloaded;
}
