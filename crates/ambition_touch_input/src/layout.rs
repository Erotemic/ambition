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

use super::exclusion::{TouchExclusionAnchor, TouchExclusionZone};

/// Marker + identity for touch action buttons. Each `TouchActionButton`
/// entity is a Bevy `Button` whose `Interaction` state is folded into
/// the matching `TouchInputState` field each frame.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
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
pub(super) const ACTION_CLUSTER_MARGIN: f32 = 10.0;
pub(super) const ACTION_BEZEL_PAD: f32 = 8.0;
pub(super) const ACTION_CLUSTER_W: f32 = 310.0 * TOUCH_SCALE;
pub(super) const ACTION_CLUSTER_H: f32 = 312.0 * TOUCH_SCALE;
pub(super) const ACTION_BEZEL_W: f32 = ACTION_CLUSTER_W + ACTION_BEZEL_PAD * 2.0;
pub(super) const ACTION_BEZEL_H: f32 = ACTION_CLUSTER_H + ACTION_BEZEL_PAD * 2.0;
/// Inset for the movement stick from the lower-left corner.
/// A slightly larger gap keeps the thumb control away from the
/// screen edge and leaves a cleaner buffer for gesture navigation.
pub(super) const JOYSTICK_MARGIN: f32 = 64.0 * TOUCH_SCALE;
/// Generous movement-stick footprint reserved from menu drag-scroll gestures.
pub(super) const JOYSTICK_EXCLUSION_SIZE: f32 = 300.0 * TOUCH_SCALE;
pub(super) const MENU_ROW_MARGIN: f32 = 12.0;
pub(super) const MENU_ROW_W: f32 = 198.0 * TOUCH_SCALE;
pub(super) const MENU_W: f32 = 88.0 * TOUCH_SCALE;
pub(super) const MENU_H: f32 = 44.0 * TOUCH_SCALE;
/// 88px button + 4px margin each side, scaled to match the shrunken
/// menu buttons so multitouch hit testing stays aligned with the
/// rendered overlay.
pub(super) const MENU_CELL: f32 = 96.0 * TOUCH_SCALE;

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

