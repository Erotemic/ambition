//! Actor-side room-graph behavior tests: possession-aware room transitions and fast-body
//! walk-zone tunneling.

use super::*;
// The room vocabulary these fixtures build is the world crate's. Named here
// rather than forwarded from the parent module, which no longer globs it.
use ambition_platformer2d_world::rooms::*;

fn empty_world(name: &str) -> ae::World {
    ae::World::new(
        name,
        ae::Vec2::new(640.0, 480.0),
        ae::Vec2::new(96.0, 96.0),
        Vec::new(),
    )
}

/// A room transition follows the CONTROLLED body, not a `PrimaryPlayer` marker: a
/// possessed actor (the controlled subject, standing in a Walk zone) triggers the
/// transition even though the vacated home avatar is nowhere near it. Pins that the
/// transition capability is body-generic and inherited by possession.
#[test]
fn a_possessed_actor_triggers_a_room_transition_through_a_walk_zone() {
    use ambition_characters::control::SlotInteractionState;
    use ambition_platformer2d_core::BodyKinematics;
    use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
    use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct Captured(Option<String>);

    // This read a `RoomTransitionRequested` message, which only an eager host ever wrote — so
    // these fixtures were checking the half of a fork that the shipped game does not take. The
    // intent names its destination by authored id, so the room is asserted by NAME rather than
    // by index.
    fn capture(
        pending: Res<crate::session::lifecycle_commit::PendingLifecycleCommit>,
        mut out: ResMut<Captured>,
    ) {
        if let Some(crate::session::lifecycle_commit::LifecycleIntent::Transition(transition)) =
            pending.pending.as_ref().map(|intent| &intent.kind)
        {
            out.0 = Some(transition.target_room.clone());
        }
    }

    let zone_center = ae::Vec2::new(100.0, 100.0);
    let mut room_a = spec_with(RoomMetadata::default(), "a");
    room_a.loading_zones = vec![LoadingZone {
        id: "exit_a".into(),
        name: "east".into(),
        activation: LoadingZoneActivation::Walk,
        aabb: ae::Aabb::new(zone_center, ae::Vec2::new(24.0, 24.0)),
    }];
    let mut room_b = spec_with(RoomMetadata::default(), "b");
    room_b.loading_zones = vec![LoadingZone {
        id: "entry_b".into(),
        name: "west".into(),
        activation: LoadingZoneActivation::Walk,
        aabb: ae::Aabb::new(ae::Vec2::new(60.0, 100.0), ae::Vec2::new(24.0, 24.0)),
    }];
    let set = RoomSet::from_parts(
        "a",
        vec![room_a, room_b],
        vec![RoomLink {
            from_room: "a".into(),
            from_zone: "exit_a".into(),
            to_room: "b".into(),
            to_zone: "entry_b".into(),
            bidirectional: false,
        }],
    );

    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        set,
    );
    app.insert_resource(
        ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown::default(),
    );
    app.insert_resource(GatePortalRegistry::default());
    // The live phase is its own resource (rollback state) since
    // `detect_room_transition_system` reads it.
    app.init_resource::<GatePortalPhases>();
    app.init_resource::<SlotInteractionState>();
    app.init_resource::<Captured>();
    app.init_resource::<ambition_time::WorldTime>();
    app.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
    app.add_systems(Update, (detect_room_transition_system, capture).chain());

    // The vacated home avatar, far from the zone.
    //
    // the `SimId`s below are what CONSTRUCTION would have given these
    // bodies, not decoration: `ensure_sim_id` runs in the sim schedule on every
    // host and files a `PrimaryPlayer` under `player_slot(0)` and an authored body
    // under `placement(feature_id)`. These fixtures build their bodies by hand and
    // never run it, so without this they model a body no construction path
    // produces — and a crossing whose subject cannot be named is now refused.
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
        BodyKinematics {
            pos: ae::Vec2::new(1000.0, 1000.0),
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(24.0, 40.0),
            facing: 1.0,
        },
    ));
    // The possessed actor the player is driving, standing IN the walk zone.
    let actor = app
        .world_mut()
        .spawn((
            ambition_platformer2d_shared_tangle::sim_id::SimId::placement("possessed_actor"),
            BodyKinematics {
                pos: zone_center,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            },
        ))
        .id();
    app.world_mut()
        .insert_resource(ControlledSubject(Some(actor)));

    app.update();

    assert_eq!(
        app.world().resource::<Captured>().0.as_deref(),
        Some("b"),
        "the possessed (controlled) actor in the walk zone triggers the transition to room b, \
         even though the home avatar is far away",
    );

    // Death resolves before room-transition detection in the production schedule.
    // Once the controlled body is out of play, the SAME geometry must therefore
    // produce no fresh crossing later in that tick. This is the Mary-O loading
    // hang race: before the filter, a corpse could immediately refill the sticky
    // lifecycle slot that death had just cleared.
    app.world_mut()
        .resource_mut::<crate::session::lifecycle_commit::PendingLifecycleCommit>()
        .take();
    app.world_mut().resource_mut::<Captured>().0 = None;
    app.world_mut()
        .entity_mut(actor)
        .insert(ambition_combat::death_rules::OutOfPlay);

    app.update();

    assert_eq!(
        app.world().resource::<Captured>().0.as_deref(),
        None,
        "an out-of-play controlled body cannot start a new room transition",
    );
}

