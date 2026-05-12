use bevy::prelude::*;

use crate::rooms::RoomMetadata;

/// One asset-backed parallax layer in a profile.
#[derive(Clone, Copy, Debug)]
pub struct ParallaxLayerProfile {
    pub name: &'static str,
    pub asset_path: &'static str,
    pub factor: Vec2,
    pub offset: Vec2,
    pub z: f32,
    pub size: Vec2,
    pub tile_x: bool,
    pub tile_y: bool,
}

/// A named background stack. First layer spawns behind later layers. The
/// placeholder profiles intentionally sit close to the world backplane so they
/// stay visible even while the sandbox still uses dark blockout visuals.
#[derive(Clone, Copy, Debug)]
pub struct ParallaxProfile {
    pub name: &'static str,
    pub layers: &'static [ParallaxLayerProfile],
}

const fn layer(
    name: &'static str,
    asset_path: &'static str,
    factor: Vec2,
    offset: Vec2,
    z: f32,
    size: Vec2,
    tile_x: bool,
    tile_y: bool,
) -> ParallaxLayerProfile {
    ParallaxLayerProfile {
        name,
        asset_path,
        factor,
        offset,
        z,
        size,
        tile_x,
        tile_y,
    }
}

const DEFAULT_LAYERS: &[ParallaxLayerProfile] = &[
    layer(
        "sky",
        "backgrounds/default/sky.png",
        Vec2::ZERO,
        Vec2::ZERO,
        -6.0,
        Vec2::new(5200.0, 3200.0),
        false,
        false,
    ),
    layer(
        "far silhouettes",
        "backgrounds/default/far.png",
        Vec2::new(0.12, 0.08),
        Vec2::new(0.0, -120.0),
        -4.0,
        Vec2::new(6400.0, 2600.0),
        true,
        false,
    ),
    layer(
        "mid structures",
        "backgrounds/default/mid.png",
        Vec2::new(0.28, 0.18),
        Vec2::new(0.0, -60.0),
        -2.0,
        Vec2::new(6400.0, 2400.0),
        true,
        false,
    ),
    layer(
        "near foreground",
        "backgrounds/default/near.png",
        Vec2::new(0.55, 0.35),
        Vec2::ZERO,
        1.5,
        Vec2::new(6400.0, 2400.0),
        true,
        true,
    ),
];

const HUB_LAYERS: &[ParallaxLayerProfile] = &[
    layer(
        "sky",
        "backgrounds/hub/sky.png",
        Vec2::ZERO,
        Vec2::ZERO,
        -6.0,
        Vec2::new(5200.0, 3200.0),
        false,
        false,
    ),
    layer(
        "far skyline",
        "backgrounds/hub/far.png",
        Vec2::new(0.10, 0.06),
        Vec2::new(0.0, -110.0),
        -4.2,
        Vec2::new(6400.0, 2500.0),
        true,
        false,
    ),
    layer(
        "mid hub structures",
        "backgrounds/hub/mid.png",
        Vec2::new(0.24, 0.16),
        Vec2::new(0.0, -55.0),
        -2.1,
        Vec2::new(6400.0, 2350.0),
        true,
        false,
    ),
    layer(
        "near tech foreground",
        "backgrounds/hub/near.png",
        Vec2::new(0.52, 0.32),
        Vec2::ZERO,
        1.4,
        Vec2::new(6400.0, 2400.0),
        true,
        true,
    ),
];

