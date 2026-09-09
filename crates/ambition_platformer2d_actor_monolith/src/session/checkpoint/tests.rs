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
    next.init_resource::<SessionStartupResume>();
    // The operation state the startup road now shares with the reset road.
    next.init_resource::<AcceptedCheckpointRestore>();
    next.init_resource::<SessionCheckpointOperations>();
    next.init_resource::<SessionCheckpointOutcomes>();
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
    app.init_resource::<SessionStartupResume>();
    // The operation state the startup road now shares with the reset road.
    app.init_resource::<AcceptedCheckpointRestore>();
    app.init_resource::<SessionCheckpointOperations>();
    app.init_resource::<SessionCheckpointOutcomes>();
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
    app.init_resource::<SessionStartupResume>();
    // The operation state the startup road now shares with the reset road.
    app.init_resource::<AcceptedCheckpointRestore>();
    app.init_resource::<SessionCheckpointOperations>();
    app.init_resource::<SessionCheckpointOutcomes>();
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
    app.init_resource::<SessionStartupResume>();
    // The operation state the startup road now shares with the reset road.
    app.init_resource::<AcceptedCheckpointRestore>();
    app.init_resource::<SessionCheckpointOperations>();
    app.init_resource::<SessionCheckpointOutcomes>();
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
    assert!(
        app.world()
            .resource::<SessionStartupResume>()
            .state_for(None)
            .is_none(),
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
    let generation = app
        .world()
        .resource::<ActiveSessionScope>()
        .current()
        .expect("the session was begun")
        .0;
    assert!(
        matches!(
            app.world()
                .resource::<SessionStartupResume>()
                .state_for(Some(generation)),
            Some(StartupResume::Routed(_))
        ),
        "an admitted crossing must name the operation it was spent on — a \
         generation latch cannot tell whether the crossing that committed was \
         this one"
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
    app.init_resource::<SessionStartupResume>();
    // The operation state the startup road now shares with the reset road.
    app.init_resource::<AcceptedCheckpointRestore>();
    app.init_resource::<SessionCheckpointOperations>();
    app.init_resource::<SessionCheckpointOutcomes>();
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
    assert!(
        app.world()
            .resource::<SessionStartupResume>()
            .state_for(None)
            .is_none(),
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

/// ⛔⛔ **NOTHING IN THE SIMULATION'S RESTORE SET MUTATES A DOMAIN ANY MORE.**
///
/// This is the invariant A1c/3b exists for, and it is a claim about WHERE
/// systems are REGISTERED, not about what their bodies say. Occurrence, custody
/// and entitlement application used to run in `CheckpointRestore` — ordinary
/// speculative simulation — so an unconfirmed reset on a rollback host
/// destructively restored ledgers and despawned objects on a frame that could be
/// rewound. They run from the commit executor's schedule now, which only a
/// committed, authorized transition reaches.
///
/// ⭐ THE POPULATION IS THE POINT. Asserting the three names would pass while a
/// fourth domain quietly re-added itself to the simulation set; this asserts
/// that the restore set holds ONLY the session's own systems, and that the
/// domain schedule holds the reducers.
#[test]
fn domain_restoration_is_registered_in_the_commit_schedule_and_not_in_the_simulation() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        CheckpointDomainApply, CheckpointRestore, LifecycleCheckpointHorizonPlugin,
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

    let mut app = App::new();
    app.add_plugins((
        LifecycleCheckpointHorizonPlugin,
        crate::ActorCheckpointHorizonPlugin,
    ));
    let sim = app.sim_schedule();
    let schedules = app.world().resource::<Schedules>();

    let sim_graph = schedules.get(sim).expect("the sim schedule exists").graph();
    let restore = set_key(sim_graph, CheckpointRestore);
    let in_simulation = sim_graph
        .systems
        .iter()
        .filter(|(key, _, _)| {
            sim_graph
                .hierarchy()
                .graph()
                .contains_edge(restore, NodeId::System(*key))
        })
        .count();
    assert_eq!(
        in_simulation, 2,
        "`CheckpointRestore` must hold the session's admission and its retirement \
         and NOTHING ELSE. A domain reducer registered here runs on speculative \
         frames, which is how an unconfirmed reset destructively restored ledgers \
         and despawned objects a rewind then had to invent again"
    );

    let apply_graph = schedules
        .get(CheckpointDomainApply)
        .expect("the lifecycle offer installs the domain-apply schedule")
        .graph();
    assert_eq!(
        apply_graph.systems.iter().count(),
        3,
        "the occurrence, entitlement and custody reducers are this composition's \
         installed domains, and all three belong to the commit's schedule"
    );
}

/// ⛔⛔ **THE INSTALLED INPUTS ARE REMOVED ON EVERY PATH, INCLUDING THE ONE THAT
/// APPLIES NOTHING — and the commit applies the operation it was OPENED for, not
/// whichever one happens to be outstanding.**
///
/// Their absence is what makes "a domain reducer cannot act outside an
/// authorized commit" true by construction. Leave a context installed and every
/// later run of that schedule — or any future caller of those systems — becomes
/// an effective restore nobody authorized.
///
/// ⛔ THE KEY IS WHY THE SECOND HALF IS TESTABLE AT ALL. Matching by intent
/// equality, two crossings to one room with one subject and one arrival compare
/// EQUAL, so a transaction opened for the FIRST operation would be served the
/// SECOND's pinned population — a room rebuilt from a checkpoint it is not
/// about. A key names one admission in one session and cannot be produced by
/// resemblance.
#[test]
fn the_commit_applies_the_operation_it_was_opened_for_and_always_removes_its_inputs() {
    use ambition_platformer2d_shared_tangle::lifecycle::CheckpointRestoreInputs;

    use crate::items::pickup::minted_horizon::ItemCheckpointRestoreInputs;
    use crate::session::lifecycle_commit::{LifecycleIntent, RoomTransitionIntent};

    let crossing = |room: &str| {
        LifecycleIntent::Transition(RoomTransitionIntent {
            subject: ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
            target_room: room.into(),
            arrival: Vec2::new(1.0, 2.0),
            edge_exit: false,
            zone_sfx: None,
        })
    };
    let mut operations = super::SessionCheckpointOperations::default();
    let first = operations.admit(None).expect("a fresh counter mints a key");
    let second = operations.admit(None).expect("and another");
    assert_ne!(
        first, second,
        "two admissions in one session must be distinguishable, or the key adds \
         nothing over the intent equality it replaces"
    );

    let mut app = App::new();
    app.add_plugins(ambition_platformer2d_shared_tangle::lifecycle::LifecycleCheckpointHorizonPlugin);
    app.init_resource::<super::AcceptedCheckpointRestore>();

    // ── AN ORDINARY DOOR: no accepted operation, nothing applied ─────────────
    assert!(
        !super::apply_committed_checkpoint_restore(app.world_mut(), first),
        "a commit with no accepted checkpoint operation applied one anyway"
    );
    assert!(!app.world().contains_resource::<CheckpointRestoreInputs>());

    // ── THE SECOND OPERATION IS OUTSTANDING; A TRANSACTION OPENED FOR THE
    //    FIRST MUST NOT BE SERVED IT — and the two intents are IDENTICAL ──────
    app.world_mut()
        .resource_mut::<super::AcceptedCheckpointRestore>()
        .accept(super::AcceptedRestore {
            key: second,
            frame: 3,
            intent: crossing("east"),
            occurrences: Default::default(),
            custody: Default::default(),
            item: Some(ItemCheckpointRestoreInputs {
                minted: Default::default(),
                owned: Default::default(),
            }),
        });
    assert!(
        !super::apply_committed_checkpoint_restore(app.world_mut(), first),
        "a commit opened for an earlier operation was served the outstanding \
         one's pinned population. The two intents are equal by construction here, \
         which is exactly the case intent matching cannot tell apart"
    );
    assert!(!app.world().contains_resource::<CheckpointRestoreInputs>());
    assert!(!app.world().contains_resource::<ItemCheckpointRestoreInputs>());

    // ── ITS OWN OPERATION: applied, and the inputs are gone afterwards ───────
    assert!(
        super::apply_committed_checkpoint_restore(app.world_mut(), second),
        "the accepted operation's own commit did not apply it"
    );
    assert!(
        !app.world().contains_resource::<CheckpointRestoreInputs>(),
        "the lifecycle restore inputs outlived the commit that installed them, so \
         anything that runs that schedule again is an unauthorized restore"
    );
    assert!(
        !app.world().contains_resource::<ItemCheckpointRestoreInputs>(),
        "the item restore inputs outlived the commit that installed them"
    );
}

/// ⛔⛔ **AN OPERATION FROM A RETIRED SESSION CANNOT ACT IN A NEW ONE.**
///
/// The sequence alone is a per-session counter, so a teardown and re-entry mints
/// key 0 again. What separates them is the session ownership stamp the key
/// carries: a load, a commit or a verification holding the old session's key
/// finds no accepted operation, rather than finding the NEW session's first one
/// because two integers matched.
#[test]
fn a_key_from_a_retired_session_matches_nothing_in_the_next_one() {
    use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;

    let mut operations = super::SessionCheckpointOperations::default();
    let old = operations
        .admit(Some(SessionScopeId(1)))
        .expect("the retired session admits one");

    // Teardown and re-entry: a NEW scope, and the counter is deliberately not
    // reset — recycling a live identifier is how a stale load gets authorized.
    let new = operations
        .admit(Some(SessionScopeId(2)))
        .expect("the new session admits one");
    assert_ne!(old, new);

    let mut held = super::AcceptedCheckpointRestore::default();
    held.accept(super::AcceptedRestore {
        key: new,
        frame: 0,
        intent: crate::session::lifecycle_commit::LifecycleIntent::ReconstituteRoom(
            crate::session::lifecycle_commit::RoomReconstitutionIntent {
                target_room: "here".into(),
            },
        ),
        occurrences: Default::default(),
        custody: Default::default(),
        item: None,
    });
    assert!(
        held.inputs_for_key(old).is_none(),
        "an operation admitted by a session that has been torn down matched the \
         new session's accepted restore"
    );
    assert!(held.inputs_for_key(new).is_some());

    // ⛔ AND AN ABSENT SCOPE IS NOT A WILDCARD. A standalone profile's key must
    // not match a real session's, in either direction.
    let standalone = super::SessionCheckpointOperations::default()
        .admit(None)
        .expect("a standalone profile admits one");
    assert!(
        held.inputs_for_key(standalone).is_none(),
        "a scopeless key matched a scoped operation, so production session code \
         treating a missing scope as 'any scope' would be authorized"
    );
}

/// ⭐ **THE ACCEPTED OPERATION OUTLIVES ITS FRAME, MATCHES ONLY ITS OWN INTENT,
/// AND IS RETIRED BY THE SLOT.**
///
/// This is the value room preparation reads several frames after admission, so
/// all three properties are load-bearing and none of them is visible from the
/// admitting frame alone:
///
/// * outliving the frame is what lets a load read it at all;
/// * matching only its own intent is what stops a DOOR crossing recorded while a
///   restore is outstanding from being prepared out of the checkpoint's
///   population — a room rebuilt from a checkpoint that is not about it;
/// * retiring when the slot gives up the intent is what stops a later,
///   unrelated transition matching a stale intent BY VALUE. Two crossings to
///   one room with one subject compare equal.
#[test]
fn the_accepted_restore_outlives_its_frame_matches_its_intent_and_retires_with_the_slot() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope, LifecycleCheckpointHorizonPlugin,
        ResetToCheckpoint,
    };
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

    use crate::session::lifecycle_commit::{
        LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent,
    };

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
    app.init_resource::<PendingLifecycleCommit>();
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

    let admitted_intent = app
        .world()
        .resource::<PendingLifecycleCommit>()
        .peek()
        .map(|pending| pending.kind.clone())
        .expect("the reset took the slot, or nothing below is about an operation");

    // ── IT SURVIVES FRAMES THE SHARED TOKEN DOES NOT ────────────────────────
    for _ in 0..3 {
        app.world_mut().run_schedule(sim);
    }
    let accepted = app.world().resource::<super::AcceptedCheckpointRestore>();
    assert!(
        accepted.accepted().is_some(),
        "the accepted restore was retired while its intent still held the slot. \
         Room preparation runs several frames after admission and reads this \
         value; retiring it early sends the load back to the live ledger"
    );
    assert!(
        accepted.inputs_for(&admitted_intent).is_some(),
        "the accepted restore does not match the intent it was admitted for"
    );

    // ⛔ AND IT DOES NOT MATCH SOMEBODY ELSE'S CROSSING. A door recorded while a
    // restore is outstanding is prepared from LIVE state.
    let a_door = LifecycleIntent::Transition(RoomTransitionIntent {
        subject: ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
        target_room: "here".into(),
        arrival: Vec2::new(9.0, 9.0),
        edge_exit: true,
        zone_sfx: Some("world.portal.enter".into()),
    });
    assert!(
        accepted.inputs_for(&a_door).is_none(),
        "an unrelated crossing matched the checkpoint's pinned population, so it \
         would rebuild its destination from a checkpoint that is not about it"
    );

    // ── THE SLOT GIVES UP THE INTENT: the commit took it ─────────────────────
    app.world_mut()
        .resource_mut::<PendingLifecycleCommit>()
        .take();
    app.world_mut().run_schedule(sim);
    assert!(
        app.world()
            .resource::<super::AcceptedCheckpointRestore>()
            .accepted()
            .is_none(),
        "the accepted restore outlived the operation that owned it. Two crossings \
         to one room with one subject compare EQUAL, so a stale one can be \
         matched by a later transition it has nothing to do with"
    );
}