/// CC2 (§3.3, the sweep law): a fast body must not tunnel an overlap-fire
/// (`Walk`) loading zone. A body that starts BEFORE the zone and ends PAST it
/// in one frame — never discretely overlapping either endpoint — still crosses
/// the zone's swept path, so the transition fires. The pre-CC2 discrete
/// `strict_intersects` check would silently miss this (blink / dash / Sanic
/// speed leaping straight over the exit band).
#[test]
fn a_fast_body_cannot_tunnel_a_walk_loading_zone() {
    use ambition_characters::control::SlotInteractionState;
    use ambition_platformer2d_core::BodyKinematics;
    use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct Captured(Option<String>);

    // This read a `RoomTransitionRequested` message, which only an eager host ever wrote — so
    // these fixtures were checking the half of a fork that the shipped game does not take. The
    // intent names its destination by authored id, so the room is asserted by NAME rather than
    // by index.
    fn capture(
        pending: Res<crate::session::lifecycle_commit::PendingLifecycleCommit>,
        mut out: ResMut<Captured>,
    ) {
        if let Some(crate::session::lifecycle_commit::LifecycleIntent::Transition(transition)) =
            pending.pending.as_ref().map(|intent| &intent.kind)
        {
            out.0 = Some(transition.target_room.clone());
        }
    }

    // A thin exit band at x = 100 (half-width 8) — thinner than the body travels
    // in one frame.
    let zone_center = ae::Vec2::new(100.0, 100.0);
    let mut room_a = spec_with(RoomMetadata::default(), "a");
    room_a.loading_zones = vec![LoadingZone {
        id: "exit_a".into(),
        name: "east".into(),
        activation: LoadingZoneActivation::Walk,
        aabb: ae::Aabb::new(zone_center, ae::Vec2::new(8.0, 40.0)),
    }];
    let mut room_b = spec_with(RoomMetadata::default(), "b");
    room_b.loading_zones = vec![LoadingZone {
        id: "entry_b".into(),
        name: "west".into(),
        activation: LoadingZoneActivation::Walk,
        aabb: ae::Aabb::new(ae::Vec2::new(60.0, 100.0), ae::Vec2::new(8.0, 40.0)),
    }];
    let set = RoomSet::from_parts(
        "a",
        vec![room_a, room_b],
        vec![RoomLink {
            from_room: "a".into(),
            from_zone: "exit_a".into(),
            to_room: "b".into(),
            to_zone: "entry_b".into(),
            bidirectional: false,
        }],
    );

    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        set,
    );
    app.insert_resource(
        ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown::default(),
    );
    app.insert_resource(GatePortalRegistry::default());
    // The live phase is its own resource (rollback state) since
    // `detect_room_transition_system` reads it.
    app.init_resource::<GatePortalPhases>();
    app.init_resource::<SlotInteractionState>();
    app.init_resource::<Captured>();
    // A 60 fps frame; the body crosses the whole zone within it.
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    app.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
    app.add_systems(Update, (detect_room_transition_system, capture).chain());

    // The body has already SHOT PAST the zone this frame: it ends at x = 200
    // (clear of the x = 100 band, half-width 12) having entered from x = 40, so
    // its velocity places the band squarely on its swept path. A discrete
    // endpoint check at x = 200 would see no overlap and miss the exit.
    let dt = 1.0 / 60.0;
    let end = ae::Vec2::new(200.0, 100.0);
    let start = ae::Vec2::new(40.0, 100.0);
    let vel = (end - start) / dt;
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
        // What `ensure_sim_id` gives a primary avatar on every host; see the
        // note in `a_possessed_actor_triggers_a_room_transition_through_a_walk_zone`.
        ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
        BodyKinematics {
            pos: end,
            vel,
            size: ae::Vec2::new(24.0, 40.0),
            facing: 1.0,
        },
    ));

    app.update();

    assert_eq!(
        app.world().resource::<Captured>().0.as_deref(),
        Some("b"),
        "a body that tunnelled through the walk zone in one frame still triggers \
         the transition — the reader sweeps its path (CC2), it does not sample \
         the endpoint",
    );
}