const LAB_LAYERS: &[ParallaxLayerProfile] = &[
    layer(
        "sky",
        "backgrounds/lab/sky.png",
        Vec2::ZERO,
        Vec2::new(0.0, -10.0),
        -6.0,
        Vec2::new(5200.0, 3200.0),
        false,
        false,
    ),
    layer(
        "far chambers",
        "backgrounds/lab/far.png",
        Vec2::new(0.11, 0.07),
        Vec2::new(0.0, -100.0),
        -4.0,
        Vec2::new(6400.0, 2450.0),
        true,
        false,
    ),
    layer(
        "mid machinery",
        "backgrounds/lab/mid.png",
        Vec2::new(0.26, 0.18),
        Vec2::new(0.0, -50.0),
        -2.0,
        Vec2::new(6400.0, 2300.0),
        true,
        false,
    ),
    layer(
        "near cables",
        "backgrounds/lab/near.png",
        Vec2::new(0.48, 0.28),
        Vec2::ZERO,
        1.2,
        Vec2::new(6400.0, 2400.0),
        true,
        true,
    ),
];

const BASEMENT_LAYERS: &[ParallaxLayerProfile] = &[
    layer(
        "sky",
        "backgrounds/basement/sky.png",
        Vec2::ZERO,
        Vec2::new(0.0, -20.0),
        -6.0,
        Vec2::new(5200.0, 3200.0),
        false,
        false,
    ),
    layer(
        "far masonry",
        "backgrounds/basement/far.png",
        Vec2::new(0.10, 0.06),
        Vec2::new(0.0, -125.0),
        -4.2,
        Vec2::new(6400.0, 2600.0),
        true,
        false,
    ),
    layer(
        "mid ruins",
        "backgrounds/basement/mid.png",
        Vec2::new(0.22, 0.14),
        Vec2::new(0.0, -70.0),
        -2.0,
        Vec2::new(6400.0, 2400.0),
        true,
        false,
    ),
    layer(
        "near pillars",
        "backgrounds/basement/near.png",
        Vec2::new(0.45, 0.26),
        Vec2::ZERO,
        1.1,
        Vec2::new(6400.0, 2400.0),
        true,
        true,
    ),
];

const COVE_LAYERS: &[ParallaxLayerProfile] = &[
    layer(
        "sky",
        "backgrounds/cove/sky.png",
        Vec2::ZERO,
        Vec2::ZERO,
        -6.0,
        Vec2::new(5200.0, 3200.0),
        false,
        false,
    ),
    layer(
        "far coast",
        "backgrounds/cove/far.png",
        Vec2::new(0.10, 0.07),
        Vec2::new(0.0, -95.0),
        -4.0,
        Vec2::new(6400.0, 2550.0),
        true,
        false,
    ),
    layer(
        "mid palms",
        "backgrounds/cove/mid.png",
        Vec2::new(0.24, 0.16),
        Vec2::new(0.0, -60.0),
        -2.0,
        Vec2::new(6400.0, 2325.0),
        true,
        false,
    ),
    layer(
        "near reeds",
        "backgrounds/cove/near.png",
        Vec2::new(0.46, 0.26),
        Vec2::ZERO,
        1.1,
        Vec2::new(6400.0, 2400.0),
        true,
        true,
    ),
];

const SKYBRIDGE_LAYERS: &[ParallaxLayerProfile] = &[
    layer(
        "sky",
        "backgrounds/skybridge/sky.png",
        Vec2::ZERO,
        Vec2::new(0.0, 10.0),
        -6.0,
        Vec2::new(5200.0, 3200.0),
        false,
        false,
    ),
    layer(
        "far clouds",
        "backgrounds/skybridge/far.png",
        Vec2::new(0.06, 0.03),
        Vec2::new(0.0, -80.0),
        -4.2,
        Vec2::new(6400.0, 2500.0),
        true,
        false,
    ),
    layer(
        "mid sky bridges",
        "backgrounds/skybridge/mid.png",
        Vec2::new(0.16, 0.10),
        Vec2::new(0.0, -30.0),
        -2.0,
        Vec2::new(6400.0, 2200.0),
        true,
        false,
    ),
    layer(
        "near gusts",
        "backgrounds/skybridge/near.png",
        Vec2::new(0.32, 0.18),
        Vec2::ZERO,
        1.0,
        Vec2::new(6400.0, 2400.0),
        true,
        true,
    ),
];

