//! Touch HUD layout: action button identity, fixed positions, and
//! visible-circle hit testing.
//!
//! Layout values are intentionally bound to the *visible circle*, not
//! to the absolute square `Node` bounds. Adjacent diamond buttons can
//! have overlapping square footprints when their visible circles
//! don't overlap; the hit test in [`touch_action_at_position`] keys
//! on circle distance so multitouch stays aligned with what the user
//! sees.

use bevy::prelude::*;

use ambition_platformer_primitives::gameplay_presentation::ScreenRect;


/// Marker + identity for touch action buttons. Each `TouchActionButton`
/// entity is a Bevy `Button` whose `Interaction` state is collected into
/// the matching `TouchInputState` field each frame; the virtual-device
/// input kinds (`crate::virtual_device`) then resolve that state through
/// the participant's bindings like any physical button — hence the extra
/// reflect/serde derives (leafwing user inputs must carry them).
#[derive(
    Component,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    bevy::prelude::Reflect,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum TouchActionButton {
    Jump,
    Attack,
    Special,
    Dash,
    Blink,
    Interact,
    Projectile,
    FlyToggle,
    Shield,
    Start,
    Reset,
}

/// Uniform shrink factor applied to every touch-control dimension
/// (action cluster, button positions/sizes, menu row). Bumped from
/// the original 1.0 layout after Android playtesting showed the HUD
/// eating too much screen real estate; keep the diamond/menu shape
/// identical and just scale through this single knob.
pub(super) const TOUCH_SCALE: f32 = 0.7;
/// Font shrinks more conservatively than geometry so the labels stay
/// legible at phone DPI even when the buttons themselves drop by 30%.
pub(super) const TOUCH_FONT_SCALE: f32 = 0.85;
pub(crate) const ACTION_CLUSTER_MARGIN: f32 = 10.0;
pub(crate) const ACTION_BEZEL_PAD: f32 = 8.0;
pub(crate) const ACTION_CLUSTER_W: f32 = 310.0 * TOUCH_SCALE;
pub(crate) const ACTION_CLUSTER_H: f32 = 312.0 * TOUCH_SCALE;
pub(crate) const ACTION_BEZEL_W: f32 = ACTION_CLUSTER_W + ACTION_BEZEL_PAD * 2.0;
pub(crate) const ACTION_BEZEL_H: f32 = ACTION_CLUSTER_H + ACTION_BEZEL_PAD * 2.0;
/// Inset for the movement stick from the lower-left corner.
/// A slightly larger gap keeps the thumb control away from the
/// screen edge and leaves a cleaner buffer for gesture navigation.
pub(crate) const JOYSTICK_MARGIN: f32 = 64.0 * TOUCH_SCALE;
/// Generous movement-stick footprint reserved from menu drag-scroll gestures.
pub(crate) const JOYSTICK_EXCLUSION_SIZE: f32 = 300.0 * TOUCH_SCALE;
pub(crate) const MENU_ROW_MARGIN: f32 = 12.0;
pub(crate) const MENU_ROW_W: f32 = 198.0 * TOUCH_SCALE;
pub(crate) const MENU_W: f32 = 88.0 * TOUCH_SCALE;
pub(crate) const MENU_H: f32 = 44.0 * TOUCH_SCALE;
/// 88px button + 4px margin each side, scaled to match the shrunken
/// menu buttons so multitouch hit testing stays aligned with the
/// rendered overlay.
pub(crate) const MENU_CELL: f32 = 96.0 * TOUCH_SCALE;

