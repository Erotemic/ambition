//! Global visual quality profile and resolved runtime/device budgets.
//!
//! The profile enum is only interpreted here. Render/asset subsystems consume
//! the resolved budget fields so Low/Medium/High never becomes a local dialect.

use serde::{Deserialize, Serialize};

use super::{cycle_next, cycle_prev};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VisualQualityProfile {
    /// Absolute bare minimum — for the slowest hardware imaginable (and a
    /// little bit of a joke). Everything is stripped: no portal recursion or
    /// parallax, shaders off, near-zero particles, and textures shrunk to a
    /// per-sheet 8px floor (~1% of the authored size). It is *meant* to look
    /// bad; the goal is "it runs at all," not "it's pretty."
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
    /// Parse a profile by the same label [`label`](Self::label) prints, so the
    /// string in a config file and the string in a diagnostic are one spelling.
    /// Case-insensitive and whitespace-tolerant, because this reads hand-edited
    /// files.
    ///
    /// ⛔ `custom` is deliberately NOT parseable. It means "use the budget table
    /// stored in the user's settings", which a boot override has no way to
    /// supply — accepting it would silently boot High under another name.
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

/// Per-process overrides for the two raster knobs, applied ON TOP of whichever
/// tier is active.
///
/// ⭐ WHY THESE ARE SEPARATE FROM THE TIER. Changing tier to reach the raster
/// knobs also changes texture resolution, parallax layers, portal budget and
/// particle counts — four other variables moving at once. An A/B that swaps
/// tiers cannot attribute its result to any of them. These exist so the pixel
/// count and the sample count can each be moved ALONE against an unchanged
/// baseline, which is the only way the answer means anything.
pub const MAX_SCALE_FACTOR_ENV: &str = "AMBITION_MAX_SCALE_FACTOR";
/// See [`MAX_SCALE_FACTOR_ENV`]. `1` turns MSAA off; 2, 4 and 8 are the counts
/// Bevy names.
pub const MSAA_ENV: &str = "AMBITION_MSAA";

impl RasterBudget {
    /// This budget with any per-process override applied.
    ///
    /// ⚠ A value that cannot be read is IGNORED, not defaulted. Substituting a
    /// number nobody typed would make a typo look like a successful experiment.
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
/// This is the seam the launcher's TOML config drives: `run_game.sh` reads the
/// file, exports this, and the game obeys it. Nothing is written back to the
/// user's saved settings — a forced profile lasts exactly as long as the
/// process, so profiling a laptop on Medium cannot quietly become that
/// machine's permanent preference.
pub const QUALITY_PROFILE_ENV: &str = "AMBITION_QUALITY_PROFILE";

/// The forced profile for this process, if one was asked for and understood.
///
/// ⚠ An unparseable value returns `None` rather than falling back to a default,
/// so a typo boots the user's OWN setting instead of silently substituting a
/// tier they did not choose. Callers are expected to say so out loud.
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
/// ⭐ MIRRORS `wgpu::DeviceType` DELIBERATELY, and stops there. The mapping from
/// the graphics API's enum belongs at the render seam that already owns that
/// dependency; the POLICY — which tier a class of hardware should start on —
/// belongs here with the tiers it names. Persistence gaining a `wgpu`
/// dependency to answer a question about its own tiers would be the tail
/// wagging the dog.
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
/// ⛔⛔ THIS IS A FIRST-RUN SEED AND NOTHING ELSE. Re-deciding it each launch
/// would silently undo the settings menu: a player who chose High on an
/// integrated laptop because they prefer the picture to the framerate would
/// find themselves back on Medium every boot, with nothing in the UI admitting
/// why. Callers must apply this only where no stored profile exists.
///
/// ⚠ WHY IT IS NEEDED: [`default_visual_quality_profile`] decides by TARGET OS,
/// so every desktop boots `High` — including one whose renderer is an Intel HD
/// 630. Measured on `calculex` 2026-08-29: that machine ran p50 51.0ms
/// (~19.6 FPS) at High, and 20.1ms (~49.7 FPS) once the raster budget matched
/// the hardware. The OS was never the thing that made it slow.
///
/// ⚠ `Cpu` gets `Potato` rather than `Low` because a software rasteriser is not
/// a weak GPU, it is NO GPU — the fill cost is paid by the same cores running
/// the sim, so the tier that merely trims effects is not the right answer.
pub fn seed_profile_for_gpu(class: DetectedGpuClass) -> VisualQualityProfile {
    match class {
        DetectedGpuClass::Discrete => VisualQualityProfile::High,
        DetectedGpuClass::Integrated | DetectedGpuClass::Virtual => VisualQualityProfile::Medium,
        DetectedGpuClass::Cpu => VisualQualityProfile::Potato,
        // ⛔ AN UNKNOWN ADAPTER KEEPS THE EXISTING DEFAULT rather than guessing
        // downward. Booting a machine we cannot classify into a degraded tier
        // would make "we did not recognise your GPU" indistinguishable from
        // "your GPU is bad", and the player has no way to tell which happened.
        DetectedGpuClass::Other => default_visual_quality_profile(),
    }
}

/// `Ord` is declaration order, which is ascending PIXEL BUDGET (Potato <
/// Quarter < Half < Full).
///
/// ⭐ IT IS A REQUEST ORDERING, AND NOTHING COMPARES ON IT TODAY — but that
/// sentence has been false once, so the history is kept rather than the claim
/// alone. It originally read *"nothing decides policy by comparing two tiers,
/// and nothing should"*; by 2026-09-02 two policy sites did, both comparing
/// REQUESTS, which is what the order means:
///
/// - `room_character_tier_bounds` took `cap.min(ceiling)` — of the room's cap
///   and the user's setting, whichever asked for fewer pixels;
/// - `has_stale_realizations_outside` asked `requested_tier < floor ||
///   requested_tier > ceiling`, an interval test over that same band.
///
/// ⊙ **BOTH ARE GONE (2026-09-03), and not because anyone set out to restore
/// this rule.** Jon's ruling that no room may lower the sprite tier (`06a494f4e`)
/// removed the room cap outright, and the band it needed collapsed to a single
/// tier with it: `has_stale_realizations(&self, active)` now asks
/// `requested_tier != active`, an EQUALITY. ⇒ The original warning is true
/// again, restored by a product decision rather than by a cleanup — which is
/// exactly why the two dead sites are named here instead of deleted. A reader
/// who meets only the current rule cannot tell a rule that has held from one
/// that has been broken and repaired by accident.
///
/// ⛔⛔ WHAT MUST NOT BE COMPARED IS A RESOLVED TIER, and that is what the
/// original warning was protecting. Whether a realization is CURRENT is decided
/// by the tier it ANSWERS, not by the pixels it got: `character_sprite_tier`
/// stamps the request precisely so the comparison is an equality that settles,
/// and `resident_tiers` records that two resolved tiers *"is NOT by itself a
/// convergence failure — a fallback is a permanent, correct disagreement"*. A
/// character whose Full sheet has no Quarter variant resolves Full inside a
/// Quarter room forever, and ranking those two would retire and rebuild it every
/// frame.
///
/// ⇒ Order the REQUESTS; compare the ANSWERS only for equality.
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

    /// The scales that get *generated as variants* (everything below `Full`).
    /// Single source of truth for the manifest-registration loops so a new
    /// tier can never be half-wired into only some asset families.
    pub const MANIFEST_VARIANTS: [Self; 3] = [Self::Half, Self::Quarter, Self::Potato];

    pub fn scale_factor(self) -> f32 {
        match self {
            // Nominal only — `Potato` is floored per-sheet in the generator, so
            // its effective factor varies by sheet. Nothing reads this at
            // runtime (the real scaling is baked into the variant PNG + RON).
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
    /// Two fields, one answer, and the two are not independent: a budget that
    /// does not `prefer_scaled_variants` loads the authored PNG whatever
    /// `resolution_scale` says. Callers that compared `resolution_scale` alone
    /// were asking a question the loader does not answer — `High` and a `Custom`
    /// budget with `resolution_scale: Half, prefer_scaled_variants: false` both
    /// load Full, and only this collapses them.
    ///
    /// It is also what makes "did the tier change?" decidable: `Low` and
    /// `Medium` are different PROFILES that resolve to the same sheet pixels, so
    /// keying a reload on the profile reloads for nothing.
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

/// How many pixels the frame is actually rasterised into, and how many samples
/// each one takes. The two knobs that scale with SCREEN AREA rather than with
/// scene content — everything else in this budget trades away detail, these
/// trade away fill.
///
/// ⭐ `max_scale_factor` IS NOT A RESOLUTION. It caps the DPI scale the
/// compositor hands us; the window keeps its logical size and the game keeps
/// its layout. On a 1x display it changes nothing at all. On a 2x display it
/// is the difference between rasterising 1600x900 and 3200x1800 — four times
/// the fragments for the same picture on the same monitor.
///
/// ⚠ Measured on `calculex` (i7-7700HQ, Intel HD 630) 2026-08-29: a 1600x900
/// window on a 2x Wayland session rasterised at 3200x1800. Every full-screen
/// pass reported exactly 5,760,000 fragment invocations, and the frame sat at
/// a p50 of ~50ms. A discrete GPU never notices; integrated graphics pay it in
/// full, and so does a handheld.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RasterBudget {
    /// Upper bound on the window's DPI scale factor. `None` honours whatever
    /// the compositor asks for, which is the right answer when there is a GPU
    /// to spare and the wrong one when there is not.
    pub max_scale_factor: Option<f32>,
    /// MSAA samples per pixel: 1 (off), 2, 4, or 8.
    ///
    /// ⭐ MSAA ANTIALIASES GEOMETRY EDGES, and a 2D sprite is an axis-aligned
    /// textured quad whose edges are already flush. What it buys here is
    /// limited to the few things with real geometry — gizmos, lines, shapes —
    /// which is why this is a tier knob rather than a deletion: the tiers that
    /// can afford it keep it.
    ///
    /// Above 1 this also adds a `msaa_writeback` pass over the whole frame,
    /// which is why it compounds with `max_scale_factor` rather than adding
    /// to it.
    pub msaa_samples: u8,
}

impl Default for RasterBudget {
    /// ⭐ THE DEFAULT IS WHAT THE ENGINE DID BEFORE THIS FIELD EXISTED — honour
    /// the compositor's scale, and Bevy's own default 4x MSAA. A settings file
    /// written before `raster` was added therefore deserialises to exactly the
    /// behaviour it was written under, which is the only correct answer: the
    /// user never chose a raster budget, so they must not be given a cheaper one
    /// by surprise.
    fn default() -> Self {
        Self { max_scale_factor: None, msaa_samples: 4 }
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
    /// ⛔ `serde(default)` IS LOAD-BEARING, AND ITS ABSENCE COST A REAL SETTINGS
    /// FILE. `VisualQualitySettings.custom` is serialised into the user's
    /// `settings.ron`, so a budget field added later is MISSING from every file
    /// written before it existed. Without a default, that is not a missing
    /// field — it fails the whole `settings.ron` parse, and the user silently
    /// loses audio, video, gameplay and control settings together:
    ///
    /// ```text
    /// WARN could not parse settings file .../settings.ron:
    ///      Unexpected missing field named `raster` in `VisualQualityBudget`;
    ///      using defaults
    /// ```
    ///
    /// ⚠ Observed on `calculex` 2026-08-29, in the run that was measuring
    /// something else. EVERY future field on a serialised settings struct needs
    /// this, and the guard below is `settings_ron_without_raster_still_parses`.
    #[serde(default)]
    pub raster: RasterBudget,
}

impl VisualQualityBudget {
    pub fn for_profile(profile: VisualQualityProfile) -> Self {
        match profile {
            // Potato: strip everything. Smallest possible portal capture,
            // refreshed at most ~4×/sec; no recursion, no parallax, no shaders,
            // almost no particles; sprites + backgrounds at the `Potato` texture
            // tier (per-sheet 8px floor). The point is to run on a literal
            // potato, not to look good.
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
    /// ⛔⛔ THE WHOLE POINT IS THAT THIS HAPPENS ONCE. [`seed_profile_for_gpu`]
    /// is a FIRST-RUN SEED; re-deciding the tier every launch would silently
    /// undo the settings menu, so the seeding step records that it ran and
    /// never runs again. `false` in every file written before this existed,
    /// which is correct: those installs have not been seeded.
    ///
    /// ⚠ AND SEEDING IS ADDITIONALLY GATED ON THE PROFILE STILL BEING THE
    /// UNTOUCHED DEFAULT, so an existing install where the player CHOSE a tier
    /// keeps it. The two conditions together leave one narrow false positive —
    /// a player whose deliberate choice happens to equal the OS default — and
    /// it costs them one tier change they can undo in the menu, once, rather
    /// than every boot.
    #[serde(default)]
    pub hardware_seeded: bool,
}

impl VisualQualitySettings {
    /// Apply the one-time hardware seed, returning the tier if it changed.
    ///
    /// ⭐ THE DECISION LIVES HERE, NOT AT THE RENDER SEAM, so it can be tested
    /// without a GPU — which is the only way it can be tested at all on the
    /// machine this most matters for. The render layer's job is reduced to
    /// naming the adapter class; every rule about WHEN to apply it is below and
    /// is exercised by unit tests.
    ///
    /// Returns `None` when nothing was changed: already seeded, or the player
    /// has moved the tier off its default and owns it now.
    pub fn seed_from_hardware(&mut self, class: DetectedGpuClass) -> Option<VisualQualityProfile> {
        if self.hardware_seeded {
            return None;
        }
        // Record the attempt even when it declines to move anything, or a
        // player who chose their tier before this existed would be re-examined
        // on every launch forever.
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
            // A fresh settings block has NOT been seeded — that is the whole
            // state this field exists to hold, and the seeding pass is what
            // sets it.
            hardware_seeded: false,
        }
    }
}
