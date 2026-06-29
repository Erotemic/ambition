//! Optional portal camera continuity: presentation-only viewpoint mapping for
//! the camera while a controlled body straddles a portal.
//!
//! The resource in this module is the single source of truth for the live mode.
//! Hosts may surface it in a debug menu, but should not mirror the default into
//! a second `DeveloperTools` or settings field. Ambition currently defaults the
//! feature to `Continuous` because it is under active portal-lab debugging; flip
//! only this resource default when promoting/demoting the feature.

use bevy::prelude::*;

/// How the host camera behaves around a portal transit.
#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq)]
pub enum PortalCameraTransitMode {
    /// The host camera behaves normally. If its focus teleports, the camera
    /// pops/snaps/lerps exactly as the host camera system normally would.
    Pop,
    /// Portal camera continuity: when the active viewpoint focus transfers
    /// through a portal, map the previous visible camera center through the
    /// same portal BODY map that moved the focus, then keep the focus at that
    /// exact screen-space offset only while it remains in the aperture. Any
    /// roll is immediate and clears as soon as normal camera policy resumes.
    Continuous,
}

impl PortalCameraTransitMode {
    /// Every mode in debug-menu cycle order. `Pop` stays first for a stable UI
    /// order, even while the live resource default is temporarily Continuous.
    pub const ALL: &'static [Self] = &[Self::Pop, Self::Continuous];

    /// Display label for dev menus / logs.
    pub fn label(self) -> &'static str {
        match self {
            Self::Pop => "Pop",
            Self::Continuous => "Continuous",
        }
    }
}

/// Single source of truth for portal camera continuity.
#[derive(Resource, Clone, Copy, Debug, Reflect, PartialEq, Eq)]
#[reflect(Resource)]
pub struct PortalCameraContinuitySelection {
    pub mode: PortalCameraTransitMode,
}

impl Default for PortalCameraContinuitySelection {
    fn default() -> Self {
        Self {
            mode: PortalCameraTransitMode::Continuous,
        }
    }
}

impl PortalCameraContinuitySelection {
    /// Advance to the next/previous camera transit mode (`dir < 0` => previous).
    pub fn cycle(&mut self, dir: i32) {
        let all = PortalCameraTransitMode::ALL;
        let i = all.iter().position(|m| *m == self.mode).unwrap_or(0) as i32;
        let n = all.len() as i32;
        let next = if dir < 0 {
            (i + n - 1) % n
        } else {
            (i + 1) % n
        };
        self.mode = all[next as usize];
    }
}

/// Tunables for the optional continuity presentation pass.
///
/// This does not contain an `enabled` flag. The live/default mode belongs to
/// [`PortalCameraContinuitySelection`].
#[derive(Resource, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Resource)]
pub struct PortalCameraContinuityConfig {
    /// Ignore camera roll below this threshold. Straight-through wall<->wall and
    /// floor<->ceiling transitions land here and should not visibly perturb the
    /// camera.
    pub roll_epsilon_radians: f32,
    /// Maximum absolute screen offset (world units from camera center) for the
    /// ENTRY aperture to be considered the visible seam that the continuity pass
    /// should preserve.
    ///
    /// The continuity effect is only meaningful when the entry aperture is on
    /// or near the current view; offscreen re-triggers fall back to the host
    /// camera.
    pub max_entry_screen_offset: Vec2,
    /// Emit one-line transition diagnostics on each focus transit. This is
    /// intentionally low volume: it logs start/skip decisions, not every frame.
    pub debug_log: bool,
    /// Emit a constraint diagnostic when the portal-continuous camera center
    /// disagrees with the host camera center by more than this many world units
    /// on either axis, or when the desired center needs room-bound padding.
    pub camera_constraint_warn_pixels: f32,
    /// Treat a new transfer as overlapping a previous continuity effect when
    /// the previous effect still has at least this much active weight.
    pub overlap_warn_weight: f32,
}

impl Default for PortalCameraContinuityConfig {
    fn default() -> Self {
        Self {
            roll_epsilon_radians: 0.01,
            max_entry_screen_offset: Vec2::new(520.0, 360.0),
            debug_log: true,
            camera_constraint_warn_pixels: 16.0,
            overlap_warn_weight: 0.20,
        }
    }
}