#[derive(Clone, Copy, Debug)]
pub struct TouchActionSpec {
    pub action: TouchActionButton,
    pub label: &'static str,
    pub left: f32,
    pub top: f32,
    pub size: f32,
    pub font_size: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchJoystickLayout {
    pub margin: f32,
    pub base_size: f32,
    pub knob_size: f32,
    pub exclusion_size: f32,
}

pub fn movement_joystick_layout() -> TouchJoystickLayout {
    TouchJoystickLayout {
        margin: JOYSTICK_MARGIN,
        base_size: 200.0 * TOUCH_SCALE,
        knob_size: 100.0 * TOUCH_SCALE,
        exclusion_size: JOYSTICK_EXCLUSION_SIZE,
    }
}

/// Canonical lower-right action layout used by both the rendered UI and
/// raw multitouch hit testing. Keep all positions here so spacing fixes
/// cannot drift between the visible overlay and the Android touch path.
pub fn touch_action_layout() -> [TouchActionSpec; 9] {
    // Authored at the original 1.0-scale layout; `scaled` multiplies
    // through TOUCH_SCALE / TOUCH_FONT_SCALE so a single knob shrinks
    // the entire HUD without disturbing the diamond shape.
    let scaled = |action, label, left, top, size, font_size| TouchActionSpec {
        action,
        label,
        left: left * TOUCH_SCALE,
        top: top * TOUCH_SCALE,
        size: size * TOUCH_SCALE,
        font_size: font_size * TOUCH_FONT_SCALE,
    };
    [
        scaled(TouchActionButton::Blink, "Blink", 18.0, 10.0, 64.0, 13.0),
        scaled(TouchActionButton::FlyToggle, "Fly", 123.0, 2.0, 68.0, 14.0),
        scaled(
            TouchActionButton::Projectile,
            "Shot",
            228.0,
            10.0,
            64.0,
            13.0,
        ),
        scaled(
            TouchActionButton::Interact,
            "Interact",
            116.0,
            76.0,
            76.0,
            14.0,
        ),
        scaled(TouchActionButton::Attack, "Attack", 48.0, 148.0, 78.0, 14.0),
        scaled(TouchActionButton::Dash, "Dash", 184.0, 148.0, 78.0, 14.0),
        scaled(TouchActionButton::Shield, "Shield", 5.0, 222.0, 72.0, 13.0),
        scaled(TouchActionButton::Jump, "Jump", 115.0, 218.0, 80.0, 15.0),
        // Signature slot — lower-right corner (below Dash, sharing Shot's right
        // column) so the diamond keeps a >=4px visible-circle gap from every
        // neighbor. Hidden when the controlled scheme has no Special (the
        // availability predicate gates both visibility and the hit test), so a
        // movement-only character shows no phantom Special.
        scaled(
            TouchActionButton::Special,
            "Special",
            230.0,
            228.0,
            72.0,
            13.0,
        ),
    ]
}

/// The drawn centre and radius of one action button, in screen pixels, for a
/// cluster resolved at `cluster`.
///
/// The ONE projection from authored layout space into screen space. The
/// rendered `Node` and the raw multitouch hit test both go through it, so the
/// visible circle and its touch target cannot drift apart — including when the
/// cluster is compacted into a reserved surround.
pub fn touch_action_circle(spec: TouchActionSpec, cluster: ScreenRect) -> (Vec2, f32) {
    let scale = action_cluster_scale(cluster);
    let center = cluster.min
        + Vec2::new(
            (spec.left + spec.size * 0.5) * scale,
            (spec.top + spec.size * 0.5) * scale,
        );
    (center, spec.size * 0.5 * scale)
}

/// How much the authored action layout was scaled to reach `cluster`.
pub fn action_cluster_scale(cluster: ScreenRect) -> f32 {
    if ACTION_CLUSTER_W <= 0.0 {
        return 1.0;
    }
    (cluster.width() / ACTION_CLUSTER_W).max(0.0)
}

/// Hit-test a `pos` against the visible action button circles and the menu row.
/// Touch positions use the same top-left-origin logical coordinate space as
/// Bevy window cursor positions.
///
/// Gameplay action buttons are visible *circles*, so this hit-tests them as
/// circles too — diagonal square bounds are allowed to overlap when the circles
/// themselves do not.
///
/// Both rectangles come from the resolved [`TouchControlPlacement`], never from
/// the window: a cluster reserved into a surround column is tappable where it
/// is DRAWN, not where a window-relative formula would have put it.
///
/// [`TouchControlPlacement`]: crate::placement::TouchControlPlacement
pub fn touch_action_at_position(
    pos: Vec2,
    cluster: Option<ScreenRect>,
    menu_row: Option<ScreenRect>,
) -> Option<TouchActionButton> {
    if let Some(cluster) = cluster {
        for spec in touch_action_layout() {
            let (center, radius) = touch_action_circle(spec, cluster);
            if pos.distance(center) <= radius {
                return Some(spec.action);
            }
        }
    }

    if let Some(menu_row) = menu_row {
        let scale = if MENU_ROW_W > 0.0 {
            (menu_row.width() / MENU_ROW_W).max(0.0)
        } else {
            1.0
        };
        for (action, col) in [
            (TouchActionButton::Start, 0usize),
            (TouchActionButton::Reset, 1),
        ] {
            let min = menu_row.min + Vec2::new((col as f32 * MENU_CELL + 4.0) * scale, 4.0 * scale);
            let size = Vec2::new(MENU_W, MENU_H) * scale;
            if pos.x >= min.x
                && pos.x <= min.x + size.x
                && pos.y >= min.y
                && pos.y <= min.y + size.y
            {
                return Some(action);
            }
        }
    }

    None
}

#[cfg(test)]
mod layout_tests {
    //! Touch HUD hit-testing. The layout is the single source for both the
    //! rendered overlay and the Android multitouch path, so the key
    //! invariant is that every button's drawn center hit-tests back to
    //! itself (no drift between visible circle and touch target).
    use super::*;


    #[test]
    fn layout_has_nine_distinct_buttons_with_positive_size() {
        let layout = touch_action_layout();
        assert_eq!(layout.len(), 9);
        for spec in layout {
            assert!(spec.size > 0.0, "{:?} has non-positive size", spec.action);
            assert!(!spec.label.is_empty());
        }
        for (i, a) in layout.iter().enumerate() {
            for b in &layout[i + 1..] {
                assert_ne!(a.action, b.action, "duplicate touch action {:?}", a.action);
            }
        }
    }

    /// A cluster resolved at an arbitrary rectangle, for tests that must not
    /// re-derive the old window-anchored formula.
    fn cluster_at(min: Vec2, scale: f32) -> ScreenRect {
        ScreenRect::from_min_size(min, Vec2::new(ACTION_CLUSTER_W, ACTION_CLUSTER_H) * scale)
    }