/// A body that WALKED into an edge exit and was stopped by the boundary still
/// transitions — even though its velocity is now zero.
///
/// that is not a key binding issue."* He was right, and the two zone tests above
/// are why nobody found it: both hand the detector a velocity chosen to make
/// the answer come out. The tunnel test computes `vel = (end - start) / dt`
/// after picking both endpoints, which is precisely the assumption production
/// breaks; the door tests place the body inside the zone and press a key. Neither
/// runs a body through the movement kernel, so neither could see this.
///
/// What production does, in order:
///
/// ```text
/// 1. the kernel integrates prev → curr and writes SweepSample
/// 2. collision advances to time-of-impact and calls zero_axis_vel
/// 3. the detector reconstructs the path as vel · dt    ZERO
/// ```
///
/// An edge exit sits at a room boundary — the one place step 2 always happens —
/// so the segment that proves the body entered the zone was being discarded on
/// exactly the frame it mattered. A body left TOUCHING the band rather than
/// strictly inside it then never transitions, however long it stands there.
#[test]
fn a_body_stopped_at_the_boundary_still_crosses_the_zone_it_walked_into() {
    use ambition_characters::control::SlotInteractionState;
    use ambition_platformer2d_core::BodyKinematics;
    use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct Captured(Option<String>);

    // This read a `RoomTransitionRequested` message, which only an eager host ever wrote — so
    // these fixtures were checking the half of a fork that the shipped game does not take. The
    // intent names its destination by authored id, so the room is asserted by NAME rather than
    // by index.
    fn capture(
        pending: Res<crate::session::lifecycle_commit::PendingLifecycleCommit>,
        mut out: ResMut<Captured>,
    ) {
        if let Some(crate::session::lifecycle_commit::LifecycleIntent::Transition(transition)) =
            pending.pending.as_ref().map(|intent| &intent.kind)
        {
            out.0 = Some(transition.target_room.clone());
        }
    }

    // The exit band at the room's east edge, and a body stopped with its right
    // face exactly ON the band's left face — touching, not overlapping. This is
    // what a collision solver leaves behind when it advances to time-of-impact.
    let zone_center = ae::Vec2::new(100.0, 100.0);
    let body_half = ae::Vec2::new(12.0, 20.0);
    let stopped_at = ae::Vec2::new(zone_center.x - 8.0 - body_half.x, 100.0);

    let build = |sample: Option<ae::SweepSample>| {
        let mut room_a = spec_with(RoomMetadata::default(), "a");
        room_a.loading_zones = vec![LoadingZone {
            id: "exit_a".into(),
            name: "east".into(),
            activation: LoadingZoneActivation::EdgeExit,
            aabb: ae::Aabb::new(zone_center, ae::Vec2::new(8.0, 40.0)),
        }];
        let mut room_b = spec_with(RoomMetadata::default(), "b");
        room_b.loading_zones = vec![LoadingZone {
            id: "entry_b".into(),
            name: "west".into(),
            activation: LoadingZoneActivation::EdgeExit,
            aabb: ae::Aabb::new(ae::Vec2::new(60.0, 100.0), ae::Vec2::new(8.0, 40.0)),
        }];
        let set = RoomSet::from_parts(
            "a",
            vec![room_a, room_b],
            vec![RoomLink {
                from_room: "a".into(),
                from_zone: "exit_a".into(),
                to_room: "b".into(),
                to_zone: "entry_b".into(),
                bidirectional: false,
            }],
        );

        let mut app = App::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            set,
        );
        app.insert_resource(
            ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown::default(),
        );
        app.insert_resource(GatePortalRegistry::default());
        // The live phase is its own resource (rollback state) since
        // `detect_room_transition_system` reads it.
        app.init_resource::<GatePortalPhases>();
        app.init_resource::<SlotInteractionState>();
        app.init_resource::<Captured>();
        app.insert_resource(ambition_time::WorldTime {
            scaled_dt: 1.0 / 60.0,
            ..Default::default()
        });
        app.init_resource::<crate::session::lifecycle_commit::PendingLifecycleCommit>();
        app.add_systems(Update, (detect_room_transition_system, capture).chain());

        let body = BodyKinematics {
            pos: stopped_at,
            // ZERO, and that is the whole point: the wall took it.
            vel: ae::Vec2::ZERO,
            size: body_half * 2.0,
            facing: 1.0,
        };
        let mut entity = app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            // What `ensure_sim_id` gives a primary avatar on every host.
            ambition_platformer2d_shared_tangle::sim_id::SimId::player_slot(0),
            body,
        ));
        if let Some(sample) = sample {
            entity.insert(sample);
        }
        app.update();
        app.world().resource::<Captured>().0.clone()
    };

    // The kernel's record of the frame: it walked 40 px east and was stopped.
    let travelled = ae::SweepSample {
        prev: stopped_at - ae::Vec2::new(40.0, 0.0),
        curr: stopped_at,
        vel: ae::Vec2::new(2400.0, 0.0),
        half: body_half,
    };
    assert_eq!(
        build(Some(travelled)).as_deref(),
        Some("b"),
        "the body walked into the exit band and was stopped ON it. Its TRUE path \
         (SweepSample) crosses the zone, so the transition fires — the reader must \
         read the kernel's segment, not the velocity collision just zeroed",
    );

    // THE POISON, and it is the shipped bug: same body, same position, no
    // sample  the reader falls back to `vel · dt`, which is zero, and the zone
    // it is standing against goes unnoticed.
    assert_eq!(
        build(None).as_deref(),
        None,
        "and with no sample the reconstruction is `vel · dt` = 0, which cannot \
         describe that movement at all — if this ever names a room the fixture \
         has stopped modelling the collision that makes the bug possible",
    );
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
        placements: Vec::new(),
        encounter_triggers: Vec::new(),
        lock_walls: Vec::new(),
        switch_commands: Vec::new(),
    }
}

