//! Global visual quality profile and resolved runtime/device budgets.
//!
//! The profile enum is only interpreted here. Render/asset subsystems consume
//! the resolved budget fields so Low/Medium/High never becomes a local dialect.

use serde::{Deserialize, Serialize};

use super::{cycle_next, cycle_prev};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VisualQualityProfile {
    /// Absolute minimum, for the slowest hardware. No portal recursion or
    /// parallax, shaders off, near-zero particles, and textures shrunk to a
    /// per-sheet 8px floor (~1% of the authored size). The goal is that it
    /// runs, not that it looks good.
    Potato,
    Low,
    Medium,
    #[default]
    High,
    Ultra,
    Custom,
}

impl VisualQualityProfile {
    pub const ALL: [Self; 6] = [
        Self::Potato,
        Self::Low,
        Self::Medium,
        Self::High,
        Self::Ultra,
        Self::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Potato => "potato",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Ultra => "ultra",
            Self::Custom => "custom",
        }
    }

    pub fn next(self) -> Self {
        // Fallback index = High (the desktop default) if `self` isn't found.
        cycle_next(&Self::ALL, self, 3)
    }

    pub fn prev(self) -> Self {
        cycle_prev(&Self::ALL, self, 3)
    }
}

impl VisualQualityProfile {
    /// Parse a profile by the label that [`label`](Self::label) prints.
    /// Case-insensitive and whitespace-tolerant, because it reads hand-edited
    /// files.
    ///
    /// `custom` is not parseable. It means "use the stored budget table", which
    /// a boot override cannot supply; it would silently boot High.
    pub fn from_label(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "potato" => Some(Self::Potato),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "ultra" => Some(Self::Ultra),
            _ => None,
        }
    }
}

/// Per-process overrides for the two raster knobs, applied on top of the
/// active tier.
///
/// They are separate from the tier so that pixel count and sample count can
/// each change alone against an unchanged baseline. A tier change also moves
/// texture resolution, parallax, portal budget, and particles.
pub const MAX_SCALE_FACTOR_ENV: &str = "AMBITION_MAX_SCALE_FACTOR";
/// See [`MAX_SCALE_FACTOR_ENV`]. `1` turns MSAA off; 2, 4 and 8 are the counts
/// Bevy names.
pub const MSAA_ENV: &str = "AMBITION_MSAA";

impl RasterBudget {
    /// This budget with any per-process override applied.
    ///
    /// A value that cannot be parsed is ignored, not defaulted, so a typo does
    /// not look like a successful experiment.
    pub fn with_env_overrides(mut self) -> Self {
        if let Ok(raw) = std::env::var(MAX_SCALE_FACTOR_ENV) {
            match raw.trim().to_ascii_lowercase().as_str() {
                "" => {}
                // An explicit way to say "honour the compositor", so a config
                // can turn the cap OFF as well as on.
                "none" | "off" | "native" => self.max_scale_factor = None,
                other => {
                    if let Ok(value) = other.parse::<f32>() {
                        if value.is_finite() && value > 0.0 {
                            self.max_scale_factor = Some(value);
                        }
                    }
                }
            }
        }
        if let Ok(raw) = std::env::var(MSAA_ENV) {
            if let Ok(value) = raw.trim().parse::<u8>() {
                self.msaa_samples = value;
            }
        }
        self
    }
}

/// The environment variable that forces a visual quality profile at boot.
///
/// The launcher's TOML config drives it: `run_game.sh` reads the file and
/// exports this variable. It is not written back to saved settings, so a
/// forced profile lasts only for the process.
pub const QUALITY_PROFILE_ENV: &str = "AMBITION_QUALITY_PROFILE";

/// The forced profile for this process, if one was asked for and understood.
///
/// An unparseable value returns `None`, so a typo boots the user's own setting,
/// not a tier they did not choose. Callers must report it.
pub fn profile_override_from_env() -> Option<VisualQualityProfile> {
    let raw = std::env::var(QUALITY_PROFILE_ENV).ok()?;
    if raw.trim().is_empty() {
        return None;
    }
    VisualQualityProfile::from_label(&raw)
}

pub fn default_visual_quality_profile() -> VisualQualityProfile {
    if cfg!(target_os = "android") {
        VisualQualityProfile::Medium
    } else {
        VisualQualityProfile::High
    }
}