/// ⛔⛔ **A PARTIAL CHECKSUM REPORTS AGREEMENT BETWEEN PEERS HOLDING DIFFERENT
/// OPERATIONS**, which is worse than having none — the desync it exists to
/// catch is exactly the case it would sleep through.
///
/// The first version of `AcceptedCheckpointRestore::checksum` covered three of
/// four fields: the frame, the pinned ledger, and the intent's TARGET ROOM. So
/// two accepted restores agreed while differing in their pinned MINT RECIPES —
/// which decide what a reconstruction can rebuild a carried runtime mint from —
/// and so did two crossings differing only in subject, arrival, edge or door
/// cue. Both halves are consumed: preparation lowers the fresh plan from the
/// ledger AND the mints, and `inputs_for` matches on the whole intent.
///
/// ⭐ EVERY FIELD IS PERTURBED SEPARATELY, because a hash that folds two fields
/// through one operation can be insensitive to a change in either while looking
/// complete. The intent's own leg is exhaustive by destructure, so a NEW field
/// stops that compiling; this covers the ones that exist.
#[test]
fn the_accepted_restores_checksum_separates_every_field_that_changes_what_it_builds() {
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    use crate::items::pickup::minted_horizon::MintedItemBaseline;
    use crate::session::lifecycle_commit::{LifecycleIntent, RoomTransitionIntent};

    fn crossing() -> RoomTransitionIntent {
        RoomTransitionIntent {
            subject: SimId::player_slot(0),
            target_room: "east".into(),
            arrival: Vec2::new(1.0, 2.0),
            edge_exit: false,
            zone_sfx: None,
        }
    }
    fn accepted(restore: AcceptedRestore) -> u64 {
        let mut held = AcceptedCheckpointRestore::default();
        held.accept(restore);
        held.checksum()
    }
    fn base() -> AcceptedRestore {
        AcceptedRestore {
            key: super::CheckpointOperationKey {
                scope: None,
                sequence: 0,
            },
            frame: 7,
            intent: LifecycleIntent::Transition(crossing()),
            occurrences: Default::default(),
            custody: Default::default(),
            item: Some(crate::items::pickup::minted_horizon::ItemCheckpointRestoreInputs {
                minted: MintedItemBaseline::default(),
                owned: Default::default(),
            }),
        }
    }

    let reference = accepted(base());
    assert_ne!(
        reference,
        AcceptedCheckpointRestore::default().checksum(),
        "an accepted operation must not hash the same as no operation at all"
    );

    let perturbations: Vec<(&str, AcceptedRestore)> = vec![
        ("the originating frame", AcceptedRestore { frame: 8, ..base() }),
        (
            "the operation SEQUENCE — two admissions with identical intents and \
             identical snapshots are still different operations",
            AcceptedRestore {
                key: super::CheckpointOperationKey {
                    sequence: 1,
                    ..base().key
                },
                ..base()
            },
        ),
        (
            "the owning SESSION — a scopeless standalone key is not session zero",
            AcceptedRestore {
                key: super::CheckpointOperationKey {
                    scope: Some(ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(0)),
                    ..base().key
                },
                ..base()
            },
        ),
        (
            "the restore SUBJECT — the body the operation is about",
            AcceptedRestore {
                intent: LifecycleIntent::Transition(RoomTransitionIntent {
                    subject: SimId::player_slot(1),
                    ..crossing()
                }),
                ..base()
            },
        ),
        (
            "the destination room",
            AcceptedRestore {
                intent: LifecycleIntent::Transition(RoomTransitionIntent {
                    target_room: "west".into(),
                    ..crossing()
                }),
                ..base()
            },
        ),
        (
            "the arrival position",
            AcceptedRestore {
                intent: LifecycleIntent::Transition(RoomTransitionIntent {
                    arrival: Vec2::new(9.0, 9.0),
                    ..crossing()
                }),
                ..base()
            },
        ),
        (
            "whether it is an edge crossing, which selects the feel",
            AcceptedRestore {
                intent: LifecycleIntent::Transition(RoomTransitionIntent {
                    edge_exit: true,
                    ..crossing()
                }),
                ..base()
            },
        ),
        (
            "the door cue",
            AcceptedRestore {
                intent: LifecycleIntent::Transition(RoomTransitionIntent {
                    zone_sfx: Some("world.portal.enter".into()),
                    ..crossing()
                }),
                ..base()
            },
        ),
        (
            "the intent's SHAPE — a bodyless rebuild is not a crossing",
            AcceptedRestore {
                intent: LifecycleIntent::ReconstituteRoom(
                    crate::session::lifecycle_commit::RoomReconstitutionIntent {
                        target_room: "east".into(),
                    },
                ),
                ..base()
            },
        ),
        (
            "an ABSENT mint baseline versus an installed empty one — a \
             composition without the item domain is not a checkpoint that saw \
             no mints",
            AcceptedRestore {
                item: None,
                ..base()
            },
        ),
    ];
    for (what, perturbed) in perturbations {
        assert_ne!(
            accepted(perturbed),
            reference,
            "the accepted-restore checksum does not see {what}, so two peers \
             holding different operations agree about their snapshot"
        );
    }
}

