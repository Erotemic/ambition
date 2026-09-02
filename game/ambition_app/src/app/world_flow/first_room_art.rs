//! The first room's art arrives BEFORE the route activates.
//!
//! A shell-hosted session activates when its preparation barrier completes,
//! and `room-loaded` fires the same frame the home body spawns. The body's
//! worn sheet was demanded in that frame and decoded after it — on the host,
//! 7.6 MP of the player's own sheet 0.15 s after every first `room-loaded`, a
//! 67-79 ms frame (`desktop-timeline-run-20260902T0155/0159/0205Z`), and a
//! placeholder rectangle where the player stands until it lands. The direct
//! `--start-room` host has a curtain for exactly this (`startup_loading`);
//! the shipped route had nothing, because its load lifecycle ends at
//! "prepared", and prepared meant validated, not decoded.
//!
//! This is the host's answer to the plan's `prepare-first-room-art` work
//! item: read the PUBLISHED session (start room, staged cast, starting
//! character), demand its art at the room's tier the way a covered room
//! transition does, and complete the item when every page is decoded — and
//! uploaded, when a render world owes it. The barrier holds the load
//! foreground meanwhile, with the item's own progress on it.

use std::collections::BTreeMap;
use std::sync::Arc;

use bevy::prelude::{MessageWriter, Res, ResMut, Resource};

use ambition_platformer2d::actors::features::RoomContentStagingRegistry;
use ambition_platformer2d::asset_manager::image_stages::RenderWorldPresent;
use ambition_platformer2d::game_shell::PREPARE_FIRST_ROOM_ART_WORK_ID;
use ambition_platformer2d::load::{
    LoadCommand, LoadFailure, LoadId, LoadWorkId, LoadWorkState, UnitProgress,
};
use ambition_platformer2d::provider::PreparedPlatformerSessions;
use ambition_platformer2d::world::rooms::RoomSpec;

use super::room_transition_assets::{
    build_loaded_room_asset_manifest, build_room_asset_manifest, inspect_demanded_characters,
    inspect_room_asset_manifest, realized_character_count, room_character_tokens,
    RoomAssetManifest, RoomTransitionAssetContext,
};

/// One published session whose first room's art this host is preparing.
struct FirstRoomArtJob {
    room: Arc<RoomSpec>,
    staged_actor_names: Vec<String>,
    /// The room's cast plus the starting character: what the reveal waits to
    /// see REALIZED, not only demanded.
    demanded_characters: Vec<String>,
    manifest: RoomAssetManifest,
    /// Sheets realize one per frame after the first build and each brings pages
    /// the manifest has not seen; rebuilt when this count moves.
    realized_at_build: usize,
    updates: u32,
    /// Updates spent waiting only on GPU uploads, for the completion line.
    waited_on_gpu_updates: u32,
}

/// The jobs in flight, by the transaction that published the session.
#[derive(Resource, Default)]
pub(crate) struct FirstRoomArtJobs {
    by_load: BTreeMap<LoadId, FirstRoomArtJob>,
}