const BOSS_LAYERS: &[ParallaxLayerProfile] = &[
    layer(
        "sky",
        "backgrounds/boss/sky.png",
        Vec2::ZERO,
        Vec2::ZERO,
        -6.0,
        Vec2::new(5200.0, 3200.0),
        false,
        false,
    ),
    layer(
        "far shards",
        "backgrounds/boss/far.png",
        Vec2::new(0.12, 0.08),
        Vec2::new(0.0, -120.0),
        -4.3,
        Vec2::new(6400.0, 2500.0),
        true,
        false,
    ),
    layer(
        "mid boss arena",
        "backgrounds/boss/mid.png",
        Vec2::new(0.26, 0.16),
        Vec2::new(0.0, -40.0),
        -2.0,
        Vec2::new(6400.0, 2300.0),
        true,
        false,
    ),
    layer(
        "near spikes",
        "backgrounds/boss/near.png",
        Vec2::new(0.56, 0.30),
        Vec2::ZERO,
        1.3,
        Vec2::new(6400.0, 2400.0),
        true,
        true,
    ),
];

const WATER_LAYERS: &[ParallaxLayerProfile] = &[
    layer(
        "sky",
        "backgrounds/water/sky.png",
        Vec2::ZERO,
        Vec2::ZERO,
        -6.0,
        Vec2::new(5200.0, 3200.0),
        false,
        false,
    ),
    layer(
        "far waves",
        "backgrounds/water/far.png",
        Vec2::new(0.08, 0.05),
        Vec2::new(0.0, -100.0),
        -4.1,
        Vec2::new(6400.0, 2500.0),
        true,
        false,
    ),
    layer(
        "mid tide",
        "backgrounds/water/mid.png",
        Vec2::new(0.18, 0.12),
        Vec2::new(0.0, -40.0),
        -2.0,
        Vec2::new(6400.0, 2200.0),
        true,
        false,
    ),
    layer(
        "near kelp",
        "backgrounds/water/near.png",
        Vec2::new(0.36, 0.22),
        Vec2::ZERO,
        1.0,
        Vec2::new(6400.0, 2400.0),
        true,
        true,
    ),
];

const CAVE_LAYERS: &[ParallaxLayerProfile] = &[
    layer(
        "sky",
        "backgrounds/cave/sky.png",
        Vec2::ZERO,
        Vec2::new(0.0, -30.0),
        -6.0,
        Vec2::new(5200.0, 3200.0),
        false,
        false,
    ),
    layer(
        "far cavern",
        "backgrounds/cave/far.png",
        Vec2::new(0.10, 0.06),
        Vec2::new(0.0, -115.0),
        -4.2,
        Vec2::new(6400.0, 2600.0),
        true,
        false,
    ),
    layer(
        "mid crystals",
        "backgrounds/cave/mid.png",
        Vec2::new(0.20, 0.12),
        Vec2::new(0.0, -65.0),
        -2.0,
        Vec2::new(6400.0, 2350.0),
        true,
        false,
    ),
    layer(
        "near drips",
        "backgrounds/cave/near.png",
        Vec2::new(0.40, 0.24),
        Vec2::ZERO,
        1.0,
        Vec2::new(6400.0, 2400.0),
        true,
        true,
    ),
];

pub const fn default_parallax_profile() -> ParallaxProfile {
    ParallaxProfile {
        name: "default",
        layers: DEFAULT_LAYERS,
    }
}

pub const fn hub_parallax_profile() -> ParallaxProfile {
    ParallaxProfile {
        name: "hub",
        layers: HUB_LAYERS,
    }
}

pub const fn lab_parallax_profile() -> ParallaxProfile {
    ParallaxProfile {
        name: "lab",
        layers: LAB_LAYERS,
    }
}

pub const fn basement_parallax_profile() -> ParallaxProfile {
    ParallaxProfile {
        name: "basement",
        layers: BASEMENT_LAYERS,
    }
}

pub const fn cove_parallax_profile() -> ParallaxProfile {
    ParallaxProfile {
        name: "cove",
        layers: COVE_LAYERS,
    }
}