/// What KIND of adapter the renderer came up on, as a fact this crate can hold
/// without depending on `wgpu`.
///
/// It mirrors `wgpu::DeviceType`. The mapping from `wgpu` belongs at the render
/// seam, which already depends on it. The policy (which tier a hardware class
/// starts on) belongs here with the tiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DetectedGpuClass {
    /// A discrete card on its own bus.
    Discrete,
    /// An IGP sharing system memory — an Intel HD 630, say.
    Integrated,
    /// A paravirtualised adapter inside a guest.
    Virtual,
    /// No GPU: a software rasteriser (llvmpipe / lavapipe / SwiftShader).
    Cpu,
    /// The adapter answered something this build does not recognise.
    Other,
}

/// The tier a machine should START on, given what it renders with.
///
/// This is a first-run seed only. Callers must apply it only where no stored
/// profile exists; otherwise it undoes the player's choice in the settings menu
/// on every boot.
///
/// It is needed because [`default_visual_quality_profile`] decides by target
/// OS, so every desktop boots `High`, including integrated GPUs such as an
/// Intel HD 630, which runs much faster with a matched raster budget.
///
/// `Cpu` gets `Potato`, not `Low`: a software rasteriser pays fill cost on the
/// same cores that run the simulation.
pub fn seed_profile_for_gpu(class: DetectedGpuClass) -> VisualQualityProfile {
    match class {
        DetectedGpuClass::Discrete => VisualQualityProfile::High,
        DetectedGpuClass::Integrated | DetectedGpuClass::Virtual => VisualQualityProfile::Medium,
        DetectedGpuClass::Cpu => VisualQualityProfile::Potato,
        // An unknown adapter keeps the existing default. Do not guess
        // downward: "GPU not recognised" must not look like "GPU is weak".
        DetectedGpuClass::Other => default_visual_quality_profile(),
    }
}

/// `Ord` is declaration order, which is ascending PIXEL BUDGET (Potato <
/// Quarter < Half < Full).
///
/// This is a request ordering. Nothing compares on it today, but it is
/// valid to order requests (for example `cap.min(ceiling)`).
///
/// Do not order resolved tiers. A realization is current when it answers the
/// requested tier: `character_sprite_tier` stamps the request so the check is
/// an equality (`stale_realizations` uses `requested_tier != active`). A
/// fallback gives a permanent, correct disagreement (see `resident_tiers`):
/// a character with no Quarter variant resolves Full in a Quarter room, and
/// ranking the two would rebuild it every frame.
///
/// Order the requests; compare the answers only for equality.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum TextureResolutionScale {
    /// Bare-minimum "potato" textures. The generator shrinks each sheet toward
    /// ~1% of its authored size but floors every frame at 8px so atlases stay
    /// loadable; the exact per-sheet factor is baked into the variant manifest.
    Potato,
    Quarter,
    Half,
    #[default]
    Full,
}

impl TextureResolutionScale {
    pub const ALL: [Self; 4] = [Self::Potato, Self::Quarter, Self::Half, Self::Full];

    /// The scales that are generated as variants (everything below `Full`).
    /// The single source for the manifest-registration loops, so a new tier
    /// cannot be wired into only some asset families.
    pub const MANIFEST_VARIANTS: [Self; 3] = [Self::Half, Self::Quarter, Self::Potato];

    pub fn scale_factor(self) -> f32 {
        match self {
            // Nominal only: `Potato` is floored per sheet in the generator.
            // Nothing reads this at runtime.
            Self::Potato => 0.1,
            Self::Quarter => 0.25,
            Self::Half => 0.5,
            Self::Full => 1.0,
        }
    }

