//! ⭐ THE RUNNING GAME PLAYS A CONTENT EDIT (fast-iteration I3 step 4: "file
//! watching calls the same request path").
//!
//! A build that reads its content off disk (no `static_content` feature) looks
//! at `pack.ron` and at every source it declares; a change to `pack.ron` also
//! changes which files are looked at, so a source added there is watched from
//! the next look. When a file changes, it compiles the pack
//! again from the files on disk ([`crate::pack::compile_pack_from`]) and asks
//! for a reload ([`crate::reload::request_reload`]). The reload road decides
//! the rest: it refuses a pack that does not compile, a change to a domain
//! that cannot reload yet, and a timeline another owner holds, and otherwise
//! re-prepares the route so the new generation is published at one boundary.
//!
//! A change seen while no route can re-prepare (a menu, a reload already in
//! flight) is kept and asked for again at the next look, so an edit made on
//! the title screen reaches the game that starts after it.

use bevy::prelude::*;

/// The declared sources and the modification time each was last seen at.
#[derive(Resource)]
pub struct ContentSourceWatch {
    root: std::path::PathBuf,
    files: Vec<(String, Option<std::time::SystemTime>)>,
    frames_until_poll: u32,
    /// A change was seen and no reload has answered for it yet.
    dirty: bool,
    /// The last answer that kept the change waiting, so it is reported once.
    waiting: Option<String>,
    /// Reloads this watch asked for. For tests and the inspector.
    pub requested: u32,
    /// The host looks at these sources together with the module files
    /// ([`look_at_content_sources`], [`request_from_sources`]), so a script
    /// and the content it uses are one candidate; [`watch_content_sources`]
    /// then stays out.
    pub paired: bool,
}

impl ContentSourceWatch {
    /// Watch the declared sources under `root`, as they are now.
    pub fn new(root: std::path::PathBuf) -> Self {
        let files = watched_files(&root)
            .into_iter()
            .map(|path| {
                let seen = modified(&root.join(&path));
                (path, seen)
            })
            .collect();
        Self {
            root,
            files,
            frames_until_poll: POLL_FRAMES,
            dirty: false,
            waiting: None,
            requested: 0,
            paired: false,
        }
    }

    /// Look at the files now, at the next poll.
    pub fn poll_now(&mut self) {
        self.frames_until_poll = 0;
    }
}

/// Frames between two looks: a stat per file is cheap, and a third of a
/// second is below a developer's switch from editor to game.
const POLL_FRAMES: u32 = 20;

/// The manifest and each source it declares.
fn watched_files(root: &std::path::Path) -> Vec<String> {
    std::iter::once("pack.ron".to_string())
        .chain(crate::pack::declared_source_paths_in(root))
        .collect()
}

fn modified(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Look at the files; on a change, compile the pack from disk and request a
/// reload.
pub fn watch_content_sources(world: &mut World) {
    {
        let Some(mut watch) = world.get_resource_mut::<ContentSourceWatch>() else {
            return;
        };
        if watch.paired {
            return;
        }
        if watch.frames_until_poll > 0 {
            watch.frames_until_poll -= 1;
            return;
        }
        watch.frames_until_poll = POLL_FRAMES;
    }
    if let Some(root) = look_at_content_sources(world) {
        request_from_sources(world, &root);
    }
}

/// The host looks at these sources with the module files from now on
/// ([`ContentSourceWatch::paired`]).
pub fn pair_with_modules(world: &mut World) {
    if let Some(mut watch) = world.get_resource_mut::<ContentSourceWatch>() {
        watch.paired = true;
    }
}

/// The directory the watch reads, for a host that asks for its content again.
pub fn watched_root(world: &World) -> std::path::PathBuf {
    world.resource::<ContentSourceWatch>().root.clone()
}

/// How [`request_from_sources`] was answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcesAnswer {
    /// A reload of the changed content is in flight.
    Requested,
    /// The compiled content is what runs: nothing to publish.
    Unchanged,
    /// No route can re-prepare now; the change is asked for again at the
    /// next look.
    Waiting,
    /// The change does not compile or was refused; the running content stays.
    Refused,
}