#[test]
fn active_metadata_returns_active_room_metadata() {
    let m1 = RoomMetadata {
        biome: Some("hub".into()),
        music_track: Some("hub_loop".into()),
        ambient_profile: None,
        visual_theme: None,
        visual_profile: Default::default(),
        nameplate_policy: Default::default(),
        gallery: false,
        mode: None,
        fall_out_margin: None,
        side_out_margin: None,
        rise_out_margin: None,
        next_room: None,
    };
    let m2 = RoomMetadata {
        biome: Some("cave".into()),
        music_track: Some("cave_loop".into()),
        ambient_profile: Some("damp".into()),
        visual_theme: None,
        visual_profile: Default::default(),
        nameplate_policy: Default::default(),
        gallery: false,
        mode: None,
        fall_out_margin: None,
        side_out_margin: None,
        rise_out_margin: None,
        next_room: None,
    };
    let mut set = RoomSet::from_parts(
        "first",
        vec![
            spec_with(m1.clone(), "first"),
            spec_with(m2.clone(), "second"),
        ],
        Vec::new(),
    );
    assert_eq!(set.active_metadata(), &m1);
    set.set_active(1);
    assert_eq!(set.active_metadata(), &m2);
}

#[test]
fn sync_room_music_request_mirrors_metadata_music_track() {
    use bevy::prelude::*;
    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ActiveRoomMetadata(RoomMetadata {
            biome: Some("cave".into()),
            music_track: Some("cave_loop".into()),
            ambient_profile: None,
            visual_theme: None,
            visual_profile: Default::default(),
            nameplate_policy: Default::default(),
            gallery: false,
            mode: None,
            fall_out_margin: None,
            side_out_margin: None,
            rise_out_margin: None,
            next_room: None,
        }),
    );
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        RoomMusicRequest::default(),
    );
    app.add_systems(Update, sync_room_music_request);
    app.update();
    assert_eq!(
        ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<RoomMusicRequest>(
            app.world()
        )
        .expect("session room music")
        .desired_track,
        Some("cave_loop".into())
    );

    // Empty active metadata clears the request.
    ambition_platformer2d_shared_tangle::lifecycle::session_world_component_mut::<
        ActiveRoomMetadata,
    >(app.world_mut())
    .expect("session active-room metadata")
    .0
    .music_track = None;
    app.update();
    assert_eq!(
        ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<RoomMusicRequest>(
            app.world()
        )
        .expect("session room music")
        .desired_track,
        None
    );
}

#[test]
fn sync_active_room_metadata_publishes_active_value() {
    use bevy::prelude::*;
    let mut app = App::new();
    let m_hub = RoomMetadata {
        biome: Some("hub".into()),
        music_track: Some("hub_loop".into()),
        ambient_profile: None,
        visual_theme: None,
        visual_profile: Default::default(),
        nameplate_policy: Default::default(),
        gallery: false,
        mode: None,
        fall_out_margin: None,
        side_out_margin: None,
        rise_out_margin: None,
        next_room: None,
    };
    let m_lab = RoomMetadata {
        biome: Some("lab".into()),
        music_track: Some("lab_loop".into()),
        ambient_profile: None,
        visual_theme: None,
        visual_profile: Default::default(),
        nameplate_policy: Default::default(),
        gallery: false,
        mode: None,
        fall_out_margin: None,
        side_out_margin: None,
        rise_out_margin: None,
        next_room: None,
    };
    let set = RoomSet::from_parts(
        "hub",
        vec![
            spec_with(m_hub.clone(), "hub"),
            spec_with(m_lab.clone(), "lab"),
        ],
        Vec::new(),
    );
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        set,
    );
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ActiveRoomMetadata::default(),
    );
    app.add_systems(Update, sync_active_room_metadata);
    app.update();
    assert_eq!(
        &ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<
            ActiveRoomMetadata,
        >(app.world())
        .expect("session active-room metadata")
        .0,
        &m_hub
    );

    ambition_platformer2d_shared_tangle::lifecycle::session_world_component_mut::<RoomSet>(
        app.world_mut(),
    )
    .expect("session room set")
    .set_active(1);
    app.update();
    assert_eq!(
        &ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<
            ActiveRoomMetadata,
        >(app.world())
        .expect("session active-room metadata")
        .0,
        &m_lab
    );
}