/// ⛔⛔ **A RESTORE THAT DID NOT COME BACK RIGHT DOES NOT BECOME A WORLD THE
/// PLAYER MAY ACT IN**, and it publishes exactly one terminal outcome saying so.
///
/// This is the protocol's trusted-failure row. The destructive application has
/// already run by the time verification looks, so there is no old world to
/// return to and this contract does not offer one — what it offers is that the
/// failure is CONTAINED: gameplay is blocked, and the outcome names the
/// operation and the domain rather than the world quietly continuing.
///
/// ⭐ THE FORCED FAILURE IS A REAL SHAPE, not an injected one. A checkpoint that
/// remembers an occurrence in somebody's hand, restored into a composition that
/// cannot materialize it, leaves a custody row nothing answers for — the case
/// `restore_custody_to_checkpoint` already logs a warning about. Before this,
/// that warning was the whole response and the session published a success.
#[test]
fn a_restore_that_fails_verification_blocks_gameplay_and_publishes_one_failure() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        CustodyBaseline, LifecycleCheckpointHorizonPlugin,
    };
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    use crate::session::lifecycle_commit::{LifecycleIntent, RoomReconstitutionIntent};

    let mut app = App::new();
    app.add_plugins(LifecycleCheckpointHorizonPlugin);
    app.init_resource::<super::AcceptedCheckpointRestore>();
    app.init_resource::<super::SessionCheckpointOutcomes>();
    // The mode resource alone: `init_state` wants a `StateTransition` schedule
    // this fixture has no use for, and what is under test is what the session
    // REQUESTS, not what a transition schedule does with the request.
    app.insert_resource(bevy::prelude::NextState::<
        ambition_platformer2d_shared_tangle::schedule::GameMode,
    >::default());

    let key = super::SessionCheckpointOperations::default()
        .admit(None)
        .expect("a fresh counter mints a key");
    // A checkpoint that remembers an object in a hand, in a world that has
    // neither the object nor an item domain to rebuild it with.
    let mut custody = CustodyBaseline::default();
    custody.adopt(
        [(
            SimId::placement("a_key_the_world_cannot_rebuild"),
            SimId::player_slot(0),
        )]
        .into_iter()
        .collect(),
    );
    app.world_mut()
        .resource_mut::<super::AcceptedCheckpointRestore>()
        .accept(super::AcceptedRestore {
            key,
            frame: 0,
            intent: LifecycleIntent::ReconstituteRoom(RoomReconstitutionIntent {
                target_room: "here".into(),
            }),
            occurrences: Default::default(),
            custody,
            item: None,
        });

    assert!(
        super::apply_committed_checkpoint_restore(app.world_mut(), key),
        "the commit did not run the operation at all, so nothing below is about \
         a verification result"
    );

    let outcomes = app.world().resource::<super::SessionCheckpointOutcomes>();
    let outcome = outcomes
        .outcome_for(key)
        .expect("an answered operation has a terminal outcome");
    assert!(
        !outcome.committed(),
        "the restore left a custody row nothing in the world answers for and the \
         session called it committed. A verification that cannot fail is a \
         formality, and a success published over an incomplete restore is worse \
         than none"
    );
    assert_eq!(
        outcome.failure(),
        Some(super::RestoreFailure::Custody),
        "the outcome must name the contract that broke, or the report says only \
         that something went wrong somewhere — and two peers that agreed on \
         'failed' while disagreeing about WHERE blocked gameplay for different \
         reasons"
    );

    assert!(
        matches!(
            app.world().resource::<bevy::prelude::NextState<
                ambition_platformer2d_shared_tangle::schedule::GameMode,
            >>(),
            bevy::prelude::NextState::Pending(
                ambition_platformer2d_shared_tangle::schedule::GameMode::Paused
            )
        ),
        "gameplay was not blocked after a restore failed verification, so the \
         player acts in a world the session knows did not come back"
    );

    // ⛔ AND EXACTLY ONE. A second publication for one operation would let a
    // `Committed` follow a `Failed` for the same restore.
    app.world_mut()
        .resource_mut::<super::SessionCheckpointOutcomes>()
        .publish(super::CheckpointRestoreOutcome::Committed { key });
    assert!(
        !app.world()
            .resource::<super::SessionCheckpointOutcomes>()
            .outcome_for(key)
            .expect("the outcome is still there")
            .committed(),
        "a second terminal outcome overwrote the first for one operation"
    );
}