/// Look at the files now. `Some(root)` while a change waits for an answer: a
/// change seen now, or one an earlier look kept ([`SourcesAnswer::Waiting`]).
pub fn look_at_content_sources(world: &mut World) -> Option<std::path::PathBuf> {
    let mut watch = world.get_resource_mut::<ContentSourceWatch>()?;
    let root = watch.root.clone();
    let mut changed = Vec::new();
    for (path, seen) in &mut watch.files {
        let now = modified(&root.join(path.as_str()));
        if now != *seen {
            *seen = now;
            changed.push(path.clone());
        }
    }
    if changed.iter().any(|path| path == "pack.ron") {
        // The list moved: a source the old list did not have is new now,
        // so it is a change; one that stayed keeps the time it was seen at.
        let old = std::mem::take(&mut watch.files);
        watch.files = watched_files(&root)
            .into_iter()
            .map(|path| match old.iter().find(|(was, _)| *was == path) {
                Some((_, seen)) => (path, *seen),
                None => {
                    let seen = modified(&root.join(&path));
                    changed.push(path.clone());
                    (path, seen)
                }
            })
            .collect();
    }
    if !changed.is_empty() {
        info!("content sources changed: {changed:?}");
        watch.dirty = true;
        watch.waiting = None;
    }
    watch.dirty.then_some(root)
}

/// Compile the pack under `root` from disk and request a reload of it.
pub fn request_from_sources(world: &mut World, root: &std::path::Path) -> SourcesAnswer {
    let pack = match crate::pack::compile_pack_from(root) {
        Ok(pack) => pack,
        Err(diagnostic) => {
            error!("content sources changed and do not compile; the running content stays:\n{diagnostic}");
            world.resource_mut::<ContentSourceWatch>().dirty = false;
            return SourcesAnswer::Refused;
        }
    };
    let candidate = ambition_content_pack::CandidateGeneration::prepared_against(std::sync::Arc::new(pack), None);
    let answer = crate::reload::request_reload(world, candidate);
    let mut watch = world.resource_mut::<ContentSourceWatch>();
    use crate::reload::ReloadRequest;
    match &answer {
        ReloadRequest::Requested { route, superseded, .. } => {
            match superseded {
                Some(older) => info!(
                    "content reload requested for route `{route}`; it supersedes the one in flight ({older:?})"
                ),
                None => info!("content reload requested for route `{route}`"),
            }
            watch.requested += 1;
            watch.dirty = false;
            SourcesAnswer::Requested
        }
        ReloadRequest::CancelledInFlight { cancelled } => {
            info!("content sources changed back to what is running; the reload in flight ({cancelled:?}) was cancelled");
            watch.dirty = false;
            SourcesAnswer::Unchanged
        }
        ReloadRequest::Unchanged => {
            info!("content sources changed and the compiled content did not");
            watch.dirty = false;
            SourcesAnswer::Unchanged
        }
        ReloadRequest::PackRefused(diagnostic) => {
            error!("content reload refused; the running content stays:\n{diagnostic}");
            watch.dirty = false;
            SourcesAnswer::Refused
        }
        ReloadRequest::Refused(reason) => {
            warn!("content reload refused; the running content stays: {reason:?}");
            watch.dirty = false;
            SourcesAnswer::Refused
        }
        // Kept: asked for again at the next look.
        ReloadRequest::NoActiveRoute
        | ReloadRequest::RouteHasNoPreparation(_) => {
            let said = format!("{answer:?}");
            if watch.waiting.as_ref() != Some(&said) {
                info!("content sources changed; the reload waits: {said}");
                watch.waiting = Some(said);
            }
            SourcesAnswer::Waiting
        }
    }
}

/// Stop asking for a change a paired refusal answered: the running content
/// stays, and the next save is a new change.
pub fn forget_source_change(world: &mut World) {
    if let Some(mut watch) = world.get_resource_mut::<ContentSourceWatch>() {
        watch.dirty = false;
        watch.waiting = None;
    }
}

/// ⭐ THE RUNNING GAME PLAYS A SAVED DIALOGUE EDIT. The Yarn files are not in
/// the pack: they are the running `YarnProject`'s assets. A saved file is
/// compiled with every other file of the project first, so an edit that does
/// not compile changes nothing; then its asset takes the new text, and
/// bevy_yarnspinner recompiles the project and restarts a running dialogue at
/// its current node.
#[cfg(feature = "ui")]
mod yarn {
    use bevy::prelude::*;
    use bevy_yarnspinner::prelude::{YarnFile, YarnProject};

    use super::{modified, POLL_FRAMES};