#[test]
fn room_metadata_is_empty_default_is_true() {
    let m = RoomMetadata::default();
    assert!(m.is_empty());
}

#[test]
fn room_metadata_is_empty_false_when_any_field_set() {
    let mut m = RoomMetadata::default();
    m.biome = Some("hub".into());
    assert!(!m.is_empty());

    let m = RoomMetadata {
        biome: None,
        music_track: Some("loop".into()),
        ambient_profile: None,
        visual_theme: None,
        visual_profile: Default::default(),
        nameplate_policy: Default::default(),
        gallery: false,
        mode: None,
        fall_out_margin: None,
        side_out_margin: None,
        rise_out_margin: None,
        next_room: None,
    };
    assert!(!m.is_empty());

    let mut m = RoomMetadata::default();
    m.visual_profile.id = Some("intro".into());
    assert!(!m.is_empty());

    let mut m = RoomMetadata::default();
    m.nameplate_policy.full_opacity_count = Some(100);
    assert!(!m.is_empty());

    let mut m = RoomMetadata::default();
    m.mode = Some("sanic".into());
    assert!(!m.is_empty());
}

#[test]
fn room_metadata_merge_preserves_existing_values() {
    let mut a = RoomMetadata {
        biome: Some("hub".into()),
        music_track: None,
        ambient_profile: None,
        visual_theme: Some("blue".into()),
        visual_profile: Default::default(),
        nameplate_policy: Default::default(),
        gallery: false,
        mode: None,
        fall_out_margin: None,
        side_out_margin: None,
        rise_out_margin: None,
        next_room: None,
    };
    let b = RoomMetadata {
        biome: Some("CONFLICT".into()),        // ignored — a.biome wins
        music_track: Some("hub_loop".into()),  // takes effect — a.music_track was None
        ambient_profile: Some("damp".into()),  // takes effect
        visual_theme: Some("CONFLICT".into()), // ignored
        visual_profile: Default::default(),
        nameplate_policy: RoomNameplatePolicy {
            full_opacity_count: Some(100),
            fade_out_count: Some(120),
            label_driven_bodies: Some(true),
        },
        gallery: true,              // takes effect — a.gallery was false (merge ORs)
        mode: Some("sanic".into()), // takes effect — a.mode was None
        fall_out_margin: None,
        side_out_margin: None,
        rise_out_margin: None,
        // takes effect — a.next_room was None. An area spanning several levels
        // has ONE exit, and it is whichever member level names one first; a
        // merge that dropped it would turn an authored circuit into a room that
        // silently loops.
        next_room: Some("cave_2".into()),
    };
    a.merge(b);
    assert_eq!(a.biome.as_deref(), Some("hub"));
    assert!(a.gallery, "merge ORs the gallery flag from a member level");
    assert_eq!(a.music_track.as_deref(), Some("hub_loop"));
    assert_eq!(a.ambient_profile.as_deref(), Some("damp"));
    assert_eq!(a.visual_theme.as_deref(), Some("blue"));
    assert_eq!(a.nameplate_policy.full_opacity_count, Some(100));
    assert_eq!(a.nameplate_policy.fade_out_count, Some(120));
    assert_eq!(
        a.mode.as_deref(),
        Some("sanic"),
        "a member level's mode tag propagates to the merged active area"
    );
    assert_eq!(
        a.next_room.as_deref(),
        Some("cave_2"),
        "a member level's exit propagates to the merged active area"
    );

    // ...and the first non-empty value still wins, so one level cannot
    // re-home an area another level already claimed for its ruleset.
    let mut a = RoomMetadata {
        mode: Some("sanic".into()),
        fall_out_margin: None,
        side_out_margin: None,
        rise_out_margin: None,
        ..Default::default()
    };
    a.merge(RoomMetadata {
        mode: Some("CONFLICT".into()),
        fall_out_margin: None,
        side_out_margin: None,
        rise_out_margin: None,
        ..Default::default()
    });
    assert_eq!(a.mode.as_deref(), Some("sanic"));
}