pub fn movement_joystick_exclusion_zone() -> TouchExclusionZone {
    let layout = movement_joystick_layout();
    TouchExclusionZone::rect(
        TouchExclusionAnchor::BottomLeft,
        Vec2::ZERO,
        Vec2::splat(layout.exclusion_size),
    )
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

pub fn touch_action_cluster_origin(window_size: Vec2) -> Vec2 {
    Vec2::new(
        window_size.x - ACTION_CLUSTER_MARGIN - ACTION_CLUSTER_W,
        window_size.y - ACTION_CLUSTER_MARGIN - ACTION_CLUSTER_H,
    )
}

/// Hit-test a `pos` against the visible action button circles and the
/// menu row at the top right. Touch positions use the same top-left-
/// origin logical coordinate space as Bevy window cursor positions.
///
/// Gameplay action buttons are visible *circles*, so this hit-tests
/// them as circles too — diagonal square bounds are allowed to
/// overlap when the circles themselves do not.
pub fn touch_action_at_position(pos: Vec2, window_size: Vec2) -> Option<TouchActionButton> {
    let cluster_origin = touch_action_cluster_origin(window_size);
    for spec in touch_action_layout() {
        let center = Vec2::new(
            cluster_origin.x + spec.left + spec.size * 0.5,
            cluster_origin.y + spec.top + spec.size * 0.5,
        );
        if pos.distance(center) <= spec.size * 0.5 {
            return Some(spec.action);
        }
    }

    // Menu row: right=MENU_ROW_MARGIN, top=MENU_ROW_MARGIN, Menu / Back.
    let menu_left = window_size.x - MENU_ROW_MARGIN - MENU_ROW_W;
    let menu_top = MENU_ROW_MARGIN;
    for (action, col) in [
        (TouchActionButton::Start, 0usize),
        (TouchActionButton::Reset, 1),
    ] {
        let left = menu_left + col as f32 * MENU_CELL + 4.0;
        let top = menu_top + 4.0;
        if pos.x >= left && pos.x <= left + MENU_W && pos.y >= top && pos.y <= top + MENU_H {
            return Some(action);
        }
    }

    None
}

pub fn touch_action_exclusion_zone(spec: TouchActionSpec) -> TouchExclusionZone {
    let offset = Vec2::new(
        ACTION_CLUSTER_MARGIN + ACTION_CLUSTER_W - spec.left - spec.size * 0.5,
        ACTION_CLUSTER_MARGIN + ACTION_CLUSTER_H - spec.top - spec.size * 0.5,
    );
    TouchExclusionZone::circle(TouchExclusionAnchor::BottomRight, offset, spec.size * 0.5)
}

pub fn touch_menu_button_exclusion_zone(col: usize) -> TouchExclusionZone {
    let offset = Vec2::new(
        MENU_ROW_MARGIN + MENU_ROW_W - (col as f32 * MENU_CELL + 4.0 + MENU_W * 0.5),
        MENU_ROW_MARGIN + 4.0 + MENU_H * 0.5,
    );
    TouchExclusionZone::rect(
        TouchExclusionAnchor::TopRight,
        offset - Vec2::new(MENU_W * 0.5, MENU_H * 0.5),
        Vec2::new(MENU_W, MENU_H),
    )
}

#[cfg(test)]
mod layout_tests {
    //! Touch HUD hit-testing. The layout is the single source for both the
    //! rendered overlay and the Android multitouch path, so the key
    //! invariant is that every button's drawn center hit-tests back to
    //! itself (no drift between visible circle and touch target).
    use super::*;

    const WINDOW: Vec2 = Vec2::new(1280.0, 720.0);

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

    #[test]
    fn each_button_center_hit_tests_back_to_itself() {
        let origin = touch_action_cluster_origin(WINDOW);
        for spec in touch_action_layout() {
            let center = Vec2::new(
                origin.x + spec.left + spec.size * 0.5,
                origin.y + spec.top + spec.size * 0.5,
            );
            assert_eq!(
                touch_action_at_position(center, WINDOW),
                Some(spec.action),
                "center of {:?} should hit itself (overlay/touch drift)",
                spec.action,
            );
        }
    }

    #[test]
    fn empty_screen_center_hits_nothing() {
        assert_eq!(touch_action_at_position(WINDOW * 0.5, WINDOW), None);
    }

    #[test]
    fn menu_row_start_button_is_hittable_top_right() {
        let menu_left = WINDOW.x - MENU_ROW_MARGIN - MENU_ROW_W;
        let start_center = Vec2::new(
            menu_left + 4.0 + MENU_W * 0.5,
            MENU_ROW_MARGIN + 4.0 + MENU_H * 0.5,
        );
        assert_eq!(
            touch_action_at_position(start_center, WINDOW),
            Some(TouchActionButton::Start),
        );
    }

    #[test]
    fn action_exclusion_matches_visible_button_center() {
        let origin = touch_action_cluster_origin(WINDOW);
        for spec in touch_action_layout() {
            let center = Vec2::new(
                origin.x + spec.left + spec.size * 0.5,
                origin.y + spec.top + spec.size * 0.5,
            );
            assert!(
                touch_action_exclusion_zone(spec).contains(center, WINDOW),
                "exclusion for {:?} should contain its visible center",
                spec.action,
            );
        }
    }

    #[test]
    fn joystick_exclusion_preserves_legacy_envelope() {
        let zone = movement_joystick_exclusion_zone();
        assert!(zone.contains(Vec2::new(4.0, WINDOW.y - 4.0), WINDOW));
        assert!(!zone.contains(
            Vec2::new(JOYSTICK_EXCLUSION_SIZE + 1.0, WINDOW.y - 4.0),
            WINDOW
        ));
    }
}
