//! The dressing of a room: the art of its theme that is not a parallax panel.
//!
//! A theme has a terrain skin, which is the art a room lays on its collision
//! blocks (a fill, a cap, an underside, a side shade and a one-way platform),
//! a picture of motes, the small things that drift in its air, and a picture
//! of decor, the small things that stand on its ground. The parts
//! are authored and published by the art submodule
//! (`tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/terrain/`,
//! which says what each one is and how it repeats), and they are keyed by the
//! theme of the room, as the parallax layers are. A theme with no part on
//! disk has none here: its blocks keep the tile of their kind, and its air
//! is empty.
//!
//! A part is a texture and nothing else, so a room has its dressing with
//! each shader off.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use ambition_asset_manager::{
    AssetEntry, AssetId, AssetKind, AssetManifest, MissingAssetPolicy, PreloadGroup,
};

use super::{load_sheet_image, ParallaxTheme};

/// One picture of the dressing of a theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoomDressingPart {
    /// The inside of a solid block. It repeats each way.
    Fill,
    /// A top edge that nothing covers. It repeats along the edge.
    Cap,
    /// A bottom edge that nothing covers. It repeats along the edge.
    Under,
    /// A left edge that nothing covers, and mirrored a right one. It repeats
    /// down the edge.
    Side,
    /// A one-way platform. It repeats each way.
    OneWay,
    /// The motes of the theme: a strip of [`MOTE_VARIANTS`] squares, each a
    /// small thing that drifts in the air (dust, an ember, a bubble).
    Motes,
    /// The ground decor of the theme: a strip of [`DECOR_VARIANTS`] squares,
    /// each a small thing that stands on the ground (a barrel, a crystal, a
    /// stone lantern).
    Decor,
    /// The door of the theme: the art of a door of a room of that theme. It
    /// has the shape and the size of the door of the entity sheet.
    Door,
    /// The shadow a body throws on the ground: a soft ellipse of the dark of
    /// the theme.
    Shadow,
    /// A ladder: it repeats down the ladder, and along one that is wide.
    Ladder,
    /// The body of clear water. It repeats each way.
    WaterClear,
    /// The body of murky water. It repeats each way.
    WaterMurky,
    /// The line of the surface of a body of water. It repeats along it.
    WaterSurface,
    /// A wall a blink goes through: a field of light. It repeats each way.
    BlinkSoft,
    /// A wall no blink goes through: armour. It repeats each way.
    BlinkHard,
    /// The line of light on an open edge of a blink wall. It repeats along
    /// it, and its top row is the outer side.
    BlinkEdge,
    /// The body of a surface that sends a body back to its start. It repeats
    /// each way.
    HazardFill,
    /// The row of spikes on an open edge of that surface. It repeats along
    /// it, and its top row is the outer side.
    HazardEdge,
    /// The pools of light of the decor: the same squares as [`Self::Decor`],
    /// one for each thing. The square of a thing that gives no light is
    /// empty.
    DecorGlow,
}

/// How many squares the decor picture of a theme has, side by side.
pub const DECOR_VARIANTS: u32 = 6;
/// The side of one square of a decor picture, in pixels.
pub const DECOR_CELL_PX: u32 = 96;
/// How many squares the mote picture of a theme has, side by side.
pub const MOTE_VARIANTS: u32 = 4;
/// The side of one square of a mote picture, in pixels.
pub const MOTE_CELL_PX: u32 = 32;

impl RoomDressingPart {
    pub const ALL: &'static [Self] =
        &[Self::Fill, Self::Cap, Self::Under, Self::Side, Self::OneWay, Self::Motes, Self::Decor, Self::Door,
            Self::Shadow, Self::Ladder, Self::WaterClear, Self::WaterMurky, Self::WaterSurface,
            Self::BlinkSoft, Self::BlinkHard, Self::BlinkEdge, Self::HazardFill, Self::HazardEdge,
            Self::DecorGlow];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Fill => "fill",
            Self::Cap => "cap",
            Self::Under => "under",
            Self::Side => "side",
            Self::OneWay => "oneway",
            Self::Motes => "motes",
            Self::Decor => "decor",
            Self::Door => "door",
            Self::Shadow => "shadow",
            Self::Ladder => "ladder",
            Self::WaterClear => "water_clear",
            Self::WaterMurky => "water_murky",
            Self::WaterSurface => "water_surface",
            Self::BlinkSoft => "blink_soft",
            Self::BlinkHard => "blink_hard",
            Self::BlinkEdge => "blink_edge",
            Self::HazardFill => "hazard_fill",
            Self::HazardEdge => "hazard_edge",
            Self::DecorGlow => "decor_glow",
        }
    }

    pub fn relative_path(self, theme: ParallaxTheme) -> String {
        format!("room_dressing/{}_{}.png", theme.key(), self.key())
    }
}

/// Stable [`AssetId`] of one part of the dressing of one theme: `room.dressing.<theme>.<part>`.
pub fn room_dressing_asset_id(theme: ParallaxTheme, part: RoomDressingPart) -> AssetId {
    AssetId::new(format!("room.dressing.{}.{}", theme.key(), part.key()))
}

