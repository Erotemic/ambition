use super::*;

use ambition_characters::actor::BodyHealth;
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core::{BodyBaseSize, BodyKinematics, BodyMana, Vec2};
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse;

use crate::shrine::{heal_save_shrine_system, HealShrine};

/// A save point that actually saves, and a session that resumes there.
///
/// The shrine claimed both halves and delivered neither: it called
/// `save.set_changed()` on a value it never modified — which the value-comparing
/// autosave correctly ignores — and there was no checkpoint field to write into
/// even if the marker had worked. It healed, logged
/// "healed to full + saved", and persisted nothing.
///
/// Both halves in one test on purpose. Either alone is worthless: a checkpoint
/// nothing records is a lie, and a checkpoint nothing restores is a number in a
/// file.
#[test]
fn resting_at_a_shrine_records_a_checkpoint_and_the_next_session_resumes_there() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };

    fn room_set(room_id: &str) -> ambition_platformer2d_world::rooms::RoomSet {
        let world = ambition_platformer2d_core::World::new(
            "Shrine Room",
            Vec2::new(640.0, 480.0),
            Vec2::new(32.0, 400.0),
            vec![ambition_platformer2d_core::Block::solid(
                "floor",
                Vec2::new(0.0, 440.0),
                Vec2::new(640.0, 40.0),
            )],
        );
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            room_id,
            vec![ambition_platformer2d_world::rooms::RoomSpec::new(
                room_id, world,
            )],
            Vec::new(),
        )
    }

    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.init_resource::<ShrineActivationPulse>();
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    insert_session_world_component(app.world_mut(), room_set("shrine_room"));
    app.add_systems(Update, heal_save_shrine_system);

    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            ActorControl::default(),
            BodyKinematics {
                pos: Vec2::new(412.0, 396.0),
                vel: Vec2::ZERO,
                size: Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
            BodyBaseSize {
                base_size: Vec2::new(24.0, 40.0),
            },
            BodyHealth::new(ambition_characters::actor::Health {
                current: 1,
                max: 5,
                invulnerable: Default::default(),
            }),
            BodyMana::default(),
        ))
        .id();
    app.world_mut().spawn(HealShrine {
        pos: Vec2::new(412.0, 396.0),
        half_extent: Vec2::new(22.0, 40.0),
    });
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .interact_pressed = true;
    app.update();

    let recorded = app
        .world()
        .resource::<ambition_persistence::save::AmbitionGameSave>()
        .data()
        .checkpoint()
        .cloned()
        .expect(
            "resting at a shrine recorded no checkpoint, so the save point saves \
             nothing — which is what it did while logging that it had",
        );
    assert_eq!(recorded.room_id, "shrine_room");
    assert_eq!((recorded.x, recorded.y), (412, 396));

    // ── The next session, from the same save ─────────────────────────────────
    //
    // A fresh app with the recorded save: the body starts at the room's authored
    // spawn and must be moved to the checkpoint instead.
    let mut next = App::new();
    next.insert_resource(ambition_persistence::save::AmbitionGameSave(
        app.world()
            .resource::<ambition_persistence::save::AmbitionGameSave>()
            .data()
            .clone(),
    ));
    next.init_resource::<ActiveSessionScope>();
    next.world_mut()
        .resource_mut::<ActiveSessionScope>()
        .begin();
    insert_session_world_component(next.world_mut(), room_set("shrine_room"));
    next.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
    next.init_resource::<CheckpointResumeProgress>();
    next.add_systems(Update, restore_checkpoint_on_session_start);
    // The REAL player bundle, at the room's authored spawn — the body the
    // construction path produces, with every cluster the transit authority reads.
    let resumed = next
        .world_mut()
        .spawn((crate::avatar::PlayerSimulationBundle::from_scratch(
            crate::avatar::primary_player_scratch(
                ambition_platformer2d_core::Vec2::new(32.0, 400.0),
                ambition_platformer2d_core::AbilitySet::default(),
            ),
            ambition_characters::actor::Health::new(5),
        ),))
        .id();
    next.update();

    let pos = next.world().get::<BodyKinematics>(resumed).unwrap().pos;
    assert_eq!(
        (pos.x, pos.y),
        (412.0, 396.0),
        "the session opened in the checkpoint's room and left the body at the \
         authored spawn — a checkpoint nothing restores is a number in a file"
    );
}
/// A checkpoint recorded in ANOTHER room must not be applied here.
#[test]
fn a_checkpoint_from_another_room_leaves_the_body_where_it_spawned() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };

    let mut app = App::new();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.set_checkpoint(ambition_persistence::save_data::PersistedCheckpoint::new(
        "somewhere_else",
        999,
        999,
    ));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave(save));
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    let world = ambition_platformer2d_core::World::new(
        "Here",
        Vec2::new(640.0, 480.0),
        Vec2::new(32.0, 400.0),
        vec![],
    );
    insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "here",
            vec![ambition_platformer2d_world::rooms::RoomSpec::new(
                "here", world,
            )],
            Vec::new(),
        ),
    );
    // The slot a transition is recorded into: production initializes it in sim-core resources,
    // so a fixture running this system owes it too.
    app.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
    app.init_resource::<CheckpointResumeProgress>();
    app.add_systems(Update, restore_checkpoint_on_session_start);
    let body = app
        .world_mut()
        .spawn((crate::avatar::PlayerSimulationBundle::from_scratch(
            crate::avatar::primary_player_scratch(
                ambition_platformer2d_core::Vec2::new(32.0, 400.0),
                ambition_platformer2d_core::AbilitySet::default(),
            ),
            ambition_characters::actor::Health::new(5),
        ),))
        .id();
    app.update();

    let pos = app.world().get::<BodyKinematics>(body).unwrap().pos;
    assert_eq!(
        (pos.x, pos.y),
        (32.0, 400.0),
        "a checkpoint from another room was applied to this one"
    );
}
/// A checkpoint in ANOTHER room of this world routes the session to it.
///
/// Distinct from `a_checkpoint_from_another_room_leaves_the_body_where_it_spawned`,
/// which covers a room this world does NOT contain. Refusing to teleport a body
/// into coordinates from a room that does not exist is right; refusing to OPEN a
/// room that does is the gap.
#[test]
fn a_checkpoint_in_another_room_of_this_world_routes_the_session_there() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };

    let mut app = App::new();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.set_checkpoint(ambition_persistence::save_data::PersistedCheckpoint::new(
        "rest_room",
        512,
        300,
    ));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave(save));
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();

    let room = |name: &str| {
        ambition_platformer2d_world::rooms::RoomSpec::new(
            name,
            ambition_platformer2d_core::World::new(
                name,
                Vec2::new(640.0, 480.0),
                Vec2::new(32.0, 400.0),
                vec![],
            ),
        )
    };
    insert_session_world_component(
        app.world_mut(),
        // Opens in `entry`; the player rested in `rest_room`.
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "entry",
            vec![room("entry"), room("rest_room")],
            Vec::new(),
        ),
    );
    // The `SimId` is what `ensure_sim_id` files a `PrimaryPlayer` under on every host.
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
    ));
    // The slot a transition is recorded into: production initializes it in sim-core resources,
    // so a fixture running this system owes it too.
    app.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
    app.init_resource::<CheckpointResumeProgress>();
    app.add_systems(Update, restore_checkpoint_on_session_start);
    app.update();

    // It wrote a `RoomTransitionRequested` around a synthetic door; the message is gone and so
    // is the invented zone. The intent names its room by AUTHORED ID, which is also what made
    // the index lookup here deletable.
    fn recorded(app: &App) -> Option<crate::session::lifecycle_commit::PendingIntent> {
        app.world()
            .resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>()
            .pending
            .clone()
    }
    let Some(intent) = recorded(&app) else {
        panic!(
            "the session opened in `entry` while the checkpoint is in `rest_room` \
             and no transition was recorded — the player does not resume where \
             they rested"
        );
    };
    // A shrine warp is a CROSSING by a body, never the bodyless
    // `ReconstituteRoom` that v146 added — it names a subject and an arrival.
    let crate::session::lifecycle_commit::LifecycleIntent::Transition(transition) = intent.kind
    else {
        panic!("the shrine recorded a bodyless room reconstitution, not a warp");
    };
    assert_eq!(transition.target_room, "rest_room");
    assert_eq!(
        transition.subject,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
        "the resume asked for a room without saying whose resume it is, so the \
         commit would transit whoever happens to be controlled several frames later"
    );
    assert_eq!(
        (transition.arrival.x, transition.arrival.y),
        (512.0, 300.0),
        "the transition must arrive AT the checkpoint, not at the room's own spawn"
    );

    // Clearing the slot is what makes the question askable at all.
    app.world_mut()
        .resource_mut::<crate::session::lifecycle_commit::PendingLifecycleCommit>()
        .take();
    app.update();
    app.update();
    assert!(
        recorded(&app).is_none(),
        "the resume transition was recorded repeatedly"
    );
}
/// ⛔⛔ **F1: THE ROUTED LATCH BELONGED TO THE ATTEMPT, NOT TO THE ADMISSION.**
///
/// `restore_checkpoint_on_session_start` wrote `routed_for` and then *discarded*
/// the `Admission` the slot returned. A slot already owned by another lifecycle
/// intent refused the crossing while the session recorded that it had spent its
/// one resume — so the player stayed in whichever room the session happened to
/// open in, permanently, because the only road back is gated on a generation
/// that line had already burned.
///
/// ⭐ THE WITNESS IS THE RETRY, NOT THE FLAG. Asserting `routed_for.is_none()`
/// alone would pass on a system that never ran at all; this releases the slot
/// and requires the crossing to actually arrive on a later tick.
#[test]
fn a_refused_slot_leaves_the_checkpoint_resume_retryable() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };
    use crate::session::lifecycle_commit::{
        LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent,
    };

    let mut app = App::new();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.set_checkpoint(ambition_persistence::save_data::PersistedCheckpoint::new(
        "rest_room",
        512,
        300,
    ));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave(save));
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();

    let room = |name: &str| {
        ambition_platformer2d_world::rooms::RoomSpec::new(
            name,
            ambition_platformer2d_core::World::new(
                name,
                Vec2::new(640.0, 480.0),
                Vec2::new(32.0, 400.0),
                vec![],
            ),
        )
    };
    insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "entry",
            vec![room("entry"), room("rest_room")],
            Vec::new(),
        ),
    );
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
    ));
    app.init_resource::<PendingLifecycleCommit>();
    app.init_resource::<CheckpointResumeProgress>();
    app.add_systems(Update, restore_checkpoint_on_session_start);

    // SOMEBODY ELSE ALREADY OWNS THE SLOT. A door crossing recorded on an
    // earlier frame is the ordinary shape of this: the slot is earliest-sticky.
    let incumbent = LifecycleIntent::Transition(RoomTransitionIntent {
        subject: ambition_platformer2d_shared_tangle::sim_id::SimId::placement("someone_else"),
        target_room: "entry".into(),
        arrival: Vec2::new(1.0, 2.0),
        edge_exit: true,
        zone_sfx: None,
    });
    assert!(app
        .world_mut()
        .resource_mut::<PendingLifecycleCommit>()
        .record(3, incumbent.clone())
        .admitted());

    app.update();
    app.update();

    assert_eq!(
        app.world()
            .resource::<PendingLifecycleCommit>()
            .pending
            .as_ref()
            .map(|pending| &pending.kind),
        Some(&incumbent),
        "the refused resume overwrote the incumbent lifecycle operation"
    );
    assert_eq!(
        app.world().resource::<CheckpointResumeProgress>().routed_for,
        None,
        "the resume recorded that it had routed while the slot refused it, so \
         the session can never ask again"
    );

    // THE INCUMBENT COMMITS AND FREES THE SLOT. The resume must now land.
    app.world_mut()
        .resource_mut::<PendingLifecycleCommit>()
        .take();
    app.update();

    let pending = app
        .world()
        .resource::<PendingLifecycleCommit>()
        .pending
        .clone()
        .expect(
            "the slot was released and the checkpoint resume never re-asked — a \
             refusal permanently stranded the player in the session's own room",
        );
    let LifecycleIntent::Transition(transition) = pending.kind else {
        panic!("the resume recorded a bodyless reconstitution, not a crossing");
    };
    assert_eq!(transition.target_room, "rest_room");
    assert_eq!(
        (transition.arrival.x, transition.arrival.y),
        (512.0, 300.0)
    );

    // ⭐ AND IT IS STILL ONCE-ONLY. The latch moved to the admission; it did not
    // disappear. A second release must not produce a second crossing.
    assert_eq!(
        app.world().resource::<CheckpointResumeProgress>().routed_for,
        Some(Some(
            app.world()
                .resource::<ActiveSessionScope>()
                .current()
                .expect("the session was begun")
                .0
        )),
        "an admitted crossing must latch the generation it was spent on"
    );
    app.world_mut()
        .resource_mut::<PendingLifecycleCommit>()
        .take();
    app.update();
    app.update();
    assert!(
        app.world()
            .resource::<PendingLifecycleCommit>()
            .pending
            .is_none(),
        "the admitted resume was recorded a second time, so a session that \
         reaches the slot twice restarts its crossing forever"
    );
}

