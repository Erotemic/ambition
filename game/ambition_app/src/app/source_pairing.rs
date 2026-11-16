//! A script and the content it uses are one candidate (runtime authoring
//! plan P5 step 1).
//!
//! The content sources and the module files were looked at by two pollers
//! on two clocks, in two schedules: a save that changed a script and the
//! content it reads (a technique's id in a pack row and in the script's
//! trigger) published the script within a few frames and the content at its
//! route's activation, so the game ran the new script against the old
//! content in between, and a refusal of one half still published the other.
//!
//! This looks at both in one system. A change to only one side goes through
//! its own road as before. A change to both stages the modules as one whole
//! candidate (a file that does not load refuses it) and asks for the content
//! reload with the modules attached ([`PairedModuleCandidate`]): the content
//! generation's commit publishes them, and every end of it that does not
//! activate discards them. A refusal of either half leaves the running pair,
//! and the next change to either side asks for both again: a pair repaired
//! one file at a time still activates as a pair.

use ambition_content::content_watch::{self, ContentSourceWatch, SourcesAnswer};
use ambition_content::reload::PairedModuleCandidate;
use ambition_platformer2d::extension as ext;
use bevy::prelude::*;

/// Frames between two looks, as each poller had.
const POLL_FRAMES: u32 = 20;

pub struct SourcePairingPlugin;

impl Plugin for SourcePairingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            watch_project_sources
                .before(content_watch::watch_content_sources)
                .run_if(resource_exists::<ContentSourceWatch>),
        );
    }
}

fn watch_project_sources(world: &mut World, mut frames_until_poll: Local<u32>, mut paired: Local<Option<bool>>) {
    // Pair once both pollers exist; without module files there is nothing to
    // pair, and the content poller keeps its own clock.
    let paired = *paired.get_or_insert_with(|| {
        let has_modules = world.contains_resource::<ext::ExtensionModuleFiles>()
            || std::env::var_os(ext::EXTENSION_MODULES_VAR).is_some();
        if has_modules {
            world.insert_resource(ext::ModuleFilesLookedWithContent);
        }
        has_modules
    });
    if !paired {
        return;
    }
    // Asserted at every look: a host or a test may put a new watch in place.
    world.resource_mut::<ContentSourceWatch>().paired = true;
    if *frames_until_poll > 0 {
        *frames_until_poll -= 1;
        return;
    }
    *frames_until_poll = POLL_FRAMES;
    let refused = ext::module_pair_was_refused(world);
    let content = content_watch::look_at_content_sources(world);
    // A refused pair is asked for again as a pair: a change to its content
    // looks at its modules again, and a change to its modules asks for its
    // content again.
    let mut modules = ext::look_at_module_files(world, refused && content.is_some());
    let content = match content {
        None if refused && modules.is_some() => Some(content_watch::watched_root(world)),
        content => content,
    };
    if refused && content.is_some() && modules.is_none() {
        modules = ext::look_at_module_files(world, true);
    }
    let held = world.contains_resource::<PairedModuleCandidate>();
    match (modules, content) {
        (None, None) => {}
        // Modules held for a content generation in flight join it: proposing
        // them now would publish the pair's code ahead of its content.
        (Some(changes), None) if held => match ext::stage_module_file_changes(world, changes) {
            Ok(names) => info!("{names:?} changed; staged with the content reload in flight"),
            Err(reason) => error!("changed modules are NOT loaded; the staged pair stays: {reason}"),
        },
        (Some(changes), None) => stage_alone(world, changes),
        (None, Some(root)) => {
            content_watch::request_from_sources(world, &root);
        }
        (Some(changes), Some(root)) => match ext::stage_module_file_changes(world, changes) {
            Err(reason) => {
                error!("a script and its content changed together and the script is refused; the running pair stays: {reason}");
                content_watch::forget_source_change(world);
                ext::discard_paired_modules(world);
            }
            Ok(names) => {
                ext::forget_module_pair_refusal(world);
                world.insert_resource(PairedModuleCandidate {
                    publish: ext::publish_paired_modules,
                    discard: ext::discard_paired_modules,
                });
                match content_watch::request_from_sources(world, &root) {
                    SourcesAnswer::Requested => {
                        info!("{names:?} changed with the content; they publish at its activation")
                    }
                    // A route is not ready: the pair waits, and the content
                    // is asked for again at the next look.
                    SourcesAnswer::Waiting => {}
                    // The content is what runs: the scripts are a change of
                    // their own.
                    SourcesAnswer::Unchanged => {
                        world.remove_resource::<PairedModuleCandidate>();
                        ext::propose_staged_modules(world);
                    }
                    SourcesAnswer::Refused => {
                        world.remove_resource::<PairedModuleCandidate>();
                        ext::discard_paired_modules(world);
                        error!("{names:?} changed with content that is refused; the running pair stays");
                    }
                }
            }
        },
    }
}

/// A module change on its own: its own road, the mechanical-edit protocol.
fn stage_alone(world: &mut World, changes: ext::ModuleFileChanges) {
    match ext::stage_module_file_changes(world, changes) {
        Ok(names) => {
            info!("{names:?} changed; reload proposed");
            ext::propose_staged_modules(world);
        }
        Err(reason) => error!("changed modules are NOT loaded; the running code stays: {reason}"),
    }
}