    /// The Yarn files the running project was built from, by name, and the
    /// modification time each was last seen at.
    #[derive(Resource)]
    pub struct YarnSourceWatch {
        root: std::path::PathBuf,
        files: Vec<(String, Option<std::time::SystemTime>)>,
        frames_until_poll: u32,
        /// Saved edits the running project took. For tests and the inspector.
        pub reloaded: u32,
    }

    impl YarnSourceWatch {
        /// Watch the files named (each read at `root`/name), as they are now.
        pub fn new(root: std::path::PathBuf, names: impl IntoIterator<Item = String>) -> Self {
            let files = names
                .into_iter()
                .map(|name| {
                    let seen = modified(&root.join(&name));
                    (name, seen)
                })
                .collect();
            Self { root, files, frames_until_poll: POLL_FRAMES, reloaded: 0 }
        }

        /// Look at the files now, at the next poll.
        pub fn poll_now(&mut self) {
            self.frames_until_poll = 0;
        }
    }

    /// Look at the files; give the running project each saved one that
    /// compiles with the rest.
    pub fn watch_yarn_sources(world: &mut World) {
        let edits = {
            let Some(mut watch) = world.get_resource_mut::<YarnSourceWatch>() else {
                return;
            };
            if watch.frames_until_poll > 0 {
                watch.frames_until_poll -= 1;
                return;
            }
            watch.frames_until_poll = POLL_FRAMES;
            let root = watch.root.clone();
            let mut edits = Vec::new();
            for (name, seen) in &mut watch.files {
                let path = root.join(name.as_str());
                let now = modified(&path);
                if now == *seen {
                    continue;
                }
                *seen = now;
                match std::fs::read_to_string(&path) {
                    Ok(text) => edits.push((name.clone(), text)),
                    Err(error) => error!("Yarn file {} changed and cannot be read: {error}", path.display()),
                }
            }
            edits
        };
        if edits.is_empty() {
            return;
        }
        let names: Vec<String> = edits.iter().map(|(name, _)| name.clone()).collect();
        match replace_yarn_sources(world, edits) {
            Ok(()) => {
                info!("dialogue reloaded from {names:?}");
                world.resource_mut::<YarnSourceWatch>().reloaded += 1;
            }
            Err(reason) => error!("dialogue edit in {names:?} refused; the running dialogue stays:\n{reason}"),
        }
    }

    /// Give the running project new text for some of its files. The whole
    /// project is compiled with the new text first; `Err` changes nothing.
    pub fn replace_yarn_sources(world: &mut World, edits: Vec<(String, String)>) -> Result<(), String> {
        let project = world
            .get_resource::<YarnProject>()
            .ok_or("no Yarn project is loaded")?;
        let assets = world.resource::<Assets<YarnFile>>();
        let mut compiler = yarnspinner::compiler::Compiler::new();
        let mut targets = Vec::new();
        for handle in project.yarn_files() {
            let file = assets.get(handle).ok_or("a project file is not loaded")?;
            let edited = edits.iter().find(|(name, _)| name == file.file_name());
            let source = edited.map_or_else(|| file.content().to_string(), |(_, text)| text.clone());
            if edited.is_some() {
                targets.push((handle.id(), source.clone()));
            }
            compiler.add_file(yarnspinner::compiler::File {
                file_name: file.file_name().to_string(),
                source,
            });
        }
        if targets.len() != edits.len() {
            return Err(format!(
                "{} of the edited files are not in the running project",
                edits.len() - targets.len()
            ));
        }
        compiler.compile().map_err(|error| error.to_string())?;
        // Each new file is built on a copy: `YarnFile::set_content` writes the
        // text BEFORE it compiles the file's strings, so a refusal there on the
        // live asset would leave text the program was not compiled from. The
        // whole-project compile above refuses every source the strings pass
        // refuses (probed: an unclosed block fails both, a type error only the
        // compile), so this is the order of the writes, not a second check.
        let mut built = Vec::new();
        for (id, source) in targets {
            let mut file = assets.get(id).ok_or("a project file is not loaded")?.clone();
            file.set_content(source).map_err(|error| error.to_string())?;
            built.push((id, file));
        }
        let mut assets = world.resource_mut::<Assets<YarnFile>>();
        for (id, file) in built {
            if let Some(mut live) = assets.get_mut(id) {
                *live = file;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "ui")]
pub use yarn::{replace_yarn_sources, watch_yarn_sources, YarnSourceWatch};