/// Add each part of each theme's dressing to the optional-image manifest. A
/// part with no file is not an error: the block keeps the tile of its kind.
///
/// A part has no smaller tiers. Its pictures are 128 px and less, and nothing
/// lowers the quality of art for cost.
pub(crate) fn insert_room_dressing_entries(manifest: &mut AssetManifest) {
    for &theme in ParallaxTheme::ALL {
        for &part in RoomDressingPart::ALL {
            manifest.insert(
                AssetEntry::new(
                    room_dressing_asset_id(theme, part),
                    AssetKind::Image,
                    part.relative_path(theme),
                )
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::Zone),
            );
        }
    }
}

/// The loaded parts of the dressing of the themes that are resident.
#[derive(Default, Clone)]
pub struct RoomDressingSet {
    handles: HashMap<(ParallaxTheme, RoomDressingPart), Handle<Image>>,
    /// The themes whose dressing was asked for. A theme in this set with no
    /// handle has no dressing on disk: a reader stops waiting for it.
    attempted: HashSet<ParallaxTheme>,
}

impl RoomDressingSet {
    /// Ask for each part of `theme`'s dressing that is not loaded. Returns how
    /// many handles it added.
    pub fn ensure_theme_loaded(
        &mut self,
        catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
        asset_server: &AssetServer,
        theme: ParallaxTheme,
    ) -> usize {
        self.attempted.insert(theme);
        let mut added = 0;
        for &part in RoomDressingPart::ALL {
            if self.handles.contains_key(&(theme, part)) {
                continue;
            }
            let Some(path) = catalog.try_path_for_load(&room_dressing_asset_id(theme, part)) else {
                continue;
            };
            self.handles
                .insert((theme, part), load_sheet_image(asset_server, "room-dressing", path));
            added += 1;
        }
        added
    }

    pub fn get(&self, theme: ParallaxTheme, part: RoomDressingPart) -> Option<&Handle<Image>> {
        self.handles.get(&(theme, part))
    }

    /// True when `theme`'s dressing was asked for: what [`Self::get`] says for it
    /// now is the answer, and not "not yet".
    pub fn attempted(&self, theme: ParallaxTheme) -> bool {
        self.attempted.contains(&theme)
    }

    /// Drop the dressing of each theme that `keep` refuses. The caller has the
    /// rule (the parallax residency rule of the app: a dressing is kept for as
    /// long as the parallax of its theme). Returns how many handles it
    /// dropped. A block that draws a part holds its own handle.
    pub fn retain_themes(&mut self, keep: impl Fn(ParallaxTheme) -> bool) -> usize {
        let before = self.handles.len();
        self.handles.retain(|(theme, _), _| keep(*theme));
        self.attempted.retain(|theme| keep(*theme));
        before - self.handles.len()
    }

    /// Put a handle in the set with no file behind it. For a test of a
    /// reader.
    pub fn insert(&mut self, theme: ParallaxTheme, part: RoomDressingPart, handle: Handle<Image>) {
        self.attempted.insert(theme);
        self.handles.insert((theme, part), handle);
    }

    /// Record that `theme` was asked for and has no dressing. For a test of a
    /// reader.
    pub fn mark_attempted(&mut self, theme: ParallaxTheme) {
        self.attempted.insert(theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The manifest has an entry for each part of each theme, at the path the
    /// art submodule publishes to. The control is the count: a manifest that
    /// had one entry would pass a test that looked up one id.
    #[test]
    fn the_manifest_names_each_part_of_each_theme() {
        let mut manifest = AssetManifest::new();
        insert_room_dressing_entries(&mut manifest);
        let mut found = 0;
        for &theme in ParallaxTheme::ALL {
            for &part in RoomDressingPart::ALL {
                let entry = manifest
                    .get(&room_dressing_asset_id(theme, part))
                    .unwrap_or_else(|| panic!("no entry for {theme:?} {part:?}"));
                assert_eq!(
                    entry.logical_path,
                    format!("room_dressing/{}_{}.png", theme.key(), part.key())
                );
                found += 1;
            }
        }
        assert_eq!(found, ParallaxTheme::ALL.len() * RoomDressingPart::ALL.len());
    }

    /// A retired theme is not "asked for" any more: a room that comes back to
    /// it must wait for its dressing again, and must not read "it has none".
    #[test]
    fn a_retired_theme_is_asked_for_again() {
        let mut set = RoomDressingSet::default();
        set.insert(ParallaxTheme::Cave, RoomDressingPart::Fill, Handle::default());
        set.mark_attempted(ParallaxTheme::Lab);
        assert!(set.attempted(ParallaxTheme::Cave) && set.attempted(ParallaxTheme::Lab));
        assert_eq!(set.retain_themes(|theme| theme == ParallaxTheme::Lab), 1);
        assert!(!set.attempted(ParallaxTheme::Cave));
        assert!(set.get(ParallaxTheme::Cave, RoomDressingPart::Fill).is_none());
        assert!(set.attempted(ParallaxTheme::Lab), "control: a kept theme stays asked for");
    }
}