pub const fn skybridge_parallax_profile() -> ParallaxProfile {
    ParallaxProfile {
        name: "skybridge",
        layers: SKYBRIDGE_LAYERS,
    }
}

pub const fn boss_parallax_profile() -> ParallaxProfile {
    ParallaxProfile {
        name: "boss",
        layers: BOSS_LAYERS,
    }
}

pub const fn water_parallax_profile() -> ParallaxProfile {
    ParallaxProfile {
        name: "water",
        layers: WATER_LAYERS,
    }
}

pub const fn cave_parallax_profile() -> ParallaxProfile {
    ParallaxProfile {
        name: "cave",
        layers: CAVE_LAYERS,
    }
}

pub fn parallax_profile_named(name: &str) -> Option<ParallaxProfile> {
    Some(match name {
        "default" => default_parallax_profile(),
        "hub" => hub_parallax_profile(),
        "lab" => lab_parallax_profile(),
        "basement" => basement_parallax_profile(),
        "cove" => cove_parallax_profile(),
        "skybridge" => skybridge_parallax_profile(),
        "boss" => boss_parallax_profile(),
        "water" => water_parallax_profile(),
        "cave" => cave_parallax_profile(),
        _ => return None,
    })
}

pub fn select_parallax_profile(metadata: &RoomMetadata) -> ParallaxProfile {
    if let Some(theme) = metadata.visual_theme.as_deref() {
        if let Some(profile) = parallax_profile_named(theme) {
            return profile;
        }
    }
    match metadata.biome.as_deref() {
        Some("hub") => hub_parallax_profile(),
        Some("lab") | Some("tower") => lab_parallax_profile(),
        Some("basement") => basement_parallax_profile(),
        Some("cove") | Some("cantina") => cove_parallax_profile(),
        Some("skybridge") | Some("mob_arena") => skybridge_parallax_profile(),
        Some("boss") => boss_parallax_profile(),
        Some("water") => water_parallax_profile(),
        Some("cave") => cave_parallax_profile(),
        _ => default_parallax_profile(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_profile_sane(profile: ParallaxProfile) {
        assert!(profile.layers.len() >= 3);
        for pair in profile.layers.windows(2) {
            assert!(pair[0].z < pair[1].z, "{} should be behind {}", pair[0].name, pair[1].name);
        }
        for layer in profile.layers {
            assert!(layer.asset_path.starts_with("backgrounds/"));
            assert!(layer.asset_path.ends_with(".png"));
            assert!(layer.asset_path.contains(profile.name), "{} should live in {}", layer.asset_path, profile.name);
        }
    }

    #[test]
    fn all_profiles_are_back_to_front() {
        let profiles = [
            default_parallax_profile(),
            hub_parallax_profile(),
            lab_parallax_profile(),
            basement_parallax_profile(),
            cove_parallax_profile(),
            skybridge_parallax_profile(),
            boss_parallax_profile(),
            water_parallax_profile(),
            cave_parallax_profile(),
        ];
        for profile in profiles {
            assert_profile_sane(profile);
        }
    }

    #[test]
    fn biome_selection_maps_known_groups() {
        let mut meta = RoomMetadata::default();
        meta.biome = Some("tower".into());
        assert_eq!(select_parallax_profile(&meta).name, "lab");
        meta.biome = Some("cantina".into());
        assert_eq!(select_parallax_profile(&meta).name, "cove");
        meta.biome = Some("mob_arena".into());
        assert_eq!(select_parallax_profile(&meta).name, "skybridge");
        meta.biome = Some("water".into());
        assert_eq!(select_parallax_profile(&meta).name, "water");
    }

    #[test]
    fn visual_theme_can_override_biome() {
        let mut meta = RoomMetadata::default();
        meta.biome = Some("hub".into());
        meta.visual_theme = Some("boss".into());
        assert_eq!(select_parallax_profile(&meta).name, "boss");
    }
}