#[test]
fn room_visual_profile_merge_prefers_existing_values() {
    let mut a = RoomVisualProfile {
        id: Some("intro".into()),
        parallax_theme: None,
        palette: Some("warm".into()),
        lighting_hint: None,
        foreground_treatment: None,
    };
    let b = RoomVisualProfile {
        id: Some("conflict".into()),
        parallax_theme: Some("basement".into()),
        palette: Some("cool".into()),
        lighting_hint: Some("low_key".into()),
        foreground_treatment: Some("dust".into()),
    };
    a.merge(b);
    assert_eq!(a.id.as_deref(), Some("intro"));
    assert_eq!(a.parallax_theme.as_deref(), Some("basement"));
    assert_eq!(a.palette.as_deref(), Some("warm"));
    assert_eq!(a.lighting_hint.as_deref(), Some("low_key"));
    assert_eq!(a.foreground_treatment.as_deref(), Some("dust"));
}

#[test]
fn camera_clamp_mode_parses_author_values() {
    assert_eq!(
        CameraClampMode::from_author_value(Some("zone_bounds")),
        CameraClampMode::ZoneBounds
    );
    assert_eq!(
        CameraClampMode::from_author_value(Some("free")),
        CameraClampMode::None
    );
    assert_eq!(
        CameraClampMode::from_author_value(Some("whatever")),
        CameraClampMode::RoomBounds
    );
}

#[test]
fn loading_zone_activation_label_is_non_empty() {
    assert!(!LoadingZoneActivation::EdgeExit.label().is_empty());
    assert!(!LoadingZoneActivation::Door.label().is_empty());
}

#[test]
fn loading_zone_is_ready_respects_activation() {
    let edge = LoadingZone {
        id: "x".into(),
        name: "x".into(),
        aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(1.0, 1.0)),
        activation: LoadingZoneActivation::EdgeExit,
    };
    // EdgeExit is always ready (auto-fires on overlap).
    assert!(edge.is_ready(false));
    assert!(edge.is_ready(true));

    let door = LoadingZone {
        id: "y".into(),
        name: "y".into(),
        aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(1.0, 1.0)),
        activation: LoadingZoneActivation::Door,
    };
    // Door requires interact press.
    assert!(!door.is_ready(false));
    assert!(door.is_ready(true));
}

#[test]
fn loading_zone_hint_includes_door_prompt() {
    let door = LoadingZone {
        id: "lab_door".into(),
        name: "lab door".into(),
        aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(1.0, 1.0)),
        activation: LoadingZoneActivation::Door,
    };
    let hint = door.hint(false);
    assert!(hint.contains("door"));
    assert!(hint.contains("Interact") || hint.contains("interact"));
    assert!(hint.contains("lab door"));
}

#[test]
fn loading_zone_hint_for_edge_exit_skips_prompt() {
    let edge = LoadingZone {
        id: "east_exit".into(),
        name: "east exit".into(),
        aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(1.0, 1.0)),
        activation: LoadingZoneActivation::EdgeExit,
    };
    let hint = edge.hint(false);
    assert!(hint.contains("east exit"));
    // Auto-firing edge exits don't need an Interact prompt.
    assert!(!hint.contains("Interact"));
}

#[test]
fn kinematic_path_spec_matches_id_accepts_the_name_slug() {
    use ambition_platformer2d_core::KinematicPath;
    use ambition_platformer2d_world::rooms::KinematicPathSpec;

    // A spec whose id was NOT derived from its display name is still
    // reachable by that name's slug — the alias exists for rooms built
    // in Rust, which may carry any id they like (or none).
    //
    // `enemy_patrol_a` is a hand-written id here, not a derived one.
    let spec = KinematicPathSpec::new(
        "enemy_patrol_a",
        "enemy patrol path A",
        ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(1.0, 1.0)),
        KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(100.0, 0.0), 40.0),
    );
    assert!(
        spec.matches_id("enemy_patrol_a"),
        "exact id alias must match"
    );
    assert!(
        spec.matches_id("enemy patrol path A"),
        "exact name alias must match"
    );
    assert!(
        spec.matches_id("enemy_patrol_path_a"),
        "raw slug-of-name must match"
    );
    assert!(
        !spec.matches_id("some_other_id"),
        "unrelated id must NOT match"
    );
}