/// A session whose avatar has not been constructed yet cannot name the body it
/// is resuming — and must not spend the once-per-session route saying so.
///
/// This is the OTHER half of the same rule as the refusal above: `routed_for` is
/// a receipt for an accepted crossing, and the two ways to fail to get one are a
/// busy slot and an unresolvable subject.
#[test]
fn a_resume_with_no_constructed_subject_stays_pending_until_the_body_exists() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };
    use crate::session::lifecycle_commit::{LifecycleIntent, PendingLifecycleCommit};

    let mut app = App::new();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.set_checkpoint(ambition_persistence::save_data::PersistedCheckpoint::new(
        "rest_room",
        512,
        300,
    ));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave(save));
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();

    let room = |name: &str| {
        ambition_platformer2d_world::rooms::RoomSpec::new(
            name,
            ambition_platformer2d_core::World::new(
                name,
                Vec2::new(640.0, 480.0),
                Vec2::new(32.0, 400.0),
                vec![],
            ),
        )
    };
    insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "entry",
            vec![room("entry"), room("rest_room")],
            Vec::new(),
        ),
    );
    app.init_resource::<PendingLifecycleCommit>();
    app.init_resource::<CheckpointResumeProgress>();
    app.add_systems(Update, restore_checkpoint_on_session_start);

    // No body at all: construction has not finished.
    app.update();
    assert!(
        app.world()
            .resource::<PendingLifecycleCommit>()
            .pending
            .is_none(),
        "a crossing was recorded for a body nobody can name"
    );
    assert_eq!(
        app.world().resource::<CheckpointResumeProgress>().routed_for,
        None,
        "the session marked its resume routed before it had a subject"
    );

    // The avatar materializes.
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
    ));
    app.update();

    let pending = app
        .world()
        .resource::<PendingLifecycleCommit>()
        .pending
        .clone()
        .expect("the body arrived and the resume never asked again");
    let LifecycleIntent::Transition(transition) = pending.kind else {
        panic!("the resume recorded a bodyless reconstitution, not a crossing");
    };
    assert_eq!(transition.target_room, "rest_room");
}

