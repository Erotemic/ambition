//! Actor-side room-graph behavior tests: possession-aware room transitions and
//! fast-body walk-zone tunneling. The pure gate-portal PHASE-transition unit
//! tests moved to `ambition_world::rooms::gate_portal` (fable audit F5.4
//! test-travel — they exercise world-owned vocabulary, so they belong there).

use super::*;

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
    use crate::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
    use crate::control::SlotInteractionState;
    use ambition_platformer_primitives::markers::ControlledSubject;
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct Captured(Option<usize>);

    fn capture(mut reqs: MessageReader<RoomTransitionRequested>, mut out: ResMut<Captured>) {
        if let Some(req) = reqs.read().last() {
            out.0 = Some(req.transition.target_room);
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
    ambition_platformer_primitives::lifecycle::insert_session_world_component(app.world_mut(), set);
    app.insert_resource(crate::SandboxSimState::default());
    app.insert_resource(GatePortalRegistry::default());
    app.init_resource::<SlotInteractionState>();
    app.init_resource::<Captured>();
    app.init_resource::<ambition_time::WorldTime>();
    app.add_message::<RoomTransitionRequested>();
    app.add_systems(Update, (detect_room_transition_system, capture).chain());

    // The vacated home avatar, far from the zone.
    app.world_mut().spawn((
        PlayerEntity,
        PrimaryPlayer,
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
        .spawn(BodyKinematics {
            pos: zone_center,
            vel: ae::Vec2::ZERO,
            size: ae::Vec2::new(24.0, 40.0),
            facing: 1.0,
        })
        .id();
    app.world_mut()
        .insert_resource(ControlledSubject(Some(actor)));

    app.update();

    assert_eq!(
        app.world().resource::<Captured>().0,
        Some(1),
        "the possessed (controlled) actor in the walk zone triggers the transition to room b, \
         even though the home avatar is far away",
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
    use crate::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
    use crate::control::SlotInteractionState;
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    struct Captured(Option<usize>);

    fn capture(mut reqs: MessageReader<RoomTransitionRequested>, mut out: ResMut<Captured>) {
        if let Some(req) = reqs.read().last() {
            out.0 = Some(req.transition.target_room);
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
    ambition_platformer_primitives::lifecycle::insert_session_world_component(app.world_mut(), set);
    app.insert_resource(crate::SandboxSimState::default());
    app.insert_resource(GatePortalRegistry::default());
    app.init_resource::<SlotInteractionState>();
    app.init_resource::<Captured>();
    // A 60 fps frame; the body crosses the whole zone within it.
    app.insert_resource(ambition_time::WorldTime {
        scaled_dt: 1.0 / 60.0,
        ..Default::default()
    });
    app.add_message::<RoomTransitionRequested>();
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
        BodyKinematics {
            pos: end,
            vel,
            size: ae::Vec2::new(24.0, 40.0),
            facing: 1.0,
        },
    ));

    app.update();

    assert_eq!(
        app.world().resource::<Captured>().0,
        Some(1),
        "a body that tunnelled through the walk zone in one frame still triggers \
         the transition — the reader sweeps its path (CC2), it does not sample \
         the endpoint",
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
    ambition_platformer_primitives::lifecycle::insert_session_world_component(app.world_mut(), ActiveRoomMetadata(RoomMetadata {
        biome: Some("cave".into()),
        music_track: Some("cave_loop".into()),
        ambient_profile: None,
        visual_theme: None,
        visual_profile: Default::default(),
        nameplate_policy: Default::default(),
        gallery: false,
        mode: None,
    }));
    ambition_platformer_primitives::lifecycle::insert_session_world_component(app.world_mut(), RoomMusicRequest::default());
    app.add_systems(Update, sync_room_music_request);
    app.update();
    assert_eq!(
        ambition_platformer_primitives::lifecycle::session_world_component::<RoomMusicRequest>(app.world()).expect("session room music").desired_track,
        Some("cave_loop".into())
    );

    // Empty active metadata clears the request.
    ambition_platformer_primitives::lifecycle::session_world_component_mut::<ActiveRoomMetadata>(
        app.world_mut(),
    )
    .expect("session active-room metadata")
        .0
        .music_track = None;
    app.update();
    assert_eq!(
        ambition_platformer_primitives::lifecycle::session_world_component::<RoomMusicRequest>(app.world()).expect("session room music").desired_track,
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
    };
    let set = RoomSet::from_parts(
        "hub",
        vec![
            spec_with(m_hub.clone(), "hub"),
            spec_with(m_lab.clone(), "lab"),
        ],
        Vec::new(),
    );
    ambition_platformer_primitives::lifecycle::insert_session_world_component(app.world_mut(), set);
    ambition_platformer_primitives::lifecycle::insert_session_world_component(app.world_mut(), ActiveRoomMetadata::default());
    app.add_systems(Update, sync_active_room_metadata);
    app.update();
    assert_eq!(&ambition_platformer_primitives::lifecycle::session_world_component::<ActiveRoomMetadata>(app.world()).expect("session active-room metadata").0, &m_hub);

    ambition_platformer_primitives::lifecycle::session_world_component_mut::<RoomSet>(app.world_mut()).expect("session room set").set_active(1);
    app.update();
    assert_eq!(&ambition_platformer_primitives::lifecycle::session_world_component::<ActiveRoomMetadata>(app.world()).expect("session active-room metadata").0, &m_lab);
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
        },
        gallery: true,              // takes effect — a.gallery was false (merge ORs)
        mode: Some("sanic".into()), // takes effect — a.mode was None
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

    // ...and the first non-empty value still wins, so one level cannot
    // re-home an area another level already claimed for its ruleset.
    let mut a = RoomMetadata {
        mode: Some("sanic".into()),
        ..Default::default()
    };
    a.merge(RoomMetadata {
        mode: Some("CONFLICT".into()),
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
fn kinematic_path_spec_matches_id_accepts_compacted_form() {
    use crate::world::rooms::KinematicPathSpec;
    use ambition_engine_core::KinematicPath;

    // Spec id is the `compact_path_name`-stripped form
    // (`enemy_patrol_a`); the authored reference uses the raw
    // snake-of-name (`enemy_patrol_path_a`). matches_id must accept
    // both.
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
