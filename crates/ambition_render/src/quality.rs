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
    mut portal_budget: ResMut<ambition_portal_presentation::PortalCaptureQualityBudget>,
) {
    let next = ambition_portal_presentation::PortalCaptureQualityBudget {
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
