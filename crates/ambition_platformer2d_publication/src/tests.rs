//! The room publication transaction's own arms: verification and
//! application answer about the exact target they were given.

use ambition_platformer2d_core as ae;
// The room vocabulary these fixtures build is the world crate's.
use ambition_platformer2d_world::rooms::*;

fn empty_world(name: &str) -> ae::World {
    ae::World::new(
        name,
        ae::Vec2::new(640.0, 480.0),
        ae::Vec2::new(96.0, 96.0),
        Vec::new(),
    )
}

fn spec_with(meta: RoomMetadata, id: &str) -> RoomSpec {
    RoomSpec {
        id: id.into(),
        world: empty_world(id),
        loading_zones: Vec::new(),
        metadata: meta,
        camera_zones: Vec::new(),
        kinematic_paths: Vec::new(),
        moving_platforms: Vec::new(),
        props: Vec::new(),
        ground_items: Vec::new(),
        portal_gun_spawns: Vec::new(),
        shrines: Vec::new(),
        gravity_zones: Vec::new(),
        enemy_spawns: Vec::new(),
        boss_spawns: Vec::new(),
        debug_labels: Vec::new(),
        mount_links: Vec::new(),
        boss_successions: Vec::new(),
        placements: Vec::new(),
        encounter_triggers: Vec::new(),
        lock_walls: Vec::new(),
        switch_commands: Vec::new(),
    }
}


