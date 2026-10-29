//! Reload sprite metadata and images from an atomic publication selection.
//!
//! A publication names immutable files. Sheet generations keep their image
//! addresses while a new selection is prepared. A change to simulation data
//! waits for the session boundary. A change to presentation data can apply at
//! once. Invalid metadata keeps the current generation.
//!
//! Logical image handles, such as prop icons, reload when their selected hashes
//! change. Sheet handles name immutable bundle members and remain valid.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use ambition_sprite_sheet::baked_sheet_rons as sheet_table;
use ambition_asset_manager::content_library::{ContentLibraries, LibrarySnapshot};
use bevy::prelude::*;


/// Time between two looks where the operating system reports no changes, and
/// between two checks of the background thread that its watch still lives.
const LOOK_EVERY: Duration = Duration::from_secs(1);

/// Adds the sprite sheet watch where the build reads its sprite tree off
/// disk. In a build that embeds its sprites it adds nothing.
#[derive(Default)]
pub struct SpriteSheetReloadPlugin {
    /// No background thread: the watch looks only when
    /// [`SpriteSheetWatch::look_now`] is called. For a test that steps the
    /// watch itself.
    pub manual: bool,
}

impl Plugin for SpriteSheetReloadPlugin {
    fn build(&self, app: &mut App) {
        let manual = self.manual;
        app.init_resource::<ambition_sprite_sheet::SpriteContent>();
        app.add_systems(Startup, move |world: &mut World| {
            let libraries = world.get_resource::<ContentLibraries>().cloned().unwrap_or_default();
            if libraries.iter().all(|library| library.is_embedded()) { return; }
            let watch = SpriteSheetWatch::new(world.resource::<ambition_sprite_sheet::SpriteContent>(), libraries);
            if !manual { watch.look_in_the_background(); }
            world.insert_resource(watch);
        });
        app.add_systems(Update, (
            take_what_the_watch_found,
            publish_the_staged_sheets.run_if(|watch: Option<Res<SpriteSheetWatch>>| watch.is_some_and(|watch| watch.is_staged())),
        ).chain());
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
    logical_images: Vec<String>,
    snapshot: Option<LibrarySnapshot>,
}

/// The files of one tree, with the time each changed when it was last seen.
struct Scanner {
    libraries: ContentLibraries,
    content: ambition_sprite_sheet::SpriteContent,
    /// The first look takes the times and finds nothing. It is not done when
    /// the watch is made: an App is built many times in a test run, and a
    /// look costs about 100 ms.
    seen_once: bool,
    /// Looks taken, the first included.
    looks: u32,
    manifests: Vec<(String, PathBuf, Option<SystemTime>)>,
    images: std::collections::BTreeMap<String, String>,
    selections: std::collections::BTreeMap<String, Result<Option<String>, String>>,
}

impl Scanner {
    fn new(libraries: ContentLibraries, content: ambition_sprite_sheet::SpriteContent) -> Self {
        Self { libraries, content, seen_once: false, looks: 0, manifests: Vec::new(), images: Default::default(), selections: Default::default() }
    }

