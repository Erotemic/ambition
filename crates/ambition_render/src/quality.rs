//! Live resolved visual-quality resource.
//!
//! Settings persist the user's profile/custom table in gameplay-core. The render
//! side mirrors that into one resource every visual subsystem can read.

use bevy::prelude::*;

use ambition_persistence::settings::{UserSettings, VisualQualityBudget, VisualQualityProfile};

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ResolvedVisualQuality {
    pub profile: VisualQualityProfile,
    pub budget: VisualQualityBudget,
}

impl Default for ResolvedVisualQuality {
    fn default() -> Self {
        let settings = ambition_persistence::settings::VisualQualitySettings::default();
        Self {
            profile: settings.profile,
            budget: settings.resolved_budget(),
        }
    }
}

impl ResolvedVisualQuality {
    pub fn from_settings(settings: &UserSettings) -> Self {
        Self {
            profile: settings.video.quality.profile,
            budget: settings.video.quality.resolved_budget(),
        }
    }
}

/// **The resource and the system that keeps it true, together.**
///
/// ⚠ **They were apart, and the half that MOVES was app-local.**
/// `ResolvedVisualQuality` was initialised by `PlatformerPresentationPlugin`
/// (and again by `game/ambition_app`), while `sync_resolved_visual_quality` —
/// the only thing that ever reads `UserSettings` into it — was registered by the
/// app alone. So every other composition held a resource that was permanently
/// its `Default`: a demo could load a user's Potato profile and draw at Full,
/// silently, because the value existed and simply never moved. Found by
/// `scripts/check_engine_systems_are_engine_installed.py`.
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
        app.init_resource::<ResolvedVisualQuality>();
        app.add_systems(Update, sync_resolved_visual_quality);
        // ⭐ **the portal budget joins its resource here, 2026-08-04.** It was
        // registered in `ambition_app`'s plugin file alone while the resource it
        // REQUIRES (`Res<ResolvedVisualQuality>`, not `Option`) was installed by
        // this plugin — so any other composition that pulled in the render
        // presentation and this system panicked on a missing resource.
        //
        // ⚠ that is the SAME defect the note beside its old call site records
        // for `sync_resolved_visual_quality` on 2026-07-31 ("they were apart, and
        // the half that MOVES was registered here alone"). The sibling was fixed
        // and this one was left, one system later in the same file.
        //
        // It cost `capture_scene` — the repo's only way to LOOK at a visual
        // change on a machine with no display — which panicked before drawing
        // anything.
        #[cfg(feature = "portal_render")]
        app.add_systems(Update, sync_portal_quality_budget);
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
