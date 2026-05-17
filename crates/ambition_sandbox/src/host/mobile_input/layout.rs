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

/// Marker + identity for touch action buttons. Each `TouchActionButton`
/// entity is a Bevy `Button` whose `Interaction` state is folded into
/// the matching `TouchInputState` field each frame.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchActionButton {
    Jump,
    Attack,
    Dash,
    Blink,
    Interact,
    Projectile,
    FlyToggle,
    Shield,
    Start,
    Reset,
}

pub(super) const ACTION_CLUSTER_MARGIN: f32 = 10.0;
pub(super) const ACTION_BEZEL_PAD: f32 = 8.0;
pub(super) const ACTION_CLUSTER_W: f32 = 310.0;
pub(super) const ACTION_CLUSTER_H: f32 = 312.0;
pub(super) const ACTION_BEZEL_W: f32 = ACTION_CLUSTER_W + ACTION_BEZEL_PAD * 2.0;
pub(super) const ACTION_BEZEL_H: f32 = ACTION_CLUSTER_H + ACTION_BEZEL_PAD * 2.0;
pub(super) const MENU_ROW_MARGIN: f32 = 12.0;
pub(super) const MENU_ROW_W: f32 = 198.0;
pub(super) const MENU_W: f32 = 88.0;
pub(super) const MENU_H: f32 = 44.0;
/// 88px button + 4px margin each side.
pub(super) const MENU_CELL: f32 = 96.0;

#[derive(Clone, Copy, Debug)]
pub struct TouchActionSpec {
    pub action: TouchActionButton,
    pub label: &'static str,
    pub left: f32,
    pub top: f32,
    pub size: f32,
    pub font_size: f32,
}

/// Canonical lower-right action layout used by both the rendered UI and
/// raw multitouch hit testing. Keep all positions here so spacing fixes
/// cannot drift between the visible overlay and the Android touch path.
pub fn touch_action_layout() -> [TouchActionSpec; 8] {
    [
        TouchActionSpec {
            action: TouchActionButton::Blink,
            label: "Blink",
            left: 18.0,
            top: 10.0,
            size: 64.0,
            font_size: 13.0,
        },
        TouchActionSpec {
            action: TouchActionButton::FlyToggle,
            label: "Fly",
            left: 123.0,
            top: 2.0,
            size: 68.0,
            font_size: 14.0,
        },
        TouchActionSpec {
            action: TouchActionButton::Projectile,
            label: "Shot",
            left: 228.0,
            top: 10.0,
            size: 64.0,
            font_size: 13.0,
        },
        TouchActionSpec {
            action: TouchActionButton::Interact,
            label: "Interact",
            left: 116.0,
            top: 76.0,
            size: 76.0,
            font_size: 14.0,
        },
        TouchActionSpec {
            action: TouchActionButton::Attack,
            label: "Attack",
            left: 48.0,
            top: 148.0,
            size: 78.0,
            font_size: 14.0,
        },
        TouchActionSpec {
            action: TouchActionButton::Dash,
            label: "Dash",
            left: 184.0,
            top: 148.0,
            size: 78.0,
            font_size: 14.0,
        },
        TouchActionSpec {
            action: TouchActionButton::Shield,
            label: "Shield",
            left: 5.0,
            top: 222.0,
            size: 72.0,
            font_size: 13.0,
        },
        TouchActionSpec {
            action: TouchActionButton::Jump,
            label: "Jump",
            left: 115.0,
            top: 218.0,
            size: 80.0,
            font_size: 15.0,
        },
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
