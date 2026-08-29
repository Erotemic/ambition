//! Live resolved visual-quality resource.
//!
//! Settings persist the user's profile/custom table in gameplay-core. The render
//! side mirrors that into one resource every visual subsystem can read.

use bevy::prelude::*;
use bevy::camera::RenderTarget;
use bevy::render::view::Msaa;

use ambition_persistence::settings::{
    profile_override_from_env, UserSettings, VisualQualityBudget, VisualQualityProfile,
};

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ResolvedVisualQuality {
    pub profile: VisualQualityProfile,
    pub budget: VisualQualityBudget,
}

impl Default for ResolvedVisualQuality {
    fn default() -> Self {
        if let Some(forced) = profile_override_from_env() {
            return Self { profile: forced, budget: VisualQualityBudget::for_profile(forced) };
        }
        let settings = ambition_persistence::settings::VisualQualitySettings::default();
        Self {
            profile: settings.profile,
            budget: settings.resolved_budget(),
        }
    }
}

impl ResolvedVisualQuality {
    /// ⭐ THE BOOT OVERRIDE WINS, AND IT IS NOT WRITTEN BACK. `AMBITION_QUALITY_PROFILE`
    /// (which `run_game.sh` sets from the launcher config) forces the tier for
    /// the life of the process. It resolves the tier's own budget table rather
    /// than the user's stored `custom` one, so a forced Medium is the same
    /// Medium everywhere.
    ///
    /// ⚠ While an override is in force the settings menu cannot change quality:
    /// this runs every frame and will put the forced tier straight back. That is
    /// the intended behaviour of a forced profile, and the reason
    /// `log_quality_profile_override` says so once at startup rather than
    /// leaving someone to discover it by clicking.
    pub fn from_settings(settings: &UserSettings) -> Self {
        if let Some(forced) = profile_override_from_env() {
            return Self { profile: forced, budget: VisualQualityBudget::for_profile(forced) };
        }
        Self {
            profile: settings.video.quality.profile,
            budget: settings.video.quality.resolved_budget(),
        }
    }
}

/// The resource and the system that keeps it true, together.
///
/// They were apart, and the half that MOVES was app-local. `ResolvedVisualQuality` was
/// initialised by `PlatformerPresentationPlugin` (and again by `game/ambition_app`), while
/// `sync_resolved_visual_quality` — the only thing that ever reads `UserSettings` into it — was
/// registered by the app alone.
///
/// Idempotent (`is_unique() -> false` plus a marker) because two plugins
/// legitimately need it: `PlatformerPresentationPlugin` for a demo, and
/// `SessionRoomVisualsPlugin` for the shipped host, which adds that one alone.
pub struct VisualQualityPlugin;

/// Present once [`VisualQualityPlugin`] has built.
#[derive(Resource)]
struct VisualQualityInstalled;

impl Plugin for VisualQualityPlugin {
    fn is_unique(&self) -> bool {
        false
    }

    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<VisualQualityInstalled>() {
            return;
        }
        app.insert_resource(VisualQualityInstalled);
        log_quality_profile_override();
        app.init_resource::<ResolvedVisualQuality>();
        app.add_systems(Update, (sync_resolved_visual_quality, sync_raster_budget).chain());
        // This bridge reads visual quality and writes portal presentation
        // quality, so register it only when the destination resource exists.
        #[cfg(feature = "portal_render")]
        app.add_systems(
            Update,
            sync_portal_quality_budget.run_if(
                resource_exists::<ambition_portal2d_presentation::PortalCaptureQualityBudget>,
            ),
        );
    }
}

/// Say once, at startup, that a forced profile is in force — and say it when the
/// value was set but not understood, which is the case that would otherwise look
/// exactly like the override working.
fn log_quality_profile_override() {
    let Ok(raw) = std::env::var(ambition_persistence::settings::QUALITY_PROFILE_ENV) else {
        return;
    };
    if raw.trim().is_empty() {
        return;
    }
    match VisualQualityProfile::from_label(&raw) {
        Some(profile) => info!(
            "visual quality forced to `{}` by {}; the settings menu cannot change it this run",
            profile.label(),
            ambition_persistence::settings::QUALITY_PROFILE_ENV,
        ),
        None => warn!(
            "{}={raw:?} is not a profile; using the saved setting instead. \
             Expected one of: potato, low, medium, high, ultra",
            ambition_persistence::settings::QUALITY_PROFILE_ENV,
        ),
    }
}