    pub fn folder_suffix(self) -> &'static str {
        match self {
            Self::Potato => "_potato",
            Self::Quarter => "_0_25x",
            Self::Half => "_0_5x",
            Self::Full => "",
        }
    }

    pub fn asset_id_suffix(self) -> Option<&'static str> {
        match self {
            Self::Potato => Some("potato"),
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
            Self::Potato => "backgrounds/parallax_layers_potato",
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

impl SpriteTextureBudget {
    /// The tier a sprite sheet actually loads at under this budget.
    ///
    /// The two fields are not independent: a budget that does not
    /// `prefer_scaled_variants` loads the authored PNG whatever
    /// `resolution_scale` says. Compare this, not `resolution_scale` alone.
    ///
    /// It also makes "did the tier change?" decidable: `Low` and `Medium` are
    /// different profiles with the same sheet pixels, so a reload keyed on the
    /// profile reloads for nothing.
    pub fn effective_scale(&self) -> TextureResolutionScale {
        if self.prefer_scaled_variants {
            self.resolution_scale
        } else {
            TextureResolutionScale::Full
        }
    }
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

/// How many pixels the frame is rasterised into, and how many samples each one
/// takes. These two knobs scale with screen area, not scene content.
///
/// `max_scale_factor` is not a resolution. It caps the DPI scale from the
/// compositor; the window keeps its logical size and layout. On a 1x display it
/// changes nothing. On a 2x display it halves each axis of the raster (for
/// example 3200x1800 to 1600x900). Integrated GPUs and handhelds pay this cost
/// in full.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RasterBudget {
    /// Upper bound on the window's DPI scale factor. `None` honours the
    /// compositor.
    pub max_scale_factor: Option<f32>,
    /// MSAA samples per pixel: 1 (off), 2, 4, or 8.
    ///
    /// MSAA antialiases geometry edges. 2D sprites are axis-aligned quads, so
    /// it helps only gizmos, lines, and shapes. It is a tier knob so tiers that
    /// can afford it keep it.
    ///
    /// Above 1 it also adds a full-frame `msaa_writeback` pass, so its cost
    /// compounds with `max_scale_factor`.
    ///
    /// This field is `pub(crate)` because it is the request, not the rendered
    /// value. `AMBITION_MSAA` accepts any `u8`, and
    /// [`Self::sanitized_msaa_samples`] rounds it down to a count Bevy supports
    /// (3 -> 2, 16 -> 8). Consumers outside this crate must use the accessor.
    pub(crate) msaa_samples: u8,
}

impl Default for RasterBudget {
    /// The default is the engine behaviour from before this field existed:
    /// honour the compositor's scale, and Bevy's default 4x MSAA. An older
    /// settings file therefore keeps its behaviour.
    fn default() -> Self {
        Self {
            max_scale_factor: None,
            msaa_samples: 4,
        }
    }
}

impl RasterBudget {
    /// Bevy wants a power-of-two sample count it recognises; anything else is
    /// a typo in a config file and must not reach the renderer.
    pub fn sanitized_msaa_samples(&self) -> u8 {
        match self.msaa_samples {
            0 | 1 => 1,
            2 => 2,
            4 => 4,
            8 => 8,
            // Round DOWN to the nearest supported tier. A machine that asked
            // for more than it can name should not be handed more work.
            other if other > 8 => 8,
            other if other > 4 => 4,
            _ => 2,
        }
    }

    /// The scale factor to actually use, given what the compositor reported.
    /// `None` means "do not override".
    pub fn effective_scale_factor(&self, reported: f32) -> Option<f32> {
        let cap = self.max_scale_factor?;
        // Only ever a CAP. A display reporting less than the cap keeps its own
        // value; raising it would be inventing pixels nobody asked for.
        (reported > cap).then_some(cap)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisualQualityBudget {
    pub portal: PortalCaptureBudget,
    pub sprites: SpriteTextureBudget,
    pub backgrounds: BackgroundTextureBudget,
    pub parallax: ParallaxBudget,
    pub shaders: ShaderBudget,
    pub particles: ParticleBudget,
    /// `serde(default)` is required. `VisualQualitySettings.custom` is saved in
    /// `settings.ron`, so a field added later is missing from older files.
    /// Without a default, the whole `settings.ron` parse fails and the user
    /// loses all settings:
    ///
    /// ```text
    /// WARN could not parse settings file .../settings.ron:
    ///      Unexpected missing field named `raster` in `VisualQualityBudget`;
    ///      using defaults
    /// ```
    ///
    /// Every new field on a saved settings struct needs this. Guarded by
    /// `settings_ron_without_raster_still_parses`.
    #[serde(default)]
    pub raster: RasterBudget,
}

impl VisualQualityBudget {
    pub fn for_profile(profile: VisualQualityProfile) -> Self {
        match profile {
            // Potato: strip everything. Smallest portal capture, refreshed at
            // most ~4×/sec; no recursion, parallax, or shaders; almost no
            // particles; `Potato` texture tier (per-sheet 8px floor).
            VisualQualityProfile::Potato => Self {
                portal: PortalCaptureBudget {
                    max_resolution: 128,
                    texels_per_world_px: 0.05,
                    recursion_depth: 0,
                    max_active_captures: 1,
                    max_updates_per_frame: 1,
                    min_refresh_interval_s: 0.250,
                    include_parallax: false,
                },
                sprites: SpriteTextureBudget {
                    resolution_scale: TextureResolutionScale::Potato,
                    prefer_scaled_variants: true,
                },
                backgrounds: BackgroundTextureBudget {
                    resolution_scale: TextureResolutionScale::Potato,
                    max_texture_resolution: 256,
                    prefer_scaled_variants: true,
                },
                parallax: ParallaxBudget {
                    enabled: false,
                    max_layers: Some(0),
                    resolution_scale: TextureResolutionScale::Potato,
                },
                shaders: ShaderBudget {
                    screen_shader_scale: 0.0,
                    allow_expensive_materials: false,
                },
                particles: ParticleBudget {
                    max_particles: 16,
                    spawn_rate_scale: 0.1,
                },
                raster: RasterBudget {
                    max_scale_factor: Some(1.0),
                    msaa_samples: 1,
                },
            },
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
                raster: RasterBudget {
                    max_scale_factor: Some(1.0),
                    msaa_samples: 1,
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
                raster: RasterBudget {
                    max_scale_factor: Some(1.0),
                    msaa_samples: 1,
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
                raster: RasterBudget {
                    max_scale_factor: None,
                    msaa_samples: 4,
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
                raster: RasterBudget {
                    max_scale_factor: None,
                    msaa_samples: 4,
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
    /// Whether this profile has already been seeded from the detected adapter.
    ///
    /// The seed must happen once: [`seed_profile_for_gpu`] is a first-run seed,
    /// and re-deciding every launch would undo the settings menu. It is `false`
    /// in older files, which is correct because those installs are not seeded.
    ///
    /// Seeding also requires the profile to still be the default, so a player
    /// who chose a tier keeps it. The one false positive (a choice equal to the
    /// OS default) costs one tier change, once.
    #[serde(default)]
    pub hardware_seeded: bool,
}

impl VisualQualitySettings {
    /// Apply the one-time hardware seed, returning the tier if it changed.
    ///
    /// The decision is here, not at the render seam, so unit tests can run it
    /// without a GPU. The render layer only names the adapter class.
    ///
    /// Returns `None` when nothing changed: already seeded, or the player has
    /// moved the tier off its default.
    pub fn seed_from_hardware(&mut self, class: DetectedGpuClass) -> Option<VisualQualityProfile> {
        if self.hardware_seeded {
            return None;
        }
        // Record the attempt even when nothing moves, or a player who chose
        // their tier earlier is re-examined on every launch.
        self.hardware_seeded = true;
        if self.profile != default_visual_quality_profile() {
            return None;
        }
        let seeded = seed_profile_for_gpu(class);
        if seeded == self.profile {
            return None;
        }
        self.profile = seeded;
        Some(seeded)
    }

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
            // A fresh settings block is not seeded yet.
            hardware_seeded: false,
        }
    }
}

/// The visual quality a running process has RESOLVED: the boot override if one
/// is in force, else the persisted setting, with the raster environment
/// overrides folded in.
///
/// This is the one authority, below every consumer. The render side publishes
/// it as a resource and keeps it current. Character residency (room-transition
/// ration, global materializer, quality convergence) reads it, or derives it
/// through the same [`Self::from_settings`] when no publisher exists. Do not
/// call `UserSettings::resolved_budget()` directly: a forced profile would
/// then disagree with the persisted one.
///
/// The boot override wins and is not written back. `AMBITION_QUALITY_PROFILE`
/// (set by `run_game.sh` from the launcher config) forces the tier for the
/// process. It uses the tier's own budget table, not the stored `custom` one.
/// While an override is active, the settings menu cannot change quality.
#[derive(bevy::prelude::Resource, Clone, Debug, PartialEq)]
pub struct ResolvedVisualQuality {
    pub profile: VisualQualityProfile,
    pub budget: VisualQualityBudget,
}

impl Default for ResolvedVisualQuality {
    fn default() -> Self {
        let (profile, mut budget) = match profile_override_from_env() {
            Some(forced) => (forced, VisualQualityBudget::for_profile(forced)),
            None => {
                let settings = VisualQualitySettings::default();
                (settings.profile, settings.resolved_budget())
            }
        };
        budget.raster = budget.raster.with_env_overrides();
        Self { profile, budget }
    }
}

impl ResolvedVisualQuality {
    /// Resolve from the persisted settings, the override winning.
    pub fn from_settings(settings: &crate::settings::UserSettings) -> Self {
        let (profile, mut budget) = match profile_override_from_env() {
            Some(forced) => (forced, VisualQualityBudget::for_profile(forced)),
            None => (
                settings.video.quality.profile,
                settings.video.quality.resolved_budget(),
            ),
        };
        budget.raster = budget.raster.with_env_overrides();
        Self { profile, budget }
    }

    /// The published resource when a publisher is installed, else the same
    /// resolution derived from the settings -- so a composition without the
    /// render plugin still agrees with one that has it.
    pub fn current<'a>(
        published: Option<&'a Self>,
        settings: Option<&crate::settings::UserSettings>,
    ) -> std::borrow::Cow<'a, Self> {
        match (published, settings) {
            (Some(live), _) => std::borrow::Cow::Borrowed(live),
            (None, Some(settings)) => std::borrow::Cow::Owned(Self::from_settings(settings)),
            (None, None) => std::borrow::Cow::Owned(Self::default()),
        }
    }
}