/// ⭐ **A CHECKPOINT-ONLY COMPOSITION RESUMES ITS SESSION**, with no held-item
/// plugin, no shrine entity and no item domain of any kind installed.
///
/// ⛔⛔ IT COULD NOT, UNTIL A1b. `CheckpointResumeProgress` was initialized and
/// `restore_checkpoint_on_session_start` was installed by
/// `ItemPickupSimulationPlugin`, so a profile that wanted checkpoints and no
/// hands got a resume that never ran — or, if it initialized the resource
/// itself, a panic-free silence, which is worse.
///
/// ⚠ THE PLUGIN IS THE SUBJECT, not the systems. Every other fixture in this
/// file adds `restore_checkpoint_on_session_start` to `Update` by hand, which
/// measures the function and says nothing about who installs it. This one runs
/// the real sim schedule through the real offer.
#[test]
fn a_checkpoint_only_composition_resumes_without_the_item_domain() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

    let mut app = App::new();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.set_checkpoint(ambition_persistence::save_data::PersistedCheckpoint::new(
        "here", 412, 396,
    ));
    app.insert_resource(ambition_persistence::save::AmbitionGameSave(save));
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "here",
            vec![ambition_platformer2d_world::rooms::RoomSpec::new(
                "here",
                ambition_platformer2d_core::World::new(
                    "Here",
                    Vec2::new(640.0, 480.0),
                    Vec2::new(32.0, 400.0),
                    vec![],
                ),
            )],
            Vec::new(),
        ),
    );
    app.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();

    // ⚠ THE CHANNELS ARE THE HOST'S, not this offer's, and that is unchanged by
    // A1b: `runtime::CheckpointHorizonPlugin` registers the reset horizon's
    // messages and the domain offers contribute systems to its sets. A domain
    // plugin that registered them would be a second owner of the channel. The
    // fixture stands in for the host because `runtime` depends on this crate.
    app.add_message::<ambition_platformer2d_shared_tangle::lifecycle::ResetToCheckpoint>();
    app.add_message::<ambition_combat::events::RoomReplayAdmitted>();

    // THE COMPOSITION UNDER TEST. No `HeldItemSimulationPlugin`, no
    // `ItemPickupSimulationPlugin`, no `HealShrine` anywhere in the world.
    let sim = app.sim_schedule();
    app.add_plugins(super::SessionCheckpointHorizonPlugin);

    let body = app
        .world_mut()
        .spawn((crate::avatar::PlayerSimulationBundle::from_scratch(
            crate::avatar::primary_player_scratch(
                Vec2::new(32.0, 400.0),
                ambition_platformer2d_core::AbilitySet::default(),
            ),
            ambition_characters::actor::Health::new(5),
        ),))
        .id();

    app.world_mut().run_schedule(sim);

    let pos = app.world().get::<BodyKinematics>(body).unwrap().pos;
    assert_eq!(
        (pos.x, pos.y),
        (412.0, 396.0),
        "a composition with checkpoints and no items did not resume at its \
         checkpoint — the offer that installs the resume still depends on the \
         item domain"
    );
}