/// A root that holds a world: `rooms` plus the geometry a session collides
/// against.
/// A session root holding one room, in its own scope, with its live room's
/// geometry on that room's own root.
fn a_root_holding(world: &mut bevy::prelude::World, room: &str) -> bevy::prelude::Entity {
    use ambition_platformer2d_shared_tangle::lifecycle::{activation_room_root, SessionRoot, SessionScopeId};
    let scope = SessionScopeId(
        world
            .query::<&SessionRoot>()
            .iter(world)
            .map(|root| root.0 .0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    let set = RoomSet::from_parts_or_panic(room, vec![spec_with(RoomMetadata::default(), room)], Vec::new());
    world.spawn((
        activation_room_root(scope),
        ambition_platformer2d_core::RoomGeometry(empty_world(room)),
    ));
    world.spawn((SessionRoot(scope), set)).id()
}

/// ⛔⛤ **A REFUSED APPLICATION MUST NOT HAVE DESTROYED THE OUTGOING WORLD
/// FIRST.** `apply_world_replacement` despawned the whole outgoing roster and
/// only THEN discovered whether the target still carried the sinks it needed,
/// logging an error and carrying on — so the one case those arms exist for left
/// the session with the old world destroyed and the new one not installed.
///
/// Review finding, 2026-09-18. It is the same shape
/// `install_rebased_sync_test_session` was corrected for on 2026-09-17 and
/// `maintain_local_session` on 2026-09-18: *"everything that can decline runs
/// while the old session is still alive; the stop is the first irreversible act
/// and nothing after it may fail."*
///
/// ⭐ **THE CONTROL IS A COMPLETE TARGET AND IT IS THE HALF THAT CAN GO WRONG
/// QUIETLY.** A preflight that refuses EVERYTHING also leaves the roster
/// standing, and would satisfy the subject below while breaking every room
/// transition in the game.
///
/// ⚠ The roster is the observable, not the room: `verify_staged_world` is what
/// normally makes this unreachable, so the subject here is what happens when the
/// world changed between verification and application — which is exactly when
/// the old world is the only one that exists.
#[test]
fn a_replacement_refused_at_application_leaves_the_outgoing_world_standing() {
    use ambition_platformer2d_shared_tangle::lifecycle::SessionRoot;

    /// A root carrying `SessionRoot` plus whichever sinks `with_geometry` asks
    /// for, and one outgoing body for the roster.
    fn world_with(with_geometry: bool) -> (bevy::prelude::App, bevy::prelude::Entity, bevy::prelude::Entity) {
        let mut app = bevy::prelude::App::new();
        let world = app.world_mut();
        let scope = ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(1);
        // The live room's root stands either way; only its geometry is missing.
        let mut room_root = world.spawn(ambition_platformer2d_shared_tangle::lifecycle::activation_room_root(scope));
        if with_geometry {
            room_root.insert(ambition_platformer2d_core::RoomGeometry(empty_world("r")));
        }
        let root = world
            .spawn((
                SessionRoot(scope),
                RoomSet::from_parts_or_panic("r", vec![spec_with(RoomMetadata::default(), "r")], Vec::new()),
            ))
            .id();
        let outgoing = world.spawn_empty().id();
        (app, root, outgoing)
    }

    fn replacement(outgoing: bevy::prelude::Entity) -> super::transaction::PendingWorldReplacement {
        super::transaction::PendingWorldReplacement::new(
            vec![(outgoing, false)],
            None,
            0,
            empty_world("r"),
            Vec::new(),
        )
        .replacing(Some(super::transaction::LiveRoomSuccession::replacing(ambition_platformer2d_world::rooms::LiveRoomInstance::ACTIVATION, ambition_platformer2d_world::rooms::LiveRoomInstance::ACTIVATION.next())))
    }

    // ── THE CONTROL: a complete target. The roster MUST be swept here, or the
    //    subject below is satisfied by a preflight that refuses everything.
    let (mut app, root, outgoing) = world_with(true);
    super::transaction::apply_world_replacement(app.world_mut(), replacement(outgoing), Some(root));
    assert!(
        app.world().get_entity(outgoing).is_err(),
        "a complete publication left the outgoing roster alive, so this preflight \
         refuses valid applications and every room transition is broken"
    );

    // ── THE SUBJECT: the target lost a sink between verification and here.
    let (mut app, root, outgoing) = world_with(false);
    super::transaction::apply_world_replacement(app.world_mut(), replacement(outgoing), Some(root));
    assert!(
        app.world().get_entity(outgoing).is_ok(),
        "the application was refused for a missing `RoomGeometry` AFTER it had \
         already despawned the outgoing world. The session now has neither the \
         room it was standing in nor the one it was moving to"
    );
}

fn a_replacement_naming(room: &str) -> super::transaction::PendingWorldReplacement {
    super::transaction::PendingWorldReplacement::new(
        Vec::new(),
        // `None`: this transaction walks within the set the target already has,
        // which is what makes the target's OWN `RoomSet` the thing consulted.
        None,
        0,
        empty_world(room),
        Vec::new(),
    )
    .replacing(Some(super::transaction::LiveRoomSuccession::replacing(ambition_platformer2d_world::rooms::LiveRoomInstance::ACTIVATION, ambition_platformer2d_world::rooms::LiveRoomInstance::ACTIVATION.next())))
}

/// ⛔⛤ **VERIFICATION ANSWERS ABOUT THE ROOT IT WAS GIVEN, NOT THE LIVE ONE —
/// 2026-09-15 AUDIT, FINDING 5.**
///
/// `verify_and_publish` resolves its target by the transaction's own session
/// scope, precisely so a hidden candidate session B can be validated while A is
/// live. `apply_world_replacement` then threw that identity away and re-asked
/// `session_world_component_mut` — *"which root is live right now"* — so a
/// publication could be validated for B and applied to A. The fix threads one
/// target through both ends; this arm is what says verification's end of it is
/// really about the entity handed in.
///
/// ⭐ TWO ROOTS, ONE CALL EACH, AND THE ONLY DIFFERENCE IS THE TARGET. A world
/// with one root cannot fail this assertion for the right reason.
#[test]
fn a_staged_world_is_verified_against_the_exact_target_it_was_given() {
    let mut app = bevy::prelude::App::new();
    let world = app.world_mut();
    let live = a_root_holding(world, "the_live_room");
    let candidate = a_root_holding(world, "the_candidate_room");

    let staged = a_replacement_naming("the_candidate_room");

    // ⭐ THE CONTROL, FIRST: this staged world is genuinely wrong for the OTHER
    // root, so an `Ok` below is a statement about which root was consulted
    // rather than about a check that accepts everything.
    let against_live = super::transaction::verify_staged_world(world, &staged, Some(live));
    assert!(
        against_live.as_ref().err().is_some_and(|violations| {
            violations.iter().any(|violation| {
                matches!(
                    violation,
                    super::transaction::StagedWorldViolation::GeometryIsNotTheTargetRoom { .. }
                )
            })
        }),
        "the staged world was ACCEPTED against the live root too, so it is not \
         discriminating between the two and the assertion below would pass for \
         any implementation: {against_live:?}"
    );

    // ⛔ THE SUBJECT. Poisoned — `verify_staged_world` reading the first `RoomSet`
    // in the world instead of the target's — this is the line that fails, and the
    // control above still passes. (The first version of this arm had the two the
    // other way round, and the poison fired on the control.)
    let against_candidate =
        super::transaction::verify_staged_world(world, &staged, Some(candidate));
    assert!(
        against_candidate.is_ok(),
        "⛔ VERIFICATION DID NOT CONSULT THE ROOT IT WAS GIVEN. A staged world \
         naming the CANDIDATE's own room was refused against the CANDIDATE's own \
         root, so `publishing_into` decides nothing and the verifier is answering \
         about some other session: {against_candidate:?}"
    );
}

/// ⛔⛤ **EVERY SINK IS PREFLIGHTED ON THE EXACT TARGET — FINDING 5's OTHER
/// HALF.** `verify_staged_world` accepted a staged room set as evidence that a
/// room set EXISTED, which answers a question about the value rather than about
/// the place it is going; `RoomGeometry` was not asked about at all. Application
/// then wrote through `if let Some(..)` and published a SUCCESS verdict having
/// silently skipped whatever was missing — a session seated in one room while
/// colliding against another.
#[test]
fn a_staged_world_is_refused_when_its_target_carries_no_sink_for_it() {
    use super::transaction::StagedWorldViolation;

    let mut app = bevy::prelude::App::new();
    let world = app.world_mut();

    // A root with the geometry and no room set.
    let no_rooms = world
        .spawn(ambition_platformer2d_core::RoomGeometry(empty_world("r")))
        .id();
    // A root with the room set, whose live room's root has no geometry.
    let scope = ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(9);
    world.spawn(ambition_platformer2d_shared_tangle::lifecycle::activation_room_root(scope));
    let no_geometry = world
        .spawn((
            ambition_platformer2d_shared_tangle::lifecycle::SessionRoot(scope),
            RoomSet::from_parts_or_panic(
                "r",
                vec![spec_with(RoomMetadata::default(), "r")],
                Vec::new(),
            ),
        ))
        .id();

    let staged = a_replacement_naming("r");

    let missing_rooms =
        super::transaction::verify_staged_world(world, &staged, Some(no_rooms)).unwrap_err();
    assert!(
        missing_rooms
            .iter()
            .any(|violation| matches!(violation, StagedWorldViolation::NoRoomSetToPublishInto)),
        "⛔ A STAGED WORLD PASSED PREFLIGHT INTO A TARGET WITH NO `RoomSet`. \
         Publishing would write the geometry and leave the active room where it \
         was: {missing_rooms:?}"
    );

    let missing_geometry =
        super::transaction::verify_staged_world(world, &staged, Some(no_geometry)).unwrap_err();
    assert!(
        missing_geometry.iter().any(|violation| matches!(
            violation,
            StagedWorldViolation::NoRoomGeometryToPublishInto
        )),
        "⛔ A STAGED WORLD PASSED PREFLIGHT INTO A TARGET WITH NO `RoomGeometry`. \
         Publishing would seat the session in the new room while it collides \
         against the old one: {missing_geometry:?}"
    );
}