/// ⭐⭐ **STARTUP IS FINISHED BY ITS OPERATION'S OUTCOME, NOT BY A FLAG IT SETS
/// WHEN IT ASKS** — and a cancelled operation is owed again.
///
/// ⛔⛔ THIS REPLACED A SECOND COMPLETION MECHANISM. `routed_for` and
/// `applied_for` were per-generation latches beside the operation model the
/// reset road uses, so a startup crossing and a death crossing — the same
/// operation asked twice — had two different ways of being "done". Two ways of
/// knowing one thing is how they drift. What replaces them is not another pair
/// of booleans: `Routed` names the admitted operation by KEY, and only that
/// operation's terminal outcome satisfies it.
///
/// ⚠ AND A CANCELLED CROSSING IS RE-ASKED. An operation that leaves the accepted
/// state without publishing an outcome was retracted; a latch would have called
/// that finished and stranded the player in the room the session opened in,
/// which is the F1 defect one level up.
#[test]
fn a_routed_startup_resume_waits_for_its_own_operations_outcome() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };

    use crate::session::lifecycle_commit::PendingLifecycleCommit;

    let mut app = App::new();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.set_checkpoint(ambition_persistence::save_data::PersistedCheckpoint::new(
        "rest_room", 512, 300,
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
    app.init_resource::<SessionStartupResume>();
    app.init_resource::<AcceptedCheckpointRestore>();
    app.init_resource::<SessionCheckpointOperations>();
    app.init_resource::<SessionCheckpointOutcomes>();
    app.add_systems(Update, restore_checkpoint_on_session_start);

    let generation = Some(
        app.world()
            .resource::<ActiveSessionScope>()
            .current()
            .expect("the session was begun")
            .0,
    );

    app.update();
    let Some(StartupResume::Routed(key)) =
        app.world().resource::<SessionStartupResume>().state_for(generation)
    else {
        panic!("the startup resume did not route a crossing to the checkpoint's room");
    };
    // ⭐ AND IT PINNED ITS INPUTS, like the reset road. Before this the startup
    // crossing recorded a bare transition and its destination was prepared from
    // whatever the live ledger held.
    assert!(
        app.world()
            .resource::<AcceptedCheckpointRestore>()
            .inputs_for_key(key)
            .is_some(),
        "a startup crossing is a checkpoint reconstruction and must be an \
         accepted operation like any other, or its room is prepared from live \
         state that only happens to agree at session start"
    );

    // ── STILL IN FLIGHT: no outcome, so nothing re-asks and nothing completes ─
    app.world_mut().resource_mut::<PendingLifecycleCommit>().take();
    app.update();
    assert_eq!(
        app.world().resource::<SessionStartupResume>().state_for(generation),
        Some(StartupResume::Routed(key)),
        "the routed resume re-asked while its own operation was still in flight"
    );

    // ── ITS OPERATION IS ANSWERED ────────────────────────────────────────────
    app.world_mut()
        .resource_mut::<SessionCheckpointOutcomes>()
        .publish(CheckpointRestoreOutcome::Committed { key });
    app.update();
    assert_eq!(
        app.world().resource::<SessionStartupResume>().state_for(generation),
        Some(StartupResume::Satisfied),
        "the routed crossing committed and startup never noticed, so a later \
         room transition can still be mistaken for the resume it is waiting on"
    );
    assert!(
        app.world().resource::<PendingLifecycleCommit>().peek().is_none(),
        "a satisfied startup resume asked for a second crossing"
    );
}

/// ⛔ **A CANCELLED STARTUP CROSSING IS OWED AGAIN**, not called finished.
#[test]
fn a_startup_resume_whose_operation_is_retracted_asks_again() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope,
    };

    use crate::session::lifecycle_commit::PendingLifecycleCommit;

    let mut app = App::new();
    let mut save = ambition_persistence::save_data::AmbitionGameSaveData::default();
    save.set_checkpoint(ambition_persistence::save_data::PersistedCheckpoint::new(
        "rest_room", 512, 300,
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
    app.init_resource::<SessionStartupResume>();
    app.init_resource::<AcceptedCheckpointRestore>();
    app.init_resource::<SessionCheckpointOperations>();
    app.init_resource::<SessionCheckpointOutcomes>();
    app.add_systems(Update, restore_checkpoint_on_session_start);

    app.update();
    let first = app.world().resource::<AcceptedCheckpointRestore>().accepted().is_some();
    assert!(first, "the first update must route and accept an operation");

    // The crossing is retracted before it commits: the slot is cleared and the
    // accepted operation retired, with no outcome ever published.
    app.world_mut().resource_mut::<PendingLifecycleCommit>().take();
    let _ = app
        .world_mut()
        .resource_mut::<AcceptedCheckpointRestore>()
        .retire();

    // One update notices the operation is gone; the next asks again.
    app.update();
    app.update();
    assert!(
        app.world().resource::<PendingLifecycleCommit>().peek().is_some(),
        "a startup crossing that was retracted before committing left the \
         session believing it had resumed. That is the once-per-session latch \
         defect again, one level up: the player stays in the room the session \
         happened to open in, forever"
    );
}

/// ⛔⛔ **AN OPERATION THIS SESSION CANNOT NAME MUST NOT TAKE THE LIFECYCLE
/// SLOT.**
///
/// The counter refuses overflow rather than recycling a live identifier, and
/// that refusal used to happen AFTER the room intent was recorded and the
/// request spent. The result was fail-OPEN in the worst shape available: a
/// lifecycle transition admitted, no accepted operation behind it, and therefore
/// a room rebuilt as an ordinary crossing — no pinned continuity, no domain
/// restore, and a reset the session believed it had handled.
///
/// ⚠ IT IS UNREACHABLE IN PLAY — at one admission per frame at 60 Hz a `u64`
/// lasts about ten billion years — and that is precisely why it needs a test.
/// Nothing else will ever execute this path, so "overflow is refused" was a
/// sentence in a doc comment and nothing more.
#[test]
fn an_exhausted_operation_counter_refuses_the_slot_rather_than_the_identity() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, ActiveSessionScope, ResetToCheckpoint,
    };
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

    use crate::session::lifecycle_commit::PendingLifecycleCommit;

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
    app.init_resource::<PendingLifecycleCommit>();
    app.add_message::<ResetToCheckpoint>();
    app.add_message::<ambition_platformer2d_shared_tangle::lifecycle::CheckpointCommitted>();
    app.add_message::<ambition_combat::events::RoomReplayAdmitted>();
    let sim = app.sim_schedule();
    app.add_plugins((
        ambition_platformer2d_shared_tangle::lifecycle::LifecycleCheckpointHorizonPlugin,
        super::SessionCheckpointHorizonPlugin,
    ));
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
    ));
    app.insert_resource(SessionCheckpointOperations::exhausted_for_test());

    app.world_mut().write_message(ResetToCheckpoint);
    app.world_mut().run_schedule(sim);

    assert!(
        app.world().resource::<PendingLifecycleCommit>().peek().is_none(),
        "a restore this session cannot name took the lifecycle slot anyway. The \
         room would then be rebuilt as an ordinary crossing: no pinned \
         continuity, no domain restore, and the reset spent"
    );
    assert!(
        app.world()
            .resource::<AcceptedCheckpointRestore>()
            .accepted()
            .is_none(),
        "an operation was accepted without an identity"
    );
    assert!(
        app.world().resource::<OutstandingCheckpointRequest>().0,
        "the request was spent on an operation that never happened; it is still \
         owed, and a session that later regains capacity must be able to serve it"
    );
}