/// Host camera sample produced by the ordinary camera-follow pass.
///
/// `current_center_world` is the last actually-rendered gameplay camera center,
/// after portal continuity has been consumed by the host camera policy. That is
/// the correct entry-side anchor for the next portal crossing: the body and the
/// camera must be mapped from the same visible frame. `ordinary_center_world`
/// keeps the host camera's normal follow/clamp answer for diagnostics and
/// recovery comparisons.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct PortalCameraContinuityHostView {
    /// Has the host written at least one camera sample?
    pub initialized: bool,
    /// Monotonic count of host camera samples. Useful in logs to verify the
    /// continuity system is reading a fresh host camera value each frame.
    pub sample_index: u64,
    /// Previous rendered gameplay camera center in world coordinates, before
    /// the latest host camera-follow sample.
    pub previous_center_world: Vec2,
    /// Current rendered gameplay camera center in world coordinates. This is
    /// the entry-side anchor for a transfer.
    pub current_center_world: Vec2,
    /// Host camera center after ordinary follow/clamp, before portal continuity
    /// screen anchoring is applied.
    pub ordinary_center_world: Vec2,
    /// Host camera target before clamp/smoothing, for diagnostics.
    pub target_world: Vec2,
    /// Host visible view size, for diagnostics and future on-screen tests.
    pub visible_view: Vec2,
    /// Number of camera zones the host camera follow saw this frame.
    pub active_camera_zones: usize,
    /// The selected host camera-zone id, if any. This is diagnostic-only: the
    /// portal crate does not interpret host zone names.
    pub active_camera_zone: Option<String>,
}

impl PortalCameraContinuityHostView {
    /// Record the latest gameplay camera sample. Call once per frame from the
    /// host camera policy after ordinary follow/clamp and portal continuity
    /// screen anchoring have both been resolved.
    pub fn capture(
        &mut self,
        center_world: Vec2,
        ordinary_center_world: Vec2,
        target_world: Vec2,
        visible_view: Vec2,
        active_camera_zones: usize,
        active_camera_zone: Option<String>,
    ) {
        if self.initialized {
            self.previous_center_world = self.current_center_world;
        } else {
            self.previous_center_world = center_world;
            self.initialized = true;
        }
        self.current_center_world = center_world;
        self.ordinary_center_world = ordinary_center_world;
        self.target_world = target_world;
        self.visible_view = visible_view;
        self.active_camera_zones = active_camera_zones;
        self.active_camera_zone = active_camera_zone;
        self.sample_index = self.sample_index.wrapping_add(1);
    }
}

/// Runtime state for one portal screen-anchor.
#[derive(Resource, Clone, Copy, Debug, Reflect, PartialEq)]
#[reflect(Resource)]
pub struct PortalCameraContinuityState {
    /// Immediate render-space camera roll, in radians, while the focus remains
    /// in the portal aperture. The final camera orientation is always identity.
    pub roll_radians: f32,
    /// Fallback last host camera center, in portal/world coordinates, used only
    /// when the host has not installed [`PortalCameraContinuityHostView`].
    /// Hosts with a real camera-view resource should prefer that resource so
    /// this state cannot be contaminated by continuity presentation offsets.
    pub last_host_camera_world: Option<Vec2>,
    /// Current absolute camera center for the exact-continuity phase.
    /// Updated every frame from [`Self::body_screen_offset_world`] while a
    /// screen-anchor is active.
    pub target_camera_world: Option<Vec2>,
    /// The controlled body center's screen-space offset from the gameplay
    /// camera center at the portal handoff. While active, the host camera
    /// follows `body_center - body_screen_offset_world`, so the sprite does not
    /// pause while the room-clamped camera target catches up.
    pub body_screen_offset_world: Option<Vec2>,
    /// Portal-authored camera clamp padding point.
    ///
    /// This is not a hold/ease lease: when the body leaves the aperture, the
    /// screen anchor clears immediately and ordinary camera follow resumes.
    /// The host camera may keep this point legal in its clamp rectangle until
    /// its normal smoothed target has returned inside ordinary camera bounds,
    /// preventing room clamps from snapping the freshly mapped chart back to the
    /// old room edge.
    pub clamp_padding_center_world: Option<Vec2>,
}

