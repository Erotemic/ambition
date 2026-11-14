//! Unit tests for the editable tuning mirrors: round-tripping
//! `EditableAbilitySet`/movement edits through their `engine_core` counterparts.

use super::*;

#[test]
fn editable_ability_set_round_trips_through_engine() {
    // Default → engine → editable should equal the original.
    let original = EditableAbilitySet::default();
    let engine = original.as_engine();
    let restored = EditableAbilitySet::from(engine);
    assert_eq!(original.move_horizontal, restored.move_horizontal);
    assert_eq!(original.glide, restored.glide);
    assert_eq!(original.swim, restored.swim);
    assert_eq!(original.ledge_grab, restored.ledge_grab);
}

#[test]
fn editable_movement_tuning_round_trips_through_engine() {
    let original = EditableMovementTuning::default();
    let engine = original.as_engine();
    let restored = EditableMovementTuning::from(engine);
    // Spot-check a handful of fields including the recently-added
    // glide tuning.
    assert!((original.gravity - restored.gravity).abs() < 1e-3);
    assert!((original.jump_speed - restored.jump_speed).abs() < 1e-3);
    assert!((original.glide_fall_speed - restored.glide_fall_speed).abs() < 1e-3);
    assert!((original.glide_air_accel - restored.glide_air_accel).abs() < 1e-3);
    assert_eq!(original.air_jumps, restored.air_jumps);
}

#[test]
fn editable_player_stats_default_matches_constants() {
    let s = EditablePlayerStats::default();
    assert_eq!(s.health, EditablePlayerStats::DEFAULT_MAX_HEALTH);
    assert_eq!(s.max_health, EditablePlayerStats::DEFAULT_MAX_HEALTH);
    assert_eq!(s.mana, EditablePlayerStats::DEFAULT_MAX_MANA);
    assert_eq!(s.max_mana, EditablePlayerStats::DEFAULT_MAX_MANA);
    assert!(!s.invincible);
    assert!(!s.refill_now);
}

#[test]
fn debug_view_presets_drive_overlay_intent() {
    let mut tools = DeveloperTools::default();
    tools.apply_debug_view_mode(DebugViewMode::Collision, true);
    assert_eq!(tools.debug_view_mode, DebugViewMode::Collision);
    assert!(tools.show_world_blocks);
    assert!(tools.show_player_hitbox);
    assert!(tools.show_feature_hitboxes);
    assert!(tools.fill_debug_boxes);
    assert_eq!(tools.debug_art_mode, DebugArtMode::Hidden);

    tools.apply_debug_view_mode(DebugViewMode::Authoring, true);
    assert_eq!(tools.debug_view_mode, DebugViewMode::Authoring);
    assert!(tools.show_room_bounds);
    assert!(tools.show_world_blocks);
    assert!(tools.show_loading_zones);
    assert!(tools.show_player_hitbox);
    assert!(tools.show_player_vectors);
    assert!(tools.show_blink_preview);
    assert!(tools.show_combat_preview);
    assert!(tools.show_feature_hitboxes);
    assert!(tools.show_health_bars);
    assert!(tools.show_rebound_vectors);
    assert!(!tools.show_micro_grid);
    assert!(!tools.show_camera_frame);
    assert!(!tools.fill_debug_boxes);
    assert_eq!(tools.debug_art_mode, DebugArtMode::Normal);
}

#[test]
fn debug_art_mode_is_single_source_for_sprite_overrides() {
    let mut tools = DeveloperTools::default();
    tools.apply_debug_art_mode(DebugArtMode::Placeholder);
    assert!(tools.placeholder_sprites);
    assert!(!tools.hide_sprites);

    tools.apply_debug_art_mode(DebugArtMode::Hidden);
    assert!(!tools.placeholder_sprites);
    assert!(tools.hide_sprites);

    tools.apply_debug_art_mode(DebugArtMode::Normal);
    assert!(!tools.placeholder_sprites);
    assert!(!tools.hide_sprites);
}

#[test]
fn normalize_debug_modes_repairs_legacy_art_toggles() {
    let mut tools = DeveloperTools {
        debug_art_mode: DebugArtMode::Normal,
        hide_sprites: true,
        placeholder_sprites: true,
        ..DeveloperTools::default()
    };
    tools.normalize_debug_modes();
    assert_eq!(tools.debug_art_mode, DebugArtMode::Placeholder);
    assert!(tools.placeholder_sprites);
    assert!(!tools.hide_sprites);

    tools.debug_art_mode = DebugArtMode::Hidden;
    tools.hide_sprites = true;
    tools.placeholder_sprites = true;
    tools.normalize_debug_modes();
    assert_eq!(tools.debug_art_mode, DebugArtMode::Placeholder);
    assert!(tools.placeholder_sprites);
    assert!(!tools.hide_sprites);
}

/// GOBS-4: a debug view preset draws its observers by namespace, so an
/// observer installed later in that namespace joins it with no edit here; a
/// single observer chosen from the menu is drawn instead of the preset.
#[test]
fn a_debug_view_preset_draws_its_geometry_observers_by_namespace() {
    let mut tools = DeveloperTools::default();
    tools.apply_debug_view_mode(DebugViewMode::Combat, false);
    assert!(tools.show_observed_geometry, "the combat preset draws observed geometry");
    assert!(tools.draws_observer("combat.volumes"));
    assert!(tools.draws_observer("combat.a_later_observer"), "a later observer in the namespace joins the preset");
    assert!(!tools.draws_observer("collision.composed"), "the combat preset does not draw the walls");

    tools.apply_debug_view_mode(DebugViewMode::Triggers, false);
    for provider in ["world.regions", "interaction.reach", "portal.apertures"] {
        assert!(tools.draws_observer(provider), "the triggers preset draws {provider}");
    }
    assert!(!tools.draws_observer("combat.volumes"));

    tools.observed_geometry_only = Some("portal.apertures".to_string());
    assert!(!tools.draws_observer("world.regions"), "one chosen observer is drawn instead of the preset");

    tools.apply_debug_view_mode(DebugViewMode::All, false);
    assert!(tools.observed_geometry_only.is_none(), "a preset drops the one chosen observer");
    assert!(tools.draws_observer("navigation.graph") && tools.draws_observer("combat.volumes"), "all draws every observer");

    tools.apply_debug_view_mode(DebugViewMode::Gameplay, false);
    assert!(!tools.show_observed_geometry, "the gameplay preset draws no geometry");
}
