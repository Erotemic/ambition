use super::*;

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
        moving_platforms: Vec::new(),
    }
}

#[test]
fn active_metadata_returns_active_room_metadata() {
    let m1 = RoomMetadata {
        biome: Some("hub".into()),
        music_track: Some("hub_loop".into()),
        ambient_profile: None,
        visual_theme: None,
    };
    let m2 = RoomMetadata {
        biome: Some("cave".into()),
        music_track: Some("cave_loop".into()),
        ambient_profile: Some("damp".into()),
        visual_theme: None,
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
    app.insert_resource(ActiveRoomMetadata(RoomMetadata {
        biome: Some("cave".into()),
        music_track: Some("cave_loop".into()),
        ambient_profile: None,
        visual_theme: None,
    }));
    app.insert_resource(RoomMusicRequest::default());
    app.add_systems(Update, sync_room_music_request);
    app.update();
    assert_eq!(
        app.world().resource::<RoomMusicRequest>().desired_track,
        Some("cave_loop".into())
    );

    // Empty active metadata clears the request.
    app.world_mut()
        .resource_mut::<ActiveRoomMetadata>()
        .0
        .music_track = None;
    app.update();
    assert_eq!(
        app.world().resource::<RoomMusicRequest>().desired_track,
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
    };
    let m_lab = RoomMetadata {
        biome: Some("lab".into()),
        music_track: Some("lab_loop".into()),
        ambient_profile: None,
        visual_theme: None,
    };
    let set = RoomSet::from_parts(
        "hub",
        vec![
            spec_with(m_hub.clone(), "hub"),
            spec_with(m_lab.clone(), "lab"),
        ],
        Vec::new(),
    );
    app.insert_resource(set);
    app.insert_resource(ActiveRoomMetadata::default());
    app.add_systems(Update, sync_active_room_metadata);
    app.update();
    assert_eq!(&app.world().resource::<ActiveRoomMetadata>().0, &m_hub);

    app.world_mut().resource_mut::<RoomSet>().set_active(1);
    app.update();
    assert_eq!(&app.world().resource::<ActiveRoomMetadata>().0, &m_lab);
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
    };
    assert!(!m.is_empty());
}

#[test]
fn room_metadata_merge_preserves_existing_values() {
    let mut a = RoomMetadata {
        biome: Some("hub".into()),
        music_track: None,
        ambient_profile: None,
        visual_theme: Some("blue".into()),
    };
    let b = RoomMetadata {
        biome: Some("CONFLICT".into()),        // ignored — a.biome wins
        music_track: Some("hub_loop".into()),  // takes effect — a.music_track was None
        ambient_profile: Some("damp".into()),  // takes effect
        visual_theme: Some("CONFLICT".into()), // ignored
    };
    a.merge(b);
    assert_eq!(a.biome.as_deref(), Some("hub"));
    assert_eq!(a.music_track.as_deref(), Some("hub_loop"));
    assert_eq!(a.ambient_profile.as_deref(), Some("damp"));
    assert_eq!(a.visual_theme.as_deref(), Some("blue"));
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
