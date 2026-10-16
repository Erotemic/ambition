//! A RUNNING GAME PLAYS AN EDIT TO ITS SPRITE SHEETS, with no Rust build and
//! no restart.
//!
//! A build that reads its sprite tree off disk (no `embedded_sprites`) adds
//! [`SpriteSheetReloadPlugin`]. The plugin looks at each sheet manifest
//! (`*_spritesheet.ron`), each body rig (`*_body_rig.ron`) and each page image
//! of those sheets. One look at
//! the published tree costs about 100 ms in a debug build (measured
//! 2026-10-16), so a background thread looks, once a second, and a frame only
//! takes the changes it found.
//!
//! The rules of the road:
//!
//! - A changed page image changes only pixels, so it is loaded again at once
//!   (`AssetServer::reload`), also inside a session. Bevy loads again only an
//!   image that is loaded.
//! - A changed sheet manifest or body rig changes records that simulation
//!   reads too (body sizes, hurtboxes, attack volumes, a boss's size, hurt
//!   parts), so it waits for
//!   the end of the session, as a restated content schema does. Nothing is
//!   published while a route prepares a session. The look that saw the
//!   change reads the whole sheet table again and parses it (about a second
//!   in a debug build, off the frame); the end of the session publishes it as
//!   one generation
//!   (`baked_sheet_rons::publish_sheet_generation`), the `SheetRegistry`
//!   resource is stated again, each cast made from the sheets is stated
//!   again ([`SheetDerivedRestaters`]), each resident character realization is
//!   retired so that the next session builds it from the new records, and
//!   the pages of each changed sheet are loaded again.
//! - A table with a file that does not parse is refused, with each file
//!   named, and the running table stays.
//!
//! Not on this road yet: part flipbooks, landmarks, portraits,
//! the pack catalogs and the prop sheets a running room drew. Each has its
//! own process cache (see `docs/planning/engine/content-generation-and-reload.md`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use ambition_sprite_sheet::baked_sheet_rons as sheet_table;
use bevy::prelude::*;


/// Time between two looks of the background thread.
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
        app.add_systems(Update, (take_what_the_watch_found, publish_the_staged_sheets_between_sessions).chain());
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
    manifests: Vec<(String, PathBuf, Option<SystemTime>)>,
    pages: Vec<(PathBuf, Option<SystemTime>)>,
}

impl Scanner {
    fn new(owner: PathBuf) -> Self {
        Self { owner, seen_once: false, manifests: Vec::new(), pages: Vec::new() }
    }

    fn look(&mut self) -> Found {
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
            found.table = Some(sheet_table::prepare_sheet_table(sheet_table::read_sheet_table_under(&self.owner)));
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
    /// The keys of the changed manifests, waiting for the end of the session.
    staged: BTreeSet<String>,
    /// The table to publish at the end of the session.
    candidate: Option<sheet_table::PreparedSheetTable>,
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

    /// A thread that looks once each [`LOOK_EVERY`]. It ends when the watch
    /// is dropped.
    fn look_in_the_background(&self) {
        let scanner = Arc::downgrade(&self.scanner);
        let found = Arc::downgrade(&self.found);
        let spawned = std::thread::Builder::new().name("sprite-sheet-watch".into()).spawn(move || loop {
            std::thread::sleep(LOOK_EVERY);
            let (Some(scanner), Some(found)) = (scanner.upgrade(), found.upgrade()) else {
                return;
            };
            let look = scanner.lock().unwrap_or_else(PoisonError::into_inner).look();
            if !look.manifests.is_empty() || !look.pages.is_empty() {
                found.lock().unwrap_or_else(PoisonError::into_inner).push(look);
            }
        });
        if let Err(error) = spawned {
            warn!(target: "ambition_platformer2d::sprites", "no sprite sheet watch: {error}");
        }
    }

    /// A changed sheet table waits for the end of the session.
    pub fn is_staged(&self) -> bool {
        self.candidate.is_some()
    }

    /// The asset path of `page`: its path under the tree's `assets/` folder.
    fn asset_path(&self, page: &Path) -> Option<String> {
        page.strip_prefix(self.owner.join("assets")).ok().map(|path| path.to_string_lossy().replace('\\', "/"))
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|metadata| metadata.modified()).ok()
}

/// Each sheet manifest and each body rig, the files a sheet generation is read
/// from. A rig is keyed by its file name, so no rig key is a manifest key.
fn stamped_manifests(owner: &Path) -> Vec<(String, PathBuf, Option<SystemTime>)> {
    let rigs = sheet_table::body_rig_files_under(owner)
        .into_iter()
        .map(|(target, path)| (format!("{target}{}", sheet_table::BODY_RIG_SUFFIX), path));
    sheet_table::sheet_files_under(owner)
        .into_iter()
        .chain(rigs)
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
    let registry = sheet_table::current().registry();
    let mut pages = BTreeSet::new();
    for (key, path, _) in manifests {
        let (Some(record), Some(dir)) = (registry.get(key), path.parent()) else {
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
                info!(target: "ambition_platformer2d::sprites", "sprite sheets changed: {:?}; they are played from the next session", look.manifests);
                watch.staged.extend(look.manifests);
                watch.candidate = Some(table);
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

/// Publish a staged sheet table where no session is live and no route
/// prepares one.
fn publish_the_staged_sheets_between_sessions(world: &mut World) {
    if !world.get_resource::<SpriteSheetWatch>().is_some_and(SpriteSheetWatch::is_staged) {
        return;
    }
    let in_a_session = world.get_resource::<crate::actor::ActiveSessionScope>().is_some_and(|scope| scope.current().is_some());
    let preparing = world.get_resource::<crate::game_shell::ShellRouter>().is_some_and(|router| router.pending.is_some());
    if in_a_session || preparing {
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
    let restaters = world.get_resource::<SheetDerivedRestaters>().cloned().unwrap_or_default();
    for (name, restate) in &restaters.0 {
        info!(target: "ambition_platformer2d::sprites", "restating `{name}` from sheet generation {generation}");
        restate(world);
    }
    if let Some(mut assets) = world.get_resource_mut::<ambition_sprite_sheet::game_assets::GameAssets>() {
        assets.characters.retire_realizations_except(&BTreeSet::new());
    }
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