impl Default for PortalCameraContinuityState {
    fn default() -> Self {
        Self {
            roll_radians: 0.0,
            last_host_camera_world: None,
            target_camera_world: None,
            body_screen_offset_world: None,
            clamp_padding_center_world: None,
        }
    }
}

impl PortalCameraContinuityState {
    pub fn clear_effect(&mut self) {
        self.target_camera_world = None;
        self.body_screen_offset_world = None;
        self.roll_radians = 0.0;
    }

    pub fn clear_clamp_padding(&mut self) {
        self.clamp_padding_center_world = None;
    }

    pub fn clear(&mut self) {
        self.clear_effect();
        self.clear_clamp_padding();
    }

    pub fn start_screen_anchor(
        &mut self,
        target_camera_world: Vec2,
        body_screen_offset_world: Vec2,
        roll_radians: f32,
    ) {
        self.target_camera_world = Some(target_camera_world);
        self.body_screen_offset_world = Some(body_screen_offset_world);
        self.clamp_padding_center_world = Some(target_camera_world);
        self.roll_radians = roll_radians;
    }

    pub fn active_weight(&self) -> f32 {
        if self.body_screen_offset_world.is_some() {
            1.0
        } else {
            0.0
        }
    }
}

/// Camera roll to use for a portal-continuity crossing.
///
/// This deliberately follows the same somersault roll policy as the transiting
/// actor presentation. In the straight-through pairs that should feel like
/// ordinary translation (floor<->ceiling, right-wall<->left-wall), the returned
/// roll is zero; for a 90-degree floor<->wall pair it is the temporary camera
/// roll that makes the visible portal chart line up during the seam.
pub fn camera_roll_for_portal_transit(n_in: Vec2, n_out: Vec2, gravity_dir: Vec2) -> f32 {
    ambition_portal::somersault_roll(n_in, n_out, gravity_dir)
}

/// Host-applied marker for the camera/viewpoint that should receive the optional
/// continuity pass. The presentation crate defines the seam; hosts decide which
/// camera entity receives it.
#[derive(Component, Default)]
pub struct PortalCameraContinuityCamera;

/// Host-applied marker for the body/focus whose portal transits should start
/// camera continuity. The marker is intentionally not named after a player.
#[derive(Component, Default)]
pub struct PortalCameraContinuityFocus;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_mode_cycle_returns_to_default() {
        let mut selection = PortalCameraContinuitySelection::default();
        assert_eq!(selection.mode, PortalCameraTransitMode::Continuous);
        selection.cycle(1);
        assert_eq!(selection.mode, PortalCameraTransitMode::Pop);
        selection.cycle(1);
        assert_eq!(selection.mode, PortalCameraTransitMode::Continuous);
        selection.cycle(-1);
        assert_eq!(selection.mode, PortalCameraTransitMode::Pop);
    }

    #[test]
    fn straight_through_portal_pairs_have_no_camera_roll() {
        let gravity_down = Vec2::Y;
        let eps = 1.0e-5;

        let ceiling_to_floor = camera_roll_for_portal_transit(Vec2::Y, -Vec2::Y, gravity_down);
        assert!(
            ceiling_to_floor.abs() < eps,
            "ceiling->floor roll = {ceiling_to_floor}"
        );

        let floor_to_ceiling = camera_roll_for_portal_transit(-Vec2::Y, Vec2::Y, gravity_down);
        assert!(
            floor_to_ceiling.abs() < eps,
            "floor->ceiling roll = {floor_to_ceiling}"
        );

        let right_wall_to_left_wall =
            camera_roll_for_portal_transit(-Vec2::X, Vec2::X, gravity_down);
        assert!(
            right_wall_to_left_wall.abs() < eps,
            "right-wall->left-wall roll = {right_wall_to_left_wall}"
        );

        let left_wall_to_right_wall =
            camera_roll_for_portal_transit(Vec2::X, -Vec2::X, gravity_down);
        assert!(
            left_wall_to_right_wall.abs() < eps,
            "left-wall->right-wall roll = {left_wall_to_right_wall}"
        );
    }

    #[test]
    fn quarter_turn_portal_pair_has_camera_roll() {
        let gravity_down = Vec2::Y;
        let roll = camera_roll_for_portal_transit(Vec2::Y, Vec2::X, gravity_down);
        assert!(
            roll.abs() > std::f32::consts::FRAC_PI_4,
            "quarter-turn roll = {roll}"
        );
    }
}