/// Drive `prepare-first-room-art` for every published, not yet activated
/// platformer session.
pub(crate) fn prepare_first_room_art_system(
    sessions: Option<Res<PreparedPlatformerSessions>>,
    content_staging: Option<Res<RoomContentStagingRegistry>>,
    mut jobs: ResMut<FirstRoomArtJobs>,
    mut context: RoomTransitionAssetContext,
    mut commands: MessageWriter<LoadCommand>,
) {
    let Some(sessions) = sessions else {
        return;
    };
    // A record that left the store — activated, retried, or cancelled — takes
    // its job with it; the item's state belongs to a barrier that is gone.
    let live: std::collections::BTreeSet<LoadId> = sessions
        .published()
        .map(|(transaction, _)| transaction.barrier.load_id.clone())
        .collect();
    jobs.by_load.retain(|load_id, _| live.contains(load_id));

    let (
        Some(assets),
        Some(catalog),
        Some(character_catalog),
        Some(asset_server),
        Some(layouts),
        Some(quality),
        Some(states),
    ) = (
        context.assets.as_deref_mut(),
        context.catalog.as_deref(),
        context.character_catalog.as_deref(),
        context.asset_server.as_deref(),
        context.layouts.as_deref_mut(),
        context.quality.as_deref(),
        context.character_load_states.as_deref_mut(),
    )
    else {
        return;
    };
    let empty_registry = Default::default();
    let registry = context
        .prepared_characters
        .as_deref()
        .unwrap_or(&empty_registry);

    for (transaction, prepared) in sessions.published() {
        let load_id = transaction.barrier.load_id.clone();
        let work_id = LoadWorkId::new(PREPARE_FIRST_ROOM_ART_WORK_ID);
        let source = prepared.content.source();
        let job = match jobs.by_load.get_mut(&load_id) {
            Some(job) => job,
            None => {
                let room = source.room_set().active_spec();
                let mut staged_actor_names: Vec<String> = match content_staging
                    .as_deref()
                    .map(|staging| staging.try_requests_for(room))
                {
                    Some(Ok(requests)) => {
                        requests.into_iter().map(|request| request.name).collect()
                    }
                    Some(Err(error)) => {
                        commands.write(LoadCommand::SetWorkState {
                            load_id: load_id.clone(),
                            work_id,
                            state: LoadWorkState::Failed(
                                LoadFailure::new(
                                    "The first room's cast could not be staged",
                                    format!("room '{}': content staging failed: {error}", room.id),
                                )
                                .retryable(false),
                            ),
                        });
                        continue;
                    }
                    None => Vec::new(),
                };
                // The home body wears the starting character into the room; the
                // room's placements never list the player, and pre-activation
                // there is no body whose `WornCharacter` would ask. Folded into
                // the staged names so ONE list demands it, waits for its
                // realization AND puts its pages in the manifest (a transition
                // re-demands a worn sheet only when it retired one).
                let worn: Vec<String> = match source.initial_body() {
                    ambition_platformer2d::actors::avatar::InitialBodyPolicy::SpawnCharacter(
                        starting,
                    ) => vec![starting
                        .effective_id(&prepared.report.starting_character)
                        .to_string()],
                    ambition_platformer2d::actors::avatar::InitialBodyPolicy::NoInitialBody => {
                        Vec::new()
                    }
                };
                staged_actor_names.extend(worn.iter().cloned());
                staged_actor_names.sort();
                staged_actor_names.dedup();
                let (manifest, remainder) = build_room_asset_manifest(
                    room,
                    &staged_actor_names,
                    assets,
                    catalog,
                    character_catalog,
                    asset_server,
                    layouts,
                    quality,
                    states,
                    registry,
                    &context.authored_sheets,
                    context.boss_catalog.as_deref(),
                    &worn,
                    true,
                );
                // Beyond the per-frame ration, to the engine's global demand at
                // this room's floor — the same hand-over a room transition makes.
                if let Some(demand) = context.character_load_demand.as_deref_mut() {
                    remainder.forward_into(demand);
                }
                let demanded_characters = room_character_tokens(room, &staged_actor_names);
                let realized_at_build = realized_character_count(&demanded_characters, assets);
                jobs.by_load
                    .entry(load_id.clone())
                    .or_insert(FirstRoomArtJob {
                        room: Arc::new(room.clone()),
                        staged_actor_names,
                        demanded_characters,
                        manifest,
                        realized_at_build,
                        updates: 0,
                        waited_on_gpu_updates: 0,
                    })
            }
        };
        job.updates = job.updates.saturating_add(1);
        let realized = realized_character_count(&job.demanded_characters, assets);
        if realized != job.realized_at_build {
            job.manifest =
                build_loaded_room_asset_manifest(&job.room, &job.staged_actor_names, assets);
            job.realized_at_build = realized;
        }
        let mut readiness = inspect_room_asset_manifest(
            asset_server,
            context.images.as_deref(),
            RenderWorldPresent::from_option(context.render_world.as_deref()),
            &job.manifest,
        );
        inspect_demanded_characters(
            &job.demanded_characters,
            assets,
            Some(&*states),
            &mut readiness,
        );

        let state = if !readiness.failed.is_empty() {
            LoadWorkState::Failed(
                LoadFailure::new(
                    "A required asset for the first room failed to load",
                    format!(
                        "room '{}': failed {}",
                        job.room.id,
                        readiness.failed.join(", ")
                    ),
                )
                .retryable(false),
            )
        } else if readiness.is_ready() {
            eprintln!(
                "[first-room-art] room '{}' ready after {} updates ({} of them waiting only on \
                 GPU uploads): {} assets, {} characters",
                job.room.id,
                job.updates,
                job.waited_on_gpu_updates,
                readiness.total,
                job.demanded_characters.len(),
            );
            LoadWorkState::Complete
        } else {
            if readiness
                .pending
                .iter()
                .all(|label| label.ends_with("(gpu upload)"))
            {
                job.waited_on_gpu_updates = job.waited_on_gpu_updates.saturating_add(1);
            }
            LoadWorkState::Running {
                progress: Some(UnitProgress::new(
                    readiness.settled as f32,
                    readiness.total.max(1) as f32,
                )),
            }
        };
        let done = state.is_complete() || matches!(state, LoadWorkState::Failed(_));
        commands.write(LoadCommand::SetWorkState {
            load_id: load_id.clone(),
            work_id,
            state,
        });
        if done {
            jobs.by_load.remove(&load_id);
        }
    }
}