/// ⛔⛔ **ASK THE SCHEDULE, NOT THE SOURCE.** Every claim below is about a
/// REGISTRATION: which set a system actually joined, and which edges the graph
/// actually holds. A reducer whose body reads `AdmittedCheckpointRestore` but
/// which was installed straight into `CheckpointRestore` would be correct in
/// source and unordered in fact — it could run before the admission that
/// decides whether it may run at all, and would then read a token left over
/// from... nothing, because retire had also not run. Silence, not a crash.
///
/// ⭐ THE POPULATION IS THE POINT, not the three names. The last assertion is
/// that NOTHING sits in `CheckpointRestore` outside the three steps, so a fourth
/// domain that installs its reducer the old way is caught by this test rather
/// than by a player losing an item.
#[test]
fn every_checkpoint_restore_system_is_inside_one_ordered_step() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        CheckpointRestore, CheckpointRestoreStep, LifecycleCheckpointHorizonPlugin,
    };
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;
    use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules, SystemSet};

    fn set_key<S: SystemSet + Copy + std::fmt::Debug>(graph: &ScheduleGraph, set: S) -> NodeId {
        NodeId::Set(
            graph
                .system_sets
                .get_key(set.intern())
                .unwrap_or_else(|| panic!("{set:?} must be a registered SystemSet")),
        )
    }
    fn members<S: SystemSet + Copy + std::fmt::Debug>(graph: &ScheduleGraph, set: S) -> Vec<NodeId> {
        let node = set_key(graph, set);
        graph
            .systems
            .iter()
            .map(|(key, _, _)| NodeId::System(key))
            .filter(|system| graph.hierarchy().graph().contains_edge(node, *system))
            .collect()
    }

    let mut app = App::new();
    // The whole actor-domain contribution plus the lifecycle one: the shape the
    // host composes. `ActorCheckpointHorizonPlugin` brings the session offer
    // (which owns the steps) and the item offer.
    app.add_plugins((
        LifecycleCheckpointHorizonPlugin,
        crate::ActorCheckpointHorizonPlugin,
    ));
    let sim = app.sim_schedule();
    let schedules = app.world().resource::<Schedules>();
    let graph = schedules.get(sim).expect("the sim schedule exists").graph();

    // The chain, and its nesting.
    for pair in [
        (CheckpointRestoreStep::Admit, CheckpointRestoreStep::Apply),
        (CheckpointRestoreStep::Apply, CheckpointRestoreStep::Retire),
    ] {
        assert!(
            graph
                .dependency()
                .graph()
                .contains_edge(set_key(graph, pair.0), set_key(graph, pair.1)),
            "{:?} -> {:?} must be an explicit edge: a domain reducer that can run \
             before the admission is a domain reducer with no answer to read",
            pair.0,
            pair.1
        );
    }
    let restore = set_key(graph, CheckpointRestore);
    for step in [
        CheckpointRestoreStep::Admit,
        CheckpointRestoreStep::Apply,
        CheckpointRestoreStep::Retire,
    ] {
        assert!(
            graph
                .hierarchy()
                .graph()
                .contains_edge(restore, set_key(graph, step)),
            "{step:?} must be inside CheckpointRestore, or the host's \
             `CheckpointRestore.before(RoomReplayAdmission)` edge does not reach it"
        );
    }

    // One admit, one retire, and the three domain reducers between them.
    assert_eq!(
        members(graph, CheckpointRestoreStep::Admit).len(),
        1,
        "the session coordinator is the SOLE admission writer"
    );
    assert_eq!(
        members(graph, CheckpointRestoreStep::Retire).len(),
        1,
        "the session coordinator is the SOLE retiring writer"
    );
    assert_eq!(
        members(graph, CheckpointRestoreStep::Apply).len(),
        3,
        "the occurrence, custody and owned-item reducers are the installed \
         domains of this composition"
    );

    // ⛔ AND NOTHING BYPASSES THE STEPS. A system installed directly into
    // `CheckpointRestore` is a system with no ordering against the admission.
    assert!(
        members(graph, CheckpointRestore).is_empty(),
        "a system joined CheckpointRestore directly instead of one of its three \
         steps, so nothing orders it against the admission that authorizes it"
    );
}