    fn look(&mut self) -> Found {
        let sprite_content = &self.content;
        self.looks += 1;
        let mut files = Vec::new();
        let mut images = std::collections::BTreeMap::new();
        let mut selections = std::collections::BTreeMap::new();
        for library in self.libraries.iter().filter(|library| !library.is_embedded()) {
            if let Some(root) = library.publication_root() {
                files.push((format!("content://{}", library.manifest.id), root.join("published.json")));
            }
            let selected = library.selection();
            selections.insert(library.manifest.id.clone(), selected.as_ref()
                .map(|selection| selection.as_ref().and_then(|selection| selection.generation.clone()))
                .map_err(Clone::clone));
            if let Ok(Some(selected)) = &selected {
                images.extend(selected.files.iter()
                    .map(|(name, member)| (format!("content://{}/{name}", library.manifest.id), member.sha256.clone())));
            }
            if let Ok(selected) = library.files() {
                files.extend(selected.into_iter().filter(|(_, file)| file.generation.is_none())
                    .filter_map(|(logical, file)| file.native_path().map(|path| (format!("content://{}/{logical}", library.manifest.id), path.to_path_buf()))));
            }
        }
        let now = stamped_manifests(files);
        if !self.seen_once {
            self.seen_once = true;
            self.manifests = now;
            self.images = images;
            self.selections = selections;
            return Found::default();
        }
        let mut found = Found::default();
        // A file can be added or removed, so the list is read again each look.
        // A file is compared by its path too: a second file of one key can
        // come and go while the first stays.
        found.logical_images = images.iter().filter(|(path, hash)| self.images.get(*path) != Some(*hash))
            .map(|(path, _)| path.clone()).collect();
        found.logical_images.extend(self.images.keys().filter(|path| !images.contains_key(*path)).cloned());
        self.images = images;
        found.manifests.extend(selections.iter().filter(|(id, selected)| self.selections.get(*id) != Some(*selected))
            .map(|(id, _)| format!("content://{id}")));
        self.selections = selections;
        found.manifests.extend(now
            .iter()
            .filter(|(key, path, seen)| !self.manifests.iter().any(|(was, at, before)| was == key && at == path && before == seen))
            .map(|(key, ..)| key.clone()));
        found.manifests.extend(
            self.manifests
                .iter()
                .filter(|(was, at, _)| !now.iter().any(|(key, path, _)| key == was && path == at))
                .map(|(was, ..)| was.clone()),
        );
        found.logical_images.extend(now.iter().filter(|(key, _, _)| found.manifests.contains(key) && !key.ends_with(".ron"))
            .filter(|(_, path, _)| path.file_name().is_some_and(|name| name != "published.json"))
            .map(|(key, _, _)| key.clone()));
        self.manifests = now;
        if !found.manifests.is_empty() {
            let snapshot = self.libraries.snapshot();
            let table = snapshot.as_ref().map_err(Clone::clone).and_then(|snapshot| sheet_table::SheetTexts::from_snapshot(snapshot, Some(sprite_content)))
                .map_err(|error| sheet_table::SheetTableRefused(vec![("libraries".into(), error)]))
                .and_then(|texts| {
                    found.manifests.extend(texts.sheets.iter().map(|(key, _)| (*key).to_owned()));
                    found.manifests.extend(sheet_table::current(sprite_content).rons().iter().map(|(key, _)| (*key).to_owned()));
                    sheet_table::prepare_sheet_table(sprite_content, texts).inspect(|table| {
                        found.manifests.extend(table.registry().keys().map(str::to_owned));
                        found.manifests.extend(sheet_table::current(sprite_content).registry().keys().map(str::to_owned));
                    })
                });
            found.changes_mechanics = table.as_ref().is_ok_and(|table| table.changes_mechanics_of(sheet_table::current(&sprite_content)));
            found.table = Some(table);
            found.snapshot = snapshot.ok();
        }
        found
    }


}

/// The watch over one sprite tree.
#[derive(Resource)]
pub struct SpriteSheetWatch {
    roots: Vec<WatchRoot>,
    scanner: Arc<Mutex<Scanner>>,
    /// What the looks found and no frame took yet.
    found: Arc<Mutex<Vec<Found>>>,
    /// The keys of the changed manifests, waiting for their publication.
    staged: BTreeSet<String>,
    /// The table to publish: at once where it changes only where frames are
    /// drawn from, else at the end of the session.
    candidate: Option<sheet_table::PreparedSheetTable>,
    snapshot: Option<LibrarySnapshot>,
    images: Vec<String>,
    /// Whether `candidate` changes a fact that simulation or a cast reads.
    candidate_changes_mechanics: bool,
    /// Sheet generations this watch published.
    pub published: u32,
    /// Image reload requests sent by this watch.
    pub reloaded_pages: u32,
    /// Why the last changed table was refused, while no later one was
    /// published.
    pub refused: Option<String>,
}

impl SpriteSheetWatch {
    /// A watch over the tree whose `assets/` folder is under `owner`.
    pub fn new(sprite_content: &ambition_sprite_sheet::SpriteContent, libraries: ContentLibraries) -> Self {
        Self {
            scanner: Arc::new(Mutex::new(Scanner::new(libraries.clone(), sprite_content.clone()))),
            roots: libraries.iter().filter(|library| !library.is_embedded()).flat_map(|library| {
                library.manifest.asset_roots.values().map(|root| WatchRoot { path: library.root.join(root), recursive: true })
                    .chain(library.publication_root().map(|path| WatchRoot { path, recursive: false })).collect::<Vec<_>>()
            }).collect(),
            found: Arc::default(),
            staged: BTreeSet::new(),
            candidate: None,
            snapshot: None,
            images: Vec::new(),
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
        let mut reports = ChangeReports::of(&self.roots);
        let spawned = std::thread::Builder::new().name("sprite-sheet-watch".into()).spawn(move || {
            // The first look takes the baseline: it costs a full look once.
            let mut changed = true;
            loop {
                let (Some(scanner), Some(found)) = (scanner.upgrade(), found.upgrade()) else {
                    return;
                };
                if changed {
                    let look = scanner.lock().unwrap_or_else(PoisonError::into_inner).look();
                    if !look.manifests.is_empty() || !look.logical_images.is_empty() {
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


}

#[derive(Clone)]
struct WatchRoot { path: PathBuf, recursive: bool }

/// What the operating system reports about the sprite folders: a file under
/// them changed. Without it a look costs a stat of each file of the tree
/// (about 0.2 to 0.4 s in a debug build) each second.
struct ChangeReports {
    #[cfg(not(target_arch = "wasm32"))]
    reports: Option<(notify::RecommendedWatcher, std::sync::mpsc::Receiver<notify::Result<notify::Event>>)>,
    /// The sprite folders that are not there yet (a quality tier the
    /// publisher has not written). The nearest folder above each that is
    /// there is watched, and each is watched itself once it comes.
    #[cfg(not(target_arch = "wasm32"))]
    awaited: Vec<WatchRoot>,
    /// The folders watched, without their contents, for an awaited one.
    #[cfg(not(target_arch = "wasm32"))]
    above: Vec<PathBuf>,
}

impl ChangeReports {
    #[cfg(not(target_arch = "wasm32"))]
    fn of(roots: &[WatchRoot]) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let dirs = roots.iter().cloned();
        let mut reports = Self { reports: None, awaited: dirs.collect(), above: Vec::new() };
        match notify::recommended_watcher(sender) {
            Ok(watcher) => {
                reports.reports = Some((watcher, receiver));
                if let Err(error) = reports.watch_what_came() {
                    reports.reports = None;
                    warn!(target: "ambition_platformer2d::sprites", "no change reports for the sprite tree ({error}); it is looked at each {LOOK_EVERY:?}");
                }
            }
            Err(error) => {
                warn!(target: "ambition_platformer2d::sprites", "no change reports for the sprite tree ({error}); it is looked at each {LOOK_EVERY:?}");
            }
        }
        reports
    }

    /// Watch each awaited folder that is there now, with all it holds, and
    /// the nearest folder above each one that is not. True when a folder
    /// came: what it holds was written with no report.
    #[cfg(not(target_arch = "wasm32"))]
    fn watch_what_came(&mut self) -> notify::Result<bool> {
        use notify::Watcher;
        let Some((watcher, _)) = &mut self.reports else {
            return Ok(false);
        };
        let mut came = false;
        let mut awaited = Vec::new();
        for dir in std::mem::take(&mut self.awaited) {
            if dir.path.is_dir() {
                let mode = if dir.recursive { notify::RecursiveMode::Recursive } else { notify::RecursiveMode::NonRecursive };
                watcher.watch(&dir.path, mode)?;
                came = true;
                continue;
            }
            if let Some(above) = dir.path.ancestors().skip(1).find(|above| above.is_dir()) {
                if !self.above.iter().any(|watched| watched == above) {
                    watcher.watch(above, notify::RecursiveMode::NonRecursive)?;
                    self.above.push(above.to_path_buf());
                }
            }
            awaited.push(dir);
        }
        self.awaited = awaited;
        Ok(came)
    }

    #[cfg(target_arch = "wasm32")]
    fn of(_roots: &[WatchRoot]) -> Self {
        Self {}
    }

    /// `None` when no report came within `longest`; else whether one of the
    /// reports (and those that follow it within a short time) is of a file a
    /// sheet generation or a page is read from.
    #[cfg(not(target_arch = "wasm32"))]
    fn reports_within(&self, longest: Duration) -> Option<bool> {
        let (_, receiver) = self.reports.as_ref()?;
        let relevant = |report: notify::Result<notify::Event>| {
            // An error can be a lost report: look.
            report.map_or(true, |event| event.paths.iter().any(|path| watched_file(path)))
        };
        let mut changed = relevant(receiver.recv_timeout(longest).ok()?);
        // A save is several reports, and a publish writes many files: one
        // look for all of them.
        while let Ok(report) = receiver.recv_timeout(SETTLE) {
            changed |= relevant(report);
        }
        Some(changed)
    }

    /// Wait at most `longest` for a report of a change to a file a sheet
    /// generation or a page is read from. True when one came (and the reports
    /// that follow it within a short time), or when no reports are possible.
    fn wait(&mut self, longest: Duration) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if self.reports.is_some() {
            let mut changed = match self.reports_within(longest) {
                Some(changed) => changed,
                None => return false,
            };
            // A report can be a folder that came: watch it, and look.
            if !self.awaited.is_empty() {
                changed |= self.watch_what_came().unwrap_or(true);
            }
            return changed;
        }
        std::thread::sleep(longest);
        true
    }
}

/// The time a burst of change reports is given to end before the look.
const SETTLE: Duration = Duration::from_millis(150);

/// Only the selected publication can change the runtime asset view.
fn watched_file(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "published.json")
        || path.extension().is_some_and(|extension| matches!(extension.to_str(), Some("ron" | "json" | "png" | "otf" | "ttf" | "ogg" | "wav" | "bank")))
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|metadata| metadata.modified()).ok()
}

/// Each sheet manifest, body rig and part flipbook, the files a sheet
/// generation is read from. A rig or a flipbook is keyed by its file name, so
/// no such key is a manifest key.
fn stamped_manifests(files: Vec<(String, PathBuf)>) -> Vec<(String, PathBuf, Option<SystemTime>)> {
    files.into_iter().map(|(key, path)| { let seen = modified(&path); (key, path, seen) }).collect()
}

fn reload(asset_server: Option<&AssetServer>, libraries: Option<&ambition_asset_manager::library_source::LibraryAssets>, paths: Vec<String>) -> u32 {
    let Some(asset_server) = asset_server else {
        return 0;
    };
    let sources = libraries.map(|libraries| libraries.relative_sources()).unwrap_or_default();
    let mut aliases = std::collections::BTreeSet::from_iter(paths);
    for path in aliases.clone() {
        let relative = path.strip_prefix("content://")
            .and_then(|path| path.split_once('/').map(|(_, relative)| relative))
            .or_else(|| (!path.contains("://")).then_some(path.as_str()));
        let Some(relative) = relative else { continue; };
        if asset_server.get_path_id(relative.to_owned()).is_some() { aliases.insert(relative.to_owned()); }
        for source in &sources {
            let qualified = format!("{source}://{relative}");
            if asset_server.get_path_id(qualified.clone()).is_some() { aliases.insert(qualified); }
        }
    }
    for path in &aliases { asset_server.reload(path.clone()); }
    aliases.len() as u32
}

/// Load each changed page image again at once, and stage each changed
/// manifest for the end of the session.
fn take_what_the_watch_found(watch: Option<ResMut<SpriteSheetWatch>>) {
    let Some(mut watch) = watch else {
        return;
    };
    let found = std::mem::take(&mut *watch.found.lock().unwrap_or_else(PoisonError::into_inner));
    for look in found {
        let paths = look.logical_images;

        match look.table {
            None => {}
            Some(Ok(table)) => {
                let when = if look.changes_mechanics { "from the next session" } else { "at once (only where frames are drawn from changed)" };
                info!(target: "ambition_platformer2d::sprites", "Sprite content changed; {} sheet identities are played {when}", look.manifests.len());
                watch.staged.extend(look.manifests);
                watch.candidate = Some(table);
                watch.snapshot = look.snapshot;
                watch.images = paths;
                watch.candidate_changes_mechanics = look.changes_mechanics;
                watch.refused = None;
            }
            // The table is read whole, so a broken one replaces any change
            // staged before it.
            Some(Err(refused)) => {
                let why = format!("the sprite sheets are refused; the running sheets stay:\n{refused}");
                error!(target: "ambition_platformer2d::sprites", "{why}");
                watch.staged.clear();
                watch.candidate = None;
                watch.snapshot = None;
                watch.images.clear();
                watch.refused = Some(why);
            }
        }
    }
}

/// Publish a staged sheet table where no route prepares a session: a table
/// that changes only where frames are drawn from at once, and any other table
/// where no session is live.
fn publish_the_staged_sheets(world: &mut World) {
    let mut sprite_content = world.resource::<ambition_sprite_sheet::SpriteContent>().clone();
    let Some(changes_mechanics) = world.get_resource::<SpriteSheetWatch>().filter(|watch| watch.is_staged()).map(|watch| watch.candidate_changes_mechanics) else {
        return;
    };
    let in_a_session = world.get_resource::<crate::actor::ActiveSessionScope>().is_some_and(|scope| scope.current().is_some());
    let preparing = world.get_resource::<crate::game_shell::ShellRouter>().is_some_and(|router| router.pending.is_some());
    if (in_a_session && changes_mechanics) || preparing {
        return;
    }
    let (table, staged, snapshot, images) = {
        let mut watch = world.resource_mut::<SpriteSheetWatch>();
        let Some(table) = watch.candidate.take() else {
            return;
        };
        (table, std::mem::take(&mut watch.staged), watch.snapshot.take(), std::mem::take(&mut watch.images))
    };
    if let Some(snapshot) = snapshot {
        if let Some(mut catalog) = world.get_resource_mut::<ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog>() {
            *catalog = catalog.clone().with_snapshot(&snapshot);
        }
        world.get_resource_or_insert_with(ambition_asset_manager::library_source::LibraryAssets::default).replace(snapshot);
    }
    let reloaded = reload(world.get_resource::<AssetServer>(), world.get_resource::<ambition_asset_manager::library_source::LibraryAssets>(), images);
    world.resource_mut::<SpriteSheetWatch>().reloaded_pages += reloaded;
    let generation = sheet_table::publish_prepared_sheet_table(&mut sprite_content, table);
    world.insert_resource(sprite_content.clone());
    world.resource::<SpriteSheetWatch>().scanner.lock().unwrap_or_else(PoisonError::into_inner).content = sprite_content.clone();
    info!(target: "ambition_platformer2d::sprites", "Sprite generation {generation} published with {} sheet identities", staged.len());
    // A restater reads only facts `changes_mechanics_of` compares, so a
    // table that changes none of them leaves each cast as it is.
    let restaters = if changes_mechanics { world.get_resource::<SheetDerivedRestaters>().cloned().unwrap_or_default() } else { SheetDerivedRestaters::default() };
    for (name, restate) in &restaters.0 {
        info!(target: "ambition_platformer2d::sprites", "restating `{name}` from sheet generation {generation}");
        restate(world);
    }
    crate::game_assets::reload_prop_sheets_of_targets(world, &staged);
    crate::game_assets::reload_fx_sheets_of_targets(world, &staged);
    // Each resident realization is now of an older generation: the residency
    // convergence makes a worn one again in place (its body keeps drawing
    // until the new one is ready, then binds again) and retires the others
    // (`converge_character_residency_to_active_quality`).
    let mut watch = world.resource_mut::<SpriteSheetWatch>();
    watch.published += 1;
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn a_loose_image_change_requests_a_reload_without_a_sheet_revision() {
        let owner = std::env::temp_dir().join(format!("loose_sprite_watch_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&owner);
        let store = owner.join("art_publication");
        std::fs::create_dir_all(&store).unwrap();
        let select = |generation: &str, hash: &str| {
            let physical = format!("bundles/{generation}/runtime/full/sprites/prop.png");
            let path = store.join(&physical);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, hash).unwrap();
            std::fs::write(store.join("next.json"), serde_json::json!({
                "schema":ambition_asset_manager::art_publication::SCHEMA,"generation":generation,
                "files":{"sprites/prop.png":{"path":physical,"sha256":hash,"role":"runtime","tier":"full"}}
            }).to_string()).unwrap();
            std::fs::rename(store.join("next.json"), store.join("published.json")).unwrap();
        };
        select("one", "old");
        std::fs::write(owner.join("library.json"), serde_json::json!({
            "schema":"ambition.content_library.v1", "id":"probe", "version":"1",
            "asset_roots":{}, "publication":"art_publication"
        }).to_string()).unwrap();
        let mut libraries = ContentLibraries::default();
        libraries.insert(ambition_asset_manager::content_library::ContentLibrary::read(owner.join("library.json")).unwrap()).unwrap();
        let mut scanner = Scanner::new(libraries, Default::default());
        assert!(scanner.look().logical_images.is_empty());
        select("two", "new");
        let found = scanner.look();
        assert_eq!(found.logical_images, ["content://probe/sprites/prop.png"]);
        assert!(found.table.is_some_and(|table| table.is_ok()));
        let _ = std::fs::remove_dir_all(owner);
    }

    /// The selection directory can be created after the watch starts.
    #[test]
    fn an_atomic_selection_created_after_the_watch_began_is_watched() {
        let owner = std::env::temp_dir().join(format!("sprite_watch_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&owner);
        std::fs::create_dir_all(&owner).unwrap();
        let store = owner.join("art_publication");
        let mut reports = ChangeReports::of(&[WatchRoot { path: store.clone(), recursive: false }]);
        assert!(reports.reports.is_some());
        assert!(!reports.wait(Duration::from_millis(300)));
        std::fs::create_dir_all(&store).unwrap();
        assert!(reports.wait(Duration::from_secs(5)));
        std::fs::write(store.join("next.json"), "{}").unwrap();
        std::fs::rename(store.join("next.json"), store.join("published.json")).unwrap();
        assert!(reports.wait(Duration::from_secs(5)));
        let _ = std::fs::remove_dir_all(&owner);
    }

}