/// The MOVEMENT KERNEL walks a body into a wall on an exit band, and the
/// transition fires from the sample the kernel actually published.
///
/// the sibling test above MANUFACTURES its `SweepSample`, and that is the hole this one closes.
/// It proves the detector reads a sample correctly; it cannot prove the kernel still WRITES one, or
/// writes one whose `prev` is the pre-collision position.
///
/// Nothing is hand-built here but the room: the floor and wall are real
/// geometry, the body accelerates under the real movement model, the stop is the
/// real collision solver's, and the delta handed to the real predicate is
/// `SweepSample::delta()` off the borrowed view the kernel wrote through.
#[test]
fn the_real_kernel_publishes_a_sample_that_crosses_the_zone_it_was_stopped_on() {
    let zone_center = ae::Vec2::new(300.0, 100.0);
    let zone_half = ae::Vec2::new(8.0, 40.0);
    let body_half = ae::Vec2::new(12.0, 20.0);
    let band_left = zone_center.x - zone_half.x;
    let floor_top = 130.0;
    let block = |name: &str, center: ae::Vec2, half: ae::Vec2| ae::Block {
        id: ae::GeoId::placement(ae::PlacementId::new(name), 0),
        name: name.into(),
        aabb: ae::Aabb::new(center, half),
        velocity: ae::Vec2::ZERO,
        kind: ae::BlockKind::Solid,
        art_color: None,
    };
    let world = ae::World::new(
        "kernel_zone",
        ae::Vec2::new(600.0, 400.0),
        ae::Vec2::new(40.0, 100.0),
        vec![
            block(
                "floor",
                ae::Vec2::new(300.0, floor_top + 50.0),
                ae::Vec2::new(300.0, 50.0),
            ),
            // The wall's left face IS the band's left face, so a body walking
            // east is stopped exactly against the zone it is trying to enter.
            block(
                "east_wall",
                ae::Vec2::new(band_left + 100.0, floor_top - 100.0),
                ae::Vec2::new(100.0, 100.0),
            ),
        ],
    );

    let start = ae::Vec2::new(band_left - body_half.x - 120.0, floor_top - body_half.y);
    let mut scratch =
        ae::BodyClusterScratch::new_with_abilities(start, ae::AbilitySet::sandbox_all());
    scratch.kinematics.size = body_half * 2.0;
    let frame = ae::MotionFrame::from_acceleration(
        ambition_platformer2d_core::movement::DEFAULT_GRAVITY_DIR
            * ambition_platformer2d_core::movement::GRAVITY,
    )
    .expect("the default gravity is non-zero");

    // Walk east until the solver stops making eastward progress — the arrival
    // tick is the one whose sample spans "short of the band" to "against it".
    let mut arrival: Option<(ae::SweepSample, ae::Vec2, ae::Vec2)> = None;
    for _ in 0..240 {
        let before = scratch.kinematics.pos;
        let mut sample = ae::SweepSample {
            prev: before,
            curr: before,
            vel: ae::Vec2::ZERO,
            half: body_half,
        };
        {
            let (model, mut clusters) = scratch.parts();
            clusters.sweep = Some(&mut sample);
            ambition_platformer2d_core::step_motion(
                model,
                &mut clusters,
                ambition_platformer2d_core::MotionStepContext {
                    world: &world,
                    input: ae::InputState {
                        axes: ae::LocalAxes::new(1.0, 0.0),
                        ..Default::default()
                    },
                    frame,
                    facing_intent: 1.0,
                    dt: 1.0 / 60.0,
                    contact: ambition_platformer2d_core::BodyContactField::NONE,
                    pose_owned_externally: false,
                    recovery_commitment_outstanding: false,
                },
            );
        }
        let after = scratch.kinematics.pos;
        if after.x - before.x < 0.01 && before.x > start.x + 1.0 {
            // Progress has stopped: the ARRIVAL tick is the last one that moved,
            // whose segment runs from short of the band to against it. This tick
            // is the pinned one after it, and its segment is a point — which is
            // exactly why a reader that waits for "now" never sees the crossing.
            break;
        }
        arrival = Some((sample, after, scratch.kinematics.vel));
    }

    let (sample, stopped, post_vel) =
        arrival.expect("the body never stopped advancing east — it never reached the wall");
    assert!(
        (stopped.x + body_half.x - band_left).abs() <= 1.0,
        "the body settled at {stopped:?}, right face {}, but the band starts at \
         {band_left} — this fixture only models the bug while the solver leaves \
         the body against the band it walked into",
        stopped.x + body_half.x
    );

    let mut room = spec_with(RoomMetadata::default(), "a");
    room.loading_zones = vec![LoadingZone {
        id: "exit_a".into(),
        name: "east".into(),
        activation: LoadingZoneActivation::EdgeExit,
        aabb: ae::Aabb::new(zone_center, zone_half),
    }];
    let mut room_b = spec_with(RoomMetadata::default(), "b");
    room_b.loading_zones = vec![LoadingZone {
        id: "entry_b".into(),
        name: "west".into(),
        activation: LoadingZoneActivation::EdgeExit,
        aabb: ae::Aabb::new(ae::Vec2::new(60.0, 100.0), zone_half),
    }];
    let set = RoomSet::from_parts(
        "a",
        vec![room, room_b],
        vec![RoomLink {
            from_room: "a".into(),
            from_zone: "exit_a".into(),
            to_room: "b".into(),
            to_zone: "entry_b".into(),
            bidirectional: false,
        }],
    );

    let body_aabb = ae::Aabb::new(stopped, body_half);
    assert!(
        set.transition_for_player(body_aabb, sample.delta(), false)
            .is_some(),
        "the kernel's own published segment ({:?} -> {:?}) reaches the band the \
         body was stopped against, so the transition must fire",
        sample.prev,
        sample.curr
    );

    // THE POISON: the fallback the detector uses when no sample exists. The
    // solver zeroed the axis, so this describes movement that never reaches the
    // band the body is touching.
    assert!(
        set.transition_for_player(body_aabb, post_vel * (1.0 / 60.0), false)
            .is_none(),
        "post-collision velocity {post_vel:?} still reaches the zone, so this \
         fixture no longer models the collision that makes the sample necessary"
    );
}