/// ⚠ **THE ADMITTED TOKEN NEVER SURVIVES ITS OWN FRAME**, which is the whole
/// reason it is not rollback state. A rewind cannot catch a value that is empty
/// at every frame boundary; the day application moves to the confirmed commit
/// boundary this stops being true, and this test is what says so out loud.
#[test]
fn the_admitted_restore_never_survives_its_own_frame() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope, AdmittedCheckpointRestore,
        LifecycleCheckpointHorizonPlugin, ResetToCheckpoint,
    };
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

    let mut app = App::new();
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.init_resource::<ActiveSessionScope>();
    app.world_mut().resource_mut::<ActiveSessionScope>().begin();
    insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts(
            "here",
            vec![ambition_platformer2d_world::rooms::RoomSpec::new(
                "here",
                ambition_platformer2d_core::World::new(
                    "Here",
                    Vec2::new(640.0, 480.0),
                    Vec2::new(32.0, 400.0),
                    vec![],
                ),
            )],
            Vec::new(),
        ),
    );
    app.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
    // The host's channels, as above: a domain offer that registered them would
    // be a second owner.
    app.add_message::<ResetToCheckpoint>();
    app.add_message::<ambition_platformer2d_shared_tangle::lifecycle::CheckpointCommitted>();
    app.add_message::<ambition_combat::events::RoomReplayAdmitted>();
    let sim = app.sim_schedule();
    app.add_plugins((
        LifecycleCheckpointHorizonPlugin,
        super::SessionCheckpointHorizonPlugin,
    ));
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
    ));

    app.world_mut().write_message(ResetToCheckpoint);
    app.world_mut().run_schedule(sim);

    // ⛔ THE PREMISE: the reset must actually have been admitted, or an empty
    // token proves only that nothing happened.
    assert!(
        app.world()
            .resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>()
            .peek()
            .is_some(),
        "the reset was never admitted, so the emptiness below measures nothing"
    );
    assert_eq!(
        *app.world().resource::<AdmittedCheckpointRestore>(),
        AdmittedCheckpointRestore::default(),
        "the admitted restore outlived the frame that admitted it. It is not \
         rollback state, so a rewind across this boundary would leave one \
         timeline holding an authorization the other never issued"
    );
}