    fn menu_at(min: Vec2) -> ScreenRect {
        ScreenRect::from_min_size(min, Vec2::new(MENU_ROW_W, crate::placement::MENU_ROW_H))
    }

    /// Every button's DRAWN centre hit-tests back to itself, wherever the
    /// cluster was placed and at whatever scale — including compacted into a
    /// reserved surround column, which is the case the old window-anchored hit
    /// test got wrong.
    #[test]
    fn each_button_center_hit_tests_back_to_itself() {
        for (name, cluster) in [
            ("bottom-right overlay", cluster_at(Vec2::new(1050.0, 500.0), 1.0)),
            ("reserved left column", cluster_at(Vec2::new(8.0, 800.0), 1.0)),
            ("compacted column", cluster_at(Vec2::new(12.0, 820.0), 0.9)),
        ] {
            for spec in touch_action_layout() {
                let (center, _) = touch_action_circle(spec, cluster);
                assert_eq!(
                    touch_action_at_position(center, Some(cluster), None),
                    Some(spec.action),
                    "{name}: centre of {:?} should hit itself (overlay/touch drift)",
                    spec.action,
                );
            }
        }
    }

    #[test]
    fn a_point_away_from_every_control_hits_nothing() {
        let cluster = cluster_at(Vec2::new(1050.0, 500.0), 1.0);
        assert_eq!(
            touch_action_at_position(Vec2::new(200.0, 200.0), Some(cluster), None),
            None,
        );
    }

    /// With no resolved rectangles there is nothing to hit — a hidden HUD must
    /// not stay tappable at its last position.
    #[test]
    fn an_unplaced_cluster_is_not_tappable() {
        assert_eq!(
            touch_action_at_position(Vec2::new(1100.0, 560.0), None, None),
            None,
        );
    }

    #[test]
    fn menu_row_buttons_are_hittable_at_their_resolved_rect() {
        let menu = menu_at(Vec2::new(900.0, 12.0));
        let start_center = menu.min + Vec2::new(4.0 + MENU_W * 0.5, 4.0 + MENU_H * 0.5);
        assert_eq!(
            touch_action_at_position(start_center, None, Some(menu)),
            Some(TouchActionButton::Start),
        );
    }

    /// Visible circles, not square bounds: diagonal neighbours may overlap as
    /// squares while their circles do not.
    #[test]
    fn touch_action_hit_test_uses_visible_circle_not_square_bounds() {
        let cluster = cluster_at(Vec2::new(1050.0, 500.0), 1.0);
        let layout = touch_action_layout();
        let attack = layout
            .iter()
            .find(|spec| matches!(spec.action, TouchActionButton::Attack))
            .expect("Attack remains in the touch action layout");
        let jump = layout
            .iter()
            .find(|spec| matches!(spec.action, TouchActionButton::Jump))
            .expect("Jump remains in the touch action layout");
        assert!(
            attack.top + attack.size > jump.top,
            "diagonal square bounds should be allowed to overlap vertically"
        );

        let square_only = cluster.min
            + Vec2::new(attack.left + attack.size - 2.0, jump.top + 2.0);
        assert_eq!(touch_action_at_position(square_only, Some(cluster), None), None);
    }
}

// ---------------------------------------------------------------------------
// Screen occupancy
// ---------------------------------------------------------------------------

use ambition_platformer_primitives::gameplay_presentation::{
    ScreenAnchor, ScreenOccluder, ScreenOcclusionPurpose,
};

/// Breathing room so the subject is not framed flush against a control.
const OCCUPANCY_PAD: f32 = 12.0;

/// The movement stick's reserved footprint — the same generous region the
/// menu-drag exclusion uses, so the two cannot disagree about where the stick
/// is.
pub fn movement_joystick_occluder() -> ScreenOccluder {
    ScreenOccluder::new(
        ScreenOcclusionPurpose::VirtualMovementStick,
        ScreenAnchor::BottomLeft,
        Vec2::ZERO,
        Vec2::splat(movement_joystick_layout().exclusion_size),
    )
    .with_padding(Vec2::splat(OCCUPANCY_PAD))
}

/// The right-thumb action cluster, measured from its bezel.
pub fn action_cluster_occluder() -> ScreenOccluder {
    ScreenOccluder::new(
        ScreenOcclusionPurpose::VirtualActionCluster,
        ScreenAnchor::BottomRight,
        Vec2::ZERO,
        Vec2::new(ACTION_BEZEL_W, ACTION_BEZEL_H),
    )
    .with_padding(Vec2::splat(OCCUPANCY_PAD))
}

/// The Menu/Back row. Published for completeness — its purpose does not
/// reserve subject space, because cornered chrome that is glanced at should
/// not shrink gameplay framing.
pub fn menu_row_occluder() -> ScreenOccluder {
    ScreenOccluder::new(
        ScreenOcclusionPurpose::SystemMenuControl,
        ScreenAnchor::TopRight,
        Vec2::splat(MENU_ROW_MARGIN),
        Vec2::new(MENU_ROW_W, 54.0),
    )
}
