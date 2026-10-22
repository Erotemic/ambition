//! A session's first room: published into a session root that may still be a
//! hidden candidate, before any body of the session is built.
//!
//! Session setup (`crate::session::setup`) builds the home body after this; the
//! caller that activates the session runs both and decides from the receipt.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;
use ambition_platformer2d_world::rooms::RoomSet;

/// What a session's first room is built and published from.
pub struct FirstRoom<'a> {
    /// ⛔ THE SESSION ROOT THIS SETUP IS BUILDING INTO. This publishes the
    /// session's content generation ON it (see `ActiveContentBinding`), so it
    /// must be handed the entity rather than looking up *"the live root"* — at
    /// activation the live root is the OUTGOING session's, or none at all.
    pub session_root: bevy::prelude::Entity,
    /// What happens to the first room's PUBLICATION RECEIPT.
    ///
    /// ⛔ A caller that will ask `publication_succeeded` — the shell provider,
    /// whose candidate session becomes authoritative only if this room published
    /// — says `UntilOwnerRetires` and owes a `retire_publication`. A direct-entry
    /// demo drops the handle and says `UntilTheVerdictIsRecorded`.
    pub retention: super::transaction::PublicationRetention,
    /// Where the first room's commit gets what the save says about its bodies.
    ///
    /// ⛔ The shell provider states the save of the session's own experience:
    /// its session is built while another one can be live. A direct-entry demo
    /// has one experience and says `TheWorldAtTheCommit`.
    pub facts: super::transaction::CommitFactsSource,
    pub room_set: &'a RoomSet,
    /// App-local hostile archetype definitions used by authored room lowering.
    /// The installed App-local placement-lowering authority. The first
    /// room's authored placements through THIS registry — the same one
    /// room transition and snapshot restore consume — so there is no
    /// setup-only reconstruction of the six built-in interpreters.
    pub placement_lowering: &'a crate::construction::placements::PlacementLoweringRegistry,
    /// The App-installed room-content staging seam. The first room drains its registered content stagers exactly as transition, reset,
    /// hot-reload, and restore staging do — one construction authority.
    pub content_staging: &'a crate::features::RoomContentStagingRegistry,
    /// The App-installed construction recipe table plus the content generation
    /// this session was prepared under (Phase 3 planned families).
    pub construction: crate::features::ActorConstructionContext<'a>,
    /// App-local boss profiles, encounter specs, sheets, and special rows.
    pub boss_catalog: &'a ambition_boss_encounter::BossCatalog,
}

/// Publish the first room of a session into `first.session_root`, and return
/// its receipt.
pub fn publish_first_room(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    first: FirstRoom<'_>,
) -> super::transaction::PublicationHandle {
    let FirstRoom {
        session_root,
        retention,
        facts,
        room_set,
        placement_lowering,
        content_staging,
        construction,
        boss_catalog,
    } = first;
    for warning in room_set.layout_warnings() {
        bevy::log::debug!(target: "ambition_platformer2d::room_layout", "{warning}");
    }
    // The session's content generation, published for the commit boundary:
    // every later room transaction (transition, reset, reconstruction) must be
    // prepared against THIS binding or be refused publication as stale.
    commands
        .entity(session_root)
        .insert(super::transaction::ActiveContentBinding(
            construction.binding,
        ));
    // The first room is the session's activation room: nothing has been
    // published into this session yet, so its occupants, and the home body
    // that starts in it, are in instance #0.
    let session_scope = session_scope.in_room(Some(
        ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION,
    ));
    let room_plan = super::RoomConstructionPlan::prepare_from_parts(
        room_set,
        room_set.activation(),
        placement_lowering,
        content_staging,
        boss_catalog,
        session_scope,
        construction,
    )
    .unwrap_or_else(|error| panic!("initial room construction failed: {error}"));
    // ⛔⛤ **THE FIRST ROOM'S RECEIPT IS THE ACTIVATION DECISION.** A candidate
    // session becomes authoritative only if the room it was built around
    // published, so the caller that owns that decision retains this handle. See
    // `FirstRoom::retention`.
    let publication = room_plan.spawn_contents(commands, retention, facts);
    // The first room's moving platforms go onto THIS session's live room root,
    // hidden with it when the session is a candidate. A candidate cannot write
    // the playing session's platforms, because they are on another root.
    // (Until OW1 cut 3b they were one process resource, so a candidate carried
    // them out as data and its adoption installed them.)
    let moving_platforms = ambition_platformer2d_world::collision::MovingPlatformSet(
        room_plan.platform_states().to_vec(),
    );
    let scope_id = session_scope.id();
    commands.queue(move |world: &mut bevy::prelude::World| {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            live_room_root_for, sole_live_room_entity, LiveRoomInstance,
        };
        let root = match scope_id {
            Some(scope) => live_room_root_for(world, scope, LiveRoomInstance::ACTIVATION),
            None => sole_live_room_entity(world),
        };
        match root {
            Some(root) => {
                world.entity_mut(root).insert(moving_platforms);
            }
            None => bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "session {scope_id:?} has no live room root for its first room's \
                 moving platforms"
            ),
        }
    });
    publication
}
