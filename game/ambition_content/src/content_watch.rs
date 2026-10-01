//! ⭐ THE RUNNING GAME PLAYS A CONTENT EDIT (fast-iteration I3 step 4: "file
//! watching calls the same request path").
//!
//! A build that reads its content off disk (no `static_content` feature) looks
//! at every source `pack.ron` declares. When one changes, it compiles the pack
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
}

impl ContentSourceWatch {
    /// Watch the declared sources under `root`, as they are now.
    pub fn new(root: std::path::PathBuf) -> Self {
        let files = crate::pack::declared_source_paths()
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

fn modified(path: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Look at the files; on a change, compile the pack from disk and request a
/// reload.
pub fn watch_content_sources(world: &mut World) {
    let root = {
        let Some(mut watch) = world.get_resource_mut::<ContentSourceWatch>() else {
            return;
        };
        if watch.frames_until_poll > 0 {
            watch.frames_until_poll -= 1;
            return;
        }
        watch.frames_until_poll = POLL_FRAMES;
        let root = watch.root.clone();
        let mut changed = Vec::new();
        for (path, seen) in &mut watch.files {
            let now = modified(&root.join(path.as_str()));
            if now != *seen {
                *seen = now;
                changed.push(path.clone());
            }
        }
        if !changed.is_empty() {
            info!("content sources changed: {changed:?}");
            watch.dirty = true;
            watch.waiting = None;
        }
        if !watch.dirty {
            return;
        }
        root
    };
    let pack = match crate::pack::compile_pack_from(&root) {
        Ok(pack) => pack,
        Err(diagnostic) => {
            error!("content sources changed and do not compile; the running content stays:\n{diagnostic}");
            world.resource_mut::<ContentSourceWatch>().dirty = false;
            return;
        }
    };
    let candidate = ambition_content_pack::CandidateGeneration::prepared_against(std::sync::Arc::new(pack), None);
    let answer = crate::reload::request_reload(world, candidate);
    let mut watch = world.resource_mut::<ContentSourceWatch>();
    use crate::reload::ReloadRequest;
    match &answer {
        ReloadRequest::Requested { route, .. } => {
            info!("content reload requested for route `{route}`");
            watch.requested += 1;
            watch.dirty = false;
        }
        ReloadRequest::Unchanged => {
            info!("content sources changed and the compiled content did not");
            watch.dirty = false;
        }
        ReloadRequest::PackRefused(diagnostic) => {
            error!("content reload refused; the running content stays:\n{diagnostic}");
            watch.dirty = false;
        }
        ReloadRequest::Refused(reason) => {
            warn!("content reload refused; the running content stays: {reason:?}");
            watch.dirty = false;
        }
        // Kept: asked for again at the next look.
        ReloadRequest::AlreadyPending { .. }
        | ReloadRequest::NoActiveRoute
        | ReloadRequest::RouteHasNoPreparation(_) => {
            let said = format!("{answer:?}");
            if watch.waiting.as_ref() != Some(&said) {
                info!("content sources changed; the reload waits: {said}");
                watch.waiting = Some(said);
            }
        }
    }
}