/// ⛔⛔ **THE REWARD GOES BACK TO THE HAND THAT BANKED IT, not to a hand.**
///
/// Custody verification first asked only whether each banked occurrence was in
/// SOMEBODY's custody, which a restore that handed the object to the wrong body
/// satisfies — and "the reward is in a hand" is not the contract. This is the
/// stronger incremental check: the custodian named by the checkpoint.
///
/// ⚠ THE SECOND ARM IS THE DUPLICATE. The materialization arm rebuilds
/// occurrences no live entity answers for, and two live things behind one
/// identity is the other way it fails — a world every later lookup answers
/// differently about depending on iteration order.
#[test]
fn custody_verification_names_the_custodian_and_refuses_a_duplicate() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        CustodyBaseline, InCustodyOf, LifecycleCheckpointHorizonPlugin,
    };
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    use crate::session::lifecycle_commit::{LifecycleIntent, RoomReconstitutionIntent};

    let banked = SimId::placement("the_reward");
    let rightful = SimId::player_slot(0);
    let somebody_else = SimId::player_slot(1);

    // `world` builds the state a restore is claimed to have produced, and the
    // fixture then asks verification what it makes of it.
    let verify = |holders: Vec<(SimId, SimId)>| -> Option<super::RestoreFailure> {
        let mut app = App::new();
        app.add_plugins(LifecycleCheckpointHorizonPlugin);
        app.init_resource::<AcceptedCheckpointRestore>();
        app.init_resource::<SessionCheckpointOutcomes>();
        app.insert_resource(bevy::prelude::NextState::<
            ambition_platformer2d_shared_tangle::schedule::GameMode,
        >::default());

        let custodian_entities: std::collections::BTreeMap<SimId, bevy::prelude::Entity> =
            [rightful.clone(), somebody_else.clone()]
                .into_iter()
                .map(|id| {
                    let entity = app.world_mut().spawn(id.clone()).id();
                    (id, entity)
                })
                .collect();
        for (occurrence, custodian) in holders {
            let holder = custodian_entities[&custodian];
            app.world_mut().spawn((occurrence, InCustodyOf(holder)));
        }

        let mut custody = CustodyBaseline::default();
        custody.adopt([(banked.clone(), rightful.clone())].into_iter().collect());
        let key = SessionCheckpointOperations::default().admit(None).unwrap();
        app.world_mut()
            .resource_mut::<AcceptedCheckpointRestore>()
            .accept(super::AcceptedRestore {
                key,
                frame: 0,
                intent: LifecycleIntent::ReconstituteRoom(RoomReconstitutionIntent {
                    target_room: "here".into(),
                }),
                occurrences: Default::default(),
                custody,
                item: None,
            });
        assert!(super::apply_committed_checkpoint_restore(app.world_mut(), key));
        app.world()
            .resource::<SessionCheckpointOutcomes>()
            .outcome_for(key)
            .expect("an answered operation has an outcome")
            .failure()
    };

    // ⛔ THE PREMISE: the shape the checkpoint describes verifies clean, or every
    // failure below is about the fixture rather than about the check.
    assert_eq!(
        verify(vec![(banked.clone(), rightful.clone())]),
        None,
        "the reward in the hand the checkpoint names must verify clean"
    );

    assert_eq!(
        verify(vec![(banked.clone(), somebody_else.clone())]),
        Some(super::RestoreFailure::Custody),
        "the restore put the banked reward in the WRONG hand and verification \
         called it correct — 'in somebody's custody' is not the contract"
    );
    assert_eq!(
        verify(vec![
            (banked.clone(), rightful.clone()),
            (banked.clone(), somebody_else.clone()),
        ]),
        Some(super::RestoreFailure::Custody),
        "the restore left TWO live things behind one identity and verification \
         accepted it, so every later lookup answers differently depending on \
         iteration order"
    );
    assert_eq!(
        verify(Vec::new()),
        Some(super::RestoreFailure::Custody),
        "the restore materialized nothing for a banked custody row"
    );
}
