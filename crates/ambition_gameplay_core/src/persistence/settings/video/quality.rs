//! Global visual quality profile and resolved runtime/device budgets.
//!
//! The profile enum is only interpreted here. Render/asset subsystems consume
//! the resolved budget fields so Low/Medium/High never becomes a local dialect.

use serde::{Deserialize, Serialize};

use super::{cycle_next, cycle_prev};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VisualQualityProfile {
    Low,
    Medium,
    #[default]
    High,
    Ultra,
    Custom,
}

impl VisualQualityProfile {
    pub const ALL: [Self; 5] = [
        Self::Low,
        Self::Medium,
        Self::High,
        Self::Ultra,
        Self::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Ultra => "ultra",
            Self::Custom => "custom",
        }
    }

    pub fn next(self) -> Self {
        cycle_next(&Self::ALL, self, 2)
    }

    pub fn prev(self) -> Self {
        cycle_prev(&Self::ALL, self, 2)
    }
}

pub fn default_visual_quality_profile() -> VisualQualityProfile {
    if cfg!(target_os = "android") {
        VisualQualityProfile::Medium
    } else {
        VisualQualityProfile::High
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextureResolutionScale {
    Quarter,
    Half,
    #[default]
    Full,
}

impl TextureResolutionScale {
    pub const ALL: [Self; 3] = [Self::Quarter, Self::Half, Self::Full];

    pub fn scale_factor(self) -> f32 {
        match self {
            Self::Quarter => 0.25,
            Self::Half => 0.5,
            Self::Full => 1.0,
        }
    }

    pub fn folder_suffix(self) -> &'static str {
        match self {
            Self::Quarter => "_0_25x",
            Self::Half => "_0_5x",
            Self::Full => "",
        }
    }

    pub fn asset_id_suffix(self) -> Option<&'static str> {
        match self {
            Self::Quarter => Some("0_25x"),
            Self::Half => Some("0_5x"),
            Self::Full => None,
        }
    }

    pub fn asset_subdir(self, base: &str) -> String {
        format!("{base}{}", self.folder_suffix())
    }

    pub fn parallax_subdir(self) -> &'static str {
        match self {
            Self::Quarter => "backgrounds/parallax_layers_0_25x",
            Self::Half => "backgrounds/parallax_layers_0_5x",
            Self::Full => "backgrounds/parallax_layers",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PortalCaptureBudget {
    pub max_resolution: u32,
    pub texels_per_world_px: f32,
    pub recursion_depth: u32,
    pub max_active_captures: u32,
    pub max_updates_per_frame: u32,
    pub min_refresh_interval_s: f32,
    pub include_parallax: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpriteTextureBudget {
    pub resolution_scale: TextureResolutionScale,
    pub prefer_scaled_variants: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackgroundTextureBudget {
    pub resolution_scale: TextureResolutionScale,
    pub max_texture_resolution: u32,
    pub prefer_scaled_variants: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParallaxBudget {
    pub enabled: bool,
    pub max_layers: Option<usize>,
    pub resolution_scale: TextureResolutionScale,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShaderBudget {
    pub screen_shader_scale: f32,
    pub allow_expensive_materials: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticleBudget {
    pub max_particles: u32,
    pub spawn_rate_scale: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisualQualityBudget {
    pub portal: PortalCaptureBudget,
    pub sprites: SpriteTextureBudget,
    pub backgrounds: BackgroundTextureBudget,
    pub parallax: ParallaxBudget,
    pub shaders: ShaderBudget,
    pub particles: ParticleBudget,
}

impl VisualQualityBudget {
    pub fn for_profile(profile: VisualQualityProfile) -> Self {
        match profile {
            VisualQualityProfile::Low => Self {
                portal: PortalCaptureBudget {
                    max_resolution: 384,
                    texels_per_world_px: 0.25,
                    recursion_depth: 0,
                    max_active_captures: 1,
                    max_updates_per_frame: 1,
                    min_refresh_interval_s: 0.100,
                    include_parallax: false,
                },
                sprites: SpriteTextureBudget {
                    resolution_scale: TextureResolutionScale::Half,
                    prefer_scaled_variants: true,
                },
                backgrounds: BackgroundTextureBudget {
                    resolution_scale: TextureResolutionScale::Half,
                    max_texture_resolution: 1024,
                    prefer_scaled_variants: true,
                },
                parallax: ParallaxBudget {
                    enabled: true,
                    max_layers: Some(2),
                    resolution_scale: TextureResolutionScale::Half,
                },
                shaders: ShaderBudget {
                    screen_shader_scale: 0.5,
                    allow_expensive_materials: false,
                },
                particles: ParticleBudget {
                    max_particles: 128,
                    spawn_rate_scale: 0.5,
                },
            },
            VisualQualityProfile::Medium => Self {
                portal: PortalCaptureBudget {
                    max_resolution: 512,
                    texels_per_world_px: 0.50,
                    recursion_depth: 0,
                    max_active_captures: 1,
                    max_updates_per_frame: 1,
                    min_refresh_interval_s: 0.050,
                    include_parallax: false,
                },
                sprites: SpriteTextureBudget {
                    resolution_scale: TextureResolutionScale::Half,
                    prefer_scaled_variants: true,
                },
                backgrounds: BackgroundTextureBudget {
                    resolution_scale: TextureResolutionScale::Half,
                    max_texture_resolution: 1536,
                    prefer_scaled_variants: true,
                },
                parallax: ParallaxBudget {
                    enabled: true,
                    max_layers: Some(3),
                    resolution_scale: TextureResolutionScale::Half,
                },
                shaders: ShaderBudget {
                    screen_shader_scale: 0.75,
                    allow_expensive_materials: true,
                },
                particles: ParticleBudget {
                    max_particles: 256,
                    spawn_rate_scale: 0.75,
                },
            },
            VisualQualityProfile::High | VisualQualityProfile::Custom => Self {
                portal: PortalCaptureBudget {
                    max_resolution: 1024,
                    texels_per_world_px: 1.0,
                    recursion_depth: 1,
                    max_active_captures: 2,
                    max_updates_per_frame: 2,
                    min_refresh_interval_s: 0.0,
                    include_parallax: true,
                },
                sprites: SpriteTextureBudget {
                    resolution_scale: TextureResolutionScale::Full,
                    prefer_scaled_variants: false,
                },
                backgrounds: BackgroundTextureBudget {
                    resolution_scale: TextureResolutionScale::Full,
                    max_texture_resolution: 2048,
                    prefer_scaled_variants: false,
                },
                parallax: ParallaxBudget {
                    enabled: true,
                    max_layers: None,
                    resolution_scale: TextureResolutionScale::Full,
                },
                shaders: ShaderBudget {
                    screen_shader_scale: 1.0,
                    allow_expensive_materials: true,
                },
                particles: ParticleBudget {
                    max_particles: 512,
                    spawn_rate_scale: 1.0,
                },
            },
            VisualQualityProfile::Ultra => Self {
                portal: PortalCaptureBudget {
                    max_resolution: 2048,
                    texels_per_world_px: 1.0,
                    recursion_depth: 1,
                    max_active_captures: 4,
                    max_updates_per_frame: 4,
                    min_refresh_interval_s: 0.0,
                    include_parallax: true,
                },
                sprites: SpriteTextureBudget {
                    resolution_scale: TextureResolutionScale::Full,
                    prefer_scaled_variants: false,
                },
                backgrounds: BackgroundTextureBudget {
                    resolution_scale: TextureResolutionScale::Full,
                    max_texture_resolution: 4096,
                    prefer_scaled_variants: false,
                },
                parallax: ParallaxBudget {
                    enabled: true,
                    max_layers: None,
                    resolution_scale: TextureResolutionScale::Full,
                },
                shaders: ShaderBudget {
                    screen_shader_scale: 1.0,
                    allow_expensive_materials: true,
                },
                particles: ParticleBudget {
                    max_particles: 1024,
                    spawn_rate_scale: 1.0,
                },
            },
        }
    }

    pub fn clamp_all(&mut self) {
        self.portal.max_resolution = self.portal.max_resolution.clamp(128, 4096);
        self.portal.texels_per_world_px = self.portal.texels_per_world_px.clamp(0.05, 1.0);
        self.portal.recursion_depth = self.portal.recursion_depth.min(4);
        self.portal.max_active_captures = self.portal.max_active_captures.clamp(1, 16);
        self.portal.max_updates_per_frame = self.portal.max_updates_per_frame.clamp(1, 16);
        self.portal.min_refresh_interval_s = self.portal.min_refresh_interval_s.clamp(0.0, 1.0);
        self.backgrounds.max_texture_resolution =
            self.backgrounds.max_texture_resolution.clamp(256, 8192);
        if let Some(max_layers) = &mut self.parallax.max_layers {
            *max_layers = (*max_layers).min(16);
        }
        self.shaders.screen_shader_scale = self.shaders.screen_shader_scale.clamp(0.0, 1.0);
        self.particles.max_particles = self.particles.max_particles.clamp(1, 100_000);
        self.particles.spawn_rate_scale = self.particles.spawn_rate_scale.clamp(0.0, 1.0);
    }
}

impl Default for VisualQualityBudget {
    fn default() -> Self {
        Self::for_profile(default_visual_quality_profile())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisualQualitySettings {
    #[serde(default = "default_visual_quality_profile")]
    pub profile: VisualQualityProfile,
    #[serde(default)]
    pub custom: VisualQualityBudget,
}

impl VisualQualitySettings {
    pub fn resolved_budget(&self) -> VisualQualityBudget {
        if self.profile == VisualQualityProfile::Custom {
            self.custom.clone()
        } else {
            VisualQualityBudget::for_profile(self.profile)
        }
    }

    pub fn clamp_all(&mut self) {
        self.custom.clamp_all();
    }
}

impl Default for VisualQualitySettings {
    fn default() -> Self {
        let profile = default_visual_quality_profile();
        Self {
            profile,
            custom: VisualQualityBudget::for_profile(profile),
        }
    }
}