/// a rewind must put the portal's PHASE back, not just the switch that
/// drives it.
///
/// This runs the shipped system over a real rollback shape: snapshot the phase
/// resource at frame 10 (a clone, which is exactly what
/// `rollback_resource_clone` stores), overshoot to frame 22, then resimulate
/// frames 11..=16 from the snapshot and compare against a timeline that reached
/// frame 16 without ever rewinding.
///
/// frame 16 is chosen to land INSIDE the ~38-tick opening window — the test
/// asserts that first, because once the portal reaches `On` both timelines
/// agree again and the fixture would prove nothing.
#[test]
fn a_rewind_across_the_portal_opening_window_restores_the_confirmed_phase() {
    use bevy::prelude::*;

    const ZONE: &str = "gate";
    const DT: f32 = 1.0 / 60.0;

    /// A world with one authored portal whose switch is already ON, so every
    /// tick advances the `Opening` timer.
    fn portal_app() -> App {
        let mut app = App::new();
        let mut registry = GatePortalRegistry::default();
        registry
            .try_register(ZONE, "gate_switch", "portal", "ring")
            .expect("a fresh registry has no conflicting portal");
        app.insert_resource(registry);
        app.init_resource::<GatePortalPhases>();
        let mut save = ambition_persistence::save::AmbitionGameSave::default();
        save.data_mut().set_switch("gate_switch", true);
        app.insert_resource(save);
        app.insert_resource(ambition_time::WorldTime {
            scaled_dt: DT,
            ..Default::default()
        });
        app.add_systems(Update, tick_portal_phases_system);
        app
    }

    fn tick_n(app: &mut App, n: usize) {
        for _ in 0..n {
            app.update();
        }
    }

    fn phases(app: &App) -> GatePortalPhases {
        app.world().resource::<GatePortalPhases>().clone()
    }

    // The confirmed timeline, frame 16.
    let mut confirmed_app = portal_app();
    tick_n(&mut confirmed_app, 16);
    let confirmed = phases(&confirmed_app).phase(ZONE);
    assert!(
        matches!(confirmed, GatePortalPhase::Opening { .. }),
        "frame 16 must still be mid-open or this fixture cannot see a divergence; \
         got {confirmed:?}"
    );
    assert!(
        !confirmed.allows_traversal(),
        "a mid-open gate must refuse traversal — that refusal is the decision \
         this state feeds"
    );

    // The speculative timeline: frame 10 is saved, then it runs on to frame 22
    // before a rollback to 10 arrives.
    let mut speculative_app = portal_app();
    tick_n(&mut speculative_app, 10);
    let snapshot_at_frame_10 = phases(&speculative_app);
    tick_n(&mut speculative_app, 12);
    let speculative_at_frame_22 = phases(&speculative_app);

    // THE POISON — the defect's own behaviour: resimulate frames 11..=16
    // with the phase NOT restored, exactly as an unregistered resource behaves
    // while everything around it rewinds.
    let mut unrestored_app = portal_app();
    unrestored_app.insert_resource(speculative_at_frame_22);
    tick_n(&mut unrestored_app, 6);
    assert_ne!(
        phases(&unrestored_app).phase(ZONE),
        confirmed,
        "an UNRESTORED phase reproduced the confirmed frame, so this fixture \
         cannot distinguish the defect from the fix"
    );

    // the registered behaviour: the phase comes back with the frame, and
    // resimulation lands where the confirmed timeline stood.
    let mut restored_app = portal_app();
    restored_app.insert_resource(snapshot_at_frame_10);
    tick_n(&mut restored_app, 6);
    assert_eq!(
        phases(&restored_app).phase(ZONE),
        confirmed,
        "resimulating frames 11..=16 from the restored frame-10 phase must land \
         on the phase the confirmed timeline had at frame 16"
    );
}