pub fn sync_resolved_visual_quality(
    settings: Option<Res<UserSettings>>,
    mut resolved: ResMut<ResolvedVisualQuality>,
) {
    let Some(settings) = settings else {
        return;
    };
    let next = ResolvedVisualQuality::from_settings(&settings);
    if *resolved != next {
        *resolved = next;
    }
}

#[cfg(feature = "portal_render")]
pub fn sync_portal_quality_budget(
    quality: Res<ResolvedVisualQuality>,
    mut portal_budget: ResMut<ambition_portal2d_presentation::PortalCaptureQualityBudget>,
) {
    let next = ambition_portal2d_presentation::PortalCaptureQualityBudget {
        max_resolution: quality.budget.portal.max_resolution,
        texels_per_world_px: quality.budget.portal.texels_per_world_px,
        recursion_depth: quality.budget.portal.recursion_depth,
        max_active_captures: quality.budget.portal.max_active_captures,
        max_updates_per_frame: quality.budget.portal.max_updates_per_frame,
        min_refresh_interval_s: quality.budget.portal.min_refresh_interval_s,
        include_parallax: quality.budget.portal.include_parallax,
    };
    if *portal_budget != next {
        *portal_budget = next;
    }
}

/// Apply the [`RasterBudget`](ambition_persistence::settings::RasterBudget): the
/// DPI-scale cap on the window, and MSAA on every camera that draws to it.
///
/// ⭐ THESE ARE THE TWO COSTS THAT SCALE WITH SCREEN AREA. Every other knob in
/// the quality budget trades away scene detail; these trade away fragments, and
/// on hardware without a discrete GPU the fragments are the frame. Measured on
/// `calculex` (Intel HD 630) 2026-08-29: a 1600x900 window on a 2x Wayland
/// session rasterised at 3200x1800, every full-screen pass reported exactly
/// 5,760,000 fragment invocations, and the frame sat at a p50 of ~50ms.
///
/// ⛔ CAPTURE CAMERAS ARE NOT TOUCHED. `ambition_render::capture` pins
/// `Msaa::Off` on the image targets it adopts, deliberately, and a blanket
/// write here would undo it. Only cameras whose target is a WINDOW are the
/// screen-area cost this budget is about.
///
/// ⚠ THE SCALE CAP IS A REQUEST, AND THE INSTRUMENT THAT CONFIRMS IT ALREADY
/// EXISTS. `set_scale_factor_override` asks winit for a smaller buffer; whether
/// a given compositor honours it by upscaling (what we want) rather than by
/// shrinking the window is a property of the platform, not of this code. The
/// check is one number in any profiling bundle:
/// `render/upscaling/fragment_shader_invocations` is the framebuffer's pixel
/// count exactly. If capping the scale does not divide it, the cap did not take
/// and the next lever is an explicit reduced render target — do not assume.
pub fn sync_raster_budget(
    quality: Res<ResolvedVisualQuality>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(Entity, &RenderTarget, Option<&Msaa>), With<Camera>>,
    mut commands: Commands,
) {
    let raster = &quality.budget.raster;

    for mut window in &mut windows {
        // Read through `Deref` so an unchanged window is not marked changed;
        // only the assignment below takes `DerefMut`.
        let reported = window.resolution.base_scale_factor();
        let desired = raster.effective_scale_factor(reported);
        if window.resolution.scale_factor_override() != desired {
            window.resolution.set_scale_factor_override(desired);
        }
    }

    let desired = match raster.sanitized_msaa_samples() {
        1 => Msaa::Off,
        2 => Msaa::Sample2,
        8 => Msaa::Sample8,
        _ => Msaa::Sample4,
    };
    for (entity, target, current) in &cameras {
        if !matches!(target, RenderTarget::Window(_)) {
            continue;
        }
        if current != Some(&desired) {
            commands.entity(entity).insert(desired);
        }
    }
}
