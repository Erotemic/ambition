//! Convert level fields and named entity layers into runtime room facts.

use super::super::project::{LdtkLayerInstance, LdtkLevel};

impl LdtkLevel {
    pub(crate) fn raw_active_area(&self) -> Option<String> {
        self.field_string("activeArea")
    }

    pub fn active_area(&self) -> String {
        self.raw_active_area()
            .map(|area| area.trim().to_string())
            .filter(|area| !area.is_empty())
            .unwrap_or_else(|| self.identifier.clone())
    }

    /// Read the optional biome metadata level fields. Empty/None values
    /// stay None so the active-area-merge in `compose_runtime_area`
    /// only takes the first non-empty value per active area.
    pub fn level_metadata(&self) -> ambition_platformer2d_world::rooms::RoomMetadata {
        let take = |name: &str| {
            self.field_string(name)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        let take_count = |name: &str| {
            self.field_i32(name)
                .filter(|value| *value >= 0)
                .map(|value| value as usize)
        };
        // A distance in whole pixels. Zero is valid ("out as soon as you cross my
        // edge"). Negative values are rejected, not clamped: a negative margin
        // would put the out-of-bounds line inside the room.
        let take_px = |name: &str| self.field_i32(name).filter(|px| *px >= 0);
        ambition_platformer2d_world::rooms::RoomMetadata {
            biome: take("biome"),
            music_track: take("music_track"),
            fight_music_track: take("fight_music_track"),
            ambient_profile: take("ambient_profile"),
            visual_theme: take("visual_theme"),
            visual_profile: ambition_platformer2d_world::rooms::RoomVisualProfile {
                id: take("visual_profile").or_else(|| take("visual_profile_id")),
                parallax_theme: take("parallax_theme"),
                palette: take("palette"),
                lighting_hint: take("lighting_hint"),
                foreground_treatment: take("foreground_treatment"),
                sky_scroll_px_s: self.field_i32("sky_scroll").filter(|px_s| *px_s != 0),
            },
            nameplate_policy: ambition_platformer2d_world::rooms::RoomNameplatePolicy {
                full_opacity_count: take_count("nameplate_full_opacity_count"),
                fade_out_count: take_count("nameplate_fade_out_count"),
                // Not an LDtk field yet: only rooms with a cast need it, and those
                // are declared in Rust.
                label_driven_bodies: None,
            },
            gallery: self.field_bool("gallery").unwrap_or(false),
            practice: self.field_bool("practice").unwrap_or(false),
            mode: take("mode"),
            fall_out_margin: take_px("fall_out_margin"),
            side_out_margin: take_px("side_out_margin"),
            rise_out_margin: take_px("rise_out_margin"),
            // Where finishing this level leads. An empty string is the same
            // answer as an unset field — "no successor, loop in place" — which
            // is why it goes through `take` rather than `field_string`: an
            // author who clears the box in the editor means it.
            next_room: take("next_room"),
            entry_cutscene: take("entry_cutscene"),
            while_live: take("while_live"),
            title: take("title"),
            fall_rescue: take("fall_rescue"),
            boss_reward_drop: take("boss_reward_drop"),
        }
    }

    pub fn ambition_layer(&self) -> Option<&LdtkLayerInstance> {
        self.layer_instances
            .iter()
            .find(|layer| layer.identifier == super::super::intgrid::AMBITION_LAYER)
    }
}
