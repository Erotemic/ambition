use super::*;

#[test]
fn map_zoom_in_clamps_to_max() {
    let mut map = MapMenuState::default();
    for _ in 0..20 {
        map.zoom_in();
    }
    assert!(map.zoom <= MAP_ZOOM_MAX + 1e-4);
    assert!(map.zoom > 1.0);
}

#[test]
fn map_zoom_out_clamps_to_min() {
    let mut map = MapMenuState::default();
    for _ in 0..20 {
        map.zoom_out();
    }
    assert!(map.zoom >= MAP_ZOOM_MIN - 1e-4);
    assert!(map.zoom < 1.0);
}

#[test]
fn map_zoom_reset_returns_to_one() {
    let mut map = MapMenuState::default();
    map.zoom_in();
    map.zoom_in();
    map.zoom_reset();
    assert_eq!(map.zoom, 1.0);
}

#[test]
fn map_zoom_step_is_round_trip_friendly() {
    let mut map = MapMenuState::default();
    let initial = map.zoom;
    map.zoom_in();
    let zoomed = map.zoom;
    map.zoom_out();
    assert!(
        (map.zoom - initial).abs() < 1e-3,
        "zoom_in then zoom_out should return near 1.0 (got {} from {})",
        map.zoom,
        zoomed
    );
}

#[test]
fn short_room_label_initializes_underscore_id() {
    assert_eq!(short_room_label("central_hub_complex"), "CHC");
    assert_eq!(short_room_label("water_world"), "WW");
    assert_eq!(short_room_label("goblin_encounter"), "GE");
}

#[test]
fn short_room_label_uppercase_truncates_single_word() {
    assert_eq!(short_room_label("alpha"), "ALPHA");
    assert_eq!(short_room_label("verylongname"), "VERYLONG");
}
